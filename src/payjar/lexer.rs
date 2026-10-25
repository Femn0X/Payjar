use super::{Token, TokenType};
use std::process;

/// Remove `//` line comments and `/* */` block comments.
/// Operates on bytes — safe because all delimiters are ASCII.
pub fn remove_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut out = Vec::with_capacity(n);
    let mut i = 0;

    // Track whether we're inside a string or template literal so comment markers
    // (// or /*) inside quotes are preserved.
    let mut in_str: Option<u8> = None; // Stores Some(b'"'), Some(b'\''), or Some(b'`')

    while i < n {
        if let Some(quote) = in_str {
            out.push(bytes[i]);
            // Skip escape sequences so escaped quotes (e.g., \") don't close the string early
            if bytes[i] == b'\\' && i + 1 < n {
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if bytes[i] == quote { 
                in_str = None; 
            }
            i += 1;
        } else if bytes[i] == b'"' || bytes[i] == b'\'' || bytes[i] == b'`' {
            // String literal start detected
            in_str = Some(bytes[i]);
            out.push(bytes[i]);
            i += 1;
        } else if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Line comment outside string
            while i < n && bytes[i] != b'\n' { i += 1; }
        } else if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment outside string
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') { i += 1; }
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }

    // SAFETY: We only copy bytes from a valid UTF-8 source.
    unsafe { String::from_utf8_unchecked(out) }
}

#[inline(always)]
fn is_ident_continue(c: u8) -> bool { c.is_ascii_alphanumeric() || c == b'_' }

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

    /// Consumes a trailing `i` right after a numeric literal (for Complex
    /// literals like `3i` / `2.5i`), but only when the `i` is a genuine
    /// suffix rather than the start of a following identifier -- so
    /// `5if x { ... }` (unusual, but valid if someone omits the space)
    /// still lexes as `5`, `if`, ... instead of swallowing part of `if`.
    fn imaginary_suffix(&mut self) -> bool {
        if self.cur() == b'i' && !is_ident_continue(self.peek()) {
            self.advance();
            true
        } else {
            false
        }
    }

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
                "enum"      =>TokenType::Enum,
                "try"       => TokenType::Try,
                "except"    => TokenType::Except,
                "finally"   => TokenType::Finally,
                "throw"     => TokenType::Throw,
                "main"      => TokenType::Main,
                "self"      => TokenType::Self_,
                "inner_self" | "innerSelf" => TokenType::InnerSelf,
                "func"      => TokenType::Def,
                "async"     => TokenType::Async,
                "await"     => TokenType::Await,
                "println"   => TokenType::Print,
                "printerr"  => TokenType::Perror,
                "let"       => TokenType::Let,
                "const"     => TokenType::Const,
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
                "Array"     => TokenType::Array,
                "Complex"   => TokenType::ComplexType,
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
                if self.imaginary_suffix() {
                    return Token::new(TokenType::ImaginaryLiteral, s);
                }
                return Token::new(TokenType::FloatLiteral, s);
            }
            let s = unsafe { std::str::from_utf8_unchecked(&self.src[start..self.pos]) };
            if self.imaginary_suffix() {
                return Token::new(TokenType::ImaginaryLiteral, s);
            }
            return Token::new(TokenType::Number, s);
        }

        // quoted strings — decode complete UTF-8 characters, not individual bytes
        if c == b'"' || c == b'\'' {
            let quote = c;
            self.advance();
            let mut buf = String::new();
            while self.pos < self.src.len() && self.cur() != quote {
                if self.cur() == b'\\' {
                    self.advance();
                    if self.pos >= self.src.len() {
                        buf.push('\\');
                        break;
                    }
                    match self.cur() {
                        b'n'  => { buf.push('\n'); self.advance(); }
                        b't'  => { buf.push('\t'); self.advance(); }
                        b'r'  => { buf.push('\r'); self.advance(); }
                        b'\\' => { buf.push('\\'); self.advance(); }
                        b'"'  => { buf.push('"'); self.advance(); }
                        b'\'' => { buf.push('\''); self.advance(); }
                        _ => {
                            // Preserve unknown escapes without corrupting Unicode.
                            buf.push('\\');
                            let ch = std::str::from_utf8(&self.src[self.pos..])
                                .expect("lexer source must be valid UTF-8")
                                .chars()
                                .next()
                                .expect("position is before end of source");
                            buf.push(ch);
                            self.pos += ch.len_utf8();
                        }
                    }
                } else {
                    let ch = std::str::from_utf8(&self.src[self.pos..])
                        .expect("lexer source must be valid UTF-8")
                        .chars()
                        .next()
                        .expect("position is before end of source");
                    buf.push(ch);
                    self.pos += ch.len_utf8();
                }
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
                    let ch = std::str::from_utf8(&self.src[self.pos..])
                        .expect("lexer source must be valid UTF-8")
                        .chars()
                        .next()
                        .expect("position is before end of source");
                    buf.push(ch);
                    self.pos += ch.len_utf8();
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
