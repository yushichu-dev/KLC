//! KLC 标准数学库 — 独立全局数学函数
//!
//! v1.4.3 新增: 提供无需 `math::` 前缀的全局数学函数。
//! 所有函数接受数字类型(Integer/Float)，非法输入安全返回 Null。
//!
//! ## 功能清单
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `abs(x)` | 数字绝对值 | `abs(-5)` → 5 |
//! | `sqrt(x)` | 平方根(负数→Null) | `sqrt(16.0)` → 4.0 |
//! | `pow(base, exp)` | 幂运算 | `pow(2, 3)` → 8.0 |
//! | `round(x)` | 四舍五入 | `round(3.6)` → 4.0 |
//! | `floor(x)` | 向下取整 | `floor(3.9)` → 3.0 |
//! | `ceil(x)` | 向上取整 | `ceil(3.1)` → 4.0 |
//! | `max(a, b)` | 取最大值 | `max(3, 7)` → 7.0 |
//! | `min(a, b)` | 取最小值 | `min(3, 7)` → 3.0 |
//! | `math_pi()` | 圆周率π | `math_pi()` → 3.1415... |
//! | `math_e()` | 自然常数e | `math_e()` → 2.7182... |
//!
//! ## 兼容性
//! - 原有 `math::abs(x)` 等带前缀调用方式完全不受影响
//! - 新增全局 `abs(x)` 等价于 `math::abs(x)`

use crate::bytecode::Value as BValue;

// ============================================================================
// 函数名常量
// ============================================================================

pub mod func_names {
    pub const ABS: &str = "abs";
    pub const SQRT: &str = "sqrt";
    pub const POW: &str = "pow";
    pub const ROUND: &str = "round";
    pub const FLOOR: &str = "floor";
    pub const CEIL: &str = "ceil";
    pub const MAX: &str = "max";
    pub const MIN: &str = "min";
    pub const MATH_PI: &str = "math_pi";
    pub const MATH_E: &str = "math_e";
}

// ============================================================================
// 核心实现
// ============================================================================

/// 提取数值为 f64（Integer 自动转换，其他类型返回 Null 并在非法输入时报错）
fn to_f64(val: &BValue) -> Option<f64> {
    match val {
        BValue::Integer(n) => Some(*n as f64),
        BValue::Float(f) => Some(*f),
        _ => None,
    }
}

/// abs(x) — 绝对值，保持原类型(Integer保持Integer，Float保持Float)
pub fn abs(val: &BValue) -> BValue {
    match val {
        BValue::Integer(n) => BValue::Integer(n.abs()),
        BValue::Float(f) => BValue::Float(f.abs()),
        _ => BValue::Null,
    }
}

/// sqrt(x) — 平方根，负数安全返回 Null
pub fn sqrt(val: &BValue) -> BValue {
    match to_f64(val) {
        Some(x) if x >= 0.0 => BValue::Float(x.sqrt()),
        Some(_) => BValue::Null, // 负数 → Null
        None => BValue::Null,
    }
}

/// pow(base, exp) — 幂运算
pub fn pow(base: &BValue, exp: &BValue) -> BValue {
    match (to_f64(base), to_f64(exp)) {
        (Some(b), Some(e)) => BValue::Float(b.powf(e)),
        _ => BValue::Null,
    }
}

/// round(x) — 四舍五入
pub fn round(val: &BValue) -> BValue {
    match to_f64(val) {
        Some(x) => BValue::Float(x.round()),
        None => BValue::Null,
    }
}

/// floor(x) — 向下取整
pub fn floor(val: &BValue) -> BValue {
    match to_f64(val) {
        Some(x) => BValue::Float(x.floor()),
        None => BValue::Null,
    }
}

/// ceil(x) — 向上取整
pub fn ceil(val: &BValue) -> BValue {
    match to_f64(val) {
        Some(x) => BValue::Float(x.ceil()),
        None => BValue::Null,
    }
}

/// max(a, b) — 取最大值
pub fn max(a: &BValue, b: &BValue) -> BValue {
    match (to_f64(a), to_f64(b)) {
        (Some(x), Some(y)) => BValue::Float(x.max(y)),
        _ => BValue::Null,
    }
}

/// min(a, b) — 取最小值
pub fn min(a: &BValue, b: &BValue) -> BValue {
    match (to_f64(a), to_f64(b)) {
        (Some(x), Some(y)) => BValue::Float(x.min(y)),
        _ => BValue::Null,
    }
}

/// math_pi() — 圆周率常量
pub fn math_pi() -> BValue {
    BValue::Float(std::f64::consts::PI)
}

/// math_e() — 自然常数常量
pub fn math_e() -> BValue {
    BValue::Float(std::f64::consts::E)
}

// ============================================================================
// VM 分发函数
// ============================================================================

/// VM 分发: 处理全局数学函数调用
///
/// 返回 `Some(value)` 表示已处理，返回 `None` 表示不匹配（VM 继续查找）。
pub fn dispatch_math_func(name: &str, args: &[BValue]) -> Option<BValue> {
    match name {
        func_names::ABS => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(abs(val))
        }
        func_names::SQRT => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(sqrt(val))
        }
        func_names::POW => {
            let base = args.first().unwrap_or(&BValue::Null);
            let exp = args.get(1).unwrap_or(&BValue::Null);
            Some(pow(base, exp))
        }
        func_names::ROUND => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(round(val))
        }
        func_names::FLOOR => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(floor(val))
        }
        func_names::CEIL => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(ceil(val))
        }
        func_names::MAX => {
            let a = args.first().unwrap_or(&BValue::Null);
            let b = args.get(1).unwrap_or(&BValue::Null);
            Some(max(a, b))
        }
        func_names::MIN => {
            let a = args.first().unwrap_or(&BValue::Null);
            let b = args.get(1).unwrap_or(&BValue::Null);
            Some(min(a, b))
        }
        func_names::MATH_PI => {
            Some(math_pi())
        }
        func_names::MATH_E => {
            Some(math_e())
        }
        _ => None,
    }
}

/// 返回所有已注册数学函数名列表
pub fn list_math_functions() -> Vec<&'static str> {
    vec![
        func_names::ABS,
        func_names::SQRT,
        func_names::POW,
        func_names::ROUND,
        func_names::FLOOR,
        func_names::CEIL,
        func_names::MAX,
        func_names::MIN,
        func_names::MATH_PI,
        func_names::MATH_E,
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abs_integer() {
        let result = abs(&BValue::Integer(-42));
        assert_eq!(result, BValue::Integer(42));
    }

    #[test]
    fn test_abs_integer_positive() {
        let result = abs(&BValue::Integer(42));
        assert_eq!(result, BValue::Integer(42));
    }

    #[test]
    fn test_abs_float() {
        let result = abs(&BValue::Float(-3.14));
        assert_eq!(result, BValue::Float(3.14));
    }

    #[test]
    fn test_abs_null_on_string() {
        let result = abs(&BValue::String("hello".into()));
        assert_eq!(result, BValue::Null);
    }

    #[test]
    fn test_sqrt_positive() {
        let result = sqrt(&BValue::Float(16.0));
        assert_eq!(result, BValue::Float(4.0));
    }

    #[test]
    fn test_sqrt_negative_returns_null() {
        let result = sqrt(&BValue::Float(-1.0));
        assert_eq!(result, BValue::Null);
    }

    #[test]
    fn test_sqrt_integer() {
        let result = sqrt(&BValue::Integer(9));
        assert_eq!(result, BValue::Float(3.0));
    }

    #[test]
    fn test_pow_basic() {
        let result = pow(&BValue::Float(2.0), &BValue::Float(3.0));
        assert_eq!(result, BValue::Float(8.0));
    }

    #[test]
    fn test_pow_integers() {
        let result = pow(&BValue::Integer(2), &BValue::Integer(8));
        assert_eq!(result, BValue::Float(256.0));
    }

    #[test]
    fn test_round_up() {
        let result = round(&BValue::Float(3.6));
        assert_eq!(result, BValue::Float(4.0));
    }

    #[test]
    fn test_round_down() {
        let result = round(&BValue::Float(3.4));
        assert_eq!(result, BValue::Float(3.0));
    }

    #[test]
    fn test_floor() {
        let result = floor(&BValue::Float(3.9));
        assert_eq!(result, BValue::Float(3.0));
    }

    #[test]
    fn test_ceil() {
        let result = ceil(&BValue::Float(3.1));
        assert_eq!(result, BValue::Float(4.0));
    }

    #[test]
    fn test_max() {
        let result = max(&BValue::Float(3.0), &BValue::Float(7.0));
        assert_eq!(result, BValue::Float(7.0));
    }

    #[test]
    fn test_min() {
        let result = min(&BValue::Float(3.0), &BValue::Float(7.0));
        assert_eq!(result, BValue::Float(3.0));
    }

    #[test]
    fn test_math_pi() {
        let result = math_pi();
        assert!(matches!(result, BValue::Float(f) if (f - std::f64::consts::PI).abs() < 0.001));
    }

    #[test]
    fn test_math_e() {
        let result = math_e();
        assert!(matches!(result, BValue::Float(f) if (f - std::f64::consts::E).abs() < 0.001));
    }

    // ─── dispatch 集成测试 ───

    #[test]
    fn test_dispatch_abs() {
        let result = dispatch_math_func("abs", &[BValue::Integer(-10)]);
        assert_eq!(result, Some(BValue::Integer(10)));
    }

    #[test]
    fn test_dispatch_sqrt() {
        let result = dispatch_math_func("sqrt", &[BValue::Float(25.0)]);
        assert_eq!(result, Some(BValue::Float(5.0)));
    }

    #[test]
    fn test_dispatch_pow() {
        let result = dispatch_math_func("pow", &[BValue::Float(2.0), BValue::Float(10.0)]);
        assert_eq!(result, Some(BValue::Float(1024.0)));
    }

    #[test]
    fn test_dispatch_max() {
        let result = dispatch_math_func("max", &[BValue::Integer(5), BValue::Integer(10)]);
        assert_eq!(result, Some(BValue::Float(10.0)));
    }

    #[test]
    fn test_dispatch_unknown_func() {
        let result = dispatch_math_func("unknown_math_func", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_list_math_functions() {
        let funcs = list_math_functions();
        assert!(funcs.contains(&"abs"));
        assert!(funcs.contains(&"sqrt"));
        assert!(funcs.contains(&"pow"));
        assert!(funcs.contains(&"round"));
        assert!(funcs.contains(&"floor"));
        assert!(funcs.contains(&"ceil"));
        assert!(funcs.contains(&"max"));
        assert!(funcs.contains(&"min"));
        assert!(funcs.contains(&"math_pi"));
        assert!(funcs.contains(&"math_e"));
    }
}
