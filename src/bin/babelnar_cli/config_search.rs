//! CIN自动搜索

use crate::{read_config_extern, LaunchConfig};
use anyhow::Result;
use babel_nar::{
    cli_support::cin_search::{name_match::is_name_match, path_walker::PathWalkerV1},
    println_cli,
};
use std::path::{Path, PathBuf};

pub fn search_configs<S: AsRef<str>>(
    start: &Path,
    allowed_extension_names: impl IntoIterator<Item = S>,
    verbose: bool,
) -> Result<impl IntoIterator<Item = LaunchConfig>> {
    // 允许的扩展名
    let extension_names = allowed_extension_names.into_iter().collect::<Vec<_>>();
    // 深入条件
    fn deep_criterion(path: &Path) -> bool {
        path.file_name()
            .is_some_and(|name| name.to_str().is_some_and(|s| is_name_match("nars", s)))
    }

    // 构建遍历者，加上条件
    let walker = PathWalkerV1::new(start, deep_criterion).unwrap();

    let is_extension_match = |path: &PathBuf| {
        path.extension().is_some_and(|ext| {
            ext.to_str().is_some_and(|ext_str| {
                extension_names
                    .iter()
                    .any(|name| is_name_match(name.as_ref(), ext_str))
            })
        })
    };

    // 遍历（成功的）
    let mut c = 0;
    let mut c_valid = 0;
    let mut valid_non_empty_configs = vec![];
    for path in walker.flatten().filter(is_extension_match) {
        if verbose {
            println_cli!([Log] "正在搜索 {path:?}");
        }
        if let Ok(config) = read_config_extern(&path) {
            c_valid += 1;
            if !config.is_empty() {
                if verbose {
                    println_cli!([Info] "搜索到配置文件：{config:?}");
                }
                valid_non_empty_configs.push(config);
            }
        }
        c += 1;
    }

    // 输出搜索结果
    println_cli!(
        [Info]
        "一共搜索了{c}个文件，其中 {c_valid} 个文件符合条件，{} 个非空",
        &valid_non_empty_configs.len()
    );
    match valid_non_empty_configs.is_empty() {
        true => println_cli!([Info] "未搜索到任何有效配置。"),
        false => {
            println_cli!([Info] "已搜索到以下有效配置：");
            for (i, config) in valid_non_empty_configs.iter().enumerate() {
                // TODO: 后续或许在其中添加描述信息？
                println_cli!([Info] "【{i}】 {config:?}");
            }
        }
    }

    // 返回
    Ok(valid_non_empty_configs)
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use babel_nar::tests::config_paths::ARG_PARSE_TEST;
    // use std::env::current_dir;

    #[test]
    fn test_path_walker_v1() {
        // 测试`config`目录下的文件
        let start = ARG_PARSE_TEST;
        // * 📌起始目录即项目根目录
        search_configs(&PathBuf::from(start), ["json", "hjson"], true).expect("搜索出错");
    }
}
