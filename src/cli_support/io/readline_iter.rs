//! 读取行迭代器
//! * 🎯以迭代器的语法获取、处理用户输入
//! * ❌【2024-04-03 14:28:02】放弃「泛型化改造」：[`Stdin`]能`read_line`，但却没实现[`std::io::BufRead`]

use crate::cli_support::io::output_print::OutputType;
use std::io::{stdin, stdout, Result as IoResult, Stdin, Write};

/// 读取行迭代器
/// * 🚩每迭代一次，请求用户输入一行
/// * ✨自动清空缓冲区
/// * ❌无法在【不复制字符串】的情况下实现「迭代出所输入内容」的功能
///   * ❌【2024-04-02 03:49:56】无论如何都无法实现：迭代器物件中引入就必须碰生命周期
/// * 🚩最终仍需复制字符串：调用处方便使用
/// * ❓是否需要支持提示词
#[derive(Debug)]
pub struct ReadlineIter {
    /// 内置的「输入内容缓冲区」
    buffer: String,
    /// 内置的「标准输入」
    stdin: Stdin,
    /// 输入提示词
    prompt: String,
}

impl ReadlineIter {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            buffer: String::new(),
            stdin: stdin(),
            prompt: prompt.into(),
        }
    }
}

impl Default for ReadlineIter {
    fn default() -> Self {
        Self::new("")
    }
}

/// 实现迭代器
impl Iterator for ReadlineIter {
    type Item = IoResult<String>;

    fn next(&mut self) -> Option<Self::Item> {
        // 清空缓冲区
        self.buffer.clear();
        // 打印提示词
        print!("{}", self.prompt);
        if let Err(e) = stdout().flush() {
            OutputType::Warn.print_line(&format!("无法冲洗输出: {e}"));
        }
        // 读取一行
        // * 📝`stdin()`是懒加载的，只会获取一次，随后返回的都是引用对象
        if let Err(e) = self.stdin.read_line(&mut self.buffer) {
            return Some(Err(e));
        }
        // 返回
        Some(IoResult::Ok(self.buffer.clone()))
    }
}
