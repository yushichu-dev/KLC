//! KLC AST 级优化 pass
//!
//! 在代码生成之前对 AST 进行变换优化:
//! - 常量折叠 (Constant Folding)
//! - 常量传播 (Constant Propagation)
//! - 死代码消除 (Dead Code Elimination)
//! - 强度削减 (Strength Reduction)
//! - 简化代数恒等式 (Algebraic Simplification)

use crate::ast::{BinOp, Expr, Stmt};
use std::collections::{HashMap, HashSet};

// ============================================================================
// 常量求值辅助
// ============================================================================

/// 尝试将表达式求值为 i64 常量
fn eval_const(e: &Expr) -> Option<i64> {
    match e {
        Expr::Integer(n) => Some(*n),
        Expr::Bool(b) => Some(if *b { 1 } else { 0 }),
        Expr::Char(c) => Some(*c as i64),
        Expr::Binary(l, op, r) => {
            let lv = eval_const(l)?;
            let rv = eval_const(r)?;
            match op {
                BinOp::Add => Some(lv.wrapping_add(rv)),
                BinOp::Sub => Some(lv.wrapping_sub(rv)),
                BinOp::Mul => Some(lv.wrapping_mul(rv)),
                BinOp::Div if rv != 0 => Some(lv / rv),
                BinOp::Mod if rv != 0 => Some(lv % rv),
                BinOp::Eq => Some(if lv == rv { 1 } else { 0 }),
                BinOp::Neq => Some(if lv != rv { 1 } else { 0 }),
                BinOp::Lt => Some(if lv < rv { 1 } else { 0 }),
                BinOp::Gt => Some(if lv > rv { 1 } else { 0 }),
                BinOp::Lte => Some(if lv <= rv { 1 } else { 0 }),
                BinOp::Gte => Some(if lv >= rv { 1 } else { 0 }),
                BinOp::And => Some(if lv != 0 && rv != 0 { 1 } else { 0 }),
                BinOp::Or => Some(if lv != 0 || rv != 0 { 1 } else { 0 }),
                _ => None,
            }
        }
        Expr::Unary(crate::ast::UnaryOp::Neg, inner) => {
            Some(eval_const(inner)?.wrapping_neg())
        }
        Expr::Unary(crate::ast::UnaryOp::Not, inner) => {
            let v = eval_const(inner)?;
            Some(if v == 0 { 1 } else { 0 })
        }
        _ => None,
    }
}

/// 检查表达式是否是 "简单" 的（编译代价很低：字面量或变量）
#[allow(dead_code)]
fn is_simple(e: &Expr) -> bool {
    matches!(e, Expr::Integer(_) | Expr::Bool(_) | Expr::Char(_) | Expr::Ident(_))
}

// ============================================================================
// 表达式优化
// ============================================================================

/// 优化表达式（递归）
fn opt_expr(e: Expr) -> Expr {
    match e {
        Expr::Binary(l, op, r) => opt_binary(*l, op, *r),
        Expr::Unary(uop, inner) => {
            let inner = opt_expr(*inner);
            // 常量折叠: -5, !true
            if let Some(v) = eval_const(&Expr::Unary(uop.clone(), Box::new(inner.clone()))) {
                return Expr::Integer(v);
            }
            Expr::Unary(uop, Box::new(inner))
        }
        Expr::If(cond, then_val, else_val) => {
            let cond = opt_expr(*cond);
            let then_val = Box::new(opt_expr(*then_val));
            let else_val = else_val.map(|e| Box::new(opt_expr(*e)));
            // 常量条件: if true → then, if false → else
            if let Some(v) = eval_const(&cond) {
                return if v != 0 { *then_val } else { *else_val.unwrap_or(Box::new(Expr::Integer(0))) };
            }
            Expr::If(Box::new(cond), then_val, else_val)
        }
        Expr::Call(name, args) => {
            Expr::Call(name, args.into_iter().map(opt_expr).collect())
        }
        Expr::TailCall(name, args) => {
            // 尾调用参数也需要优化（常量折叠等）
            Expr::TailCall(name, args.into_iter().map(opt_expr).collect())
        }
        Expr::StructLiteral { type_name, fields } => {
            Expr::StructLiteral {
                type_name,
                fields: fields.into_iter()
                    .map(|(n, v)| (n, opt_expr(v)))
                    .collect(),
            }
        }
        other => other, // Ident, Integer, Float, String, Bool, Char 等不需要优化
    }
}

/// 优化二元运算
fn opt_binary(l: Expr, op: BinOp, r: Expr) -> Expr {
    let l = opt_expr(l);
    let r = opt_expr(r);

    // 1. 常量折叠: 两个操作数都是常量 → 直接计算结果
    if let (Some(lv), Some(rv)) = (eval_const(&l), eval_const(&r)) {
        if let Some(result) = match &op {
            BinOp::Add => Some(lv.wrapping_add(rv)),
            BinOp::Sub => Some(lv.wrapping_sub(rv)),
            BinOp::Mul => Some(lv.wrapping_mul(rv)),
            BinOp::Div if rv != 0 => Some(lv / rv),
            BinOp::Mod if rv != 0 => Some(lv % rv),
            BinOp::Eq => Some(if lv == rv { 1 } else { 0 }),
            BinOp::Neq => Some(if lv != rv { 1 } else { 0 }),
            BinOp::Lt => Some(if lv < rv { 1 } else { 0 }),
            BinOp::Gt => Some(if lv > rv { 1 } else { 0 }),
            BinOp::Lte => Some(if lv <= rv { 1 } else { 0 }),
            BinOp::Gte => Some(if lv >= rv { 1 } else { 0 }),
            BinOp::And => Some(if lv != 0 && rv != 0 { 1 } else { 0 }),
            BinOp::Or => Some(if lv != 0 || rv != 0 { 1 } else { 0 }),
            _ => None,
        } {
            return Expr::Integer(result);
        }
    }

    // 2. 代数恒等式 / 强度削减
    match &op {
        // x + 0 → x, 0 + x → x
        BinOp::Add => {
            if eval_const(&r) == Some(0) { return l; }
            if eval_const(&l) == Some(0) { return r; }
        }
        // x - 0 → x
        BinOp::Sub => {
            if eval_const(&r) == Some(0) { return l; }
            // x - x → 0
            if l == r { return Expr::Integer(0); }
        }
        // x * 1 → x, 1 * x → x, x * 0 → 0
        BinOp::Mul => {
            if eval_const(&r) == Some(1) { return l; }
            if eval_const(&l) == Some(1) { return r; }
            if eval_const(&r) == Some(0) { return Expr::Integer(0); }
            if eval_const(&l) == Some(0) { return Expr::Integer(0); }
        }
        // x / 1 → x
        BinOp::Div => {
            if eval_const(&r) == Some(1) { return l; }
            if l == r { return Expr::Integer(1); }
        }
        // x % 1 → 0
        BinOp::Mod => {
            if eval_const(&r) == Some(1) { return Expr::Integer(0); }
        }
        // x == x → 1
        BinOp::Eq => {
            if l == r { return Expr::Integer(1); }
        }
        // x != x → 0
        BinOp::Neq => {
            if l == r { return Expr::Integer(0); }
        }
        // 逻辑: x and false → 0
        BinOp::And => {
            if eval_const(&r) == Some(0) { return Expr::Integer(0); }
            if eval_const(&l) == Some(0) { return Expr::Integer(0); }
        }
        BinOp::Or => {
            if eval_const(&r) == Some(0) { return l; }
            if eval_const(&l) == Some(0) { return r; }
        }
        _ => {}
    }

    Expr::Binary(Box::new(l), op, Box::new(r))
}

// ============================================================================
// 语句优化
// ============================================================================

/// 优化单个语句
fn opt_stmt(s: Stmt) -> Stmt {
    match s {
        Stmt::Let { name, type_ann, mutable, value } => {
            let value = opt_expr(value);
            Stmt::Let { name, type_ann, mutable, value }
        }
        Stmt::Assign { name, value } => {
            let value = opt_expr(value);
            Stmt::Assign { name, value }
        }
        Stmt::FieldAssign { obj, field, value } => {
            let value = opt_expr(value);
            Stmt::FieldAssign { obj, field, value }
        }
        Stmt::Expr(e) => Stmt::Expr(opt_expr(e)),
        Stmt::Return(e) => Stmt::Return(e.map(opt_expr)),
        Stmt::Print(e) => Stmt::Print(opt_expr(e)),
        Stmt::PrintLn(e) => Stmt::PrintLn(opt_expr(e)),
        Stmt::While(cond, body) => {
            let cond = opt_expr(cond);
            let body = body.into_iter().map(opt_stmt).collect();
            // while false → skip (死代码消除)
            if eval_const(&cond) == Some(0) {
                return Stmt::Expr(Expr::Integer(0)); // no-op
            }
            Stmt::While(cond, body)
        }
        Stmt::For { var, iterable, body } => {
            let iterable = opt_expr(iterable);
            let body = body.into_iter().map(opt_stmt).collect();
            Stmt::For { var, iterable, body }
        }
        Stmt::If { cond, then_block, else_block } => {
            let cond = opt_expr(cond);
            // if false → else_block or skip
            if eval_const(&cond) == Some(0) {
                if let Some(b) = else_block {
                    return Stmt::Block(b);
                }
                return Stmt::Expr(Expr::Integer(0)); // no-op
            }
            // if true → then_block
            if let Some(v) = eval_const(&cond) {
                if v != 0 {
                    return Stmt::Block(then_block);
                }
            }
            let then_block = then_block.into_iter().map(opt_stmt).collect();
            let else_block = else_block.map(|b| b.into_iter().map(opt_stmt).collect());
            Stmt::If { cond, then_block, else_block }
        }
        Stmt::Block(stmts) => {
            Stmt::Block(stmts.into_iter().map(opt_stmt).collect())
        }
        // 其他语句原样保留
        other => other,
    }
}

// ============================================================================
// 常量传播
// ============================================================================

/// 收集直接赋值常量的变量（单次扫描，不做迭代到不动点）
fn collect_const_props(stmts: &[Stmt]) -> HashMap<String, i64> {
    let mut props = HashMap::new();
    for s in stmts {
        match s {
            Stmt::Let { name, value, .. } | Stmt::Assign { name, value } => {
                if let Some(v) = eval_const(value) {
                    props.insert(name.clone(), v);
                } else {
                    // 变量被赋予非常量值，移除旧绑定
                    props.remove(name);
                }
            }
            Stmt::While(_, body) => {
                // 循环体中的赋值不能安全传播（可能多次执行）
                for s in body {
                    if let Stmt::Let { name, .. } | Stmt::Assign { name, .. } = s {
                        props.remove(name);
                    }
                }
            }
            Stmt::For { body, .. } => {
                for s in body {
                    if let Stmt::Let { name, .. } | Stmt::Assign { name, .. } = s {
                        props.remove(name);
                    }
                }
            }
            Stmt::If { then_block, else_block, .. } => {
                // 两个分支可能赋不同值，保守处理
                for s in then_block.iter().chain(else_block.iter().flatten()) {
                    if let Stmt::Let { name, .. } | Stmt::Assign { name, .. } = s {
                        props.remove(name);
                    }
                }
            }
            _ => {}
        }
    }
    props
}

/// 在表达式中替换常量变量
fn propagate_expr(e: Expr, consts: &HashMap<String, i64>) -> Expr {
    match e {
        Expr::Ident(name) => {
            if let Some(&v) = consts.get(&name) {
                Expr::Integer(v)
            } else {
                Expr::Ident(name)
            }
        }
        Expr::Binary(l, op, r) => {
            let l = propagate_expr(*l, consts);
            let r = propagate_expr(*r, consts);
            opt_binary(l, op, r)
        }
        Expr::Unary(uop, inner) => {
            Expr::Unary(uop, Box::new(propagate_expr(*inner, consts)))
        }
        Expr::Call(name, args) => {
            Expr::Call(name, args.into_iter().map(|a| propagate_expr(a, consts)).collect())
        }
        Expr::TailCall(name, args) => {
            Expr::TailCall(name, args.into_iter().map(|a| propagate_expr(a, consts)).collect())
        }
        Expr::If(cond, then_val, else_val) => {
            Expr::If(
                Box::new(propagate_expr(*cond, consts)),
                Box::new(propagate_expr(*then_val, consts)),
                else_val.map(|e| Box::new(propagate_expr(*e, consts))),
            )
        }
        Expr::StructLiteral { type_name, fields } => {
            Expr::StructLiteral {
                type_name,
                fields: fields.into_iter()
                    .map(|(n, v)| (n, propagate_expr(v, consts)))
                    .collect(),
            }
        }
        other => other,
    }
}

/// 在语句中替换常量变量
fn propagate_stmt(s: Stmt, consts: &HashMap<String, i64>) -> Stmt {
    match s {
        Stmt::Let { name, type_ann, mutable, value } => {
            Stmt::Let { name, type_ann, mutable, value: propagate_expr(value, consts) }
        }
        Stmt::Assign { name, value } => {
            Stmt::Assign { name, value: propagate_expr(value, consts) }
        }
        Stmt::FieldAssign { obj, field, value } => {
            Stmt::FieldAssign { obj, field, value: propagate_expr(value, consts) }
        }
        Stmt::Expr(e) => Stmt::Expr(propagate_expr(e, consts)),
        Stmt::Return(e) => Stmt::Return(e.map(|v| propagate_expr(v, consts))),
        Stmt::Print(e) => Stmt::Print(propagate_expr(e, consts)),
        Stmt::PrintLn(e) => Stmt::PrintLn(propagate_expr(e, consts)),
        Stmt::While(cond, body) => {
            Stmt::While(
                propagate_expr(cond, consts),
                body.into_iter().map(|s| propagate_stmt(s, consts)).collect(),
            )
        }
        Stmt::For { var, iterable, body } => {
            Stmt::For {
                var,
                iterable: propagate_expr(iterable, consts),
                body: body.into_iter().map(|s| propagate_stmt(s, consts)).collect(),
            }
        }
        Stmt::If { cond, then_block, else_block } => {
            Stmt::If {
                cond: propagate_expr(cond, consts),
                then_block: then_block.into_iter().map(|s| propagate_stmt(s, consts)).collect(),
                else_block: else_block.map(|b| b.into_iter().map(|s| propagate_stmt(s, consts)).collect()),
            }
        }
        Stmt::Block(stmts) => {
            Stmt::Block(stmts.into_iter().map(|s| propagate_stmt(s, consts)).collect())
        }
        other => other,
    }
}

// ============================================================================
// 未使用变量消除 (减少栈空间分配)
// ============================================================================

/// 分析变量使用情况，返回真正被引用的变量集合
fn analyze_used_vars(stmts: &[Stmt]) -> HashSet<String> {
    let mut used = HashSet::new();
    fn walk_expr(e: &Expr, used: &mut HashSet<String>) {
        match e {
            Expr::Ident(name) => { used.insert(name.clone()); }
            Expr::Binary(l, _, r) => { walk_expr(l, used); walk_expr(r, used); }
            Expr::Unary(_, inner) => walk_expr(inner, used),
            Expr::Call(_, args) => { for a in args { walk_expr(a, used); } }
            Expr::TailCall(_, args) => { for a in args { walk_expr(a, used); } }
            Expr::If(c, t, el) => { walk_expr(c, used); walk_expr(t, used); if let Some(e) = el { walk_expr(e, used); } }
            Expr::StructLiteral { fields, .. } => { for (_, v) in fields { walk_expr(v, used); } }
            _ => {}
        }
    }
    fn walk_stmts(stmts: &[Stmt], used: &mut HashSet<String>) {
        for s in stmts {
            match s {
                Stmt::Let { value, .. } | Stmt::Assign { value, .. } => walk_expr(value, used),
                Stmt::Expr(e) => walk_expr(e, used),
                Stmt::Print(e) | Stmt::PrintLn(e) => walk_expr(e, used),
                Stmt::Return(e) => { if let Some(v) = e { walk_expr(v, used); } }
                Stmt::While(cond, body) => {
                    walk_expr(cond, used);
                    walk_stmts(body, used);
                }
                Stmt::For { iterable, body, .. } => {
                    walk_expr(iterable, used);
                    walk_stmts(body, used);
                }
                Stmt::If { cond, then_block, else_block } => {
                    walk_expr(cond, used);
                    walk_stmts(then_block, used);
                    if let Some(b) = else_block { walk_stmts(b, used); }
                }
                Stmt::Block(stmts) => walk_stmts(stmts, used),
                Stmt::FieldAssign { value, .. } => walk_expr(value, used),
                _ => {}
            }
        }
    }
    walk_stmts(stmts, &mut used);
    used
}

// ============================================================================
// 公开 API
// ============================================================================

/// 对整个程序进行 AST 级优化
///
/// 优化步骤:
/// 1. 表达式级优化（常量折叠 + 代数简化）
/// 2. 常量传播
/// 3. 再次常量折叠（传播后可能产生新的可折叠表达式）
/// 4. 公共子表达式消除 (CSE)
/// 5. 尾调用优化 (TCO) — 标记尾位置的 Call 为 TailCall
/// 6. 标记未使用变量（让 codegen 可以跳过它们的栈分配）
pub fn optimize_program(stmts: Vec<Stmt>) -> Vec<Stmt> {
    // Step 1: 表达式级优化（常量折叠 + 代数简化）
    let stmts: Vec<Stmt> = stmts.into_iter().map(opt_stmt).collect();

    // Step 2: 常量传播
    let consts = collect_const_props(&stmts);
    let stmts: Vec<Stmt> = stmts.into_iter().map(|s| propagate_stmt(s, &consts)).collect();

    // Step 3: 再次常量折叠（传播后可能产生新的可折叠表达式）
    let stmts: Vec<Stmt> = stmts.into_iter().map(opt_stmt).collect();

    // Step 4: 公共子表达式消除 (CSE)
    let stmts = cse_optimize(stmts);

    // Step 5: 尾调用优化 (TCO) — 在 CSE 之后，避免 CSE 干扰尾调用识别
    let stmts = tail_call_optimize(stmts);

    // Step 6: 标记未使用变量（让 codegen 可以跳过它们的栈分配）
    stmts
}

/// 分析程序中实际被引用的变量集合
pub fn get_used_variables(stmts: &[Stmt]) -> HashSet<String> {
    analyze_used_vars(stmts)
}

/// 尝试将表达式求值为 i64 常量（公开给 codegen 使用）
pub fn try_eval_const(e: &Expr) -> Option<i64> {
    eval_const(e)
}

// ============================================================================
// 公共子表达式消除 (Common Subexpression Elimination, CSE)
// ============================================================================
//
// 策略: 基本块级别的可用表达式分析。
// - 维护一张 表达式 → 变量名 的映射 (Vec 线性查找, 避免 Hash/Eq trait bound)
// - 当变量被修改时，失效所有引用该变量的表项
// - 遇到已计算过的纯表达式时，替换为对应的变量加载
// - 不跨越控制流边界 (while / for / if)，保证安全性

/// 判断表达式是否为"纯表达式"（无副作用，可安全 CSE）
fn is_cse_pure(e: &Expr) -> bool {
    match e {
        Expr::Integer(_) | Expr::Float(_) | Expr::String(_) |
        Expr::Bool(_) | Expr::Char(_) => true,
        Expr::Ident(_) => true,
        Expr::Binary(l, _, r) => is_cse_pure(l) && is_cse_pure(r),
        Expr::Unary(_, inner) => is_cse_pure(inner),
        _ => false, // Call, TailCall, StructLiteral, If, Match, Lambda, FieldAccess — 不纯
    }
}

/// 收集表达式中引用的所有变量名
fn expr_free_vars(e: &Expr) -> HashSet<String> {
    let mut vars = HashSet::new();
    fn walk(e: &Expr, out: &mut HashSet<String>) {
        match e {
            Expr::Ident(name) => { out.insert(name.clone()); }
            Expr::Binary(l, _, r) => { walk(l, out); walk(r, out); }
            Expr::Unary(_, inner) => walk(inner, out),
            _ => {}
        }
    }
    walk(e, &mut vars);
    vars
}

/// CSE 可用表达式表（基于 Vec 的线性查找）
struct CseEnv {
    /// (表达式, 存储结果的变量名)
    entries: Vec<(Expr, String)>,
}

impl CseEnv {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// 查找表达式是否已被计算并存储在某变量中
    fn lookup(&self, expr: &Expr) -> Option<&str> {
        if !is_cse_pure(expr) { return None; }
        self.entries.iter()
            .find(|(e, _)| e == expr)
            .map(|(_, var)| var.as_str())
    }

    /// 注册: 表达式 expr 的结果存放在变量 var 中
    fn define(&mut self, expr: &Expr, var: &str) {
        if is_cse_pure(expr) {
            self.entries.push((expr.clone(), var.to_string()));
        }
    }

    /// 当变量 name 被重新赋值时，失效所有引用 name 的表达式
    fn invalidate_var(&mut self, name: &str) {
        self.entries.retain(|(expr, var)| {
            !expr_free_vars(expr).contains(name) && var != name
        });
    }

    /// 清空表（控制流汇合点）
    fn clear(&mut self) {
        self.entries.clear();
    }
}

// ---------- 表达式级 CSE ----------

/// 对表达式应用 CSE：如果 expr 已在 CSE 表中，替换为变量加载；否则返回原表达式
fn cse_expr(e: Expr, env: &CseEnv) -> Expr {
    // 先尝试直接查找
    if let Some(var) = env.lookup(&e) {
        return Expr::Ident(var.to_string());
    }

    // 递归处理子表达式
    match e {
        Expr::Binary(l, op, r) => {
            let l = cse_expr(*l, env);
            let r = cse_expr(*r, env);
            let expr = Expr::Binary(Box::new(l.clone()), op.clone(), Box::new(r.clone()));
            // 对组合后的表达式再次查找
            if let Some(var) = env.lookup(&expr) {
                Expr::Ident(var.to_string())
            } else {
                expr
            }
        }
        Expr::Unary(uop, inner) => {
            let inner = cse_expr(*inner, env);
            let expr = Expr::Unary(uop, Box::new(inner.clone()));
            if let Some(var) = env.lookup(&expr) {
                Expr::Ident(var.to_string())
            } else {
                expr
            }
        }
        // 其他表达式类型不做 CSE 替换
        other => other,
    }
}

// ---------- 语句级 CSE ----------

/// 对基本块中的语句列表应用 CSE
fn cse_block(stmts: Vec<Stmt>, env: &mut CseEnv) -> Vec<Stmt> {
    let mut result = Vec::new();

    for stmt in stmts {
        let new_stmt = match stmt {
            // ─── Let 绑定 ───
            Stmt::Let { name, type_ann, mutable, value } => {
                // 先对 value 应用 CSE（可能替换子表达式）
                let value = cse_expr(value, env);

                // 如果 value 本身是一个 Ident 且不是递归引用自身
                let is_self_ref = matches!(&value, Expr::Ident(v) if v == &name);

                // 注册到 CSE 表（除非是自身引用）
                if !is_self_ref {
                    env.define(&value, &name);
                }

                Stmt::Let { name, type_ann, mutable, value }
            }

            // ─── 赋值 ───
            Stmt::Assign { name, value } => {
                // 赋值会修改 name，先失效
                env.invalidate_var(&name);
                let value = cse_expr(value, env);
                env.define(&value, &name);
                Stmt::Assign { name, value }
            }

            // ─── 字段赋值 ───
            Stmt::FieldAssign { obj, field, value } => {
                env.invalidate_var(&obj);
                let value = cse_expr(value, env);
                Stmt::FieldAssign { obj, field, value }
            }

            // ─── 表达式语句 ───
            Stmt::Expr(e) => {
                Stmt::Expr(cse_expr(e, env))
            }

            // ─── 返回 ───
            Stmt::Return(e) => {
                Stmt::Return(e.map(|v| cse_expr(v, env)))
            }

            // ─── Print / PrintLn / Exit ───
            Stmt::Print(e) => Stmt::Print(cse_expr(e, env)),
            Stmt::PrintLn(e) => Stmt::PrintLn(cse_expr(e, env)),
            Stmt::Exit(e) => Stmt::Exit(cse_expr(e, env)),

            // ─── While 循环 — 清空 CSE 表（循环体可能修改变量） ───
            Stmt::While(cond, body) => {
                let cond = cse_expr(cond, env);
                // 循环体在新的 CSE 环境中处理
                let mut body_env = CseEnv::new();
                let body = cse_block(body, &mut body_env);
                // 循环后的 CSE 状态不可靠（循环可能修改任意变量），清空
                env.clear();
                Stmt::While(cond, body)
            }

            // ─── For 循环 ───
            Stmt::For { var, iterable, body } => {
                let iterable = cse_expr(iterable, env);
                env.clear(); // 循环前清空
                let mut body_env = CseEnv::new();
                let body = cse_block(body, &mut body_env);
                env.clear(); // 循环后清空
                Stmt::For { var, iterable, body }
            }

            // ─── If 分支 — 两个分支各自独立的 CSE 环境 ───
            Stmt::If { cond, then_block, else_block } => {
                let cond = cse_expr(cond, env);
                // then 和 else 各自独立 CSE
                let mut then_env = CseEnv::new();
                let then_block = cse_block(then_block, &mut then_env);
                let else_block = else_block.map(|b| {
                    let mut else_env = CseEnv::new();
                    cse_block(b, &mut else_env)
                });
                // if 后的 CSE 不可靠（分支可能修改变量），清空
                env.clear();
                Stmt::If { cond, then_block, else_block }
            }

            // ─── Block ───
            Stmt::Block(stmts) => {
                Stmt::Block(cse_block(stmts, env))
            }

            // ─── 其他语句（FnDef, TypeDef, EnumDef, ImplBlock 等）不参与 CSE ───
            other => other,
        };

        result.push(new_stmt);
    }

    result
}

// ---------- 对函数体应用 CSE ----------

fn cse_fn_body(stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut env = CseEnv::new();
    cse_block(stmts, &mut env)
}

// ---------- 公开 CSE API ----------

/// 对整个程序的语句列表应用 CSE 优化
/// 仅处理顶层非函数定义的语句；函数体单独处理
pub fn cse_optimize(stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut result = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::FnDef { name, params, return_type, body } => {
                // 函数体在独立的 CSE 环境中处理
                let body = cse_fn_body(body);
                result.push(Stmt::FnDef { name, params, return_type, body });
            }
            Stmt::ImplBlock { type_name, methods } => {
                let methods = methods.into_iter().map(|m| {
                    if let Stmt::FnDef { name, params, return_type, body } = m {
                        let body = cse_fn_body(body);
                        Stmt::FnDef { name, params, return_type, body }
                    } else {
                        m
                    }
                }).collect();
                result.push(Stmt::ImplBlock { type_name, methods });
            }
            // 其他顶层语句暂存
            other => result.push(other),
        }
    }

    // 对非函数定义的顶层语句做一次整体 CSE
    let mut env = CseEnv::new();
    let result = cse_block(result, &mut env);
    result
}

// ============================================================================
// CSE 单元测试
// ============================================================================

#[cfg(test)]
mod cse_tests {
    use super::*;
    use crate::ast::*;

    /// 直接构造 AST，避免 parser 产生的噪声
    fn let_stmt(name: &str, value: Expr) -> Stmt {
        Stmt::Let { name: name.to_string(), type_ann: None, mutable: false, value }
    }

    fn assign_stmt(name: &str, value: Expr) -> Stmt {
        Stmt::Assign { name: name.to_string(), value }
    }

    fn bin(l: Expr, op: BinOp, r: Expr) -> Expr {
        Expr::Binary(Box::new(l), op, Box::new(r))
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident(name.to_string())
    }

    fn integer(n: i64) -> Expr {
        Expr::Integer(n)
    }

    // ---------- 基本消除 ----------

    #[test]
    fn test_cse_simple_dup() {
        // let a = x + y
        // let b = x + y   ← 应替换为 let b = a
        let stmts = vec![
            let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
            let_stmt("b", bin(ident("x"), BinOp::Add, ident("y"))),
        ];
        let result = cse_optimize(stmts);
        // a 应保持原样 (Binary)
        assert!(matches!(&result[0], Stmt::Let { name, value, .. }
            if name == "a" && matches!(value, Expr::Binary(_, BinOp::Add, _))));
        // b 的值应为 Ident("a")
        assert!(matches!(&result[1], Stmt::Let { name, value, .. }
            if name == "b" && matches!(value, Expr::Ident(v) if v == "a")));
    }

    #[test]
    fn test_cse_no_elim_after_mutation() {
        // let a = x + y
        // x = 10            ← 修改 x，应失效 x + y
        // let b = x + y     ← 不应替换
        let stmts = vec![
            let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
            assign_stmt("x", integer(10)),
            let_stmt("b", bin(ident("x"), BinOp::Add, ident("y"))),
        ];
        let result = cse_optimize(stmts);
        // b 的值应为 x + y (Binary), 不是 Ident("a")
        assert!(matches!(&result[2], Stmt::Let { name, value, .. }
            if name == "b" && matches!(value, Expr::Binary(_, BinOp::Add, _))));
    }

    #[test]
    fn test_cse_nested_expr() {
        // let a = (x + y) * 2
        // let b = (x + y) * 2  ← 应替换为 let b = a
        let expr = bin(bin(ident("x"), BinOp::Add, ident("y")), BinOp::Mul, integer(2));
        let stmts = vec![
            let_stmt("a", expr.clone()),
            let_stmt("b", expr.clone()),
        ];
        let result = cse_optimize(stmts);
        assert!(matches!(&result[1], Stmt::Let { name, value, .. }
            if name == "b" && matches!(value, Expr::Ident(v) if v == "a")));
    }

    // ---------- 控制流边界 ----------

    #[test]
    fn test_cse_no_elim_across_if() {
        // let a = x + y
        // if a > 0 { x = 1 }
        // let b = x + y  ← if 可能修改 x，CSE 不应生效
        let stmts = vec![
            let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
            Stmt::If {
                cond: bin(ident("a"), BinOp::Gt, integer(0)),
                then_block: vec![assign_stmt("x", integer(1))],
                else_block: None,
            },
            let_stmt("b", bin(ident("x"), BinOp::Add, ident("y"))),
        ];
        let result = cse_optimize(stmts);
        // b 的值应为 x + y (Binary)
        assert!(matches!(&result[2], Stmt::Let { name, value, .. }
            if name == "b" && matches!(value, Expr::Binary(_, BinOp::Add, _))));
    }

    #[test]
    fn test_cse_inside_if_branch() {
        // if 1 > 0 {
        //     let a = x + y
        //     let b = x + y   ← 同一分支内应消除
        // }
        let stmts = vec![Stmt::If {
            cond: bin(integer(1), BinOp::Gt, integer(0)),
            then_block: vec![
                let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
                let_stmt("b", bin(ident("x"), BinOp::Add, ident("y"))),
            ],
            else_block: None,
        }];
        let result = cse_optimize(stmts);
        if let Stmt::If { then_block, .. } = &result[0] {
            assert!(matches!(&then_block[1], Stmt::Let { name, value, .. }
                if name == "b" && matches!(value, Expr::Ident(v) if v == "a")));
        } else {
            panic!("expected If statement");
        }
    }

    // ---------- 不同表达式不消除 ----------

    #[test]
    fn test_cse_different_exprs() {
        // let a = x + y
        // let b = y + x  ← 不同结构，不消除
        let stmts = vec![
            let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
            let_stmt("b", bin(ident("y"), BinOp::Add, ident("x"))),
        ];
        let result = cse_optimize(stmts);
        // b 应保持 Binary
        if let Stmt::Let { value, .. } = &result[1] {
            assert!(matches!(value, Expr::Binary(_, BinOp::Add, _)));
            assert!(!matches!(value, Expr::Ident(_)));
        } else {
            panic!("expected Let statement");
        }
    }

    // ---------- 纯表达式判定 ----------

    #[test]
    fn test_cse_no_elim_for_calls() {
        // let a = foo(x)
        // let b = foo(x)  ← 函数调用不纯，不消除
        let stmts = vec![
            let_stmt("a", Expr::Call("foo".into(), vec![ident("x")])),
            let_stmt("b", Expr::Call("foo".into(), vec![ident("x")])),
        ];
        let result = cse_optimize(stmts);
        assert!(matches!(&result[1], Stmt::Let { name, value, .. }
            if name == "b" && matches!(value, Expr::Call(_, _))));
    }

    // ---------- 子表达式消除 ----------

    #[test]
    fn test_cse_sub_expr_elimination() {
        // let a = x + y
        // let b = a * 2   ← a 已替换 x + y，但 b 仍是 a * 2
        // let c = a * 2   ← 应替换为 let c = b
        let stmts = vec![
            let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
            let_stmt("b", bin(ident("a"), BinOp::Mul, integer(2))),
            let_stmt("c", bin(ident("a"), BinOp::Mul, integer(2))),
        ];
        let result = cse_optimize(stmts);
        // c 应为 Ident("b")
        assert!(matches!(&result[2], Stmt::Let { name, value, .. }
            if name == "c" && matches!(value, Expr::Ident(v) if v == "b")));
    }

    // ---------- 完整流水线测试 ----------

    #[test]
    fn test_cse_in_full_optimize_pipeline() {
        let stmts = vec![
            let_stmt("a", bin(ident("x"), BinOp::Add, ident("y"))),
            let_stmt("b", bin(ident("x"), BinOp::Add, ident("y"))),
            let_stmt("c", bin(ident("b"), BinOp::Mul, integer(2))),
            let_stmt("d", bin(ident("b"), BinOp::Mul, integer(2))),
        ];
        let optimized = optimize_program(stmts);
        // b 应为 Ident("a")
        assert!(matches!(&optimized[1], Stmt::Let { name, value, .. }
            if name == "b" && matches!(value, Expr::Ident(v) if v == "a")));
        // c 应为 Ident("b") — 因为 optimize_program 先做常量传播等，再做 CSE
        // 但 b 的值在 CSE 前已被替换为 Ident("a")
        // 所以 c = a * 2, d = c
        assert!(matches!(&optimized[3], Stmt::Let { name, value, .. }
            if name == "d" && matches!(value, Expr::Ident(v) if v == "c")));
    }

    #[test]
    fn test_cse_fn_body() {
        // 函数体内 CSE 应独立工作
        let stmts = vec![Stmt::FnDef {
            name: "add".into(),
            params: vec![
                Param { name: "a".into(), type_ann: Some("i32".into()) },
                Param { name: "b".into(), type_ann: Some("i32".into()) },
            ],
            return_type: Some("i32".into()),
            body: vec![
                let_stmt("c", bin(ident("a"), BinOp::Add, ident("b"))),
                let_stmt("d", bin(ident("a"), BinOp::Add, ident("b"))),
                Stmt::Return(Some(ident("d"))),
            ],
        }];
        let result = cse_optimize(stmts);
        if let Stmt::FnDef { body, .. } = &result[0] {
            // d 应为 Ident("c")
            assert!(matches!(&body[1], Stmt::Let { name, value, .. }
                if name == "d" && matches!(value, Expr::Ident(v) if v == "c")));
        } else {
            panic!("expected FnDef");
        }
    }
}

// ============================================================================
// 尾调用优化 (Tail Call Optimization, TCO)
// ============================================================================
//
// 策略: 遍历函数体，识别尾位置的函数调用，将 Call 转换为 TailCall。
// 代码生成层遇到 TailCall 时使用 jmp 替代 call，消除栈帧开销。
//
// 尾位置定义: 函数中最后执行的操作
// - 函数体的最后一条语句
// - return <expr> 中的 <expr>
// - if/else 的两个分支的最后一条语句（整体处于尾位置时）
// - block 的最后一条语句（block 处于尾位置时）
//
// 不视为尾位置:
// - while/for 循环体（循环可能不终止）
// - 非最后一条语句

/// 标记尾位置的表达式中的 Call 为 TailCall
fn mark_tail_expr(expr: &mut Expr) {
    if let Expr::Call(name, args) = expr {
        let name = std::mem::take(name);
        let args = std::mem::take(args);
        *expr = Expr::TailCall(name, args);
    }
    // TailCall 本身已经是尾调用，无需再处理
}

/// 标记语句列表（处于尾位置）的最后一条语句中的尾调用
fn mark_tail_in_block(stmts: &mut [Stmt]) {
    if let Some(last) = stmts.last_mut() {
        mark_tail_in_stmt(last);
    }
}

/// 递归标记语句中的尾调用
fn mark_tail_in_stmt(stmt: &mut Stmt) {
    match stmt {
        // return <expr> — expr 整体处于尾位置
        Stmt::Return(Some(expr)) => {
            mark_tail_expr(expr);
        }
        // 最后一条表达式语句 — 可能是尾调用（函数的隐式返回值）
        Stmt::Expr(expr) => {
            mark_tail_expr(expr);
        }
        // if/else — 两个分支各自独立处理
        Stmt::If { then_block, else_block, .. } => {
            mark_tail_in_block(then_block);
            if let Some(else_block) = else_block {
                mark_tail_in_block(else_block);
            }
        }
        // block — 内部最后一条语句可能在尾位置
        Stmt::Block(stmts) => {
            mark_tail_in_block(stmts);
        }
        // while/for — 循环体不在尾位置（循环可能不终止），不处理
        _ => {}
    }
}

/// 对函数体应用尾调用标记
fn tco_fn_body(stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut stmts = stmts;
    mark_tail_in_block(&mut stmts);
    stmts
}

/// 对整个程序应用尾调用优化
pub fn tail_call_optimize(stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut result = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::FnDef { name, params, return_type, body } => {
                let body = tco_fn_body(body);
                result.push(Stmt::FnDef { name, params, return_type, body });
            }
            Stmt::ImplBlock { type_name, methods } => {
                let methods = methods.into_iter().map(|m| {
                    if let Stmt::FnDef { name, params, return_type, body } = m {
                        let body = tco_fn_body(body);
                        Stmt::FnDef { name, params, return_type, body }
                    } else {
                        m
                    }
                }).collect();
                result.push(Stmt::ImplBlock { type_name, methods });
            }
            // 其他顶层语句不处理（主函数入口不需要 TCO）
            other => result.push(other),
        }
    }

    result
}

// ============================================================================
// TCO 单元测试
// ============================================================================

#[cfg(test)]
mod tco_tests {
    use super::*;
    use crate::ast::*;

    fn param(name: &str, ty: &str) -> crate::ast::Param {
        crate::ast::Param { name: name.to_string(), type_ann: Some(ty.to_string()) }
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident(name.to_string())
    }

    fn integer(n: i64) -> Expr {
        Expr::Integer(n)
    }

    fn bin(l: Expr, op: BinOp, r: Expr) -> Expr {
        Expr::Binary(Box::new(l), op, Box::new(r))
    }

    fn call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call(name.to_string(), args)
    }

    fn ret(e: Expr) -> Stmt {
        Stmt::Return(Some(e))
    }

    // ---------- 测试 1: 尾递归阶乘 ----------

    #[test]
    fn test_tco_tail_recursive_factorial() {
        // fn fact(n, acc) {
        //     if n <= 1 { return acc }
        //     else { return fact(n - 1, n * acc) }
        // }
        let stmts = vec![Stmt::FnDef {
            name: "fact".into(),
            params: vec![param("n", "i64"), param("acc", "i64")],
            return_type: Some("i64".into()),
            body: vec![
                Stmt::If {
                    cond: bin(ident("n"), BinOp::Lte, integer(1)),
                    then_block: vec![ret(ident("acc"))],
                    else_block: Some(vec![
                        ret(call("fact", vec![
                            bin(ident("n"), BinOp::Sub, integer(1)),
                            bin(ident("n"), BinOp::Mul, ident("acc")),
                        ])),
                    ]),
                },
            ],
        }];

        let result = tail_call_optimize(stmts);

        // 验证: then 分支的 return acc → 不是 TailCall（acc 是 Ident）
        if let Stmt::FnDef { body, .. } = &result[0] {
            if let Stmt::If { then_block, else_block, .. } = &body[0] {
                // then: return acc → acc 不是 Call，保持 Ident
                assert!(matches!(&then_block[0], Stmt::Return(Some(Expr::Ident(v))) if v == "acc"));

                // else: return fact(n-1, n*acc) → 应变成 TailCall
                if let Some(else_block) = else_block {
                    assert!(matches!(&else_block[0], Stmt::Return(Some(Expr::TailCall(name, args)))
                        if name == "fact" && args.len() == 2));
                } else {
                    panic!("expected else_block");
                }
            } else {
                panic!("expected If");
            }
        } else {
            panic!("expected FnDef");
        }
    }

    // ---------- 测试 2: 普通尾调用（非递归） ----------

    #[test]
    fn test_tco_normal_tail_call() {
        // fn helper() {
        //     return foo(1, 2)
        // }
        let stmts = vec![Stmt::FnDef {
            name: "helper".into(),
            params: vec![],
            return_type: None,
            body: vec![
                ret(call("foo", vec![integer(1), integer(2)])),
            ],
        }];

        let result = tail_call_optimize(stmts);

        if let Stmt::FnDef { body, .. } = &result[0] {
            // return foo(1, 2) → 应变成 TailCall
            assert!(matches!(&body[0], Stmt::Return(Some(Expr::TailCall(name, args)))
                if name == "foo" && args.len() == 2
                && args[0] == Expr::Integer(1) && args[1] == Expr::Integer(2)));
        } else {
            panic!("expected FnDef");
        }
    }

    // ---------- 测试 3: 非尾调用不优化 ----------

    #[test]
    fn test_tco_non_tail_call_not_optimized() {
        // fn non_tail() {
        //     let x = foo(1)       ← 不是尾位置
        //     return bar(x + 1)    ← bar 是尾调用
        // }
        let stmts = vec![Stmt::FnDef {
            name: "non_tail".into(),
            params: vec![],
            return_type: None,
            body: vec![
                Stmt::Let {
                    name: "x".into(),
                    type_ann: None,
                    mutable: false,
                    value: call("foo", vec![integer(1)]),
                },
                ret(call("bar", vec![bin(ident("x"), BinOp::Add, integer(1))])),
            ],
        }];

        let result = tail_call_optimize(stmts);

        if let Stmt::FnDef { body, .. } = &result[0] {
            // let x = foo(1) → foo 不是尾位置，应保持 Call
            assert!(matches!(&body[0], Stmt::Let { name, value, .. }
                if name == "x" && matches!(value, Expr::Call(name, _) if name == "foo")));

            // return bar(x + 1) → bar 是尾位置，应变 TailCall
            assert!(matches!(&body[1], Stmt::Return(Some(Expr::TailCall(name, args)))
                if name == "bar" && args.len() == 1));
        } else {
            panic!("expected FnDef");
        }
    }

    // ---------- 测试 4: 两个分支都有尾调用 ----------

    #[test]
    fn test_tco_both_branches_tail_call() {
        // fn choose(flag) {
        //     if flag { return a() } else { return b() }
        // }
        let stmts = vec![Stmt::FnDef {
            name: "choose".into(),
            params: vec![param("flag", "bool")],
            return_type: None,
            body: vec![
                Stmt::If {
                    cond: ident("flag"),
                    then_block: vec![ret(call("a", vec![]))],
                    else_block: Some(vec![ret(call("b", vec![]))]),
                },
            ],
        }];

        let result = tail_call_optimize(stmts);

        if let Stmt::FnDef { body, .. } = &result[0] {
            if let Stmt::If { then_block, else_block, .. } = &body[0] {
                // then: return a() → TailCall
                assert!(matches!(&then_block[0], Stmt::Return(Some(Expr::TailCall(n, _))) if n == "a"));
                // else: return b() → TailCall
                if let Some(else_block) = else_block {
                    assert!(matches!(&else_block[0], Stmt::Return(Some(Expr::TailCall(n, _))) if n == "b"));
                } else {
                    panic!("expected else_block");
                }
            } else {
                panic!("expected If");
            }
        } else {
            panic!("expected FnDef");
        }
    }

    // ---------- 测试 5: while 循环中的调用不是尾调用 ----------

    #[test]
    fn test_tco_while_loop_not_tail() {
        // fn loop_fn() {
        //     while true { call foo() }
        //     return bar()
        // }
        let stmts = vec![Stmt::FnDef {
            name: "loop_fn".into(),
            params: vec![],
            return_type: None,
            body: vec![
                Stmt::While(
                    Expr::Bool(true),
                    vec![Stmt::Expr(call("foo", vec![]))],
                ),
                ret(call("bar", vec![])),
            ],
        }];

        let result = tail_call_optimize(stmts);

        if let Stmt::FnDef { body, .. } = &result[0] {
            // while 中的 foo() 不是尾调用
            if let Stmt::While(_, while_body) = &body[0] {
                assert!(matches!(&while_body[0], Stmt::Expr(Expr::Call(n, _)) if n == "foo"));
            }
            // return bar() 是尾调用
            assert!(matches!(&body[1], Stmt::Return(Some(Expr::TailCall(n, _))) if n == "bar"));
        } else {
            panic!("expected FnDef");
        }
    }

    // ---------- 测试 6: ImplBlock 中的方法 TCO ----------

    #[test]
    fn test_tco_in_impl_block() {
        // impl Foo {
        //     fn self_call(x) { return self_call(x) }
        // }
        let stmts = vec![Stmt::ImplBlock {
            type_name: "Foo".into(),
            methods: vec![Stmt::FnDef {
                name: "self_call".into(),
                params: vec![param("x", "i64")],
                return_type: None,
                body: vec![ret(call("self_call", vec![ident("x")]))],
            }],
        }];

        let result = tail_call_optimize(stmts);

        if let Stmt::ImplBlock { methods, .. } = &result[0] {
            if let Stmt::FnDef { body, .. } = &methods[0] {
                assert!(matches!(&body[0], Stmt::Return(Some(Expr::TailCall(n, args)))
                    if n == "self_call" && args.len() == 1));
            } else {
                panic!("expected FnDef in ImplBlock");
            }
        } else {
            panic!("expected ImplBlock");
        }
    }

    // ---------- 测试 7: 完整流水线中的 TCO ----------

    #[test]
    fn test_tco_in_full_optimize_pipeline() {
        // fn add(a, b) { return a + b }
        // fn main() { return add(1, 2) }
        let stmts = vec![
            Stmt::FnDef {
                name: "add".into(),
                params: vec![param("a", "i64"), param("b", "i64")],
                return_type: Some("i64".into()),
                body: vec![ret(bin(ident("a"), BinOp::Add, ident("b")))],
            },
            Stmt::FnDef {
                name: "main".into(),
                params: vec![],
                return_type: Some("i64".into()),
                body: vec![ret(call("add", vec![integer(1), integer(2)]))],
            },
        ];

        let optimized = optimize_program(stmts);

        // main 中的 return add(1, 2) → 应被标记为 TailCall
        if let Stmt::FnDef { name, body, .. } = &optimized[1] {
            assert_eq!(name, "main");
            assert!(matches!(&body[0], Stmt::Return(Some(Expr::TailCall(n, args)))
                if n == "add" && args.len() == 2));
        } else {
            panic!("expected FnDef");
        }
    }
}

