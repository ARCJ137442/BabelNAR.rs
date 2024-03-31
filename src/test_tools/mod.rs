//! 有关NAVM的**测试工具集**支持
//! * 🎯提供可复用的测试用代码
//! * 🎯提供自动化测试工具
//! * 🎯提供一种通用的`.nal`测试方法
//!   * ✅存量支持：兼容大部分OpenNARS、ONA的`.nal`文件
//!   * ✨增量特性：基于NAVM提供新的测试语法

util::mods! {
    // 结构定义
    pub pub structs;
    // NAL格式支持
    pub nal_format;
    // NAVM交互
    pub pub vm_interact;
}
