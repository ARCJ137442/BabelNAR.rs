//! 统一的「路径构建器」逻辑

use navm::vm::{VmLauncher, VmRuntime};
use std::path::Path;

/// CIN路径构建器
/// * 🚩本身不承担「遍历路径」的任务，只负责
///   * 📌判断是否「可以用于构建NAVM运行时」
///   * 📌从某路径构建「NAVM启动器」
/// * ❌【2024-03-30 19:05:48】放弃「通用启动器/通用运行时」的适配尝试
///   * 📝目前[`CinSearch::Launcher`]总是要带上[`CinSearch::Runtime`]作类型参数
///   * 📌堵点：难以定义一个使用`Box<dyn VmLauncher<?>>`封装的`AnyhowVmLauncher`类型
///     * 📍问题领域：特征对象及其转换
///     * ❓一个可能的参考：[`anyhow`]对「错误类型」的统一
///     * ❌【2024-03-30 21:24:10】尝试仍然失败：有关`Box<Self>`的所有权转换问题
///     * 🔗技术参考1：<https://stackoverflow.com/questions/46620790/how-to-call-a-method-that-consumes-self-on-a-boxed-trait-object>
pub trait CinPathBuilder {
    /// 搜索结果的启动器类型
    /// * 📌启动后变为[`CinSearch::Runtime`]运行时类型
    type Launcher: VmLauncher<Self::Runtime>;

    /// 搜索结果的运行时类型
    type Runtime: VmRuntime;

    /// 路径匹配
    /// * 🎯匹配某路径（可能是文件夹，也可能是文件）是否可用于「构建NAVM启动器」
    /// * ⚠️与**该路径是否存在**有关
    ///   * 📌需要访问本地文件系统
    ///   * 📄一些CIN可能要求判断其子目录的文件（附属文件）
    /// * ⚙️返回「匹配度」
    ///   * 📌`0`⇒不匹配，其它⇒不同程度的匹配
    ///   * 🎯对接「名称匹配」中的「匹配度」
    ///   * ✨可用于后续排序
    fn match_path(&self, path: &Path) -> usize;

    /// 用于检查路径是否匹配
    /// * 🔗参见[`match_path`]
    fn is_path_matched(&self, path: &Path) -> bool {
        self.match_path(path) > 0
    }

    /// 路径构建
    /// * 🎯从某个路径构建出一个NAVM启动器
    ///   * ✅除路径以外，其它参数可作默认
    ///     * 📄OpenNARS的「Java最大堆大小」
    ///
    /// # Panics
    ///
    /// ⚠️需要保证[`is_path_matched`]为真
    /// * 为假时可能`panic`
    fn construct_from_path(&self, path: &Path) -> Self::Launcher;

    /// 尝试路径构建
    /// * 🚩返回一个[`Option`]
    ///   * 能构建⇒返回构建后的结果 `Some((启动器, 匹配度))`
    ///   * 无法构建⇒返回[`None`]
    #[inline]
    fn try_construct_from_path(&self, path: &Path) -> Option<(Self::Launcher, usize)> {
        match self.match_path(path) {
            // 不匹配⇒无
            0 => None,
            // 匹配⇒元组
            n => Some((self.construct_from_path(path), n)),
        }
    }
}
