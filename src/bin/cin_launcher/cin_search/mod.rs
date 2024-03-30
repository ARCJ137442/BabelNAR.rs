//! CIN启动器中有关「CIN路径构建（搜索）」的逻辑
//! * ✨根据「CIN路径构建器」搜索（判别）系统中已存在的CIN实现（并自动构建）
//!  * 🚩输入：搜索起点（一般是编译后exe所在文件夹）
//!  * 🚩输出：NAVM启动器列表
//! * ❓【2024-03-30 19:12:29】是否要考虑返回更细化的「CIN实例位置」而非「CIN启动器」，以避免额外的性能开销？

use nar_dev_utils::mods;

/// 导出模块
mods! {
    // 路径遍历器
    use pub path_walker;
    // 路径构造器
    use pub path_builder;
    // anyhow | 弃用
    // anyhow_vm;
    // 名称匹配
    use pub name_match;
}
