//! 「非公理虚拟机」的NARS-Python运行时
//! * 🚩只提供「一行启动」的功能封装
//!   * 🎯无需自行配置「输入输出转译器」

// 转译器
util::mod_and_pub_use! {
    // 转译器
    translators
    // 启动器
    launcher
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{test::EXE_PATH_NARS_PYTHON, CommandVmRuntime};
    use navm::vm::{VmLauncher, VmRuntime};

    #[test]
    fn test() {
        // 从别的地方获取exe路径
        let exe_path = EXE_PATH_NARS_PYTHON;
        // 一行代码启动NARS-Python
        let vm = NARSPython::new(exe_path).launch();
        // 运行专有测试
        _test_nars_python(vm)
    }

    /// 测试/NARS-Python
    pub(crate) fn _test_nars_python(vm: CommandVmRuntime) {
        // TODO: 实际的测试代码

        // 等待四秒钟，让exe的界面显示出来
        std::thread::sleep(std::time::Duration::from_secs(4));

        // 终止虚拟机运行时
        vm.terminate().expect("无法终止虚拟机");
    }
}
