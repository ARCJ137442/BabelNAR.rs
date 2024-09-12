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
//! * `EXE: $0.11;0.33;0.57$ ^left([{SELF}, a, b, (/,^left,a,b,_)])=null`

use super::dialect::parse as parse_dialect_opennars;
use crate::runtimes::TranslateError;
use anyhow::Result;
use nar_dev_utils::ResultBoost;
use narsese::lexical::{Narsese, Term};
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};
use regex::Regex;

/// OpenNARS的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「OpenNARS Shell输入」
pub fn input_translate(cmd: Cmd) -> Result<String> {
    let content = match cmd {
        // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
        Cmd::NSE(..) => cmd.tail(),
        // CYC指令：运行指定周期数
        // ! OpenNARS Shell是自动步进的
        Cmd::CYC(n) => n.to_string(),
        // VOL指令：调整音量
        Cmd::VOL(n) => format!("*volume={n}"),
        // 注释 ⇒ 忽略 | ❓【2024-04-02 22:43:05】可能需要打印，但这样却没法统一IO（到处print的习惯不好）
        Cmd::REM { .. } => String::new(),
        // 退出码
        Cmd::EXI { .. } => "*exit".into(),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        // ! 🚩【2024-03-27 22:42:56】不使用[`anyhow!`]：打印时会带上一大堆调用堆栈
        _ => return Err(TranslateError::UnsupportedInput(cmd).into()),
    };
    // 转译
    Ok(content)
}

/// OpenNARS的「输出转译」函数
/// * 🎯用于将OpenNARS Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
pub fn output_translate(content_raw: String) -> Result<Output> {
    // 根据冒号分隔一次，然后得到「头部」
    let (head, tail) = content_raw.split_once(':').unwrap_or(("", &content_raw));
    let tail = tail.trim();
    // 根据「头部」生成输出
    let output = match &*head.to_uppercase() {
        "IN" => Output::IN {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: parse_narsese_opennars(head, tail)?,
            // 然后传入整个内容
            content: content_raw,
        },
        "OUT" => {
            // 返回
            Output::OUT {
                // 先提取其中的Narsese | ⚠️借用了`content_raw`
                narsese: parse_narsese_opennars(head, tail)?,
                // 然后传入整个内容
                content_raw,
            }
        }
        "ANSWER" => Output::ANSWER {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: parse_narsese_opennars(head, tail)?,
            // 然后传入整个内容
            content_raw,
        },
        "EXE" => Output::EXE {
            operation: parse_operation_opennars(tail.trim_start()),
            content_raw,
        },
        // ! 🚩【2024-03-27 19:40:37】现在将ANTICIPATE降级到`UNCLASSIFIED`
        "ANTICIPATE" => Output::UNCLASSIFIED {
            // 指定的头部
            r#type: "ANTICIPATE".to_string(),
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: try_parse_narsese(tail)
                .ok_or_run(|e| println!("【{head}】在解析Narsese「{tail}」时出现错误：{e}")),
            // 然后传入整个内容
            content: content_raw,
        },
        "ERR" | "ERROR" => Output::ERROR {
            description: content_raw,
        },
        // * 🚩【2024-05-09 14:41:11】目前为OpenNARS 1.5.8（定制版）专用
        "TERMINATED" | "EXITED" | "QUITTED" => Output::TERMINATED {
            description: content_raw,
        },
        // * 🚩利用OpenNARS常见输出「全大写」的特征，兼容「confirm」与「disappoint」
        upper if !head.is_empty() && head == upper => Output::UNCLASSIFIED {
            r#type: head.to_string(),
            content: content_raw,
            // 默认不捕获Narsese
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

/// （OpenNARS）从原始输出中解析Narsese
/// * 🎯用于结合`#[cfg]`控制「严格模式」
///   * 🚩生产环境下「Narsese解析出错」仅打印错误信息
#[cfg(not(test))]
pub fn parse_narsese_opennars(head: &str, tail: &str) -> Result<Option<Narsese>> {
    use nar_dev_utils::ResultBoost;
    // ! ↓下方会转换为None
    Ok(try_parse_narsese(tail)
        .ok_or_run(|e| println!("【{head}】在解析Narsese「{tail}」时出现错误：{e}")))
}

/// （OpenNARS）从原始输出中解析Narsese
/// * 🎯用于结合`#[cfg]`控制「严格模式」
///   * 🚩测试环境下「Narsese解析出错」会上抛错误
#[cfg(test)]
pub fn parse_narsese_opennars(_: &str, tail: &str) -> Result<Option<Narsese>> {
    // ! ↓下方会上抛错误
    Ok(Some(try_parse_narsese(tail)?))
}

/// 在OpenNARS输出中解析出「NARS操作」
/// * 📄`$0.11;0.33;0.57$ ^left([{SELF}, a, b, (/,^left,a,b,_)])=null`
/// * 🚩【2024-03-29 22:45:11】目前能提取出其中的预算值，但实际上暂且不需要
pub fn parse_operation_opennars(tail: &str) -> Operation {
    // * 构建正则表达式（仅一次编译）
    let r = Regex::new(r"(\$[0-9.;]+\$)\s*\^(\w+)\(\[(.*)\]\)=").unwrap();

    // 构建返回值（参数）
    let mut params = vec![];

    // 提取输出中的字符串
    let c = r.captures(tail);
    // let budget;
    let operator_name;
    let params_str;
    if let Some(c) = c {
        // 提取
        // budget = &c[1];
        operator_name = c[2].to_string();
        params_str = &c[3];
        // 尝试解析
        for param in params_str.split(", ") {
            match parse_term_from_operation(param) {
                Ok(term) => params.push(term),
                // ? 【2024-03-27 22:29:43】↓是否要将其整合到一个日志系统中去
                Err(e) => println!("【EXE】在解析Narsese时出现错误：{e}"),
            }
        }
    } else {
        operator_name = String::new();
    }

    // 返回
    Operation {
        operator_name,
        params,
    }
}

/// 从操作参数中解析出Narsese词项
fn parse_term_from_operation(term_str: &str) -> Result<Term> {
    // 首先尝试解析出Narsese
    let parsed = parse_dialect_opennars(term_str)?;
    // 其次尝试将其转换成Narsese词项
    parsed
        .try_into_term()
        .transform_err(TranslateError::error_anyhow)
}

/// 切分尾部字符串，并（尝试）从中解析出Narsese
/// * 🎯对OpenNARS中的「时间戳/证据基」做切分
///   * 📄`<{SELF} --> [satisfied]>! :|: %1.00;0.90% {1269408|1269408 : (-8058943780727144183,628)}`
///   * 🚩现在无需考虑：[`pest`]会自动忽略无关前缀
///   * ❌在「无证据基case」如`ANTICIPATE: <{powerup_bad_x} --> [seen]>`中报错：把`{`截掉了
/// * 📌此中`tail`已做好行切分
fn try_parse_narsese(tail: &str) -> Result<Narsese> {
    // 提取并解析Narsese字符
    // 提取解析结果
    let narsese = parse_dialect_opennars(tail);
    match narsese {
        // 解析成功⇒提取 & 返回
        Ok(narsese) => Ok(narsese),
        // 解析失败⇒打印错误日志 | 返回None
        Err(err) => Err(TranslateError::from(err).into()),
    }
}
