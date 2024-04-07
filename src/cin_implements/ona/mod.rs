//! 「非公理虚拟机」的ONA运行时
//! * 🚩只提供「一行启动」的功能封装
//!   * 🎯无需自行配置「输入输出转译器」

// 转译器
util::mod_and_pub_use! {
    // 转译器
    translators
    // 启动器
    launcher
    // 方言 | 【2024-03-27 18:42:50】使用`pest`库解析特殊语法
    dialect
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtimes::{
            tests::{_test_ona, test_simple_answer},
            CommandVmRuntime,
        },
        tests::cin_paths::ONA as EXE_PATH_ONA,
    };
    use navm::vm::VmLauncher;

    /// 工具/启动ONA，获得虚拟机运行时
    fn launch_vm() -> CommandVmRuntime {
        // 从别的地方获取exe路径
        let exe_path = EXE_PATH_ONA;
        // 一行代码启动ONA
        ONA::new(exe_path).launch().expect("无法启动虚拟机")
    }

    #[test]
    fn test() {
        // 启动ONA虚拟机
        let vm = launch_vm();
        // 直接复用之前对ONA的测试
        _test_ona(vm)
    }

    /// 测试/通用 | 基于Narsese
    #[test]
    fn test_universal() {
        // 启动ONA虚拟机
        let vm = launch_vm();
        // 使用通用测试逻辑
        test_simple_answer(vm)
    }
}
