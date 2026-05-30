//! KLC 词法分析器 — 源码 → Token 流

use crate::token::{Token, TokenKind, lookup_keyword};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// 将整个源码解析为 Token 列表（过滤空白和注释）
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            // 跳过 Newline，但保留作为语句分隔符
            match &token.kind {
                TokenKind::Newline => {} // 可以选择保留或丢弃
                _ => tokens.push(token),
            }
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        if self.pos >= self.source.len() {
            return Token::new(TokenKind::Eof, self.line, self.col, "");
        }

        let start_line = self.line;
        let start_col = self.col;
        let ch = self.current();

        // 换行 — 作为语句分隔符
        if ch == '\n' {
            self.advance();
            return Token::new(TokenKind::Newline, start_line, start_col, "\n");
        }

        // 字符串字面量
        if ch == '"' {
            return self.read_string(start_line, start_col);
        }

        // 字符字面量
        if ch == '\'' {
            return self.read_char(start_line, start_col);
        }

        // 数字字面量
        if ch.is_ascii_digit() {
            return self.read_number(start_line, start_col);
        }

        // 标识符或关键字
        if ch.is_alphabetic() || ch == '_' {
            return self.read_ident_or_keyword(start_line, start_col);
        }

        // 运算符和其他符号
        self.read_operator(start_line, start_col)
    }

    fn current(&self) -> char {
        self.source.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek(&self) -> char {
        self.source.get(self.pos + 1).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let ch = self.current();
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // 跳过空格、制表符、回车
            while self.pos < self.source.len() && matches!(self.current(), ' ' | '\t' | '\r') {
                self.advance();
            }

            // 跳过注释
            if self.pos < self.source.len() && self.current() == '-' && self.peek() == '-' {
                self.advance(); // 跳过第一个 -
                self.advance(); // 跳过第二个 -

                // 检查是否为多行注释 --| ... |--
                if self.pos < self.source.len() && self.current() == '|' {
                    self.advance(); // 跳过 |
                    self.skip_block_comment();
                } else {
                    // 单行注释，读到行尾
                    self.skip_line_comment();
                }
                continue;
            }
            break;
        }
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.source.len() && self.current() != '\n' {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        loop {
            if self.pos >= self.source.len() {
                break;
            }
            if self.current() == '|' && self.peek() == '-' && self.pos + 2 < self.source.len() {
                let next_ch = self.source[self.pos + 2];
                if next_ch == '-' {
                    self.advance(); // |
                    self.advance(); // -
                    self.advance(); // -
                    break;
                }
            }
            self.advance();
        }
    }

    fn read_string(&mut self, line: usize, col: usize) -> Token {
        self.advance(); // 跳过开始 "
        let mut s = String::new();
        while self.pos < self.source.len() && self.current() != '"' {
            if self.current() == '\\' {
                self.advance();
                match self.current() {
                    'n'  => { s.push('\n'); self.advance(); }
                    't'  => { s.push('\t'); self.advance(); }
                    'r'  => { s.push('\r'); self.advance(); }
                    '"'  => { s.push('"');  self.advance(); }
                    '\\' => { s.push('\\'); self.advance(); }
                    c    => { s.push('\\'); s.push(c); self.advance(); }
                }
            } else {
                s.push(self.advance());
            }
        }
        if self.pos < self.source.len() {
            self.advance(); // 跳过结束 "
        }
        Token::new(TokenKind::String(s.clone()), line, col, format!("\"{}\"", s))
    }

    fn read_char(&mut self, line: usize, col: usize) -> Token {
        self.advance(); // 跳过 '
        let ch = if self.current() == '\\' {
            self.advance();
            match self.current() {
                'n' => { self.advance(); '\n' }
                't' => { self.advance(); '\t' }
                '\'' => { self.advance(); '\'' }
                '\\' => { self.advance(); '\\' }
                c => { self.advance(); c }
            }
        } else {
            self.advance()
        };
        if self.pos < self.source.len() && self.current() == '\'' {
            self.advance();
        }
        Token::new(TokenKind::Char(ch), line, col, format!("'{}'", ch))
    }

    fn read_number(&mut self, line: usize, col: usize) -> Token {
        let mut num_str = String::new();
        let mut is_float = false;

        while self.pos < self.source.len() && self.current().is_ascii_digit() {
            num_str.push(self.advance());
        }

        // 检查小数点
        if self.pos < self.source.len() && self.current() == '.' && self.peek().is_ascii_digit() {
            is_float = true;
            num_str.push(self.advance()); // .
            while self.pos < self.source.len() && self.current().is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        if is_float {
            let val: f64 = num_str.parse().unwrap_or(0.0);
            Token::new(TokenKind::Float(val), line, col, num_str)
        } else {
            let val: i64 = num_str.parse().unwrap_or(0);
            Token::new(TokenKind::Integer(val), line, col, num_str)
        }
    }

    fn read_ident_or_keyword(&mut self, line: usize, col: usize) -> Token {
        let mut ident = String::new();
        while self.pos < self.source.len() && (self.current().is_alphanumeric() || self.current() == '_') {
            ident.push(self.advance());
        }

        match lookup_keyword(&ident) {
            Some(kind) => Token::new(kind, line, col, ident),
            None => Token::new(TokenKind::Ident(ident.clone()), line, col, ident),
        }
    }

    fn read_operator(&mut self, line: usize, col: usize) -> Token {
        let ch = self.advance();
        let lexeme = ch.to_string();

        match ch {
            '+' => {
                if self.current() == '+' {
                    self.advance();
                    return Token::new(TokenKind::Concat, line, col, "++");
                }
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::PlusEq, line, col, "+=");
                }
                Token::new(TokenKind::Plus, line, col, "+")
            }
            '-' => {
                if self.current() == '>' {
                    self.advance();
                    return Token::new(TokenKind::Arrow, line, col, "->");
                }
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::MinusEq, line, col, "-=");
                }
                Token::new(TokenKind::Minus, line, col, "-")
            }
            '*' => {
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::StarEq, line, col, "*=");
                }
                Token::new(TokenKind::Star, line, col, "*")
            }
            '/' => {
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::SlashEq, line, col, "/=");
                }
                Token::new(TokenKind::Slash, line, col, "/")
            }
            '%' => Token::new(TokenKind::Percent, line, col, "%"),
            '=' => {
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::Eq, line, col, "==");
                }
                if self.current() == '>' {
                    self.advance();
                    return Token::new(TokenKind::FatArrow, line, col, "=>");
                }
                Token::new(TokenKind::Assign, line, col, "=")
            }
            '!' => {
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::Neq, line, col, "!=");
                }
                Token::new(TokenKind::Not, line, col, "!")
            }
            '<' => {
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::Lte, line, col, "<=");
                }
                if self.current() == '-' {
                    self.advance();
                    return Token::new(TokenKind::Lt, line, col, "<-"); // channel receive
                }
                Token::new(TokenKind::Lt, line, col, "<")
            }
            '>' => {
                if self.current() == '=' {
                    self.advance();
                    return Token::new(TokenKind::Gte, line, col, ">=");
                }
                Token::new(TokenKind::Gt, line, col, ">")
            }
            ':' => {
                if self.current() == ':' {
                    self.advance();
                    return Token::new(TokenKind::Colon2, line, col, "::");
                }
                Token::new(TokenKind::Colon, line, col, ":")
            }
            '.' => {
                if self.current() == '.' {
                    self.advance();
                    if self.current() == '=' {
                        self.advance();
                        return Token::new(TokenKind::DotDotEq, line, col, "..=");
                    }
                    return Token::new(TokenKind::DotDot, line, col, "..");
                }
                Token::new(TokenKind::Dot, line, col, ".")
            }
            ',' => Token::new(TokenKind::Comma, line, col, ","),
            '(' => Token::new(TokenKind::LParen, line, col, "("),
            ')' => Token::new(TokenKind::RParen, line, col, ")"),
            '{' => Token::new(TokenKind::LBrace, line, col, "{"),
            '}' => Token::new(TokenKind::RBrace, line, col, "}"),
            '[' => Token::new(TokenKind::LBracket, line, col, "["),
            ']' => Token::new(TokenKind::RBracket, line, col, "]"),
            '|' => {
                if self.current() == '>' {
                    self.advance();
                    return Token::new(TokenKind::Pipe, line, col, "|>");
                }
                Token::new(TokenKind::Bar, line, col, "|")
            }
            '&' => Token::new(TokenKind::Ampersand, line, col, "&"),
            '?' => {
                if self.current() == '?' {
                    self.advance();
                    return Token::new(TokenKind::Question2, line, col, "??");
                }
                Token::new(TokenKind::Question, line, col, "?")
            }
            _ => Token::new(TokenKind::Eof, line, col, lexeme),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let source = "let x = 42";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(kinds, vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Assign,
            TokenKind::Integer(42),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_string() {
        let source = r#""hello\nworld""#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].kind, TokenKind::String("hello\nworld".into()));
    }

    #[test]
    fn test_comment() {
        let source = "let x = 1 -- this is a comment\nlet y = 2";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        // 过滤 Eof
        let non_eof: Vec<_> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();
        assert!(non_eof.len() >= 6); // let, x, =, 1, let, y, =, 2
    }

    #[test]
    fn test_operators() {
        let source = "++ == != <= >= -> :: .. ..= |> ??";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(kinds, vec![
            TokenKind::Concat,
            TokenKind::Eq,
            TokenKind::Neq,
            TokenKind::Lte,
            TokenKind::Gte,
            TokenKind::Arrow,
            TokenKind::Colon2,
            TokenKind::DotDot,
            TokenKind::DotDotEq,
            TokenKind::Pipe,
            TokenKind::Question2,
            TokenKind::Eof,
        ]);
    }
}
