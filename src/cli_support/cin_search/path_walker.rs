//! 路径遍历器
//! * 🎯用于分离「路径查找」与「CIN识别」两功能
//!   * 📌「路径遍历器」负责「提供路径，并有选择地 深入/跳出 路径」

use anyhow::{Error, Result};
use std::path::{Path, PathBuf};

/// 抽象的「路径遍历」特征
/// * ✨允许「迭代出下一个路径」
/// * 🏗️后续可能会添加更多特性，如「根据结果调整遍历策略」等
pub trait PathWalker {
    /// ✨返回「下一个路径」
    /// * 可能为空，也可能返回错误
    fn next_path(&mut self) -> Result<Option<PathBuf>>;

    /// 类似迭代器的`next`方法
    /// * 🎯对标`Iterator<Item = Result<PathBuf>>`
    /// * 🚩【2024-03-31 01:03:04】是「没法为`impl PathWalker`自动实现`Iterator`」的补偿
    fn iter_next_path(&mut self) -> Option<Result<PathBuf>> {
        match self.next_path() {
            // 正常情况
            Ok(Some(path)) => Some(Ok(path)),
            // 中途报错⇒返回错误
            Err(e) => Some(Err(e)),
            // 终止⇒真正终止
            Ok(None) => None,
        }
    }

    /// 利用[`std::iter::from_fn`]将自身转换为迭代器，而无需实现[`Iterator`]特征
    /// * 🎯便于在`impl PathWalker`中使用
    #[inline]
    fn to_iter_fn<'a>(mut self) -> impl Iterator<Item = Result<PathBuf>> + 'a
    where
        Self: Sized + 'a,
    {
        std::iter::from_fn(move || self.iter_next_path())
    }
}

/// 初代路径遍历器
/// * ✨使用「渐近回退性扫描」机制，总体为「深度优先」
///   * 📌「起始目录」一般为exe所在目录
///   * 🚩从「起始目录」开始，扫描其下子目录
///     * 递归深入、迭代出文件夹与文件
///   * 🚩若「起始目录」已扫描完毕，向上「条件扫描」父目录
///     * 遍历其【直接包含】的文件/文件夹
///     * 若有满足特定「可深入条件」的文件夹，则深入扫描该文件夹（仍然是「条件扫描」）
///   * 🚩父目录扫描完毕后，继续扫描父目录
pub struct PathWalkerV1<'a> {
    // 父目录堆栈
    ancestors_stack: Vec<PathBuf>,

    /// 待遍历目录的堆栈
    to_visit_stack: Vec<PathBuf>,

    /// 可深入条件
    deep_criterion: Box<dyn Fn(&Path) -> bool + Send + Sync + 'a>,

    /// 当前在遍历目录的迭代器
    current_dir_iter: Box<dyn Iterator<Item = Result<PathBuf>>>,
}

impl<'a> PathWalkerV1<'a> {
    pub fn new(
        start: &Path,
        deep_criterion: impl Fn(&Path) -> bool + Send + Sync + 'a,
    ) -> Result<Self> {
        // 计算根目录
        // * 🚩不是文件夹⇒向上寻找根目录
        let mut root = start;
        while !root.is_dir() {
            root = root.parent().unwrap();
        }
        // 构造路径堆栈
        let mut ancestors_stack = root.ancestors().map(Path::to_owned).collect::<Vec<_>>();
        ancestors_stack.reverse(); // 从「当前→根」转为「根→当前」，先遍历当前，再遍历根
                                   // 拿出目录
        let root = match ancestors_stack.pop() {
            Some(path) => path,
            None => return Err(Error::msg("起始目录无效")),
        };
        let deep_criterion = Box::new(deep_criterion);
        let current_dir_iter = Box::new(Self::new_path_iter(&root)?);
        Ok(Self {
            ancestors_stack,
            to_visit_stack: vec![], // 空栈初始化
            deep_criterion,
            current_dir_iter,
        })
    }

    /// ✨构造路径迭代器
    /// * 🎯尽可能让异常变得可处理：避免`unwrap`
    fn new_path_iter(path: &Path) -> Result<impl Iterator<Item = Result<PathBuf>>> {
        Ok(std::fs::read_dir(path)?.map(|e| match e {
            Ok(entry) => Ok(entry.path()),
            Err(e) => Err(e.into()),
        }))
    }

    /// 可能返回[`None`]的[`Self::next`]
    /// * 🎯应对「切换到父目录的迭代器后，首个迭代结果还是[`None`]」的情况
    ///   * 🚩解决方案：再次[`Self::poll_path`]
    fn poll_path(&mut self) -> PathPollResult {
        // ! ❌【2024-03-30 22:34:04】目前没法稳定地使用`?`
        match self.current_dir_iter.next() {
            // 正常情况
            Some(Ok(path)) => {
                // 如果「值得深入」⇒预备在后续深入
                if path.is_dir() && (self.deep_criterion)(&path) {
                    self.to_visit_stack.push(path.clone())
                }
                // 返回
                PathPollResult::Some(path)
            }
            // 中途报错情况
            Some(Err(e)) => PathPollResult::Err(e),
            // 没有⇒尝试切换路径
            None => self.try_switch_current_path(),
        }
    }

    /// 尝试切换路径
    /// * 切换到一个新的路径
    fn try_switch_current_path(&mut self) -> PathPollResult {
        match self.to_visit_stack.pop() {
            // 「待检查路径」有⇒尝试pop一个，构造并切换到新的迭代器
            Some(path) => match self.change_current_path(&path) {
                Ok(()) => PathPollResult::None, // 构造了就收手，无需立马查看里边有无路径
                Err(e) => PathPollResult::Err(e),
            },
            // 「待检查路径」没有⇒尝试从「祖先路径」中尝试pop一个
            None => match self.ancestors_stack.pop() {
                // 「祖先路径」有⇒尝试pop一个，构造并切换到新的迭代器
                Some(path) => match self.change_current_path(&path) {
                    Ok(()) => PathPollResult::None, // 构造了就收手，无需立马查看里边有无路径
                    Err(e) => PathPollResult::Err(e),
                }, // 「祖先路径」没有⇒终止
                None => PathPollResult::Ended,
            },
        }
    }

    /// 尝试更改到某个目录（的迭代器）
    fn change_current_path(&mut self, path: &Path) -> Result<()> {
        let iter = Self::new_path_iter(path)?;
        self.current_dir_iter = Box::new(iter);
        Ok(())
    }
}

/// 枚举「路径遍历」结果
/// * 🎯用于「路径遍历器」的返回值
pub enum PathPollResult {
    /// 拿到了一个路径
    Some(PathBuf),
    /// 尝试拿，但没拿到路径
    None,
    /// 尝试拿，但发生错误
    Err(Error),
    /// 结束了
    Ended,
}

impl From<Option<PathBuf>> for PathPollResult {
    fn from(value: Option<PathBuf>) -> Self {
        match value {
            Some(path) => Self::Some(path),
            None => Self::None,
        }
    }
}

impl From<Result<PathBuf>> for PathPollResult {
    fn from(value: Result<PathBuf>) -> Self {
        match value {
            Ok(path) => Self::Some(path),
            Err(e) => Self::Err(e),
        }
    }
}

impl PathWalker for PathWalkerV1<'_> {
    fn next_path(&mut self) -> Result<Option<PathBuf>> {
        // 持续不断poll自身，压缩掉其中的`None`项
        loop {
            match self.poll_path() {
                // 正常返回路径
                PathPollResult::Some(path) => break Ok(Some(path)),
                // 没有⇒继续循环（压缩掉）
                PathPollResult::None => continue,
                // 报错⇒返回错误
                PathPollResult::Err(e) => break Err(e),
                // 终止⇒返回终止信号
                PathPollResult::Ended => break Ok(None),
            }
        }
    }
}

/// 实现迭代器，返回所有「搜索结果」
impl Iterator for PathWalkerV1<'_> {
    type Item = Result<PathBuf>;
    fn next(&mut self) -> Option<Result<PathBuf>> {
        self.iter_next_path()
    }
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_support::cin_search::name_match::is_name_match;
    use std::env::current_dir;

    fn _test_path_walker_v1(start: impl Into<PathBuf>) {
        // 起始目录
        let start = &start.into();
        // 深入条件
        fn deep_criterion(path: &Path) -> bool {
            path.file_name()
                .is_some_and(|name| name.to_str().is_some_and(|s| is_name_match("nars", s)))
        }
        // 构建遍历者，加上条件
        let walker = PathWalkerV1::new(start, deep_criterion).unwrap();
        // 打印遍历者的「祖先列表」
        println!("{:?}", walker.ancestors_stack);
        // 遍历
        for path in walker {
            println!("{path:?}");
        }
    }

    #[test]
    fn test_path_walker_v1() {
        // 测试当前路径
        _test_path_walker_v1(current_dir().unwrap());
    }
}
