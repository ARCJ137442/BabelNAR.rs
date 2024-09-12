//! 用于封装表示「非公理虚拟机」的通用运行时支持
//! * 📌不与特定的CIN相关
//!   * 📄一个「命令行运行时」可同时适用于OpenNARS、ONA、NARS-Python……

nar_dev_utils::mods! {
    // 命令行运行时
    pub pub command_vm;
}
