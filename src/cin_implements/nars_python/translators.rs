//! NARS-Python在「命令行运行时」的转译器
//! * 🎯维护与NARS-Python exe的交互
//! * 📌基于命令行输入输出的字符串读写
//! * ✨NAVM指令→字符串
//! * ✨字符串→NAVM输出
//!
//! ## 输出样例
//!
//! * `EXE: ^left based on desirability: 0.9`
//! * `PROCESSED GOAL: SentenceID:2081:ID ({SELF} --> [SAFE])! :|: %1.00;0.03%from SentenceID:2079:ID ({SELF} --> [SAFE])! :|: %1.00;0.00%,SentenceID:2080:ID ({SELF} --> [SAFE])! :|: %1.00;0.02%,`
//! * `PREMISE IS TRUE: ((*,{SELF}) --> ^right)`
//! * `PREMISE IS SIMPLIFIED ({SELF} --> [SAFE]) FROM (&|,({SELF} --> [SAFE]),((*,{SELF}) --> ^right))`

use super::format_in_nars_python;
use crate::runtime::TranslateError;
use anyhow::Result;
use narsese::lexical::Narsese;
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};

/// NARS-Python的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「NARS-Python输入」
pub fn input_translate(cmd: Cmd) -> Result<String> {
    let content = match cmd {
        // 使用「末尾」将自动格式化任务（可兼容「空预算」的形式）
        // * ✅【2024-03-26 01:44:49】目前采用特定的「方言格式」解决格式化问题
        Cmd::NSE(narsese) => format_in_nars_python(&Narsese::Task(narsese)),
        // CYC指令：运行指定周期数
        // ! NARS-Python Shell同样是自动步进的
        Cmd::CYC(n) => n.to_string(),
        // 其它类型
        // ! 🚩【2024-03-27 22:42:56】不使用[`anyhow!`]：打印时会带上一大堆调用堆栈
        _ => return Err(TranslateError(format!("该指令类型暂不支持：{cmd:?}")).into()),
    };
    // 转译
    Ok(content)
}

/// NARS-Python的「输出转译」函数
/// * 🎯用于将NARS-Python Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
pub fn output_translate(content: String) -> Result<Output> {
    // 根据冒号分隔一次，然后得到「头部」
    let head = content.split_once(':').unwrap_or(("", "")).0.to_lowercase();
    // 根据「头部」生成输出
    let output = match &*head {
        // TODO: 有待适配
        "answer" => Output::ANSWER {
            content_raw: content,
            // TODO: 有待捕获转译
            narsese: None,
        },
        "derived" => Output::OUT {
            content_raw: content,
            // TODO: 有待捕获转译
            narsese: None,
        },
        "input" => Output::IN { content },
        "exe" => Output::EXE {
            content_raw: content,
            // TODO: 有待捕获转译
            operation: Operation::new("UNKNOWN", [].into_iter()),
        },
        "err" | "error" => Output::ERROR {
            description: content,
        },
        _ => Output::OTHER { content },
    };
    // 返回
    Ok(output)
}
