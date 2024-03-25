//! ONA运行时的启动器
//! * 🎯允许ONA对原先运行时特别配置功能，同时也支持为ONA定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::runtime::{CommandVm, CommandVmRuntime};
use navm::vm::VmLauncher;
use std::{path::PathBuf, process::Command};

/// ONA Shell启动的默认指令参数
/// * 🎯默认预置指令：`[.exe文件路径] shell`
const COMMAND_ARGS_ONA: [&str; 1] = ["shell"];

/// ONA运行时启动器
/// * 🎯配置ONA专有的东西
/// * 🎯以Java运行时专有形式启动ONA
/// * 🚩基于exe文件启动ONA Shell
///   * 默认预置指令：`[.exe文件路径] shell`
/// * 📌【2024-03-25 08:41:16】目前跟随Rust命名规则，仅首字母大写
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ona {
    /// exe文件路径
    exe_path: PathBuf,
    /// ONA Shell
    default_volume: Option<usize>,
}

impl Ona {
    pub fn new(exe_path: impl Into<PathBuf>) -> Self {
        Self {
            // 转换为路径
            exe_path: exe_path.into(),
            // 其它全是`None`
            ..Default::default()
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher<CommandVmRuntime> for Ona {
    fn launch(self) -> CommandVmRuntime {
        // 构造指令
        let mut command = Command::new(self.exe_path);
        // * 📝这里的`args`、`arg都返回的可变借用。。
        command.args(COMMAND_ARGS_ONA);

        // 构造并启动虚拟机
        CommandVm::from_io_process(command.into())
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch()
    }
}

// ! 单元测试见[`super`]
