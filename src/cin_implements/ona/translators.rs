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
//!
//! ## 其它杂项
//!
//! 💭【2024-03-29 16:58:01】ONA中「注册操作」可以被翻译成`*setopname 操作ID ^操作符名`的形式
//! * ⚠️但需要自行保证「操作ID」不重复
//! * 📄`*setopname 1 ^left`
//! * 🔗参见<https://github.com/opennars/OpenNARS-for-Applications/blob/master/misc/Python/OpenNARS_for_Applications.ipynb>

use super::dialect::parse as parse_dialect_ona;
use crate::{
    cin_implements::ona::{fold_pest_compound, DialectParser, Rule},
    cli_support::io::output_print::OutputType,
    runtimes::TranslateError,
};
use anyhow::Result;
#[cfg(not(test))]
use nar_dev_utils::OptionBoost;
use nar_dev_utils::{if_return, pipe};
use narsese::lexical::{Narsese, Term};
use navm::{
    cmd::Cmd,
    output::{type_names::ANTICIPATE, Operation, Output},
};
use pest::Parser;
use regex::{Captures, Regex};

/// ONA已内置的操作列表
/// * 🎯避免「重复操作注册」
/// * 🎯【2024-04-07 23:12:56】兼容PyNARS的同时，不将自身搞崩
/// * 📄首次出现场景：Matriangle Websocket服务器链接
/// * 🔗参考：<https://github.com/opennars/OpenNARS-for-Applications/blob/2c6b7b966aa627818cb3eb4b2c0ae360bfada8c3/src/Shell.c#L37>
pub const OPERATOR_NAME_LIST: &[&str] = &[
    "left",
    "right",
    "up",
    "down",
    "say",
    "pick",
    "drop",
    "go",
    "activate",
    "deactivate",
];

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
        // REG指令：注册操作
        Cmd::REG { name } => match OPERATOR_NAME_LIST.contains(&name.as_str()) {
            true => String::new(),
            false => format!("*setopname {} ^{name}", hash_operator_id(&name)),
        },
        // 注释 ⇒ 忽略 | ❓【2024-04-02 22:43:05】可能需要打印，但这样却没法统一IO（到处print的习惯不好）
        Cmd::REM { .. } => String::new(),
        // 退出 ⇒ 无效输入 | // ! 🚩故意使用ONA中会「报错退出」的输入，强制ONA shell退出（其后不会再接收输入）
        Cmd::EXI { .. } => "*quit".into(),
        // 其它类型
        // * 📌【2024-03-24 22:57:18】基本足够支持
        _ => return Err(TranslateError::UnsupportedInput(cmd).into()),
    };
    // 转译
    Ok(content)
}

/// 🔗参见<https://vscode.dev/github/ARCJ137442/OpenNARS-for-Applications/blob/master/src/Config.h#L112>
/// ```c
/// //Maximum amount of operations which can be registered
/// #define OPERATIONS_MAX 10
/// ```
static mut NEXT_OPERATOR_ID: usize = 0;
const OPERATIONS_MAX: usize = 10;

/// 从「操作名」到「唯一操作数值ID」
/// * 🎯用于保证操作ID不重复
///   * 📌尽可能保证一一映射：操作名（字符串） ↔ 操作ID（无符号整数）
///
/// * 🚩现在因ONA的「操作符数量限制」不推荐直接用散列函数
///   * 📄取余后的已知散列冲突：`^op = ^op2`
/// * 🚩【2024-03-29 17:13:41】目前使用「循环取余」尽可能避免「索引越界」
///   * ⚠️仍然避免不了「操作重复」
///   * 🚩【2024-03-29 17:19:43】目前采用「及早失败」策略，"let it crash"
///
/// * 📌ONA中「操作ID」的范围：1..OPERATIONS_MAX
fn hash_operator_id(_: &str) -> usize {
    // ! 静态可变量是不安全方法：无法避免数据竞争
    // SAFETY: 实际使用时只需保证
    unsafe {
        NEXT_OPERATOR_ID += 1;
        NEXT_OPERATOR_ID %= OPERATIONS_MAX;
        NEXT_OPERATOR_ID + 1
    }
    // ! 🚩【2024-03-29 17:12:28】弃用
    // use std::hash::{DefaultHasher, Hash, Hasher};
    // let mut hasher = DefaultHasher::new();
    // op_name.hash(&mut hasher);
    // (hasher.finish() % 10) as usize
}

/// 测试/获取注册的操作符id
#[test]
fn test_hash_operator_id() {
    dbg!([
        hash_operator_id("left"),
        hash_operator_id("left"),
        hash_operator_id("right"),
        hash_operator_id("op"),
        hash_operator_id("op2"),
        hash_operator_id("oq"),
    ]);
}

/// ONA的「输出转译」函数
/// * 🎯用于将ONA Shell的输出（字符串）转译为「NAVM输出」
/// * 🚩直接根据选取的「头部」进行匹配
/// 超参数：严格模式
/// * 🚩测试环境下「输出Narsese解析失败」会上报错误
/// TODO: 解决`Input: <(* {SELF}) --> ^left>. :|: occurrenceTime=119 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000`
pub fn output_translate(content_raw: String) -> Result<Output> {
    // 特别处理
    if_return! {
        // 终止信号
        content_raw.contains("Test failed.") => Ok(Output::TERMINATED { description: content_raw })
        // 操作索引越界
        // * 📄`Operator index out of bounds, it can only be between 1 and OPERATIONS_MAX!`
        content_raw.contains("Operator index out of bounds") => Ok(Output::ERROR { description: content_raw })
    }
    // 根据冒号分隔一次，然后得到「头部」
    let (head, tail) = content_raw.split_once(':').unwrap_or(("", ""));
    // 根据「头部」生成输出
    // * 🚩此处不直接使用NAVM输出中的「头部字串常量」主要考虑是「此为ONA特有」
    let output = match head.to_lowercase().as_str() {
        // 回答，但排除「似是而非」的`Answer: None.`
        // * 🚩ONA会输出带有误导性的`Answer: None.`
        //   * 看起来是回答，实际上不是
        // * 🚩【2024-04-11 23:01:50】现在将`Answer: None.`开除出「回答」的输出格式
        "answer" if !content_raw.contains("Answer: None.") => Output::ANSWER {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: parse_narsese_ona(head, tail)?,
            // 然后传入整个内容
            content_raw,
        },
        "derived" => Output::OUT {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: parse_narsese_ona(head, tail)?,
            // 然后传入整个内容
            content_raw,
        },
        "input" => Output::IN {
            // 先提取其中的Narsese | ⚠️借用了`content_raw`
            narsese: parse_narsese_ona(head, tail)?,
            content: content_raw,
        },
        "err" | "error" => Output::ERROR {
            description: content_raw,
        },
        // * 🚩对于「操作」的特殊语法
        // * 🚩【2024-04-02 18:45:17】仅截取`executed with args`，不截取`executed by NAR`
        _ if content_raw.contains("executed with args") => Output::EXE {
            operation: parse_operation_ona(&content_raw)?,
            content_raw,
        },
        // * 🚩对于「决策预期→ANTICIPATE」的特殊语法
        // * 🚩【2024-04-02 18:45:17】仅截取`executed with args`，不截取`executed by NAR`
        _ if content_raw.contains("decision expectation=") => Output::UNCLASSIFIED {
            r#type: ANTICIPATE.into(),
            narsese: parse_anticipate_ona(&content_raw)?,
            content: content_raw,
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
/// * 📄`^deactivate executed with args`
/// * 📄`^left executed with args (* {SELF})`
/// * 📄`^left executed with args ({SELF} * x)`
/// * ❌`right executed by NAR`
pub fn parse_operation_ona(content_raw: &str) -> Result<Operation> {
    // 匹配ONA输出中的「操作」⇒转换 | 操作名 | 操作参数（Narsese复合词项⇒提取组分，变成字符串）
    let re_operation = Regex::new(r"\^([^\s]+)\s*executed with args\s*(.*)").unwrap();
    let captures = re_capture(&re_operation, content_raw.trim())?;
    // ! 即便是测试环境下，也有可能是[`None`]（但只在测试环境下返回[`Err`]并报错）
    match captures {
        Some(captures) => {
            // 操作名称
            let operator_name = captures[1].into();
            // 操作参数
            let params = match captures[2].trim() {
                // 空字串⇒空参数组
                "" => vec![],
                // 否则⇒作为复合词项解析
                term_str => pipe! {
                    // 获取操作参数字符串
                    term_str
                    // 基于[`pest`]的词法解析
                    => DialectParser::parse(Rule::narsese, _)
                    => {?}# // 后缀语法：抛出错误/解包
                    => .next()
                    => .unwrap()
                    // 折叠到「词法Narsese」
                    => fold_pest_compound
                    => {?}# // 后缀语法：抛出错误/解包
                    // 提取出词项
                    => extract_params
                },
            };
            // 返回
            Ok(Operation {
                operator_name,
                params,
            })
        }
        // 「未知操作」的占位符 | 仅在生产环境中返回
        None => Ok(Operation {
            operator_name: "UNKNOWN".into(),
            params: vec![],
        }),
    }
}

/// （ONA）从原始输出中解析「ANTICIPATE」预期
/// * 🚩通过「前缀正则截取」分割并解析随后Narsese获得
/// * 📄`"decision expectation=0.502326 implication: <((<{SELF} --> [good]> &/ <a --> b>) &/ <(* {SELF}) --> ^left>) =/> <{SELF} --> [good]>>. Truth: frequency=0.872512 confidence=0.294720 dt=12.000000 precondition: (<{SELF} --> [good]> &/ <a --> b>). :|: Truth: frequency=1.000000 confidence=0.360000 occurrenceTime=35124\n"`
/// * 📄`"decision expectation=0.578198 implication: <(a &/ ^left) =/> g>. Truth: frequency=1.000000 confidence=0.241351 dt=1.000000 precondition: a. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=4\n"`
pub fn parse_anticipate_ona(content_raw: &str) -> Result<Option<Narsese>> {
    // 正则捕获
    let re_operation = Regex::new(r"implication:\s*(.*)\s*dt=").unwrap();
    let captures = re_capture(&re_operation, content_raw.trim())?;
    match captures {
        Some(captures) => {
            // 获取内容
            let narsese_content = captures[1].to_string();
            // 解析
            let parse_result =
                parse_narsese_ona(ANTICIPATE, narsese_content.trim()).inspect_err(|e| {
                    OutputType::Error.eprint_line(&format!("ONA「预期」解析失败：{e}"));
                });
            // 返回
            parse_result
        }
        // 截取失败的情形
        None => {
            OutputType::Error.eprint_line(&format!("ONA「预期」正则捕获失败：{content_raw:?}"));
            Ok(None)
        }
    }
}
/// 操作参数提取
/// * 🎯从一个解析出来的词项中提取出「操作参数列表」
/// * 🚩测试环境中仅允许「复合词项」被解包
#[cfg(test)]
fn extract_params(params: Term) -> Vec<Term> {
    match params {
        Term::Compound { terms, .. } => terms,
        _ => unreachable!("ONA的「操作参数」只能由「复合词项」承载"),
    }
}

/// 操作参数提取
/// * 🎯从一个解析出来的词项中提取出「操作参数列表」
/// * 🚩测试环境中仅允许「复合词项」被解包
/// * 🚩生产环境中允许多种词项形式（原子词项⇒仅含其自身的参数列表）
#[cfg(not(test))]
fn extract_params(params: Term) -> Vec<Term> {
    match params {
        Term::Compound { terms, .. } => terms,
        Term::Set { terms, .. } => terms,
        Term::Statement {
            subject, predicate, ..
        } => vec![*subject, *predicate],
        Term::Atom { .. } => vec![params],
    }
}

/// 正则捕获
/// * 🎯用于在测试环境中启用「严格模式」（无法匹配⇒报错）
/// * 🚩测试环境中会上抛错误
/// * 🚩生产环境中仅打印错误消息
#[cfg(not(test))]
fn re_capture<'a>(re: &'a Regex, haystack: &'a str) -> Result<Option<Captures<'a>>> {
    Ok(re
        .captures(haystack)
        .inspect_none(|| println!("使用正则表达式「{re}」无法捕获「{haystack}」")))
}

/// 正则捕获
/// * 🎯用于在测试环境中启用「严格模式」（无法匹配⇒报错）
/// * 🚩测试环境中会上抛错误
/// * 🚩生产环境中仅打印错误消息
#[cfg(test)]
fn re_capture<'a>(re: &'a Regex, haystack: &'a str) -> Result<Option<Captures<'a>>> {
    use anyhow::anyhow;
    match re.captures(haystack) {
        // * 🚩↓因为这里要包一层[`Some`]，所以无法使用[`Option::ok_or`]
        Some(captures) => Ok(Some(captures)),
        None => Err(anyhow!("无法使用正则表达式「{re}」捕获「{haystack}」")),
    }
}

/// （ONA）从原始输出中解析Narsese
/// * 🎯用于结合`#[cfg]`控制「严格模式」
///   * 🚩生产环境下「Narsese解析出错」仅打印错误信息
#[cfg(not(test))]
pub fn parse_narsese_ona(head: &str, tail: &str) -> Result<Option<Narsese>> {
    use nar_dev_utils::ResultBoost;
    // ! ↓下方会转换为None
    Ok(try_parse_narsese(tail).ok_or_run(|e| println!("【{head}】在解析Narsese时出现错误：{e}")))
}

/// （ONA）从原始输出中解析Narsese
/// * 🎯用于结合`#[cfg]`控制「严格模式」
///   * 🚩测试环境下「Narsese解析出错」会上抛错误
#[cfg(test)]
pub fn parse_narsese_ona(_: &str, tail: &str) -> Result<Option<Narsese>> {
    // ! ↓下方会上抛错误
    Ok(Some(try_parse_narsese(tail)?))
}

/// （尝试）从输出中解析出Narsese
/// * ❌【2024-03-27 22:01:18】目前引入[`anyhow::Error`]会出问题：不匹配/未满足的特征
pub fn try_parse_narsese(tail: &str) -> Result<Narsese> {
    // 提取并解析Narsese字符串
    pipe! {
        tail
        // 重整
        => #{&}
        => reform_output_to_narsese
        // 解析方言
        => #{&}
        => parse_dialect_ona
    }
}

/// 重整ONA输出到合法Narsese
/// * 🎯通过「重整→正确解析」的方式，实现初步输出解析兼容
/// * 🚩【2024-03-25 21:38:39】目前使用正则表达式[`regex`]库
/// * 🚩【2024-03-25 21:38:52】目前仅基于正则表达式做文本替换
/// * 📌参数`tail`不附带`Answer:`等部分
fn reform_output_to_narsese(out: &str) -> String {
    // 构造正则表达式（实现中只会编译一次） //
    // 匹配ONA输出中的「真值」⇒转换
    let re_truth = Regex::new(r"Truth:\s*frequency=([0-9.]+),\s*confidence=([0-9.]+)").unwrap();
    // 匹配ONA输出的「创建时间」⇒删去
    let re_creation_t = Regex::new(r"creationTime=([0-9.]+)\s+").unwrap();
    // 匹配ONA输出的「发生时间」⇒删去
    let re_occurrence_t = Regex::new(r"occurrenceTime=([0-9.]+)\s+").unwrap();
    // 匹配ONA输出的「时间递进」⇒删去
    let re_dt = Regex::new(r"dt=([0-9.]+)\s+").unwrap();
    // 匹配ONA输出的「优先级」⇒删去
    let re_priority = Regex::new(r"Priority=([0-9.]+)\s+").unwrap();

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
        => #{&} // 必须借用
        // 删去非必要的「发生时间」
        => [re_occurrence_t.replace_all](_, "")
        => #{&} // 必须借用
        // 删去非必要的「递进时间」
        => [re_dt.replace_all](_, "")
        => #{&} // 必须借用
        // 删去非必要的「优先级」
        => [re_priority.replace_all](_, "")
        // 剪切前后空白符
        => .trim()
        // 返回字符串 //
        => .into()
    }
}

/// 单元测试
#[cfg(test)]
mod test {
    use super::*;
    use nar_dev_utils::asserts;
    use narsese::conversion::string::impl_lexical::format_instances::FORMAT_ASCII;
    use navm::output::type_names::ANSWER;

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

    /// 测试/输出解析
    #[test]
    fn test_output_parse() {
        // 📄输出源自ONA测试文件`whatwarmer.nal`与ONA的命令行交互
        let outputs = "
        <a --> [warm]>. :|: %0.8%
        Input: <a --> [warm]>. :|: occurrenceTime=1 Priority=1.000000 Truth: frequency=0.800000, confidence=0.900000
        <a --> [warm]>. :|: %0.8%
        Input: <a --> [warm]>. :|: occurrenceTime=2 Priority=1.000000 Truth: frequency=0.800000, confidence=0.900000
        <a --> [warm]>. :|: %0.8%
        Input: <a --> [warm]>. :|: occurrenceTime=3 Priority=1.000000 Truth: frequency=0.800000, confidence=0.900000
        <b --> [warm]>. :|: %0.3%
        Input: <b --> [warm]>. :|: occurrenceTime=4 Priority=1.000000 Truth: frequency=0.300000, confidence=0.900000
        Derived: dt=1.000000 <<a --> [$1]> =/> <b --> [$1]>>. Priority=0.120425 Truth: frequency=0.300000, confidence=0.254517
        Derived: dt=1.000000 <<a --> [warm]> =/> <b --> [warm]>>. Priority=0.120425 Truth: frequency=0.300000, confidence=0.254517
        Derived: <a --> b>. :|: occurrenceTime=4 Priority=0.246973 Truth: frequency=0.800000, confidence=0.162760
        Derived: <b --> a>. :|: occurrenceTime=4 Priority=0.194273 Truth: frequency=0.300000, confidence=0.341412
        Derived: <a <-> b>. :|: occurrenceTime=4 Priority=0.189423 Truth: frequency=0.279070, confidence=0.357855
        Derived: <b <-> a>. :|: occurrenceTime=4 Priority=0.189423 Truth: frequency=0.279070, confidence=0.357855
        Derived: <(b | a) --> [warm]>. :|: occurrenceTime=4 Priority=0.099456 Truth: frequency=0.240000, confidence=0.648000
        Derived: <(a | b) --> [warm]>. :|: occurrenceTime=4 Priority=0.099456 Truth: frequency=0.240000, confidence=0.648000
        Derived: <(b & a) --> [warm]>. :|: occurrenceTime=4 Priority=0.219984 Truth: frequency=0.860000, confidence=0.648000
        Derived: <(a & b) --> [warm]>. :|: occurrenceTime=4 Priority=0.219984 Truth: frequency=0.860000, confidence=0.648000
        Derived: <(b ~ a) --> [warm]>. :|: occurrenceTime=4 Priority=0.064464 Truth: frequency=0.060000, confidence=0.648000
        Derived: <(a ~ b) --> [warm]>. :|: occurrenceTime=4 Priority=0.161664 Truth: frequency=0.560000, confidence=0.648000
        Derived: <(a * b) --> (+ warm)>. :|: occurrenceTime=4 Priority=0.247200 Truth: frequency=1.000000, confidence=0.648000
        Derived: <<a --> [$1]> ==> <b --> [$1]>>. :|: occurrenceTime=4 Priority=0.108382 Truth: frequency=0.300000, confidence=0.341412
        Derived: <<b --> [$1]> ==> <a --> [$1]>>. :|: occurrenceTime=4 Priority=0.137782 Truth: frequency=0.800000, confidence=0.162760
        Derived: <<a --> [$1]> <=> <b --> [$1]>>. :|: occurrenceTime=4 Priority=0.105676 Truth: frequency=0.279070, confidence=0.357855
        Derived: <<b --> [$1]> <=> <a --> [$1]>>. :|: occurrenceTime=4 Priority=0.105676 Truth: frequency=0.279070, confidence=0.357855
        Derived: (<a --> [#1]> && <b --> [#1]>). :|: occurrenceTime=4 Priority=0.083228 Truth: frequency=0.240000, confidence=0.648000
        Derived: (<b --> [#1]> && <a --> [#1]>). :|: occurrenceTime=4 Priority=0.083228 Truth: frequency=0.240000, confidence=0.648000
        <(?1 ~ ?2) --> [warm]>? :|:
        Input: <(?1 ~ ?2) --> [warm]>? :|:
        Answer: <(a ~ b) --> [warm]>. :|: occurrenceTime=4 creationTime=4 Truth: frequency=0.560000, confidence=0.648000
        ^pick. :|:
        Input: ^pick. :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        G. :|:
        Input: G. :|: occurrenceTime=6 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=1.000000 <(<a --> [warm]> &/ ^pick) =/> G>. Priority=0.185124 Truth: frequency=1.000000, confidence=0.186952
        Derived: dt=1.000000 <(<(a | b) --> [warm]> &/ ^pick) =/> G>. Priority=0.149877 Truth: frequency=1.000000, confidence=0.069427
        Derived: dt=1.000000 <(<a --> b> &/ ^pick) =/> G>. Priority=0.177205 Truth: frequency=1.000000, confidence=0.059471
        Derived: dt=1.000000 <(<b --> a> &/ ^pick) =/> G>. Priority=0.175070 Truth: frequency=1.000000, confidence=0.047999
        Derived: dt=1.000000 <(<a <-> b> &/ ^pick) =/> G>. Priority=0.174870 Truth: frequency=1.000000, confidence=0.046913
        Derived: dt=1.000000 <(<b <-> a> &/ ^pick) =/> G>. Priority=0.174870 Truth: frequency=1.000000, confidence=0.046913
        Derived: dt=1.000000 <(<(b | a) --> [warm]> &/ ^pick) =/> G>. Priority=0.149877 Truth: frequency=1.000000, confidence=0.069427
        Derived: dt=1.000000 <(<b --> [warm]> &/ ^pick) =/> G>. Priority=0.168996 Truth: frequency=1.000000, confidence=0.109355
        Derived: dt=1.000000 <(<(a & b) --> [warm]> &/ ^pick) =/> G>. Priority=0.170733 Truth: frequency=1.000000, confidence=0.183101
        Derived: dt=1.000000 <(<(b ~ a) --> [warm]> &/ ^pick) =/> G>. Priority=0.142227 Truth: frequency=1.000000, confidence=0.019374
        Derived: dt=1.000000 <(<(a ~ b) --> [warm]> &/ ^pick) =/> G>. Priority=0.161554 Truth: frequency=1.000000, confidence=0.136690
        Derived: dt=1.000000 <(<(a * b) --> (+ warm)> &/ ^pick) =/> G>. Priority=0.174542 Truth: frequency=1.000000, confidence=0.200929
        Derived: dt=1.000000 <((<a --> [#1]> && <b --> [#1]>) &/ ^pick) =/> G>. Priority=0.134326 Truth: frequency=1.000000, confidence=0.069427
        Derived: dt=1.000000 <((<b --> [#1]> && <a --> [#1]>) &/ ^pick) =/> G>. Priority=0.134326 Truth: frequency=1.000000, confidence=0.069427
        Derived: dt=1.000000 <((<a --> [warm]> &/ <b --> [warm]>) &/ ^pick) =/> G>. Priority=0.134326 Truth: frequency=1.000000, confidence=0.069427
        Derived: dt=1.000000 <(<(b & a) --> [warm]> &/ ^pick) =/> G>. Priority=0.170733 Truth: frequency=1.000000, confidence=0.183101
        Derived: dt=3.000000 <<a --> [warm]> =/> G>. Priority=0.208187 Truth: frequency=1.000000, confidence=0.199438
        Derived: dt=2.000000 <<(a | b) --> [warm]> =/> G>. Priority=0.162890 Truth: frequency=1.000000, confidence=0.075969
        Derived: dt=2.000000 <<a --> b> =/> G>. Priority=0.206921 Truth: frequency=1.000000, confidence=0.065217
        Derived: dt=2.000000 <<b --> a> =/> G>. Priority=0.204202 Truth: frequency=1.000000, confidence=0.052770
        Derived: dt=2.000000 <<a <-> b> =/> G>. Priority=0.203948 Truth: frequency=1.000000, confidence=0.051588
        Derived: dt=2.000000 <<b <-> a> =/> G>. Priority=0.203948 Truth: frequency=1.000000, confidence=0.051588
        Derived: dt=2.000000 <<(b | a) --> [warm]> =/> G>. Priority=0.162890 Truth: frequency=1.000000, confidence=0.075969
        Derived: dt=2.000000 <<(a * b) --> (+ warm)> =/> G>. Priority=0.191425 Truth: frequency=1.000000, confidence=0.213712
        Derived: dt=2.000000 <(<a --> [#1]> && <b --> [#1]>) =/> G>. Priority=0.142122 Truth: frequency=1.000000, confidence=0.075969
        Derived: dt=2.000000 <(<b --> [#1]> && <a --> [#1]>) =/> G>. Priority=0.142122 Truth: frequency=1.000000, confidence=0.075969
        Derived: dt=2.000000 <(<a --> [warm]> &/ <b --> [warm]>) =/> G>. Priority=0.142122 Truth: frequency=1.000000, confidence=0.075969
        Derived: dt=2.000000 <<(b & a) --> [warm]> =/> G>. Priority=0.187089 Truth: frequency=1.000000, confidence=0.195491
        Derived: dt=2.000000 <<b --> [warm]> =/> G>. Priority=0.189098 Truth: frequency=1.000000, confidence=0.118623
        Derived: dt=2.000000 <<(a & b) --> [warm]> =/> G>. Priority=0.187089 Truth: frequency=1.000000, confidence=0.195491
        Derived: dt=2.000000 <<(b ~ a) --> [warm]> =/> G>. Priority=0.153812 Truth: frequency=1.000000, confidence=0.021435
        Derived: dt=2.000000 <<(a ~ b) --> [warm]> =/> G>. Priority=0.176536 Truth: frequency=1.000000, confidence=0.147400
        <(<(a ~ b) --> [warm]> &/ ^pick) =/> G>?
        Input: <(<(a ~ b) --> [warm]> &/ ^pick) =/> G>?
        Answer: <(<(a ~ b) --> [warm]> &/ ^pick) =/> G>. creationTime=6 Truth: frequency=1.000000, confidence=0.136690

        a. :|:
        Input: a. :|: occurrenceTime=1 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        ^left. :|:
        Input: ^left. :|: occurrenceTime=2 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        g. :|:
        Input: g. :|: occurrenceTime=3 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=1.000000 <(a &/ ^left) =/> g>. Priority=0.254962 Truth: frequency=1.000000, confidence=0.241351
        Derived: dt=2.000000 <a =/> g>. Priority=0.335353 Truth: frequency=1.000000, confidence=0.254517
        a. :|:
        Input: a. :|: occurrenceTime=4 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=1.000000 <g =/> a>. Priority=0.348301 Truth: frequency=1.000000, confidence=0.282230
        Derived: dt=1.000000 <(a &/ g) =/> a>. Priority=0.246000 Truth: frequency=1.000000, confidence=0.213712
        g! :|:
        Input: g! :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        decision expectation=0.578198 implication: <(a &/ ^left) =/> g>. Truth: frequency=1.000000 confidence=0.241351 dt=1.000000 precondition: a. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=4
        ^left executed with args
        Input: ^left. :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        A. :|:
        Input: A. :|: occurrenceTime=7 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=2.000000 <((g &/ a) &/ ^left) =/> A>. Priority=0.201969 Truth: frequency=1.000000, confidence=0.174792
        Derived: dt=2.000000 <(a &/ ^left) =/> A>. Priority=0.246000 Truth: frequency=1.000000, confidence=0.213712
        Derived: dt=2.000000 <((a &/ g) &/ ^left) =/> A>. Priority=0.191125 Truth: frequency=1.000000, confidence=0.127972
        Derived: dt=2.000000 <(g &/ ^left) =/> A>. Priority=0.237903 Truth: frequency=1.000000, confidence=0.186952
        Derived: dt=3.000000 <(g &/ a) =/> A>. Priority=0.237903 Truth: frequency=1.000000, confidence=0.186952
        Derived: dt=3.000000 <a =/> A>. Priority=0.323287 Truth: frequency=1.000000, confidence=0.226692
        Derived: dt=4.000000 <(a &/ g) =/> A>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=4.000000 <g =/> A>. Priority=0.312281 Truth: frequency=1.000000, confidence=0.199438
        <(*, {SELF}) --> ^left>. :|:
        Input: <(* {SELF}) --> ^left>. :|: occurrenceTime=8 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: (* {SELF}). :|: occurrenceTime=8 Priority=0.182344 Truth: frequency=1.000000, confidence=0.293146
        G. :|:
        Input: G. :|: occurrenceTime=9 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=1.000000 <(((g &/ A) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.134179 Truth: frequency=1.000000, confidence=0.068411
        Derived: dt=1.000000 <((a &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.144347 Truth: frequency=1.000000, confidence=0.090215
        Derived: dt=1.000000 <(((g &/ a) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.134179 Truth: frequency=1.000000, confidence=0.068411
        Derived: dt=1.000000 <((g &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.141953 Truth: frequency=1.000000, confidence=0.074873
        Derived: dt=1.000000 <(((a &/ A) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.136267 Truth: frequency=1.000000, confidence=0.082685
        Derived: dt=1.000000 <(((a &/ g) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.131034 Truth: frequency=1.000000, confidence=0.046051
        Derived: dt=1.000000 <((A &/ ^left) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.154562 Truth: frequency=1.000000, confidence=0.150345
        Derived: dt=4.000000 <(a &/ ^left) =/> G>. Priority=0.230723 Truth: frequency=1.000000, confidence=0.161649
        Derived: dt=4.000000 <((g &/ a) &/ ^left) =/> G>. Priority=0.191125 Truth: frequency=1.000000, confidence=0.127972
        Derived: dt=4.000000 <(g &/ ^left) =/> G>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=4.000000 <((a &/ g) &/ ^left) =/> G>. Priority=0.183193 Truth: frequency=1.000000, confidence=0.090215
        Derived: dt=1.000000 <((g &/ A) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.150597 Truth: frequency=1.000000, confidence=0.127972
        Derived: dt=1.000000 <(a &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.166364 Truth: frequency=1.000000, confidence=0.161649
        Derived: dt=1.000000 <((g &/ a) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.150597 Truth: frequency=1.000000, confidence=0.127972
        Derived: dt=1.000000 <(g &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.161849 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=1.000000 <((a &/ A) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.154562 Truth: frequency=1.000000, confidence=0.150345
        Derived: dt=1.000000 <((a &/ g) &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.144347 Truth: frequency=1.000000, confidence=0.090215
        Derived: dt=1.000000 <(A &/ <(* {SELF}) --> ^left>) =/> G>. Priority=0.183842 Truth: frequency=1.000000, confidence=0.241351
        Derived: dt=2.000000 <(g &/ A) =/> G>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=5.000000 <a =/> G>. Priority=0.302437 Truth: frequency=1.000000, confidence=0.173382
        Derived: dt=5.000000 <(g &/ a) =/> G>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=6.000000 <g =/> G>. Priority=0.293787 Truth: frequency=1.000000, confidence=0.149042
        Derived: dt=2.000000 <(a &/ A) =/> G>. Priority=0.230723 Truth: frequency=1.000000, confidence=0.161649
        Derived: dt=1.000000 <(* {SELF}) =/> G>. Priority=0.195713 Truth: frequency=1.000000, confidence=0.148415
        Derived: dt=6.000000 <(a &/ g) =/> G>. Priority=0.214505 Truth: frequency=1.000000, confidence=0.098268
        Derived: dt=2.000000 <A =/> G>. Priority=0.335353 Truth: frequency=1.000000, confidence=0.254517
        A. :|:
        Input: A. :|: occurrenceTime=10 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=2.000000 <((a &/ ^left) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.141953 Truth: frequency=1.000000, confidence=0.074873
        Derived: dt=2.000000 <(((g &/ a) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.132453 Truth: frequency=1.000000, confidence=0.056268
        Derived: dt=2.000000 <(((g &/ A) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.132453 Truth: frequency=1.000000, confidence=0.056268
        Derived: dt=2.000000 <(((a &/ g) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.129874 Truth: frequency=1.000000, confidence=0.037532
        Derived: dt=2.000000 <((g &/ ^left) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.139967 Truth: frequency=1.000000, confidence=0.061748
        Derived: dt=2.000000 <(((a &/ A) &/ ^left) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.134179 Truth: frequency=1.000000, confidence=0.068411
        Derived: dt=2.000000 <(a &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.161849 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=2.000000 <((g &/ a) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.147209 Truth: frequency=1.000000, confidence=0.107901
        Derived: dt=2.000000 <((g &/ A) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.147209 Truth: frequency=1.000000, confidence=0.107901
        Derived: dt=2.000000 <((a &/ g) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.141953 Truth: frequency=1.000000, confidence=0.074873
        Derived: dt=2.000000 <(g &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.157967 Truth: frequency=1.000000, confidence=0.117083
        Derived: dt=2.000000 <((a &/ A) &/ <(* {SELF}) --> ^left>) =/> A>. Priority=0.150597 Truth: frequency=1.000000, confidence=0.127972
        Derived: dt=5.000000 <(a &/ ^left) =/> A>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.138259
        Revised: dt=3.113558 <(a &/ ^left) =/> A>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.301794
        Derived: dt=5.000000 <((g &/ a) &/ ^left) =/> A>. Priority=0.186825 Truth: frequency=1.000000, confidence=0.107901
        Revised: dt=3.090418 <((g &/ a) &/ ^left) =/> A>. Priority=0.186825 Truth: frequency=1.000000, confidence=0.249682
        Derived: dt=5.000000 <((a &/ g) &/ ^left) =/> A>. Priority=0.180156 Truth: frequency=1.000000, confidence=0.074873
        Revised: dt=3.066382 <((a &/ g) &/ ^left) =/> A>. Priority=0.180156 Truth: frequency=1.000000, confidence=0.185459
        Derived: dt=5.000000 <(g &/ ^left) =/> A>. Priority=0.219076 Truth: frequency=1.000000, confidence=0.117083
        Revised: dt=3.097308 <(g &/ ^left) =/> A>. Priority=0.219076 Truth: frequency=1.000000, confidence=0.266081
        Derived: dt=6.000000 <a =/> A>. Priority=0.293787 Truth: frequency=1.000000, confidence=0.149042
        Revised: dt=4.100474 <a =/> A>. Priority=0.293787 Truth: frequency=0.980787, confidence=0.323166
        Derived: dt=1.000000 <G =/> A>. Priority=0.348301 Truth: frequency=1.000000, confidence=0.282230
        Derived: dt=2.000000 <(* {SELF}) =/> A>. Priority=0.190743 Truth: frequency=1.000000, confidence=0.126225
        Derived: dt=1.000000 <(A &/ G) =/> A>. Priority=0.246000 Truth: frequency=1.000000, confidence=0.213712
        Derived: dt=1.000000 <(g &/ G) =/> A>. Priority=0.219076 Truth: frequency=1.000000, confidence=0.117083
        Derived: dt=1.000000 <((* {SELF}) &/ G) =/> A>. Priority=0.170371 Truth: frequency=1.000000, confidence=0.116545
        Derived: dt=7.000000 <(a &/ g) =/> A>. Priority=0.210665 Truth: frequency=1.000000, confidence=0.081831
        Revised: dt=5.053462 <(a &/ g) =/> A>. Priority=0.210665 Truth: frequency=0.983303, confidence=0.202427
        Derived: dt=7.000000 <g =/> A>. Priority=0.286301 Truth: frequency=1.000000, confidence=0.126793
        Revised: dt=5.084493 <g =/> A>. Priority=0.286301 Truth: frequency=0.981712, confidence=0.286567
        Derived: dt=1.000000 <(a &/ G) =/> A>. Priority=0.224460 Truth: frequency=1.000000, confidence=0.138259
        Derived: dt=6.000000 <(g &/ a) =/> A>. Priority=0.219076 Truth: frequency=1.000000, confidence=0.117083
        Revised: dt=4.077649 <(g &/ a) =/> A>. Priority=0.219076 Truth: frequency=0.982085, confidence=0.269626
        G! :|:
        Input: G! :|: occurrenceTime=11 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=4.000000 <a =/> (* {SELF})>. Priority=0.182921 Truth: frequency=1.000000, confidence=0.088860
        Derived: dt=4.000000 <(g &/ a) =/> (* {SELF})>. Priority=0.161381 Truth: frequency=1.000000, confidence=0.067330
        Derived: dt=5.000000 <(a &/ g) =/> (* {SELF})>. Priority=0.157655 Truth: frequency=1.000000, confidence=0.045286
        Derived: dt=5.000000 <g =/> (* {SELF})>. Priority=0.179929 Truth: frequency=1.000000, confidence=0.073708
        decision expectation=0.578198 implication: <(A &/ <(* {SELF}) --> ^left>) =/> G>. Truth: frequency=1.000000 confidence=0.241351 dt=1.000000 precondition: A. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=10
        ^left executed with args (* {SELF})
        Input: <(* {SELF}) --> ^left>. :|: occurrenceTime=11 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: (* {SELF}). :|: occurrenceTime=11 Priority=0.120799 Truth: frequency=1.000000, confidence=0.175147

        A. :|:
        Input: A. :|: occurrenceTime=1 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        <(*, {SELF}) --> ^left>. :|:
        Input: <(* {SELF}) --> ^left>. :|: occurrenceTime=2 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        G. :|:
        Input: G. :|: occurrenceTime=3 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        A. :|:
        Input: A. :|: occurrenceTime=4 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        G! :|:
        Input: G! :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        decision expectation=0.578198 implication: <(A &/ <(* {SELF}) --> ^left>) =/> G>. Truth: frequency=1.000000 confidence=0.241351 dt=1.000000 precondition: A. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=4
        ^left executed with args (* {SELF})
        Input: <(* {SELF}) --> ^left>. :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000

        A2. :|:
        Input: A2. :|: occurrenceTime=8 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        <(*, {SELF}, P) --> ^left>. :|:
        Input: <({SELF} * P) --> ^left>. :|: occurrenceTime=9 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        G2. :|:
        Input: G2. :|: occurrenceTime=10 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        A2. :|:
        Input: A2. :|: occurrenceTime=11 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        G2! :|:
        Input: G2! :|: occurrenceTime=12 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        decision expectation=0.578198 implication: <(A2 &/ <({SELF} * P) --> ^left>) =/> G2>. Truth: frequency=1.000000 confidence=0.241351 dt=1.000000 precondition: A2. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=11
        ^left executed with args ({SELF} * P)
        Input: <({SELF} * P) --> ^left>. :|: occurrenceTime=12 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000

        A. :|:
        Input: A. :|: occurrenceTime=1 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        <(*, {SELF}) --> ^op>. :|:
        Input: <(* {SELF}) --> ^op>. :|: occurrenceTime=2 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        G. :|:
        Input: G. :|: occurrenceTime=3 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=1.000000 <(A &/ <(* {SELF}) --> ^op>) =/> G>. Priority=0.183842 Truth: frequency=1.000000, confidence=0.241351
        Derived: dt=2.000000 <A =/> G>. Priority=0.335353 Truth: frequency=1.000000, confidence=0.254517
        A. :|:
        Input: A. :|: occurrenceTime=4 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        Derived: dt=1.000000 <G =/> A>. Priority=0.348301 Truth: frequency=1.000000, confidence=0.282230
        Derived: dt=1.000000 <(A &/ G) =/> A>. Priority=0.246000 Truth: frequency=1.000000, confidence=0.213712
        G! :|:
        Input: G! :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000
        decision expectation=0.578198 implication: <(A &/ <(* {SELF}) --> ^op>) =/> G>. Truth: frequency=1.000000 confidence=0.241351 dt=1.000000 precondition: A. :|: Truth: frequency=1.000000 confidence=0.900000 occurrenceTime=4
        ^op executed with args (* {SELF})
        Input: <(* {SELF}) --> ^op>. :|: occurrenceTime=5 Priority=1.000000 Truth: frequency=1.000000, confidence=0.900000

        A.
        B?
        Answer: None.
        " // 【2024-03-29 16:58:32】省略的「操作注册」语法：`*setopname 1 ^op`
        // 初步数据处理
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty());

        // 开始测试解析
        for output in outputs {
            // ! 测试环境下[`parse_narsese_ona`]会强制要求「Narsese内容解析成功」
            let o = output_translate(output.into()).expect("输出解析失败");
            // * 📌测试不能放过`Answer: None.`这个「不是回答的『回答』」
            // * 🚩「是回答」与「内容为`Answer: None.`」不能共存
            assert!(!(o.is_type(ANSWER) && o.raw_content().contains("None.")));
            // 正常解析并展示Narsese
            if let Some(narsese) = o.get_narsese() {
                println!("{}", FORMAT_ASCII.format_narsese(narsese))
            } else {
                println!("[{}] {}", o.type_name(), o.raw_content())
            }
        }
    }
}
