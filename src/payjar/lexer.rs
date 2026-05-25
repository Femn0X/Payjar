use super::{Token, TokenType};
use std::process;

pub fn remove_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if i + 1 < n && chars[i] == '/' && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' { i += 1; }
        } else if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') { i += 1; }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Lexer { src: src.chars().collect(), pos: 0 }
    }

    fn cur(&self) -> char {
        self.src.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek(&self) -> char {
        self.src.get(self.pos + 1).copied().unwrap_or('\0')
    }

    fn advance(&mut self) { self.pos += 1; }

    pub fn next_token(&mut self) -> Token {
        // skip whitespace
        while self.pos < self.src.len() && self.cur().is_ascii_whitespace() {
            self.advance();
        }
        if self.pos >= self.src.len() {
            return Token::new(TokenType::Eof, "");
        }

        let c = self.cur();

        // identifiers / keywords
        if c.is_ascii_alphabetic() || c == '_' {
            let start = self.pos;
            while self.pos < self.src.len() && (self.cur().is_ascii_alphanumeric() || self.cur() == '_') {
                self.advance();
            }
            let id: String = self.src[start..self.pos].iter().collect();
            let tt = match id.as_str() {
                "import"    => TokenType::Import,
                "public"    => TokenType::Public,
                "private"   => TokenType::Private,
                "class"     => TokenType::Class,
                "main"      => TokenType::Main,
                "self"      => TokenType::Self_,
                "inner_self" | "innerSelf" => TokenType::InnerSelf,
                "func"      => TokenType::Def,
                "println"   => TokenType::Print,
                "printerr"  => TokenType::Perror,
                "let"       => TokenType::Let,
                "const"     => TokenType::Const,
                "var"       => TokenType::Var,
                "new"       => TokenType::New,
                "readln"    => TokenType::Readln,
                "readi"     => TokenType::Readi,
                "readf"     => TokenType::Readf,
                "return"    => TokenType::Return,
                "if"        => TokenType::If,
                "else"      => TokenType::Else,
                "while"     => TokenType::While,
                "for"       => TokenType::For,
                "in"        => TokenType::In,
                "range"     => TokenType::Range,
                "package"   => TokenType::Package,
                "and"       => TokenType::And,
                "or"        => TokenType::Or,
                "not"       => TokenType::Not,
                "as"        => TokenType::As,
                "true"      => return Token::new(TokenType::Number, "1"),
                "false"     => return Token::new(TokenType::Number, "0"),
                "null"      => return Token::new(TokenType::Number, "0"),
                "pass"      => return self.next_token(), // skip
                "Int"       => TokenType::Int,
                "Float"     => TokenType::Float,
                "String"    => TokenType::Str,
                "Null"      => TokenType::Null,
                "Any"       => TokenType::Any,
                "Array"     => TokenType::Array,
                _           => TokenType::Identifier,
            };
            return Token::new(tt, id);
        }

        // numbers
        if c.is_ascii_digit() {
            let start = self.pos;
            while self.pos < self.src.len() && self.cur().is_ascii_digit() {
                self.advance();
            }
            if self.pos < self.src.len() && self.cur() == '.' {
                self.advance();
                while self.pos < self.src.len() && self.cur().is_ascii_digit() {
                    self.advance();
                }
                let s: String = self.src[start..self.pos].iter().collect();
                return Token::new(TokenType::FloatLiteral, s);
            }
            let s: String = self.src[start..self.pos].iter().collect();
            return Token::new(TokenType::Number, s);
        }

        // string literals
        if c == '"' || c == '\'' {
            let quote = c;
            self.advance();
            let mut buf = String::new();
            while self.pos < self.src.len() && self.cur() != quote {
                let ch = self.cur();
                if ch == '\\' {
                    self.advance();
                    let esc = self.cur();
                    match esc {
                        'n'  => buf.push('\n'),
                        't'  => buf.push('\t'),
                        'r'  => buf.push('\r'),
                        '\\' => buf.push('\\'),
                        '"'  => buf.push('"'),
                        '\'' => buf.push('\''),
                        _    => { buf.push('\\'); buf.push(esc); }
                    }
                } else {
                    buf.push(ch);
                }
                self.advance();
            }
            if self.pos < self.src.len() { self.advance(); }
            return Token::new(TokenType::StringLiteral, buf);
        }

        // backtick strings
        if c == '`' {
            self.advance();
            let start = self.pos;
            while self.pos < self.src.len() && self.cur() != '`' { self.advance(); }
            let s: String = self.src[start..self.pos].iter().collect();
            if self.pos < self.src.len() { self.advance(); }
            return Token::new(TokenType::BacktickString, s);
        }

        // operators and punctuation
        self.advance();
        match c {
            '§' => {
                if self.cur() == '"' || self.cur() == '\'' {
                    let quote = self.cur();
                    self.advance();
                    let mut buf = String::new();
                    while self.pos < self.src.len() && self.cur() != quote {
                        let ch = self.cur();
                        buf.push(ch);
                        self.advance();
                    }
                    if self.pos < self.src.len() { self.advance(); }
                    return Token::new(TokenType::RegexString, buf);
                } else {
                    eprintln!("Lexer Error: No quote found after §");
                    process::exit(1);
                }
            }
            '+' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::PlusEqual, "+=") }
                else if self.cur() == '+' { self.advance(); Token::new(TokenType::PlusPlus, "++") }
                else { Token::new(TokenType::Plus, "+") }
            }
            '-' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::MinusEqual, "-=") }
                else if self.cur() == '-' { self.advance(); Token::new(TokenType::MinusMinus, "--") }
                else if self.cur() == '>' { self.advance(); Token::new(TokenType::Arrow, "->") }
                else { Token::new(TokenType::Minus, "-") }
            }
            '*' => Token::new(TokenType::Multiply, "*"),
            '/' => Token::new(TokenType::Divide, "/"),
            '%' => Token::new(TokenType::Modulo, "%"),
            '(' => Token::new(TokenType::LParen, "("),
            ')' => Token::new(TokenType::RParen, ")"),
            '[' => Token::new(TokenType::LBrack, "["),
            ']' => Token::new(TokenType::RBrack, "]"),
            '{' => Token::new(TokenType::LBrace, "{"),
            '}' => Token::new(TokenType::RBrace, "}"),
            ';' => Token::new(TokenType::Semicolon, ";"),
            ',' => Token::new(TokenType::Comma, ","),
            '.' => Token::new(TokenType::Dot, "."),
            ':' => Token::new(TokenType::Colon, ":"),
            '@' => Token::new(TokenType::At, "@"),
            '^' => Token::new(TokenType::Caret, '^'),
            '?' => Token::new(TokenType::QM, "?"),
            '|' => {
                if self.cur() == '|' { self.advance(); Token::new(TokenType::Or, "||") }
                else { Token::new(TokenType::Line, '|') }
            }
            '&' => {
                if self.cur() == '&' { self.advance(); Token::new(TokenType::And, "&&") }
                else { Token::new(TokenType::Amp, '&') }
            }
            '=' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::EqualEqual, "==") }
                else { Token::new(TokenType::Equal, "=") }
            }
            '!' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::NotEqual, "!=") }
                else { Token::new(TokenType::Not, "!") }
            }
            '<' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::LessEqual, "<=") }
                else { Token::new(TokenType::LessThan, "<") }
            }
            '>' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::GreaterEqual, ">=") }
                else { Token::new(TokenType::GreaterThan, ">") }
            }
            _ => {
                eprintln!("Lexer Error: Invalid character '{}'", c);
                process::exit(1);
            }
        }
    }
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let clean = remove_comments(src);
    let mut lexer = Lexer::new(&clean);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.typ == TokenType::Eof;
        tokens.push(tok);
        if is_eof { break; }
    }
    tokens
}
