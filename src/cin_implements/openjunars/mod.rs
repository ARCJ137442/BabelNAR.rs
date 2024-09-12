//! 「非公理虚拟机」的OpenJunars运行时
//! * 🚩只提供「一行启动」的功能封装
//!   * 🎯无需自行配置「输入输出转译器」

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
    #![allow(unused)]

    use super::*;
    use crate::{runtimes::CommandVmRuntime, tests::cin_paths::OPENJUNARS};
    use narsese::conversion::string::impl_lexical::shortcuts::*;
    use navm::{
        cmd::Cmd,
        vm::{VmLauncher, VmRuntime},
    };

    #[test]
    #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
    fn test() {
        // 从别的地方获取jl路径
        let jl_path = OPENJUNARS;
        // 一行代码启动OpenJunars
        let vm = OpenJunars::new(jl_path).launch().expect("无法启动虚拟机");
        // 运行专有测试
        // ! ❌【2024-03-25 13:56:21】目前无法截取到Julia运行时输出，弃用
        _test_open_junars(vm)
    }

    /// 测试/OpenJunars
    pub(crate) fn _test_open_junars(mut vm: CommandVmRuntime) {
        // ! ❌【2024-03-25 13:55:57】无效：似乎无法截取到Julia运行时输出

        vm.input_cmd(Cmd::NSE(nse_task!(<A --> B>.)))
            .expect("无法输入指令");

        // 等待四秒钟，让Junars启动
        std::thread::sleep(std::time::Duration::from_secs(4));

        vm.input_cmd(Cmd::NSE(nse_task!(<A --> B>.)))
            .expect("无法输入指令");

        std::thread::sleep(std::time::Duration::from_secs(1));

        vm.input_cmd(Cmd::CYC(1)).expect("无法输入指令");

        std::thread::sleep(std::time::Duration::from_secs(1));

        vm.input_cmd(Cmd::NSE(nse_task!(<A --> B>?)))
            .expect("无法输入指令");

        std::thread::sleep(std::time::Duration::from_secs(1));

        vm.input_cmd(Cmd::CYC(1)).expect("无法输入指令");

        std::thread::sleep(std::time::Duration::from_secs(3));

        // 尝试截获其所有输出
        // * 🚩【2024-04-13 16:10:27】目前经由Julia侧`flush(stdout)`，仍然无法捕获
        // * 有输出`[ Info: Answer: <A-->B>. %1.0;0.9%`，但无法被程序捕获为文本
        while let Ok(Some(output)) = vm.try_fetch_output() {
            dbg!(output);
        }

        std::thread::sleep(std::time::Duration::from_secs(2));

        // 终止虚拟机运行时
        vm.terminate().expect("无法终止虚拟机");
    }
}
