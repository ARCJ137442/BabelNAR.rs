//! 用于封装抽象「进程通信」逻辑
//! 示例代码来源：https://www.nikbrendler.com/rust-process-communication/
//! * 📌基于「通道」的「子进程+专职读写的子线程」通信逻辑
//!

nar_dev_utils::pub_mod_and_pub_use! {
    // 输入输出进程
    io_process
}
