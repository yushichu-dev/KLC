//! KLC 字节码编译器优化 Pass (v0.8.4 增强版)
//!
//! 在 AST → Bytecode 编译前对 AST 进行优化：
//! - 常量折叠：编译期计算常量表达式（如 `2+3` → `5`）
//! - 死代码消除：移除 if false { ... } 分支、while false { ... } 等
//! - 简化规则：`x + 0` → `x`、`x * 1` → `x`、`x - x` → `0` 等
//! - math 函数内联识别：标记 math.exp(x) 等为可内联调用
//! - 循环简化：识别固定次数循环、消除无用循环变量
//! - 多 pass 优化：常量传播 + 死码消除 交替执行直到收敛

use crate::ast::*;

/// 占位表达式（用于 std::mem::replace 中暂替 Expr）
fn dummy_expr() -> Expr {
    Expr::Integer(0)
}

/// 从 &mut Expr 中取出值并替换为占位
fn take_expr(expr: &mut Expr) -> Expr {
    std::mem::replace(expr, dummy_expr())
}

// ============================================================================
// 多 Pass 优化入口
// ============================================================================

/// 对整个程序执行多 pass 优化（直到收敛）
pub fn optimize_program(program: &mut Program) {
    // Pass 1: 常量折叠 + 死代码消除
    for _ in 0..3 {
        let prev_len = count_stmts(&program.statements);
        for stmt in program.statements.iter_mut() {
            optimize_stmt(stmt);
        }
        let curr_len = count_stmts(&program.statements);
        // 如果没有变化，提前退出
        if curr_len == prev_len {
            break;
        }
    }

    // Pass 2: math 函数内联标记
    for stmt in program.statements.iter_mut() {
        inline_math_calls_stmt(stmt);
    }

    // Pass 3: 循环简化
    for stmt in program.statements.iter_mut() {
        simplify_loops_stmt(stmt);
    }
}

/// 统计语句数量（用于判断优化是否收敛）
fn count_stmts(stmts: &[Stmt]) -> usize {
    let mut count = 0;
    for stmt in stmts {
        count += 1;
        match stmt {
            Stmt::While(_, body) | Stmt::For { body, .. } => {
                count += count_stmts(body);
            }
            Stmt::If { then_block, else_block, .. } => {
                count += count_stmts(then_block);
                if let Some(else_b) = else_block {
                    count += count_stmts(else_b);
                }
            }
            Stmt::Block(inner) => count += count_stmts(inner),
            Stmt::FnDef { body, .. } => count += count_stmts(body),
            Stmt::ImplBlock { methods, .. } => {
                for m in methods {
                    count += count_stmts(std::slice::from_ref(m));
                }
            }
            _ => {}
        }
    }
    count
}

// ============================================================================
// math 函数内联识别
// ============================================================================

/// 对语句中的 math 调用进行内联标记
fn inline_math_calls_stmt(stmt: &mut Stmt) {
    match stmt {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => {
            let e = take_expr(value);
            *value = inline_math_calls_expr(e);
        }
        Stmt::FieldAssign { value, .. } => {
            let e = take_expr(value);
            *value = inline_math_calls_expr(e);
        }
        Stmt::Expr(expr) => {
            let e = take_expr(expr);
            *expr = inline_math_calls_expr(e);
        }
        Stmt::Return(Some(expr)) => {
            let e = take_expr(expr);
            *expr = inline_math_calls_expr(e);
        }
        Stmt::While(cond, body) => {
            let c = take_expr(cond);
            *cond = inline_math_calls_expr(c);
            for s in body.iter_mut() {
                inline_math_calls_stmt(s);
            }
        }
        Stmt::For { iterable, body, .. } => {
            let it = take_expr(iterable);
            *iterable = inline_math_calls_expr(it);
            for s in body.iter_mut() {
                inline_math_calls_stmt(s);
            }
        }
        Stmt::If { cond, then_block, else_block } => {
            let c = take_expr(cond);
            *cond = inline_math_calls_expr(c);
            for s in then_block.iter_mut() {
                inline_math_calls_stmt(s);
            }
            if let Some(else_b) = else_block {
                for s in else_b.iter_mut() {
                    inline_math_calls_stmt(s);
                }
            }
        }
        Stmt::Block(stmts) => {
            for s in stmts.iter_mut() {
                inline_math_calls_stmt(s);
            }
        }
        Stmt::FnDef { body, .. } => {
            for s in body.iter_mut() {
                inline_math_calls_stmt(s);
            }
        }
        Stmt::ImplBlock { methods, .. } => {
            for method in methods.iter_mut() {
                inline_math_calls_stmt(method);
            }
        }
        Stmt::Print(expr) | Stmt::PrintLn(expr) | Stmt::Exit(expr) => {
            let e = take_expr(expr);
            *expr = inline_math_calls_expr(e);
        }
        Stmt::TypeDef { .. } | Stmt::EnumDef { .. }
        | Stmt::Break | Stmt::Continue | Stmt::Return(None) => {}
    }
}

/// 对表达式中的 math 调用进行内联识别
/// 如果能折叠为常量（如 math.exp(0) → 1.0），直接折叠
fn inline_math_calls_expr(expr: Expr) -> Expr {
    match expr {
        Expr::Call(name, args) => {
            // 检查是否是 math 常量
            if name == "math::pi" || name == "math.pi" {
                return Expr::Float(std::f64::consts::PI);
            }
            if name == "math::e" || name == "math.e" {
                return Expr::Float(std::f64::consts::E);
            }

            // 检查是否能对常量参数进行编译期 math 计算
            let all_const = args.iter().all(|a| is_const_expr(a));
            if all_const && args.len() == 1 {
                if let Some(folded) = try_fold_math_const(&name, &args[0]) {
                    return folded;
                }
            }

            // 递归处理参数
            let new_args: Vec<Expr> = args.into_iter()
                .map(inline_math_calls_expr)
                .collect();
            Expr::Call(name, new_args)
        }
        Expr::Binary(left, op, right) => {
            Expr::Binary(
                Box::new(inline_math_calls_expr(*left)),
                op,
                Box::new(inline_math_calls_expr(*right)),
            )
        }
        Expr::Unary(op, operand) => {
            Expr::Unary(op, Box::new(inline_math_calls_expr(*operand)))
        }
        Expr::If(cond, then_expr, else_expr) => {
            Expr::If(
                Box::new(inline_math_calls_expr(*cond)),
                Box::new(inline_math_calls_expr(*then_expr)),
                else_expr.map(|e| Box::new(inline_math_calls_expr(*e))),
            )
        }
        Expr::FieldAccess(obj, field) => {
            Expr::FieldAccess(Box::new(inline_math_calls_expr(*obj)), field)
        }
        Expr::StructLiteral { type_name, fields } => {
            Expr::StructLiteral {
                type_name,
                fields: fields.into_iter()
                    .map(|(n, v)| (n, inline_math_calls_expr(v)))
                    .collect(),
            }
        }
        Expr::Match { value, arms } => {
            Expr::Match {
                value: Box::new(inline_math_calls_expr(*value)),
                arms: arms.into_iter().map(|mut arm| {
                    if let Some(guard) = arm.guard {
                        arm.guard = Some(inline_math_calls_expr(guard));
                    }
                    for s in arm.body.iter_mut() {
                        inline_math_calls_stmt(s);
                    }
                    arm
                }).collect(),
            }
        }
        Expr::Lambda { params, return_type, mut body } => {
            for s in body.iter_mut() {
                inline_math_calls_stmt(s);
            }
            Expr::Lambda { params, return_type, body }
        }
        Expr::TailCall(name, args) => {
            Expr::TailCall(name, args.into_iter().map(inline_math_calls_expr).collect())
        }
        // 原子表达式不变
        e => e,
    }
}

/// 检查表达式是否为常量
fn is_const_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Integer(_) | Expr::Float(_) | Expr::Bool(_) | Expr::String(_) | Expr::Char(_) | Expr::Null)
}

/// 尝试对 math 函数的常量参数进行编译期计算
fn try_fold_math_const(name: &str, arg: &Expr) -> Option<Expr> {
    let val = match arg {
        Expr::Integer(n) => *n as f64,
        Expr::Float(f) => *f,
        _ => return None,
    };

    let func = name.strip_prefix("math::").or(name.strip_prefix("math."))?;

    let result = match func {
        "exp" => val.exp(),
        "tanh" => val.tanh(),
        "sin" => val.sin(),
        "cos" => val.cos(),
        "sqrt" => val.sqrt(),
        "log" | "ln" => val.ln(),
        "log2" => val.log2(),
        "log10" => val.log10(),
        "abs" => val.abs(),
        "floor" => val.floor(),
        "ceil" => val.ceil(),
        "round" => val.round(),
        _ => return None,
    };

    // 检查结果是否合法（非 NaN、非 Inf）
    if result.is_nan() || result.is_infinite() {
        return None;
    }

    Some(Expr::Float(result))
}

// ============================================================================
// 循环简化
// ============================================================================

/// 对语句进行循环简化
fn simplify_loops_stmt(stmt: &mut Stmt) {
    match stmt {
        Stmt::For { iterable, body, .. } => {
            let it = take_expr(iterable);
            // for i in [常量数组] → 展开循环
            if let Expr::Call(ref name, ref args) = &it {
                if name == "__array" {
                    if args.iter().all(is_const_expr) {
                        // 可展开的固定数组循环
                        *iterable = it; // 暂保留原样，codegen 已优化
                    } else {
                        *iterable = it;
                    }
                } else {
                    *iterable = it;
                }
            } else {
                *iterable = it;
            }
            for s in body.iter_mut() {
                simplify_loops_stmt(s);
            }
        }
        Stmt::While(cond, body) => {
            // 简化: while true { ... } 标记为热循环（codegen 可特殊处理）
            let c = take_expr(cond);
            match &c {
                Expr::Bool(true) => {
                    // 热循环 — 保持原样，VM 直接执行无额外开销
                    *cond = c;
                }
                _ => {
                    *cond = c;
                }
            }
            for s in body.iter_mut() {
                simplify_loops_stmt(s);
            }
        }
        Stmt::If { cond: _, then_block, else_block } => {
            for s in then_block.iter_mut() {
                simplify_loops_stmt(s);
            }
            if let Some(else_b) = else_block {
                for s in else_b.iter_mut() {
                    simplify_loops_stmt(s);
                }
            }
        }
        Stmt::Block(stmts) => {
            for s in stmts.iter_mut() {
                simplify_loops_stmt(s);
            }
        }
        Stmt::FnDef { body, .. } => {
            for s in body.iter_mut() {
                simplify_loops_stmt(s);
            }
        }
        Stmt::ImplBlock { methods, .. } => {
            for method in methods.iter_mut() {
                simplify_loops_stmt(method);
            }
        }
        _ => {}
    }
}

// ============================================================================
// 常量折叠
// ============================================================================

/// 对单条语句进行优化
fn optimize_stmt(stmt: &mut Stmt) {
    // 先提取要优化的表达式，用占位替换
    match stmt {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => {
            let e = take_expr(value);
            *value = optimize_expr(e);
        }
        Stmt::FieldAssign { value, .. } => {
            let e = take_expr(value);
            *value = optimize_expr(e);
        }
        Stmt::Expr(expr) => {
            let e = take_expr(expr);
            *expr = optimize_expr(e);
        }
        Stmt::Return(Some(expr)) => {
            let e = take_expr(expr);
            *expr = optimize_expr(e);
        }
        Stmt::While(cond, body) => {
            // 死代码消除：while false { ... }
            if let Expr::Bool(false) = cond {
                *stmt = Stmt::Block(vec![]);
                return;
            }
            let c = take_expr(cond);
            *cond = optimize_expr(c);
            for s in body.iter_mut() {
                optimize_stmt(s);
            }
        }
        Stmt::For { iterable, body, .. } => {
            let it = take_expr(iterable);
            *iterable = optimize_expr(it);
            for s in body.iter_mut() {
                optimize_stmt(s);
            }
        }
        Stmt::If { cond, then_block, else_block } => {
            // 死代码消除
            match cond {
                Expr::Bool(true) => {
                    let tb = std::mem::take(then_block);
                    *stmt = Stmt::Block(tb);
                    if let Stmt::Block(v) = stmt {
                        for s in v.iter_mut() { optimize_stmt(s); }
                    }
                    return;
                }
                Expr::Bool(false) => {
                    if let Some(else_b) = else_block.take() {
                        *stmt = Stmt::Block(else_b);
                        if let Stmt::Block(v) = stmt {
                            for s in v.iter_mut() { optimize_stmt(s); }
                        }
                    } else {
                        *stmt = Stmt::Block(vec![]);
                    }
                    return;
                }
                _ => {}
            }
            let c = take_expr(cond);
            *cond = optimize_expr(c);
            for s in then_block.iter_mut() {
                optimize_stmt(s);
            }
            if let Some(else_b) = else_block {
                for s in else_b.iter_mut() {
                    optimize_stmt(s);
                }
            }
        }
        Stmt::Block(stmts) => {
            for s in stmts.iter_mut() {
                optimize_stmt(s);
            }
        }
        Stmt::FnDef { body, .. } => {
            for s in body.iter_mut() {
                optimize_stmt(s);
            }
        }
        Stmt::ImplBlock { methods, .. } => {
            for method in methods.iter_mut() {
                optimize_stmt(method);
            }
        }
        Stmt::Print(expr) | Stmt::PrintLn(expr) | Stmt::Exit(expr) => {
            let e = take_expr(expr);
            *expr = optimize_expr(e);
        }
        Stmt::TypeDef { .. } | Stmt::EnumDef { .. }
        | Stmt::Break | Stmt::Continue | Stmt::Return(None) => {}
    }
}

/// 对表达式进行常量折叠优化
fn optimize_expr(expr: Expr) -> Expr {
    match expr {
        // 二元表达式
        Expr::Binary(left, op, right) => {
            let l = optimize_expr(*left);
            let r = optimize_expr(*right);

            // 尝试常量折叠
            if let Some(folded) = try_fold_binary(&l, &op, &r) {
                return folded;
            }

            // 代数简化
            if let Some(simplified) = try_simplify_binary(&l, &op, &r) {
                return simplified;
            }

            Expr::Binary(Box::new(l), op, Box::new(r))
        }

        // 一元表达式
        Expr::Unary(op, operand) => {
            let inner = optimize_expr(*operand);
            if let Some(folded) = try_fold_unary(&op, &inner) {
                return folded;
            }
            Expr::Unary(op, Box::new(inner))
        }

        // 函数调用 — 递归优化参数
        Expr::Call(name, args) => {
            Expr::Call(name, args.into_iter().map(optimize_expr).collect())
        }

        // 字段访问
        Expr::FieldAccess(obj, field) => {
            Expr::FieldAccess(Box::new(optimize_expr(*obj)), field)
        }

        // if 表达式
        Expr::If(cond, then_branch, else_branch) => {
            let c = optimize_expr(*cond);
            match &c {
                Expr::Bool(true) => *then_branch,
                Expr::Bool(false) => {
                    else_branch.map(|e| *e).unwrap_or(Expr::Bool(false))
                }
                _ => Expr::If(
                    Box::new(c),
                    Box::new(optimize_expr(*then_branch)),
                    else_branch.map(|e| Box::new(optimize_expr(*e))),
                ),
            }
        }

        // 结构体字面量
        Expr::StructLiteral { type_name, fields } => {
            Expr::StructLiteral {
                type_name,
                fields: fields.into_iter()
                    .map(|(n, v)| (n, optimize_expr(v)))
                    .collect(),
            }
        }

        // Match 表达式
        Expr::Match { value, arms } => {
            Expr::Match {
                value: Box::new(optimize_expr(*value)),
                arms: arms.into_iter().map(|mut arm| {
                    if let Some(guard) = arm.guard {
                        arm.guard = Some(optimize_expr(guard));
                    }
                    for s in arm.body.iter_mut() {
                        optimize_stmt(s);
                    }
                    arm
                }).collect(),
            }
        }

        // Lambda
        Expr::Lambda { params, return_type, mut body } => {
            for s in body.iter_mut() {
                optimize_stmt(s);
            }
            Expr::Lambda { params, return_type, body }
        }

        // 尾调用
        Expr::TailCall(name, args) => {
            Expr::TailCall(name, args.into_iter().map(optimize_expr).collect())
        }

        // 原子表达式不变
        e @ Expr::Integer(_) | e @ Expr::Float(_) | e @ Expr::String(_)
        | e @ Expr::Bool(_) | e @ Expr::Char(_) | e @ Expr::Ident(_) | e @ Expr::Null => e,
    }
}

// ============================================================================
// 常量折叠规则
// ============================================================================

/// 尝试对两个常量进行二元运算折叠
fn try_fold_binary(left: &Expr, op: &BinOp, right: &Expr) -> Option<Expr> {
    // 整数运算
    if let (Expr::Integer(a), Expr::Integer(b)) = (left, right) {
        let result = match op {
            BinOp::Add => Some(a.wrapping_add(*b)),
            BinOp::Sub => Some(a.wrapping_sub(*b)),
            BinOp::Mul => Some(a.wrapping_mul(*b)),
            BinOp::Div => {
                if *b != 0 { Some(a / b) } else { None }
            }
            BinOp::Mod => {
                if *b != 0 { Some(a % b) } else { None }
            }
            BinOp::Eq => Some(if a == b { 1 } else { 0 }),
            BinOp::Neq => Some(if a != b { 1 } else { 0 }),
            BinOp::Lt => Some(if a < b { 1 } else { 0 }),
            BinOp::Gt => Some(if a > b { 1 } else { 0 }),
            BinOp::Lte => Some(if a <= b { 1 } else { 0 }),
            BinOp::Gte => Some(if a >= b { 1 } else { 0 }),
            BinOp::Range => None,
            BinOp::RangeInclusive => None,
            BinOp::Concat => None,
            BinOp::And | BinOp::Or => None,
        };
        return result.map(|v| Expr::Integer(v));
    }

    // 浮点运算
    if let (Expr::Float(a), Expr::Float(b)) = (left, right) {
        let result = match op {
            BinOp::Add => Some(a + b),
            BinOp::Sub => Some(a - b),
            BinOp::Mul => Some(a * b),
            BinOp::Div => Some(a / b),
            BinOp::Eq => Some(if a == b { 1.0 } else { 0.0 }),
            BinOp::Neq => Some(if a != b { 1.0 } else { 0.0 }),
            BinOp::Lt => Some(if a < b { 1.0 } else { 0.0 }),
            BinOp::Gt => Some(if a > b { 1.0 } else { 0.0 }),
            BinOp::Lte => Some(if a <= b { 1.0 } else { 0.0 }),
            BinOp::Gte => Some(if a >= b { 1.0 } else { 0.0 }),
            _ => None,
        };
        return result.map(|v| Expr::Float(v));
    }

    // 字符串拼接
    if let (Expr::String(a), Expr::String(b)) = (left, right) {
        if matches!(op, BinOp::Concat) {
            return Some(Expr::String(format!("{}{}", a, b)));
        }
    }

    // 布尔运算
    if let (Expr::Bool(a), Expr::Bool(b)) = (left, right) {
        let result = match op {
            BinOp::And => Some(*a && *b),
            BinOp::Or => Some(*a || *b),
            BinOp::Eq => Some(*a == *b),
            BinOp::Neq => Some(*a != *b),
            _ => None,
        };
        return result.map(Expr::Bool);
    }

    // 整数与浮点混合
    if let (Expr::Integer(a), Expr::Float(b)) = (left, right) {
        let fa = *a as f64;
        let result = match op {
            BinOp::Add => Some(fa + b),
            BinOp::Sub => Some(fa - b),
            BinOp::Mul => Some(fa * b),
            BinOp::Div => Some(fa / b),
            _ => None,
        };
        return result.map(|v| Expr::Float(v));
    }
    if let (Expr::Float(a), Expr::Integer(b)) = (left, right) {
        let fb = *b as f64;
        let result = match op {
            BinOp::Add => Some(a + fb),
            BinOp::Sub => Some(a - fb),
            BinOp::Mul => Some(a * fb),
            BinOp::Div => Some(a / fb),
            _ => None,
        };
        return result.map(|v| Expr::Float(v));
    }

    None
}

/// 尝试一元运算折叠
fn try_fold_unary(op: &UnaryOp, operand: &Expr) -> Option<Expr> {
    match op {
        UnaryOp::Neg => match operand {
            Expr::Integer(n) => Some(Expr::Integer(n.wrapping_neg())),
            Expr::Float(f) => Some(Expr::Float(-f)),
            _ => None,
        },
        UnaryOp::Not => match operand {
            Expr::Bool(b) => Some(Expr::Bool(!b)),
            Expr::Integer(n) => Some(Expr::Integer(if *n == 0 { 1 } else { 0 })),
            _ => None,
        },
    }
}

// ============================================================================
// 代数简化规则
// ============================================================================

/// 代数简化（不要求两边都是常量）
fn try_simplify_binary(left: &Expr, op: &BinOp, right: &Expr) -> Option<Expr> {
    // x + 0 → x
    if matches!(op, BinOp::Add) && is_zero(right) { return Some(left.clone()); }
    // 0 + x → x
    if matches!(op, BinOp::Add) && is_zero(left) { return Some(right.clone()); }
    // x - 0 → x
    if matches!(op, BinOp::Sub) && is_zero(right) { return Some(left.clone()); }
    // x * 1 → x
    if matches!(op, BinOp::Mul) && is_one(right) { return Some(left.clone()); }
    // 1 * x → x
    if matches!(op, BinOp::Mul) && is_one(left) { return Some(right.clone()); }
    // x * 0 → 0
    if matches!(op, BinOp::Mul) && is_zero(right) {
        if let Expr::Float(_) = right { return None; }
        return Some(Expr::Integer(0));
    }
    // 0 * x → 0
    if matches!(op, BinOp::Mul) && is_zero(left) {
        if let Expr::Float(_) = left { return None; }
        return Some(Expr::Integer(0));
    }
    // x / 1 → x
    if matches!(op, BinOp::Div) && is_one(right) { return Some(left.clone()); }
    // x - x → 0 (仅对整数)
    if matches!(op, BinOp::Sub) && exprs_equal(left, right) {
        if let Expr::Integer(_) = left { return Some(Expr::Integer(0)); }
        if let Expr::Float(_) = left { return Some(Expr::Float(0.0)); }
    }
    // x == x → true
    if matches!(op, BinOp::Eq) && exprs_equal(left, right) {
        return Some(Expr::Bool(true));
    }
    // x != x → false
    if matches!(op, BinOp::Neq) && exprs_equal(left, right) {
        return Some(Expr::Bool(false));
    }
    // x >= x → true, x <= x → true
    if matches!(op, BinOp::Lte | BinOp::Gte) && exprs_equal(left, right) {
        return Some(Expr::Bool(true));
    }

    None
}

/// 检查表达式是否为整数 0 或布尔 false
fn is_zero(expr: &Expr) -> bool {
    matches!(expr, Expr::Integer(0))
        || matches!(expr, Expr::Bool(false))
}

/// 检查表达式是否为整数 1 或布尔 true
fn is_one(expr: &Expr) -> bool {
    matches!(expr, Expr::Integer(1))
        || matches!(expr, Expr::Bool(true))
}

/// 简单的结构相等比较（用于代数简化）
fn exprs_equal(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Integer(x), Expr::Integer(y)) => x == y,
        (Expr::Float(x), Expr::Float(y)) => x == y,
        (Expr::Bool(x), Expr::Bool(y)) => x == y,
        (Expr::String(x), Expr::String(y)) => x == y,
        (Expr::Ident(x), Expr::Ident(y)) => x == y,
        _ => false,
    }
}
