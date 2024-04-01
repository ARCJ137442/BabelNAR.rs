//! 用于从「启动参数」启动NAVM运行时

use crate::{LaunchConfig, LaunchConfigCommand, LaunchConfigTranslators};
use anyhow::{anyhow, Ok, Result};
use babel_nar::{
    cin_implements::{
        common::generate_command, cxin_js, nars_python, ona, openjunars, opennars, pynars,
    },
    cli_support::cin_search::name_match::name_match,
    runtimes::{
        api::{InputTranslator, IoTranslators},
        CommandVm, OutputTranslator,
    },
};
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmLauncher, VmRuntime},
};

/// （若缺省）要求用户手动填充配置项
pub fn polyfill_config_from_user(config: &mut LaunchConfig) {
    if config.need_polyfill() {
        // TODO: 在有缺省的情况下 手动要求用户输入填补缺省项
    }
}

/// 从「启动参数」中启动
/// * 🚩默认所有参数都经过确认
pub fn launch_by_config(config: LaunchConfig) -> Result<impl VmRuntime> {
    // 生成虚拟机
    let config_command = config.command.ok_or_else(|| anyhow!("缺少启动命令"))?;
    let mut vm = load_command_vm(config_command)?;

    // 配置虚拟机
    if let Some(translators) = config.translators {
        // 因为配置函数的设计，此处要暂时借用所有权
        vm = config_launcher_translators(vm, &translators)?;
    }

    // 启动虚拟机
    let runtime = vm.launch()?;
    Ok(runtime)
}

/// 从「启动参数/启动命令」启动「命令行虚拟机」
/// * ❓需要用到「具体启动器实现」吗
pub fn load_command_vm(config: LaunchConfigCommand) -> Result<CommandVm> {
    let command = generate_command(
        config.cmd,
        config.current_dir,
        // ↓此处`unwrap_or_default`默认使用一个空数组作为迭代器
        config.cmd_args.unwrap_or_default().into_iter().by_ref(),
    );
    let vm = command.into();
    Ok(vm)
}

/// 从「启动参数/输入输出转译器」配置「命令行虚拟机」
/// * 🚩【2024-04-02 01:03:54】此处暂时需要**硬编码**现有的CIN实现
/// * 🏗️后续可能支持定义自定义转译器（long-term）
/// * ⚠️可能会有「转译器没找到/转译器加载失败」等
/// * 📌【2024-04-02 01:49:46】此处需要暂时借用所有权
pub fn config_launcher_translators(
    vm: CommandVm,
    config: &LaunchConfigTranslators,
) -> Result<CommandVm> {
    let translators = get_translator_by_name(config)?;
    Ok(vm.translators(translators))
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
