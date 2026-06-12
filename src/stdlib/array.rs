//! KLC 数组操作函数库 — 独立全局数组函数
//!
//! v1.4.3 新增: 提供无需方法调用的全局数组函数。
//! 非数组输入安全处理。
//!
//! ## 功能清单
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `arr_len(arr)` | 数组长度 | `arr_len([1, 2, 3])` → 3 |
//! | `arr_push(arr, val)` | 追加元素(修改原数组) | `arr_push(a, 42)` |
//! | `arr_pop(arr)` | 弹出最后元素 | `arr_pop(a)` → 弹出的值 |
//! | `arr_slice(arr, start, end)` | 切片(返回新数组) | `arr_slice(a, 0, 2)` |
//!
//! ## 兼容性
//! - 原有数组方法 (`arr.push()`, `arr.pop()` 等) 完全不受影响

use std::rc::Rc;
use std::cell::RefCell;
use crate::bytecode::Value as BValue;

// ============================================================================
// 函数名常量
// ============================================================================

pub mod func_names {
    pub const ARR_LEN: &str = "arr_len";
    pub const ARR_PUSH: &str = "arr_push";
    pub const ARR_POP: &str = "arr_pop";
    pub const ARR_SLICE: &str = "arr_slice";
}

// ============================================================================
// 核心实现
// ============================================================================

/// arr_len(arr) — 获取数组长度。非数组返回 0
pub fn arr_len(val: &BValue) -> BValue {
    match val {
        BValue::Array(arr) => BValue::Integer(arr.borrow().len() as i64),
        _ => BValue::Integer(0),
    }
}

/// arr_push(arr, val) — 向数组追加元素，修改原数组并返回
pub fn arr_push(arr_val: &BValue, new_val: &BValue) -> Option<BValue> {
    match arr_val {
        BValue::Array(arr) => {
            arr.borrow_mut().push(new_val.clone());
            Some(arr_val.clone()) // 返回修改后的数组
        }
        _ => None, // 非数组，不可操作
    }
}

/// arr_pop(arr) — 弹出数组最后一个元素，返回弹出的值
pub fn arr_pop(arr_val: &BValue) -> Option<BValue> {
    match arr_val {
        BValue::Array(arr) => {
            let popped = arr.borrow_mut().pop().unwrap_or(BValue::Null);
            Some(popped)
        }
        _ => Some(BValue::Null),
    }
}

/// arr_slice(arr, start, end) — 切片，返回新数组（不修改原数组）
/// start/end 可以超出范围，自动裁剪到有效范围
pub fn arr_slice(arr_val: &BValue, start_val: &BValue, end_val: &BValue) -> BValue {
    match arr_val {
        BValue::Array(arr) => {
            let items = arr.borrow();
            let len = items.len() as i64;
            let start = match start_val {
                BValue::Integer(s) => (*s).max(0).min(len),
                _ => 0,
            };
            let end = match end_val {
                BValue::Integer(e) => (*e).max(0).min(len),
                _ => len,
            };
            if start >= end {
                return BValue::Array(Rc::new(RefCell::new(Vec::new())));
            }
            let sliced: Vec<BValue> = items[start as usize..end as usize].to_vec();
            BValue::Array(Rc::new(RefCell::new(sliced)))
        }
        _ => BValue::Null,
    }
}

// ============================================================================
// VM 分发函数
// ============================================================================

/// VM 分发: 处理全局数组函数调用
pub fn dispatch_array_func(name: &str, args: &[BValue]) -> Option<BValue> {
    match name {
        func_names::ARR_LEN => {
            let val = args.first().unwrap_or(&BValue::Null);
            Some(arr_len(val))
        }
        func_names::ARR_PUSH => {
            let arr_val = args.first().unwrap_or(&BValue::Null);
            let new_val = args.get(1).unwrap_or(&BValue::Null);
            arr_push(arr_val, new_val)
        }
        func_names::ARR_POP => {
            let arr_val = args.first().unwrap_or(&BValue::Null);
            arr_pop(arr_val)
        }
        func_names::ARR_SLICE => {
            let arr_val = args.first().unwrap_or(&BValue::Null);
            let start = args.get(1).unwrap_or(&BValue::Integer(0));
            let end_val = args.get(2).unwrap_or(&BValue::Integer(i64::MAX));
            Some(arr_slice(arr_val, start, end_val))
        }
        _ => None,
    }
}

/// 返回所有已注册数组函数名列表
pub fn list_array_functions() -> Vec<&'static str> {
    vec![
        func_names::ARR_LEN,
        func_names::ARR_PUSH,
        func_names::ARR_POP,
        func_names::ARR_SLICE,
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_arr(items: Vec<BValue>) -> BValue {
        BValue::Array(Rc::new(RefCell::new(items)))
    }

    #[test]
    fn test_arr_len_empty() {
        let arr = make_arr(vec![]);
        assert_eq!(arr_len(&arr), BValue::Integer(0));
    }

    #[test]
    fn test_arr_len_three() {
        let arr = make_arr(vec![
            BValue::Integer(1),
            BValue::Integer(2),
            BValue::Integer(3),
        ]);
        assert_eq!(arr_len(&arr), BValue::Integer(3));
    }

    #[test]
    fn test_arr_len_non_array() {
        assert_eq!(arr_len(&BValue::Integer(42)), BValue::Integer(0));
    }

    #[test]
    fn test_arr_push() {
        let arr = make_arr(vec![BValue::Integer(1)]);
        let result = arr_push(&arr, &BValue::Integer(2));
        assert!(result.is_some());
        // 验证原数组被修改
        assert_eq!(arr_len(&arr), BValue::Integer(2));
    }

    #[test]
    fn test_arr_push_non_array() {
        let result = arr_push(&BValue::Integer(42), &BValue::Integer(1));
        assert!(result.is_none());
    }

    #[test]
    fn test_arr_pop() {
        let arr = make_arr(vec![BValue::Integer(1), BValue::Integer(2)]);
        let popped = arr_pop(&arr);
        assert_eq!(popped, Some(BValue::Integer(2)));
        // 验证原数组被修改
        assert_eq!(arr_len(&arr), BValue::Integer(1));
    }

    #[test]
    fn test_arr_pop_empty() {
        let arr = make_arr(vec![]);
        let popped = arr_pop(&arr);
        assert_eq!(popped, Some(BValue::Null));
    }

    #[test]
    fn test_arr_slice_basic() {
        let arr = make_arr(vec![
            BValue::Integer(10),
            BValue::Integer(20),
            BValue::Integer(30),
            BValue::Integer(40),
        ]);
        let result = arr_slice(&arr, &BValue::Integer(1), &BValue::Integer(3));
        match result {
            BValue::Array(rc) => {
                let items = rc.borrow();
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], BValue::Integer(20));
                assert_eq!(items[1], BValue::Integer(30));
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_arr_slice_out_of_range() {
        let arr = make_arr(vec![BValue::Integer(1)]);
        // end 超出范围，应该自动裁剪
        let result = arr_slice(&arr, &BValue::Integer(0), &BValue::Integer(100));
        match result {
            BValue::Array(rc) => {
                let items = rc.borrow();
                assert_eq!(items.len(), 1);
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_arr_slice_non_array() {
        let result = arr_slice(
            &BValue::Integer(42),
            &BValue::Integer(0),
            &BValue::Integer(1),
        );
        assert_eq!(result, BValue::Null);
    }

    // ─── dispatch 集成测试 ───

    #[test]
    fn test_dispatch_arr_len() {
        let arr = make_arr(vec![BValue::Integer(1), BValue::Integer(2)]);
        let result = dispatch_array_func("arr_len", &[arr]);
        assert_eq!(result, Some(BValue::Integer(2)));
    }

    #[test]
    fn test_dispatch_arr_pop() {
        let arr = make_arr(vec![BValue::Integer(100)]);
        let result = dispatch_array_func("arr_pop", &[arr.clone()]);
        assert_eq!(result, Some(BValue::Integer(100)));
    }

    #[test]
    fn test_dispatch_unknown_func() {
        let result = dispatch_array_func("unknown_arr_func", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_list_array_functions() {
        let funcs = list_array_functions();
        assert!(funcs.contains(&"arr_len"));
        assert!(funcs.contains(&"arr_push"));
        assert!(funcs.contains(&"arr_pop"));
        assert!(funcs.contains(&"arr_slice"));
    }
}
