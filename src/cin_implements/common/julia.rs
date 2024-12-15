//! Julia 模块启动器
//! * 🎯通用于任何基于Julia源码的CIN，不仅仅是OpenJunars
//! * 🎯封装「NAVM运行时启动过程」中有关「Julia启动环境配置」的部分
//! * 🚩从Julia脚本（`.jl`）启动NARS

use crate::runtimes::CommandGenerator;
use std::{path::PathBuf, process::Command};

/// 启动Julia运行时的命令
const COMMAND_JULIA: &str = "julia";

// ! Julia启动脚本无需附加参数

/// OpenJunars运行时启动器
/// * 🎯配置OpenJunars专有的东西
/// * 🎯以Julia模块形式启动OpenJunars
/// * 📌没有内置的「音量」配置
/// * 🚩【2024-03-25 08:55:07】基于Julia模块文件启动OpenJunars
///   * 默认预置指令：``julia [`.jl`脚本文件路径]``
/// * 🚩【2024-03-25 09:15:07】删去[`Default`]派生：因为可能导致无效的路径
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandGeneratorJulia {
    /// Julia脚本文件路径
    jl_path: PathBuf,
}

impl CommandGeneratorJulia {
    pub fn new(jl_path: impl Into<PathBuf>) -> Self {
        Self {
            // 转换为路径
            jl_path: jl_path.into(),
        }
    }
}

/// 转换为Julia启动命令
impl CommandGenerator for CommandGeneratorJulia {
    fn generate_command(&self) -> Command {
        // 构造指令
        let mut command_julia = Command::new(COMMAND_JULIA);

        // 填入路径参数
        command_julia.arg(&self.jl_path);

        // 返回
        command_julia
    }
}
