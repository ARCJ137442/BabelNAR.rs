//! 输入输出转译
//! * ✨Cmd输入转译：直接将[`Cmd`]转换为字符串形式
//! * ✨NAVM_JSON输出转译：基于[`serde_json`]直接从JSON字符串读取[`Output`]

use anyhow::Result;
use navm::{cmd::Cmd, output::Output};
extern crate serde_json;

/// Cmd输入转译
/// * 🚩直接将[`Cmd`]转换为字符串形式
/// * 📌总是成功
pub fn input_translate(cmd: Cmd) -> Result<String> {
    Ok(cmd.to_string())
}

/// NAVM_JSON输出转译
/// * 🚩基于[`serde_json`]直接从JSON字符串读取[`Output`]
pub fn output_translate(content_raw: String) -> Result<Output> {
    match serde_json::from_str(&content_raw) {
        // 解析成功⇒返回
        Ok(output) => Ok(output),
        // 解析失败⇒转为`OTHER`
        Err(..) => Ok(Output::OTHER {
            content: content_raw,
        }),
    }
}
