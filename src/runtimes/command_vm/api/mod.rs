//! 定义有关「命令行虚拟机」的抽象API
//! * 【2024-03-26 22:33:19】总体想法：💡一个「转译器集成包」+「命令行参数生成器」⇒统一复用的「IO进程启动器」
//!   * 📌转译器集成包：用于将「输入转译器」与「输出转译器」打包成一个统一类型的值以传入

util::pub_mod_and_pub_use! {
    // 转译器
    translators
    // 命令行参数生成器
    command_generator
}
