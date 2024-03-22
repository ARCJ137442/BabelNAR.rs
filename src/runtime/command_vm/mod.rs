//! 基于「进程通信」与「IO转译器」的「命令行运行时」

use crate::IoProcessManager;
use navm::{
    cmd::Cmd,
    vm::{Output, VmBuilder, VmRuntime},
};

util::pub_mod_and_pub_use! {
    // 抽象特征
    traits
    // 构建器
    builder
    // 运行时
    runtime
}
