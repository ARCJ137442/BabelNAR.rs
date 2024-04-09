//! NAVM原生的虚拟机（转译器）
//! * ✨Cmd输入转译：直接将[`Cmd`]转换为字符串形式
//! * ✨NAVM_JSON输出转译：基于[`serde_json`]直接从JSON字符串读取[`Output`]
//! * 📌没有固定的启动器：仅通过「命令行启动器」即可启动

util::mods! {
    // 输入输出转译
    pub pub translators;
}
