//! 命令行虚拟机（构建者）

use super::{InputTranslator, IoTranslators, OutputTranslator};
use crate::process_io::IoProcess;
use anyhow::Result;
use navm::{cmd::Cmd, output::Output};
use std::{ffi::OsStr, process::Command};

/// 命令行虚拟机（构建者）
/// * 🎯配置化构造[`CommandVmRuntime`]
///   * 封装内部「输入输出进程」的「输出侦听器」逻辑
/// * 🚩有关「启动」的流程，放在「虚拟机运行时」[`super::runtime`]中
pub struct CommandVm {
    /// 内部存储的「输入输出进程」
    pub(super) io_process: IoProcess,

    /// [`Cmd`]→进程输入 转译器
    pub(super) input_translator: Option<Box<InputTranslator>>,

    /// 进程输出→[`Output`]转译器
    pub(super) output_translator: Option<Box<OutputTranslator>>,
}

impl CommandVm {
    /// 构造函数
    /// * 🚩接收一个可执行文件路径
    ///   * 📌直接生成[`IoProcess`]对象，无需额外配置
    pub fn new(program_path: impl AsRef<OsStr>) -> Self {
        let io_process = IoProcess::new(program_path);
        Self::from(io_process)
    }

    /// 配置/输入转译器
    /// * 💭何时Rust能给特征起别名。。
    /// * 🚩【2024-04-04 02:06:57】不再需要借走所有权
    ///   * ✅链式操作现在可以使用[`util::manipulate`]简化
    pub fn input_translator(
        &mut self,
        translator: impl Fn(Cmd) -> Result<String> + Send + Sync + 'static,
    ) {
        self.input_translator = Some(Box::new(translator));
    }

    /// 配置/输出转译器
    /// * 🚩【2024-04-04 02:06:57】不再需要借走所有权
    ///   * ✅链式操作现在可以使用[`util::manipulate`]简化
    pub fn output_translator(
        &mut self,
        translator: impl Fn(String) -> Result<Output> + Send + Sync + 'static,
    ) {
        self.output_translator = Some(Box::new(translator));
    }

    /// 配置/输入输出转译器组
    pub fn translators(&mut self, translators: impl Into<IoTranslators>) {
        // 一次实现俩
        let translators = translators.into();
        // 直接赋值
        self.input_translator = Some(translators.input_translator);
        self.output_translator = Some(translators.output_translator);
    }
}

/// 实现/从[`IoProcess`]对象转换为[`CommandVm`]对象
/// * ✅这里的[`IoProcess`]必定是未被启动的：Launch之后会变成其它类型
impl From<IoProcess> for CommandVm {
    fn from(io_process: IoProcess) -> Self {
        Self {
            // IO进程
            io_process,
            // 其它所有置空
            input_translator: None,
            output_translator: None,
        }
    }
}

/// 实现/从[`Command`]对象转换为[`CommandVm`]对象
impl From<Command> for CommandVm {
    fn from(command: Command) -> Self {
        Self::from(IoProcess::from(command))
    }
}
