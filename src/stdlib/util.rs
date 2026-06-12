//! KLC 实用工具函数库 — 断言、环境变量、调试增强
//!
//! v1.4.3 新增: 提供 assert 断言、env_get 环境变量读取等开发辅助函数。
//!
//! ## 功能清单
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `assert(condition, msg?)` | 断言校验，失败报错终止 | `assert(x > 0, "x must > 0")` |
//! | `env_get(key)` | 读取系统环境变量 | `env_get("PATH")` → 路径字符串 |
//!
//! ## 兼容性
//! - `sleep(ms)` / `type_of(val)` 等已存在于 VM 全局函数，此处不重复注册

use crate::bytecode::Value as BValue;

// ============================================================================
// 函数名常量
// ============================================================================

pub mod func_names {
    pub const ASSERT: &str = "assert";
    pub const ENV_GET: &str = "env_get";
}

// ============================================================================
// 核心实现
// ============================================================================

/// assert(condition, msg?) — 断言校验
///
/// 当 condition 为 false 时返回错误信息，msg 为可选的自定义错误消息。
/// 在 VM 中，返回的 Err 会终止脚本执行。
///
/// ## KLC 用法
/// ```klc
/// assert(x > 0);              // 不带消息
/// assert(x > 0, "x 必须大于0"); // 带自定义消息
/// ```
pub fn assert_check(cond: &BValue, msg: Option<&BValue>) -> Result<(), String> {
    let is_true = match cond {
        BValue::Bool(b) => *b,
        BValue::Integer(n) => *n != 0,
        BValue::Float(f) => *f != 0.0,
        BValue::Null => false,
        _ => true, // 非布尔/数字的非Null值视为真
    };

    if !is_true {
        let err_msg = match msg {
            Some(BValue::String(s)) if !s.is_empty() => s.clone(),
            _ => "断言失败 (assert failed)".to_string(),
        };
        return Err(format!("[断言失败] {}", err_msg));
    }
    Ok(())
}

/// env_get(key) — 读取系统环境变量
///
/// 环境变量不存在时返回 Null；key 为非字符串时也返回 Null。
///
/// ## KLC 用法
/// ```klc
/// let home = env_get("HOME");  // Unix
/// let path = env_get("PATH");
/// let username = env_get("USERNAME"); // Windows
/// ```
pub fn env_get(key: &BValue) -> BValue {
    match key {
        BValue::String(k) => {
            match std::env::var(k) {
                Ok(val) => BValue::String(val),
                Err(_) => BValue::Null,
            }
        }
        _ => BValue::Null,
    }
}

// ============================================================================
// VM 分发函数
// ============================================================================

/// VM 分发: 处理工具函数调用
///
/// assert 比较特殊: 错误时需要返回 Err 而非 Null，这里仅做检查返回结果。
/// VM 层根据返回的 Result 决定是否终止执行。
pub fn dispatch_util_func(name: &str, args: &[BValue]) -> Option<BValue> {
    match name {
        func_names::ASSERT => {
            let cond = args.first().unwrap_or(&BValue::Bool(false));
            let msg = args.get(1);
            match assert_check(cond, msg) {
                Ok(()) => Some(BValue::Null),
                Err(e) => {
                    // assert 失败时打印错误到 stderr
                    eprintln!("{}", e);
                    // 返回 Null 让 VM 层决定是否终止
                    // 注意: VM 层需要额外处理 assert 错误
                    Some(BValue::Null)
                }
            }
        }
        func_names::ENV_GET => {
            let key = args.first().unwrap_or(&BValue::Null);
            Some(env_get(key))
        }
        _ => None,
    }
}

/// assert 专用的带错误检查函数 — VM 层调用此函数获取 Result
/// 返回 Ok(()) 或 Err(msg)，让 VM 可以直接 propagate 错误
pub fn dispatch_assert(args: &[BValue]) -> Result<(), String> {
    let cond = args.first().unwrap_or(&BValue::Bool(false));
    let msg = args.get(1);
    assert_check(cond, msg)
}

/// 返回所有已注册工具函数名列表
pub fn list_util_functions() -> Vec<&'static str> {
    vec![
        func_names::ASSERT,
        func_names::ENV_GET,
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── assert 测试 ───

    #[test]
    fn test_assert_true() {
        let result = assert_check(&BValue::Bool(true), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_false_without_msg() {
        let result = assert_check(&BValue::Bool(false), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("断言失败"));
    }

    #[test]
    fn test_assert_false_with_msg() {
        let result = assert_check(
            &BValue::Bool(false),
            Some(&BValue::String("x 必须为正数".into())),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("x 必须为正数"));
    }

    #[test]
    fn test_assert_integer_nonzero_is_true() {
        let result = assert_check(&BValue::Integer(1), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_integer_zero_is_false() {
        let result = assert_check(&BValue::Integer(0), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_float_nonzero_is_true() {
        let result = assert_check(&BValue::Float(3.14), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_float_zero_is_false() {
        let result = assert_check(&BValue::Float(0.0), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_null_is_false() {
        let result = assert_check(&BValue::Null, None);
        assert!(result.is_err());
    }

    // ─── env_get 测试 ───

    #[test]
    fn test_env_get_path_exists() {
        // PATH 在所有主流 OS 上都存在
        let result = env_get(&BValue::String("PATH".into()));
        assert!(matches!(result, BValue::String(_)));
    }

    #[test]
    fn test_env_get_nonexistent() {
        let result = env_get(&BValue::String("__KLC_NONEXISTENT_VAR_99999__".into()));
        assert_eq!(result, BValue::Null);
    }

    #[test]
    fn test_env_get_non_string_key() {
        let result = env_get(&BValue::Integer(42));
        assert_eq!(result, BValue::Null);
    }

    // ─── dispatch 集成测试 ───

    #[test]
    fn test_dispatch_assert_true() {
        let result = dispatch_util_func("assert", &[BValue::Bool(true)]);
        assert_eq!(result, Some(BValue::Null));
    }

    #[test]
    fn test_dispatch_env_get() {
        let result = dispatch_util_func("env_get", &[BValue::String("PATH".into())]);
        assert!(matches!(result, Some(BValue::String(_))));
    }

    #[test]
    fn test_dispatch_unknown_func() {
        let result = dispatch_util_func("unknown_util_func", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_list_util_functions() {
        let funcs = list_util_functions();
        assert!(funcs.contains(&"assert"));
        assert!(funcs.contains(&"env_get"));
    }
}
