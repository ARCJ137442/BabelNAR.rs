//! Node.js 模块启动器
//! * 🎯通用于任何基于Node.js脚本的CIN
//! * 🎯封装「NAVM运行时启动过程」中有关「Node.js启动环境配置」的部分
//! * 🚩从Node.js脚本（`.js`）启动NARS

use crate::runtimes::CommandGenerator;
use std::{path::PathBuf, process::Command};

/// 启动Node.js运行时的命令
const COMMAND_NODE: &str = "node";

/// ! Node.js启动脚本无需附加参数

/// CXinNARS运行时启动器
/// * 🎯配置CXinNARS专有的东西
/// * 🎯以Node.js模块形式启动CXinNARS
/// * 📌没有内置的「音量」配置
/// * 🚩【2024-03-25 08:55:07】基于Node.js模块文件启动CXinNARS
///   * 默认预置指令：``node [`.js`脚本文件路径]``
/// * 🚩【2024-03-25 09:15:07】删去[`Default`]派生：因为可能导致无效的路径
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandGeneratorNodeJS {
    /// Node.js脚本文件路径
    js_path: PathBuf,
    /// 附加的命令行参数
    /// * 📄CXinNARS中用到了`shell`参数
    extra_args: Vec<String>,
}

impl CommandGeneratorNodeJS {
    pub fn new(
        js_path: impl Into<PathBuf>,
        extra_args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        Self {
            // 转换为路径
            js_path: js_path.into(),
            extra_args: extra_args
                .into_iter()
                .map(|s| s.as_ref().to_string())
                .collect::<Vec<_>>(),
        }
    }
}

/// 转换为Node.js启动命令
impl CommandGenerator for CommandGeneratorNodeJS {
    fn generate_command(&self) -> Command {
        // 构造指令
        let mut command_nodejs = Command::new(COMMAND_NODE);

        // 填入路径参数
        command_nodejs.arg(&self.js_path);

        // 填入其它参数
        command_nodejs.args(&self.extra_args);

        // 返回
        command_nodejs
    }
}
