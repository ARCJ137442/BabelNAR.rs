//! 用于封装表示「非公理虚拟机」运行时
//! TODO: 给出一个基于「进程通信」实现[`VM`]的结构

use navm::{
    cmd::Cmd,
    vm::{Output, VmBuilder, VmRuntime},
};

/// 命令行虚拟机（构建者）
/// * 🎯配置化构造[`CommandVmRuntime`]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandVm {
    // TODO: 增加具体字段
}

/// 命令行虚拟机运行时
/// * 🎯封装「进程通信」逻辑
pub struct CommandVmRuntime {
    // TODO: 增加具体字段
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
    fn build(self) -> CommandVmRuntime {
        CommandVmRuntime {}
    }
}
