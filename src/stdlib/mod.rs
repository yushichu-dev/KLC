#![allow(dead_code)]

//! KLC 标准库 (v1.3.2) — 独立模块化标准库入口
//!
//! KLC 标准库提供开箱即用的 IO、格式化、工具函数等基础能力。
//! 所有功能均通过 VM 内置函数注册，KLC 脚本无需 import 即可直接调用。
//!
//! ## 模块结构
//!
//! - `io.rs`    : 控制台 IO、文件 IO、格式化 IO、工具函数
//!
//! ## 使用方式
//!
//! ```klc
//! // KLC 脚本中直接调用，无需任何 import
//! println("Hello, KLC!");
//! let name = input("Enter your name: ");
//! file_write("test.txt", "Hello world");
//! ```
//!
//! ## VM 注册
//!
//! 标准库函数在 VM 初始化时通过 `register_io_builtins()` 自动注册。
//! 所有 IO 调用最终进入 VM 的 `handle_call` → 字符串匹配 → 对应内置函数。

pub mod io;

/// 标准库统计信息
#[derive(Debug, Clone)]
pub struct StdStats {
    /// 注册的 IO 函数总数
    pub io_funcs: usize,
}

impl Default for StdStats {
    fn default() -> Self {
        Self { io_funcs: 0 }
    }
}

impl std::fmt::Display for StdStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[KLC Std] IO函数: {}", self.io_funcs)
    }
}
