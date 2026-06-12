#![allow(dead_code)]

//! KLC 标准库 (v1.3.6) — 独立模块化标准库入口
//!
//! KLC 标准库提供开箱即用的 IO、数学、字符串、数组、工具函数等基础能力。
//! 所有功能均通过 VM 内置函数注册，KLC 脚本无需 import 即可直接调用。
//!
//! ## 模块结构
//!
//! - `io.rs`     : 控制台 IO、文件 IO、格式化 IO、工具函数
//! - `math.rs`   : 全局数学函数 (abs, sqrt, pow, round, floor, ceil, max, min)
//! - `string.rs` : 全局字符串函数 (str_upper, str_lower, str_trim, str_contains, str_replace)
//! - `array.rs`  : 全局数组函数 (arr_len, arr_push, arr_pop, arr_slice)
//! - `util.rs`   : 实用工具函数 (assert, env_get)
//!
//! ## 使用方式
//!
//! ```klc
//! // KLC 脚本中直接调用，无需任何 import
//! println("Hello, KLC!");
//! let name = input("Enter your name: ");
//! file_write("test.txt", "Hello world");
//! let result = abs(-42);           // 绝对值
//! let upper = str_upper("hello");  // 大写
//! assert(x > 0, "x 必须为正数");    // 断言
//! ```
//!
//! ## VM 注册
//!
//! 标准库函数在 VM 的 `handle_call` 中通过统一分发入口 `dispatch_new_stdlib()` 自动注册。
//! 所有调用最终进入 VM 的 `handle_call` → 字符串匹配 → 对应内置函数。

pub mod io;
pub mod math;
pub mod string;
pub mod array;
pub mod util;

// ============================================================================
// 统一分发入口 — v1.3.6 新增
// ============================================================================

use crate::bytecode::Value;

/// 统一分发所有 v1.3.6 新增 stdlib 模块 (math/string/array/util)
///
/// 在 VM 的 handle_call 中按顺序尝试各模块分发器。
/// 返回 `Some(value)` 表示已处理，返回 `None` 表示不匹配（VM 继续查找）。
pub fn dispatch_new_stdlib(name: &str, args: &[Value]) -> Option<Value> {
    math::dispatch_math_func(name, args)
        .or_else(|| string::dispatch_string_func(name, args))
        .or_else(|| array::dispatch_array_func(name, args))
        .or_else(|| util::dispatch_util_func(name, args))
}

/// 标准库统计信息
#[derive(Debug, Clone)]
pub struct StdStats {
    /// 注册的 IO 函数总数
    pub io_funcs: usize,
    /// 注册的数学函数总数
    pub math_funcs: usize,
    /// 注册的字符串函数总数
    pub string_funcs: usize,
    /// 注册的数组函数总数
    pub array_funcs: usize,
    /// 注册的工具函数总数
    pub util_funcs: usize,
}

impl Default for StdStats {
    fn default() -> Self {
        Self {
            io_funcs: 0,
            math_funcs: 0,
            string_funcs: 0,
            array_funcs: 0,
            util_funcs: 0,
        }
    }
}

impl std::fmt::Display for StdStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[KLC Std] IO:{} Math:{} String:{} Array:{} Util:{}",
            self.io_funcs, self.math_funcs, self.string_funcs, self.array_funcs, self.util_funcs
        )
    }
}