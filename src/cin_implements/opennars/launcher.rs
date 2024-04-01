//! OpenNARS 启动器
//! * 🎯允许OpenNARS对原先运行时特别配置功能，同时也支持为OpenNARS定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时
//! * 🚩通过[`CommandGeneratorJava`]管理启动参数

use super::{input_translate, output_translate};
use crate::{
    cin_implements::common::CommandGeneratorJava,
    runtimes::{CommandGenerator, CommandVm, CommandVmRuntime},
};
use anyhow::Result;
use navm::{
    cmd::Cmd,
    vm::{VmLauncher, VmRuntime},
};
use std::path::PathBuf;

/// OpenNARS Shell启动器
/// * 🎯配置OpenNARS专有的东西
/// * 🚩基于jar文件启动OpenNARS Shell
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenNARS {
    /// Java [`Command`]生成器
    /// * 📌必须有（包含jar文件路径）
    command_generator: CommandGeneratorJava,
    /// NARS的初始音量
    /// * 🚩可能没有：此时不会输入指令
    initial_volume: Option<usize>,
}

impl OpenNARS {
    /// 构造函数
    pub fn new(jar_path: impl Into<PathBuf>) -> Self {
        Self {
            // 传入路径
            command_generator: CommandGeneratorJava::new(jar_path),
            // 其它沿用默认配置
            ..Default::default()
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher<CommandVmRuntime> for OpenNARS {
    fn launch(self) -> Result<CommandVmRuntime> {
        // 构造指令
        // * 🚩细致的Java参数配置，都外包给[`CommandGeneratorJava`]
        let command_java = self.command_generator.generate_command();

        // 构造并启动虚拟机
        let mut vm = CommandVm::from(command_java)
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch()?;

        // 设置初始音量
        if let Some(volume) = self.initial_volume {
            // 输入指令，并在执行错误时打印信息
            if let Err(e) = vm.input_cmd(Cmd::VOL(volume)) {
                println!("无法设置初始音量「{volume}」：{e}");
            }
        };

        // 返回
        Ok(vm)
    }
}

// ! 单元测试见[`super`]
