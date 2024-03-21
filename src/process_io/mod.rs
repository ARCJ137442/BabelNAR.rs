//! 用于封装抽象「进程通信」逻辑
//! 示例代码来源：https://www.nikbrendler.com/rust-process-communication/
//! * 📌基于「通道」的「子进程+专职读写的子线程」通信逻辑
//!
//! TODO: 封装抽象提取

#![allow(unused)]

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

fn sleep_secs(secs: u64) {
    sleep(Duration::from_secs(secs));
}

/// 启动子进程
fn start_process<S: AsRef<OsStr>>(
    program_path: S,
    sender: Sender<String>,
    receiver: Receiver<String>,
) {
    // 创建一个子进程
    let child =
        // 指令+参数
            Command::new(program_path)
            .arg("shell")
            // 输入输出
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // 产生进程
            .spawn()
            .expect("Failed to start process");

    println!("Started process: {}", child.id());

    let stdin = child.stdin.unwrap();
    let stdout = child.stdout.unwrap();
    /// 生成进程的「读写守护」（线程）
    let thread_write_in = spawn_thread_write_in(stdin, receiver);
    let thread_read_out = spawn_thread_read_out(stdout, sender);
}

/// 生成一个子线程，管理子进程的标准输入，接收通道另一端输出
/// * 📌读输入，写进程
fn spawn_thread_write_in(stdin: ChildStdin, receiver: Receiver<String>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // 从通道接收者读取输入 | 从「进程消息发送者」向进程发送文本
        let mut stdin = stdin;
        for line in receiver {
            // 写入输出
            if let Err(e) = stdin.write_all(line.as_bytes()) {
                println!("无法向子进程输入：{e:?}");
            }
        }
    })
}

/// 生成一个子线程，管理子进程的标准输出，传送输出的消息到另一端
/// * 📌写输出
fn spawn_thread_read_out(stdout: ChildStdout, sender: Sender<String>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // 读取输出
        let mut stdout_reader = BufReader::new(stdout);
        // 持续循环
        loop {
            // 从子进程「标准输出」读取输入
            let mut buf = String::new();
            match stdout_reader.read_line(&mut buf) {
                // 没有任何输入⇒跳过
                Ok(0) => continue,
                // 有效输入
                Ok(_) => {
                    println!("子进程输出: {buf:?}");
                    // 向「进程消息接收者」传递消息（实际上是「输出」）
                    if let Err(e) = sender.send(buf) {
                        println!("无法接收子进程输出：{e:?}");
                        break;
                    }
                    continue;
                }
                Err(e) => {
                    println!("子进程报错: {:?}", e);
                    break;
                }
            }
        }
    })
}

fn start_command_thread(mutex: Mutex<Sender<String>>) {
    // 生成一个子线程，对上述进程进行读取
    thread::spawn(move || {
        let sender = mutex.lock().unwrap();
        // 测试输入输出
        sleep_secs(1);
        sender.send("<A --> B>.\n".into()).unwrap();
        sleep_secs(1);
        sender.send("<B --> C>.\n".into()).unwrap();
        sleep_secs(1);
        sender.send("<A --> C>?\n".into()).unwrap();
        sleep_secs(1);
    });
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    // 定义一系列路径
    const EXE_PATH_ONA: &str = r"..\..\NARS-executables\NAR.exe";
    const EXE_PATH_REPL: &str = r"..\..\..\Julia\语言学小工Ju\繁简转换\dist\repl_简化.exe";
    const EXE_PATH_ECHO: &str = r"..\NAVM.rs\target\debug\examples\echo_exe.exe";

    /// 实验用测试
    #[test]
    fn test() {
        // 创建通道
        let (child_out, out_sender) = channel();
        let (in_receiver, child_in) = channel();

        // 启动进程
        start_process(EXE_PATH_ONA, child_out, child_in);

        // tx2.send(("Command 1\n".into())).unwrap();
        let mutex = Mutex::new(in_receiver);
        start_command_thread(mutex);
        // println!("{in_receiver:?}");

        // 从外部获取输出（阻塞）
        // for line in out_sender {
        //     println!("Got this back: {}", line);
        // }

        // 等待
        sleep_secs(5);
        println!("程序结束！");
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
    /// TODO: 【2024-03-21 10:02:34】按想要的「目标形式」写测试，然后以此驱动开发整个库（面向用法）
    #[test]
    fn test_ona() {
        // let runtime = Runtime::builder();
    }
}
