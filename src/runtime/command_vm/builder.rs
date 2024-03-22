//! 命令行虚拟机（构建者）

/// 命令行虚拟机（构建者）
/// * 🎯配置化构造[`CommandVmRuntime`]
/// * 🚩有关「启动」的流程，放在「虚拟机运行时」[`super::runtime`]中
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandVm {
    // TODO: 增加具体字段
}

impl CommandVm {
    /// 构造函数
    pub fn new() -> Self {
        Self {}
    }
}
