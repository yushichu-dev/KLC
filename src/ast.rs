//! KLC AST 定义 — 抽象语法树节点

/// 二元运算符
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Mod,      // %
    Eq,       // ==
    Neq,      // !=
    Lt,       // <
    Gt,       // >
    Lte,      // <=
    Gte,      // >=
    And,      // and
    Or,       // or
    Concat,   // ++
    Range,            // ..
    RangeInclusive,   // ..=
}

/// 一元运算符
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,   // -
    Not,   // ! / not
}

/// 表达式
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    Ident(String),
    Null,
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Call(String, Vec<Expr>),
    FieldAccess(Box<Expr>, String),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>), // if cond then else_opt
    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expr)>,  // (field_name, value)
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Lambda {
        params: Vec<Param>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    /// 尾调用 — TCO 优化 pass 将尾位置的 Call 标记为 TailCall
    /// 代码生成时使用 jmp 替代 call，消除函数调用栈开销
    TailCall(String, Vec<Expr>),
}

/// 函数参数
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<String>,
}

/// 结构体字段定义
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_ann: String,
    pub default: Option<Expr>,
}

/// 语句
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Stmt {
    Let {
        name: String,
        type_ann: Option<String>,
        mutable: bool,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    FieldAssign {
        obj: String,
        field: String,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>),
    While(Expr, Vec<Stmt>),
    For {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    Block(Vec<Stmt>),
    Break,
    Continue,
    FnDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Print(Expr),   // io.print(...)
    PrintLn(Expr), // io.println(...)
    Exit(Expr),    // exit(code) — 终止程序
    TypeDef {
        name: String,
        fields: Vec<StructField>,
    },
    ImplBlock {
        type_name: String,
        methods: Vec<Stmt>,  // FnDef statements
    },
    EnumDef {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

/// 枚举变体
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<EnumField>,  // 空 = 无数据变体 (None)
}

/// 枚举变体字段
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    pub name: Option<String>,  // None = 位置字段
    pub type_ann: String,
}

/// 模式匹配 — match 分支模式
#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Literal(Expr),            // 0, "hello", true
    Variable(String),         // n (catch-all 绑定)
    Or(Vec<MatchPattern>),    // 1 | 2 | 3
}

/// match 分支
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<Expr>,   // if condition
    pub body: Vec<Stmt>,       // 臂体语句块
    pub bind: Option<String>,  // 模式解构绑定变量名 (e.g. Some(val) → "val")
}

/// 程序（AST 顶层）
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
