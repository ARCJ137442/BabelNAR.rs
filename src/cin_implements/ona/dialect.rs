//! ONA方言
//! * 🎯解析ONA输出，如
//!   * 📄以空格分隔的词项：`(* {SELF})`
//!   * 📄`({SELF} * x)`
//!
//! TODO: 完成语法解析

use narsese::conversion::string::impl_lexical::{
    format_instances::FORMAT_ASCII, structs::ParseResult,
};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)] // ! ↓ 必须从项目根目录开始
#[grammar = "src/cin_implements/ona/dialect_ona.pest"]
pub struct DialectParser;

/// 使用[`pest`]将输入的「ONA方言」转换为「词法Narsese」
/// 以ONA的语法解析出Narsese
/// * 🚩【2024-03-25 21:08:34】目前是直接调用ASCII解析器
///
/// TODO: 兼容ONA的方言语法
/// * 📌重点在「用空格分隔乘积词项/中缀情形」的语法
///   * 📄`(* {SELF})`
///   * 📄`({SELF} * x)`
pub fn parse(input: &str) -> ParseResult {
    let _ = dbg!(DialectParser::parse(Rule::narsese, input));
    FORMAT_ASCII.parse(input)
    // #![allow(unused)]
    // todo!("ONA方言！")
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试/方言解析器 🚧
    #[test]
    fn test_dialect_parser() {
        let x = parse("<A --> B>.");
        let _ = dbg!(x);
    }
}
