//! 用于OpenNARS的路径构建器

use crate::{
    name_match::{is_name_match, name_match},
    path_builder::CinPathBuilder,
};
use babel_nar::{opennars::OpenNARS, runtime::CommandVmRuntime};
use nar_dev_utils::{if_return, list, OptionBoost};
use navm::vm::{VmLauncher, VmRuntime};
use std::path::{Path, PathBuf};

/// OpenNARS路径构建器
/// * 🎯判别路径并构建OpenNARS启动器
pub struct PathBuilderOpenNARS;

impl PathBuilderOpenNARS {
    // 匹配文件名
    #[inline(always)]
    fn match_name(name: &str) -> usize {
        // 二者综合，只需「二者均不满足⇒0」即可
        name_match("opennars", name) + name_match("open_nars", name)
    }

    /// 检查文件匹配度
    fn valid_jar(path: &Path) -> usize {
        // ! 不一定是本地存在的文件
        if_return! { !path.extension().is_some_and(|ex| ex == "jar") => 0}
        // 名称匹配`opennars`
        path.file_name().map_unwrap_or(
            |name_os| name_os.to_str().map_unwrap_or(Self::match_name, 0),
            0,
        )
    }
}

impl CinPathBuilder for PathBuilderOpenNARS {
    type Runtime = CommandVmRuntime;
    type Launcher = OpenNARS;

    fn match_path(&self, path: &Path) -> usize {
        // ! 与本地文件系统有关
        // 不是本地的文件⇒0
        if_return! { !path.is_file() => 0 }
        // 否则⇒查看jar匹配度
        Self::valid_jar(path)
    }

    fn construct_from_path(&self, path: &Path) -> Self::Launcher {
        OpenNARS::new(path)
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
        assert!(dbg!(PathBuilderOpenNARS::valid_jar(path)) > 0);
    }

    /// 测试/名称匹配
    #[test]
    fn test_match() {
        f_parallel![
            test_matched;
            "../opennars-304-T-modified.jar";
            "../OpenNARS-3.0.4-Snapshot.jar";
            "../opennars.jar";
            "open_nars.jar";
            "opennars-3.0.4-SNAPSHOT.jar";
        ];
    }

    fail_tests! {
        无效扩展名 test_matched("../opennars-304-T-modified.jar.exe");
        无效名称 test_matched("../ona-T-modified.jar");
    }
}
