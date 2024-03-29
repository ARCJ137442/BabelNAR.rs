//! 「非公理虚拟机」的OpenJunars运行时
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
    #![allow(unused)]

    use super::*;
    use crate::runtime::{tests::JL_PATH_OPEN_JUNARS, CommandVmRuntime};
    use narsese::conversion::string::impl_lexical::shortcuts::*;
    use navm::{
        cmd::Cmd,
        vm::{VmLauncher, VmRuntime},
    };

    #[test]
    fn test() {
        // 从别的地方获取jl路径
        let jl_path = JL_PATH_OPEN_JUNARS;
        // 一行代码启动OpenJunars
        let vm = OpenJunars::new(jl_path).launch();
        // 运行专有测试
        // ! ❌【2024-03-25 13:56:21】目前无法截取到Julia运行时输出，弃用
        // _test_opennars(vm)
        _test_open_junars(vm)
    }

    /// 测试/OpenJunars
    pub(crate) fn _test_open_junars(mut vm: CommandVmRuntime) {
        // ! ❌【2024-03-25 13:55:57】无效：似乎无法截取到Julia运行时输出

        // vm.input_cmd(Cmd::NSE(nse_task!(<A --> B>.)))
        //     .expect("无法输入指令");

        // // 等待四秒钟，让Junars启动
        // std::thread::sleep(std::time::Duration::from_secs(1));

        // vm.input_cmd(Cmd::NSE(nse_task!(<A --> B>.)))
        //     .expect("无法输入指令");
        // std::thread::sleep(std::time::Duration::from_secs(6));

        // 终止虚拟机运行时
        vm.terminate().expect("无法终止虚拟机");
    }
}
