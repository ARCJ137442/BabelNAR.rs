//! 启动后运行时的（交互与）管理

use super::websocket_server::*;
use crate::{launch_by_runtime_config, InputMode, LaunchConfigPreludeNAL, RuntimeConfig};
use anyhow::{anyhow, Result};
use babel_nar::{
    cli_support::{
        error_handling_boost::error_anyhow,
        io::{
            navm_output_cache::{ArcMutex, OutputCache},
            readline_iter::ReadlineIter,
        },
    },
    eprintln_cli, println_cli,
    runtimes::TranslateError,
    test_tools::{nal_format::parse, put_nal, VmOutputCache},
};
use nar_dev_utils::{if_return, ResultBoost};
use navm::{
    cmd::Cmd,
    vm::{VmRuntime, VmStatus},
};
use std::{
    fmt::Debug,
    ops::{ControlFlow, ControlFlow::Break, ControlFlow::Continue},
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

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
    pub(crate) runtime: ArcMutex<R>,

    /// 内部封装的「命令行参数」
    /// * 🎯用于从命令行中加载配置
    /// * 🚩只读
    pub(crate) config: Arc<RuntimeConfig>,

    /// 内部缓存的「NAVM输出」
    /// * 🎯用于NAL测试
    /// * 🚩多线程共享
    pub(crate) output_cache: ArcMutex<OutputCache>,
}

impl<R> RuntimeManager<R>
where
    R: VmRuntime + Send + Sync + 'static,
{
    /// 构造函数
    /// * 🎯由此接管虚拟机实例、配置的所有权
    pub fn new(runtime: R, config: RuntimeConfig) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            config: Arc::new(config),
            output_cache: OutputCache::default_arc_mutex(),
        }
    }

    /// 【主函数】在运行时启动后，对其进行管理
    /// * 🎯健壮性：更多「警告/重来」而非`panic`
    /// * 🎯用户友好：尽可能隐藏底层内容
    ///   * 如错误堆栈
    /// * 📌主要逻辑
    ///   * `.nal`脚本预加载
    ///   * 用户的运行时交互
    ///   * Websocket服务端
    /// * 🚩【2024-04-03 00:33:41】返回的[`Result`]作为程序的终止码
    ///   * `Ok(Ok(..))` ⇒ 程序正常终止
    ///   * `Ok(Err(..))` ⇒ 程序异常终止
    ///   * `Err(..)` ⇒ 程序异常中断
    pub fn manage(&mut self) -> Result<Result<()>> {
        // 生成「读取输出」子线程 | 📌必须最先
        let thread_read = self.spawn_read_output()?;

        // 预置输入 | ⚠️阻塞
        let prelude_result = self.prelude_nal();
        match prelude_result {
            // 预置输入要求终止⇒终止
            Break(result) => return Ok(result),
            // 预置输入发生错误⇒展示 & 继续
            Continue(Err(e)) => println_cli!([Error] "预置NAL输入发生错误：{e}"),
            Continue(Ok(..)) => (),
        }

        // 虚拟机被终止 & 无用户输入 ⇒ 程序退出
        if let VmStatus::Terminated(..) = self.runtime.lock().transform_err(error_anyhow)?.status()
        {
            if !self.config.user_input {
                // 直接返回，使程序退出
                return Ok(Ok(()));
            }
        }

        // 生成「Websocket服务」子线程（若有连接）
        let thread_ws = self.try_spawn_ws_server()?;

        // 生成「用户输入」子线程
        let mut thread_input = None;
        if self.config.user_input {
            thread_input = Some(self.spawn_user_input()?);
        }

        // ! 🚩不要在主线程开始用户输入

        // 等待子线程结束，并抛出其抛出的错误
        // ! 🚩【2024-04-02 15:09:32】错误处理交给外界
        thread_read.join().transform_err(error_anyhow)??;
        if let Some(thread_ws) = thread_ws {
            thread_ws.join().transform_err(error_anyhow)??
        }
        if let Some(thread_input) = thread_input {
            thread_input.join().transform_err(error_anyhow)??;
        }

        // 正常运行结束
        Ok(Ok(()))
    }

    /// 预置NAL
    /// * 🎯用于自动化调取`.nal`文件进行测试
    /// * 🚩【2024-04-03 10:28:18】使用[`ControlFlow`]对象以控制「是否提前返回」和「返回的结果」
    ///   * 📌[`Continue`] => 使用「警告&忽略」的方式处理[`Result`] => 继续（用户输入/Websocket服务端）
    ///   * 📌[`Break`] => 告知调用者「需要提前结束」
    ///     * 📌[`Break`]([`Ok`]) => 正常退出
    ///     * 📌[`Break`]([`Err`]) => 异常退出（报错）
    pub fn prelude_nal(&mut self) -> ControlFlow<Result<()>, Result<()>> {
        let config = &*self.config;

        /// 尝试获取结果并返回
        /// * 🎯对错误返回`Break(Err(错误))`而非`Err(错误)`
        macro_rules! try_break {
            // 统一逻辑
            ($v:expr => $e_id:ident $e:expr) => {
                match $v {
                    // 获取成功⇒返回并继续
                    Ok(v) => v,
                    // 获取失败⇒ 告知「异常结束」
                    Err($e_id) => return Break(Err($e)),
                }
            };
            // 两种错误分派方法
            ($v:expr) => { try_break!($v => e e.into()) };
            (anyhow $v:expr) => { try_break!($v => e error_anyhow(e)) }; // * 🎯针对`PoisonError`
        }

        // 尝试获取运行时引用 | 仅有其它地方panic了才会停止
        let runtime = &mut *try_break!(anyhow self.runtime.lock());

        // 仅在有预置NAL时开始
        if let Some(prelude_nal) = &config.prelude_nal {
            // 尝试获取输出缓冲区引用 | 仅有其它地方panic了才会停止
            let output_cache =
                &mut *try_break!(OutputCache::unlock_arc_mutex(&mut self.output_cache));

            // 读取内容
            let nal = match prelude_nal {
                // 文件⇒尝试读取文件内容 | ⚠️此处创建了一个新值，所以要统一成`String`
                LaunchConfigPreludeNAL::File(path) => {
                    try_break!(std::fs::read_to_string(path) => e {
                        println_cli!([Error] "读取预置NAL文件 {path:?} 发生错误：{e}");
                        // 继续（用户输入/Websocket服务端）
                        e.into()
                    })
                }
                // 纯文本⇒直接引入
                LaunchConfigPreludeNAL::Text(nal) => nal.to_string(),
            };

            // 输入NAL并处理
            // * 🚩【2024-04-03 11:10:44】遇到错误，统一上报
            //   * 根据「严格模式」判断要「继续」还是「终止」
            let put_result = Self::input_nal_to_vm(runtime, &nal, output_cache, config);
            match self.config.strict_mode {
                false => Continue(put_result),
                true => Break(put_result),
            }
        }
        // 否则自动返回「正常」
        else {
            // 返回 | 正常继续
            Continue(Ok(()))
        }
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
                    // 缓存输出
                    // * 🚩在缓存时格式化输出
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
    pub fn try_spawn_ws_server(&mut self) -> Result<Option<JoinHandle<Result<()>>>> {
        // 若有⇒启动
        if let Some(config) = &self.config.websocket {
            let thread = spawn_ws_server(self, &config.host, config.port);
            return Ok(Some(thread));
        }

        // 完成，即便没有启动
        Ok(None)
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
            // ! 不建议在此启用提示词：会被异步的输出所打断
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
        config: &RuntimeConfig,
        output_cache: &mut OutputCache,
    ) -> Result<()> {
        // 向运行时输入
        match config.input_mode {
            // NAVM指令
            InputMode::Cmd => Self::input_cmd_to_vm(runtime, line),
            // NAL输入
            InputMode::Nal => Self::input_nal_to_vm(runtime, line, output_cache, config),
        }
    }

    /// 像NAVM实例输入NAVM指令
    fn input_cmd_to_vm(runtime: &mut R, line: &str) -> Result<()> {
        let cmd =
            Cmd::parse(line).inspect_err(|e| eprintln_cli!([Error] "NAVM指令解析错误：{e}"))?;
        runtime
            .input_cmd(cmd)
            .inspect_err(|e| eprintln_cli!([Error] "NAVM指令执行错误：{e}"))
    }

    /// 向NAVM实例输入NAL（输入）
    /// * 🎯预置、用户输入、Websocket输入
    /// * 🎯严格模式
    ///   * 📌要么是「有失败 + 非严格模式 ⇒ 仅报告错误」
    ///   * 📌要么是「有一个失败 + 严格模式 ⇒ 返回错误」
    /// * ⚠️可能有多行
    fn input_nal_to_vm(
        runtime: &mut R,
        input: &str,
        output_cache: &mut OutputCache,
        config: &RuntimeConfig,
    ) -> Result<()> {
        // 解析输入，并遍历解析出的每个NAL输入
        for input in parse(input) {
            // 尝试解析NAL输入
            match input {
                // 错误⇒根据严格模式处理
                Err(e) => {
                    // 无论是否严格模式，都报告错误
                    eprintln_cli!([Error] "解析NAL输入时发生错误：{e}");
                    // 严格模式下提前返回
                    if_return! { config.strict_mode => Err(e) }
                }
                Ok(nal) => {
                    // 尝试置入NAL输入 | 为了错误消息，必须克隆
                    let put_result = put_nal(runtime, nal.clone(), output_cache, config.user_input);
                    // 处理错误
                    if let Err(e) = put_result {
                        // 无论是否严格模式，都报告错误
                        eprintln_cli!([Error] "置入NAL输入「{nal:?}」时发生错误：{e}");
                        // 严格模式下考虑上报错误
                        if config.strict_mode {
                            match e.downcast_ref::<TranslateError>() {
                                // * 🚩在「不支持的指令」时仅警告
                                // * 🎯**兼容尽可能多的CIN版本**
                                Some(TranslateError::UnsupportedInput(..)) => {}
                                // * 🚩在「其他错误」时直接返回
                                _ => return Err(e),
                            }
                        }
                    }
                }
            }
        }
        // 正常返回
        Ok(())
    }
}

/// 重启虚拟机
/// * 🚩消耗原先的虚拟机管理者，返回一个新的管理者
///   * 🚩【2024-04-02 20:25:21】目前对「终止先前虚拟机」持放松态度
/// * 📝从`ArcMutex<T>>`中拿取值的所有权：[`Arc::try_unwrap`] + [`Mutex::into_inner]
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
    let new_runtime = launch_by_runtime_config(config_ref)?;
    let new_manager = RuntimeManager::new(new_runtime, config_ref.clone());

    // 返回
    Ok(new_manager)
}

/// 根据配置（的「是否重启」选项）管理（一系列）虚拟机实例
pub fn loop_manage(
    mut manager: RuntimeManager<impl VmRuntime + Send + Sync>,
    config: &RuntimeConfig,
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
