//! 主模块
//! * ✨进程IO库
//! * ✨通用运行时
//! * ✨运行时的各类实现（可选）

nar_dev_utils::mods! {
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

    // ! 📌【2024-09-12 17:52:40】决议：将`cli_support`迁移到 BabelNAR-CLI.rs 中
    // * 🎯代码功能分离——CIN配置搜索、命令行输出、错误处理等

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
}
