//! KLC Token 定义 — 词法单元

/// 所有 Token 类型
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 关键字
    Let,
    Mut,
    Fn,
    Return,
    If,
    Else,
    While,
    Loop,
    For,
    In,
    Break,
    Continue,
    Type,
    Impl,
    Mod,
    Use,
    Pub,
    Own,
    Borrow,
    Task,
    Go,
    Match,
    Trait,
    Async,
    Await,
    True,
    False,
    And,
    Or,
    Not,
    Enum,
    Const,
    Yield,
    As,
    Self_,
    Any,
    Null,
    // 字面量
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    // 标识符
    Ident(String),
    // 运算符
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Eq,         // ==
    Neq,        // !=
    Lt,         // <
    Gt,         // >
    Lte,        // <=
    Gte,        // >=
    Assign,     // =
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    Arrow,      // ->
    FatArrow,   // =>
    Colon,      // :
    Colon2,     // ::
    Dot,        // .
    DotDot,     // ..
    DotDotEq,   // ..=
    Comma,      // ,
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Pipe,       // |>
    Bar,        // | (模式匹配 OR)
    Ampersand,  // &
    Question,   // ?
    Question2,  // ??
    Concat,     // ++
    // 换行（语句分隔）
    Newline,
    // 特殊
    Eof,
}

/// 源码中的一个 Token
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize, lexeme: impl Into<String>) -> Self {
        Self { kind, line, col, lexeme: lexeme.into() }
    }
}

/// 关键字映射
pub fn lookup_keyword(word: &str) -> Option<TokenKind> {
    match word {
        "let"      => Some(TokenKind::Let),
        "mut"      => Some(TokenKind::Mut),
        "fn"       => Some(TokenKind::Fn),
        "return"   => Some(TokenKind::Return),
        "if"       => Some(TokenKind::If),
        "else"     => Some(TokenKind::Else),
        "while"    => Some(TokenKind::While),
        "loop"     => Some(TokenKind::Loop),
        "for"      => Some(TokenKind::For),
        "in"       => Some(TokenKind::In),
        "break"    => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "type"     => Some(TokenKind::Type),
        "impl"     => Some(TokenKind::Impl),
        "mod"      => Some(TokenKind::Mod),
        "use"      => Some(TokenKind::Use),
        "pub"      => Some(TokenKind::Pub),
        "own"      => Some(TokenKind::Own),
        "borrow"   => Some(TokenKind::Borrow),
        "task"     => Some(TokenKind::Task),
        "go"       => Some(TokenKind::Go),
        "match"    => Some(TokenKind::Match),
        "trait"    => Some(TokenKind::Trait),
        "async"    => Some(TokenKind::Async),
        "await"    => Some(TokenKind::Await),
        "true"     => Some(TokenKind::True),
        "false"    => Some(TokenKind::False),
        "and"      => Some(TokenKind::And),
        "or"       => Some(TokenKind::Or),
        "not"      => Some(TokenKind::Not),
        "enum"     => Some(TokenKind::Enum),
        "const"    => Some(TokenKind::Const),
        "yield"    => Some(TokenKind::Yield),
        "as"       => Some(TokenKind::As),
        "self"     => Some(TokenKind::Self_),
        "null"     => Some(TokenKind::Null),
        "any"      => Some(TokenKind::Any),
        _ => None,
    }
}
