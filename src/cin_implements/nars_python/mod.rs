//! 「非公理虚拟机」的NARS-Python运行时
//! * 🚩只提供「一行启动」的功能封装
//!   * 🎯无需自行配置「输入输出转译器」

// 转译器
util::mod_and_pub_use! {
    // 方言（Narsese格式）
    dialect
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
    use narsese::conversion::string::impl_lexical::shortcuts::*;
    use navm::{
        cmd::Cmd,
        vm::{VmLauncher, VmRuntime},
    };

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
    /// * ❌【2024-03-26 01:42:14】目前还没法真正截取到输出
    pub(crate) fn _test_nars_python(mut vm: CommandVmRuntime) {
        // 等待几秒钟，让exe的界面显示出来
        std::thread::sleep(std::time::Duration::from_secs(2));

        vm.input_cmd(Cmd::NSE(nse_task!(<A --> B>.)))
            .expect("无法输入NAVM指令");
        vm.input_cmd(Cmd::NSE(nse_task!(<B --> C>.)))
            .expect("无法输入NAVM指令");
        vm.input_cmd(Cmd::NSE(nse_task!(<A --> C>?)))
            .expect("无法输入NAVM指令");

        std::thread::sleep(std::time::Duration::from_secs(4));

        // 终止虚拟机运行时
        vm.terminate().expect("无法终止虚拟机");
    }

    /* // ! 【2024-03-26 01:44:27】NARS-Python输出崩溃的内容：
        running 1 test
    Started process: 65784
    Traceback (most recent call last):
      File "main.py", line 122, in <module>
      File "main.py", line 118, in main
      File "NARS.py", line 54, in run
      File "NARS.py", line 63, in do_working_cycle
      File "InputChannel.py", line 74, in process_pending_sentence
      File "InputChannel.py", line 87, in process_sentence
      File "NARS.py", line 247, in process_task
      File "NARS.py", line 323, in process_question_task
      File "NARS.py", line 491, in process_sentence_semantic_inference
      File "NARSInferenceEngine.py", line 73, in do_semantic_inference_two_premise
    AttributeError: 'NoneType' object has no attribute 'frequency'
    [38676] Failed to execute script 'main' due to unhandled exception!
    Fatal Python error: could not acquire lock for <_io.BufferedReader name='<stdin>'> at interpreter shutdown, possibly due to daemon threads
    Python runtime state: finalizing (tstate=00000213FB525D60)

    Thread 0x00017e0c (most recent call first):
      File "InputChannel.py", line 25 in get_user_input
      File "threading.py", line 870 in run
      File "threading.py", line 932 in _bootstrap_inner
      File "threading.py", line 890 in _bootstrap

    Current thread 0x00013918 (most recent call first):
    <no Python frame>
    成功: 已终止 PID 为 65784 的进程。
    test cin_implements::nars_python::tests::test ... ok

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 6.56s
    */
}
