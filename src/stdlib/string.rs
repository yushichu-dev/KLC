//! KLC 字符串增强工具库 — 独立全局字符串函数
//!
//! v1.4.3 新增: 提供无需方法调用的全局字符串函数。
//! 非字符串输入安全处理，不 panic。
//!
//! ## 功能清单
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `str_upper(s)` | 转大写 | `str_upper("hello")` → "HELLO" |
//! | `str_lower(s)` | 转小写 | `str_lower("HELLO")` → "hello" |
//! | `str_trim(s)` | 去除首尾空白 | `str_trim("  hi  ")` → "hi" |
//! | `str_contains(s, sub)` | 判断包含子串 | `str_contains("hello", "ll")` → true |
//! | `str_replace(s, old, new)` | 字符串替换 | `str_replace("abc", "b", "x")` → "axc" |
//!
//! ## 兼容性
//! - 原有字符串方法 (`s.to_upper()`, `s.trim()` 等) 完全不受影响
//! - 全局 `str_len(s)` 在 VM 中已存在，此处不再重复注册

use crate::bytecode::Value as BValue;

// ============================================================================
// 函数名常量
// ============================================================================

pub mod func_names {
    pub const STR_UPPER: &str = "str_upper";
    pub const STR_LOWER: &str = "str_lower";
    pub const STR_TRIM: &str = "str_trim";
    pub const STR_CONTAINS: &str = "str_contains";
    pub const STR_REPLACE: &str = "str_replace";
}

// ============================================================================
// 核心实现
// ============================================================================

/// 提取字符串引用，非字符串返回 None
fn as_str_ref(val: &BValue) -> Option<&str> {
    match val {
        BValue::String(s) => Some(s.as_str()),
        _ => None,
    }
}

/// str_upper(s) — 转大写。非字符串返回原值（安全处理）
pub fn str_upper(val: &BValue) -> BValue {
    match as_str_ref(val) {
        Some(s) => BValue::String(s.to_uppercase()),
        None => val.clone(),
    }
}

/// str_lower(s) — 转小写。非字符串返回原值
pub fn str_lower(val: &BValue) -> BValue {
    match as_str_ref(val) {
        Some(s) => BValue::String(s.to_lowercase()),
        None => val.clone(),
    }
}

/// str_trim(s) — 去除首尾空白字符。非字符串返回原值
pub fn str_trim(val: &BValue) -> BValue {
    match as_str_ref(val) {
        Some(s) => BValue::String(s.trim().to_string()),
        None => val.clone(),
    }
}

/// str_contains(s, substr) — 判断是否包含子串。非字符串返回 false
pub fn str_contains(val: &BValue, substr: &BValue) -> BValue {
    match (as_str_ref(val), as_str_ref(substr)) {
        (Some(s), Some(sub)) => BValue::Bool(s.contains(sub)),
        _ => BValue::Bool(false),
    }
}

/// str_replace(s, old, new) — 字符串替换。非字符串返回原值
pub fn str_replace(val: &BValue, old: &BValue, new: &BValue) -> BValue {
    match (as_str_ref(val), as_str_ref(old), as_str_ref(new)) {
        (Some(s), Some(o), Some(n)) => BValue::String(s.replace(o, n)),
        _ => val.clone(),
    }
}

// ============================================================================
// VM 分发函数
// ============================================================================

/// VM 分发: 处理全局字符串函数调用
pub fn dispatch_string_func(name: &str, args: &[BValue]) -> Option<BValue> {
    match name {
        func_names::STR_UPPER => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(str_upper(val))
        }
        func_names::STR_LOWER => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(str_lower(val))
        }
        func_names::STR_TRIM => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(str_trim(val))
        }
        func_names::STR_CONTAINS => {
            let val = args.first().unwrap_or(&BValue::Null);
            let sub = args.get(1).unwrap_or(&BValue::Null);
            Some(str_contains(val, sub))
        }
        func_names::STR_REPLACE => {
            let val = args.first().unwrap_or(&BValue::Null);
            let old = args.get(1).unwrap_or(&BValue::Null);
            let new = args.get(2).unwrap_or(&BValue::Null);
            Some(str_replace(val, old, new))
        }
        _ => None,
    }
}

/// 返回所有已注册字符串函数名列表
pub fn list_string_functions() -> Vec<&'static str> {
    vec![
        func_names::STR_UPPER,
        func_names::STR_LOWER,
        func_names::STR_TRIM,
        func_names::STR_CONTAINS,
        func_names::STR_REPLACE,
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_upper_basic() {
        let result = str_upper(&BValue::String("hello".into()));
        assert_eq!(result, BValue::String("HELLO".into()));
    }

    #[test]
    fn test_str_upper_mixed() {
        let result = str_upper(&BValue::String("Hello World".into()));
        assert_eq!(result, BValue::String("HELLO WORLD".into()));
    }

    #[test]
    fn test_str_upper_non_string() {
        let result = str_upper(&BValue::Integer(42));
        assert_eq!(result, BValue::Integer(42));
    }

    #[test]
    fn test_str_lower_basic() {
        let result = str_lower(&BValue::String("HELLO".into()));
        assert_eq!(result, BValue::String("hello".into()));
    }

    #[test]
    fn test_str_trim_basic() {
        let result = str_trim(&BValue::String("  hello  ".into()));
        assert_eq!(result, BValue::String("hello".into()));
    }

    #[test]
    fn test_str_trim_newlines() {
        let result = str_trim(&BValue::String("\n  test \n".into()));
        assert_eq!(result, BValue::String("test".into()));
    }

    #[test]
    fn test_str_trim_no_whitespace() {
        let result = str_trim(&BValue::String("hello".into()));
        assert_eq!(result, BValue::String("hello".into()));
    }

    #[test]
    fn test_str_contains_true() {
        let result = str_contains(
            &BValue::String("hello world".into()),
            &BValue::String("world".into()),
        );
        assert_eq!(result, BValue::Bool(true));
    }

    #[test]
    fn test_str_contains_false() {
        let result = str_contains(
            &BValue::String("hello world".into()),
            &BValue::String("xyz".into()),
        );
        assert_eq!(result, BValue::Bool(false));
    }

    #[test]
    fn test_str_contains_non_string() {
        let result = str_contains(
            &BValue::Integer(42),
            &BValue::String("x".into()),
        );
        assert_eq!(result, BValue::Bool(false));
    }

    #[test]
    fn test_str_replace_basic() {
        let result = str_replace(
            &BValue::String("abcabc".into()),
            &BValue::String("b".into()),
            &BValue::String("x".into()),
        );
        assert_eq!(result, BValue::String("axcaxc".into()));
    }

    #[test]
    fn test_str_replace_non_string() {
        let result = str_replace(
            &BValue::Integer(42),
            &BValue::String("a".into()),
            &BValue::String("b".into()),
        );
        assert_eq!(result, BValue::Integer(42));
    }

    // ─── dispatch 集成测试 ───

    #[test]
    fn test_dispatch_str_upper() {
        let result = dispatch_string_func("str_upper", &[BValue::String("hi".into())]);
        assert_eq!(result, Some(BValue::String("HI".into())));
    }

    #[test]
    fn test_dispatch_str_contains() {
        let result = dispatch_string_func(
            "str_contains",
            &[BValue::String("hello".into()), BValue::String("ll".into())],
        );
        assert_eq!(result, Some(BValue::Bool(true)));
    }

    #[test]
    fn test_dispatch_unknown_func() {
        let result = dispatch_string_func("unknown_str_func", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_list_string_functions() {
        let funcs = list_string_functions();
        assert!(funcs.contains(&"str_upper"));
        assert!(funcs.contains(&"str_lower"));
        assert!(funcs.contains(&"str_trim"));
        assert!(funcs.contains(&"str_contains"));
        assert!(funcs.contains(&"str_replace"));
    }
}
