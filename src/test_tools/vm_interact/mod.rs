//! 与NAVM虚拟机的交互逻辑

// 词项判等
mod term_equal;

// 输出预期
mod output_expectation;
pub use output_expectation::*;

// 置入NAL
mod put_nal;
pub use put_nal::*;
