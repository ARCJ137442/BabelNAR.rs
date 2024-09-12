//! OpenJunars在「命令行运行时」的转译器
//! * 📌基于命令行输入输出的字符串读写
//! * ✨NAVM指令→字符串
//! * ✨字符串→NAVM输出
//!
//! TODO: 🚧自OpenNARS复制而来，一些地方需要特别适配

use crate::runtimes::TranslateError;
use anyhow::Result;
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};

/// OpenJunars的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「OpenJunars Shell输入」
pub fn input_translate(cmd: Cmd) -> Result<String> {
    let content = match cmd {
        // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
        Cmd::NSE(..) => cmd.tail(),
        // CYC指令：运行指定周期数
        Cmd::CYC(n) => format!(":c {n}"),
        // 注释 ⇒ 忽略 | ❓【2024-04-02 22:43:05】可能需要打印，但这样却没法统一IO（到处print的习惯不好）
        Cmd::REM { .. } => String::new(),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        // ! 🚩【2024-03-27 22:42:56】不使用[`anyhow!`]：打印时会带上一大堆调用堆栈
        _ => return Err(TranslateError::UnsupportedInput(cmd).into()),
    };
    // 转译
    Ok(content)
}

/// OpenJunars的「输出转译」函数
/// * 🎯用于将OpenJunars Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
pub fn output_translate(content: String) -> Result<Output> {
    // 根据冒号分隔一次，然后得到「头部」
    let head = content.split_once(':').unwrap_or(("", "")).0.to_lowercase();
    // 根据「头部」生成输出
    let output = match &*head {
        "answer" => Output::ANSWER {
            // TODO: 有待捕获转译
            narsese: None,
            content_raw: content,
        },
        "out" => Output::OUT {
            // TODO: 有待捕获转译
            narsese: None,
            content_raw: content,
        },
        "in" => Output::IN {
            // TODO: 有待捕获转译
            narsese: None,
            content,
        },
        "exe" => Output::EXE {
            // TODO: 有待捕获转译
            operation: Operation::new("UNKNOWN", []),
            content_raw: content,
        },
        "err" | "error" => Output::ERROR {
            description: content,
        },
        _ => Output::OTHER { content },
    };
    // 返回
    Ok(output)
}
