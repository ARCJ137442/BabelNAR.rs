//! 命令行支持
//! * 🎯通用、可选地复用「CIN启动器」等「命令行工具」的内容
//! * 🎯亦可为后续基于UI的应用提供支持

nar_dev_utils::mods! {
    // CIN搜索
    pub cin_search;

    // 输入输出
    pub io;
}

// 错误处理增强
pub mod error_handling_boost;
