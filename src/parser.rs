//! KLC 语法分析器 — Token 流 → AST（递归下降）

use crate::token::{Token, TokenKind};
use crate::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    eof_token: Token,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        // 过滤 Newline 和 Eof 之外的 token，但保留它们用于分隔
        let filtered: Vec<Token> = tokens.into_iter()
            .filter(|t| t.kind != TokenKind::Newline)
            .collect();
        let eof = Token::new(TokenKind::Eof, 0, 0, "");
        Self { tokens: filtered, pos: 0, eof_token: eof }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    // ─── 语句解析 ───

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match self.peek().kind {
            TokenKind::Let => self.parse_let(),
            TokenKind::Fn => self.parse_fn_def(),
            TokenKind::Task => {
                self.advance(); // task 语法同 fn，复用
                self.parse_fn_like()
            }
            TokenKind::Loop => self.parse_loop(),
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Return(None)) // continue 暂当作 return
            }
            TokenKind::Break => {
                self.advance(); // break 暂 no-op，返回 Null
                Ok(Stmt::Return(None))
            }
            TokenKind::Pub | TokenKind::Any => {
                // v1.0.3-正式版: pub 作为可见性修饰符前缀
                // pub fn / pub type / pub impl → 跳过 pub，继续解析后续语句
                self.advance(); // skip pub/any
                if self.is_at_end() {
                    return Ok(Stmt::Expr(Expr::Integer(0)));
                }
                // pub 后面应该是 fn, type, impl, let, const 等
                self.parse_statement()
            }
            TokenKind::Return => self.parse_return(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::LBrace => self.parse_block_stmt(),
            TokenKind::Type => self.parse_type_def(),
            TokenKind::Impl => self.parse_impl_block(),
            TokenKind::Enum => self.parse_enum_def(),
            TokenKind::Const | TokenKind::Yield => {
                self.advance();
                Ok(Stmt::Expr(Expr::Integer(0)))
            }
            TokenKind::Trait => {
                self.advance(); // skip trait
                self.consume_ident_str()?; // skip name
                self.skip_generic_params();
                // 跳过 { ... }
                if self.peek().kind == TokenKind::LBrace {
                    self.advance();
                    let mut depth = 1;
                    while depth > 0 && !self.is_at_end() {
                        match self.advance().kind {
                            TokenKind::LBrace => depth += 1,
                            TokenKind::RBrace => depth -= 1,
                            _ => {}
                        }
                    }
                }
                Ok(Stmt::Expr(Expr::Integer(0))) // no-op
            }
            TokenKind::Mod => {
                self.advance(); // skip mod
                self.consume_ident_str()?; // skip module name (e.g. "main")
                Ok(Stmt::Expr(Expr::Integer(0))) // no-op placeholder
            }
            TokenKind::Use => {
                self.advance(); // skip use
                // skip import path (simple: just one ident for now)
                self.consume_ident_str()?;
                Ok(Stmt::Expr(Expr::Integer(0))) // no-op placeholder
            }
            _ => {
                // 检查是否是 io.println/io.print 调用
                if self.match_io_call() {
                    return self.parse_io_call();
                }
                // 检查是否是裸 println/print/exit 调用（无 io. 前缀）
                if self.match_bare_print_call() {
                    return self.parse_bare_print_call();
                }
                // 检查是否是赋值语句
                if self.is_assignment() {
                    return self.parse_assignment();
                }
                // 否则是表达式语句
                let expr = self.parse_expr()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    /// 检查是否是 io.println 或 io.print
    fn match_io_call(&self) -> bool {
        let saved = self.pos;
        let mut i = saved;
        if i < self.tokens.len() && matches!(&self.tokens[i].kind, TokenKind::Ident(s) if s == "io") {
            i += 1;
            if i < self.tokens.len() && self.tokens[i].kind == TokenKind::Dot {
                i += 1;
                if i < self.tokens.len() {
                    match &self.tokens[i].kind {
                        TokenKind::Ident(s) if s == "println" || s == "print" => return true,
                        _ => {}
                    }
                }
            }
        }
        false
    }

    fn parse_io_call(&mut self) -> Result<Stmt, String> {
        self.consume_ident("io")?; // 消费 io
        self.consume(TokenKind::Dot)?;
        let func_name = self.consume_ident_str()?; // println or print
        self.consume(TokenKind::LParen)?;
        let arg = self.parse_expr()?;
        self.consume(TokenKind::RParen)?;

        match func_name.as_str() {
            "println" => Ok(Stmt::PrintLn(arg)),
            "print" => Ok(Stmt::Print(arg)),
            _ => Err(format!("未知的 io 函数: io.{}", func_name)),
        }
    }

    /// 检查是否是裸 println( / print( / exit( 调用（无 io. 前缀）
    fn match_bare_print_call(&self) -> bool {
        if let TokenKind::Ident(s) = &self.peek().kind {
            if s == "println" || s == "print" || s == "exit" {
                // 确保下一个 token 是 (
                if self.pos + 1 < self.tokens.len() {
                    return self.tokens[self.pos + 1].kind == TokenKind::LParen;
                }
            }
        }
        false
    }

    fn parse_bare_print_call(&mut self) -> Result<Stmt, String> {
        let func_name = self.consume_ident_str()?; // println, print or exit
        self.consume(TokenKind::LParen)?;
        let arg = self.parse_expr()?;
        self.consume(TokenKind::RParen)?;

        match func_name.as_str() {
            "println" => Ok(Stmt::PrintLn(arg)),
            "print" => Ok(Stmt::Print(arg)),
            "exit" => Ok(Stmt::Exit(arg)),
            _ => unreachable!(),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Let)?;
        let mutable = if self.peek().kind == TokenKind::Mut {
            self.advance();
            true
        } else {
            false
        };
        // 元组解构: let (a, b) = expr → 取第一个变量名
        let name = if self.peek().kind == TokenKind::LParen {
            self.advance(); // skip (
            let first = self.consume_ident_str()?;
            // 跳过剩余的解构部分直到 )
            let mut depth = 1;
            while depth > 0 && !self.is_at_end() {
                match self.advance().kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    _ => {}
                }
            }
            first
        } else {
            self.consume_ident_str()?
        };
        let type_ann = if self.peek().kind == TokenKind::Colon {
            self.advance();
            Some(self.parse_type_ann()?)
        } else {
            None
        };
        self.consume(TokenKind::Assign)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let { name, type_ann, mutable, value })
    }

    fn is_assignment(&self) -> bool {
        // 检查是否为 ident = expr 或 ident.field = expr 或 self.field = expr 模式
        let saved = self.pos;
        let mut i = saved;
        if i < self.tokens.len() && (matches!(&self.tokens[i].kind, TokenKind::Ident(_))
                                     || matches!(&self.tokens[i].kind, TokenKind::Self_)) {
            i += 1;
            // 检查 .field 字段访问
            if i < self.tokens.len() && self.tokens[i].kind == TokenKind::Dot {
                i += 1;
                if i < self.tokens.len() && matches!(&self.tokens[i].kind, TokenKind::Ident(_)) {
                    i += 1;
                } else {
                    return false;
                }
            }
            if i < self.tokens.len() && self.tokens[i].kind == TokenKind::Assign {
                return true;
            }
            // 也检查复合赋值: +=, -= 等
            if i < self.tokens.len() && matches!(&self.tokens[i].kind,
                TokenKind::PlusEq | TokenKind::MinusEq | TokenKind::StarEq | TokenKind::SlashEq)
            {
                return true;
            }
        }
        false
    }

    fn parse_assignment(&mut self) -> Result<Stmt, String> {
        let name = self.consume_ident_str()?;
        // 处理字段赋值: ident.field = expr
        if self.peek().kind == TokenKind::Dot {
            self.advance(); // skip .
            let field = self.consume_ident_str()?;
            self.consume(TokenKind::Assign)?;
            let value = self.parse_expr()?;
            return Ok(Stmt::FieldAssign { obj: name, field, value });
        }
        // 简单赋值（不支持复合赋值 operator 暂）
        self.consume(TokenKind::Assign)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Assign { name, value })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Return)?;
        if self.is_at_end() || matches!(self.peek().kind, TokenKind::RBrace) {
            return Ok(Stmt::Return(None));
        }
        let expr = self.parse_expr()?;
        Ok(Stmt::Return(Some(expr)))
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::While)?;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While(cond, body))
    }

    fn parse_loop(&mut self) -> Result<Stmt, String> {
        // loop { body } → while true { body }
        self.consume(TokenKind::Loop)?;
        let body = self.parse_block()?;
        Ok(Stmt::While(Expr::Bool(true), body))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::For)?;
        let var = self.consume_ident_str()?;
        self.consume(TokenKind::In)?;
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::For { var, iterable, body })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::If)?;
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_block = if self.peek().kind == TokenKind::Else {
            self.advance();
            if self.peek().kind == TokenKind::If {
                // else if → 递归为嵌套 If
                let nested = self.parse_if_stmt()?;
                Some(vec![nested])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If { cond, then_block, else_block })
    }

    fn parse_fn_def(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Fn)?;
        self.parse_fn_like()
    }

    /// 解析 fn/task 函数体（Fn/Task token 已消费）
    fn parse_fn_like(&mut self) -> Result<Stmt, String> {
        let name = self.consume_ident_str()?;
        self.skip_generic_params(); // 跳过 <T, ...>
        self.consume(TokenKind::LParen)?;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                // 跳过所有权修饰符: own, borrow, borrow mut (未实现, no-op)
                self.skip_ownership_modifiers();
                let param_name = self.consume_ident_str()?;
                // 处理 self mut 语法
                if param_name == "self" && self.peek().kind == TokenKind::Mut {
                    self.advance(); // skip mut
                }
                let type_ann = if self.peek().kind == TokenKind::Colon {
                    self.advance();
                    Some(self.parse_type_ann()?)
                } else {
                    None
                };
                params.push(Param { name: param_name, type_ann });
                if self.peek().kind != TokenKind::Comma {
                    break;
                }
                self.advance(); // 跳过 ,
            }
        }
        self.consume(TokenKind::RParen)?;
        let return_type = if self.peek().kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_ann()?)
        } else {
            None
        };
        let body = if self.peek().kind == TokenKind::Assign {
            // = expr 简写 → { return expr; }
            self.advance(); // 跳过 =
            let expr = self.parse_expr()?;
            vec![Stmt::Return(Some(expr))]
        } else {
            self.parse_block()?
        };
        Ok(Stmt::FnDef { name, params, return_type, body })
    }

    fn parse_block_stmt(&mut self) -> Result<Stmt, String> {
        let stmts = self.parse_block()?;
        Ok(Stmt::Block(stmts))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.consume(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
        }
        self.consume(TokenKind::RBrace)?;
        Ok(stmts)
    }

    // ─── 表达式解析（递归下降，按优先级） ───

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_range()
    }

    /// .. / ..= 范围（优先级最低的表达式运算符）
    fn parse_range(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_or()?;
        while matches!(self.peek().kind, TokenKind::DotDot | TokenKind::DotDotEq) {
            let op = if self.peek().kind == TokenKind::DotDotEq { BinOp::RangeInclusive } else { BinOp::Range };
            self.advance();
            let right = self.parse_or()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.peek().kind == TokenKind::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        while self.peek().kind == TokenKind::And {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_concat()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::Neq => BinOp::Neq,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Lte => BinOp::Lte,
                TokenKind::Gte => BinOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.parse_concat()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        while self.peek().kind == TokenKind::Concat {
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary(Box::new(left), BinOp::Concat, Box::new(right));
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().kind {
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(expr)))
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary(UnaryOp::Not, Box::new(expr)))
            }
            TokenKind::Lt => {
                // <- prefix（通道接收）→ 暂 no-op
                self.advance(); // skip <-
                self.parse_call()
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.peek().kind == TokenKind::LBracket {
                // 索引: expr[index]
                self.advance(); // skip [
                let index = self.parse_expr()?;
                self.consume(TokenKind::RBracket)?;
                // 表示为函数调用: __index_get(expr, index)
                let inner = expr.clone();
                expr = Expr::Call("__index_get".into(), vec![inner, index]);
            } else if self.peek().kind == TokenKind::LParen {
                // 函数调用
                self.advance(); // 跳过 (
                let mut args = Vec::new();
                if self.peek().kind != TokenKind::RParen {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.peek().kind != TokenKind::Comma {
                            break;
                        }
                        self.advance();
                    }
                }
                self.consume(TokenKind::RParen)?;

                // 提取函数名
                let func_name = match &expr {
                    Expr::Ident(name) => name.clone(),
                    _ => return Err("期望在 ( 前有函数名".into()),
                };
                expr = Expr::Call(func_name, args);
            } else if self.peek().kind == TokenKind::Colon2 {
                // Namespace::method 或 Enum::Variant 或 Type::<T>::method(...)
                self.advance(); // 跳过 ::
                loop {
                    self.skip_generic_params(); // 跳过 ::<T> turbofish
                    if self.peek().kind == TokenKind::Colon2 {
                        self.advance(); // 跳过下一层 ::
                    } else {
                        break;
                    }
                }
                // 提取 :: 前的类型名
                let type_name = match &expr {
                    Expr::Ident(name) => name.clone(),
                    _ => return Err("期望在 :: 前有类型名".into()),
                };

                // Type::<T>(args) 直接调用（无方法名）— 如 Option::<i32>(42)
                if self.peek().kind == TokenKind::LParen {
                    self.advance(); // skip (
                    let mut args = Vec::new();
                    if self.peek().kind != TokenKind::RParen {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.peek().kind != TokenKind::Comma { break; }
                            self.advance();
                        }
                    }
                    self.consume(TokenKind::RParen)?;
                    expr = Expr::Call(type_name, args);
                } else {
                    let method_or_variant = self.consume_ident_str()?;

                    // 判断是否为枚举构造器（变体名以大写字母开头，KLC 枚举变体命名惯例）
                    let is_enum_variant = method_or_variant.chars().next()
                        .map_or(false, |c| c.is_uppercase());

                    if self.peek().kind == TokenKind::LParen {
                        self.advance(); // 跳过 (
                        let mut args = Vec::new();
                        if self.peek().kind != TokenKind::RParen {
                            loop {
                                args.push(self.parse_expr()?);
                                if self.peek().kind != TokenKind::Comma { break; }
                                self.advance();
                            }
                        }
                        self.consume(TokenKind::RParen)?;

                        if is_enum_variant {
                            // 枚举构造器: Type::Variant(args...)
                            expr = Expr::EnumConstructor {
                                type_name,
                                variant: method_or_variant,
                                args,
                            };
                        } else {
                            // 普通方法调用: Type::method(args...)
                            let func_name = format!("{}::{}", type_name, method_or_variant);
                            expr = Expr::Call(func_name, args);
                        }
                    } else {
                        if is_enum_variant {
                            // 无参枚举变体: Type::Variant (如 Option::None)
                            expr = Expr::EnumConstructor {
                                type_name,
                                variant: method_or_variant,
                                args: vec![],
                            };
                        } else {
                            // 没有 ()，只是 Namespace::ident，暂作为 Ident 处理
                            expr = Expr::Ident(format!("{}::{}", type_name, method_or_variant));
                        }
                    }
                }
            } else if self.peek().kind == TokenKind::LBrace && matches!(&expr, Expr::Ident(name) if name.chars().next().map_or(false, |c| c.is_uppercase())) {
                // 结构体字面量: TypeName { field: val, ... } — 类型名按 PascalCase 惯例首字母大写
                let type_name = match &expr {
                    Expr::Ident(name) => name.clone(),
                    _ => unreachable!(),
                };
                expr = self.parse_struct_literal(type_name)?;
            } else if self.peek().kind == TokenKind::As {
                // as 类型转换（暂 no-op，跳过 as Type）
                self.advance(); // skip as
                self.parse_type_ann()?; // skip type
            } else if self.peek().kind == TokenKind::Dot {
                // 字段访问或方法调用
                self.advance();
                let field = self.consume_ident_str()?;

                // 检查是否跟有 ( → 方法调用
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek().kind != TokenKind::RParen {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.peek().kind != TokenKind::Comma { break; }
                            self.advance();
                        }
                    }
                    self.consume(TokenKind::RParen)?;
                    // 转换为 Call: method(base, args...)
                    // expr 被移入 all_args，随后被重新赋值为 Call
                    let mut all_args = vec![expr.clone()];
                    all_args.extend(args);
                    expr = Expr::Call(field, all_args);
                } else {
                    expr = Expr::FieldAccess(Box::new(expr), field);
                }
            } else if self.peek().kind == TokenKind::Question {
                // ? 运算符: expr? → try { expr } → Result 提前返回
                self.advance(); // skip ?
                expr = Expr::Try(Box::new(expr));
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer(n) => Ok(Expr::Integer(n)),
            TokenKind::Float(f) => Ok(Expr::Float(f)),
            TokenKind::String(s) => Ok(Expr::String(s)),
            TokenKind::Char(c) => Ok(Expr::Char(c)),
            TokenKind::True => Ok(Expr::Bool(true)),
            TokenKind::False => Ok(Expr::Bool(false)),
            TokenKind::Ident(name) => Ok(Expr::Ident(name)),
            TokenKind::Null => Ok(Expr::Null),
            TokenKind::Self_ => Ok(Expr::Ident("self".into())),
            TokenKind::Go => {
                // go expr → 异步派发到线程池
                // parse_primary 已消费 Go token
                let expr = self.parse_expr()?;
                Ok(Expr::GoSpawn(Box::new(expr)))
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.consume(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBrace => {
                // Map 字面量: {key: val, ...} 或 {} 空映射
                let mut pairs = Vec::new();
                if self.peek().kind != TokenKind::RBrace {
                    loop {
                        let key = match self.peek().kind.clone() {
                            TokenKind::String(s) => { self.advance(); s }
                            TokenKind::Ident(s) => { self.advance(); s }
                            _ => return Err("Map 字面量中期望字符串键".into()),
                        };
                        self.consume(TokenKind::Colon)?;
                        let val = self.parse_expr()?;
                        pairs.push((key, val));
                        if self.peek().kind != TokenKind::Comma { break; }
                        self.advance();
                    }
                }
                self.consume(TokenKind::RBrace)?;
                // 转为 Call("__map", pairs 展开为 key1, val1, key2, val2...)
                let mut args = Vec::new();
                for (k, v) in pairs {
                    args.push(Expr::String(k));
                    args.push(v);
                }
                Ok(Expr::Call("__map".into(), args))
            }
            TokenKind::LBracket => {
                // 数组字面量: [expr, expr, ...]
                let mut items = Vec::new();
                if self.peek().kind != TokenKind::RBracket {
                    loop {
                        items.push(self.parse_expr()?);
                        if self.peek().kind != TokenKind::Comma {
                            break;
                        }
                        self.advance();
                    }
                }
                self.consume(TokenKind::RBracket)?;
                // 表示为 Call("__array", items) — VM 暂不支持数组，由调用方决定如何降级
                Ok(Expr::Call("__array".into(), items))
            }
            TokenKind::If => {
                // if 表达式: if cond { expr } else { expr }
                let cond = self.parse_expr()?;
                self.consume(TokenKind::LBrace)?;
                let then_expr = self.parse_expr()?;
                self.consume(TokenKind::RBrace)?;
                let else_expr = if self.peek().kind == TokenKind::Else {
                    self.advance();
                    if self.peek().kind == TokenKind::LBrace {
                        self.advance();
                        let e = self.parse_expr()?;
                        self.consume(TokenKind::RBrace)?;
                        Some(e)
                    } else {
                        Some(self.parse_expr()?)
                    }
                } else {
                    None
                };
                Ok(Expr::If(Box::new(cond), Box::new(then_expr), else_expr.map(Box::new)))
            }
            TokenKind::Fn => self.parse_lambda(),  // fn(x) -> T { body } 匿名函数
            TokenKind::Match => self.parse_match_expr(),
            _ => Err(format!("意外的 Token: {:?}，位于第 {} 行", token.kind, token.line)),
        }
    }

    // ─── 辅助方法 ───

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.eof_token)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.pos)
            .cloned()
            .unwrap_or(Token { kind: TokenKind::Eof, line: 0, col: 0, lexeme: String::new() });
        self.pos += 1;
        token
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token, String> {
        let token = self.advance();
        if token.kind == expected {
            Ok(token)
        } else {
            Err(format!(
                "期望 {:?}，得到 {:?}，位于第 {} 行 第 {} 列",
                expected, token.kind, token.line, token.col
            ))
        }
    }

    fn consume_ident_str(&mut self) -> Result<String, String> {
        let token = self.advance();
        match token.kind {
            TokenKind::Ident(s) => Ok(s),
            TokenKind::Self_ => Ok("self".into()),
            _ => Err(format!("期望标识符，得到 {:?}，位于第 {} 行", token.kind, token.line)),
        }
    }

    fn consume_ident(&mut self, expected: &str) -> Result<(), String> {
        let name = self.consume_ident_str()?;
        if name == expected {
            Ok(())
        } else {
            Err(format!("期望标识符 '{}'，得到 '{}'", expected, name))
        }
    }

    /// 跳过函数参数中的所有权修饰符: own, borrow, borrow mut
    fn skip_ownership_modifiers(&mut self) {
        match self.peek().kind {
            TokenKind::Own => { self.advance(); }
            TokenKind::Borrow => {
                self.advance();
                if self.peek().kind == TokenKind::Mut { self.advance(); }
            }
            _ => {}
        }
    }

    /// 跳过泛型参数: <T>, <T: Bound>, <K, V>, 支持嵌套 <>
    fn skip_generic_params(&mut self) {
        if self.peek().kind == TokenKind::Lt {
            let mut depth = 1;
            self.advance(); // skip <
            while depth > 0 && !self.is_at_end() {
                match self.advance().kind {
                    TokenKind::Lt => depth += 1,
                    TokenKind::Gt => depth -= 1,
                    _ => {}
                }
            }
        }
    }

    /// 解析类型标注: ident | borrow T | ident<T> | [T] | [ident<T>]
    fn parse_type_ann(&mut self) -> Result<String, String> {
        // 跳过所有权修饰符: borrow, borrow mut
        self.skip_ownership_modifiers();
        if self.peek().kind == TokenKind::LBracket {
            self.advance(); // skip [
            let inner = self.parse_type_ann()?;
            self.skip_generic_params();
            self.consume(TokenKind::RBracket)?; // skip ]
            return Ok(format!("[{}]", inner));
        }
        let name = self.consume_ident_str()?;
        self.skip_generic_params(); // 跳过 Option<T> 中的 <T>
        Ok(name)
    }

    // ─── 结构体相关解析 ───

    /// 解析 type Name { field: Type, ... }
    fn parse_type_def(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Type)?;
        let name = self.consume_ident_str()?;
        self.skip_generic_params(); // 跳过 <T, ...>
        self.consume(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            let field_name = self.consume_ident_str()?;
            self.consume(TokenKind::Colon)?;
            let type_ann = self.parse_type_ann()?;
            let default = if self.peek().kind == TokenKind::Assign {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            fields.push(StructField { name: field_name, type_ann, default });
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            }
        }
        self.consume(TokenKind::RBrace)?;
        Ok(Stmt::TypeDef { name, fields })
    }

    /// 解析 enum Name { Variant(Type), Variant2, ... }
    fn parse_enum_def(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Enum)?;
        let name = self.consume_ident_str()?;
        self.skip_generic_params();
        self.consume(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            let vname = self.consume_ident_str()?;
            let fields = if self.peek().kind == TokenKind::LParen {
                self.advance(); // skip (
                let mut fields = Vec::new();
                if self.peek().kind != TokenKind::RParen {
                    loop {
                        let type_ann = self.parse_type_ann()?;
                        fields.push(EnumField { name: None, type_ann });
                        if self.peek().kind != TokenKind::Comma { break; }
                        self.advance();
                    }
                }
                self.consume(TokenKind::RParen)?;
                fields
            } else if self.peek().kind == TokenKind::LBrace {
                self.advance();
                let mut fields = Vec::new();
                while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
                    let fname = self.consume_ident_str()?;
                    self.consume(TokenKind::Colon)?;
                    let type_ann = self.parse_type_ann()?;
                    fields.push(EnumField { name: Some(fname), type_ann });
                    if self.peek().kind == TokenKind::Comma { self.advance(); }
                }
                self.consume(TokenKind::RBrace)?;
                fields
            } else {
                Vec::new()
            };
            variants.push(EnumVariant { name: vname, fields });
            if self.peek().kind == TokenKind::Comma { self.advance(); }
        }
        self.consume(TokenKind::RBrace)?;
        Ok(Stmt::EnumDef { name, variants })
    }

    /// 解析 impl Name { ... } 或 impl Trait for Type { ... }
    fn parse_impl_block(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Impl)?;
        self.skip_generic_params(); // impl<T>
        let first_name = self.consume_ident_str()?;

        // impl Trait for Type { ... } → 编译方法
        if self.peek().kind == TokenKind::For {
            self.advance(); // skip for
            let target_type = self.consume_ident_str()?; // e.g. "i32"
            self.skip_generic_params();
            self.consume(TokenKind::LBrace)?;
            let mut methods = Vec::new();
            while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
                if self.peek().kind == TokenKind::Fn {
                    methods.push(self.parse_fn_def()?);
                } else {
                    return Err(format!("trait impl 中期望 fn，得到 {:?}", self.peek().kind));
                }
            }
            self.consume(TokenKind::RBrace)?;
            // 使用 trait::type 作为 impl 名
            return Ok(Stmt::ImplBlock { type_name: format!("{}::{}", first_name, target_type), methods });
        }

        // impl<T> Type<T> { ... } → 正常 impl
        self.skip_generic_params(); // 跳过 Type<T> 中的 <T>
        self.consume(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            // impl 块内只允许 fn 定义
            if self.peek().kind == TokenKind::Fn {
                methods.push(self.parse_fn_def()?);
            } else {
                return Err(format!("impl 块中期望 fn，得到 {:?}", self.peek().kind));
            }
        }
        self.consume(TokenKind::RBrace)?;
        Ok(Stmt::ImplBlock { type_name: first_name, methods })
    }

    /// 解析 { field: val, ... } 或 { field, ... }（简写，变量名=字段名）
    fn parse_struct_literal(&mut self, type_name: String) -> Result<Expr, String> {
        self.consume(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        if self.peek().kind != TokenKind::RBrace {
            loop {
                let field_name = self.consume_ident_str()?;
                let value = if self.peek().kind == TokenKind::Colon {
                    self.advance(); // skip :
                    self.parse_expr()?
                } else {
                    // 简写: { id, content } → { id: id, content: content }
                    Expr::Ident(field_name.clone())
                };
                fields.push((field_name, value));
                if self.peek().kind != TokenKind::Comma {
                    break;
                }
                self.advance(); // skip ,
            }
        }
        self.consume(TokenKind::RBrace)?;
        Ok(Expr::StructLiteral { type_name, fields })
    }

    // ─── match 表达式解析 ───

    /// fn(params) -> T { body } 或 fn(params) -> T = expr 匿名函数
    fn parse_lambda(&mut self) -> Result<Expr, String> {
        // Fn token 已由 parse_primary 消费
        self.consume(TokenKind::LParen)?;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                self.skip_ownership_modifiers();
                let param_name = self.consume_ident_str()?;
                if param_name == "self" && self.peek().kind == TokenKind::Mut { self.advance(); }
                let type_ann = if self.peek().kind == TokenKind::Colon {
                    self.advance();
                    Some(self.parse_type_ann()?)
                } else { None };
                params.push(Param { name: param_name, type_ann });
                if self.peek().kind != TokenKind::Comma { break; }
                self.advance();
            }
        }
        self.consume(TokenKind::RParen)?;
        let return_type = if self.peek().kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_ann()?)
        } else { None };
        let body = if self.peek().kind == TokenKind::Assign {
            self.advance();
            let expr = self.parse_expr()?;
            vec![Stmt::Return(Some(expr))]
        } else {
            self.parse_block()?
        };
        Ok(Expr::Lambda { params, return_type, body })
    }

    /// match value { pat1 => expr1, pat2 if guard => expr2, ... }
    fn parse_match_expr(&mut self) -> Result<Expr, String> {
        // Match token 已由 parse_primary 消费
        let value = self.parse_expr()?;
        self.consume(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
            // 可选尾随逗号
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            }
        }
        self.consume(TokenKind::RBrace)?;
        Ok(Expr::Match { value: Box::new(value), arms })
    }

    /// 单个 match 分支: pattern (if guard)? => { body } 或 => stmt
    fn parse_match_arm(&mut self) -> Result<MatchArm, String> {
        let (pattern, bind) = self.parse_match_pattern()?;
        let guard = if self.peek().kind == TokenKind::If {
            self.advance(); // skip if
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.consume(TokenKind::FatArrow)?;
        let body = if self.peek().kind == TokenKind::LBrace {
            self.parse_block()?
        } else {
            // 单语句臂: 解析为表达式语句或完整语句
            match self.peek().kind {
                TokenKind::Return => vec![self.parse_return()?],
                TokenKind::Let => vec![self.parse_let()?],
                TokenKind::While => vec![self.parse_while()?],
                TokenKind::For => vec![self.parse_for()?],
                TokenKind::If => vec![self.parse_if_stmt()?],
                TokenKind::Break => {
                    self.advance();
                    vec![Stmt::Return(None)] // break 暂当作 return
                }
                _ => {
                    // 检查 io.print / io.println
                    if self.match_io_call() {
                        vec![self.parse_io_call()?]
                    } else if self.is_assignment() {
                        vec![self.parse_assignment()?]
                    } else {
                        let expr = self.parse_expr()?;
                        vec![Stmt::Expr(expr)]
                    }
                }
            }
        };
        Ok(MatchArm { pattern, guard, body, bind })
    }

    /// 匹配模式: literal | variable | Some(val) | pattern | pattern ...
    fn parse_match_pattern(&mut self) -> Result<(MatchPattern, Option<String>), String> {
        let mut patterns = Vec::new();
        let (pat, bound) = self.parse_single_pattern_with_args()?;
        let bind = bound.clone();
        patterns.push(pat);
        while self.peek().kind == TokenKind::Bar {
            self.advance(); // skip |
            let (p, _) = self.parse_single_pattern_with_args()?;
            patterns.push(p);
        }
        let pattern = if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            MatchPattern::Or(patterns)
        };
        Ok((pattern, bind))
    }

    /// 解析单个模式 + 参数，返回 (模式, 绑定变量名)
    fn parse_single_pattern_with_args(&mut self) -> Result<(MatchPattern, Option<String>), String> {
        let pat = self.parse_single_pattern()?;
        let bound = self.parse_pattern_args();
        Ok((pat, bound))
    }

    /// 解析模式参数，返回绑定的变量名（如有）: Some(val) → 绑定 "val"
    fn parse_pattern_args(&mut self) -> Option<String> {
        let mut bound_var = None;
        if self.peek().kind == TokenKind::LParen {
            self.advance(); // skip (
            if self.peek().kind != TokenKind::RParen {
                // 尝试读取第一个标识符作为绑定变量
                if let TokenKind::Ident(_) = &self.peek().kind {
                    bound_var = Some(self.consume_ident_str().ok()?);
                }
                // 跳过剩余参数
                let mut depth = 1;
                while depth > 0 && !self.is_at_end() {
                    match self.advance().kind {
                        TokenKind::LParen => depth += 1,
                        TokenKind::RParen => depth -= 1,
                        _ => {}
                    }
                }
            } else {
                self.advance(); // skip )
            }
        }
        // 也跳过 { ... }
        if self.peek().kind == TokenKind::LBrace {
            let mut depth = 1;
            self.advance();
            while depth > 0 && !self.is_at_end() {
                match self.advance().kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => depth -= 1,
                    _ => {}
                }
            }
        }
        bound_var
    }

    /// 单个模式（不含 |）
    fn parse_single_pattern(&mut self) -> Result<MatchPattern, String> {
        match self.peek().kind.clone() {
            TokenKind::Integer(n) => { self.advance(); Ok(MatchPattern::Literal(Expr::Integer(n))) }
            TokenKind::Float(f) => { self.advance(); Ok(MatchPattern::Literal(Expr::Float(f))) }
            TokenKind::String(s) => { self.advance(); Ok(MatchPattern::Literal(Expr::String(s))) }
            TokenKind::Char(c) => { self.advance(); Ok(MatchPattern::Literal(Expr::Char(c))) }
            TokenKind::True => { self.advance(); Ok(MatchPattern::Literal(Expr::Bool(true))) }
            TokenKind::False => { self.advance(); Ok(MatchPattern::Literal(Expr::Bool(false))) }
            TokenKind::Minus => {
                self.advance();
                match self.peek().kind.clone() {
                    TokenKind::Integer(n) => { self.advance(); Ok(MatchPattern::Literal(Expr::Integer(-n))) }
                    TokenKind::Float(f) => { self.advance(); Ok(MatchPattern::Literal(Expr::Float(-f))) }
                    _ => Err("模式中 - 后期望数字".into()),
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                // 检查是否为 Type::Variant 枚举变体模式
                if self.peek().kind == TokenKind::Colon2 {
                    self.advance(); // skip ::
                    let variant = self.consume_ident_str()?;
                    // 返回枚举变体模式 (使用变体名称 + 类型前缀)
                    // 如 Color::Red → Variable("Red"), 会在 pattern_args 中解析参数
                    Ok(MatchPattern::Variable(variant))
                } else {
                    Ok(MatchPattern::Variable(name))
                }
            }
            _ => Err(format!("模式中出现意外 Token: {:?}", self.peek().kind)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Program, String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_let_integer() {
        let program = parse("let x = 42").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Let { name, value, mutable, .. } => {
                assert_eq!(name, "x");
                assert_eq!(*mutable, false);
                assert_eq!(*value, Expr::Integer(42));
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_let_mut() {
        let program = parse("let mut y = 10").unwrap();
        match &program.statements[0] {
            Stmt::Let { name, mutable, value, .. } => {
                assert_eq!(name, "y");
                assert_eq!(*mutable, true);
                assert_eq!(*value, Expr::Integer(10));
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_arithmetic() {
        let program = parse("let z = 1 + 2 * 3").unwrap();
        match &program.statements[0] {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "z");
                // 应该是 1 + (2 * 3)
                match value {
                    Expr::Binary(left, BinOp::Add, right) => {
                        assert_eq!(**left, Expr::Integer(1));
                        match &**right {
                            Expr::Binary(l, BinOp::Mul, r) => {
                                assert_eq!(**l, Expr::Integer(2));
                                assert_eq!(**r, Expr::Integer(3));
                            }
                            _ => panic!("Expected Mul"),
                        }
                    }
                    _ => panic!("Expected Add"),
                }
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_if_stmt() {
        let source = "if x > 5 { io.println(x) }";
        let program = parse(source).unwrap();
        match &program.statements[0] {
            Stmt::If { cond, then_block, else_block } => {
                assert!(matches!(cond, Expr::Binary(_, BinOp::Gt, _)));
                assert!(then_block.len() > 0);
                assert!(else_block.is_none());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_fn_def() {
        let source = "fn add(a: i32, b: i32) -> i32 { return a + b }";
        let program = parse(source).unwrap();
        match &program.statements[0] {
            Stmt::FnDef { name, params, return_type, body } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(return_type.as_deref(), Some("i32"));
                assert!(body.len() > 0);
            }
            _ => panic!("Expected FnDef"),
        }
    }

    #[test]
    fn test_concat() {
        let source = r#"let msg = "Hello " ++ name"#;
        let program = parse(source).unwrap();
        match &program.statements[0] {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "msg");
                assert!(matches!(value, Expr::Binary(_, BinOp::Concat, _)));
            }
            _ => panic!("Expected Let"),
        }
    }
}
