//! 命令行虚拟机 运行时
//! * ✨核心内容
//!   * ⇄ 基于「进程通信」的消息互转
//!     * 📌核心IO流程：
//!       1. NAVM指令[`Cmd`] >>> 进程输入 >>> 子进程
//!       2. 子进程 >>> 进程输出 >>> NAVM输出[`Output`]
//!     * 🚩实现方式：两处转译器

use super::{CommandVm, InputTranslator, OutputTranslator};
use crate::process_io::IoProcessManager;
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmBuilder, VmRuntime},
};

/// 命令行虚拟机运行时
/// * 🎯封装「进程通信」逻辑
pub struct CommandVmRuntime {
    /// 封装的「进程管理者」
    /// * 🚩使用[`IoProcessManager`]封装「进程通信」的逻辑细节
    process: IoProcessManager,

    /// [`Cmd`]→进程输入 转译器
    input_translator: Box<InputTranslator>,

    /// 进程输出→[`Output`]转译器
    output_translator: Box<OutputTranslator>,
    // TODO: 输出侦听系统
}

impl VmRuntime for CommandVmRuntime {
    fn input_cmd(&mut self, cmd: Cmd) {
        todo!()
    }

    fn store_output(&mut self, output: Output) {
        todo!()
    }

    fn fetch_output(&mut self) -> Option<Output> {
        todo!()
    }

    fn add_output_listener<Listener>(&mut self, listener: Listener)
    where
        Listener: FnMut(Output) -> Option<Output>,
    {
        todo!()
    }

    fn iter_output_listeners<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a mut dyn FnMut(Output) -> Option<Output>> + 'a> {
        todo!()
    }
}

impl VmBuilder<CommandVmRuntime> for CommandVm {
    fn launch(self) -> CommandVmRuntime {
        CommandVmRuntime {
            // 启动内部的「进程管理者」
            process: self.io_process.launch(),
            // 输入转译器
            input_translator: self
            .input_translator
            // 默认值：直接调用Cmd的`to_string`方法 | 使用NAVM Cmd语法
            .unwrap_or(Box::new(|cmd| cmd.to_string())),
            // 输出转译器
            output_translator: self
                .output_translator
                // 默认值：直接归入「其它」输出 | 约等于不分类
                .unwrap_or(Box::new(|content| Output::OTHER { content })),
            // TODO: 其它
        }
    }
}
