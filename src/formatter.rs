//! KLC 代码格式化器 — AST 驱动的代码美化工具
//!
//! `klc fmt` 子命令的核心实现。通过解析 AST 后重新生成代码，
//! 实现统一的代码风格：自动缩进、空格、换行、对齐。

use crate::ast::*;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// 格式化配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FormatConfig {
    /// 缩进宽度（空格数）
    pub indent_width: usize,
    /// 最大行宽（超过则尝试拆分）
    pub max_line_width: usize,
    /// 函数左大括号是否另起一行
    pub fn_brace_newline: bool,
    /// 运算符两侧空格
    pub spaces_around_operators: bool,
    /// 逗号后空格
    pub space_after_comma: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        FormatConfig {
            indent_width: 4,
            max_line_width: 100,
            fn_brace_newline: true,
            spaces_around_operators: true,
            space_after_comma: true,
        }
    }
}

/// 格式化 KLC 源代码
pub fn format_source(source: &str, config: &FormatConfig) -> Result<String, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    let mut formatter = Formatter::new(config);
    formatter.format_program(&program);
    Ok(formatter.output())
}

/// 格式化 KLC 源文件并写入
pub fn format_file(file_path: &str, config: &FormatConfig) -> Result<(bool, String), String> {
    let source = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Cannot read '{}': {}", file_path, e))?;
    let formatted = format_source(&source, config)?;
    let changed = formatted != source;
    Ok((changed, formatted))
}

/// AST 格式化器 — 从 AST 重建格式化的源代码
struct Formatter {
    config: FormatConfig,
    output: String,
    indent_level: usize,
}

impl Formatter {
    fn new(config: &FormatConfig) -> Self {
        Formatter {
            config: config.clone(),
            output: String::new(),
            indent_level: 0,
        }
    }

    fn output(self) -> String {
        self.output
    }

    fn indent(&self) -> String {
        " ".repeat(self.config.indent_width * self.indent_level)
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emit_line(&mut self) {
        self.output.push('\n');
    }

    fn emit_indent(&mut self) {
        self.emit(&self.indent());
    }

    #[allow(dead_code)]
    fn emit_space_if(&mut self) {
        if self.config.spaces_around_operators {
            self.emit(" ");
        }
    }

    fn format_program(&mut self, program: &Program) {
        for (i, stmt) in program.statements.iter().enumerate() {
            if i > 0 {
                self.emit_line();
            }
            // 顶层声明之间加空行
            if i > 0 && matches!(stmt,
                Stmt::FnDef { .. } | Stmt::TypeDef { .. } |
                Stmt::EnumDef { .. } | Stmt::ImplBlock { .. }
            ) {
                self.emit_line();
            }
            self.format_stmt(stmt, false);
        }
        // 确保文件以换行结尾
        if !self.output.ends_with('\n') {
            self.emit_line();
        }
    }

    fn format_stmt(&mut self, stmt: &Stmt, indent: bool) {
        if indent {
            self.emit_indent();
        }

        match stmt {
            Stmt::Let { name, mutable, value, type_ann } => {
                self.emit("let");
                if *mutable {
                    self.emit(" mut");
                }
                self.emit(&format!(" {}", name));
                if let Some(t) = type_ann {
                    self.emit(&format!(": {}", t));
                }
                self.emit(" = ");
                self.format_expr(value);
            }

            Stmt::Assign { name, value } => {
                self.emit(&format!("{} = ", name));
                self.format_expr(value);
            }

            Stmt::FieldAssign { obj, field, value } => {
                self.emit(&format!("{}.{} = ", obj, field));
                self.format_expr(value);
            }

            Stmt::Expr(expr) => {
                self.format_expr(expr);
            }

            Stmt::Return(expr) => {
                match expr {
                    Some(e) => {
                        self.emit("return ");
                        self.format_expr(e);
                    }
                    None => self.emit("return"),
                }
            }

            Stmt::While(cond, body) => {
                self.emit("while ");
                self.format_expr(cond);
                self.emit_block_open();
                self.format_body(body);
                self.emit_block_close(false);
            }

            Stmt::For { var, iterable, body } => {
                self.emit(&format!("for {} in ", var));
                self.format_expr(iterable);
                self.emit_block_open();
                self.format_body(body);
                self.emit_block_close(false);
            }

            Stmt::If { cond, then_block, else_block } => {
                self.format_if(cond, then_block, else_block, indent);
                return;
            }

            Stmt::Block(stmts) => {
                self.emit_block_open();
                self.format_body(stmts);
                self.emit_block_close(false);
            }

            Stmt::Break => self.emit("break"),

            Stmt::Continue => self.emit("continue"),

            Stmt::FnDef { name, params, return_type, body } => {
                self.emit(&format!("fn {}(", name));
                self.format_params(params);
                self.emit(")");
                if let Some(rt) = return_type {
                    self.emit(&format!(" -> {}", rt));
                }
                // 检查是否为简写体 (= expr)
                if body.len() == 1 {
                    if let Stmt::Return(Some(_)) = &body[0] {
                        // 简写体
                        if self.config.fn_brace_newline {
                            self.emit_block_open();
                            self.format_body(body);
                            self.emit_block_close(false);
                        } else {
                            self.emit(" ");
                            self.emit("{");
                            self.emit_line();
                            self.indent_level += 1;
                            self.format_stmt(&body[0], true);
                            self.indent_level -= 1;
                            self.emit_indent();
                            self.emit("}");
                        }
                        return;
                    }
                }
                self.emit_block_open();
                self.format_body(body);
                self.emit_block_close(false);
            }

            Stmt::Print(expr) => {
                self.emit("io.print(");
                self.format_expr(expr);
                self.emit(")");
            }

            Stmt::PrintLn(expr) => {
                self.emit("io.println(");
                self.format_expr(expr);
                self.emit(")");
            }

            Stmt::Exit(expr) => {
                self.emit("exit(");
                self.format_expr(expr);
                self.emit(")");
            }

            Stmt::TypeDef { name, fields } => {
                self.emit(&format!("type {} ", name));
                self.emit_block_open();
                self.format_type_fields(fields);
                self.emit_block_close(true);
            }

            Stmt::ImplBlock { type_name, methods } => {
                self.emit(&format!("impl {} ", type_name));
                self.emit_block_open();
                self.format_body(methods);
                self.emit_block_close(false);
            }

            Stmt::EnumDef { name, variants } => {
                self.emit(&format!("enum {} ", name));
                self.emit_block_open();
                self.format_enum_variants(variants);
                self.emit_block_close(true);
            }
        }

        if indent {
            self.emit_line();
        }
    }

    fn format_if(&mut self, cond: &Expr, then_block: &[Stmt], else_block: &Option<Vec<Stmt>>, indent: bool) {
        if indent {
            self.emit_indent();
        }
        self.emit("if ");
        self.format_expr(cond);
        self.emit_block_open();
        self.format_body(then_block);
        self.emit_block_close(false);

        if let Some(else_b) = else_block {
            // 检查是否为 else if
            if else_b.len() == 1 {
                if let Stmt::If { cond: ec, then_block: et, else_block: ee } = &else_b[0] {
                    self.emit(" else ");
                    self.format_if(ec, et, ee, false);
                    self.emit_line();
                    return;
                }
            }
            self.emit(" else");
            self.emit_block_open();
            self.format_body(else_b);
            self.emit_block_close(false);
        }

        if indent {
            self.emit_line();
        }
    }

    fn emit_block_open(&mut self) {
        if self.config.fn_brace_newline {
            self.emit_line();
            self.emit_indent();
            self.emit("{");
            self.emit_line();
        } else {
            self.emit(" {");
            self.emit_line();
        }
        self.indent_level += 1;
    }

    fn emit_block_close(&mut self, trailing_comma: bool) {
        self.indent_level -= 1;
        if trailing_comma {
            // 类型定义、枚举定义末尾有可选的尾逗号
        }
        self.emit_indent();
        self.emit("}");
    }

    fn format_body(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.format_stmt(stmt, true);
            self.emit_line();
        }
    }

    fn format_params(&mut self, params: &[Param]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.emit(",");
                if self.config.space_after_comma {
                    self.emit(" ");
                }
            }
            self.emit(&param.name);
            if let Some(t) = &param.type_ann {
                self.emit(&format!(": {}", t));
            }
        }
    }

    fn format_type_fields(&mut self, fields: &[StructField]) {
        for (i, field) in fields.iter().enumerate() {
            self.emit_indent();
            self.emit(&format!("{}: {}", field.name, field.type_ann));
            if let Some(default) = &field.default {
                self.emit(" = ");
                self.format_expr(default);
            }
            if i < fields.len() - 1 {
                self.emit(",");
            }
            self.emit_line();
        }
    }

    fn format_enum_variants(&mut self, variants: &[EnumVariant]) {
        for (i, variant) in variants.iter().enumerate() {
            self.emit_indent();
            self.emit(&variant.name);
            if !variant.fields.is_empty() {
                self.emit("(");
                for (j, f) in variant.fields.iter().enumerate() {
                    if j > 0 {
                        self.emit(", ");
                    }
                    if let Some(name) = &f.name {
                        self.emit(&format!("{}: ", name));
                    }
                    self.emit(&f.type_ann);
                }
                self.emit(")");
            }
            if i < variants.len() - 1 {
                self.emit(",");
            }
            self.emit_line();
        }
    }

    // ─── 表达式格式化 ───

    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Integer(n) => self.emit(&n.to_string()),
            Expr::Float(f) => self.emit(&f.to_string()),
            Expr::String(s) => self.emit(&format!("\"{}\"", escape_string(s))),
            Expr::Bool(b) => self.emit(if *b { "true" } else { "false" }),
            Expr::Char(c) => {
                if *c == '\\' {
                    self.emit("'\\\\'");
                } else if *c == '\'' {
                    self.emit("'\\''");
                } else if *c == '\n' {
                    self.emit("'\\n'");
                } else if *c == '\t' {
                    self.emit("'\\t'");
                } else {
                    self.emit(&format!("'{}'", c));
                }
            }
            Expr::Ident(name) => self.emit(name),
            Expr::Null => self.emit("null"),

            Expr::Binary(left, op, right) => {
                let need_paren_left = needs_parens(left, op, true);
                let need_paren_right = needs_parens(right, op, false);

                if need_paren_left { self.emit("("); }
                self.format_expr(left);
                if need_paren_left { self.emit(")"); }

                self.emit(&binop_str(op));

                if need_paren_right { self.emit("("); }
                self.format_expr(right);
                if need_paren_right { self.emit(")"); }
            }

            Expr::Unary(op, inner) => {
                match op {
                    UnaryOp::Neg => self.emit("-"),
                    UnaryOp::Not => self.emit("not "),
                }
                let need_paren = matches!(inner.as_ref(),
                    Expr::Binary(_, _, _) | Expr::Unary(_, _)
                );
                if need_paren { self.emit("("); }
                self.format_expr(inner);
                if need_paren { self.emit(")"); }
            }

            Expr::Call(name, args) => {
                self.emit(name);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.emit(",");
                        if self.config.space_after_comma {
                            self.emit(" ");
                        }
                    }
                    self.format_expr(arg);
                }
                self.emit(")");
            }

            Expr::FieldAccess(obj, field) => {
                self.format_expr(obj);
                self.emit(&format!(".{}", field));
            }

            Expr::If(cond, then, else_) => {
                self.emit("if ");
                self.format_expr(cond);
                self.emit(" { ");
                self.format_expr(then);
                self.emit(" }");
                if let Some(e) = else_ {
                    self.emit(" else { ");
                    self.format_expr(e);
                    self.emit(" }");
                }
            }

            Expr::StructLiteral { type_name, fields } => {
                self.emit(type_name);
                self.emit(" {");
                if !fields.is_empty() {
                    self.emit(" ");
                    for (i, (name, value)) in fields.iter().enumerate() {
                        if i > 0 {
                            self.emit(", ");
                        }
                        // 简写检测: 字段名和值相同 → 不写值
                        if let Expr::Ident(val_name) = value {
                            if val_name == name {
                                self.emit(name);
                                continue;
                            }
                        }
                        self.emit(&format!("{}: ", name));
                        self.format_expr(value);
                    }
                    self.emit(" ");
                }
                self.emit("}");
            }

            Expr::Match { value, arms } => {
                self.emit("match ");
                self.format_expr(value);
                self.emit(" {");
                self.emit_line();
                self.indent_level += 1;
                for (i, arm) in arms.iter().enumerate() {
                    self.emit_indent();
                    self.format_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.emit(" if ");
                        self.format_expr(guard);
                    }
                    self.emit(" => ");
                    // 检查单语句还是块
                    if arm.body.len() == 1 && !matches!(&arm.body[0], Stmt::While(_, _) | Stmt::For { .. } | Stmt::If { .. }) {
                        self.format_single_stmt_as_expr(&arm.body[0]);
                    } else {
                        self.emit("{");
                        self.emit_line();
                        self.format_body(&arm.body);
                        self.emit_indent();
                        self.emit("}");
                    }
                    if i < arms.len() - 1 {
                        self.emit(",");
                    }
                    self.emit_line();
                }
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}");
            }

            Expr::Lambda { params, return_type, body } => {
                self.emit("fn(");
                self.format_params(params);
                self.emit(")");
                if let Some(rt) = return_type {
                    self.emit(&format!(" -> {}", rt));
                }
                self.emit(" {");
                self.emit_line();
                self.format_body(body);
                self.emit_indent();
                self.emit("}");
            }

            Expr::TailCall(name, args) => {
                // 输出为普通调用 (尾调用语义由编译器处理)
                self.emit(name);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.format_expr(arg);
                }
                self.emit(")");
            }
        }
    }

    fn format_single_stmt_as_expr(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => self.format_expr(expr),
            Stmt::Return(Some(expr)) => self.format_expr(expr),
            Stmt::PrintLn(expr) => {
                self.emit("io.println(");
                self.format_expr(expr);
                self.emit(")");
            }
            Stmt::Print(expr) => {
                self.emit("io.print(");
                self.format_expr(expr);
                self.emit(")");
            }
            Stmt::Let { name, value, .. } => {
                self.emit(&format!("let {} = ", name));
                self.format_expr(value);
            }
            Stmt::Assign { name, value } => {
                self.emit(&format!("{} = ", name));
                self.format_expr(value);
            }
            _ => self.format_expr(&Expr::Integer(0)),
        }
    }

    fn format_pattern(&mut self, pattern: &MatchPattern) {
        match pattern {
            MatchPattern::Literal(expr) => self.format_expr(expr),
            MatchPattern::Variable(name) => self.emit(name),
            MatchPattern::Or(patterns) => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.emit(" | ");
                    }
                    self.format_pattern(p);
                }
            }
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for ch in s.chars() {
        match ch {
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            '\r' => result.push_str("\\r"),
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c => result.push(c),
        }
    }
    result
}

fn binop_str(op: &BinOp) -> String {
    match op {
        BinOp::Add => String::from(" + "),
        BinOp::Sub => String::from(" - "),
        BinOp::Mul => String::from(" * "),
        BinOp::Div => String::from(" / "),
        BinOp::Mod => String::from(" % "),
        BinOp::Eq => String::from(" == "),
        BinOp::Neq => String::from(" != "),
        BinOp::Lt => String::from(" < "),
        BinOp::Gt => String::from(" > "),
        BinOp::Lte => String::from(" <= "),
        BinOp::Gte => String::from(" >= "),
        BinOp::And => String::from(" and "),
        BinOp::Or => String::from(" or "),
        BinOp::Concat => String::from(" ++ "),
        BinOp::Range => String::from(".."),
        BinOp::RangeInclusive => String::from("..="),
    }
}

/// 检查子表达式是否需要括号
fn needs_parens(inner: &Expr, parent_op: &BinOp, is_left: bool) -> bool {
    match inner {
        Expr::Binary(_, inner_op, _) => {
            precedence(inner_op) < precedence(parent_op) ||
            (precedence(inner_op) == precedence(parent_op) && !is_left && is_right_associative(inner_op))
        }
        _ => false,
    }
}

fn precedence(op: &BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Neq => 3,
        BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte => 4,
        BinOp::Concat => 5,
        BinOp::Add | BinOp::Sub => 6,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 7,
        BinOp::Range | BinOp::RangeInclusive => 8,
    }
}

fn is_right_associative(op: &BinOp) -> bool {
    matches!(op, BinOp::Range | BinOp::RangeInclusive)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(source: &str) -> String {
        format_source(source, &FormatConfig::default()).unwrap()
    }

    #[test]
    fn test_let_statement() {
        assert_eq!(fmt("let x=42"), "let x = 42\n");
        assert_eq!(fmt("let mut y=10"), "let mut y = 10\n");
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(fmt("let z=1+2*3"), "let z = 1 + 2 * 3\n");
    }

    #[test]
    fn test_fn_def() {
        let result = fmt("fn add(a:i32,b:i32)->i32{return a+b}");
        assert!(result.contains("fn add(a: i32, b: i32) -> i32"));
        assert!(result.contains("return a + b"));
    }

    #[test]
    fn test_if_else() {
        let result = fmt("if x>5{io.println(x)}else{io.println(0)}");
        assert!(result.contains("if x > 5"));
        assert!(result.contains("else"));
    }

    #[test]
    fn test_string_escape() {
        assert_eq!(fmt(r#"let s="hello\nworld""#), "let s = \"hello\\nworld\"\n");
    }
}
