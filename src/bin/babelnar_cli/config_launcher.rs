//! 用于从「启动参数」启动NAVM运行时

use crate::{
    read_config_extern, LaunchConfig, LaunchConfigCommand, LaunchConfigTranslators, RuntimeConfig,
};
use anyhow::{anyhow, Result};
use babel_nar::{
    cin_implements::{
        common::generate_command, cxin_js, nars_python, ona, openjunars, opennars, pynars,
    },
    cli_support::{cin_search::name_match::name_match, io::readline_iter::ReadlineIter},
    eprintln_cli,
    runtimes::{
        api::{InputTranslator, IoTranslators},
        CommandVm, OutputTranslator,
    },
};
use nar_dev_utils::pipe;
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmLauncher, VmRuntime},
};
use std::path::PathBuf;

/// （若缺省）要求用户手动填充配置项
pub fn polyfill_config_from_user(config: &mut LaunchConfig) {
    if config.need_polyfill() {
        // * 🚩【2024-04-03 19:33:20】目前是要求输入配置文件路径
        for line in ReadlineIter::new("请输入配置文件路径（如`BabelNAR.launch.json`）: ")
        {
            // 检验输入
            if let Err(e) = line {
                eprintln_cli!([Error] "输入无效：{e}");
                continue;
            }
            // 检验路径
            let path = PathBuf::from(line.unwrap().trim());
            if !path.is_file() {
                eprintln_cli!([Error] "文件「{path:?}」不存在");
                continue;
            }
            // 读取配置文件
            let content = match read_config_extern(&path) {
                Ok(config) => config,
                Err(e) => {
                    eprintln_cli!([Error] "配置文件「{path:?}」读取失败：{e}");
                    continue;
                }
            };
            // 读取成功⇒覆盖，返回
            *config = content;
            break;
        }
    }
}

/// 从「启动参数」中启动
/// * 🚩在转换中确认参数
/// * ⚙️返回(启动后的运行时, 转换后的『运行时配置』)
/// * ❌无法使用`impl TryInto<RuntimeConfig>`统一「启动参数」与「运行参数」
///   * 📌即便：对于「运行时参数」，[`TryInto::try_into`]始终返回自身
///   * 📝然而：对自身的[`TryInto`]错误类型总是[`std::convert::Infallible`]
///   * ❗错误类型不一致，无法统一返回
pub fn launch_by_config(
    config: impl TryInto<RuntimeConfig, Error = anyhow::Error>,
) -> Result<(impl VmRuntime, RuntimeConfig)> {
    // 转换启动配置
    let config: RuntimeConfig = config.try_into()?;

    // 生成虚拟机
    let runtime = launch_by_runtime_config(&config)?;

    // 返回
    Ok((runtime, config))
}

pub fn launch_by_runtime_config(config: &RuntimeConfig) -> Result<impl VmRuntime> {
    // 生成虚拟机
    let config_command = &config.command;
    let mut vm = load_command_vm(config_command)?;

    // 配置虚拟机
    // * 🚩【2024-04-04 03:17:43】现在「转译器」成了必选项，所以必定会有配置
    config_launcher_translators(&mut vm, &config.translators)?;

    // 启动虚拟机
    let runtime = vm.launch()?;
    Ok(runtime)
}

/// 从「启动参数/启动命令」启动「命令行虚拟机」
/// * ❓需要用到「具体启动器实现」吗
pub fn load_command_vm(config: &LaunchConfigCommand) -> Result<CommandVm> {
    // 构造指令
    let command = generate_command(
        &config.cmd,
        config.current_dir.as_ref(),
        // 🚩获取其内部数组的引用，或使用一个空数组作迭代器（无法简化成[`unwrap_or`]）
        match &config.cmd_args {
            Some(v) => v.iter(),
            // ↓此处`unwrap_or_default`默认使用一个空数组作为迭代器
            None => [].iter(),
        },
    );
    // 构造虚拟机
    let vm = command.into();
    // 返回
    Ok(vm)
}

/// 从「启动参数/输入输出转译器」配置「命令行虚拟机」
/// * 🚩【2024-04-02 01:03:54】此处暂时需要**硬编码**现有的CIN实现
/// * 🏗️后续可能支持定义自定义转译器（long-term）
/// * ⚠️可能会有「转译器没找到/转译器加载失败」等
/// * 📌【2024-04-02 01:49:46】此处需要暂时借用所有权
pub fn config_launcher_translators(
    vm: &mut CommandVm,
    config: &LaunchConfigTranslators,
) -> Result<()> {
    Ok(pipe! {
        // 获取转译器
        get_translator_by_name(config) => {?}#
        // 设置转译器
        => [vm.translators](_)
        // 返回成功
    })
}

/// 从「转译器名」检索「输入输出转译器」
/// * 🚩继续分派到「输入转译器检索」与「输出转译器检索」
pub fn get_translator_by_name(config: &LaunchConfigTranslators) -> Result<IoTranslators> {
    let name_i = match config {
        LaunchConfigTranslators::Same(input) | LaunchConfigTranslators::Separated { input, .. } => {
            input
        }
    };
    let name_o = match config {
        LaunchConfigTranslators::Same(output)
        | LaunchConfigTranslators::Separated { output, .. } => output,
    };
    Ok(IoTranslators {
        input_translator: get_input_translator_by_name(name_i.as_str())?,
        output_translator: get_output_translator_by_name(name_o.as_str())?,
    })
}

/// 输入转译器的索引字典类型
/// * 📌结构：`[(转译器名, 输入转译器, 输出转译器)]`
pub type TranslatorDict<'a> = &'a [(
    &'a str,
    fn(Cmd) -> Result<String>,
    fn(String) -> Result<Output>,
)];
/// 输入转译器的索引字典
/// * 🚩静态存储映射，后续遍历可有序可无序
pub const TRANSLATOR_DICT: TranslatorDict = &[
    (
        "OpenNARS",
        opennars::input_translate,
        opennars::output_translate,
    ),
    ("ONA", ona::input_translate, ona::output_translate),
    (
        "NARS-Python",
        nars_python::input_translate,
        nars_python::output_translate,
    ),
    (
        "NARSPython",
        nars_python::input_translate,
        nars_python::output_translate,
    ),
    ("PyNARS", pynars::input_translate, pynars::output_translate),
    (
        "OpenJunars",
        openjunars::input_translate,
        openjunars::output_translate,
    ),
    (
        "CXinJS",
        cxin_js::input_translate,
        cxin_js::output_translate,
    ),
];

pub fn get_input_translator_by_name(cin_name: &str) -> Result<Box<InputTranslator>> {
    // 根据「匹配度」的最大值选取
    let translator = TRANSLATOR_DICT
        .iter()
        .max_by_key(|(name, _, _)| name_match(name, cin_name))
        .ok_or_else(|| anyhow!("未找到输入转译器"))?
        .1; // 输入转译器
    Ok(Box::new(translator))
}

pub fn get_output_translator_by_name(cin_name: &str) -> Result<Box<OutputTranslator>> {
    // 根据「匹配度」的最大值选取
    let translator = TRANSLATOR_DICT
        .iter()
        .max_by_key(|(name, _, _)| name_match(name, cin_name))
        .ok_or_else(|| anyhow!("未找到输出转译器"))?
        .2; // 输出转译器
    Ok(Box::new(translator))
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use nar_dev_utils::{asserts, f_parallel};

    #[test]
    fn t() {
        dbg!(format!("{:p}", opennars::input_translate as fn(_) -> _));
    }

    /// 测试
    /// * 🚩仅能测试「是否查找成功」，无法具体地比较函数是否相同
    ///   * 📝函数在被装进[`Box`]后，对原先结构的完整引用就丧失了
    #[test]
    fn test() {
        fn t(name: &str) {
            asserts! {
                get_input_translator_by_name(name).is_ok()
                get_output_translator_by_name(name).is_ok()
            }
        }
        f_parallel![
            t;
            "opennars"; "ona"; "nars-python"; "narsPython"; "pynars"; "openjunars"; "cxinJS"
        ];
    }
}
