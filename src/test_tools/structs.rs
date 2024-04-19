//! 有关「NAVM测试工具」的数据结构支持
//! * 🎯构造在「NAVM指令」之上的超集，支持与测试有关的数据结构存储
//! * ✨[`NALInput`]：在「直接对应CIN输入输出」的「NAVM指令」之上，引入「等待」「预期」等机制
//! * ✨[`OutputExpectation`]：面向NAL测试，具体实现「预期」机制

use narsese::{conversion::string::impl_lexical::format_instances::FORMAT_ASCII, lexical::Narsese};
use navm::{cmd::Cmd, output::Operation};
use std::{fmt::Display, time::Duration};
use thiserror::Error;

/// NAVM测试中的「NAL输入」
/// * 📌`.nal`文件中一行的超集
/// * 🎯在原有NAVM指令下，扩展与测试有关的功能
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NALInput {
    /// 置入
    /// * 🎯向CIN置入NAVM指令
    Put(Cmd),

    /// 睡眠
    /// * 📄语法示例：`''sleep 1s`
    /// * 📌调用[`thread::sleep`]单纯等待一段时间（单位：[`Duration`]）
    ///   * 🚩语法中常用的是秒数，但这里不直接存储
    Sleep(Duration),

    /// 输出等待
    /// * 📄语法示例：`''await: IN <A --> B>.`
    /// * 📌在CIN输出与指定[`Output`]符合后，再继续运行
    /// * 🎯用于结合`IN`等待CIN「回显」
    Await(OutputExpectation),

    /// 对「输出含有」的预期
    /// * 📄语法示例：`''expect-contains: ANSWER <A --> C>.`
    /// * 🎯用于「在现有的输出中检查是否任一和指定的[`Output`]符合」
    /// * 📄对应OpenNARS中常有的`''outputMustContain('')`
    ExpectContains(OutputExpectation),

    /// 对「输出含有」的循环预期
    /// * 📄语法示例：`''expect-cycle(500, 10, 0.1s): ANSWER <A --> C>.`
    /// * 🎯用于「在『最大步数』的限定下循环尝试获取『期望的输出』，未获得预期输出⇒预期失败」
    /// * 🚩循环指定周期（最大步数），并在其中检查预期；
    ///   * 每步进1周期后，检查NAVM输出预期，有⇒终止，打印输出`expect-cycle(【次数】): 【输出】`
    ///   * 若循环后仍无，视作「预期不符」
    /// * 📄在「最大步数=0」的情形之下，`expect-cycle(0)`等价于[`expect-contains`](NALInput::ExpectContains)
    ExpectCycle(usize, usize, Option<Duration>, OutputExpectation),

    /// 保存「输出缓存」到指定文件
    /// * 📄语法示例：`''save-outputs: outputs.log`
    /// * 🎯用于「将现有所有输出以『NAVM输出的JSON格式』存档至指定文件中」
    SaveOutputs(String),

    /// 终止虚拟机
    /// * 🎯用于「预加载NAL『测试』结束后，程序自动退出/交给用户输入」
    /// * 📄语法示例：
    ///   * `''terminate`
    ///   * `''terminate(if-no-user): 异常的退出消息！`
    /// * 🔧可选的「子参数」
    ///   * `if-no-user`：仅在「用户无法输入」时退出
    Terminate {
        /// 仅在「用户无法输入」时退出
        /// * 🎯用于「测试完毕后交给用户输入」的测试
        if_not_user: bool,

        /// 退出的返回值
        /// * 🎯用于「测试完毕后向外部传递结果」的测试
        /// * 💭始终注意这只是个线性执行的指令，不要做得太复杂
        /// * 🚩【2024-04-02 23:56:34】目前不在此装载[`anyhow::Error`]类型：避免复杂
        result: std::result::Result<(), String>,
    },
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

impl Display for OutputExpectation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OutputExpectation {{ {} {} {} }}",
            self.output_type.as_deref().unwrap_or("*"),
            match &self.narsese {
                Some(narsese) => FORMAT_ASCII.format_narsese(narsese),
                None => "*".to_string(),
            },
            self.operation
                .as_ref()
                .map(|op| op.to_string())
                .unwrap_or("*".to_string()),
        )
    }
}

/// 预期错误
/// * 🎯用于定义可被识别的「NAL预期失败/脱离预期」错误
/// * 🚩使用[`thiserror`]快捷定义
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum OutputExpectationError {
    /// 输出未包含预期
    /// * 🎯对应[`NALInput::ExpectContains`]
    /// * 📝此处`{0:?}`参照<https://lib.rs/crates/thiserror>
    #[error("输出内容中不存在符合预期的输出：{0}")]
    ExpectedNotExists(OutputExpectation),
}
