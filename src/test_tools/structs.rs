//! 有关「NAVM测试工具」的数据结构支持
//! * 🎯构造在「NAVM指令」之上的超集，支持与测试有关的数据结构存储
//! * ✨[`NALInput`]：在「直接对应CIN输入输出」的「NAVM指令」之上，引入「等待」「预期」等机制
//! * ✨[`OutputExpectation`]：面向NAL测试，具体实现「预期」机制

use narsese::lexical::Narsese;
use navm::{cmd::Cmd, output::Operation};
use std::time::Duration;

/// NAVM测试中的「NAL输入」
/// * 📌`.nal`文件中一行的超集
/// * 🎯在原有NAVM指令下，扩展与测试有关的功能
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NALInput {
    /// 置入
    /// * 🎯向CIN置入NAVM指令
    Put(Cmd),

    /// 睡眠
    /// * 📌调用[`thread::sleep`]单纯等待一段时间（单位：[`Duration`]）
    ///   * 🚩语法中常用的是秒数，但这里不直接存储
    Sleep(Duration),

    /// 输出等待
    /// * 📌在CIN输出与指定[`Output`]符合后，再继续运行
    /// * 🎯用于结合`IN`等待CIN「回显」
    Await(OutputExpectation),

    /// 对「输出含有」的预期
    /// * 🎯用于「在现有的输出中检查是否任一和指定的[`Output`]符合」
    /// * 📄对应OpenNARS中常有的`''outputMustContain('')`
    ExpectContains(OutputExpectation),
    // 🏗️后续还能有更多
}

/// 输出预期
/// * 📌对应语法中的`output_expectation`结构
/// * 🎯用于统一表示对「NAVM输出」的预期
///   * 🚩除了「原始内容」外，与[`Output`]类型一致
///   * ✨可进行有关「检查范围」「严格性」等更细致的配置，而非仅仅是「文本包含」
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputExpectation {
    /// 预期的「输出类型」
    /// * 🚩可能没有：此时是「通配」情形
    ///   * 对任何可能的输入都适用
    pub output_type: Option<String>,

    /// 预期的「Narsese」字段
    /// * 🚩可能没有：此时是「通配」情形
    ///   * 对任何可能的输入都适用
    /// * 🚩对内部[`Narsese`]会进行**递归匹配**
    pub narsese: Option<Narsese>,

    /// 预期的「NAVM操作」字段
    /// * 🚩可能没有：此时是「通配」情形
    ///   * 对任何可能的输入都适用
    pub operation: Option<Operation>,
}
