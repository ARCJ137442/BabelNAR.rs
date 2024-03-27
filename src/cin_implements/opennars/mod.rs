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
    use crate::runtime::test::{_test_opennars, JAR_PATH_OPENNARS};
    use navm::vm::VmLauncher;

    #[test]
    fn test() {
        // 从别的地方获取jar路径
        let jar_path = JAR_PATH_OPENNARS;
        // 一行代码启动OpenNARS
        let vm = OpenNARS::new(jar_path).launch();
        // 直接复用之前对OpenNARS的测试
        _test_opennars(vm)
    }
}
