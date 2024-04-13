//! OpenJunars 启动器
//! * 🎯允许OpenJunars对原先运行时特别配置功能，同时也支持为OpenJunars定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时
//! * 🚩通过[`CommandGeneratorJulia`]管理启动参数

use super::{input_translate, output_translate};
use crate::{
    cin_implements::common::CommandGeneratorJulia,
    runtimes::{CommandGenerator, CommandVm, CommandVmRuntime},
};
use anyhow::Result;
use nar_dev_utils::manipulate;
use navm::vm::VmLauncher;
use std::path::PathBuf;

/// OpenJunars运行时启动器
/// * 🎯配置OpenJunars专有的东西
/// * 🎯以Julia模块形式启动OpenJunars
/// * 📌没有内置的「音量」配置
/// * 🚩【2024-03-25 08:55:07】基于Julia模块文件启动OpenJunars
///   * 默认预置指令：``julia [`.jl`脚本文件路径]``
/// * 🚩【2024-03-25 09:15:07】删去[`Default`]派生：因为可能导致无效的路径
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenJunars {
    /// Julia脚本文件路径
    command_generator: CommandGeneratorJulia,
}

impl OpenJunars {
    pub fn new(jl_path: impl Into<PathBuf>) -> Self {
        Self {
            // 转换为路径
            command_generator: CommandGeneratorJulia::new(jl_path),
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher for OpenJunars {
    type Runtime = CommandVmRuntime;
    fn launch(self) -> Result<CommandVmRuntime> {
        // 构造指令
        let command = self.command_generator.generate_command();

        // 构造并启动虚拟机
        manipulate!(
            CommandVm::from(command)
            // * 🚩固定的「输入输出转译器」
            => .input_translator(input_translate)
            => .output_translator(output_translate)
        )
        // 🔥启动
        .launch()
    }
}

// ! 单元测试见[`super`]
