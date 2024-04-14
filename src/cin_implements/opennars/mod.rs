//! 「非公理虚拟机」的OpenNARS运行时
//! * 🚩只提供「一行启动」的功能封装
//!   * 🎯无需自行配置「输入输出转译器」

// 转译器
util::mod_and_pub_use! {
    // 转译器
    translators
    // 启动器
    launcher
    // 方言
    dialect
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtimes::{
            tests::{_test_opennars, test_simple_answer},
            CommandVmRuntime,
        },
        tests::cin_paths::OPENNARS as JAR_PATH_OPENNARS,
    };
    use navm::vm::VmLauncher;

    /// 工具/启动OpenNARS，获得虚拟机运行时
    fn launch_vm() -> CommandVmRuntime {
        // 从别的地方获取jar路径
        let jar_path = JAR_PATH_OPENNARS;
        // 一行代码启动OpenNARS
        OpenNARS::new(jar_path).launch().expect("无法启动虚拟机")
    }

    /// 测试
    #[test]
    #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
    fn test() {
        // 启动OpenNARS虚拟机
        let vm = launch_vm();
        // 直接复用之前对OpenNARS的测试
        _test_opennars(vm)
    }

    /// 测试/通用 | 基于Narsese
    #[test]
    #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
    fn test_universal() {
        // 启动OpenNARS虚拟机
        let vm = launch_vm();
        // 使用通用测试逻辑
        test_simple_answer(vm)
    }
}
