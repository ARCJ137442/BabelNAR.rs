//! 封装一个简单的「交互式输入输出」

use std::{
    ffi::OsStr,
    io::{BufRead, BufReader, Result as IoResult, Write},
    process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};
use util::*;

/// 统一定义「输出侦听器」的类型
type OutputListener = dyn FnMut(String) + Send + Sync;

/// 简化定义`Arc< Mutex<T>>`
type ArcMutex<T> = Arc<Mutex<T>>;

/// 简化定义`Result<T, String>`
type ResultS<T> = Result<T, String>;

/// 构建一个「IO进程」
/// * 📌只是作为一个「构建器」存在
///   * 作为真正的`IoProcessManager`的launcher/builder
///
/// ! 因为有「系统指令」与「函数闭包」，无法派生任何常规宏
pub struct IoProcess {
    /// 内部封装的「进程指令」对象
    command: Command,
    /// 内部配置的「输出侦听器」
    out_listener: Option<Box<OutputListener>>,
}

impl IoProcess {
    /// 构造函数
    pub fn new(program_path: impl AsRef<OsStr>) -> Self {
        Self {
            command: Command::new(program_path),
            out_listener: None,
        }
    }

    /// 添加命令行参数
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        // 添加参数
        self.command.arg(arg);
        // 返回自身以便链式调用
        self
    }

    /// 添加输出侦听器
    /// * 📌此处因生命周期问题（难以绑定`listener`到`self`）设置`F`的约束为`'static`
    pub fn out_listener<F>(mut self, listener: F) -> Self
    where
        F: FnMut(String) + Send + Sync + 'static,
    {
        // 字段赋值
        self.out_listener = Some(Box::new(listener));
        // 返回自身以便链式调用
        self
    }

    /// 启动
    /// * 🚩通过[`Self::try_launch`]尝试启动，然后直接解包
    ///
    /// # Panics
    /// * 📌如果子进程创建失败，将直接 panic
    pub fn launch(self) -> IoProcessManager {
        self
            // 尝试启动
            .try_launch()
            //解包
            .expect("无法启动子进程")
    }

    /// 启动
    /// * 🚩此处只负责创建子进程[`Child`]，
    ///   * ⚠️不负责对子进程的控制（监听、通道）等
    pub fn try_launch(mut self) -> std::io::Result<IoProcessManager> {
        // 创建一个子进程
        let child =
            // 指令+参数
            self.command
                .arg("shell")
                // 输入输出
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                // 产生进程
                .spawn()?;
        println!("Started process: {}", child.id());

        // 获取输出侦听器
        let out_listener = self.out_listener;

        // 创建「子进程管理器」对象
        Ok(IoProcessManager::new(child, out_listener))
    }
}

/// 子进程管理器
/// * 🎯负责
///   * 统一管理子进程
///   * 封装提供易用的（字符串）输入输出接口
/// * 🚩现在兼容「输出侦听」与「输出通道」两处
///   * 🎯「输出侦听」用于「需要**响应式**即时处理输出，但又不想阻塞主进程/开新进程」时
///   * 🎯「输出通道」用于「需要封装『并发异步获取』延迟处理输出，兼容已有异步并发模型」时
#[allow(dead_code)]
pub struct IoProcessManager {
    /// 正在管理的子进程
    process: Child,

    /// 子进程的「写（到子进程的）输入」守护线程
    thread_write_in: JoinHandle<()>,
    /// 子进程的「读（到子进程的）输出」守护线程
    /// * 🚩现在兼容「侦听器」「通道」两种模式，重新必要化
    // thread_read_out: Option<JoinHandle<()>>,
    thread_read_out: JoinHandle<()>,

    /// 子线程的终止信号
    termination_signal: ArcMutex<bool>,

    /// 子进程输出的「接收者」
    /// * 🚩子进程发送给外部侦听器，同时由外部接收
    ///   * 在将输出发送给侦听器时，会在此留下备份
    /// * ⚠️如果直接调用[`Receiver::recv`]方法，可能会导致线程阻塞
    child_out: Mutex<Receiver<String>>,
    // ! 【2024-03-23 19:31:56】现在兼容「输出侦听」与「输出通道」二者
    /// 子进程输入的「发送者」
    /// * 🚩子进程接收来自外部发送的消息，由外部发送
    child_in: Mutex<Sender<String>>,
    // /// 子进程的「输出监听器」
    // out_listener: Option<Box<OutputListener>>,
    // ! 【2024-03-22 09:54:22】↑现在使用「输出侦听器」模式，此字段数据存储在`thread_read_out`中
    // /// 输出计数
    // /// * 🎯用于追踪输出数量，以便在不阻塞[`Self::child_out`]
    // num_output: ArcMutex<usize>,
    // ! ✅【2024-03-24 01:20:11】↑现在因[`Receiver::try_recv`]，无需使用此计数
    // * 📌【2024-03-24 01:20:38】并且，这个计数在测试中还偶发有不稳定行为（可能遗漏计数）
}

impl IoProcessManager {
    // * 初始化 * //

    /// 构造方法
    /// * 🚩从「子进程」与「输出侦听器」构造「进程管理者」
    pub fn new(mut child: Child, out_listener: Option<Box<OutputListener>>) -> Self {
        // 提取子进程的标准输入输出
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // 创建通道
        // * 📌IO流向：从左到右
        // ! 🚩【2024-03-22 09:53:12】现在采用「输出侦听器」的方法，不再需要封装通道
        let (child_out, out_sender) = channel();
        let (in_receiver, child_in) = channel();

        // 生成「终止信号」共享数据
        let termination_signal = Arc::new(Mutex::new(false));

        // // 生成「输出计数」共享数据
        // let num_output = Arc::new(Mutex::new(0));

        // 生成进程的「读写守护」（线程）
        let thread_write_in =
            IoProcessManager::spawn_thread_write_in(stdin, child_in, termination_signal.clone());
        let thread_read_out = IoProcessManager::spawn_thread_read_out(
            stdout,
            child_out,
            out_listener,
            termination_signal.clone(),
            // num_output.clone(),
        );
        // let thread_read_out =
        // out_listener.map(|listener| IoProcessManager::spawn_thread_read_out(stdout, listener));
        // ! 🚩【2024-03-23 19:33:45】↑现在兼容「侦听器」「通道」二者

        // 构造并返回自身
        Self {
            process: child,
            thread_read_out,
            thread_write_in,
            // 捕获通道的两端
            child_out: Mutex::new(out_sender),
            child_in: Mutex::new(in_receiver),
            // out_listener,
            // ! 【2024-03-22 09:53:50】↑不再于自身存储「输出侦听器」，而是存储在`thread_read_out`中
            // 共享变量
            termination_signal,
            // num_output,
            // ! 【2024-03-24 01:24:58】↑不再使用「输出计数」：有时会遗漏输出，并且有`try_recv`的更可靠方案
        }
    }

    /// 生成一个子线程，管理子进程的标准输入，接收通道另一端输出
    /// * 📌读输入，写进程 | stdin >>> child_in_receiver
    #[inline]
    fn spawn_thread_write_in(
        stdin: ChildStdin,
        child_in_receiver: Receiver<String>,
        termination_signal: ArcMutex<bool>,
    ) -> thread::JoinHandle<()> {
        // 生成一个简单的子线程，从通道中（阻塞性）读取数据，并随时将此计入标准输入
        thread::spawn(move || {
            // 从通道接收者读取输入 | 从「进程消息发送者」向进程发送文本
            let mut stdin = stdin;
            // ! 注意：这个`for`循环是阻塞的
            for line in child_in_receiver {
                // 检查终止信号 | ⚠️不要在终止后还发消息
                if *termination_signal.lock().expect("无法锁定终止信号") {
                    // println!("子进程收到终止信号");
                    break;
                }
                // 写入输出
                if let Err(e) = stdin.write_all(line.as_bytes()) {
                    println!("无法向子进程输入：{e:?}");
                }
            }
        })
    }

    /// 生成一个子线程，管理子进程的标准输出，传送输出的消息到另一端
    /// * 📌写输出 | child_out_sender >>> stdout
    /// * 🚩【2024-03-23 20:46:38】现在「侦听器」与「通道」并行运作
    /// * 📌核心逻辑
    ///   * 通过「缓冲区读取器」[`BufReader`]读取子进程输出
    ///   * 不断尝试读取，直到有内容
    ///   * 朝通道[`Sender`]发送内容
    #[inline]
    fn spawn_thread_read_out(
        stdout: ChildStdout,
        child_out_sender: Sender<String>,
        out_listener: Option<Box<dyn FnMut(String) + Send + Sync>>,
        termination_signal: ArcMutex<bool>,
        // num_output: ArcMutex<usize>,
    ) -> thread::JoinHandle<()> {
        // 将Option包装成一个新的函数
        // ! ⚠️【2024-03-23 19:54:43】↓类型注释是必须的：要约束闭包类型一致
        let mut listener_code: Box<dyn FnMut(&String) + Send + Sync> = match out_listener {
            // * 🚩先前有⇒实际执行 | 仅在实际有值时拷贝并传送给侦听器
            Some(mut listener) => Box::new(move |s: &String| listener(s.clone())),
            // * 🚩先前无⇒空函数
            None => Box::new(move |_| {}),
        };
        // 启动线程
        thread::spawn(move || {
            // 创建缓冲区读取器 | ⚠️【2024-03-23 23:42:08】这里的`BufReader`不能简化
            // * 📝`ChildStdout`没有`read_line`功能，但可以通过`BufReader`封装
            let mut stdout_reader = BufReader::new(stdout);

            // 创建缓冲区 | 🎯可持续使用
            let mut buf = String::new();

            // 持续循环
            loop {
                // 从子进程「标准输出」读取输入
                // * 📌此处非阻塞（会读到空），且`buf`会有换行符
                match stdout_reader.read_line(&mut buf) {
                    // 没有任何输入⇒检查终止信号
                    // * 📌不能在这里中断，需要检查终止信号
                    // * 🚩【2024-03-24 01:48:19】目前**允许**在进程终止时获取其输出
                    //   * 一般侦听器都能侦听到
                    Ok(0) => {
                        if dbg!(*termination_signal.lock().expect("无法锁定终止信号")) {
                            // println!("子进程收到终止信号");
                            break;
                        }
                    }
                    // 有效输入
                    Ok(_) => {
                        // ! 🚩现在兼容「侦听器」「通道」二者
                        // 先侦听 | 只传递引用，仅在「实际有侦听器」时拷贝消息
                        listener_code(&buf);
                        // 向「进程消息接收者」传递消息（实际上是「输出」）
                        if let Err(e) = child_out_sender.send(buf.clone()) {
                            println!("无法接收子进程输出：{e:?}");
                            break;
                        }
                        // // 输出计数
                        // match num_output.lock() {
                        //     Ok(mut n) => *n += 1,
                        //     Err(e) => println!("无法对子进程输出计数：{e:?}"),
                        // }
                        // ! 【2024-03-24 01:42:46】现在取消「输出计数」机制：计数可能不准确，并且被`try_recv`取代
                    }
                    // 报错⇒显示错误，终止读取
                    Err(e) => {
                        println!("子进程报错: {:?}", e);
                        break;
                    }
                }
                buf.clear();
            }
        })
    }

    // * 正常运作 * //

    /// 获取子进程id
    /// * 🚩调用[`Child::id`]方法
    pub fn id(&self) -> u32 {
        self.process.id()
    }

    /// （从「输出通道」中）拉取一个输出
    /// * 🎯用于（阻塞式等待）从子进程中收取输出信息
    /// * 🚩以字符串形式报告错误
    /// * ⚠️【2024-03-24 01:22:02】先前基于自身内置`num_output`的计数方法不可靠：有时会遗漏计数
    pub fn fetch_output(&mut self) -> ResultS<String> {
        // 访问自身「子进程输出」字段
        self.child_out
            // 互斥锁锁定
            .lock()
            .transform_err_string()?
            // 通道接收者接收
            .recv()
            .transform_err_string()
    }

    /// 尝试（从「输出通道」中）拉取一个输出
    /// * 🎯保证不会发生「线程阻塞」
    /// * 🚩类似[`Self::fetch_output`]，但仅在「有输出」时拉取
    /// * 📝[`Receiver`]自带的[`Receiver::try_recv`]就做了这件事
    /// * ⚠️【2024-03-24 01:22:02】先前基于自身内置`num_output`的计数方法不可靠：有时会遗漏计数
    pub fn try_fetch_output(&mut self) -> ResultS<Option<String>> {
        // 访问自身「子进程输出」字段，但加上`try`
        let out = self
            .child_out
            // 互斥锁锁定
            .lock()
            .transform_err_string()?
            // 通道接收者接收
            .try_recv()
            .ok();
        // ! ↑此处使用`ok`是为了区分「锁定错误」与「通道无输出」
        // 返回
        Ok(out)
    }

    /// 向子进程写入数据（字符串）
    /// * 🚩通过使用自身「子进程输入」的互斥锁，从中输入数据
    /// * ⚙️返回空，或返回字符串形式的错误（互斥锁错误）
    /// * ⚠️此方法需要【自行尾缀换行符】，否则不被视作有效输入
    ///   * 📄要触发输入，需传入"<A --> B>.\n"而非"<A --> B>."
    pub fn put(&self, input_line: impl ToString) -> ResultS<()> {
        // 从互斥锁中获取输入
        // * 🚩等待直到锁定互斥锁，最终在作用域结束（MutexGuard析构）时释放（解锁）
        // ! ❌【2024-03-23 23:59:20】此处的闭包无法简化成函数指针
        self.child_in
            // 锁定以获取`Sender`
            .lock()
            .transform_err_string()?
            // 发送
            .send(input_line.to_string())
            .transform_err_string()
    }

    /// 向子进程写入**一行**数据（字符串）
    /// * 🚩功能同[`Self::put`]，但会自动加上换行符
    /// * 📌类似[`print`]和[`println`]的关系
    /// * ⚠️此方法在输入后【自动添加换行符】
    ///   * 📄传入"<A --> B>."将自动转换成"<A --> B>.\n"
    ///   * ✅以此总是触发输入
    pub fn put_line(&self, input: impl ToString) -> ResultS<()> {
        self.put(format!("{}\n", input.to_string()))
    }

    /// 等待子进程结束
    /// * 🚩调用[`Child::wait`]方法
    /// * ⚠️对于【不会主动终止】的子进程，此举可能导致调用者死锁
    pub fn wait(&mut self) -> IoResult<ExitStatus> {
        self.process.wait()
    }

    /// 杀死自身
    /// * 🚩设置终止信号，通知子线程（以及标准IO）终止
    /// * 🚩调用[`Child::kill`]方法，终止子进程
    /// * ⚠️将借走自身所有权，终止并销毁自身
    pub fn kill(mut self) -> ResultS<()> {
        // ! ❌【2024-03-23 21:08:56】暂不独立其中的逻辑：无法脱开对`self`的借用
        // ! 📌更具体而言：对其中两个线程`thread_write_in`、`thread_read_out`的部分借用
        // 向子线程发送终止信号 //
        let mut signal = self.termination_signal.lock().transform_err_string()?;
        *signal = true;
        drop(signal); // ! 手动释放锁
                      // * 📝【2024-03-24 00:15:10】必须手动释放锁，否则会导致后续线程死锁

        // ! 解除子线程「write_stdin」的阻塞
        self.put("\n").unwrap();

        // 等待子线程终止 //
        self.thread_write_in.join().transform_err_debug()?;
        self.thread_read_out.join().transform_err_debug()?;

        // * 📝此时子线程连同「子进程的标准输入输出」一同关闭，
        //   * 子进程自身可以做输出
        // * 📄如：ONA在最后会打印程序运行报告
        //   * ⚠️这意味着「输出侦听器」仍然能对其输出产生响应

        // 杀死子进程 //
        self.process.kill().transform_err_string()
    }
}

/// 单元测试
#[cfg(test)]
mod tests {

    use super::*;
    use std::{
        process::exit,
        sync::{Arc, Mutex},
    };

    // 定义一系列路径
    #[allow(unused)]
    const EXE_PATH_ONA: &str = r"..\..\NARS-executables\NAR.exe";
    #[allow(unused)]
    const EXE_PATH_REPL: &str = r"..\..\..\Julia\语言学小工Ju\繁简转换\dist\repl_简化.exe";
    #[allow(unused)]
    const EXE_PATH_ECHO: &str = r"..\NAVM.rs\target\debug\examples\echo_exe.exe";

    /// 实用测试工具：启动一个ONA，并附带「输出缓存」
    fn launch_ona() -> (IoProcessManager, ArcMutex<Vec<String>>) {
        // 输出缓存
        let outputs = Arc::new(Mutex::new(vec![]));
        let outputs_inner = outputs.clone();
        // 从一个系统指令开始构建子进程
        let process = IoProcess::new(EXE_PATH_ONA)
            // 添加命令参数
            .arg("shell")
            // 添加输出监听器 | 简单回显
            // ! 【2024-03-22 10:06:38】基于「输出侦听器」的情形，若需要与外部交互，则会遇到所有权/生命周期问题
            // * 📄子进程与子进程外部（如此处的主进程）的问题
            // * ✅【2024-03-22 10:16:32】↑已使用`Arc<Mutex>`解决
            .out_listener(move |output: String| {
                outputs_inner
                    .lock()
                    .expect("无法锁定 outputs_inner")
                    .push(output.clone());
                print!("[OUT] {}", output);
            });
        // 启动子进程并返回
        (process.launch(), outputs)
    }

    /// 标准案例：ONA交互
    ///
    /// ## 测试输入
    ///
    /// ```plaintext
    /// <A --> B>.
    /// <B --> C>.
    /// <A --> C>?
    /// ```
    ///
    /// ## 预期输出
    ///
    /// ```plaintext
    /// Answer: <A --> C>. creationTime=2 Truth: frequency=1.000000, confidence=0.810000
    /// ```
    ///
    /// ## 笔记
    ///
    /// * 📝[`Arc`]能满足[`Sync`]+[`Send`]，但R[`efCell`]不满足
    ///   * ❌无法使用`Arc<RefCell<T>>`组合
    /// * 📝[`Mutex`]能进行进程交互，但无法共享引用
    /// * 🚩最终使用`ArcMutex<T>`作为进程交互的共享引用
    ///   * 📌[`Arc`]允许被拷贝并移动入闭包（共享引用，超越生命周期）
    ///   * 📌[`Mutex`]允许进程间共享的内部可变性（运行时借用检查）
    #[test]
    fn test_ona() {
        // 创建子进程
        let (mut process, outputs) = launch_ona();

        // 测试：输入输出 //
        let output_must_contains = |s: &str| {
            let outputs = outputs.lock().expect("无法锁定 outputs");
            let line = outputs
                .iter()
                .find(|line| line.contains(s))
                .expect("没有指定的输出！");
            println!("检验「{s:?}」成功！所在之处：{line:?}");
        };
        /// 等待子进程输出，直到输出满足条件
        fn await_fetch_until(process: &mut IoProcessManager, criterion: impl Fn(String) -> bool) {
            loop {
                let o = dbg!(process.fetch_output().expect("无法拉取输出"));
                println!("fetch到其中一个输入: {o:?}");
                if criterion(o) {
                    break;
                }
            }
        }

        // 先置入输入
        let input = "<A --> B>.";
        process.put_line(input).expect("无法放置输入");
        await_fetch_until(&mut process, |s| s.contains(input));

        // 中途检验
        output_must_contains("<A --> B>.");

        // 继续输入
        process.put("<B --> C>.\n").expect("无法放置输入");
        await_fetch_until(&mut process, |s| s.contains("<B --> C>."));

        process.put("<A --> C>?\n").expect("无法放置输入");
        await_fetch_until(&mut process, |s| s.contains("<A --> C>?"));

        // 不断fetch直到有答案
        const EXPECTED_ANSWER: &str = "Answer: <A --> C>.";
        await_fetch_until(&mut process, |s| s.contains(EXPECTED_ANSWER));

        // 最后检验 | 因为是缓存，所以会成功
        output_must_contains(EXPECTED_ANSWER);

        // // 等待结束
        // process.wait();

        // 读取其中缓冲区里边的数据（多了会阻塞！）
        {
            let r = process.child_out.lock().unwrap();
            for _ in r.try_iter() {
                let line = r.recv().expect("接收失败！");
                print!("从输出中读取到的一行（多了会阻塞！）：{line}");
            }
            // * 此处自动释放锁
        }

        // // 等待1秒并强制结束
        // println!("Waiting for 1 seconds and then killing the process...");
        // sleep_secs(1);
        // ! 【2024-03-24 01:39:45】现在由于`await_until_output`的存在，已无需手动等待
        process.kill().expect("无法杀死进程");
        println!("Process killed.");

        // 读取检验输出 | 杀死进程后还有
        dbg!(&*outputs);

        // 退出
        exit(0);
    }
}
