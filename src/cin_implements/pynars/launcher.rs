//! Python模块 启动器
//! * 📌PyNARS运行时的启动器
//! * 🎯允许PyNARS对原先运行时特别配置功能，同时也支持为PyNARS定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时
//! * 🚩通过[`CommandGeneratorPython`]管理启动参数

use super::{input_translate, output_translate};
use crate::{
    cin_implements::common::CommandGeneratorPython,
    runtime::{CommandGenerator, CommandVm, CommandVmRuntime},
};
use navm::vm::VmLauncher;
use std::path::PathBuf;

/// PyNARS运行时启动器
/// * 🎯配置PyNARS专有的东西
/// * 🎯以Python模块形式启动PyNARS
/// * 📌没有内置的「音量」配置
///   * ⚠️该配置参考的是PyNARS的`ConsolePlus`模块
/// * 🚩【2024-03-25 08:55:07】基于Python模块文件启动PyNARS Shell
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PyNARS {
    /// 命令生成器
    command_generator: CommandGeneratorPython,
}

impl PyNARS {
    pub fn new(root_path: impl Into<PathBuf>, module_path: &str) -> Self {
        Self {
            command_generator: CommandGeneratorPython::new(root_path, module_path),
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher<CommandVmRuntime> for PyNARS {
    fn launch(self) -> CommandVmRuntime {
        // 构造指令
        let command = self.command_generator.generate_command();

        // 构造并启动虚拟机
        CommandVm::from(command)
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch()
    }
}

// ! 单元测试见[`super`]
