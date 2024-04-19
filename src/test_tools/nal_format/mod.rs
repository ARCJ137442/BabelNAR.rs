//! 基于NAVM的「统一`.nal`格式」支持
//! * ✨语法（解析）支持
//! * 🎯提供一种（部分）兼容现有`.nal`格式文件的语法
//!   * ⚠️对其中所有Narsese部分使用CommonNarsese「通用纳思语」：不兼容方言

use std::{result::Result::Err as StdErr, result::Result::Ok as StdOk, time::Duration};

use super::structs::*;
use anyhow::{Ok, Result};
use narsese::{
    conversion::string::impl_lexical::format_instances::FORMAT_ASCII,
    lexical::{Narsese, Sentence, Task},
};
use navm::{cmd::Cmd, output::Operation};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use util::{first, pipe};

#[derive(Parser)] // ! ↓ 必须从项目根目录开始
#[grammar = "src/test_tools/nal_format/nal_grammar.pest"]
pub struct NALParser;

/// 使用[`pest`]将整个`.nal`文件内容转换为[`NALInput`]结果序列
/// * ✨也可只输入一行，用以解析单个[`NALInput`]
/// * 📌重点在其简写的「操作」语法`(^left, {SELF}, x)` => `<(*, {SELF}, x) --> ^left>`
pub fn parse(input: &str) -> Vec<Result<NALInput>> {
    input
        // 切分并过滤空行
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        // 逐行解析
        .map(parse_single)
        // 收集所有结果
        .collect::<Vec<_>>()
}

pub fn parse_single(line: &str) -> Result<NALInput> {
    // 解析一行
    pipe! {
        line
        // 从一行输入解析到[`pest`]的一个[`Pairs`]
        => NALParser::parse(Rule::nal_input, _)
        // 发现错误即上抛
        => {?}#
        // 🚩只对应[`Rule::nal_input`]规则，因此只会有一个[`Pair`]，不会有其它情形
        => .next()
        => .unwrap()
        // 折叠，返回结果
        => fold_pest
    }
}

/// 将[`pest`]解析出的[`Pair`]辅助折叠到「词法Narsese」中
/// * 🚩只需处理单行输入：行与行之间分开解析，避免上下文污染
/// * 📌只会存在如下主要情况
///   * `cyc_uint`：`CYC`语法糖，亦兼容原`.nal`格式
///   * `narsese`：`NSE`语法糖，亦兼容原`.nal`格式
///   * `comment`：各类或「魔法」或「非魔法」的注释
fn fold_pest(pair: Pair<Rule>) -> Result<NALInput> {
    // * 🚩【2024-04-02 18:33:05】此处不用再`trim`了：入口`parse`已经做过
    let pair_str = pair.as_str();
    match pair.as_rule() {
        // 一行的无符号整数 //
        Rule::cyc_uint => {
            // 仅取数字部分
            let n: usize = pair_str.parse()?;
            // * 🚩作为`CYC`语法糖
            let input = NALInput::Put(Cmd::CYC(n));
            Ok(input)
        }
        // 一行的Narsese //
        Rule::narsese => {
            // 作为CommonNarsese，直接取字符串，然后调用CommonNarsese ASCII解析器
            // * 🚩【2024-03-31 16:37:32】虽可能有失灵活性，但代码上更显通用
            let narsese = pair_str;
            let narsese = FORMAT_ASCII.parse(narsese)?.try_into_task_compatible()?;
            // * 🚩作为`NSE`语法糖
            let input = NALInput::Put(Cmd::NSE(narsese));
            Ok(input)
        }
        // 各种魔法注释 //
        // 单纯的行注释：`REM`语法糖
        Rule::comment_raw => {
            // 仅取注释部分
            // ! 不能用`to_string`：后者只会显示其总体信息，而非捕获相应字符串切片
            let comment = pair_str.into();
            // * 🚩作为`REM`语法糖
            let input = NALInput::Put(Cmd::REM { comment });
            Ok(input)
        }
        // 魔法注释/置入指令
        Rule::comment_navm_cmd => {
            // 取其中第一个`comment_raw`元素 | 一定只有唯一一个`comment_raw`
            let comment_raw = pair.into_inner().next().unwrap();
            // 仅取注释部分
            let line = comment_raw.as_str().trim();
            // * 🚩作为所有NAVM指令的入口
            let input = NALInput::Put(Cmd::parse(line)?);
            Ok(input)
        }
        // 魔法注释/睡眠等待
        Rule::comment_sleep => {
            // 取其中第一个`comment_raw`元素 | 一定只有唯一一个`comment_raw`
            let duration_raw = pair.into_inner().next().unwrap().as_str().trim();
            // 尝试解析时间
            let duration = parse_duration(duration_raw)?;
            // * 封装
            let input = NALInput::Sleep(duration);
            Ok(input)
        }
        // 魔法注释/等待
        Rule::comment_await => {
            // 取其中唯一一个「输出预期」
            let output_expectation = pair.into_inner().next().unwrap();
            let output_expectation = fold_pest_output_expectation(output_expectation)?;
            Ok(NALInput::Await(output_expectation))
        }
        // 魔法注释/输出包含
        Rule::comment_expect_contains => {
            // 取其中唯一一个「输出预期」
            let output_expectation = pair.into_inner().next().unwrap();
            let output_expectation = fold_pest_output_expectation(output_expectation)?;
            Ok(NALInput::ExpectContains(output_expectation))
        }
        // 魔法注释/保存输出
        Rule::comment_save_outputs => {
            // 取其中唯一一个「输出预期」
            let file_path = pair.into_inner().next().unwrap().as_str().into();
            Ok(NALInput::SaveOutputs(file_path))
        }
        // 魔法注释/循环预期
        Rule::comment_expect_cycle => {
            let mut pairs = pair.into_inner();
            // 取其中的「最大步数」
            let max_cycles = pipe! {
                pairs.next().unwrap()
                => .as_str()
                => {.parse::<usize>()}#
                => {?}#
            };
            // 取其中的「每次步长」
            let step_cycles = pipe! {
                pairs.next().unwrap()
                => .as_str()
                => {.parse::<usize>()}#
                => {?}#
            };
            // 取其中的「输出预期」
            let step_duration = pairs.next();
            let step_duration = match step_duration {
                Some(step_duration) => {
                    // 尝试解析时间
                    let step_duration = parse_duration(step_duration.as_str())?;
                    // 封装
                    Some(step_duration)
                }
                None => None,
            };
            // 取其中的「输出预期」
            let output_expectation = pipe! {
                pairs.next().unwrap()
                => fold_pest_output_expectation
                => {?}#
            };
            // 构造 & 返回
            Ok(NALInput::ExpectCycle(
                max_cycles,
                step_cycles,
                step_duration,
                output_expectation,
            ))
        }
        // 魔法注释/终止
        Rule::comment_terminate => {
            // 预置默认值
            let mut if_not_user = false;
            let mut result = StdOk(());

            // 遍历其中的Pair
            for inner in pair.into_inner() {
                // 逐个匹配规则类型
                //   * ✨comment_terminate_option: `if-not-user`
                //   * ✨comment_raw: Err(`message`)
                match inner.as_rule() {
                    // 可选规则
                    Rule::comment_terminate_option => {
                        if inner.as_str() == "if-no-user" {
                            if_not_user = true;
                        }
                    }
                    // 错误消息
                    Rule::comment_raw => {
                        // 构造错误 | 仅取注释部分
                        result = StdErr(inner.as_str().trim().into())
                    }
                    // 其它
                    _ => unreachable!("不该被匹配到的规则\tpair = {inner:?}"),
                }
            }

            // 构造&返回
            Ok(NALInput::Terminate {
                if_not_user,
                result,
            })
        }
        // 其它情况
        _ => unreachable!("不该被匹配到的规则\tpair = {pair:?}"),
    }
}

/// 解析其中的「输出预期」[`Pair`]
/// * 🚩在「遍历内部元素」时消耗[`Pair`]对象
#[inline]
fn fold_pest_output_expectation(pair: Pair<Rule>) -> Result<OutputExpectation> {
    // 构造一个（全空的）输出预期对象
    let mut result = OutputExpectation::default();
    // 开始遍历其中的元素
    for inner in pair.into_inner() {
        // 逐个匹配规则类型
        // * 🚩【2024-04-01 00:18:23】目前只可能有三个
        //   * ✨输出类型
        //   * ✨Narsese
        //   * ✨NAVM操作
        match inner.as_rule() {
            // 输出类型
            Rule::output_type => {
                // 取其中唯一一个`output_type_name`
                // ! 不能用`to_string`：后者只会显示其总体信息，而非捕获相应字符串切片
                let output_type = inner.as_str().into();
                // 添加到结果中
                result.output_type = Some(output_type);
            }
            // Narsese
            Rule::narsese => {
                // 取其中唯一一个`narsese`
                let narsese = inner.as_str();
                // 解析Narsese
                let narsese = FORMAT_ASCII.parse(narsese)?;
                // 添加到结果中
                result.narsese = Some(narsese);
            }
            // NAVM操作
            Rule::output_operation => result.operation = Some(fold_pest_output_operation(inner)?),
            // 其它情况
            _ => unreachable!("不该被匹配到的规则\tpair = {inner:?}"),
        }
    }

    // 返回
    Ok(result)
}

/// 解析其中的「NAVM操作」[`Pair`]
/// * 其中[`Pair`]的`rule`属性必是`output_operation`
#[inline]
fn fold_pest_output_operation(pair: Pair<Rule>) -> Result<Operation> {
    // 生成迭代器
    let mut pairs = pair.into_inner();
    // 取第一个子Pair当操作名 | 语法上保证一定有
    let operator_name = pairs.next().unwrap().as_str().to_owned();
    // 操作参数
    let mut params = vec![];
    // 消耗剩下的，填充参数
    for inner in pairs {
        // 尝试作为Narsese词项解析 | 无法使用 *narsese.get_term()强制转换成词项
        let term = match FORMAT_ASCII.parse(inner.as_str())? {
            Narsese::Term(term)
            | Narsese::Sentence(Sentence { term, .. })
            | Narsese::Task(Task {
                sentence: Sentence { term, .. },
                ..
            }) => term,
        };
        // 添加到参数中
        params.push(term);
    }
    // 返回
    Ok(Operation {
        operator_name,
        params,
    })
}

fn parse_duration(duration_raw: &str) -> Result<Duration> {
    Ok(first! {
        // 毫秒→微秒→纳秒→秒 | 对于「秒」分「整数」「浮点」两种
        duration_raw.ends_with("ms") => Duration::from_millis(duration_raw.strip_suffix("ms").unwrap().parse()?),
        duration_raw.ends_with("μs") => Duration::from_micros(duration_raw.strip_suffix("μs").unwrap().parse()?),
        duration_raw.ends_with("ns") => Duration::from_nanos(duration_raw.strip_suffix("ns").unwrap().parse()?),
        duration_raw.ends_with('s') && duration_raw.contains('.') => Duration::try_from_secs_f64(duration_raw.strip_suffix('s').unwrap().parse()?)?,
        duration_raw.ends_with('s') => Duration::from_secs(duration_raw.strip_suffix('s').unwrap().parse()?),
        // 否则报错
        _ => return Err(anyhow::anyhow!("未知的睡眠时间参数 {duration_raw:?}"))
    })
}

/// 单元测试
#[cfg(test)]
pub mod tests {
    use super::*;
    use util::{for_in_ifs, list};

    pub const TESTSET: &str = "\
' 用于测试CIN的「简单演绎推理」
' * 📝利用现有`Narsese`语法
'
' 输出预期
' * 📝统一的NAL测试语法：`''expect-contains: 【输出类别】 【其它内容】`
'   * 📄预期「回答」：`''expect-contains: ANSWER 【CommonNarsese】`
'   * 📄预期「操作」：`''expect-contains: EXE (^【操作名】, 【操作参数（CommonNarsese词项）】)`

'/VOL 0
<A --> B>.
<B --> C>.
<A --> C>?
5
''sleep: 1s
''expect-contains: ANSWER <A --> C>.

A3. :|:
<(*, {SELF}, (*, P1, P2)) --> ^left>. :|:
G3. :|:
A3. :|:
G3! :|:
''sleep: 500ms
10

''expect-contains: EXE (^left, {SELF}, (*, P1, P2))
''terminate(if-no-user)";

    #[test]
    fn test_parse() {
        _test_parse("<A --> B>.");
        _test_parse("5");
        _test_parse("'这是一个注释");
        _test_parse("'/VOL 0");
        _test_parse("'''VOL 0");
        _test_parse("''await: OUT <A --> B>.");
        _test_parse("''sleep: 500ms");
        _test_parse("''sleep: 5000μs");
        _test_parse("''sleep: 600ns");
        _test_parse("''terminate(if-no-user): 异常的退出消息！");
        _test_parse(TESTSET);
    }

    fn _test_parse(input: &str) {
        let results = parse(input);
        let results = list![
            (r.expect("解析失败！"))
            for r in (results)
        ];
        for_in_ifs! {
            {println!("{:?}", r);}
            for r in (results)
        }
    }
}
