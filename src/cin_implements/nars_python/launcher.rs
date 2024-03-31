//! NARS-Python运行时的启动器
//! * 🎯允许NARS-Python对原先运行时特别配置功能，同时也支持为NARS-Python定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::runtimes::{CommandVm, CommandVmRuntime};
use navm::vm::VmLauncher;
use std::path::PathBuf;

// ! NARS-Python作为一个独立的`main.exe`，没有默认的启动参数

/// NARS-Python运行时启动器
/// * 🎯配置NARS-Python专有的东西
/// * 🚩基于exe文件启动NARS-Python exe
/// * 🚩【2024-03-25 08:51:30】目前保留原有缩写的大小写风格，与OpenNARS、PyNARS一致
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NARSPython {
    /// exe文件路径
    exe_path: PathBuf,
}

// ! 🚩【2024-03-25 09:37:22】目前暂时不提取至「VmExe」：参考`impl_runtime`根目录说明

impl NARSPython {
    /// 构造函数
    pub fn new(exe_path: impl Into<PathBuf>) -> Self {
        Self {
            // 转换为路径
            exe_path: exe_path.into(),
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher<CommandVmRuntime> for NARSPython {
    fn launch(self) -> CommandVmRuntime {
        // 构造指令，并启动虚拟机
        CommandVm::new(self.exe_path)
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch()
    }
}

// ! 单元测试见[`super`]
