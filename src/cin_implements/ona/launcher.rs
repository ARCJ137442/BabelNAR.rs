//! ONA运行时的启动器
//! * 🎯允许ONA对原先运行时特别配置功能，同时也支持为ONA定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::{
    cin_implements::common::{generate_command, generate_command_vm},
    runtimes::CommandVmRuntime,
};
use anyhow::Result;
use navm::{
    cmd::Cmd,
    vm::{VmLauncher, VmRuntime},
};
use std::path::PathBuf;
use util::pipe;

/// ONA Shell启动的默认指令参数
/// * 🎯默认预置指令：`[.exe文件路径] shell`
const COMMAND_ARGS_ONA: [&str; 1] = ["shell"];

/// ONA运行时启动器
/// * 🎯配置ONA专有的东西
/// * 🚩基于exe文件启动ONA Shell
///   * 默认预置指令：`[.exe文件路径] shell`
/// * 🚩【2024-03-25 08:51:30】目前保留原有缩写的大小写风格，与OpenNARS、PyNARS一致
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ONA {
    /// exe文件路径
    exe_path: PathBuf,
    /// ONA Shell的初始音量
    /// * 🚩可能没有：此时不会输入指令
    initial_volume: Option<usize>,
}

// ! 🚩【2024-03-25 09:37:22】目前暂时不提取至「VmExe」：预置的`shell`参数需要被处理
// * ✅【2024-03-27 16:07:48】现在通过作为工具的`generate_command`部分实现了代码复用

impl ONA {
    /// 构造函数
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
impl VmLauncher<CommandVmRuntime> for ONA {
    fn launch(self) -> Result<CommandVmRuntime> {
        // 构造并启动虚拟机
        let mut runtime = pipe! {
            self.exe_path
            // 构造指令 | 预置的指令参数
            => generate_command(_, None::<String>, COMMAND_ARGS_ONA.into_iter().by_ref())
            // * 🚩固定的「输入输出转换器」
            => generate_command_vm(_, (input_translate, output_translate))
            // 🔥启动
            => .launch()
        }?;

        // 选择性设置初始音量
        if let Some(volume) = self.initial_volume {
            // 输入指令，并在执行错误时打印信息
            if let Err(e) = runtime.input_cmd(Cmd::VOL(volume)) {
                println!("无法设置初始音量「{volume}」：{e}");
            }
        };
        Ok(runtime)
    }
}

// ! 单元测试见[`super`]
