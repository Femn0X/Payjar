use super::{Token, TokenType};
use std::process;

/// Remove `//` line comments and `/* */` block comments.
/// Operates on bytes — safe because all delimiters are ASCII.
pub fn remove_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut out = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < n && bytes[i] != b'\n' { i += 1; }
        } else if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') { i += 1; }
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    // SAFETY: we only copy bytes from valid UTF-8 source.
    unsafe { String::from_utf8_unchecked(out) }
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0 }
    }

    #[inline(always)] fn cur(&self) -> u8  { if self.pos < self.src.len() { self.src[self.pos] } else { 0 } }
    #[inline(always)] fn peek(&self) -> u8 { let p = self.pos+1; if p < self.src.len() { self.src[p] } else { 0 } }
    #[inline(always)] fn advance(&mut self) { self.pos += 1; }

    pub fn next_token(&mut self) -> Token {
        // skip whitespace
        while self.pos < self.src.len() && self.cur().is_ascii_whitespace() { self.advance(); }
        if self.pos >= self.src.len() { return Token::new(TokenType::Eof, ""); }

        let c = self.cur();

        // identifiers / keywords
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = self.pos;
            while self.pos < self.src.len() && (self.cur().is_ascii_alphanumeric() || self.cur() == b'_') {
                self.advance();
            }
            // SAFETY: identifiers are ASCII subsets of valid UTF-8.
            let id = unsafe { std::str::from_utf8_unchecked(&self.src[start..self.pos]) };
            let tt = match id {
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
                "break"     => TokenType::Break,
                "continue"  => TokenType::Continue,
                "and"       => TokenType::And,
                "or"        => TokenType::Or,
                "not"       => TokenType::Not,
                "as"        => TokenType::As,
                "true"      => return Token::new(TokenType::Number, "1"),
                "false"     => return Token::new(TokenType::Number, "0"),
                "null"      => return Token::new(TokenType::Number, "0"),
                "pass"      => return self.next_token(),
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
            while self.pos < self.src.len() && self.cur().is_ascii_digit() { self.advance(); }
            if self.pos < self.src.len() && self.cur() == b'.' && self.peek().is_ascii_digit() {
                self.advance();
                while self.pos < self.src.len() && self.cur().is_ascii_digit() { self.advance(); }
                let s = unsafe { std::str::from_utf8_unchecked(&self.src[start..self.pos]) };
                return Token::new(TokenType::FloatLiteral, s);
            }
            let s = unsafe { std::str::from_utf8_unchecked(&self.src[start..self.pos]) };
            return Token::new(TokenType::Number, s);
        }

        // quoted strings
        if c == b'"' || c == b'\'' {
            let quote = c;
            self.advance();
            let mut buf = String::new();
            while self.pos < self.src.len() && self.cur() != quote {
                let ch = self.cur();
                if ch == b'\\' {
                    self.advance();
                    match self.cur() {
                        b'n'  => buf.push('\n'),
                        b't'  => buf.push('\t'),
                        b'r'  => buf.push('\r'),
                        b'\\' => buf.push('\\'),
                        b'"'  => buf.push('"'),
                        b'\'' => buf.push('\''),
                        esc   => { buf.push('\\'); buf.push(esc as char); }
                    }
                } else {
                    buf.push(ch as char);
                }
                self.advance();
            }
            if self.pos < self.src.len() { self.advance(); }
            return Token::new(TokenType::StringLiteral, buf);
        }

        // backtick strings — copy to String first to avoid borrow conflict
        if c == b'`' {
            self.advance();
            let start = self.pos;
            while self.pos < self.src.len() && self.cur() != b'`' { self.advance(); }
            // Copy into owned String before the mutable advance below.
            let s: String = unsafe {
                std::str::from_utf8_unchecked(&self.src[start..self.pos]).to_owned()
            };
            if self.pos < self.src.len() { self.advance(); }
            return Token::new(TokenType::BacktickString, s);
        }

        // § regex strings (UTF-8: 0xC2 0xA7)
        if c == 0xC2 && self.peek() == 0xA7 {
            self.advance(); self.advance(); // consume both bytes
            if self.cur() == b'"' || self.cur() == b'\'' {
                let quote = self.cur();
                self.advance();
                let mut buf = String::new();
                while self.pos < self.src.len() && self.cur() != quote {
                    buf.push(self.cur() as char);
                    self.advance();
                }
                if self.pos < self.src.len() { self.advance(); }
                return Token::new(TokenType::RegexString, buf);
            } else {
                eprintln!("Lexer Error: No quote found after §");
                process::exit(1);
            }
        }

        // operators and punctuation
        self.advance();
        match c {
            b'+' => {
                if self.cur() == b'=' { self.advance(); Token::new(TokenType::PlusEqual, "+=") }
                else if self.cur() == b'+' { self.advance(); Token::new(TokenType::PlusPlus, "++") }
                else { Token::new(TokenType::Plus, "+") }
            }
            b'-' => {
                if self.cur() == b'=' { self.advance(); Token::new(TokenType::MinusEqual, "-=") }
                else if self.cur() == b'-' { self.advance(); Token::new(TokenType::MinusMinus, "--") }
                else if self.cur() == b'>' { self.advance(); Token::new(TokenType::Arrow, "->") }
                else { Token::new(TokenType::Minus, "-") }
            }
            b'*' => Token::new(TokenType::Multiply, "*"),
            b'/' => Token::new(TokenType::Divide, "/"),
            b'%' => Token::new(TokenType::Modulo, "%"),
            b'(' => Token::new(TokenType::LParen, "("),
            b')' => Token::new(TokenType::RParen, ")"),
            b'[' => Token::new(TokenType::LBrack, "["),
            b']' => Token::new(TokenType::RBrack, "]"),
            b'{' => Token::new(TokenType::LBrace, "{"),
            b'}' => Token::new(TokenType::RBrace, "}"),
            b';' => Token::new(TokenType::Semicolon, ";"),
            b',' => Token::new(TokenType::Comma, ","),
            b'.' => Token::new(TokenType::Dot, "."),
            b':' => Token::new(TokenType::Colon, ":"),
            b'@' => Token::new(TokenType::At, "@"),
            b'^' => Token::new(TokenType::Caret, "^"),
            b'?' => Token::new(TokenType::QM, "?"),
            b'|' => {
                if self.cur() == b'|' { self.advance(); Token::new(TokenType::Or, "||") }
                else { Token::new(TokenType::Line, "|") }
            }
            b'&' => {
                if self.cur() == b'&' { self.advance(); Token::new(TokenType::And, "&&") }
                else { Token::new(TokenType::Amp, "&") }
            }
            b'=' => {
                if self.cur() == b'=' { self.advance(); Token::new(TokenType::EqualEqual, "==") }
                else { Token::new(TokenType::Equal, "=") }
            }
            b'!' => {
                if self.cur() == b'=' { self.advance(); Token::new(TokenType::NotEqual, "!=") }
                else { Token::new(TokenType::Not, "!") }
            }
            b'<' => {
                if self.cur() == b'=' { self.advance(); Token::new(TokenType::LessEqual, "<=") }
                else { Token::new(TokenType::LessThan, "<") }
            }
            b'>' => {
                if self.cur() == b'=' { self.advance(); Token::new(TokenType::GreaterEqual, ">=") }
                else { Token::new(TokenType::GreaterThan, ">") }
            }
            _ => {
                eprintln!("Lexer Error: Invalid character 0x{:02x} '{}'", c, c as char);
                process::exit(1);
            }
        }
    }
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let clean = remove_comments(src);
    let mut lexer = Lexer::new(&clean);
    let mut tokens = Vec::with_capacity(clean.len() / 4 + 8);
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.typ == TokenType::Eof;
        tokens.push(tok);
        if is_eof { break; }
    }
    tokens
}
