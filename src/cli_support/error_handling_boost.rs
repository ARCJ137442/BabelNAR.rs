//! 增强的快捷错误处理
//! * 🎯用于（在命令行）快速处理、输出各种错误

use crate::cli_support::io::output_print::OutputType;
use anyhow::{anyhow, Error};
use std::fmt::Debug;

/// 打印错误
/// * 🚩在标准错误中打印基于[`Debug`]的信息
/// * 🎯快速表示「报错而非panic」
/// * 🚩【2024-04-02 18:59:19】不建议使用：不应向用户打印大量错误堆栈信息
///   * ✨替代用法可参考[`crate::eprintln_cli`]
#[deprecated = "不建议使用：不应向用户打印大量错误堆栈信息"]
pub fn println_error(e: &impl Debug) {
    // ! 无法在此直接使用：macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths
    // * 🚩【2024-04-02 16:33:47】目前处理办法：直接展开
    println!("{}", OutputType::Error.format_line(&format!("{e:?}")));
}

/// 打印错误
/// * 🚩在标准错误中打印基于[`Debug`]的信息
/// * 🎯快速表示「报错而非panic」
/// * 🎯用于「传入所有权而非不可变引用」的[`Result::unwrap_or_else`]
/// * 🚩【2024-04-02 18:59:19】不建议使用：不应向用户打印大量错误堆栈信息
///   * ✨替代用法可参考[`crate::eprintln_cli`]
#[deprecated = "不建议使用：不应向用户打印大量错误堆栈信息"]
pub fn println_error_owned(e: impl Debug) {
    println!("{}", OutputType::Error.format_line(&format!("{e:?}")));
}

/// 将错误转换为[`anyhow::Error`]
/// * 🚩将错误转换为[`Debug`]信息，装入[`anyhow::Error`]中
/// * 🎯在线程通信中安全抛出未实现[`Send`]的[`std::sync::PoisonError`]
pub fn error_anyhow(e: impl Debug) -> Error {
    anyhow!("{e:?}")
}
