//! OpenNARS方言
//! * 🎯解析OpenNARS输出，如
//!   * 📄以空格分隔的词项：`(* {SELF})`
//!   * 📄`({SELF} * x)`
//!
//! TODO: 完成语法解析

use narsese::conversion::string::impl_lexical::{
    format_instances::FORMAT_ASCII, structs::ParseResult,
};

/// 以OpenNARS的语法解析出Narsese
/// * 🚩【2024-03-25 21:08:34】目前是直接调用ASCII解析器
///
/// TODO: 兼容OpenNARS特有之语法
/// * 📌重点在其简写的「操作」语法`(^left, {SELF}, x)` => `<(*, {SELF}, x) --> ^left>`
///
/// TODO: 使用[`pest`]将输入的「OpenNARS方言」转换为「词法Narsese」
pub fn parse(input: &str) -> ParseResult {
    FORMAT_ASCII.parse(input)
    // todo!("OpenNARS方言！")
}
