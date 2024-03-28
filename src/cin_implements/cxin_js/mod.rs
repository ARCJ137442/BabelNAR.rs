//! 「非公理虚拟机」的ONA运行时
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
    use crate::runtime::{
        test::{await_fetch_until, input_cmd_and_await_contains},
        CommandVmRuntime,
    };
    use narsese::lexical_nse_task as nse_task;
    use navm::{
        cmd::Cmd,
        vm::{VmLauncher, VmRuntime},
    };

    /// 测试用路径
    const CXIN_NARS_JS_PATH: &str = r"..\cxin-nars-py-to-ts\src\cxin-nars-shell.js";

    #[test]
    fn test() {
        // 从别的地方获取exe路径
        let js_path = CXIN_NARS_JS_PATH;
        // 一行代码启动CxinNARS
        let vm = CXinJS::new(js_path).launch();
        // 进入专用测试
        _test_cxin_js(vm)
    }

    /// 专用测试/CXinNARS.js
    pub fn _test_cxin_js(mut vm: CommandVmRuntime) {
        // 专有闭包 | ⚠️无法再提取出另一个闭包：重复借用问题
        let mut input_cmd_and_await =
            |cmd, contains| input_cmd_and_await_contains(&mut vm, cmd, contains);
        // input_cmd_and_await(Cmd::VOL(0), "");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> B>.)), "<A-->B>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<B --> C>.)), "<B-->C>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> C>?)), "<A-->C>?");
        input_cmd_and_await(Cmd::CYC(20), ""); // * CYC无需自动等待

        // 等待回答（字符串）
        await_fetch_until(&mut vm, |_o, raw_content| {
            // ! ❌【2024-03-28 09:51:48】目前CXinNARS能输出导出结论，但无法输出ANSWER
            /* matches!(_o, Output::ANSWER { .. }) && */
            raw_content.contains("<A-->C>.")
        });

        // 终止虚拟机
        vm.terminate().expect("无法终止虚拟机");
        println!("Virtual machine terminated...");
    }
}
