//! 启动后运行时的（交互与）管理

use crate::{launch_by_config, InputMode, LaunchConfig, LaunchConfigPreludeNAL};
use anyhow::{anyhow, Result};
use babel_nar::{
    cli_support::error_handling_boost::error_anyhow,
    eprintln_cli, println_cli,
    test_tools::{nal_format::parse, put_nal, VmOutputCache},
};
use nar_dev_utils::ResultBoost;
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmRuntime, VmStatus},
};
use std::{
    fmt::Debug,
    io::Result as IoResult,
    ops::ControlFlow,
    sync::{Arc, Mutex, MutexGuard},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

/// 读取行迭代器
/// * 🚩每迭代一次，请求用户输入一行
/// * ✨自动清空缓冲区
/// * ❌无法在【不复制字符串】的情况下实现「迭代出所输入内容」的功能
///   * ❌【2024-04-02 03:49:56】无论如何都无法实现：迭代器物件中引入就必须碰生命周期
/// * 🚩最终仍需复制字符串：调用处方便使用
/// * ❓是否需要支持提示词
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadlineIter {
    pub buffer: String,
}

/// 实现迭代器
impl Iterator for ReadlineIter {
    type Item = IoResult<String>;

    fn next(&mut self) -> Option<Self::Item> {
        // 清空缓冲区
        self.buffer.clear();
        // 读取一行
        // * 📝此处的`stdin`是懒加载的
        if let Err(e) = std::io::stdin().read_line(&mut self.buffer) {
            return Some(Err(e));
        }
        // 返回
        Some(IoResult::Ok(self.buffer.clone()))
    }
}

/// 线程间可变引用计数的别名
type ArcMutex<T> = Arc<Mutex<T>>;

/// 输出缓存
/// * 🎯统一「加入输出⇒打印输出」的逻辑
/// * 🚩仅封装一个[`Vec`]，而不对其附加任何[`Arc`]、[`Mutex`]的限定
///   * ❌【2024-04-03 01:43:13】[`Arc`]必须留给[`RuntimeManager`]：需要对其中键的值进行引用
#[derive(Debug)]
pub struct OutputCache {
    /// 内部封装的输出数组
    /// * 🚩【2024-04-03 01:43:41】不附带任何包装类型，仅包装其自身
    inner: Vec<Output>,
}

/// 功能实现
impl OutputCache {
    /// 构造函数
    pub fn new(inner: Vec<Output>) -> Self {
        Self { inner }
    }

    /// 默认[`Arc`]<[`Mutex`]>
    pub fn default_arc_mutex() -> ArcMutex<Self> {
        Arc::new(Mutex::new(Self::default()))
    }

    /// 从[`Arc`]<[`Mutex`]>中解锁
    pub fn unlock_arc_mutex(arc_mutex: &mut ArcMutex<Self>) -> Result<MutexGuard<'_, Self>> {
        arc_mutex.lock().transform_err(error_anyhow)
    }
}

/// 默认构造：空数组
impl Default for OutputCache {
    fn default() -> Self {
        Self::new(vec![])
    }
}

/// 实现「输出缓存」
/// * 不再涉及任何[`Arc`]或[`Mutex`]
impl VmOutputCache for OutputCache {
    /// 存入输出
    /// * 🎯统一的「打印输出」逻辑
    ///   * 🚩【2024-04-03 01:07:55】不打算封装了
    fn put(&mut self, output: Output) -> Result<()> {
        // 尝试打印输出
        println_cli!(&output);

        // 加入输出
        self.inner.push(output);
        Ok(())
    }

    /// 遍历输出
    /// * 🚩不是返回迭代器，而是用闭包开始计算
    fn for_each<T>(&self, f: impl Fn(&Output) -> ControlFlow<T>) -> Result<Option<T>> {
        // 遍历
        for output in self.inner.iter() {
            // 基于控制流的运行
            match f(output) {
                ControlFlow::Break(value) => return Ok(Some(value)),
                ControlFlow::Continue(()) => {}
            }
        }

        // 返回
        Ok(None)
    }
}

/// 运行时管理器
/// * 🎯在一个数据结构中封装「虚拟机运行时」与「配置信息」
/// * 📌只负责**单个运行时**的运行管理
///   * 🚩不负责「终止、重启运行时」等过程
#[derive(Debug, Clone)]
pub struct RuntimeManager<R>
where
    // ! 🚩【2024-04-02 14:51:23】需要`Send + Sync`进行多线程操作，需要`'static`保证生命周期
    R: VmRuntime + Send + Sync + 'static,
{
    /// 内部封装的虚拟机运行时
    /// * 🏗️后续可能会支持「同时运行多个虚拟机」
    /// * 🚩多线程共享：输入/输出
    runtime: Arc<Mutex<R>>,

    /// 内部封装的「命令行参数」
    /// * 🎯用于从命令行中加载配置
    /// * 🚩只读
    config: Arc<LaunchConfig>,

    /// 内部缓存的「NAVM输出」
    /// * 🎯用于NAL测试
    /// * 🚩多线程共享
    output_cache: Arc<Mutex<OutputCache>>,
}

impl<R> RuntimeManager<R>
where
    R: VmRuntime + Send + Sync + 'static,
{
    /// 构造函数
    pub fn new(runtime: R, config: LaunchConfig) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            config: Arc::new(config),
            output_cache: OutputCache::default_arc_mutex(),
        }
    }

    /// 在运行时启动后，对其进行管理
    /// * 🎯健壮性：更多「警告/重来」而非`panic`
    /// * 🎯用户友好：尽可能隐藏底层内容
    ///   * 如错误堆栈
    /// * 📌主要逻辑
    ///   * `.nal`脚本预加载
    ///   * 用户的运行时交互
    ///   * Websocket服务端
    /// * 🚩【2024-04-03 00:33:41】返回的[`Result`]作为程序的终止码
    ///   * `Ok(Ok(..))` ⇒ 程序正常退出
    ///   * `Ok(Err(..))` ⇒ 程序异常退出
    pub fn manage(&mut self) -> Result<Result<()>> {
        // 生成「读取输出」子线程 | 📌必须最先
        let thread_read = self.spawn_read_output()?;

        // 预置输入 | ⚠️阻塞
        if let Err(e) = self.prelude_nal() {
            println_cli!([Error] "预置NAL输入发生错误：{e}")
        }

        // 虚拟机被终止 & 无用户输入 ⇒ 程序退出
        if let VmStatus::Terminated(..) = self.runtime.lock().transform_err(error_anyhow)?.status()
        {
            if !self.config.user_input {
                // 直接返回，使程序退出
                return Ok(Ok(()));
            }
        }

        // 生成「Websocket服务」子线程
        let thread_ws = self.spawn_ws_server()?;

        // 生成「用户输入」子线程
        let mut thread_input = None;
        if self.config.user_input {
            thread_input = Some(self.spawn_user_input()?);
        }

        // ! 🚩不要在主线程开始用户输入

        // 等待子线程结束，并抛出其抛出的错误
        // ! 🚩【2024-04-02 15:09:32】错误处理交给外界
        thread_read.join().transform_err(error_anyhow)??;
        thread_ws.join().transform_err(error_anyhow)??;
        if let Some(thread_input) = thread_input {
            thread_input.join().transform_err(error_anyhow)??;
        }

        // 正常运行结束
        Ok(Ok(()))
    }

    /// 预置NAL
    /// * 🎯用于自动化调取`.nal`文件进行测试
    pub fn prelude_nal(&mut self) -> Result<()> {
        let config = &*self.config;

        // 尝试获取运行时引用 | 仅有其它地方panic了才会停止
        let runtime = &mut *self.runtime.lock().transform_err(error_anyhow)?;

        // 仅在有预置NAL时开始
        if let Some(prelude_nal) = &config.prelude_nal {
            // 尝试获取输出缓冲区引用 | 仅有其它地方panic了才会停止
            let output_cache = &mut *OutputCache::unlock_arc_mutex(&mut self.output_cache)?;

            // 读取内容
            let nal = match prelude_nal {
                // 文件⇒尝试读取文件内容 | ⚠️此处创建了一个新值，所以要统一成`String`
                LaunchConfigPreludeNAL::File(path) => std::fs::read_to_string(path)?,
                // 纯文本⇒直接引入
                LaunchConfigPreludeNAL::Text(nal) => nal.to_string(),
            };

            // 输入NAL
            Self::input_nal_to_vm(runtime, &nal, output_cache, config)
        }

        // 返回
        Ok(())
    }

    /// 生成「读取输出」子线程
    pub fn spawn_read_output(&mut self) -> Result<JoinHandle<Result<()>>> {
        // 准备引用
        let runtime = self.runtime.clone();
        let output_cache = self.output_cache.clone();

        // 启动线程
        let thread = thread::spawn(move || {
            loop {
                // 尝试获取运行时引用 | 仅有其它地方panic了才会停止
                let mut runtime = runtime.lock().transform_err(error_anyhow)?;

                // 若运行时已终止，返回终止信号
                if let VmStatus::Terminated(result) = runtime.status() {
                    // * 🚩【2024-04-02 21:48:07】↓下面没法简化：[`anyhow::Result`]拷贝之后还是引用
                    match result {
                        Ok(..) => break Ok(()),
                        Err(e) => break Err(anyhow!("NAVM运行时已终止：{e}")),
                    }
                }

                // 尝试拉取所有NAVM运行时输出
                while let Ok(Some(output)) = runtime
                    .try_fetch_output()
                    .inspect_err(|e| eprintln_cli!([Error] "尝试拉取NAVM运行时输出时发生错误：{e}"))
                {
                    // 格式化输出
                    // * 🚩可能还要交给Websocket
                    println_cli!(&output);

                    // 缓存输出
                    match output_cache.lock() {
                        Ok(mut output_cache) => output_cache.put(output)?,
                        Err(e) => eprintln_cli!([Error] "缓存NAVM运行时输出时发生错误：{e}"),
                    }
                }
            }
        });

        // 返回启动的线程
        Ok(thread)
    }

    /// 生成「Websocket服务」子线程
    pub fn spawn_ws_server(&mut self) -> Result<JoinHandle<Result<()>>> {
        // 准备引用
        let runtime_arc = self.runtime.clone();

        // 启动线程
        let thread = thread::spawn(move || {
            loop {
                // 尝试获取运行时引用 | 仅有其它地方panic了才会停止
                let mut runtime = runtime_arc.lock().transform_err(error_anyhow)?;
                // TODO: Websocket服务端逻辑
            }
        });

        // 返回启动的线程
        Ok(thread)
    }

    /// 生成「用户输入」子线程
    pub fn spawn_user_input(&mut self) -> Result<JoinHandle<Result<()>>> {
        // 准备引用
        // ! 📝不能在此外置「可复用引用」变量：borrowed data escapes outside of method
        let runtime = self.runtime.clone();
        let config = self.config.clone();
        let output_cache = self.output_cache.clone();

        // 启动线程
        let thread = thread::spawn(move || {
            // 主循环
            // ! 📝不能在此中出现裸露的`MutexGuard`对象：其并非线程安全
            //   * ✅可使用`&(mut) *`重引用语法，从`MutexGuard`转换为线程安全的引用
            //   * ✅对`Arc`使用`&*`同理：可以解包成引用，以便后续统一传递值的引用
            for io_result in ReadlineIter::default() {
                // 从迭代器中读取一行
                let line = io_result?;

                // 尝试获取运行时引用 | 仅有其它地方panic了才会停止
                // ! 📝PoisonError无法在线程中传递
                let runtime = &mut *runtime
                    .lock()
                    .transform_err(|e| anyhow!("获取运行时引用时发生错误：{e:?}"))?;

                // 若运行时已终止，返回终止信号
                if let VmStatus::Terminated(result) = runtime.status() {
                    // * 🚩【2024-04-02 21:48:07】↓下面没法简化：[`anyhow::Result`]拷贝之后还是引用
                    match result {
                        Ok(..) => return Ok(()),
                        Err(e) => return Err(anyhow!("NAVM运行时已终止：{e}")),
                    }
                }

                // 尝试获取输出缓冲区引用 | 仅有其它地方panic了才会停止
                // ! 🚩【2024-04-02 19:27:01】及早报错：即便无关紧要，也停止
                let output_cache = &mut *output_cache
                    .lock()
                    .transform_err(|e| anyhow!("获取NAVM输出缓存时发生错误：{e}"))?;

                // 非空⇒解析输入并执行
                if !line.trim().is_empty() {
                    if let Err(e) = Self::input_line_to_vm(runtime, &line, &config, output_cache) {
                        println_cli!([Error] "输入过程中发生错误：{e}")
                    }
                }
            }

            // 返回
            Ok(())
        });

        // 返回启动的线程
        Ok(thread)
    }

    /// 置入一行输入
    pub fn input_line_to_vm(
        runtime: &mut R,
        line: &str,
        config: &LaunchConfig,
        output_cache: &mut OutputCache,
    ) -> Result<()> {
        // 向运行时输入
        match config.input_mode {
            // NAVM指令
            InputMode::Cmd => Self::input_cmd_to_vm(runtime, line),
            // NAL输入
            InputMode::Nal => Self::input_nal_to_vm(runtime, line, output_cache, config),
        }

        // 输入完成
        Ok(())
    }

    /// 像NAVM实例输入NAVM指令
    fn input_cmd_to_vm(runtime: &mut R, line: &str) {
        if let Ok(cmd) =
            Cmd::parse(line).inspect_err(|e| eprintln_cli!([Error] "NAVM指令解析错误：{e}"))
        {
            let _ = runtime
                .input_cmd(cmd)
                .inspect_err(|e| eprintln_cli!([Error] "NAVM指令执行错误：{e}"));
        }
    }

    /// 像NAVM实例输入NAL（输入）
    /// * 🎯预置、用户输入、Websocket输入
    /// * ⚠️可能有多行
    fn input_nal_to_vm(
        runtime: &mut R,
        input: &str,
        output_cache: &mut OutputCache,
        config: &LaunchConfig,
    ) {
        // 解析输入，并遍历解析出的每个NAL输入
        for input in parse(input) {
            // 尝试解析NAL输入
            match input {
                Ok(nal) => {
                    // 尝试置入NAL输入 | 为了错误消息，必须克隆
                    put_nal(runtime, nal.clone(), output_cache, config.user_input).unwrap_or_else(
                        // TODO: 严格模式：预期失败时上报错误，乃至使整个程序运行失败
                        |e| eprintln_cli!([Error] "置入NAL输入「{nal:?}」时发生错误：{e}"),
                    );
                }
                // 错误⇒报错
                Err(e) => eprintln_cli!(
                    [Error] "解析NAL输入时发生错误：{e}"
                ),
            }
        }
    }
}

/// 重启虚拟机
/// * 🚩消耗原先的虚拟机管理者，返回一个新的管理者
///   * 🚩【2024-04-02 20:25:21】目前对「终止先前虚拟机」持放松态度
/// * 📝从`Arc<Mutex<T>>`中拿取值的所有权：[`Arc::try_unwrap`] + [`Mutex::into_inner`]
///   * 🔗参考：<https://users.rust-lang.org/t/move-out-of-arc-mutex-t/85940>
pub fn restart_manager(
    manager: RuntimeManager<impl VmRuntime + Send + Sync>,
) -> Result<RuntimeManager<impl VmRuntime + Send + Sync>> {
    // 尝试终止先前的虚拟机
    // ! ❌[`Arc::try_unwrap`]的返回值包括`VmRuntime`，所以连[`Debug`]都不支持
    // ! ❌【2024-04-02 20:33:01】目前测试中`Arc::into_inner`基本总是失败（线程里还有引用）
    // * 🚩【2024-04-02 20:33:18】现在通过修改NAVM API，不再需要获取运行时所有权了（销毁交给）
    // let old_runtime_mutex =
    // Arc::into_inner(manager.runtime).ok_or(anyhow!("runtime Arc解包失败"))?;
    // let mut old_runtime = old_runtime_mutex.into_inner()?;
    let old_runtime = &mut *manager
        .runtime
        .lock()
        .transform_err(|e| anyhow!("runtime Mutex解锁失败：{e:?}"))?;
    old_runtime.terminate()?;

    // 启动新的虚拟机
    let config_ref = &*manager.config;
    let new_runtime = launch_by_config(config_ref.clone())?;
    let new_manager = RuntimeManager::new(new_runtime, config_ref.clone());

    // 返回
    Ok(new_manager)
}

/// 根据配置（的「是否重启」选项）管理（一系列）虚拟机实例
pub fn loop_manage(
    mut manager: RuntimeManager<impl VmRuntime + Send + Sync>,
    config: &LaunchConfig,
) -> Result<()> {
    match manager.manage() {
        // 返回了「结果」⇒解包并传递结果
        Ok(result) => result,
        // 发生错误⇒尝试处理
        Err(e) => {
            // 打印错误信息
            println_cli!([Error] "运行时发生错误：{e}");
            // 尝试重启
            if config.auto_restart {
                println_cli!([Info] "程序将在 2 秒后自动重启。。。");
                sleep(Duration::from_secs(2));
                let new_manager = match restart_manager(manager) {
                    Ok(manager) => manager,
                    Err(e) => {
                        println_cli!([Error] "重启失败：{e}");
                        return Err(anyhow!("NAVM运行时发生错误，且重启失败：{e}"));
                    }
                };
                // 重启之后继续循环
                return loop_manage(new_manager, config);
            }
            // 正常返回
            Ok(())
        }
    }
}
