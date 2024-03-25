//! OpenNARS在「命令行运行时」的转译器
//! * 🎯维护与OpenNARS Shell的交互
//!   * https://github.com/ARCJ137442/opennars-304/blob/master/src/main/java/org/opennars/main/Shell.java
//! * 📌基于命令行输入输出的字符串读写
//! * ✨NAVM指令→字符串
//! * ✨字符串→NAVM输出
//!
//! ## 输出样例
//!
//! * `IN: <A --> B>. %1.00;0.90% {-1 : (-7995324758518856376,0)}`
//! * `OUT: <A --> B>. %1.00;0.90% {-1 : (-7995324758518856376,0)}`
//! * `Answer: <A --> C>. %1.00;0.81% {1584885193 : (-7995324758518856376,0);(-7995324758518856376,1)}`
//! * `EXE: $1.00;0.99;1.00$ ^left([{SELF}])=null`
//! * `ANTICIPATE: <{SELF} --> [SAFE]>`
//! * `CONFIRM: <{SELF} --> [SAFE]><{SELF} --> [SAFE]>`
//! * `DISAPPOINT: <{SELF} --> [SAFE]>`
//! * `Executed based on: $0.2904;0.1184;0.7653$ <(&/,<{SELF} --> [right_blocked]>,+7,(^left,{SELF}),+55) =/> <{SELF} --> [SAFE]>>. %1.00;0.53%`

use narsese::{
    conversion::string::impl_lexical::{format_instances::FORMAT_ASCII, structs::ParseResult},
    lexical::Narsese,
};
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};
use util::{ResultBoost, ResultS};

/// OpenNARS的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「OpenNARS Shell输入」
pub fn input_translate(cmd: Cmd) -> ResultS<String> {
    let content = match cmd {
        // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
        Cmd::NSE(..) => cmd.tail(),
        // CYC指令：运行指定周期数
        // ! OpenNARS Shell是自动步进的
        Cmd::CYC(n) => n.to_string(),
        // VOL指令：调整音量
        Cmd::VOL(n) => format!("*volume={n}"),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        _ => return Err(format!("该指令类型暂不支持：{cmd:?}")),
    };
    // 转译
    Ok(content)
}

/// OpenNARS的「输出转译」函数
/// * 🎯用于将OpenNARS Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
pub fn output_translate(content_raw: String) -> ResultS<Output> {
    // 根据冒号分隔一次，然后得到「头部」
    let (head, tail) = content_raw.split_once(':').unwrap_or(("", &content_raw));
    // 根据「头部」生成输出
    let output = match &*head.to_uppercase() {
        "IN" => Output::IN {
            content: content_raw,
        },
        "OUT" => {
            // 返回
            Output::OUT {
                // 先提取其中的Narsese | ⚠️借用了`content_raw`
                narsese: strip_parse_narsese(tail)
                    .ok_or_run(|e| println!("【ERR/{head}】在解析Narsese时出现错误：{e}")),
                // 然后传入整个内容
                content_raw,
            }
        }
        "ANSWER" => Output::ANSWER {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: strip_parse_narsese(tail)
                .ok_or_run(|e| println!("【ERR/{head}】在解析Narsese时出现错误：{e}")),
            // 然后传入整个内容
            content_raw,
        },
        "EXE" => Output::EXE {
            operation: parse_operation_opennars(&content_raw),
            content_raw,
        },
        "ANTICIPATE" => Output::ANTICIPATE {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: strip_parse_narsese(tail)
                .ok_or_run(|e| println!("【ERR/{head}】在解析Narsese时出现错误：{e}")),
            // 然后传入整个内容
            content_raw,
        },
        "ERR" | "ERROR" => Output::ERROR {
            description: content_raw,
        },
        // * 🚩利用OpenNARS常见输出「全大写」的特征，兼容「confirm」与「disappoint」
        upper if head == upper => Output::UNCLASSIFIED {
            r#type: head.to_string(),
            content: content_raw,
        },
        // 其它
        _ => Output::OTHER {
            content: content_raw,
        },
    };
    // 返回
    Ok(output)
}

/// 在OpenNARS输出中解析出「NARS操作」
///
/// TODO: 结合正则表达式进行解析
pub fn parse_operation_opennars(content_raw: &str) -> Operation {
    // use regex::Regex;
    Operation {
        // TODO: 有待捕获转译
        head: "UNKNOWN".into(),
        params: vec![content_raw.into()],
    }
}

/// 切分尾部字符串，并（尝试）从中解析出Narsese
fn strip_parse_narsese(tail: &str) -> ResultS<Narsese> {
    // 提取并解析Narsese字符串
    let narsese = tail
        // 去尾
        .rfind('{')
        // 截取 & 解析
        .map(|right_index| parse_narsese_opennars(&tail[..right_index]));
    // 提取解析结果
    match narsese {
        // 解析成功⇒提取 & 返回
        Some(Ok(narsese)) => Ok(narsese),
        // 解析失败⇒打印错误日志 | 返回None
        Some(Err(err)) => Err(format!("输出「OUT」解析失败：{err}")),
        // 未找到括号的情况
        None => Err("输出「OUT」解析失败：未找到「{」".into()),
    }
}

/// 以OpenNARS的语法解析出Narsese
/// * 🚩【2024-03-25 21:08:34】目前是直接调用ASCII解析器
///
/// TODO: 兼容OpenNARS特有之语法
/// * 📌重点在其简写的「操作」语法`(^left, {SELF}, x)` => `<(*, {SELF}, x) --> ^left>`
fn parse_narsese_opennars(input: &str) -> ParseResult {
    FORMAT_ASCII.parse(input)
}
