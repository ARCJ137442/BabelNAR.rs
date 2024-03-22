//! 命令行虚拟机 运行时

/// 命令行虚拟机运行时
/// * 🎯封装「进程通信」逻辑
pub struct CommandVmRuntime<I, O>
where
    I: InputTranslator,
    O: OutputTranslator,
{
    /// 封装的「进程管理者」
    /// * 🚩使用[`IoProcessManager`]封装「进程通信」的逻辑细节
    io_process: IoProcessManager,

    /// [`Cmd`]→进程输入 转译器
    input_translator: I,

    /// 进程输出→[`Output`]转译器
    output_translator: O,
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
        // TODO: 增加启动流程
        todo!()
        // CommandVmRuntime {

        // }
    }
}
