//! CXinNARS.js在「命令行运行时」的转译器
//! * 🎯维护与CXinNARS.js Shell的交互
//! * 📌基于命令行输入输出的字符串读写
//! * ✨NAVM指令→字符串
//! * ✨字符串→NAVM输出
//!
//! ## 输出样例
//!
//! * `Input: <<(* x) --> ^left> ==> A>. Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000`
//! * `Derived: <<(* x) --> ^left> ==> <self --> good>>. Priority=0.245189 Truth: frequency=1.000000, confidence=0.810000`
//! * `Answer: <B --> C>. creationTime=2 Truth: frequency=1.000000, confidence=0.447514`
//! * `Answer: None.`
//! * `^deactivate executed with args`
//! * `^left executed with args (* {SELF})`
//! * `^left executed with args ({SELF} * x)`
//! * `decision expectation=0.616961 implication: <((<{SELF} --> [left_blocked]> &/ ^say) &/ <(* {SELF}) --> ^left>) =/> <{SELF} --> [SAFE]>>. Truth: frequency=0.978072 confidence=0.394669 dt=1.000000 precondition: <{SELF} --> [left_blocked]>. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=50`

use crate::runtimes::TranslateError;
use anyhow::Result;
use narsese::{
    conversion::string::impl_lexical::{format_instances::FORMAT_ASCII, structs::ParseResult},
    lexical::Narsese,
};
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};
use regex::Regex;
use util::{if_return, pipe};

/// CXinNARS.js的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「CXinNARS.js Shell输入」
/// * 📝[`IoProcess`]会自动将输入追加上换行符
pub fn input_translate(cmd: Cmd) -> Result<String> {
    let content = match cmd {
        // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
        Cmd::NSE(..) => cmd.tail(),
        // CYC指令：运行指定周期数
        Cmd::CYC(n) => n.to_string(),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        _ => return Err(TranslateError(format!("该指令类型暂不支持：{cmd:?}")).into()),
    };
    // 转译
    Ok(content)
}

/// CXinNARS.js的「输出转译」函数
/// * 🎯用于将CXinNARS.js Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
pub fn output_translate(content_raw: String) -> Result<Output> {
    // 特别处理：终止信号
    // * 📄"node:internal/modules/cjs/loader:1080\n  throw err"
    // * ❌【2024-03-28 09:00:23】似乎不可行：打开时的错误无法被捕捉
    if_return! {
        // 模块未找到
        content_raw.contains("Error: Cannot find module") => Ok(Output::TERMINATED { description: content_raw })
    }
    // 匹配「输出类型」的正则表达式
    // * ✅此处的「尾部」不会有前导空格（若识别出了「头部」）
    let line_r = Regex::new(r"\[(\w+)\]\s*(.*)").unwrap();
    let head;
    let tail;
    if let Some(captures) = line_r.captures(&content_raw) {
        head = captures[1].to_lowercase();
        tail = captures[2].to_owned();
    } else {
        head = String::new();
        tail = content_raw.clone();
    }
    // 根据「头部」生成输出
    let output = match head.as_str() {
        "answer" => Output::ANSWER {
            // 先提取其中的Narsese
            narsese: segment_narsese(&head, &tail),
            // 然后传入整个内容
            content_raw,
        },
        "in" => Output::IN {
            // 先提取其中的Narsese
            narsese: segment_narsese(&head, &tail),
            // 然后传入整个内容
            content: tail,
        },
        "out" => Output::OUT {
            // 先提取其中的Narsese
            narsese: segment_narsese(&head, &tail),
            // 然后传入整个内容
            content_raw: tail,
        },
        "comment" => Output::COMMENT { content: tail },
        "err" | "error" => Output::ERROR { description: tail },
        "exe" => Output::EXE {
            operation: parse_operation(&tail),
            content_raw: tail,
        },
        // 若是连续的「头部」⇒识别为「未归类」类型
        _ if !content_raw.contains(char::is_whitespace) => Output::UNCLASSIFIED {
            r#type: head,
            // 尝试自动捕获Narsese
            narsese: match try_segment_narsese(&tail) {
                Some(Ok(narsese)) => Some(narsese),
                _ => None,
            },
            content: tail,
        },
        // 其它
        _ => Output::OTHER {
            content: content_raw,
        },
    };
    // 返回
    Ok(output)
}

/// （CXinNARS.js）从原始输出中解析操作
pub fn parse_operation(content_raw: &str) -> Operation {
    #![allow(unused_variables)]
    todo!("CXinNARS.js暂不支持NAL-8")
}

fn segment_narsese(head: &str, tail: &str) -> Option<Narsese> {
    match try_segment_narsese(tail) {
        Some(Ok(narsese)) => Some(narsese),
        Some(Err(e)) => {
            println!("【{head}】在解析Narsese时出现错误：{e}");
            None
        }
        None => {
            println!("【{head}】未匹配到输出中的Narsese块");
            None
        }
    }
}

/// 分割 & 解析Narsese
/// * 🎯提供解析CXinNARS中Narsese的方法
///   * ❗不包含任何副作用（如打印）
/// * 🚩先通过正则表达式从模式`Narsese{{ 【Narsese内容】 }}【Narsese类型】`中分解出Narsese
/// * 🚩再通过标准ASCII解析器解析
pub fn try_segment_narsese(input: &str) -> Option<ParseResult> {
    let re_narsese = Regex::new(r"Narsese\{\{ (.+) \}\}").unwrap();
    pipe!(
        // 尝试从模式中提取Narsese
        re_narsese.captures(input)
        // 提取Narsese
        => .map(
            // 尝试解析Narsese
            |captures| try_parse_narsese(&captures[1])
        )
    )
}

/// （尝试）从输出中解析出Narsese
/// * ❌【2024-03-27 22:01:18】目前引入[`anyhow::Error`]会出问题：不匹配/未满足的特征
pub fn try_parse_narsese(narsese: &str) -> ParseResult {
    // 提取并解析Narsese字符串
    FORMAT_ASCII.parse(narsese)
}
