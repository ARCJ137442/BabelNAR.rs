//! ONA在「命令行运行时」的转译器
//! * 🎯维护与ONA Shell的交互
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

use super::dialect::parse as parse_narsese_ona;
use crate::runtime::TranslateError;
use anyhow::Result;
use narsese::conversion::string::impl_lexical::structs::ParseResult;
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};
use regex::Regex;
use util::{if_return, pipe, ResultBoost};

/// ONA的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「ONA Shell输入」
pub fn input_translate(cmd: Cmd) -> Result<String> {
    let content = match cmd {
        // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
        Cmd::NSE(..) => cmd.tail(),
        // CYC指令：运行指定周期数
        // ! ONA Shell同样是自动步进的
        Cmd::CYC(n) => n.to_string(),
        // VOL指令：调整音量
        Cmd::VOL(n) => format!("*volume={n}"),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        _ => return Err(TranslateError(format!("该指令类型暂不支持：{cmd:?}")).into()),
    };
    // 转译
    Ok(content)
}

/// ONA的「输出转译」函数
/// * 🎯用于将ONA Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
pub fn output_translate(content_raw: String) -> Result<Output> {
    // 特别处理：终止信号
    if_return! {
        content_raw.contains("Test failed.") => Ok(Output::TERMINATED { description: content_raw })
    }
    // 根据冒号分隔一次，然后得到「头部」
    let (head, tail) = content_raw.split_once(':').unwrap_or(("", ""));
    // 根据「头部」生成输出
    let output = match head.to_lowercase().as_str() {
        "answer" => Output::ANSWER {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            // * 🚩ONA会输出带有误导性的`Answer: None.`
            //   * 看起来是回答，实际上不是
            narsese: match content_raw.contains("Answer: None.") {
                true => None,
                false => try_parse_narsese(tail)
                    .ok_or_run(|e| println!("【ERR/{head}】在解析Narsese时出现错误：{e}")),
            },
            // 然后传入整个内容
            content_raw,
        },
        "derived" => Output::OUT {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: try_parse_narsese(tail)
                .ok_or_run(|e| println!("【ERR/{head}】在解析Narsese时出现错误：{e}")),
            // 然后传入整个内容
            content_raw,
        },
        "input" => Output::IN {
            content: content_raw,
        },
        "err" | "error" => Output::ERROR {
            description: content_raw,
        },
        // * 🚩对于「操作」的特殊语法
        _ if content_raw.contains("executed") => Output::EXE {
            operation: parse_operation_ona(&content_raw),
            content_raw,
        },
        // 若是连续的「头部」⇒识别为「未归类」类型
        _ if !content_raw.contains(char::is_whitespace) => Output::UNCLASSIFIED {
            r#type: head.into(),
            content: content_raw,
            // 不尝试捕获Narsese | 💭后续或许可以自动捕获？
            narsese: None,
        },
        // 其它
        _ => Output::OTHER {
            content: content_raw,
        },
    };
    // 返回
    Ok(output)
}

/// （ONA）从原始输出中解析操作
pub fn parse_operation_ona(content_raw: &str) -> Operation {
    println!("截获到操作：{content_raw:?}");
    Operation {
        // TODO: 有待分析
        operator_name: "UNKNOWN".into(),
        params: vec![],
    }
}

/// （尝试）从输出中解析出Narsese
/// * ❌【2024-03-27 22:01:18】目前引入[`anyhow::Error`]会出问题：不匹配/未满足的特征
pub fn try_parse_narsese(tail: &str) -> ParseResult {
    // 提取并解析Narsese字符串
    pipe! {
        tail
        // 重整
        => #{&}
        => reform_output_to_narsese
        // 解析
        => #{&}
        => parse_narsese_ona
        // 转换错误 | 解析失败⇒返回错误信息 | 返回None
        // => .transform_err(|err| format!("输出「OUT」解析失败：{err}"))
    }
}

/// 重整ONA输出到合法Narsese
/// * 🎯通过「重整→正确解析」的方式，实现初步输出解析兼容
/// * 🚩【2024-03-25 21:38:39】目前使用正则表达式[`regex`]库
/// * 🚩【2024-03-25 21:38:52】目前仅基于正则表达式做文本替换
/// * 📌参数`tail`不附带`Answer:`等部分
fn reform_output_to_narsese(out: &str) -> String {
    // 构造正则表达式（实现中只会编译一次） //
    // 匹配ONA输出中的「真值」
    let re_truth = Regex::new(r"Truth:\s*frequency=([0-9.]+),\s*confidence=([0-9.]+)").unwrap();
    // 匹配ONA输出的「创建时间」
    let re_creation_t = Regex::new(r"creationTime=([0-9.]+)\s+").unwrap();

    // 两次替换 //
    pipe! {
        out
        // 重建真值表达式
        => [re_truth.replace_all](_, |caps: &regex::Captures<'_>| {
            // * 第`0`个是正则表达式匹配的整个内容
            let f = &caps[1];
            let c = &caps[2];
            // 重建CommonNarsese合法的真值
            format!("%{f};{c}%")
        })
        => #{&}
        // 删去非必要的「创建时间」
        => [re_creation_t.replace_all](_, "")
        // 返回字符串 //
        => .into()
    }
}

/// 单元测试
#[cfg(test)]
mod test {
    use super::*;
    use util::asserts;

    /// 测试/正则重整
    #[test]
    fn test_regex_reform() {
        let inp = "<B --> C>. creationTime=2 Truth: frequency=1.000000, confidence=0.447514";
        let s = pipe! {
            inp
            => reform_output_to_narsese
            => .chars()
            => .into_iter()
            => .filter(|c|!c.is_whitespace())
            // => .collect::<String>() // ! ❌暂时不支持「完全限定语法」
        }
        .collect::<String>();

        // 断言
        asserts! {
            s => "<B-->C>.%1.000000;0.447514%",
        }
    }
}
