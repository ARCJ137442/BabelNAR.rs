//! 启动后运行时的（交互与）管理

use crate::CliArgs;
use anyhow::Result;
use nar_dev_utils::if_return;
use navm::{cmd::Cmd, output::Output, vm::VmRuntime};
use std::{fmt::Debug, io::Result as IoResult};

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

/// 打印错误
fn println_error(e: &impl Debug) {
    println!("{e:?}");
}

/// 在运行时启动后，对其进行管理
/// * 🚩`.nal`脚本预加载逻辑
/// * 🚩用户的运行时交互逻辑
/// * 🚩Websocket服务器逻辑
pub fn manage(mut nars: impl VmRuntime, args: &CliArgs) -> Result<()> {
    // TODO: 优化并行逻辑
    // TODO: 结合test_tools
    if_return! { args.no_user_input => Ok(()) }

    // 用户输入主循环
    'main: for io_result in ReadlineIter::default() {
        // 读取一行
        let line = io_result?;

        // 非空⇒解析出NAVM指令，作为输入执行
        if !line.trim().is_empty() {
            if let Ok(cmd) = Cmd::parse(&line).inspect_err(println_error) {
                let _ = nars.input_cmd(cmd).inspect_err(println_error);
            }
        }

        // 尝试拉取所有NAVM运行时输出
        while let Ok(Some(output)) = nars.try_fetch_output().inspect_err(println_error) {
            println!("{output:?}");
            if let Output::TERMINATED { .. } = output {
                println!("NAVM已终止运行，正在重启。。。");
                nars.terminate()?;
                break 'main; // ! 这个告诉Rust编译器，循环必将在此结束
            }
        }
    }
    // 正常运行结束
    Ok(())
}
