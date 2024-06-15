//! 主模块
//! * ✨进程IO库
//! * ✨通用运行时
//! * ✨运行时的各类实现（可选）

// 实用库别名
pub extern crate nar_dev_utils as util;

util::mods! {
    // 必选模块 //

    // 进程IO
    pub process_io;

    // NAVM运行时
    pub runtimes;

    // 输出处理者
    pub output_handler;

    // 可选模块 //

    // 各CIN的启动器、运行时实现
    "cin_implements" => pub cin_implements;

    // 命令行支持
    "cli_support" => pub cli_support;

    // 测试工具集
    "test_tools" => pub test_tools;
}

/// 单元测试
/// * 🎯为下属单元测试提供测试支持
///   * 📄测试用配置文件的名称及路径
///   * 📄各测试用CIN的内部路径（`executables`）
/// * ❌【2024-04-07 08:50:07】已知问题：不同crate的`[cfg(test)]`代码无法互通
///   * 🚩【2024-04-07 08:52:36】当下解决方案：禁用`#[cfg(test)]`
///   * 📌以**十数个常量**的编译成本，换得**更方便的测试可维护性**（无需复制代码）
// #[cfg(test)]
pub mod tests {
    #![allow(unused_variables)]

    /// 实用宏/简化字符串常量
    macro_rules! str_const {
        ($(
            $(#[$m:meta])*
            $name:ident = $value:literal $(;)?
        )*) => {$(
            $(#[$m])*
            pub const $name: &str = $value;
        )*};
    }

    /// 测试用配置文件路径
    /// * 🎯后续其它地方统一使用该处路径
    /// * 📌相对路径の根目录：项目根目录（`Cargo.toml`所在目录）
    /// * ⚠️只与配置文件路径有关，不与CIN位置有关
    ///   * 💭后续若在不同工作环境中，需要调整配置文件中有关「CIN位置」的信息
    /// * ⚠️此处所涉及的CIN不附带于源码中，而是**另行发布**
    ///   * ❗部分CIN涉及c
    pub mod config_paths {
        str_const! {

            /// 用于「启动参数解析」的测试环境
            ARG_PARSE_TEST =
                "./src/tests/cli/config/_arg_parse_test.opennars.hjson"

            /// OpenNARS
            OPENNARS = "./src/tests/cli/config/cin_opennars.hjson"
            /// OpenNARS
            OPENNARS_158 = "./src/tests/cli/config/cin_opennars_158.hjson"
            /// ONA
            ONA = "./src/tests/cli/config/cin_ona.hjson"
            /// PyNARS
            PYNARS = "./src/tests/cli/config/cin_pynars.hjson"
            /// CXinJS
            CXIN_JS = "./src/tests/cli/config/cin_cxin_js.hjson"
            /// 原生IL-1
            NATIVE_IL_1 = "./src/tests/cli/config/cin_native_il_1.hjson"

            /// 预引入/NAL测试环境
            PRELUDE_TEST = "./src/tests/cli/config/prelude_test.hjson"
            /// NAL/简单演绎
            NAL_SIMPLE_DEDUCTION = "./src/tests/cli/config/nal_simple_deduction.hjson"
            /// NAL/高阶演绎
            NAL_HIGHER_DEDUCTION = "./src/tests/cli/config/nal_higher_deduction.hjson"
            /// NAL/自变量消除
            NAL_I_VAR_ELIMINATION = "./src/tests/cli/config/nal_i_var_elimination.hjson"
            /// NAL/时间归纳
            NAL_TEMPORAL_INDUCTION = "./src/tests/cli/config/nal_temporal_induction.hjson"
            /// NAL/操作
            NAL_OPERATION = "./src/tests/cli/config/nal_operation.hjson"
            /// NAL/简单操作
            NAL_SIMPLE_OPERATION = "./src/tests/cli/config/nal_simple_operation.hjson"
            /// NAL/真值通配
            NAL_TRUTH_WILDCARD = "./src/tests/cli/config/nal_truth_wildcard.hjson"

            /// Websocket
            WEBSOCKET = "./src/tests/cli/config/websocket.hjson"
            /// Matriangle服务器
            MATRIANGLE_SERVER = "./src/tests/cli/config/matriangle_server.hjson"
        }
    }

    /// 测试用CIN路径
    /// * 🎯后续其它地方统一使用该处路径
    /// * 🎯存储测试用的本地CIN
    ///   * ⚠️该处CIN被自动忽略，不附带于源码中，需要另外的运行时包以启动
    /// * 📌相对路径の根目录：项目根目录（`Cargo.toml`所在目录）
    pub mod cin_paths {
        str_const! {
            OPENNARS = "./executables/opennars-304-T-modified.jar"
            ONA = "./executables/ONA.exe"
            PYNARS_ROOT = "./executables/PyNARS"
            PYNARS_MODULE = "pynars.ConsolePlus"
            NARS_PYTHON = "./executables/nars-python-main.exe"
            CXIN_JS = "./executables/cxin-nars-shell.js"
            OPENJUNARS = "./executables/OpenJunars/launch.jl"
        }
    }

    /// 测试用宏/找不到路径即退出
    /// * 🚩输入一个`&str`，构建`&Path`并在其不存在时退出程序，或返回该`&Path`对象
    #[macro_export]
    macro_rules! exists_or_exit {
        ($path:expr) => {{
            let path = std::path::Path::new($path);
            if !path.exists() {
                println!("所需路径 {path:?} 不存在，已自动退出");
                std::process::exit(0)
            }
            path
        }};
    }
}
