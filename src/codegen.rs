//! KLC 字节码生成器 — AST → Bytecode

use crate::ast::*;
use crate::bytecode::*;

pub struct Codegen {
    constants: Vec<Value>,
    next_label: usize,
    lambda_count: usize,
    lambdas: Vec<CompiledFunction>,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            next_label: 0,
            lambda_count: 0,
            lambdas: Vec::new(),
        }
    }

    fn new_label(&mut self) -> usize {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    fn add_constant(&mut self, value: Value) -> usize {
        // 去重
        if let Some(pos) = self.constants.iter().position(|c| c == &value) {
            return pos;
        }
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// 编译整个程序
    pub fn compile(program: &Program) -> Result<BytecodeProgram, String> {
        let mut cg = Self::new();
        let mut main_instructions = Vec::new();
        let mut functions = Vec::new();

        for stmt in &program.statements {
            match stmt {
                Stmt::FnDef { name, params, body, .. } => {
                    let compiled = cg.compile_function(name, params, body)?;
                    functions.push(compiled);
                }
                Stmt::ImplBlock { type_name, methods } => {
                    for method in methods {
                        if let Stmt::FnDef { name, params, body, .. } = method {
                            // 编译方法三次: 简名(实例方法调用) + Type::method + Type.method
                            let compiled = cg.compile_function(name, params, body)?;
                            functions.push(compiled);
                            // 注册 Type::method 名称（关联函数写法1）
                            let prefixed = format!("{}::{}", type_name, name);
                            let compiled2 = cg.compile_function(&prefixed, params, body)?;
                            functions.push(compiled2);
                            // 注册 Type.method 名称（关联函数写法2）— v1.0.3-正式版 双写法兼容
                            let dotted = format!("{}.{}", type_name, name);
                            let compiled3 = cg.compile_function(&dotted, params, body)?;
                            functions.push(compiled3);
                        }
                    }
                }
                Stmt::TypeDef { .. } | Stmt::EnumDef { .. } => { /* no-op */ }
                _ => {
                    cg.compile_stmt(stmt, &mut main_instructions)?;
                }
            }
        }

        // 如果有 fn main() 定义，在程序末尾调用它
        let has_main = functions.iter().any(|f| f.name == "main" && f.param_count == 0);
        if has_main {
            main_instructions.push(Instruction::Call("main".into(), 0));
            main_instructions.push(Instruction::Pop); // 丢弃 main 返回值
        }

        main_instructions.push(Instruction::Halt);

        // 追加 lambda 函数
        functions.extend(cg.lambdas);

        Ok(BytecodeProgram {
            main: main_instructions,
            functions,
            constants: cg.constants,
        })
    }

    fn compile_function(
        &mut self,
        name: &str,
        _params: &[Param],
        body: &[Stmt],
    ) -> Result<CompiledFunction, String> {
        let mut instructions = Vec::new();

        // 参数绑定由 VM 在 Call 指令中直接完成，这里不再生成 Store 指令

        for stmt in body {
            self.compile_stmt(stmt, &mut instructions)?;
        }

        // 确保函数以 Return 结束
        if !instructions.is_empty() {
            if !matches!(instructions.last(), Some(Instruction::Return)) {
                instructions.push(Instruction::Return);
            }
        } else {
            instructions.push(Instruction::Return);
        }

        Ok(CompiledFunction {
            name: name.to_string(),
            instructions,
            param_count: _params.len(),
            param_names: _params.iter().map(|p| p.name.clone()).collect(),
        })
    }

    fn compile_stmt(&mut self, stmt: &Stmt, code: &mut Vec<Instruction>) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value, code)?;
                // 如果是 Lambda，记录别名以便调用
                if let Expr::Lambda { .. } = value {
                    let lname = format!("__lambda_{}", self.lambda_count - 1);
                    code.push(Instruction::RegFn(name.clone(), lname));
                }
                code.push(Instruction::InitVar(name.clone()));
            }
            Stmt::Assign { name, value } => {
                self.compile_expr(value, code)?;
                code.push(Instruction::Store(name.clone()));
            }
            Stmt::FieldAssign { obj, field, value } => {
                // obj.field = value
                // 1. 加载 obj (struct 在栈底)
                code.push(Instruction::Load(obj.clone()));
                // 2. 计算新值压栈 (value 在栈顶)
                self.compile_expr(value, code)?;
                // 3. StructSet(field) — pop value(顶) + struct(底), push modified struct
                code.push(Instruction::StructSet(field.clone()));
                // 4. 存回 obj
                code.push(Instruction::Store(obj.clone()));
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr, code)?;
                code.push(Instruction::Pop); // 丢弃表达式值
            }
            Stmt::Return(expr) => {
                match expr {
                    Some(e) => self.compile_expr(e, code)?,
                    None => {
                        let null_idx = self.add_constant(Value::Null);
                        code.push(Instruction::Const(null_idx));
                    }
                }
                code.push(Instruction::Return);
            }
            Stmt::While(cond, body) => {
                let loop_start = code.len();
                self.compile_expr(cond, code)?;
                let _exit_label = self.new_label();
                code.push(Instruction::JmpFalse(0)); // 占位
                let jmp_pos = code.len() - 1;

                for s in body {
                    self.compile_stmt(s, code)?;
                }
                code.push(Instruction::Jmp(loop_start));

                let exit_pos = code.len();
                // 回填跳转地址
                code[jmp_pos] = Instruction::JmpFalse(exit_pos);
            }
            Stmt::For { var, iterable, body } => {
                // for var in iterable { body }
                // v1.0.3-正式版 增强: 支持 range start..end、数组遍历
                self.compile_expr(iterable, code)?;
                // iterable 求值后栈顶为 range/array 值
                // 使用 __for_iter / __for_next 迭代器模式
                code.push(Instruction::Call("__for_iter".into(), 1));

                // 迭代器 key 在栈顶
                let iter_key_var = format!("__for_iter_key_{}", var);
                code.push(Instruction::InitVar(iter_key_var.clone()));

                let loop_start = code.len();
                // __for_next(iter_key) → bool, val (或 key, val for map)
                code.push(Instruction::Load(iter_key_var.clone()));
                code.push(Instruction::Call("__for_next".into(), 1));
                // 栈顶: has_more(bool), val
                let _exit_label = self.new_label();
                code.push(Instruction::JmpFalse(0));
                let jmp_pos = code.len() - 1;

                // 绑定当前值到 var
                code.push(Instruction::InitVar(var.clone()));

                for s in body {
                    self.compile_stmt(s, code)?;
                }

                code.push(Instruction::Jmp(loop_start));
                let exit_pos = code.len();
                code[jmp_pos] = Instruction::JmpFalse(exit_pos);
                // 弹出循环结束后栈上残留的值
                code.push(Instruction::Pop);
            }
            Stmt::If { cond, then_block, else_block } => {
                self.compile_expr(cond, code)?;
                let _else_label = self.new_label();
                code.push(Instruction::JmpFalse(0));
                let jmp_else_pos = code.len() - 1;

                for s in then_block {
                    self.compile_stmt(s, code)?;
                }

                if let Some(else_stmts) = else_block {
                    let _end_label = self.new_label();
                    code.push(Instruction::Jmp(0));
                    let jmp_end_pos = code.len() - 1;

                    let else_pos = code.len();
                    code[jmp_else_pos] = Instruction::JmpFalse(else_pos);

                    for s in else_stmts {
                        self.compile_stmt(s, code)?;
                    }

                    let end_pos = code.len();
                    code[jmp_end_pos] = Instruction::Jmp(end_pos);
                } else {
                    let else_pos = code.len();
                    code[jmp_else_pos] = Instruction::JmpFalse(else_pos);
                }
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.compile_stmt(s, code)?;
                }
            }
            Stmt::Print(expr) | Stmt::PrintLn(expr) => {
                self.compile_expr(expr, code)?;
                if matches!(stmt, Stmt::PrintLn(_)) {
                    code.push(Instruction::PrintLn);
                } else {
                    code.push(Instruction::Print);
                }
            }
            Stmt::Exit(expr) => {
                self.compile_expr(expr, code)?;
                code.push(Instruction::Pop); // 丢弃退出码
                code.push(Instruction::Halt);
            }
            Stmt::FnDef { .. } => {
                // 函数定义在顶层处理，这里不应该遇到
            }
            Stmt::TypeDef { .. } | Stmt::EnumDef { .. } => {
                // 定义不需要字节码
            }
            Stmt::ImplBlock { .. } => {
                // impl 块在顶层处理
            }
            Stmt::Break | Stmt::Continue => {
                // 阶段五: 原生代码生成器处理
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr, code: &mut Vec<Instruction>) -> Result<(), String> {
        match expr {
            Expr::Integer(n) => {
                let idx = self.add_constant(Value::Integer(*n));
                code.push(Instruction::Const(idx));
            }
            Expr::Float(f) => {
                let idx = self.add_constant(Value::Float(*f));
                code.push(Instruction::Const(idx));
            }
            Expr::String(s) => {
                let idx = self.add_constant(Value::String(s.clone()));
                code.push(Instruction::Const(idx));
            }
            Expr::Bool(b) => {
                let idx = self.add_constant(Value::Bool(*b));
                code.push(Instruction::Const(idx));
            }
            Expr::Char(c) => {
                let idx = self.add_constant(Value::Char(*c));
                code.push(Instruction::Const(idx));
            }
            Expr::Ident(name) => {
                code.push(Instruction::Load(name.clone()));
            }
            Expr::Null => {
                let idx = self.add_constant(Value::Null);
                code.push(Instruction::Const(idx));
            }
            Expr::Binary(left, op, right) => {
                if matches!(op, BinOp::Range | BinOp::RangeInclusive) {
                    // 范围运算暂未实现 → push Null 代替
                    let null_idx = self.add_constant(Value::Null);
                    code.push(Instruction::Const(null_idx));
                } else {
                    self.compile_expr(left, code)?;
                    self.compile_expr(right, code)?;
                    let instr = match op {
                        BinOp::Add => Instruction::Add,
                        BinOp::Sub => Instruction::Sub,
                        BinOp::Mul => Instruction::Mul,
                        BinOp::Div => Instruction::Div,
                        BinOp::Mod => Instruction::Mod,
                        BinOp::Eq => Instruction::Eq,
                        BinOp::Neq => Instruction::Neq,
                        BinOp::Lt => Instruction::Lt,
                        BinOp::Gt => Instruction::Gt,
                        BinOp::Lte => Instruction::Lte,
                        BinOp::Gte => Instruction::Gte,
                        BinOp::And => Instruction::And,
                        BinOp::Or => Instruction::Or,
                        BinOp::Concat => Instruction::Concat,
                        _ => unreachable!(),
                    };
                    code.push(instr);
                }
            }
            Expr::Unary(op, expr) => {
                self.compile_expr(expr, code)?;
                match op {
                    UnaryOp::Neg => code.push(Instruction::Neg),
                    UnaryOp::Not => code.push(Instruction::Not),
                }
            }
            Expr::Call(name, args) => {
                if name == "to_str" && args.len() == 1 {
                    self.compile_expr(&args[0], code)?;
                    code.push(Instruction::ToString);
                } else if name == "read_line" && args.is_empty() {
                    code.push(Instruction::ReadLine);
                } else if name == "Ok" && args.len() == 1 {
                    // Result::Ok(value) 构造器 — 编译为 EnumNew
                    self.compile_expr(&args[0], code)?;
                    code.push(Instruction::EnumNew("Result".into(), "Ok".into(), 1));
                } else if name == "Err" && args.len() == 1 {
                    // Result::Err(msg) 构造器 — 编译为 EnumNew
                    self.compile_expr(&args[0], code)?;
                    code.push(Instruction::EnumNew("Result".into(), "Err".into(), 1));
                } else if name == "Some" && args.len() == 1 {
                    // Option::Some(value) 构造器 — 编译为 EnumNew
                    self.compile_expr(&args[0], code)?;
                    code.push(Instruction::EnumNew("Option".into(), "Some".into(), 1));
                } else if name == "None" && args.is_empty() {
                    // Option::None 构造器 — 编译为 EnumNew
                    code.push(Instruction::EnumNew("Option".into(), "None".into(), 0));
                } else {
                    for arg in args {
                        self.compile_expr(arg, code)?;
                    }
                    code.push(Instruction::Call(name.clone(), args.len()));
                }
            }
            Expr::Lambda { params, body, .. } => {
                let name = format!("__lambda_{}", self.lambda_count);
                self.lambda_count += 1;
                let compiled = CompiledFunction {
                    name: name.clone(),
                    instructions: {
                        let mut code = Vec::new();
                        for s in body { self.compile_stmt(s, &mut code)?; }
                        if !matches!(code.last(), Some(Instruction::Return)) {
                            code.push(Instruction::Return);
                        }
                        code
                    },
                    param_count: params.len(),
                    param_names: params.iter().map(|p| p.name.clone()).collect(),
                };
                self.lambdas.push(compiled);
                // 常量是 Function(name) + RegFn 指令
                let fn_idx = self.add_constant(Value::Function(name.clone()));
                code.push(Instruction::Const(fn_idx));
                // 但不设置别名——由 InitVar 后的 RegFn 手动处理
            }
            Expr::FieldAccess(base, field) => {
                // 字段访问: base.field → 加载 base，StructGet(field)
                self.compile_expr(base, code)?;
                code.push(Instruction::StructGet(field.clone()));
            }
            Expr::StructLiteral { type_name, fields } => {
                // 结构体字面量: name1, val1, name2, val2, ... → StructNew
                for (field_name, field_value) in fields {
                    // 压入字段名常量
                    let name_idx = self.add_constant(Value::String(field_name.clone()));
                    code.push(Instruction::Const(name_idx));
                    // 压入字段值
                    self.compile_expr(field_value, code)?;
                }
                code.push(Instruction::StructNew(type_name.clone(), fields.len()));
            }
            Expr::If(cond, then_expr, else_expr) => {
                self.compile_expr(cond, code)?;
                let _else_label = self.new_label();
                code.push(Instruction::JmpFalse(0));
                let jmp_else_pos = code.len() - 1;

                self.compile_expr(then_expr, code)?;

                if let Some(else_e) = else_expr {
                    let _end_label = self.new_label();
                    code.push(Instruction::Jmp(0));
                    let jmp_end_pos = code.len() - 1;

                    let else_pos = code.len();
                    code[jmp_else_pos] = Instruction::JmpFalse(else_pos);
                    self.compile_expr(else_e, code)?;

                    let end_pos = code.len();
                    code[jmp_end_pos] = Instruction::Jmp(end_pos);
                } else {
                    // 无 else，push Null
                    let else_pos = code.len();
                    code[jmp_else_pos] = Instruction::JmpFalse(else_pos);
                    let null_idx = self.add_constant(Value::Null);
                    code.push(Instruction::Const(null_idx));
                }
            }
            Expr::Match { value, arms } => {
                // 编译 match: 求值一次 → 逐臂比较 → if-else 链
                let end_label = self.new_label();
                let tmp_var = format!("__match_{}", end_label);
                self.compile_expr(value, code)?;
                code.push(Instruction::InitVar(tmp_var.clone()));

                let mut arm_starts: Vec<usize> = Vec::new();
                let mut fail_jmps: Vec<Vec<usize>> = Vec::new(); // 每个 arm 的失败跳转
                let mut end_jmps: Vec<usize> = Vec::new();       // 跳到 match 末尾

                for arm in arms {
                    arm_starts.push(code.len());
                    let mut arm_fails = Vec::new();

                    // 编译模式匹配条件 → 栈顶 bool
                    self.compile_pattern_check(&tmp_var, &arm.pattern, code, arm.bind.as_deref())?;
                    code.push(Instruction::JmpFalse(0));
                    arm_fails.push(code.len() - 1);

                    // 编译 guard
                    if let Some(guard) = &arm.guard {
                        self.compile_expr(guard, code)?;
                        code.push(Instruction::JmpFalse(0));
                        arm_fails.push(code.len() - 1);
                    }

                    // 构造器模式：匹配成功后绑定/解包变量
                    // 枚举变体如 Some(val), Ok(val), Err(msg) → 提取字段数据
                    if let Some(bind_name) = &arm.bind {
                        if let MatchPattern::Variable(name) = &arm.pattern {
                            let is_variant = name.chars().next().map_or(false, |c| c.is_uppercase());
                            if is_variant {
                                code.push(Instruction::Load(tmp_var.clone()));
                                if name == "None" {
                                    // None 变体没有数据字段，直接绑定枚举值本身
                                    code.push(Instruction::InitVar(bind_name.clone()));
                                } else {
                                    // 带数据的变体 (Some, Ok, Err, 自定义变体等): 提取第一个字段
                                    code.push(Instruction::EnumGet(0));
                                    code.push(Instruction::InitVar(bind_name.clone()));
                                }
                            }
                        }
                    }

                    // 编译 body（语句列表）
                    let has_last_expr = matches!(arm.body.last(), Some(Stmt::Expr(_)));
                    for (i, s) in arm.body.iter().enumerate() {
                        let is_last = i == arm.body.len() - 1;
                        if is_last && matches!(s, Stmt::Expr(_)) {
                            // 最后一条表达式：编译但不 pop，保留值
                            if let Stmt::Expr(expr) = s {
                                self.compile_expr(expr, code)?;
                            }
                        } else {
                            self.compile_stmt(s, code)?;
                        }
                    }
                    if !has_last_expr {
                        let null_idx = self.add_constant(Value::Null);
                        code.push(Instruction::Const(null_idx));
                    }
                    code.push(Instruction::Jmp(0));
                    end_jmps.push(code.len() - 1);

                    fail_jmps.push(arm_fails);
                }

                // 无臂匹配的兜底：push Null
                let null_fallback = code.len();
                let null_idx = self.add_constant(Value::Null);
                code.push(Instruction::Const(null_idx));
                let end_pos = code.len();
                // 回填 body 的 Jmp → end_pos（跳过 Null 兜底）
                for pos in &end_jmps {
                    code[*pos] = Instruction::Jmp(end_pos);
                }
                // 回填失败跳转：arm i 失败 → arm i+1 起始（最后 arm → null_fallback）
                for i in 0..arms.len() {
                    let target = if i + 1 < arms.len() { arm_starts[i + 1] } else { null_fallback };
                    for &pos in &fail_jmps[i] {
                        code[pos] = Instruction::JmpFalse(target);
                    }
                }
            }
            Expr::TailCall(name, args) => {
                // 字节码 VM 不支持 TCO，回退为普通函数调用
                for arg in args {
                    self.compile_expr(arg, code)?;
                }
                code.push(Instruction::Call(name.clone(), args.len()));
            }
            Expr::EnumConstructor { type_name, variant, args } => {
                // 编译枚举构造器: Enum::Variant(arg1, arg2, ...) → EnumNew 指令
                // 栈: arg1, arg2, ... → Enum { type, variant, [arg1, arg2, ...] }
                for arg in args {
                    self.compile_expr(arg, code)?;
                }
                code.push(Instruction::EnumNew(type_name.clone(), variant.clone(), args.len()));
            }
            Expr::ResultOk(expr) => {
                // Ok(value) → Enum { "Result", "Ok", [value] }
                self.compile_expr(expr, code)?;
                code.push(Instruction::EnumNew("Result".into(), "Ok".into(), 1));
            }
            Expr::ResultErr(expr) => {
                // Err(msg) → Enum { "Result", "Err", [msg] }
                self.compile_expr(expr, code)?;
                code.push(Instruction::EnumNew("Result".into(), "Err".into(), 1));
            }
            Expr::Try(expr) => {
                // try expr / expr? → 如果 expr 是 Err，提前返回 Err
                // 编译为：求值 expr，如果是 Err 变体则直接 return，否则提取 Ok 中的值
                let end_label = self.new_label();
                let tmp_var = format!("__try_{}", end_label);
                self.compile_expr(expr, code)?;
                code.push(Instruction::InitVar(tmp_var.clone()));
                // 检查是否为 Err 变体
                code.push(Instruction::Load(tmp_var.clone()));
                code.push(Instruction::IsVariant("Err".into()));
                // 如果是 Err → 提前返回
                code.push(Instruction::JmpFalse(0)); // 不是 Err 就跳到正常路径
                let jmp_ok_pos = code.len() - 1;
                // 是 Err → 返回该 Err 值
                code.push(Instruction::Load(tmp_var.clone()));
                code.push(Instruction::Return);
                // 不是 Err → 提取 Ok 中的值压栈
                let ok_pos = code.len();
                code[jmp_ok_pos] = Instruction::JmpFalse(ok_pos);
                code.push(Instruction::Load(tmp_var.clone()));
                code.push(Instruction::EnumGet(0)); // 提取 Ok(value) 中的 value
            }
            Expr::GoSpawn(expr) => {
                // go func(args) → 编译参数 + Spawn 指令（异步线程池派发）
                match expr.as_ref() {
                    Expr::Call(name, args) => {
                        for arg in args {
                            self.compile_expr(arg, code)?;
                        }
                        code.push(Instruction::Spawn(name.clone(), args.len()));
                    }
                    _ => {
                        self.compile_expr(expr, code)?;
                        code.push(Instruction::Pop);
                    }
                }
            }
        }
        Ok(())
    }

    /// 编译模式匹配条件：生成代码检查 match 值是否匹配模式，结果在栈顶(bool)
    fn compile_pattern_check(&mut self, val_var: &str, pattern: &MatchPattern, code: &mut Vec<Instruction>, bind: Option<&str>) -> Result<(), String> {
        match pattern {
            MatchPattern::Literal(expr) => {
                code.push(Instruction::Load(val_var.to_string()));
                self.compile_expr(expr, code)?;
                code.push(Instruction::Eq);
            }
            MatchPattern::Variable(name) => {
                // 判断是否为枚举变体模式（以大写字母开头的名称）
                // 如 Some, None, Ok, Err, Red, Green, Blue 等
                let is_variant = name.chars().next().map_or(false, |c| c.is_uppercase());
                if is_variant {
                    // 枚举变体匹配: 使用 IsVariant 检查
                    code.push(Instruction::Load(val_var.to_string()));
                    code.push(Instruction::IsVariant(name.clone()));
                } else if name == "_" {
                    // 通配符模式: 始终匹配
                    let true_idx = self.add_constant(Value::Bool(true));
                    code.push(Instruction::Const(true_idx));
                } else {
                    // 普通变量: 始终匹配, 绑定值到变量名（catch-all）
                    let bind_name = bind.unwrap_or(name);
                    code.push(Instruction::Load(val_var.to_string()));
                    code.push(Instruction::InitVar(bind_name.to_string()));
                    let true_idx = self.add_constant(Value::Bool(true));
                    code.push(Instruction::Const(true_idx));
                }
            }
            MatchPattern::Or(patterns) => {
                if patterns.is_empty() {
                    let false_idx = self.add_constant(Value::Bool(false));
                    code.push(Instruction::Const(false_idx));
                    return Ok(());
                }
                // a | b | c → val == a or val == b or val == c
                for (i, pat) in patterns.iter().enumerate() {
                    code.push(Instruction::Load(val_var.to_string()));
                    match pat {
                        MatchPattern::Literal(expr) => self.compile_expr(expr, code)?,
                        _ => return Err("| 模式中仅支持字面量模式".into()),
                    }
                    code.push(Instruction::Eq);
                    if i > 0 {
                        code.push(Instruction::Or);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn compile_source(source: &str) -> Result<BytecodeProgram, String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program()?;
        Codegen::compile(&program)
    }

    #[test]
    fn test_simple_arithmetic() {
        let result = compile_source("let x = 1 + 2 * 3").unwrap();
        assert!(result.main.len() > 0);
        assert!(result.constants.len() > 0);
    }

    #[test]
    fn test_if_statement() {
        let result = compile_source("let x = 10\nif x > 5 { io.println(x) }").unwrap();
        assert!(result.main.len() > 0);
    }

    #[test]
    fn test_function() {
        let result = compile_source("fn add(a: i32, b: i32) -> i32 { return a + b }").unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "add");
    }
}
