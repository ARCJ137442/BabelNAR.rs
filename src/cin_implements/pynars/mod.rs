//! 「非公理虚拟机」的PyNARS运行时
//! * 🚩只提供「一行启动」的功能封装
//!   * 🎯无需自行配置「输入输出转译器」
//!
//! * ❌【2024-03-25 13:00:14】目前无法在Rust侧解决「杀死子进程后，Python继续输出无关信息」的问题
//!   * 📄主要形式：子进程结束后打印错误堆栈，输出`OSError: [Errno 22] Invalid argument`
//!   * ❗无法被Rust捕获，可能是Python运行时的问题（输出未链接到管道）

// 转译器
nar_dev_utils::mod_and_pub_use! {
    // 转译器
    translators
    // 启动器
    launcher
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtimes::{
            tests::{_test_pynars, test_simple_answer},
            CommandVmRuntime,
        },
        tests::cin_paths::{PYNARS_MODULE, PYNARS_ROOT},
    };
    use navm::vm::VmLauncher;

    /// 工具/启动PyNARS，获得虚拟机运行时
    fn launch_vm() -> CommandVmRuntime {
        // 从别的地方获取Python模块根目录、模块自身路径
        let root_path = PYNARS_ROOT;
        let module_path = PYNARS_MODULE;
        // 一行代码启动PyNARS | `python -m pynars.Console` @ "..\..\PyNARS-dev"
        PyNARS::new(root_path, module_path)
            .launch()
            .expect("无法启动虚拟机")
    }

    /// 测试/先前PyNARS测试
    #[test]
    fn test() {
        // 启动PyNARS虚拟机
        let vm = launch_vm();
        // 直接复用之前对PyNARS的测试
        _test_pynars(vm)
    }

    /// 测试/通用 | 基于Narsese
    #[test]
    fn test_universal() {
        // 启动PyNARS虚拟机
        let vm = launch_vm();
        // 使用通用测试逻辑
        test_simple_answer(vm)
    }
}
