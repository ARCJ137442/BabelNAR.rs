//! 定义有关「命令行虚拟机」的抽象特征
//! * ✨核心内容
//!   * ⇄ 基于「进程通信」的消息互转
//!     * 📌核心IO流程：
//!       1. NAVM指令[`Cmd`] >>> 进程输入 >>> 子进程
//!       2. 子进程 >>> 进程输出 >>> NAVM输出[`Output`]
//!     * 🚩实现方式：两处转译器

use navm::{cmd::Cmd, vm::Output};

/// [`Cmd`]→进程输入 转译器
pub trait InputTranslator {
    fn translate_to_input(cmd: Cmd) -> String;
}

/// 进程输出→[`Output`]转译器
pub trait OutputTranslator {
    fn translate_from_output(output: String) -> Output;
}
