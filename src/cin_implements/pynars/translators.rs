//! ONA在「命令行运行时」的转译器
//! * 🎯维护与ONA Shell的交互
//! * 📌基于命令行输入输出的字符串读写
//! * ✨NAVM指令→字符串
//! * ✨字符串→NAVM输出

use crate::runtime::TranslateError;
use anyhow::Result;
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};
use regex::Regex;
use util::pipe;

/// ONA的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「ONA Shell输入」
pub fn input_translate(cmd: Cmd) -> Result<String> {
    let content = match cmd {
        // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
        Cmd::NSE(..) => cmd.tail(),
        // CYC指令：运行指定周期数
        // * 📌PyNARS需要手动指定步进数
        Cmd::CYC(n) => n.to_string(),
        // VOL指令：调整音量
        // ! ⚠️该指令仅适用于`ConsolePlus`
        Cmd::VOL(n) => format!("/volume {n}"),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        // ! 🚩【2024-03-27 22:42:56】不使用[`anyhow!`]：打印时会带上一大堆调用堆栈
        _ => return Err(TranslateError(format!("该指令类型暂不支持：{cmd:?}")).into()),
    };
    // 转译
    Ok(content)
}

/// 尝试获取输出类型（「头」文本）
fn try_get_output_type(inp: &str) -> Option<String> {
    // ! `\e` => `\u{1b}`
    let re = Regex::new(r"\u{1b}\[[0-9;]*m").unwrap();
    // let inp = "\u{1b}[48;2;110;10;10m 0.78 \u{1b}[49m\u{1b}[48;2;10;41;10m 0.25 \u{1b}[49m\u{1b}[48;2;10;10;125m 0.90 \u{1b}[49m\u{1b}[33mOUT   :\u{1b}[39m<A-->C>. %1.000;0.810%\r\n";
    // 三个预算+一个头
    let re2 = Regex::new(r"([0-9.]+)\s+([0-9.]+)\s+([0-9.]+)\s+(\w+)\s*:").unwrap();
    let replaced = pipe! {
        inp
        => [re.replace_all](_, "")
        => .to_string()
    };
    let _ = " 0.78  0.25  0.90 OUT   :<A-->C>. %1.000;0.810%\r\n";
    dbg!(&replaced);
    let captured = dbg!(pipe! {
        replaced
        => #{&}
        => [re2.captures](_)
    });
    captured.map(|c| c[4].to_string())
}

/// ONA的「输出转译」函数
/// * 🎯用于将ONA Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
/// # * 去除其中的ANSI转义序列，如：`\e[39m` # 并去除前后多余空格
/// local actual_line::String = strip(replace(line, r"\e\[[0-9;]*m" => ""))
/// #= 去除后样例：
/// * `0.70  0.25  0.60 OUT   :<B==><(*, x)-->^left>>. %1.000;0.200%`
/// * INFO  : Loading RuleMap <LUT.pkl>...
/// * EXE   :<(*, x)-->^left> = $0.016;0.225;0.562$ <(*, x)-->^left>! %1.000;0.125% {None: 3, 1, 2}
/// * EXE   :<(*, 1, 2, 3)-->^left> = $0.000;0.225;0.905$ <(*, 1, 2, 3)-->^left>! %1.000;0.287% {None: 2, 1, 0}
/// * EXE   :<(*, {SELF}, [good])-->^f> = $0.026;0.450;0.905$ <(*, {SELF}, [good])-->^f>! %1.000;0.810% {None: 2, 1}
/// =#
///
/// # * 特殊处理「信息」"INFO"：匹配「INFO」开头的行 样例：`INFO  : Loading RuleMap <LUT.pkl>...`
pub fn output_translate(content: String) -> Result<Output> {
    // 根据冒号分隔一次，然后得到「头部」
    let head = pipe! {
        &content
        => try_get_output_type
        => .map(|s|s.to_lowercase())
    };
    // 取切片 | ❌不能使用闭包，因为闭包无法返回引用
    let head = match &head {
        Some(s) => s,
        None => "",
    };
    // 根据「头部」生成输出
    let output = match head {
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
