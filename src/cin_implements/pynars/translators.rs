//! PyNARS在「命令行运行时」的转译器
//! * 🎯维护与PyNARS的交互
//! * 📌基于命令行输入输出的字符串读写
//! * ✨NAVM指令→字符串
//! * ✨字符串→NAVM输出
//!
//! ## 输出样例
//!
//! * 📄`\u{1b}[90mInput: \u{1b}[39m\u{1b}[48;2;124;10;10m 0.90 \u{1b}[49m\u{1b}[48;2;10;124;10m 0.90 \u{1b}[49m\u{1b}[48;2;10;10;137m 1.00 \u{1b}[49m\u{1b}[36mIN    :\u{1b}[39m<A-->C>?\r\n`
//! * 📄`\u{1b}[90mInput: \u{1b}[39m    \u{1b}[49m    \u{1b}[49m    \u{1b}[49m\u{1b}[34mINFO  :\u{1b}[39m\u{1b}[38;5;249mRun 5 cycles.\u{1b}[39m\r\n`
//! * 📄`\u{1b}[48;2;106;10;10m 0.75 \u{1b}[49m\u{1b}[48;2;10;41;10m 0.25 \u{1b}[49m\u{1b}[48;2;10;10;102m 0.72 \u{1b}[49m\u{1b}[33mOUT   :\u{1b}[39m<C-->A>. %1.000;0.448%\r\n`
//! * 📄`\u{1b}[48;2;134;10;10m 0.98 \u{1b}[49m\u{1b}[48;2;10;124;10m 0.90 \u{1b}[49m\u{1b}[48;2;10;10;125m 0.90 \u{1b}[49m\u{1b}[32mANSWER:\u{1b}[39m<A-->C>. %1.000;0.810%\r\n`
//! * 📄`    \u{1b}[49m    \u{1b}[49m    \u{1b}[49m\u{1b}[32mEXE   :\u{1b}[39m<(*, 0)-->^op> = $0.022;0.232;0.926$ <(*, 0)-->^op>! :\\: %1.000;0.853% {7: 2, 0, 1}\r\n`

use crate::runtimes::TranslateError;
use anyhow::{anyhow, Result};
use narsese::{
    api::ExtractTerms,
    conversion::string::{
        impl_enum::format_instances::FORMAT_ASCII as FORMAT_ASCII_ENUM,
        impl_lexical::format_instances::FORMAT_ASCII,
    },
    lexical::{Narsese, Term},
};
use navm::{
    cmd::Cmd,
    output::{Operation, Output},
};
use regex::{Captures, Regex};
use util::{pipe, JoinTo};

/// PyNARS的「输入转译」函数
/// * 🎯用于将统一的「NAVM指令」转译为「PyNARS输入」
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
        // REG指令：注册操作符
        // * 📄Input: /register name
        //  * `Operator ^name was successfully registered without code`
        Cmd::REG { name, .. } => format!("/register {name}"),
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

/// 预处理
/// * 🎯去掉输出字串中语义无关的杂项
///   * 📄ANSI转义序列
pub fn preprocess(s: &str) -> String {
    // ! `\e` => `\u{1b}`
    let re = Regex::new(r"\u{1b}\[[0-9;]*m").unwrap();
    pipe! {
        s
        // 去掉ANSI转义序列
        => [re.replace_all](_, "")
        // 去掉前后缀空白符
        => .trim()
        // 转换为字符串
        => .to_string()
    }
}

/// 尝试获取输出类型（「头」文本）
/// * 🚩输入：[`preprocess`]预处理后的文本
/// * 🎯尝试获取「类型」字符串，若无则返回[`None`]
fn try_get_output_type(preprocessed: &str) -> Option<String> {
    // 截获输出类型，忽略前边的预算值
    let re2 = Regex::new(r"[0-9\s|]*(\w+)\s*:").unwrap();
    pipe! {
        preprocessed
        // 捕获
        => [re2.captures](_)
        // 转换为字符串
        => .map(|captures|captures[1].into())
    }
}

/// 尝试获取输出中的Narsese
/// * 🚩输入：[`preprocess`]预处理后的文本
/// * 🎯尝试获取「Narsese」值
fn try_get_narsese(preprocessed: &str) -> Result<Narsese> {
    // 删去无用内容，并替换成预算值 | 三个预算+一个头
    // * 🚩【2024-03-30 00:15:24】开头必须是`[^0-9.]*`，以避免吃掉预算值「`0.98`⇒`8`」💥
    let re_trim_and_budget =
        Regex::new(r"^[^0-9.]*([0-9.]+)[\s|]+([0-9.]+)[\s|]+([0-9.]+)[\s|]+\w+\s*:\s*").unwrap();
    let trimmed = re_trim_and_budget
        // 删去其中无用的内容，并重整其中的预算值 //
        .replace(preprocessed, |s: &Captures| {
            // 创建「预算值」字串
            let mut budget = FORMAT_ASCII_ENUM.task.budget_brackets.0.to_string();

            // 构造迭代器
            let mut s = s.iter();
            s.next(); // 消耗掉第一个「被匹配到的字符串」

            // 遍历所有匹配到的「预算内容」
            s.flatten()
                // 全部转换成「字串切片」
                .map(|c| c.as_str())
                // 拼接到已预置好「预算起始括弧」的字符串中
                .join_to(&mut budget, FORMAT_ASCII_ENUM.task.budget_separator);

            // 最后加入并返回
            budget + FORMAT_ASCII_ENUM.task.budget_brackets.1
        })
        .to_string();
    let parsed_narsese = FORMAT_ASCII.parse(&trimmed)?;
    Ok(parsed_narsese)
}

/// 获取输出中的Narsese
/// * 🎯根据「测试环境」与「生产环境」启用不同的模式
///   * 🚩测试环境中「解析失败」会报错（成功了总返回[`Some`]）
///   * 🚩生产环境中「解析失败」仅提示（然后返回[`None`]）
#[cfg(not(test))]
fn get_narsese(preprocessed: &str) -> Result<Option<Narsese>> {
    use util::ResultBoost;
    // * 🚩解析失败⇒提示⇒返回[`None`]
    Ok(try_get_narsese(preprocessed).ok_or_run(|e| println!("尝试解析Narsese错误：{e}")))
}

/// 获取输出中的Narsese
/// * 🎯根据「测试环境」与「生产环境」启用不同的模式
///   * 🚩测试环境中「解析失败」会报错（成功了总返回[`Some`]）
///   * 🚩生产环境中「解析失败」仅提示（然后返回[`None`]）
#[cfg(test)]
fn get_narsese(preprocessed: &str) -> Result<Option<Narsese>> {
    // * 🚩解析失败会上抛，成功了总是返回[`Some`]
    Ok(Some(try_get_narsese(preprocessed)?))
}

/// 尝试获取输出中的「Narsese操作」
/// * 🎯截获PyNARS中的「EXE」部分
/// * 📄`    \u{1b}[49m    \u{1b}[49m    \u{1b}[49m\u{1b}[32mEXE   :\u{1b}[39m<(*, 0)-->^op> = $0.022;0.232;0.926$ <(*, 0)-->^op>! :\\: %1.000;0.853% {7: 2, 0, 1}\r\n`
/// * 📄"executed: arguments=<Terms: (0, 1, 2, 3)>, task=$0.000;0.339;0.950$ <(*, 0, 1, 2, 3)-->^op>! %1.000;0.853% {None: 7, 4, 5}, memory=<Memory: #items=21, #buckets=100>. the \"task\" will be returned\r\n"
/// * 📄`    \u{1b}[49m    \u{1b}[49m    \u{1b}[49m\u{1b}[32mEXE   :\u{1b}[39m<(*, 0, 1, 2, 3)-->^op> = $0.000;0.339;0.950$ <(*, 0, 1, 2, 3)-->^op>! %1.000;0.853% {None: 7, 4, 5}\r\n`
/// * 📄"executed: arguments=<Terms: (0)>, task=$0.220;0.232;0.926$ <(*, 0)-->^op>! :\\: %1.000;0.853% {7: 2, 0, 1}, memory=<Memory: #items=8, #buckets=100>. the \"task\" will be returned\r\n"
fn try_get_operation(preprocessed: &str) -> Result<Operation> {
    let re_operation = Regex::new(r"EXE\s*:\s*(.+) = ").unwrap();
    let op = re_operation
        .captures(preprocessed)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();
    let op = FORMAT_ASCII.parse(op).unwrap().try_into_term().unwrap();
    match op {
        // * 📄`<(*, 0)-->^op>`
        Term::Statement {
            subject, predicate, ..
        } => {
            // 从主词提取操作参数
            let params = subject.extract_terms_to_vec();
            // 从谓词提取操作名
            let operator_name = match *predicate {
                Term::Atom { name, .. } => name,
                _ => return Err(anyhow!("陈述谓词不是原子词项")),
            };
            Ok(Operation {
                operator_name,
                params,
            })
        }
        _ => Err(anyhow::anyhow!("无效的「操作表示」词项：{op:?}")),
    }
}

/// 获取输出中的「Narsese操作」
/// * 🎯获取名称及其参数
/// * 🎯根据「测试环境」与「生产环境」启用不同的模式
///   * 🚩测试环境中「解析失败」会报错（成功了总返回[`Some`]）
///   * 🚩生产环境中「解析失败」仅提示（然后返回[`None`]）
#[cfg(not(test))]
fn get_operation(preprocessed: &str) -> Operation {
    // * 🚩解析失败仅提示，然后返回「空操作」
    try_get_operation(preprocessed).unwrap_or_else(|e| {
        println!("尝试从「{preprocessed}」解析Narsese操作错误：{e}");
        // 空操作
        Operation {
            operator_name: "".into(),
            params: vec![],
        }
    })
}

/// 获取输出中的Narsese
/// * 🎯根据「测试环境」与「生产环境」启用不同的模式
///   * 🚩测试环境中「解析失败」会报错（成功了总返回[`Some`]）
///   * 🚩生产环境中「解析失败」仅提示（然后返回[`None`]）
#[cfg(test)]
fn get_operation(preprocessed: &str) -> Operation {
    // * 🚩解析失败会直接报错
    try_get_operation(preprocessed)
        .unwrap_or_else(|e| panic!("无法从「{preprocessed}」解析出Narsese操作：{e}"))
}

/// PyNARS的「输出转译」函数
/// * 🎯用于将PyNARS的输出（字符串）转译为「NAVM输出」
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
    // 预处理 | 利用变量遮蔽，在输出中屏蔽ANSI转义序列
    let content = preprocess(&content);
    // 根据冒号分隔一次，然后得到「头部」
    let head = pipe! {
        &content
        // 获取输出类型
        => try_get_output_type
        // 统一转成小写 | ✅无需`trim`：在`try_get_output_type`中使用正则表达式保证
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
            narsese: get_narsese(&content)?,
            content_raw: content,
        },
        "achieved" => Output::ACHIEVED {
            narsese: get_narsese(&content)?,
            content_raw: content,
        },
        "out" => Output::OUT {
            narsese: get_narsese(&content)?,
            content_raw: content,
        },
        "input" | "in" => Output::IN {
            narsese: get_narsese(&content)?,
            content,
        },
        "info" => Output::INFO { message: content },
        "exe" => Output::EXE {
            operation: get_operation(&content),
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

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试/尝试获取输出
    #[test]
    fn test_try_get_output() {
        test("\u{1b}[48;2;110;10;10m 0.78 \u{1b}[49m\u{1b}[48;2;10;41;10m 0.25 \u{1b}[49m\u{1b}[48;2;10;10;125m 0.90 \u{1b}[49m\u{1b}[33mOUT   :\u{1b}[39m<A-->C>. %1.000;0.810%\r\n");
        test("|0.80|0.50|0.95| IN    : A. %1.000;0.900%");
        test("\u{1b}[90mInput: \u{1b}[39m\u{1b}[48;2;124;10;10m 0.90 \u{1b}[49m\u{1b}[48;2;10;124;10m 0.90 \u{1b}[49m\u{1b}[48;2;10;10;137m 1.00 \u{1b}[49m\u{1b}[36mIN    :\u{1b}[39m<A-->C>?\r\n");
        test("0.98  0.90  0.90 ANSWER:<A-->C>. %1.000;0.810%");

        fn test(inp: &str) {
            let preprocessed = preprocess(inp);
            let _ = " 0.78  0.25  0.90 OUT   :<A-->C>. %1.000;0.810%\r\n";
            dbg!(&preprocessed);
            let t = try_get_output_type(&preprocessed);
            dbg!(&t);

            // 删去无用内容，并替换成预算值 | 三个预算+一个头
            dbg!(try_get_narsese(&preprocessed).expect("Narsese解析失败！"));
        }
    }

    /// 测试/尝试获取操作
    #[test]
    fn test_try_get_operation() {
        test("    \u{1b}[49m    \u{1b}[49m    \u{1b}[49m\u{1b}[32mEXE   :\u{1b}[39m<(*, 0)-->^op> = $0.022;0.232;0.926$ <(*, 0)-->^op>! :\\: %1.000;0.853% {7: 2, 0, 1}\r\n");
        test("    \u{1b}[49m    \u{1b}[49m    \u{1b}[49m\u{1b}[32mEXE   :\u{1b}[39m<(*, 0, 1, 2, 3)-->^op> = $0.000;0.339;0.950$ <(*, 0, 1, 2, 3)-->^op>! %1.000;0.853% {None: 7, 4, 5}\r\n");
        fn test(inp: &str) {
            let inp = preprocess(inp);
            let op = try_get_operation(&inp).unwrap();
            dbg!(op);
        }
    }
}
