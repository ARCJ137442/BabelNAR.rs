//! ONA方言
//! * 🎯解析ONA输出，如
//!   * 📄以空格分隔的词项：`(* {SELF})`
//!   * 📄`({SELF} * x)`

use crate::runtime::TranslateError;
use anyhow::{Ok, Result};
use narsese::{
    conversion::string::{
        impl_enum::format_instances::FORMAT_ASCII, impl_lexical::structs::MidParseResult,
    },
    lexical::{Budget, Narsese, Term, Truth},
};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)] // ! ↓ 必须从项目根目录开始
#[grammar = "src/cin_implements/ona/dialect_ona.pest"]
pub struct DialectParser;

/// 使用[`pest`]将输入的「ONA方言」转换为「词法Narsese」
/// 以ONA的语法解析出Narsese
/// * 📌重点在「用空格分隔乘积词项/中缀情形」的语法
///   * 📄`(* {SELF})`
///   * 📄`({SELF} * x)`
pub fn parse(input: &str) -> Result<Narsese> {
    // 语法解析
    let pair = DialectParser::parse(Rule::narsese, input)?.next().unwrap();

    // 语法折叠
    let folded = fold_pest(pair)?;

    // 返回
    Ok(folded)
}

/// 将[`pest`]解析出的[`Pair`]辅助折叠到「词法Narsese」中
fn fold_pest(pest_parsed: Pair<Rule>) -> Result<Narsese> {
    let mut mid_result = MidParseResult {
        budget: None,
        term: None,
        punctuation: None,
        stamp: None,
        truth: None,
    };
    fold_pest_procedural(pest_parsed, &mut mid_result)?;
    match mid_result.fold() {
        Some(narsese) => Ok(narsese),
        None => TranslateError::err_anyhow("无效的中间结果"),
    }
}

/// 过程式折叠[`pest`]词法值
/// * 🎯向「中间解析结果」填充元素，而无需考虑元素的顺序与返回值类型
pub(super) fn fold_pest_procedural(pair: Pair<Rule>, result: &mut MidParseResult) -> Result<()> {
    match pair.as_rule() {
        // 不会被匹配的`_{..}`元素
        Rule::WHITESPACE | Rule::narsese | Rule::budget_content | Rule::term => {
            unreachable!("规则{:?}不会被匹配到！{pair:?}", pair.as_rule())
        }
        // Narsese：转发 | 📝语法文件中前缀`_`的，若为纯内容则自动忽略，若内部有元素则自动提取
        // Rule::narsese => fold_pest_procedural(pair.into_inner().next().unwrap(), result),
        // 任务⇒所有内部元素递归 | 安装「预算值」「语句」
        Rule::task => {
            for pair in pair.into_inner() {
                fold_pest_procedural(pair, result)?;
            }
        }
        // 预算⇒尝试解析并填充预算
        Rule::budget => result.budget = Some(fold_pest_budget(pair)?),
        // 语句⇒所有内部元素递归 | 安装「词项」「标点」「时间戳」「真值」
        Rule::sentence => {
            for pair in pair.into_inner() {
                fold_pest_procedural(pair, result)?;
            }
        }
        // 词项⇒提取其中的元素 | 安装 原子 / 复合 / 陈述 | ✅pest自动解包
        // Rule::term => fold_pest_procedural(pair.into_inner().next().unwrap(), result),
        Rule::statement => result.term = Some(fold_pest_statement(pair)?),
        Rule::compound => result.term = Some(fold_pest_compound(pair)?),
        Rule::atom => result.term = Some(fold_pest_atom(pair)?),
        // 时间戳 / 标点 ⇒ 直接插入
        Rule::punctuation => result.punctuation = Some(pair.as_str().into()),
        Rule::stamp => result.stamp = Some(pair.as_str().into()),
        // 真值 ⇒ 解析 ~ 插入
        Rule::truth => result.truth = Some(fold_pest_truth(pair)?),
        // 仅出现在内部解析中的不可达规则
        _ => unreachable!("仅出现在内部解析的不可达规则！{:?}{pair}", pair.as_rule()),
    }
    Ok(())
}

/// 折叠[`pest`]真值
pub(super) fn fold_pest_truth(pair: Pair<Rule>) -> Result<Truth> {
    let mut v = Truth::new();
    for pair_value_str in pair.into_inner() {
        v.push(pair_value_str.as_str().to_string());
    }
    Ok(v)
}

/// 折叠[`pest`]预算值
pub(super) fn fold_pest_budget(pair: Pair<Rule>) -> Result<Budget> {
    let mut v = Budget::new();
    for pair_value_str in pair.into_inner() {
        v.push(pair_value_str.as_str().to_string());
    }
    Ok(v)
}

/// 折叠[`pest`]词项
/// * 🎯用于「复合词项/陈述」内部词项的解析
/// * 📌原子、复合、陈述均可
pub(super) fn fold_pest_term(pair: Pair<Rule>) -> Result<Term> {
    // 根据规则分派
    match pair.as_rule() {
        Rule::atom => fold_pest_atom(pair),
        Rule::compound => fold_pest_compound(pair),
        Rule::statement => fold_pest_statement(pair),
        _ => unreachable!("词项只有可能是原子、复合与陈述 | {pair}"),
    }
}

/// 折叠[`pest`]原子词项
pub(super) fn fold_pest_atom(pair: Pair<Rule>) -> Result<Term> {
    let mut prefix = String::new();
    let mut name = String::new();
    for pair in pair.into_inner() {
        let pair_str = pair.as_str();
        match pair.as_rule() {
            Rule::atom_prefix => prefix.push_str(pair_str),
            Rule::atom_content => name.push_str(pair_str),
            // 占位符
            Rule::placeholder => {
                prefix.push('_');
                if pair_str.len() > 1 {
                    name.push_str(&pair_str[1..]);
                }
            }
            _ => unreachable!("原子词项只可能有「占位符」或「前缀+名称（内容）」两种 | {pair}"),
        }
    }
    Ok(Term::Atom { prefix, name })
}

/// 折叠[`pest`]复合词项
/// * 🚩【2024-03-29 09:42:36】因「需要通过规则识别『外延集/内涵集』」通过「进一步向下分发」细化被折叠对象
pub(super) fn fold_pest_compound(pair: Pair<Rule>) -> Result<Term> {
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::compound_common => {
            // * 🚩通用复合词项：连接词 词项...
            let mut pairs = pair.into_inner();
            let connecter = pairs.next().unwrap().as_str().into();
            let mut terms = vec![];
            // 遍历剩下的元素
            for pair in pairs {
                terms.push(fold_pest_term(pair)?);
            }
            Ok(Term::Compound { connecter, terms })
        }
        Rule::compound_binary => {
            // * 🆕ONA特有的「二元复合词项」
            let mut pairs = pair.into_inner();
            // 第一个是左边的词项
            let left = fold_pest_term(pairs.next().unwrap())?;
            // 连接词
            let connecter = pairs.next().unwrap().as_str().to_string();
            // 第二个是右边的词项
            let right = fold_pest_term(pairs.next().unwrap())?;
            // 构造 & 返回
            Ok(Term::Compound {
                connecter,
                terms: vec![left, right],
            })
        }
        Rule::ext_set => {
            let mut terms = vec![];
            for pair in pair.into_inner() {
                terms.push(fold_pest_term(pair)?);
            }
            // 构造 & 返回
            // * 🚩【2024-03-29 09:51:46】使用「枚举Narsese」的语法内容，避免硬编码
            Ok(Term::Set {
                left_bracket: FORMAT_ASCII.compound.brackets_set_extension.0.into(),
                terms,
                right_bracket: FORMAT_ASCII.compound.brackets_set_extension.1.into(),
            })
        }
        Rule::int_set => {
            let mut terms = vec![];
            for pair in pair.into_inner() {
                terms.push(fold_pest_term(pair)?);
            }
            // 构造 & 返回
            // * 🚩【2024-03-29 09:51:46】使用「枚举Narsese」的语法内容，避免硬编码
            Ok(Term::Set {
                left_bracket: FORMAT_ASCII.compound.brackets_set_intension.0.into(),
                terms,
                right_bracket: FORMAT_ASCII.compound.brackets_set_intension.1.into(),
            })
        }
        _ => unreachable!("复合词项只可能是「通用」「操作」「外延集」「内涵集」四种 | {pair}"),
    }
}

/// 折叠[`pest`]陈述
pub(super) fn fold_pest_statement(pair: Pair<Rule>) -> Result<Term> {
    // ! 陈述结构保证：主词+系词+谓词
    let mut pairs = pair.into_inner();
    // 🚩顺序折叠
    let subject = fold_pest_term(pairs.next().unwrap())?;
    let copula = pairs.next().unwrap().as_str();
    let predicate = fold_pest_term(pairs.next().unwrap())?;
    // 创建
    Ok(Term::new_statement(copula, subject, predicate))
}

/// 单元测试
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use narsese::conversion::string::impl_lexical::format_instances::FORMAT_ASCII;
    use util::first;

    /// 测试/方言解析器 🚧
    #[test]
    fn test_dialect_parser() {
        // 统计用
        let mut 直接相等的个数: usize = 0;
        let mut 删去空格与分隔符后相等的个数: usize = 0;
        let mut 去掉空格与分隔符_字符重排后相等的个数: usize = 0; // * 🚩【2024-03-29 15:08:24】用于辨别「中缀运算符⇒重排」的情况
        let mut 形式有变的 = vec![];

        // 📄部分改自OpenNARS`long_term_stability.nal`
        // 📄部分源自ONA`Nalifier_ex1.nal`、`NAR_Nalifier_ex1.nal`、
        let narseses = "
        (* {SELF})
        ({SELF} * x)

        <(&|,<(*,{SELF},$1,FALSE)-->^want>,<(*,{SELF},$1)-->^anticipate>) =|> <(*,{SELF},$1) --> afraid_of>>.
        <A --> B>.
        {A, B}
        <{tim} --> (/,livingIn,_,{graz})>. %0%
        <<(*,$1,sunglasses) --> own> ==> <$1 --> [aggressive]>>.
        <(*,{tom},sunglasses) --> own>.
        <<$1 --> [aggressive]> ==> <$1 --> murder>>.
        <<$1 --> (/,livingIn,_,{graz})> ==> <$1 --> murder>>.
        <{?who} --> murder>?
        <{tim} --> (/,livingIn,_,{graz})>.
        <{tim} --> (/,livingIn,_,{graz})>. %0%
        <<(*,$1,sunglasses) --> own> ==> <$1 --> [aggressive]>>.
        <(*,{tom},(&,[black],glasses)) --> own>.
        <<$1 --> [aggressive]> ==> <$1 --> murder>>.
        <<$1 --> (/,livingIn,_,{graz})> ==> <$1 --> murder>>.
        <sunglasses --> (&,[black],glasses)>.
        <{?who} --> murder>?
        <(*,toothbrush,plastic) --> made_of>.
        <(&/,<(*,$1,plastic) --> made_of>,<(*,{SELF},$1)-->^lighter>) =/> <$1 --> [heated]>>.
        <<$1 --> [heated]> =/> <$1 --> [melted]>>.
        <<$1 --> [melted]> <|> <$1 --> [pliable]>>.
        <(&/,<$1 --> [pliable]>,<(*,{SELF},$1)-->^reshape>) =/> <$1 --> [hardened]>>.
        <<$1 --> [hardened]> =|> <$1 --> [unscrewing]>>.
        <toothbrush --> object>.
        (&&,<#1 --> object>,<#1 --> [unscrewing]>)!
        <{SELF} --> [hurt]>! %0%
        <{SELF} --> [hurt]>. :|: %0%
        <(&/,<(*,{SELF},wolf) --> close_to>,+1000) =/> <{SELF} --> [hurt]>>.
        <(*,{SELF},wolf) --> close_to>. :|:
        <(&|,<(*,{SELF},$1,FALSE)-->^want>,<(*,{SELF},$1)-->^anticipate>) =|> <(*,{SELF},$1) --> afraid_of>>.
        <(*,{SELF},?what) --> afraid_of>?
        <a --> A>. :|: %1.00;0.90%
        <b --> B>. :|: %1.00;0.90%
        <c --> C>. :|: %1.00;0.90%
        <a --> A>. :|: %1.00;0.90%
        <b --> B>. :|: %1.00;0.90%
        <?1 =/> <c --> C>>?
        <(*,cup,plastic) --> made_of>.
        <cup --> object>.
        <cup --> [bendable]>.
        <toothbrush --> [bendable]>.
        <toothbrush --> object>.
        <(&/,<(*,$1,plastic) --> made_of>,<(*,{SELF},$1)-->^lighter>) =/> <$1 --> [heated]>>.
        <<$1 --> [heated]> =/> <$1 --> [melted]>>.
        <<$1 --> [melted]> <|> <$1 --> [pliable]>>.
        <(&/,<$1 --> [pliable]>,<(*,{SELF},$1)-->^reshape>) =/> <$1 --> [hardened]>>.
        <<$1 --> [hardened]> =|> <$1 --> [unscrewing]>>.
        (&&,<#1 --> object>,<#1 --> [unscrewing]>)!

        <{redInst} |-> [red]>. :|: %1.0%
        <{redInst} |-> [green]>. :|: %0.0%
        <{redInst} |-> [blue]>. :|: %0.0%
        <{greenInst} |-> [red]>. :|: %0.0%
        <{greenInst} |-> [green]>. :|: %1.0%
        <{greenInst} |-> [blue]>. :|: %0.0%
        <{blueInst} |-> [red]>. :|: %0.0%
        <{blueInst} |-> [green]>. :|: %0.0%
        <{blueInst} |-> [blue]>. :|: %1.0%
        <{newColor} |-> [red]>. :|: %0.0%
        <{newColor} |-> [green]>. :|: %0.0%
        <{newColor} |-> [blue]>. :|: %0.1%
        <{?what} <-> {newColor}>? :|:
        <{blueInst} <-> {newColor}>. :|: %1.000000;0.810000%
        <({blueInst} * {newColor}) --> (+ blue)>? :|:
        <({blueInst} * {newColor}) --> (+ blue)>. :|: %1.000000;0.810000%

        <{cat} --> [meowing]>. :|: %0.6%
        <{cat} --> [barking]>. :|: %0.0%
        <(<({#1} ~ {#2}) --> [meowing]> &/ <({SELF} * #1) --> ^say>) =/> G>.
        G! :|:
        <{dog} --> [barking]>. :|: %1.0%
        <{dog} --> [meowing]>. :|: %0.3%
        <({cat} ~ {dog}) --> [meowing]>? :|:
        <({cat} ~ {dog}) --> [meowing]>. :|: %1.000000;0.810000%
        G! :|:
        ({SELF} * cat)

        <(<obstacle --> [left]> &/ ^right) =/> <obstacle --> [free]>>.
        <(<obstacle --> [right]> &/ ^left) =/> <obstacle --> [free]>>.
        <(<obstacle --> [front]> &/ ^left) =/> <obstacle --> [free]>>.
        <((<gripper --> [open]> &/ <obstacle --> [free]>) &/ ^forward) =/> G>.
        <(<gripper --> [hold]> &/ <({SELF} * $obj) --> ^goto>) =/> <$obj --> [left]>>.
        <(<gripper --> [hold]> &/ <({SELF} * $obj) --> ^goto>) =/> <$obj --> [front]>>.
        <(<gripper --> [hold]> &/ <({SELF} * $obj) --> ^goto>) =/> <$obj --> [right]>>.
        <((<gripper --> [open]> &/ <bottle --> [left]>) &/ <({SELF} * bottle) --> ^pick>) =/> G>.
        <((<gripper --> [open]> &/ <bottle --> [front]>) &/ <({SELF} * bottle) --> ^pick>) =/> G>.
        <((<gripper --> [open]> &/ <bottle --> [right]>) &/ <({SELF} * bottle) --> ^pick>) =/> G>.
        <((<gripper --> [hold]> &/ <person --> [left]>) &/ ^drop) =/> G>.
        <((<gripper --> [hold]> &/ <person --> [front]>) &/ ^drop) =/> G>.
        <((<gripper --> [hold]> &/ <person --> [right]>) &/ ^drop) =/> G>.

        "
        // 初步数据处理
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty());

        // 开始测试解析
        let 去掉空格与分隔符 = |s: &str| {
            s.chars()
                .filter(|c| !c.is_whitespace() && *c != ',')
                .collect::<String>()
        };
        let 去掉空格与分隔符_字符集合 = |s: &str| {
            s.chars()
                .filter(|c| !c.is_whitespace() && *c != ',')
                .collect::<HashSet<_>>()
        };
        for narsese in narseses {
            let parsed = parse(narsese).expect("pest解析失败！");
            let parsed_str = FORMAT_ASCII.format_narsese(&parsed);
            // 对齐并展示
            println!("    {narsese:?}\n => {:?}", parsed_str);

            first! {
                narsese == parsed_str => 直接相等的个数 += 1,
                去掉空格与分隔符(narsese) == 去掉空格与分隔符(&parsed_str) => 删去空格与分隔符后相等的个数 += 1,
                去掉空格与分隔符_字符集合(narsese) == 去掉空格与分隔符_字符集合(&parsed_str) => 去掉空格与分隔符_字符重排后相等的个数 += 1,
                _ => 形式有变的.push((去掉空格与分隔符(narsese), 去掉空格与分隔符(&parsed_str), parsed)),
            }
        }

        // 报告
        println!("✅直接相等的个数：{直接相等的个数}");
        println!("✅删去空格与分隔符后相等的个数：{删去空格与分隔符后相等的个数}");
        println!(
            "✅去掉空格与分隔符_字符重排后相等的个数：{去掉空格与分隔符_字符重排后相等的个数}"
        );
        println!("⚠️形式有变的个数：{}", 形式有变的.len());
        for (n, (narsese, parsed_str, parsed)) in 形式有变的.iter().enumerate() {
            // 报告形式有变的
            println!("  {n}:\n\t{narsese:?}\n    =?>\t{:?}", parsed_str);
            // 报告长度明显变化的
            let len_diff = parsed_str.len().abs_diff(narsese.len());
            if len_diff as f64 / narsese.len() as f64 > 0.5 {
                println!("❗长度有较大变化( 变化量={len_diff} )：{parsed:#?}");
            }
        }
        // ! 🚩【2024-03-29 15:12:43】现在假设「去掉空格与分隔符、字符重排后必定相等」
        assert!(形式有变的.is_empty(), "❌出现形式有变的解析结果！");
        println!("测试完毕！");
    }
}
