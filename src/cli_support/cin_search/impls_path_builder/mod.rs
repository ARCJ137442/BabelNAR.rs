//! 存储各CIN的「路径构建器」
//! * ✅OpenNARS
//! * ✅ONA
//! TODO: PyNARS
//! TODO: CXinNARS
//! * 🚩【2024-03-31 01:27:09】其它接口完成度不高的CIN，暂时弃了

use crate::cli_support::cin_search::{
    name_match::is_name_match, path_builder::CinPathBuilder, path_walker::PathWalker,
};
use navm::vm::{VmLauncher, VmRuntime};
use std::path::Path;

util::mods! {
    // OpenNARS
    use pub path_builder_opennars;
    // ONA
    use pub path_builder_ona;
}

// 深入条件
pub fn file_name_matches(path: &Path, name: &str) -> bool {
    path.file_name().is_some_and(|name_os| {
        name_os
            .to_str()
            .is_some_and(|name_str| is_name_match(name, name_str))
    })
}

/// 从遍历者中找到匹配的所有启动器
/// * 🎯仅搜索出「可能有效，故构建好」的启动器
pub fn launchers_from_walker<R: VmRuntime, L: VmLauncher<R>>(
    path_walker: impl PathWalker,
    path_builder: impl CinPathBuilder<Launcher = L, Runtime = R>,
) -> Vec<(L, usize)> {
    path_walker
        .to_iter_fn()
        .filter_map(Result::ok)
        .filter_map(|p| path_builder.try_construct_from_path(&p))
        .collect::<Vec<_>>()
}

/// 类似[`launchers_from_walker`]，但根据返回的「匹配度」从高到底排序
pub fn launchers_from_walker_sorted<R: VmRuntime, L: VmLauncher<R>>(
    path_walker: impl PathWalker,
    path_builder: impl CinPathBuilder<Launcher = L, Runtime = R>,
) -> Vec<L> {
    // 获取 & 排序
    let mut launchers = launchers_from_walker(path_walker, path_builder);
    launchers.sort_by(|(_, a), (_, b)| b.cmp(a)); // ←此处是倒序
                                                  // 提取左侧元素
    launchers.into_iter().map(|(l, _)| l).collect::<Vec<_>>()
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_support::cin_search::path_walker::PathWalkerV1;
    use std::env::current_dir;

    #[test]
    fn test() {
        let path_walker = PathWalkerV1::new(&current_dir().unwrap(), |path| {
            file_name_matches(path, "nars")
        })
        .unwrap();

        dbg!(launchers_from_walker_sorted(
            path_walker,
            PathBuilderOpenNARS
        ));
    }
}
