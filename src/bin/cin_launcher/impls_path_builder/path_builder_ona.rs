//! 用于ONA的路径构建器

use crate::{
    name_match::{is_name_match, name_match, name_match_only_contains},
    path_builder::CinPathBuilder,
};
use babel_nar::{ona::ONA, runtime::CommandVmRuntime};
use nar_dev_utils::{if_return, list, OptionBoost};
use navm::vm::{VmLauncher, VmRuntime};
use std::path::{Path, PathBuf};

/// ONA路径构建器
/// * 🎯判别路径并构建ONA启动器
pub struct PathBuilderONA;

impl PathBuilderONA {
    // 匹配文件名
    #[inline(always)]
    fn match_name(name: &str) -> usize {
        // 常用的`NAR.exe`
        (if name == "NAR.exe" { 10 } else { 0 })
        // 综合，只需「均不满足⇒0」即可
            + name_match("ona", name)
            + name_match_only_contains("opennars-for-application", name)
            + name_match_only_contains("opennars_for_application", name)
    }

    /// 检查文件匹配度
    fn valid_exe(path: &Path) -> usize {
        // ! 不一定是本地存在的文件
        if_return! { !path.extension().is_some_and(|ex| ex == "exe") => 0}
        // 名称匹配`ona`
        path.file_name().map_unwrap_or(
            |name_os| name_os.to_str().map_unwrap_or(Self::match_name, 0),
            0,
        )
    }
}

impl CinPathBuilder for PathBuilderONA {
    type Runtime = CommandVmRuntime;
    type Launcher = ONA;

    fn match_path(&self, path: &Path) -> usize {
        // ! 与本地文件系统有关
        // 不是本地的文件⇒0
        if_return! { !path.is_file() => 0 }
        // 否则⇒查看exe匹配度
        Self::valid_exe(path)
    }

    fn construct_from_path(&self, path: &Path) -> Self::Launcher {
        ONA::new(path)
    }
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use nar_dev_utils::{f_parallel, fail_tests};
    use std::path::{self, Path};

    /// 工具/测试单个路径
    fn test_matched(path: &str) {
        let path = Path::new(path);
        assert!(dbg!(PathBuilderONA::valid_exe(path)) > 0);
    }

    /// 测试/名称匹配
    #[test]
    fn test_match() {
        f_parallel![
            test_matched;
            "../NAR.exe";
            "../opennars-for-applications.exe";
            "../ona.exe";
            "ona_old.exe";
        ];
    }

    fail_tests! {
        无效扩展名 test_matched("../opennars.exe");
        无效名称 test_matched("../NARust.exe");
    }
}
