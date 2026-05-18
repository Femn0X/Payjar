use tempfile::NamedTempFile;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env::{self, args};
use std::f32::consts::E;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
// ============================================================
// TOKENS
// ============================================================
#[derive(Clone,Copy)]
enum RunMode{
 PJC,PJRT,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenType {
    Import, Public, Private, Class, Main, Self_, InnerSelf,
    Def, Print,Perror, Let, Const, Var, New,
    Readln, Readi, Readf, Return, If, Else, While, For, In, Range,
    Package,
    And, Or, Not,
    PlusEqual, MinusEqual,
    Identifier, Number, FloatLiteral, StringLiteral, BacktickString,RegexString,
    Plus, Minus, Multiply, Divide, Modulo,
    Equal, EqualEqual, NotEqual,
    LessThan, GreaterThan, LessEqual, GreaterEqual,
    LParen, RParen, LBrack, RBrack, LBrace, RBrace,
    Line,Amp,QM,Caret,
    Semicolon, Comma, Dot, Colon, At, As,
    Eof,
    Int,Float,Null,Str,Any,Array,
    PlusPlus,MinusMinus,Arrow,
}
#[derive(Debug,Clone)]
struct Complex{
    real:f64,
    imag:f64,
}
impl Complex{
    fn new(r:f64,i:f64)->Self{
        Self{real:r,imag:i}
    }
    fn to_string(&self)->String{
        let l=match self.real{
            0.0=>"".to_string(),
            _=>self.real.to_string()
        };
        let r=match self.imag{
            0.0=>"".to_string(),
            1.0=>"+i".to_string(),
            -1.0=>"-i".to_string(),
            _=>format!("{}i",self.imag)
        };
        format!("{}{}",l,r)
    }
    fn powc(&mut self,other:Complex)->Complex{
        if other.imag!=0.0{
            return Complex::new(0.0,0.0);
        }
        else{
            return Complex::new(self.real.powf(other.real),self.imag.powf(other.imag));
        }
    }
    fn sqrt(&self)->Complex{
        if self.imag!=0.0{
            eprintln!("Not implemented");
        }
        if self.real<0.0{
            return Complex::new(0.0,self.real.abs());
        }
        else{
            return Complex::new(self.real.sqrt(),0.0);
        }
    }
}
#[derive(Debug, Clone)]
struct Token {
    typ: TokenType,
    value: String,
}

impl Token {
    fn new(typ: TokenType, value: impl Into<String>) -> Self {
        Token { typ, value: value.into() }
    }
}

// ============================================================
// LEXER
// ============================================================

fn remove_comments(src: &str) -> String {
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

struct Lexer {
    src: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(src: &str) -> Self {
        Lexer { src: src.chars().collect(), pos: 0 }
    }

    fn cur(&self) -> char {
        self.src.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek(&self) -> char {
        self.src.get(self.pos + 1).copied().unwrap_or('\0')
    }

    fn advance(&mut self) { self.pos += 1; }
    fn next_token(&mut self) -> Token {
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
                if self.cur()== '"' || self.cur()=='\''{
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
                }
                else{eprintln!("No \" found");process::exit(1);}
            }
            '+' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::PlusEqual, "+=") }
                else if self.cur() == '+'{ self.advance();Token::new(TokenType::PlusPlus, "++")}
                else { Token::new(TokenType::Plus, "+") }
            }
            '-' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::MinusEqual, "-=") }
                else if self.cur() == '-' {self.advance();Token::new(TokenType::MinusMinus, "--")}
                else if self.cur() == '>' {self.advance();Token::new(TokenType::Arrow, "->")}
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
            '^' => Token::new(TokenType::Caret,'^'),
            '?' => Token::new(TokenType::QM, "?"),
            '|'=>{
                if self.cur() == '|'{self.advance();Token::new(TokenType::Or, "||")}
                else{Token::new(TokenType::Line, '|')}
            }
            '&'=>{
                if self.cur() == '&' {self.advance();Token::new(TokenType::And, "&&")}
                else{Token::new(TokenType::Amp, '&')}
            }
            '=' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::EqualEqual, "==") }
                else { Token::new(TokenType::Equal, "=") }
            }
            '!' => {
                if self.cur() == '=' { self.advance(); Token::new(TokenType::NotEqual, "!=") }
                else {Token::new(TokenType::Not, "!")}
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

fn tokenize(src: &str) -> Vec<Token> {
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

// ============================================================
// AST NODES
// ============================================================

#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Import, MainDef, FuncDef, ClassDef,
    VarDecl, Assign, CompoundAssign, FieldDecl,
    Print,Perror, Return,
    If, While,
    FuncCall, MemberAccess, MemberAssign,
    NewExpr, ObjectCreation,
    BinaryOp, UnaryOp, LogicalOp,  Index, IndexAssign,
    Int, Float, RegexString,Str,Array,Null, TemplateStr,
    VarAccess, Readln, Readi, Readf, For, Generic,
}

#[derive(Debug, Clone)]
struct AstNode{
    own_type:String,
    var_type:String,
    typ: NodeType,
    int_val: i64,
    fval: f64,
    compval:Complex,
    str_val: Option<String>,
    left: Option<Box<AstNode>>,
    right: Option<Box<AstNode>>,
    expr: Option<Box<AstNode>>,
    object: Option<Box<AstNode>>,
    body: Vec<AstNode>,
    args: Vec<AstNode>,
    else_if_conds: Vec<AstNode>,
    else_if_bodies: Vec<Vec<AstNode>>,
    else_body: Vec<AstNode>,
    fields: Vec<AstNode>,
    methods: Vec<AstNode>,
    template_parts: Vec<AstNode>,
    params: Vec<String>,
    is_method: bool,
    is_const: bool,
    is_call: bool,
    name: Option<String>,
    constructor: Option<Box<AstNode>>,
}

impl AstNode {
    fn new(typ: NodeType) -> Self {
        AstNode {
            own_type:"".to_string(),
            var_type:"".to_string(),
            typ,
            int_val: 0,
            fval: 0.0,
            compval:Complex::new(0.0,0.0),
            str_val: None,
            left: None,
            right: None,
            expr: None,
            object: None,
            body: Vec::new(),
            args: Vec::new(),
            else_if_conds: Vec::new(),
            else_if_bodies: Vec::new(),
            else_body: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            template_parts: Vec::new(),
            params: Vec::new(),
            is_method: false,
            is_const: false,
            is_call: false,
            name: None,
            constructor: None,
        }
    }
}

// ============================================================
// PARSER
// ============================================================

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}
fn to_str(a:NodeType)->String{
   format!("{:#?}",a)
}
fn all_same<T:PartialEq>(slice:&[T])->bool{
    if let Some(first) = slice.first() {
        slice.iter().all(|x| x==first)
    }else {
        true
    }
}
impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, idx: 0 }
    }

    fn cur(&self) -> &Token {
        if self.idx < self.tokens.len() {
            &self.tokens[self.idx]
        } else {
            self.tokens.last().unwrap()
        }
    }

    fn peek(&self, offset: usize) -> &Token {
        let i = self.idx + offset;
        if i < self.tokens.len() { &self.tokens[i] } else { self.tokens.last().unwrap() }
    }

    fn advance(&mut self) { self.idx += 1; }

    fn eat(&mut self, expected: TokenType) {
        if self.cur().typ == expected {
            self.advance();
        } else {
            eprintln!("Syntax Error: expected {:?} but got {:?} ('{}') at index {}",
                expected, self.cur().typ, self.cur().value, self.idx);
            process::exit(1);
        }
    }

    fn parse_program(&mut self) -> AstNode {
        // zero or more imports before public class main
        // supports: import X; / import X as Y; / from X import *;
        loop {
            if self.cur().typ == TokenType::Import {
                self.parse_import();
            } else if self.cur().typ == TokenType::Identifier && self.cur().value == "from" {
                // from <pkg> import * ;
                self.advance(); // skip "from"
                self.advance(); // skip <pkg>
                self.eat(TokenType::Import); // import
                self.eat(TokenType::Multiply); // *
                self.eat(TokenType::Semicolon);
            } else {
                break;
            }
        }
        self.eat(TokenType::Public);
        self.eat(TokenType::Class);
        self.eat(TokenType::Main);
        self.eat(TokenType::LParen);
        self.eat(TokenType::At);
        self.eat(TokenType::Self_);
        self.eat(TokenType::RParen);
        self.eat(TokenType::LBrace);
        let mut prog = AstNode::new(NodeType::MainDef);
        prog.body = self.parse_body();
        self.eat(TokenType::RBrace);
        prog
    }

    fn parse_import(&mut self) -> AstNode {
        self.eat(TokenType::Import);
        let pack = self.cur().value.clone();
        self.advance();
        // optional alias: import math as m
        let alias = if self.cur().typ == TokenType::As {
            self.eat(TokenType::As);
            let a = self.cur().value.clone();
            self.advance();
            Some(a)
        } else {
            None
        };
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Import);
        n.str_val = Some(pack);   // folder/package name
        n.name = alias;           // alias (None = load flat into globals)
        n
    }

    fn parse_package_file(&mut self) -> AstNode {
        self.eat(TokenType::Package);
        self.advance(); // consume name
        self.eat(TokenType::Semicolon);
        let mut pkg = AstNode::new(NodeType::MainDef);
        while self.cur().typ != TokenType::Eof {
            match self.cur().typ {
                TokenType::Def => { pkg.body.push(self.parse_function_definition(false)); }
                TokenType::Public => {
                    self.advance();
                    if self.cur().typ != TokenType::Class {
                        eprintln!("Package Error: expected 'class' after 'public'"); process::exit(1);
                    }
                    pkg.body.push(self.parse_class_definition());
                }
                TokenType::Class => { pkg.body.push(self.parse_class_definition()); }
                _ => {
                    eprintln!("Package Error: only func/class definitions allowed in package, got '{}'",
                        self.cur().value);
                    process::exit(1);
                }
            }
        }
        pkg
    }

    fn parse_body(&mut self) -> Vec<AstNode> {
        let mut stmts = Vec::new();
        while self.cur().typ != TokenType::RBrace && self.cur().typ != TokenType::Eof {
            stmts.push(self.parse_statement());
        }
        stmts
    }

    fn parse_function_definition(&mut self, is_method: bool) -> AstNode {
        self.eat(TokenType::Def);
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::LParen);
        let mut n = AstNode::new(NodeType::FuncDef);
        let  mut t:Vec<String>=vec![];
        n.name = Some(name);
        n.is_method = is_method;
        if self.cur().typ != TokenType::RParen {
            if self.cur().typ == TokenType::Self_ {
                n.params.push("self".to_string());
                self.advance();
            } else {
                n.params.push(self.cur().value.clone());
                self.eat(TokenType::Identifier);
                self.eat(TokenType::Colon);
                t.push(self.parse_type());
            }
            while self.cur().typ == TokenType::Comma {
                self.advance();
                n.params.push(self.cur().value.clone());
                self.eat(TokenType::Identifier);
                self.eat(TokenType::Colon);
                t.push(self.parse_type());
            }
        }
        self.eat(TokenType::RParen);
        self.eat(TokenType::Arrow);
        let ret_type=self.parse_type();
        self.eat(TokenType::LBrace);
        n.body = self.parse_body();
        self.eat(TokenType::RBrace);
        n.own_type=ret_type;
        n
    }

    fn parse_class_definition(&mut self) -> AstNode {
        self.eat(TokenType::Class);
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::LParen);
        self.eat(TokenType::At);
        self.eat(TokenType::InnerSelf);
        self.eat(TokenType::RParen);
        self.eat(TokenType::LBrace);
        let mut n = AstNode::new(NodeType::ClassDef);
        n.name = Some(name);
        while self.cur().typ != TokenType::RBrace && self.cur().typ != TokenType::Eof {
            match self.cur().typ {
                TokenType::Let | TokenType::Const => {
                    let is_const = self.cur().typ == TokenType::Const;
                    self.advance();
                    let fname = self.cur().value.clone();
                    self.eat(TokenType::Identifier);
                    let fval = if self.cur().typ == TokenType::Equal {
                        self.advance();
                        Some(Box::new(self.parse_expression()))
                    } else { None };
                    self.eat(TokenType::Colon);
                    let typ=self.parse_type();
                    
                    self.eat(TokenType::Semicolon);
                    let mut fd = AstNode::new(NodeType::FieldDecl);
                    fd.name = Some(fname);
                    fd.is_const = is_const;
                    fd.expr = fval;
                    n.fields.push(fd);
                }
                TokenType::Def => {
                    let method = self.parse_function_definition(true);
                    if method.name.as_deref() == Some("init") {
                        n.constructor = Some(Box::new(method));
                    } else {
                        n.methods.push(method);
                    }
                }
                _ => {
                    eprintln!("Syntax Error: unexpected token in class body: '{}'", self.cur().value);
                    process::exit(1);
                }
            }
        }
        self.eat(TokenType::RBrace);
        n
    }

     fn parse_primary(&mut self) -> AstNode {
        let cur_typ = self.cur().typ.clone();
        let cur_val = self.cur().value.clone();

        match cur_typ {
            TokenType::Number => {
                let mut n = AstNode::new(NodeType::Int);
                n.int_val = cur_val.parse().unwrap_or(0);
                self.advance();
                n
            }
            TokenType::FloatLiteral => {
                let mut n = AstNode::new(NodeType::Float);
                n.fval = cur_val.parse().unwrap_or(0.0);
                self.advance();
                n
            }
            TokenType::StringLiteral => {
                let mut n = AstNode::new(NodeType::Str);
                n.str_val = Some(cur_val);
                self.advance();
                n
            }
            TokenType::BacktickString => {
                self.advance();
                let s = cur_val;
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let mut n = AstNode::new(NodeType::TemplateStr);
                let mut i = 0;
                let mut buf = String::new();
                while i <= len {
                    if i < len && chars[i] == '$' && i + 1 < len && chars[i + 1] == '{' {
                        if !buf.is_empty() {
                            let mut lit = AstNode::new(NodeType::Str);
                            lit.str_val = Some(buf.clone());
                            n.template_parts.push(lit);
                            buf.clear();
                        }
                        i += 2;
                        let vstart = i;
                        while i < len && chars[i] != '}' { i += 1; }
                        let varexpr: String = chars[vstart..i].iter().collect();
                        i += 1; // skip }
                        let part_node = if let Some(dot_pos) = varexpr.find('.') {
                            let obj_n = &varexpr[..dot_pos];
                            let mem_n = &varexpr[dot_pos + 1..];
                            let mut obj_node = AstNode::new(NodeType::VarAccess);
                            obj_node.name = Some(obj_n.to_string());
                            let mut ma = AstNode::new(NodeType::MemberAccess);
                            ma.object = Some(Box::new(obj_node));
                            ma.name = Some(mem_n.to_string());
                            ma
                        } else {
                            let mut va = AstNode::new(NodeType::VarAccess);
                            va.name = Some(varexpr);
                            va
                        };
                        n.template_parts.push(part_node);
                    } else if i < len {
                        buf.push(chars[i]);
                        i += 1;
                    } else {
                        if !buf.is_empty() {
                            let mut lit = AstNode::new(NodeType::Str);
                            lit.str_val = Some(buf.clone());
                            n.template_parts.push(lit);
                        }
                        break;
                    }
                }
                n
            }
            TokenType::Range => {
                self.advance();
                self.eat(TokenType::LParen);
                let mut n = AstNode::new(NodeType::FuncCall);
                n.name = Some("range".to_string());
                if self.cur().typ != TokenType::RParen {
                    n.args.push(self.parse_expression());
                    while self.cur().typ == TokenType::Comma {
                        self.advance();
                        n.args.push(self.parse_expression());
                    }
                }
                self.eat(TokenType::RParen);
                n
            }
            TokenType::Readln => {
                self.advance();
                self.eat(TokenType::LParen);
                let prompt = self.parse_expression();
                self.eat(TokenType::RParen);
                let mut n = AstNode::new(NodeType::Readln);
                n.expr = Some(Box::new(prompt));
                n
            }
            TokenType::Readi => {
                self.advance();
                self.eat(TokenType::LParen);
                let prompt = self.parse_expression();
                self.eat(TokenType::RParen);
                let mut n = AstNode::new(NodeType::Readi);
                n.expr = Some(Box::new(prompt));
                n
            }
            TokenType::Readf => {
                self.advance();
                self.eat(TokenType::LParen);
                let prompt = self.parse_expression();
                self.eat(TokenType::RParen);
                let mut n = AstNode::new(NodeType::Readf);
                n.expr = Some(Box::new(prompt));
                n
            }
            TokenType::LBrack => {
                self.advance();
                let mut n = AstNode::new(NodeType::Generic);
                if self.cur().typ != TokenType::RBrack {
                    n.args.push(self.parse_expression());
                    while self.cur().typ == TokenType::Comma {
                        self.advance();
                        if self.cur().typ == TokenType::RBrack { break; }
                        n.args.push(self.parse_expression());
                    }
                }
                self.eat(TokenType::RBrack);
                n
            }
            TokenType::New => {
                self.advance();
                let class_name = self.cur().value.clone();
                self.eat(TokenType::Identifier);
                self.eat(TokenType::LParen);
                let mut n = AstNode::new(NodeType::ObjectCreation);
                n.name = Some(class_name);
                if self.cur().typ != TokenType::RParen {
                    n.args.push(self.parse_expression());
                    while self.cur().typ == TokenType::Comma {
                        self.advance();
                        n.args.push(self.parse_expression());
                    }
                }
                self.eat(TokenType::RParen);
                n
            }
            TokenType::LParen => {
                self.advance();
                let e = self.parse_expression();
                self.eat(TokenType::RParen);
                e
            }
            TokenType::Self_ => {
                self.advance();
                let mut node = AstNode::new(NodeType::VarAccess);
                node.name = Some("self".to_string());
                while self.cur().typ == TokenType::Dot {
                    self.advance();
                    let member = self.cur().value.clone();
                    self.eat(TokenType::Identifier);
                    let mut ma = AstNode::new(NodeType::MemberAccess);
                    ma.object = Some(Box::new(node));
                    ma.name = Some(member);
                    if self.cur().typ == TokenType::LParen {
                        ma.is_call = true;
                        self.advance();
                        if self.cur().typ != TokenType::RParen {
                            ma.args.push(self.parse_expression());
                            while self.cur().typ == TokenType::Comma {
                                self.advance();
                                ma.args.push(self.parse_expression());
                            }
                        }
                        self.eat(TokenType::RParen);
                    }
                    node = ma;
                }
                node
            }
            TokenType::Identifier => {
                let name = cur_val;
                self.advance();
                let nxt = self.cur().typ.clone();
                if nxt == TokenType::LParen {
                    self.advance();
                    let mut n = AstNode::new(NodeType::FuncCall);
                    n.name = Some(name);
                    if self.cur().typ != TokenType::RParen {
                        n.args.push(self.parse_expression());
                        while self.cur().typ == TokenType::Comma {
                            self.advance();
                            n.args.push(self.parse_expression());
                        }
                    }
                    self.eat(TokenType::RParen);
                    return n;
                }
                if nxt == TokenType::Dot {
                    let mut obj = AstNode::new(NodeType::VarAccess);
                    obj.name = Some(name);
                    while self.cur().typ == TokenType::Dot {
                        self.advance();
                        let member = self.cur().value.clone();
                        self.eat(TokenType::Identifier);
                        let mut ma = AstNode::new(NodeType::MemberAccess);
                        ma.object = Some(Box::new(obj));
                        ma.name = Some(member);
                        if self.cur().typ == TokenType::LParen {
                            ma.is_call = true;
                            self.advance();
                            if self.cur().typ != TokenType::RParen {
                                ma.args.push(self.parse_expression());
                                while self.cur().typ == TokenType::Comma {
                                    self.advance();
                                    ma.args.push(self.parse_expression());
                                }
                            }
                            self.eat(TokenType::RParen);
                        }
                        obj = ma;
                    }
                    return obj;
                }
                let mut n = AstNode::new(NodeType::VarAccess);
                n.name = Some(name);
                n
            }
            TokenType::RegexString=>{
                let mut n = AstNode::new(NodeType::RegexString);
                n.str_val = Some(cur_val);
                self.advance();
                n
            }
            _ => {
                eprintln!("Syntax Error: unexpected token '{:?}' ('{}') in primary", cur_typ, cur_val);
                process::exit(1);
            }
        }
    }

    fn parse_postfix(&mut self) -> AstNode {
        let mut node = self.parse_primary();
        while self.cur().typ == TokenType::LBrack {
            self.advance();
            let idx = self.parse_expression();
            self.eat(TokenType::RBrack);
            let mut n = AstNode::new(NodeType::Index);
            n.left = Some(Box::new(node));
            n.right = Some(Box::new(idx));
            node = n;
        }
        node
    }

    fn parse_factor(&mut self) -> AstNode {
        if self.cur().typ == TokenType::Plus || self.cur().typ == TokenType::Minus {
            let op = self.cur().value.clone();
            self.advance();
            let mut n = AstNode::new(NodeType::UnaryOp);
            n.str_val = Some(op);
            n.left = Some(Box::new(self.parse_postfix()));
            return n;
        }
        if self.cur().typ == TokenType::Not{
            self.advance();
            let mut n = AstNode::new(NodeType::UnaryOp);
            n.str_val = Some("not".to_string());
            n.left = Some(Box::new(self.parse_postfix()));
            return n;
        }
        if self.cur().typ==TokenType::Amp{
            self.advance();
            let mut n = AstNode::new(NodeType::UnaryOp);
            n.str_val = Some("pointer".to_string());
            n.left = Some(Box::new(self.parse_postfix()));
            return n;
        }
        let mut left = self.parse_postfix();
        while matches!(self.cur().typ, TokenType::Multiply | TokenType::Divide | TokenType::Modulo) {
            let op = self.cur().value.clone();
            self.advance();
            let mut n = AstNode::new(NodeType::BinaryOp);
            n.str_val = Some(op);
            n.left = Some(Box::new(left));
            n.right = Some(Box::new(self.parse_primary()));
            left = n;
        }
        left
    }

    fn parse_term(&mut self) -> AstNode {
        let mut left = self.parse_factor();
        while self.cur().typ == TokenType::Plus || self.cur().typ == TokenType::Minus {
            let op = self.cur().value.clone();
            self.advance();
            let mut n = AstNode::new(NodeType::BinaryOp);
            n.str_val = Some(op);
            n.left = Some(Box::new(left));
            n.right = Some(Box::new(self.parse_factor()));
            left = n;
        }
        left
    }

    fn parse_comparison(&mut self) -> AstNode {
        let mut left = self.parse_term();
        while matches!(self.cur().typ,
            TokenType::EqualEqual | TokenType::NotEqual |
            TokenType::LessThan | TokenType::GreaterThan |
            TokenType::LessEqual | TokenType::GreaterEqual |
            TokenType::Caret | TokenType::Line)
        {
            let op = self.cur().value.clone();
            self.advance();
            let mut n = AstNode::new(NodeType::BinaryOp);
            n.str_val = Some(op);
            n.left = Some(Box::new(left));
            n.right = Some(Box::new(self.parse_term()));
            left = n;
        }
        left
    }

    fn parse_expression(&mut self) -> AstNode {
        let mut left = self.parse_comparison();
        while self.cur().typ == TokenType::And || self.cur().typ == TokenType::Or {
            let op = self.cur().value.clone();
            self.advance();
            let mut n = AstNode::new(NodeType::LogicalOp);
            n.str_val = Some(op);
            n.left = Some(Box::new(left));
            n.right = Some(Box::new(self.parse_comparison()));
            left = n;
        }
        left
    }

    fn parse_var_decl(&mut self) -> AstNode {
        let is_const = self.cur().typ == TokenType::Const;
        self.advance();
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::Colon);
        let var_type=self.parse_type();
        self.eat(TokenType::Equal);
        let val = self.parse_expression();
        let is_ptr_type  = var_type.contains('*');
        let is_list_node = matches!(val.typ, NodeType::Array | NodeType::Generic);
        if is_ptr_type && !is_list_node {
            eprintln!("TypeError: '{}' declared as pointer type '{}' but assigned a non-list value", name, var_type);
            process::exit(1);
        }
        if !is_ptr_type && is_list_node {
            eprintln!("TypeError: '{}' declared as '{}' but assigned a list — did you mean '{}*'?", name, var_type, var_type);
            process::exit(1);
        }
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::VarDecl);
        n.name = Some(name);
        n.is_const = is_const;
        n.expr = Some(Box::new(val));
        n.var_type=var_type;
        n
    }
    fn parse_type(&mut self)->String{
        let mut re:String="".to_string();
        match self.cur().typ{
            TokenType::Array=>{
                re+="Array";
                self.eat(TokenType::Array);
                while self.cur().typ==TokenType::Multiply{ self.advance(); re+="*"; }
            },
            TokenType::Float=>{
                re+="Float";
                self.eat(TokenType::Float);
                while self.cur().typ==TokenType::Multiply { self.advance(); re+="*"; }
            },
            TokenType::Int=>{
                re+="Int";
                self.eat(TokenType::Int);
                while self.cur().typ==TokenType::Multiply { self.advance(); re+="*"; }
            },
            TokenType::Str=>{
                re+="Str";
                self.eat(TokenType::Str);
                while self.cur().typ==TokenType::Multiply { self.advance(); re+="*"; }
            },
            TokenType::Null=>{
                re+="Null";
                self.eat(TokenType::Null);
                if self.cur().typ==TokenType::Multiply{
                    eprintln!("TypeError: Null cannot be a pointer type"); process::exit(1);
                }
            },
            TokenType::Any=>{
                re+="Any";
                self.eat(TokenType::Any);
                while self.cur().typ==TokenType::Multiply { self.advance(); re+="*"; }
            },
            _=>{ eprintln!("TypeError: {:?} is not a valid type", self.cur().value); process::exit(1); },
        };
        re
    }
    fn parse_assign(&mut self) -> AstNode {
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::Equal);
        let val = self.parse_expression();
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Assign);
        n.name = Some(name);
        n.expr = Some(Box::new(val));
        n
    }

    fn parse_print(&mut self) -> AstNode {
        self.eat(TokenType::Print);
        self.eat(TokenType::LParen);
        let e = self.parse_expression();
        self.eat(TokenType::RParen);
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Print);
        n.expr = Some(Box::new(e));
        n
    }
    fn parse_perror(&mut self) -> AstNode{
    	   self.eat(TokenType::Perror);
        self.eat(TokenType::LParen);
        let e = self.parse_expression();
        self.eat(TokenType::RParen);
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Perror);
        n.expr= Some(Box::new(e));
        n
    }
    fn parse_return_stmt(&mut self) -> AstNode {
        self.eat(TokenType::Return);
        let e = self.parse_expression();
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Return);
        n.expr = Some(Box::new(e));
        n
    }

    fn parse_if(&mut self) -> AstNode {
        self.eat(TokenType::If);
        self.eat(TokenType::LParen);
        let cond = self.parse_expression();
        self.eat(TokenType::RParen);
        self.eat(TokenType::LBrace);
        let then_body = self.parse_body();
        self.eat(TokenType::RBrace);
        let mut n = AstNode::new(NodeType::If);
        n.expr = Some(Box::new(cond));
        n.body = then_body;
        while self.cur().typ == TokenType::Else {
            self.advance();
            if self.cur().typ == TokenType::If {
                self.advance();
                self.eat(TokenType::LParen);
                let ei_cond = self.parse_expression();
                self.eat(TokenType::RParen);
                self.eat(TokenType::LBrace);
                let ei_body = self.parse_body();
                self.eat(TokenType::RBrace);
                n.else_if_conds.push(ei_cond);
                n.else_if_bodies.push(ei_body);
            } else {
                self.eat(TokenType::LBrace);
                n.else_body = self.parse_body();
                self.eat(TokenType::RBrace);
                break;
            }
        }
        n
    }

    fn parse_while(&mut self) -> AstNode {
        self.eat(TokenType::While);
        self.eat(TokenType::LParen);
        let cond = self.parse_expression();
        self.eat(TokenType::RParen);
        self.eat(TokenType::LBrace);
        let body = self.parse_body();
        self.eat(TokenType::RBrace);
        let mut n = AstNode::new(NodeType::While);
        n.expr = Some(Box::new(cond));
        n.body = body;
        n
    }

    fn parse_for_post(&mut self) -> AstNode {
        if self.cur().typ == TokenType::Identifier {
            let name = self.cur().value.clone();
            self.advance();
            // i++
            if self.cur().typ == TokenType::PlusPlus {
                self.advance(); self.advance();
                let mut one = AstNode::new(NodeType::Int); one.int_val = 1;
                let mut var = AstNode::new(NodeType::VarAccess); var.name = Some(name.clone());
                let mut add = AstNode::new(NodeType::BinaryOp); add.str_val = Some("+".to_string());
                add.left = Some(Box::new(var)); add.right = Some(Box::new(one));
                let mut n = AstNode::new(NodeType::Assign); n.name = Some(name); n.expr = Some(Box::new(add));
                return n;
            }
            // i--
            if self.cur().typ == TokenType::MinusMinus {
                self.advance(); self.advance();
                let mut one = AstNode::new(NodeType::Int); one.int_val = 1;
                let mut var = AstNode::new(NodeType::VarAccess); var.name = Some(name.clone());
                let mut sub = AstNode::new(NodeType::BinaryOp); sub.str_val = Some("-".to_string());
                sub.left = Some(Box::new(var)); sub.right = Some(Box::new(one));
                let mut n = AstNode::new(NodeType::Assign); n.name = Some(name); n.expr = Some(Box::new(sub));
                return n;
            }
            // i = expr
            if self.cur().typ == TokenType::Equal {
                self.advance();
                let val = self.parse_expression();
                let mut n = AstNode::new(NodeType::Assign); n.name = Some(name); n.expr = Some(Box::new(val));
                return n;
            }
        }
        eprintln!("Syntax Error: invalid for-loop post-step"); process::exit(1);
    }

    fn parse_for(&mut self) -> AstNode {
        self.eat(TokenType::For);
        self.eat(TokenType::LParen);
        if !matches!(self.cur().typ, TokenType::Let | TokenType::Const | TokenType::Var) {
            eprintln!("Syntax Error: for loop expects variable declaration"); process::exit(1);
        }
        self.advance();
        let var_name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        let mut n = AstNode::new(NodeType::For);
        n.name = Some(var_name);
        if self.cur().typ == TokenType::Colon || self.cur().typ == TokenType::In {
            n.is_method = true; // flag: for-in
            if self.cur().typ == TokenType::Colon {
                self.eat(TokenType::Colon);
                let var_type=self.parse_type();
                n.var_type=var_type;
            }
            self.eat(TokenType::In);
            n.object = Some(Box::new(self.parse_expression()));
            self.eat(TokenType::RParen);
            self.eat(TokenType::LBrace);
            n.body = self.parse_body();
            self.eat(TokenType::RBrace);
        } else {
            n.is_method = false;
            self.eat(TokenType::Equal);
            n.left = Some(Box::new(self.parse_expression()));
            self.eat(TokenType::Semicolon);
            n.expr = Some(Box::new(self.parse_expression()));
            self.eat(TokenType::Semicolon);
            n.right = Some(Box::new(self.parse_for_post()));
            self.eat(TokenType::RParen);
            self.eat(TokenType::LBrace);
            n.body = self.parse_body();
            self.eat(TokenType::RBrace);
        }
        n
    }

    fn parse_statement(&mut self) -> AstNode {
        let cur_typ = self.cur().typ.clone();
        match cur_typ {
            TokenType::Print => self.parse_print(),
            TokenType::Perror => self.parse_perror(),
            TokenType::Readln | TokenType::Readi | TokenType::Readf => {
                let expr = self.parse_primary();
                self.eat(TokenType::Semicolon);
                expr
            }
            TokenType::Let | TokenType::Const | TokenType::Var => self.parse_var_decl(),
            TokenType::Def => self.parse_function_definition(false),
            TokenType::Public => {
                self.advance();
                if self.cur().typ == TokenType::Class { return self.parse_class_definition(); }
                eprintln!("Syntax Error: expected 'class' after 'public'"); process::exit(1);
            }
            TokenType::Class => self.parse_class_definition(),
            TokenType::If => self.parse_if(),
            TokenType::While => self.parse_while(),
            TokenType::For => self.parse_for(),
            TokenType::Return => self.parse_return_stmt(),
            TokenType::Import => self.parse_import(),
            _ => {
                // index assign: name[expr] = expr;
                if cur_typ == TokenType::Identifier && self.peek(1).typ == TokenType::LBrack {
                    let obj_name = self.cur().value.clone();
                    self.advance(); // eat identifier
                    self.advance(); // eat [
                    let idx = self.parse_expression();
                    self.eat(TokenType::RBrack);
                    self.eat(TokenType::Equal);
                    let val = self.parse_expression();
                    self.eat(TokenType::Semicolon);
                    let mut obj = AstNode::new(NodeType::VarAccess);
                    obj.name = Some(obj_name);
                    let mut n = AstNode::new(NodeType::IndexAssign);
                    n.object = Some(Box::new(obj));
                    n.left = Some(Box::new(idx));
                    n.expr = Some(Box::new(val));
                    return n;
                }
                if cur_typ == TokenType::Identifier || cur_typ == TokenType::Self_ {
                    let nxt = self.peek(1).typ.clone();
                    if cur_typ == TokenType::Identifier && nxt == TokenType::Equal {
                        return self.parse_assign();
                    }
                    if cur_typ == TokenType::Identifier
                        && (nxt == TokenType::PlusEqual || nxt == TokenType::MinusEqual)
                    {
                        let name = self.cur().value.clone();
                        self.advance();
                        let op = self.cur().value.clone();
                        self.advance();
                        let val = self.parse_expression();
                        self.eat(TokenType::Semicolon);
                        let mut var = AstNode::new(NodeType::VarAccess); var.name = Some(name.clone());
                        let mut bin = AstNode::new(NodeType::BinaryOp);
                        bin.str_val = Some(if op.starts_with('+') { "+".to_string() } else { "-".to_string() });
                        bin.left = Some(Box::new(var)); bin.right = Some(Box::new(val));
                        let mut n = AstNode::new(NodeType::Assign); n.name = Some(name); n.expr = Some(Box::new(bin));
                        return n;
                    }
                    if nxt == TokenType::Dot {
                        let obj_name = self.cur().value.clone();
                        self.advance();
                        let mut obj = AstNode::new(NodeType::VarAccess);
                        obj.name = Some(obj_name);
                        while self.cur().typ == TokenType::Dot {
                            self.advance();
                            let member = self.cur().value.clone();
                            self.eat(TokenType::Identifier);
                            if self.cur().typ == TokenType::Equal {
                                self.advance();
                                let val = self.parse_expression();
                                self.eat(TokenType::Semicolon);
                                let mut n = AstNode::new(NodeType::MemberAssign);
                                n.object = Some(Box::new(obj));
                                n.name = Some(member);
                                n.expr = Some(Box::new(val));
                                return n;
                            }
                            let mut ma = AstNode::new(NodeType::MemberAccess);
                            ma.object = Some(Box::new(obj));
                            ma.name = Some(member);
                            if self.cur().typ == TokenType::LParen {
                                ma.is_call = true;
                                self.advance();
                                if self.cur().typ != TokenType::RParen {
                                    ma.args.push(self.parse_expression());
                                    while self.cur().typ == TokenType::Comma {
                                        self.advance();
                                        ma.args.push(self.parse_expression());
                                    }
                                }
                                self.eat(TokenType::RParen);
                            }
                            obj = ma;
                        }
                        if obj.typ == NodeType::MemberAccess && obj.is_call {
                            self.eat(TokenType::Semicolon);
                        }
                        return obj;
                    }
                    if cur_typ == TokenType::Identifier && nxt == TokenType::LParen {
                        let e = self.parse_primary();
                        self.eat(TokenType::Semicolon);
                        return e;
                    }
                }
                eprintln!("Syntax Error: unexpected token '{:?}' ('{}') in statement",
                    self.cur().typ, self.cur().value);
                process::exit(1);
            }
        }
    }
}

// ============================================================
// RUNTIME VALUES
// ============================================================

#[derive(Debug, Clone)]
enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Comp(Complex),
    Null,
    Object(Rc<RefCell<PJObject>>),
    List(Rc<RefCell<Vec<Value>>>),
}

impl Value {
    fn as_int(&self)->i64{
        match self{
            Value::Null=>0,
            Value::Int(i)=>*i,
            Value::Float(f)=>*f as i64,
            Value::Str(s)=>0,
            _=>0,
        }
    }
    fn as_complex(&self)->Complex{
     match self{
      Value::Null=>Complex::new(0.0,0.0),
      Value::Int(i)=>Complex::new((*i) as f64,0.0),
      Value::Float(f)=>Complex::new(*f,0.0),
      Value::Str(s)=>Complex::new(0.0,0.0),
      Value::Comp(c)=>Complex::new(c.real,c.imag),
      _=>Complex::new(0.0,0.0),
     }
    }
    fn as_float(&self)->f64{
        match self{
            Value::Null=>0.0,
            Value::Int(i)=>*i as f64,
            Value::Float(f)=>*f,
            Value::Str(_s)=>0.0,
            _=>0.0,
        }
    }
    fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    fn to_string_repr(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                // mimic %g format
                if f.fract() == 0.0 && f.abs() < 1e15 { format!("{}", *f as i64) }
                else { format!("{}", f) }
            }
            Value::Str(s) => s.clone(),
            Value::Null => "null".to_string(),
            Value::Object(o) => format!("<{} object>", o.borrow().class_name),
            Value::List(l) => {
                let items: Vec<String> = l.borrow().iter().map(|v| {
                    if let Value::Str(s) = v { format!("'{}'", s) }
                    else { v.to_string_repr() }
                }).collect();
                format!("[{}]", items.join(", "))
            }
            _=>"".to_string(),
        }
    }
    fn perror(&self) {
        match self {
            Value::Int(i) => eprint!("{}", i),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 { eprint!("{}", *f as i64); }
                else { eprint!("{}", f); }
            }
            Value::Str(s) => eprint!("{}", s),
            Value::Null => eprint!("null"),
            Value::Object(o) => eprint!("<{} object>", o.borrow().class_name),
            Value::List(l) => {
                eprint!("[");
                let items = l.borrow();
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { eprint!(", "); }
                    if let Value::Str(s) = v { eprint!("'{}'", s); }
                    else { v.perror(); }
                }
                eprint!("]");
            }
            _=>eprint!(""),
        }
    }
    fn print(&self) {
        match self {
            Value::Int(i) => print!("{}", i),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 { print!("{}", *f as i64); }
                else { print!("{}", f); }
            }
            Value::Str(s) => print!("{}", s),
            Value::Null => print!("null"),
            Value::Object(o) => print!("<{} object>", o.borrow().class_name),
            Value::List(l) => {
                print!("[");
                let items = l.borrow();
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { print!(", "); }
                    if let Value::Str(s) = v { print!("'{}'", s); }
                    else { v.print(); }
                }
                print!("]");
            }
            _=>print!(""),
        }
    }
}
#[derive(Debug, Clone)]
struct PJField {
    name: String,
    val: Value,
    is_const: bool,
}

#[derive(Debug, Clone)]
struct PJObject {
    class_name: String,
    fields: Vec<PJField>,
    class_def: AstNode, // NODE_CLASS_DEF
}

impl PJObject {
    fn find_field(&self, name: &str) -> Option<&PJField> {
        self.fields.iter().find(|f| f.name == name)
    }
    fn find_field_mut(&mut self, name: &str) -> Option<&mut PJField> {
        self.fields.iter_mut().find(|f| f.name == name)
    }
    fn find_method(&self, name: &str) -> Option<&AstNode> {
        self.class_def.methods.iter().find(|m| m.name.as_deref() == Some(name))
    }
}

// ============================================================
// SCOPE
// ============================================================

#[derive(Debug, Clone)]
struct VarEntry {
    val: Value,
    is_const: bool,
}

#[derive(Debug)]
struct Scope {
    vars: HashMap<String, VarEntry>,
    parent: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
    fn new(parent: Option<Rc<RefCell<Scope>>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Scope { vars: HashMap::new(), parent }))
    }

    fn set_declare(scope: &Rc<RefCell<Scope>>, name: &str, val: Value, is_const: bool) {
        let mut s = scope.borrow_mut();
        if s.vars.contains_key(name) {
            eprintln!("Runtime Error: Redeclaration of '{}'", name);
            process::exit(1);
        }
        s.vars.insert(name.to_string(), VarEntry { val, is_const });
    }

    fn set_assign(scope: &Rc<RefCell<Scope>>, name: &str, val: Value) {
        let mut cur = Rc::clone(scope);
        loop {
            {
                let mut s = cur.borrow_mut();
                if let Some(entry) = s.vars.get_mut(name) {
                    if entry.is_const {
                        eprintln!("Runtime Error: Cannot assign to const '{}'", name);
                        process::exit(1);
                    }
                    entry.val = val;
                    return;
                }
            }
            let parent = cur.borrow().parent.as_ref().map(Rc::clone);
            match parent {
                Some(p) => cur = p,
                None => {
                    eprintln!("Runtime Error: Assignment to undefined variable '{}'", name);
                    process::exit(1);
                }
            }
        }
    }

    fn get(scope: &Rc<RefCell<Scope>>, name: &str) -> Value {
        let mut cur = Rc::clone(scope);
        loop {
            {
                let s = cur.borrow();
                if let Some(entry) = s.vars.get(name) {
                    return entry.val.clone();
                }
            }
            let parent = cur.borrow().parent.as_ref().map(Rc::clone);
            match parent {
                Some(p) => cur = p,
                None => {
                    eprintln!("Runtime Error: Undefined variable '{}'", name);
                    process::exit(1);
                }
            }
        }
    }
}

// ============================================================
// INTERPRETER / ENV
// ============================================================

struct Env {
    funcs: HashMap<String, AstNode>,
    classes: HashMap<String, AstNode>,
    scope: Rc<RefCell<Scope>>,
}

impl Env {
    fn new() -> Self {
        let mut env = Env {
            funcs: HashMap::new(),
            classes: HashMap::new(),
            scope: Scope::new(None),
        };
        register_builtin_modules(&mut env);
        env
    }
}

fn create_builtin_module(name: &str) -> Rc<RefCell<PJObject>> {
    let mut class_def = AstNode::new(NodeType::ClassDef);
    class_def.name = Some(name.to_string());
    Rc::new(RefCell::new(PJObject {
        class_name: name.to_string(),
        fields: Vec::new(),
        class_def,
    }))
}

fn register_builtin_modules(env: &mut Env) {
    for name in &["math","io","sqllite3","random","regex","gui"] {
        let obj = create_builtin_module(name);
        Scope::set_declare(&env.scope, name, Value::Object(obj), false);
    }
}

// Return signal — we use a Result-like enum threaded through visit()
enum Signal {
    None,
    Return(Value),
}

fn call_func_with_vals(
    env: &mut Env,
    func_def: &AstNode,
    arg_vals: Vec<Value>,
    self_obj: Option<Rc<RefCell<PJObject>>>,
) -> Value {
    let saved_scope = Rc::clone(&env.scope);
    env.scope = Scope::new(Some(Rc::clone(&saved_scope)));

    let mut param_offset = 0;
    if func_def.is_method
        && func_def.params.first().map(String::as_str) == Some("self")
    {
        if let Some(ref obj) = self_obj {
            Scope::set_declare(&env.scope, "self", Value::Object(Rc::clone(obj)), false);
        }
        param_offset = 1;
    }

    let expected = func_def.params.len() - param_offset;
    if expected != arg_vals.len() {
        eprintln!("Runtime Error: Function '{}' expected {} args but got {}",
            func_def.name.as_deref().unwrap_or("?"), expected, arg_vals.len());
        process::exit(1);
    }
    for (i, v) in arg_vals.into_iter().enumerate() {
        Scope::set_declare(&env.scope, &func_def.params[i + param_offset], v, false);
    }

    let body = func_def.body.clone();
    let mut ret = Value::Null;
    for stmt in &body {
        match visit_stmt(env, stmt, self_obj.clone()) {
            Signal::Return(v) => { ret = v; break; }
            Signal::None => {}
        }
    }

    env.scope = saved_scope;
    ret
}
fn visit_stmt(env: &mut Env, node: &AstNode, self_obj: Option<Rc<RefCell<PJObject>>>) -> Signal {
    match node.typ {
        NodeType::Print => {
            let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            v.print();
            println!();
            Signal::None
        }
        NodeType::Perror=>{
                     let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            v.perror();
            Signal::None
        }
        NodeType::VarDecl => {
            let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            Scope::set_declare(&env.scope, node.name.as_deref().unwrap(), v, node.is_const);
            Signal::None
        }
        NodeType::Assign => {
            let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            Scope::set_assign(&env.scope, node.name.as_deref().unwrap(), v);
            Signal::None
        }
        NodeType::Return => {
            let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            Signal::Return(v)
        }
        NodeType::FuncDef => {
            env.funcs.insert(node.name.clone().unwrap(), node.clone());
            Signal::None
        }
        NodeType::ClassDef => {
            env.classes.insert(node.name.clone().unwrap(), node.clone());
            Signal::None
        }
        NodeType::If => {
            let cond = eval(env, node.expr.as_ref().unwrap(), self_obj.clone());
            if cond.truthy() {
                for stmt in &node.body {
                    if let s @ Signal::Return(_) = visit_stmt(env, stmt, self_obj.clone()) { return s; }
                }
            } else {
                let mut handled = false;
                for (ei, ec) in node.else_if_conds.iter().enumerate() {
                    let ecv = eval(env, ec, self_obj.clone());
                    if ecv.truthy() {
                        for stmt in &node.else_if_bodies[ei] {
                            if let s @ Signal::Return(_) = visit_stmt(env, stmt, self_obj.clone()) { return s; }
                        }
                        handled = true;
                        break;
                    }
                }
                if !handled {
                    for stmt in &node.else_body {
                        if let s @ Signal::Return(_) = visit_stmt(env, stmt, self_obj.clone()) { return s; }
                    }
                }
            }
            Signal::None
        }
        NodeType::While => {
            loop {
                let cond = eval(env, node.expr.as_ref().unwrap(), self_obj.clone());
                if !cond.truthy() { break; }
                for stmt in &node.body {
                    if let s @ Signal::Return(_) = visit_stmt(env, stmt, self_obj.clone()) { return s; }
                }
            }
            Signal::None
        }
        NodeType::For => {
            if node.is_method {
                let iter = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
                let items = match iter {
                    Value::List(ref l) => l.borrow().clone(),
                    _ => { eprintln!("Runtime Error: for-in requires a list"); process::exit(1); }
                };
                for item in items {
                    let saved = Rc::clone(&env.scope);
                    env.scope = Scope::new(Some(Rc::clone(&saved)));
                    Scope::set_declare(&env.scope, node.name.as_deref().unwrap(), item, false);
                    for stmt in &node.body {
                        if let s @ Signal::Return(_) = visit_stmt(env, stmt, self_obj.clone()) {
                            env.scope = saved;
                            return s;
                        }
                    }
                    env.scope = saved;
                }
            } else {
                let saved = Rc::clone(&env.scope);
                env.scope = Scope::new(Some(Rc::clone(&saved)));
                let init_val = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
                Scope::set_declare(&env.scope, node.name.as_deref().unwrap(), init_val, false);
                loop {
                    let cond = eval(env, node.expr.as_ref().unwrap(), self_obj.clone());
                    if !cond.truthy() { break; }
                    let body_saved = Rc::clone(&env.scope);
                    env.scope = Scope::new(Some(Rc::clone(&body_saved)));
                    for stmt in &node.body {
                        if let s @ Signal::Return(_) = visit_stmt(env, stmt, self_obj.clone()) {
                            env.scope = body_saved;
                            env.scope = saved;
                            return s;
                        }
                    }
                    env.scope = body_saved;
                    visit_stmt(env, node.right.as_ref().unwrap(), self_obj.clone());
                }
                env.scope = saved;
            }
            Signal::None
        }
        NodeType::MemberAssign => {
            let obj_val = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
            if let Value::Object(ref obj_rc) = obj_val {
                let new_val = eval(env, node.expr.as_ref().unwrap(), self_obj);
                let name = node.name.as_deref().unwrap();
                let mut obj = obj_rc.borrow_mut();
                if let Some(f) = obj.find_field_mut(name) {
                    if f.is_const {
                        eprintln!("Runtime Error: Cannot assign to const field '{}'", name);
                        process::exit(1);
                    }
                    f.val = new_val;
                } else {
                    eprintln!("Runtime Error: Field '{}' not found on '{}'", name, obj.class_name);
                    process::exit(1);
                }
            } else {
                eprintln!("Runtime Error: Member assignment on non-object");
                process::exit(1);
            }
            Signal::None
        }
        NodeType::IndexAssign => {
            let lst = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
            let idx = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            let newval = eval(env, node.expr.as_ref().unwrap(), self_obj);
            if let Value::List(ref l) = lst {
                let mut items = l.borrow_mut();
                let i = idx.as_int() as usize;
                if i >= items.len() {
                    eprintln!("Runtime Error: List index out of range");
                    process::exit(1);
                }
                items[i] = newval;
            } else {
                eprintln!("Runtime Error: Cannot index-assign on non-list");
                process::exit(1);
            }
            Signal::None
        }
        NodeType::Import => Signal::None,
        NodeType::FieldDecl => Signal::None,
        // expression statements (func calls, member calls)
        _ => {
            eval(env, node, self_obj);
            Signal::None
        }
    }
}

fn eval(env: &mut Env, node: &AstNode, self_obj: Option<Rc<RefCell<PJObject>>>) -> Value {
    match node.typ {
        NodeType::Int => Value::Int(node.int_val),
        NodeType::Float => Value::Float(node.fval),
        NodeType::Str => Value::Str(node.str_val.clone().unwrap_or_default()),

        NodeType::TemplateStr => {
            let mut result = String::new();
            for part in &node.template_parts {
                let pv = eval(env, part, self_obj.clone());
                result.push_str(&pv.to_string_repr());
            }
            Value::Str(result)
        }

        NodeType::VarAccess => {
            let name = node.name.as_deref().unwrap();
            if name == "self" {
                if let Some(ref obj) = self_obj {
                    return Value::Object(Rc::clone(obj));
                }
            }
            Scope::get(&env.scope, name)
        }

        NodeType::Readln => {
            let prompt = eval(env, node.expr.as_ref().unwrap(), self_obj);
            print!("{}", prompt.to_string_repr());
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).ok();
            if line.ends_with('\n') { line.pop(); }
            if line.ends_with('\r') { line.pop(); }
            Value::Str(line)
        }

        NodeType::Readi => {
            let prompt = eval(env, node.expr.as_ref().unwrap(), self_obj);
            print!("{}", prompt.to_string_repr());
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).ok();
            let line = line.trim().to_string();
            Value::Int(line.parse().unwrap_or(0))
        }

        NodeType::Readf => {
            let prompt = eval(env, node.expr.as_ref().unwrap(), self_obj);
            print!("{}", prompt.to_string_repr());
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).ok();
            let line = line.trim().to_string();
            Value::Float(line.parse().unwrap_or(0.0))
        }

        /*  list=> {
            let items: Vec<Value> = node.args.iter()
                .map(|a| eval(env, a, self_obj.clone()))
                .collect();
            Value::List(Rc::new(RefCell::new(items)))
        }*/

        NodeType::Index => {
            let lst = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            let idx: Value = eval(env, node.right.as_ref().unwrap(), self_obj);
            match lst {
                Value::List(ref l) => {
                    let items = l.borrow();
                    let mut i = idx.as_int();
                    if i < 0 { i = items.len() as i64 + i; }
                    if i < 0 || i >= items.len() as i64 {
                        eprintln!("Runtime Error: List index {} out of range", i);
                        process::exit(1);
                    }
                    items[i as usize].clone()
                }
                Value::Str(ref s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let mut i = idx.as_int();
                    if i < 0 { i = chars.len() as i64 + i; }
                    if i < 0 || i >= chars.len() as i64 {
                        eprintln!("Runtime Error: String index out of range"); process::exit(1);
                    }
                    Value::Str(chars[i as usize].to_string())
                }
                _ => { eprintln!("Runtime Error: Cannot index into that type"); process::exit(1); }
            }
        }

        NodeType::BinaryOp => {
            let lv = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            let rv = eval(env, node.right.as_ref().unwrap(), self_obj);
            let op = node.str_val.as_deref().unwrap();
            // string concat
            if op == "+" {
                if matches!(lv, Value::Str(_)) || matches!(rv, Value::Str(_)) {
                    return Value::Str(format!("{}{}", lv.to_string_repr(), rv.to_string_repr()));
                }
                if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                    return Value::Float(lv.as_float() + rv.as_float());
                }
                // promote to float if result would overflow i64
                let l = lv.as_int(); let r = rv.as_int();
                return match l.checked_add(r) {
                    Some(v) => Value::Int(v),
                    None    => Value::Float(l as f64 + r as f64),
                };
            }
            match op {
                "-" => {
                    if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                        return Value::Float(lv.as_float() - rv.as_float());
                    }
                    let l = lv.as_int(); let r = rv.as_int();
                    match l.checked_sub(r) {
                        Some(v) => Value::Int(v),
                        None    => Value::Float(l as f64 - r as f64),
                    }
                }
                "*" => {
                    if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                        return Value::Float(lv.as_float() * rv.as_float());
                    }
                    let l = lv.as_int(); let r = rv.as_int();
                    match l.checked_mul(r) {
                        Some(v) => Value::Int(v),
                        None    => Value::Float(l as f64 * r as f64),
                    }
                }
                "/" => {
                    if rv.as_float() == 0.0 { eprintln!("Runtime Error: Division by zero"); process::exit(1); }
                    Value::Float(lv.as_float() / rv.as_float())
                }
                "%" => {
                    if rv.as_int() == 0 { eprintln!("Runtime Error: Modulo by zero"); process::exit(1); }
                    Value::Int(lv.as_int() % rv.as_int())
                }
                "==" => {
                    if let (Value::Str(ls), Value::Str(rs)) = (&lv, &rv) {
                        Value::Int(if ls == rs { 1 } else { 0 })
                    } else { Value::Int(if lv.as_int() == rv.as_int() { 1 } else { 0 }) }
                }
                "!=" => {
                    if let (Value::Str(ls), Value::Str(rs)) = (&lv, &rv) {
                        Value::Int(if ls != rs { 1 } else { 0 })
                    } else { Value::Int(if lv.as_int() != rv.as_int() { 1 } else { 0 }) }
                }
                "<"  => Value::Int(if lv.as_int() <  rv.as_int() { 1 } else { 0 }),
                ">"  => Value::Int(if lv.as_int() >  rv.as_int() { 1 } else { 0 }),
                "<=" => Value::Int(if lv.as_int() <= rv.as_int() { 1 } else { 0 }),
                ">=" => Value::Int(if lv.as_int() >= rv.as_int() { 1 } else { 0 }),
                _    => { eprintln!("Runtime Error: Unknown operator '{}'", op); process::exit(1); }
            }
        }

        NodeType::UnaryOp => {
            let v = eval(env, node.left.as_ref().unwrap(), self_obj);
            match node.str_val.as_deref().unwrap() {
                "-" => Value::Int(-v.as_int()),
                "not"|"!" => Value::Int(if !v.truthy() { 1 } else { 0 }),
                _ => v,
            }
        }

        NodeType::LogicalOp => {
            let lv = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            match node.str_val.as_deref().unwrap() {
                "and" => {
                    if !lv.truthy() { Value::Int(0) }
                    else {
                        let rv = eval(env, node.right.as_ref().unwrap(), self_obj);
                        Value::Int(if rv.truthy() { 1 } else { 0 })
                    }
                }
                "or" => {
                    if lv.truthy() { Value::Int(1) }
                    else {
                        let rv = eval(env, node.right.as_ref().unwrap(), self_obj);
                        Value::Int(if rv.truthy() { 1 } else { 0 })
                    }
                }
                _ => { eprintln!("Runtime Error: Unknown logical op"); process::exit(1); }
            }
        }

        NodeType::FuncCall => {
            let arg_vals: Vec<Value> = node.args.iter()
                .map(|a| eval(env, a, self_obj.clone()))
                .collect();
            let name = node.name.as_deref().unwrap();
            eval_builtin_or_func(env, name, arg_vals, self_obj)
        }

        NodeType::ObjectCreation => {
            let class_name = node.name.as_deref().unwrap();
            let cls = env.classes.get(class_name).cloned().unwrap_or_else(|| {
                eprintln!("Runtime Error: Undefined class '{}'", class_name); process::exit(1);
            });
            let fields: Vec<PJField> = cls.fields.iter().map(|fd| {
                let fv = if let Some(ref e) = fd.expr {
                    eval(env, e, None)
                } else { Value::Null };
                PJField { name: fd.name.clone().unwrap(), val: fv, is_const: fd.is_const }
            }).collect();
            let obj = Rc::new(RefCell::new(PJObject {
                class_name: class_name.to_string(),
                fields,
                class_def: cls.clone(),
            }));
            let arg_vals: Vec<Value> = node.args.iter()
                .map(|a| eval(env, a, self_obj.clone()))
                .collect();
            if let Some(ref ctor) = cls.constructor {
                call_func_with_vals(env, ctor, arg_vals, Some(Rc::clone(&obj)));
            } else if !node.args.is_empty() {
                eprintln!("Runtime Error: Class '{}' has no constructor but got args", class_name);
                process::exit(1);
            }
            Value::Object(obj)
        }

        NodeType::MemberAccess => {
            let obj_val = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
            let member_name = node.name.as_deref().unwrap();
            match obj_val {
                Value::List(ref l) => {
                    if node.is_call {
                        let avals: Vec<Value> = node.args.iter()
                            .map(|a| eval(env, a, self_obj.clone()))
                            .collect();
                        match member_name {
                            "append" => {
                                if avals.len() != 1 { eprintln!("append() takes 1 arg"); process::exit(1); }
                                l.borrow_mut().push(avals.into_iter().next().unwrap());
                                Value::Null
                            }
                            "pop" => {
                                let mut items = l.borrow_mut();
                                if items.is_empty() { eprintln!("Runtime Error: pop from empty list"); process::exit(1); }
                                let idx = if !avals.is_empty() { avals[0].as_int() as usize } else { items.len() - 1 };
                                items.remove(idx)
                            }
                            "remove" => {
                                if avals.len() != 1 { eprintln!("remove() takes 1 arg"); process::exit(1); }
                                let mut items = l.borrow_mut();
                                let pos = items.iter().position(|v| {
                                    match (v, &avals[0]) {
                                        (Value::Str(a), Value::Str(b)) => a == b,
                                        (a, b) => a.as_int() == b.as_int(),
                                    }
                                });
                                match pos {
                                    Some(i) => { items.remove(i); Value::Null }
                                    None => { eprintln!("Runtime Error: remove: value not in list"); process::exit(1); }
                                }
                            }
                            "len" | "length" => Value::Int(l.borrow().len() as i64),
                            "contains" => {
                                if avals.len() != 1 { eprintln!("contains() takes 1 arg"); process::exit(1); }
                                let found = l.borrow().iter().any(|v| {
                                    match (v, &avals[0]) {
                                        (Value::Str(a), Value::Str(b)) => a == b,
                                        (a, b) => a.as_int() == b.as_int(),
                                    }
                                });
                                Value::Int(if found { 1 } else { 0 })
                            }
                            _ => { eprintln!("Runtime Error: List has no method '{}'", member_name); process::exit(1); }
                        }
                    } else {
                        match member_name {
                            "len" | "length" => Value::Int(l.borrow().len() as i64),
                            _ => { eprintln!("Runtime Error: List has no field '{}'", member_name); process::exit(1); }
                        }
                    }
                }
                Value::Object(ref obj_rc) => {
                    if node.is_call {
                        let class_name = obj_rc.borrow().class_name.clone();
                        if let Some(v) = eval_builtin_module_method(
                            class_name.as_str(), member_name, &node.args, env, self_obj.clone())
                        {
                            return v;
                        }
                        let method = {
                            let obj = obj_rc.borrow();
                            obj.find_method(member_name).cloned()
                        };
                        let method = method.unwrap_or_else(|| {
                            eprintln!("Runtime Error: Method '{}' not found on '{}'",
                                member_name, obj_rc.borrow().class_name);
                            process::exit(1);
                        });
                        let avals: Vec<Value> = node.args.iter()
                            .map(|a| eval(env, a, self_obj.clone()))
                            .collect();
                        call_func_with_vals(env, &method, avals, Some(Rc::clone(obj_rc)))
                    } else {
                        obj_rc.borrow().find_field(member_name)
                            .map(|f| f.val.clone())
                            .unwrap_or_else(|| {
                                eprintln!("Runtime Error: Field '{}' not found", member_name);
                                process::exit(1);
                            })
                    }
                }
                _ => { eprintln!("Runtime Error: Member access on non-object/list"); process::exit(1); }
            }
        }

        NodeType::Array | NodeType::Generic => {
            let items: Vec<Value> = node.args.iter()
                .map(|a| eval(env, a, self_obj.clone()))
                .collect();
            Value::List(Rc::new(RefCell::new(items)))
        }

        NodeType::FuncDef | NodeType::ClassDef => {
            // Registrations happen in run() first pass; eval of these is a no-op
            Value::Null
        }

        _ => Value::Null,
    }
}

fn eval_builtin_module_method(
    module: &str,
    name: &str,
    arg_nodes: &[AstNode],
    env: &mut Env,
    self_obj: Option<Rc<RefCell<PJObject>>>,
) -> Option<Value> {
    let args: Vec<Value> = arg_nodes.iter()
        .map(|a| eval(env, a, self_obj.clone()))
        .collect();
    match module {
        "math" => match name {
            "abs" | "sqrt" | "cbrt" | "floor" | "ceil" | "round" |
            "sum" | "min" | "max" | "ln" | "log" | "log2" | "log10" |
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" |
            "sinh" | "cosh" | "tanh" |
            "pow" | "exp" | "hypot" | "sign" | "clamp" | "lerp" |
            "pi" | "e" | "tau" | "inf" | "nan" | "isNan" | "isInf" |
            "toRad" | "toDeg" | "trunc" | "fract" | "gcd" | "lcm" =>
                Some(eval_math_builtin(name, args)),
            _ => None,
        },
        "io" => match name {
            "print" | "println" | "readln" | "readi" | "readf" => {
                Some(eval_builtin_or_func(env, name, args, self_obj))
            }
            _ => None,
        },
        "random" => Some(eval_random_builtin(name, args)),
        "regex"  => Some(eval_regex_builtin(name, args)),
        "gui"    => Some(eval_gui_builtin(name, args)),
        _ => None,
    }
}
macro_rules! need {
        ($n:expr) => {
            if args().len() != $n {
                eprintln!("Function takes {} arg(s)", $n);
                process::exit(1);
            }
        };
    }
fn eval_cmath_buildin(name:&str,args:Vec<Value>)->Value{
    match name{
        "i" => {return Value::Comp(Complex::new(0.0,1.0));}
        _=>{}
    }
    match name{
        "pow"=>{
            need!(2);
            Value::Comp(args[0].as_complex().powc(args[1].as_complex()))
        } 
        "sqrt"=>{
            need!(1);
            Value::Comp(args[0].as_complex().sqrt())
        },
        _=>{
         eprintln!("Runtime Error: math.{}() is not a built-in math function", name);
         process::exit(1);
        }
    }
}
fn eval_math_builtin(name: &str, args: Vec<Value>) -> Value {
    // Helper: require exactly n args

    match name {
        // Constants (called with 0 args)
        "pi"  => { return Value::Float(std::f64::consts::PI); }
        "e"   => { return Value::Float(std::f64::consts::E); }
        "tau" => { return Value::Float(std::f64::consts::TAU); }
        "inf" => { return Value::Float(f64::INFINITY); }
        "nan" => { return Value::Float(f64::NAN); }
        _ => {}
    }
    match name {
        "abs" => {
            need!(1);
            match &args[0] {
                Value::Int(i)   => Value::Int(i.abs()),
                _               => Value::Float(args[0].as_float().abs()),
            }
        }
        "sqrt" => {
            need!(1);
            let x = args[0].as_float();
            if x < 0.0 { eprintln!("math.sqrt: argument must be >= 0"); process::exit(1); }
            Value::Float(x.sqrt())
        }
        "cbrt" => { need!(1); Value::Float(args[0].as_float().cbrt()) }
        "ceil"  => { need!(1); Value::Float(args[0].as_float().ceil()) }
        "round" => { need!(1); Value::Float(args[0].as_float().round()) }
        "trunc" => { need!(1); Value::Float(args[0].as_float().trunc()) }
        "fract" => { need!(1); Value::Float(args[0].as_float().fract()) }
        "sign"  => {
            need!(1);
            let x = args[0].as_float();
            if x > 0.0 { Value::Int(1) } else if x < 0.0 { Value::Int(-1) } else { Value::Int(0) }
        }
        "floor" => {
            // floor(a) or floor(a, b) — floor(a,b) = integer division floor
            match args.len() {
                1 => Value::Float(args[0].as_float().floor()),
                2 => {
                    let a = args[0].as_int();
                    let b = args[1].as_int();
                    if b == 0 { eprintln!("math.floor: division by zero"); process::exit(1); }
                    Value::Int(a.div_euclid(b))
                }
                _ => { eprintln!("math.floor() takes 1 or 2 args"); process::exit(1); }
            }
        }
        "pow" => {
            need!(2);
            if let (Value::Int(base), Value::Int(exp)) = (&args[0], &args[1]) {
                if *exp >= 0 { return Value::Int(base.pow(*exp as u32)); }
            }
            Value::Float(args[0].as_float().powf(args[1].as_float()))
        }
        "exp"  => { need!(1); Value::Float(args[0].as_float().exp()) }
        "ln"   => {
            need!(1);
            let x = args[0].as_float();
            if x <= 0.0 { eprintln!("math.ln: argument must be > 0"); process::exit(1); }
            Value::Float(x.ln())
        }
        "log" => {
            need!(2);
            let x = args[0].as_float();
            let b = args[1].as_float();
            if x <= 0.0 || b <= 0.0 || b == 1.0 {
                eprintln!("math.log: invalid arguments (x and base must be > 0, base != 1)");
                process::exit(1);
            }
            Value::Float(x.log(b))
        }
        "log2"  => {
            need!(1);
            let x = args[0].as_float();
            if x <= 0.0 { eprintln!("math.log2: argument must be > 0"); process::exit(1); }
            Value::Float(x.log2())
        }
        "log10" => {
            need!(1);
            let x = args[0].as_float();
            if x <= 0.0 { eprintln!("math.log10: argument must be > 0"); process::exit(1); }
            Value::Float(x.log10())
        }
        "sin"   => { need!(1); Value::Float(args[0].as_float().sin()) }
        "cos"   => { need!(1); Value::Float(args[0].as_float().cos()) }
        "tan"   => { need!(1); Value::Float(args[0].as_float().tan()) }
        "asin"  => { need!(1); Value::Float(args[0].as_float().asin()) }
        "acos"  => { need!(1); Value::Float(args[0].as_float().acos()) }
        "atan"  => { need!(1); Value::Float(args[0].as_float().atan()) }
        "atan2" => { need!(2); Value::Float(args[0].as_float().atan2(args[1].as_float())) }
        "sinh"  => { need!(1); Value::Float(args[0].as_float().sinh()) }
        "cosh"  => { need!(1); Value::Float(args[0].as_float().cosh()) }
        "tanh"  => { need!(1); Value::Float(args[0].as_float().tanh()) }
        "toRad" => { need!(1); Value::Float(args[0].as_float().to_radians()) }
        "toDeg" => { need!(1); Value::Float(args[0].as_float().to_degrees()) }
        "hypot" => { need!(2); Value::Float(args[0].as_float().hypot(args[1].as_float())) }
        "isNan" => { need!(1); Value::Int(if args[0].as_float().is_nan() { 1 } else { 0 }) }
        "isInf" => { need!(1); Value::Int(if args[0].as_float().is_infinite() { 1 } else { 0 }) }
        "min" => {
            if args.is_empty() { eprintln!("math.min() takes at least 1 arg"); process::exit(1); }
            // accept either math.min(a, b) or math.min(list)
            let vals: Vec<f64> = if args.len() == 1 {
                if let Value::List(l) = &args[0] {
                    l.borrow().iter().map(|v| v.as_float()).collect()
                } else { vec![args[0].as_float()] }
            } else { args.iter().map(|v| v.as_float()).collect() };
            Value::Float(vals.into_iter().fold(f64::INFINITY, f64::min))
        }
        "max" => {
            if args.is_empty() { eprintln!("math.max() takes at least 1 arg"); process::exit(1); }
            let vals: Vec<f64> = if args.len() == 1 {
                if let Value::List(l) = &args[0] {
                    l.borrow().iter().map(|v| v.as_float()).collect()
                } else { vec![args[0].as_float()] }
            } else { args.iter().map(|v| v.as_float()).collect() };
            Value::Float(vals.into_iter().fold(f64::NEG_INFINITY, f64::max))
        }
        "sum" => {
            if args.is_empty() { eprintln!("math.sum() takes at least 1 arg"); process::exit(1); }
            let vals: Vec<f64> = if args.len() == 1 {
                if let Value::List(l) = &args[0] {
                    l.borrow().iter().map(|v| v.as_float()).collect()
                } else { vec![args[0].as_float()] }
            } else { args.iter().map(|v| v.as_float()).collect() };
            let total: f64 = vals.iter().sum();
            if vals.iter().all(|x| x.fract() == 0.0) {
                Value::Int(total as i64)
            } else {
                Value::Float(total)
            }
        }
        "clamp" => {
            need!(3);
            let x   = args[0].as_float();
            let lo  = args[1].as_float();
            let hi  = args[2].as_float();
            Value::Float(x.clamp(lo, hi))
        }
        "lerp" => {
            need!(3);
            let a = args[0].as_float();
            let b = args[1].as_float();
            let t = args[2].as_float();
            Value::Float(a + (b - a) * t)
        }
        "gcd" => {
            need!(2);
            let mut a = args[0].as_int().unsigned_abs();
            let mut b = args[1].as_int().unsigned_abs();
            while b != 0 { let t = b; b = a % b; a = t; }
            Value::Int(a as i64)
        }
        "lcm" => {
            need!(2);
            let a = args[0].as_int().unsigned_abs();
            let b = args[1].as_int().unsigned_abs();
            if a == 0 || b == 0 { return Value::Int(0); }
            let mut ga = a; let mut gb = b;
            while gb != 0 { let t = gb; gb = ga % gb; ga = t; }
            Value::Int((a / ga * b) as i64)
        }
        _ => {
            eprintln!("Runtime Error: math.{}() is not a built-in math function", name);
            process::exit(1);
        }
    }
}

fn eval_random_builtin(name: &str, args: Vec<Value>) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    fn rng() -> u64 {
        let n = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs().wrapping_mul(2654435761))
            .unwrap_or(99991);
        n.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
    }
    match name {
        "randint" => {
            if args.len() != 2 { eprintln!("random.randint() takes 2 args"); process::exit(1); }
            let lo = args[0].as_int(); let hi = args[1].as_int();
            if lo > hi { eprintln!("random.randint: lo > hi"); process::exit(1); }
            Value::Int(lo + (rng() % (hi - lo + 1) as u64) as i64)
        }
        "random" => Value::Float(rng() as f64 / u64::MAX as f64),
        "randFloat" => {
            if args.len() != 2 { eprintln!("random.randFloat() takes 2 args"); process::exit(1); }
            let lo = args[0].as_float(); let hi = args[1].as_float();
            Value::Float(lo + (rng() as f64 / u64::MAX as f64) * (hi - lo))
        }
        "choice" => {
            if args.len() != 1 { eprintln!("random.choice() takes 1 arg"); process::exit(1); }
            if let Value::List(ref l) = args[0] {
                let b = l.borrow();
                if b.is_empty() { eprintln!("random.choice: empty list"); process::exit(1); }
                b[(rng() as usize) % b.len()].clone()
            } else { eprintln!("random.choice: need list"); process::exit(1); }
        }
        "shuffle" => {
            if args.len() != 1 { eprintln!("random.shuffle() takes 1 arg"); process::exit(1); }
            if let Value::List(ref l) = args[0] {
                let mut b = l.borrow_mut();
                for i in (1..b.len()).rev() { let j = (rng() as usize) % (i+1); b.swap(i,j); }
                Value::Null
            } else { eprintln!("random.shuffle: need list"); process::exit(1); }
        }
        "seed" => Value::Null,
        _ => { eprintln!("random.{}() not found", name); process::exit(1); }
    }
}

fn eval_regex_builtin(name: &str, args: Vec<Value>) -> Value {
    fn cmatch(pat: &[char], pi: usize, c: char) -> bool {
        if pat[pi] == '.' { return true; }
        if pat[pi] == '\\' && pi+1 < pat.len() {
            return match pat[pi+1] {
                'd'=>c.is_ascii_digit(), 'D'=>!c.is_ascii_digit(),
                'w'=>c.is_alphanumeric()||c=='_', 'W'=>!(c.is_alphanumeric()||c=='_'),
                's'=>c.is_whitespace(), 'S'=>!c.is_whitespace(), e=>c==e,
            };
        }
        pat[pi] == c
    }
    fn step(pat: &[char], pi: usize) -> usize { if pat[pi]=='\\' && pi+1<pat.len() {2} else {1} }
    fn mhere(pat: &[char], pi: usize, txt: &[char], ti: usize) -> Option<usize> {
        if pi >= pat.len() { return Some(ti); }
        if pat[pi]=='$' && pi+1==pat.len() { return if ti==txt.len() {Some(ti)} else {None}; }
        let s = step(pat, pi);
        if pi+s < pat.len() && matches!(pat[pi+s], '*'|'+'|'?') {
            let q = pat[pi+s]; let npi = pi+s+1;
            return match q {
                '*'|'+' => {
                    let min = if q=='+' {1} else {0};
                    let mut ti2=ti; while ti2<txt.len() && cmatch(pat,pi,txt[ti2]) {ti2+=1;}
                    if ti2-ti < min {return None;}
                    for e in (ti+min..=ti2).rev() { if let Some(r)=mhere(pat,npi,txt,e){return Some(r);} }
                    None
                }
                _ => {
                    if ti<txt.len() && cmatch(pat,pi,txt[ti]) { if let Some(r)=mhere(pat,npi,txt,ti+1){return Some(r);} }
                    mhere(pat,npi,txt,ti)
                }
            };
        }
        if ti<txt.len() && cmatch(pat,pi,txt[ti]) { mhere(pat,pi+s,txt,ti+1) } else { None }
    }
    fn find(pattern: &str, text: &str) -> Option<(usize,usize)> {
        let pat: Vec<char>=pattern.chars().collect();
        let txt: Vec<char>=text.chars().collect();
        let anch = pat.first()==Some(&'^');
        let spi = if anch {1} else {0};
        for i in 0..=if anch {0} else {txt.len()} {
            if let Some(e)=mhere(&pat,spi,&txt,i) {
                return Some((txt[..i].iter().collect::<String>().len(),
                             txt[..e].iter().collect::<String>().len()));
            }
        }
        None
    }
    match name {
        "test" => {
            if args.len()!=2{eprintln!("regex.test() takes 2 args");process::exit(1);}
            Value::Int(if find(&args[0].to_string_repr(),&args[1].to_string_repr()).is_some(){1}else{0})
        }
        "match" => {
            if args.len()!=2{eprintln!("regex.match() takes 2 args");process::exit(1);}
            let t=args[1].to_string_repr();
            match find(&args[0].to_string_repr(),&t){Some((s,e))=>Value::Str(t[s..e].to_string()),None=>Value::Null}
        }
        "findAll" => {
            if args.len()!=2{eprintln!("regex.findAll() takes 2 args");process::exit(1);}
            let pat=args[0].to_string_repr(); let txt=args[1].to_string_repr();
            let mut res=Vec::new(); let mut off=0;
            while off<=txt.len() {
                match find(&pat,&txt[off..]) {
                    None=>break,
                    Some((s,e))=>{ if s==e{off+=1;continue;} res.push(Value::Str(txt[off+s..off+e].to_string())); off+=e; }
                }
            }
            Value::List(Rc::new(RefCell::new(res)))
        }
        "replace" => {
            if args.len()!=3{eprintln!("regex.replace() takes 3 args");process::exit(1);}
            let pat=args[0].to_string_repr(); let rep=args[1].to_string_repr(); let txt=args[2].to_string_repr();
            let mut out=String::new(); let mut off=0;
            loop {
                match find(&pat,&txt[off..]) {
                    None=>{out.push_str(&txt[off..]);break;}
                    Some((s,e))=>{
                        out.push_str(&txt[off..off+s]); out.push_str(&rep);
                        if s==e { if let Some(c)=txt[off+e..].chars().next(){out.push(c);off+=e+c.len_utf8();}else{break;} }
                        else{off+=e;}
                    }
                }
            }
            Value::Str(out)
        }
        "split" => {
            if args.len()!=2{eprintln!("regex.split() takes 2 args");process::exit(1);}
            let pat=args[0].to_string_repr(); let txt=args[1].to_string_repr();
            let mut parts=Vec::new(); let mut off=0;
            loop {
                match find(&pat,&txt[off..]) {
                    None=>{parts.push(Value::Str(txt[off..].to_string()));break;}
                    Some((s,e))=>{parts.push(Value::Str(txt[off..off+s].to_string())); off+=if s==e{s+1}else{e};}
                }
            }
            Value::List(Rc::new(RefCell::new(parts)))
        }
        _ => { eprintln!("regex.{}() not found", name); process::exit(1); }
    }
}

fn eval_gui_builtin(name: &str, args: Vec<Value>) -> Value {
    match name {
        "alert"|"msgbox" => {
            let (title,msg) = match args.len() {
                1 => ("Payjar".to_string(), args[0].to_string_repr()),
                2 => (args[0].to_string_repr(), args[1].to_string_repr()),
                _ => { eprintln!("gui.{}() takes 1 or 2 args",name); process::exit(1); }
            };
            let ok = Command::new("zenity").args(["--info","--title",&title,"--text",&msg]).status().map(|s|s.success()).unwrap_or(false)
                || Command::new("osascript").arg("-e").arg(format!("display dialog \"{}\" with title \"{}\" buttons {{\"OK\"}} default button \"OK\"",msg.replace('"',"\\\""),title.replace('"',"\\\"")))
                    .status().map(|s|s.success()).unwrap_or(false);
            if !ok { eprintln!("[gui] {}: {}",title,msg); }
            Value::Null
        }
        "confirm" => {
            if args.len()!=1{eprintln!("gui.confirm() takes 1 arg");process::exit(1);}
            let msg=args[0].to_string_repr();
            let ok = Command::new("zenity").args(["--question","--text",&msg]).status().map(|s|s.success()).unwrap_or(false)
                || Command::new("osascript").arg("-e").arg(format!("display dialog \"{}\" buttons {{\"Cancel\",\"OK\"}} default button \"OK\"",msg.replace('"',"\\\"")))
                    .status().map(|s|s.success()).unwrap_or(false);
            Value::Int(if ok{1}else{0})
        }
        "prompt" => {
            if args.len()!=1{eprintln!("gui.prompt() takes 1 arg");process::exit(1);}
            let msg=args[0].to_string_repr();
            if let Ok(o)=Command::new("zenity").args(["--entry","--text",&msg]).output() {
                if o.status.success() { let mut s=String::from_utf8_lossy(&o.stdout).to_string(); if s.ends_with('\n'){s.pop();} return Value::Str(s); }
            }
            Value::Null
        }
        "notify" => {
            let (title,msg) = match args.len() {
                1 => ("Payjar".to_string(), args[0].to_string_repr()),
                2 => (args[0].to_string_repr(), args[1].to_string_repr()),
                _ => { eprintln!("gui.notify() takes 1 or 2 args",); process::exit(1); }
            };
            Command::new("notify-send").args([&title,&msg]).status().ok();
            Value::Null
        }
        _ => { eprintln!("gui.{}() not found", name); process::exit(1); }
    }
}

fn eval_builtin_or_func(
    env: &mut Env,
    name: &str,
    args: Vec<Value>,
    self_obj: Option<Rc<RefCell<PJObject>>>,
) -> Value {
    match name {
        "toStr" => {
            if args.len() != 1 { eprintln!("toStr() takes 1 arg"); process::exit(1); }
            Value::Str(args[0].to_string_repr())
        }
        "toInt" => {
            if args.len() != 1 { eprintln!("toInt() takes 1 arg"); process::exit(1); }
            match &args[0] {
                Value::Str(s) => Value::Int(s.parse().unwrap_or(0)),
                v => Value::Int(v.as_int()),
            }
        }
        "toFloat" => {
            if args.len() != 1 { eprintln!("toFloat() takes 1 arg"); process::exit(1); }
            Value::Float(args[0].as_float())
        }
        "pow" => {
            if args.len() != 2 { eprintln!("pow() takes 2 args"); process::exit(1); }
            if let (Value::Int(base), Value::Int(exp)) = (&args[0], &args[1]) {
                if *exp >= 0 {
                    return Value::Int(base.pow(*exp as u32));
                }
            }
            Value::Float(args[0].as_float().powf(args[1].as_float()))
        }
        "print" => {
            if args.len() > 1 { eprintln!("print() takes 0 or 1 arg"); process::exit(1); }
            if let Some(v) = args.get(0) { v.print(); }
            Value::Null
        }
        "println" => {
            if args.len() > 1 { eprintln!("println() takes 0 or 1 arg"); process::exit(1); }
            if let Some(v) = args.get(0) { v.print(); }
            println!();
            Value::Null
        }
        "len" => {
            if args.len() != 1 { eprintln!("len() takes 1 arg"); process::exit(1); }
            match &args[0] {
                Value::Str(s) => Value::Int(s.len() as i64),
                Value::List(l) => Value::Int(l.borrow().len() as i64),
                _ => Value::Int(0),
            }
        }
        "strLen" => {
            if args.len() != 1 { eprintln!("strLen() takes 1 arg"); process::exit(1); }
            match &args[0] {
                Value::Str(s) => Value::Int(s.len() as i64),
                _ => { eprintln!("strLen() takes a string"); process::exit(1); }
            }
        }
        "charAt" => {
            if args.len() != 2 { eprintln!("charAt() takes 2 args"); process::exit(1); }
            if let Value::Str(s) = &args[0] {
                let chars: Vec<char> = s.chars().collect();
                let mut i = args[1].as_int();
                if i < 0 { i = chars.len() as i64 + i; }
                if i < 0 || i >= chars.len() as i64 { eprintln!("charAt: index out of range"); process::exit(1); }
                Value::Str(chars[i as usize].to_string())
            } else { eprintln!("charAt: first arg must be string"); process::exit(1); }
        }
        "strSlice" => {
            if args.len() < 2 { eprintln!("strSlice() takes 2 or 3 args"); process::exit(1); }
            if let Value::Str(s) = &args[0] {
                let chars: Vec<char> = s.chars().collect();
                let slen = chars.len() as i64;
                let mut start = args[1].as_int();
                let mut end = if args.len() >= 3 { args[2].as_int() } else { slen };
                if start < 0 { start = slen + start; }
                if end < 0 { end = slen + end; }
                start = start.clamp(0, slen);
                end = end.clamp(0, slen);
                if start > end { start = end; }
                Value::Str(chars[start as usize..end as usize].iter().collect())
            } else { eprintln!("strSlice: first arg must be string"); process::exit(1); }
        }
        "strContains" => {
            if args.len() != 2 { eprintln!("strContains() takes 2 args"); process::exit(1); }
            if let (Value::Str(s), Value::Str(sub)) = (&args[0], &args[1]) {
                Value::Int(if s.contains(sub.as_str()) { 1 } else { 0 })
            } else { eprintln!("strContains: requires string args"); process::exit(1); }
        }
        "strReplace" => {
            if args.len() != 3 { eprintln!("strReplace() takes 3 args"); process::exit(1); }
            if let (Value::Str(s), Value::Str(from), Value::Str(to)) = (&args[0], &args[1], &args[2]) {
                Value::Str(s.replace(from.as_str(), to.as_str()))
            } else { eprintln!("strReplace: requires string args"); process::exit(1); }
        }
        "strSplit" => {
            if args.len() != 2 { eprintln!("strSplit() takes 2 args"); process::exit(1); }
            if let (Value::Str(s), Value::Str(delim)) = (&args[0], &args[1]) {
                let items: Vec<Value> = if delim.is_empty() {
                    s.chars().map(|c| Value::Str(c.to_string())).collect()
                } else {
                    s.split(delim.as_str()).map(|p| Value::Str(p.to_string())).collect()
                };
                Value::List(Rc::new(RefCell::new(items)))
            } else { eprintln!("strSplit: requires string args"); process::exit(1); }
        }
        "strTrim" => {
            if args.len() != 1 { eprintln!("strTrim() takes 1 arg"); process::exit(1); }
            if let Value::Str(s) = &args[0] { Value::Str(s.trim().to_string()) }
            else { eprintln!("strTrim: requires string"); process::exit(1); }
        }
        "strStartsWith" => {
            if args.len() != 2 { eprintln!("strStartsWith() takes 2 args"); process::exit(1); }
            if let (Value::Str(s), Value::Str(pre)) = (&args[0], &args[1]) {
                Value::Int(if s.starts_with(pre.as_str()) { 1 } else { 0 })
            } else { eprintln!("strStartsWith: requires string args"); process::exit(1); }
        }
        "strEndsWith" => {
            if args.len() != 2 { eprintln!("strEndsWith() takes 2 args"); process::exit(1); }
            if let (Value::Str(s), Value::Str(suf)) = (&args[0], &args[1]) {
                Value::Int(if s.ends_with(suf.as_str()) { 1 } else { 0 })
            } else { eprintln!("strEndsWith: requires string args"); process::exit(1); }
        }
        "readFile" => {
            if args.len() != 1 { eprintln!("readFile() takes 1 arg"); process::exit(1); }
            if let Value::Str(path) = &args[0] {
                Value::Str(fs::read_to_string(path).unwrap_or_default())
            } else { eprintln!("readFile: requires string path"); process::exit(1); }
        }
        "readi" => {
            if args.len() != 1 { eprintln!("readi() takes 1 arg"); process::exit(1); }
            let prompt = args[0].to_string_repr();
            print!("{}", prompt);
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).ok();
            let line = line.trim().to_string();
            Value::Int(line.parse().unwrap_or(0))
        }
        "readf" => {
            if args.len() != 1 { eprintln!("readf() takes 1 arg"); process::exit(1); }
            let prompt = args[0].to_string_repr();
            print!("{}", prompt);
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).ok();
            let line = line.trim().to_string();
            Value::Float(line.parse().unwrap_or(0.0))
        }
        "writeFile" => {
            if args.len() != 2 { eprintln!("writeFile() takes 2 args"); process::exit(1); }
            if let Value::Str(path) = &args[0] {
                let content = args[1].to_string_repr();
                fs::write(path, content).unwrap_or_else(|_| {
                    eprintln!("writeFile: cannot write '{}'", path); process::exit(1);
                });
                Value::Null
            } else { eprintln!("writeFile: requires string path"); process::exit(1); }
        }
        "appendFile" => {
            if args.len() != 2 { eprintln!("appendFile() takes 2 args"); process::exit(1); }
            if let Value::Str(path) = &args[0] {
                use std::io::Write as _;
                let content = args[1].to_string_repr();
                let mut f = std::fs::OpenOptions::new().append(true).create(true).open(path)
                    .unwrap_or_else(|_| { eprintln!("appendFile: cannot open '{}'", path); process::exit(1); });
                f.write_all(content.as_bytes()).ok();
                Value::Null
            } else { eprintln!("appendFile: requires string path"); process::exit(1); }
        }
        "exit" => {
            let code = if !args.is_empty() { args[0].as_int() as i32 } else { 0 };
            process::exit(code);
        }
        "listLen" => {
            if args.len() != 1 { eprintln!("listLen() takes 1 arg"); process::exit(1); }
            match &args[0] {
                Value::List(l) => Value::Int(l.borrow().len() as i64),
                _ => Value::Int(0),
            }
        }
        "range" => {
            let (start, end, step) = match args.len() {
                1 => (0, args[0].as_int(), 1),
                2 => (args[0].as_int(), args[1].as_int(), 1),
                3 => (args[0].as_int(), args[1].as_int(), args[2].as_int()),
                _ => { eprintln!("range() takes 1–3 args"); process::exit(1); }
            };
            if step == 0 { eprintln!("range: step cannot be 0"); process::exit(1); }
            let mut items = Vec::new();
            let mut i = start;
            while if step > 0 { i < end } else { i > end } {
                items.push(Value::Int(i));
                i += step;
            }
            Value::List(Rc::new(RefCell::new(items)))
        }
        "typeOf" => {
            if args.len() != 1 { eprintln!("typeOf() takes 1 arg"); process::exit(1); }
            let s = match &args[0] {
                Value::Int(_) => "int",
                Value::Float(_) => "float",
                Value::Str(_) => "str",
                Value::Null => "null",
                Value::Object(_) => "object",
                Value::List(_) => "list",
                Value::Comp(_) => "complex",
            };
            Value::Str(s.to_string())
        }
        //regex stuff
        "match"=>{
            if args.len()!=2{eprintln!("match() takes 2 aarguments");process::exit(1);}
            let p=args[0].clone().to_string_repr();
            let s=args[1].clone().to_string_repr();
            
            Value::Str("null".to_string())
        }
        _ => {
            // math functions available as bare calls too
            if matches!(name,
                "abs" | "sqrt" | "cbrt" | "floor" | "ceil" | "round" |
                "sum" | "min" | "max" | "ln" | "log" | "log2" | "log10" |
                "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" |
                "sinh" | "cosh" | "tanh" |
                "exp" | "hypot" | "sign" | "clamp" | "lerp" |
                "isNan" | "isInf" | "toRad" | "toDeg" | "trunc" | "fract" |
                "gcd" | "lcm"
            ) {
                return eval_math_builtin(name, args);
            }
            // random functions available as bare calls after `import random;`
            if matches!(name, "randint"|"randFloat"|"choice"|"shuffle"|"seed") {
                return eval_random_builtin(name, args);
            }
            // user-defined function
            let func = env.funcs.get(name).cloned();
            if let Some(f) = func {
                return call_func_with_vals(env, &f, args, None);
            }
            // method on self?
            if let Some(ref so) = self_obj {
                let method = so.borrow().find_method(name).cloned();
                if let Some(m) = method {
                    return call_func_with_vals(env, &m, args, Some(Rc::clone(so)));
                }
            }
            eprintln!("Runtime Error: Undefined function '{}'", name);
            process::exit(1);
        }
    }
}

fn run(env: &mut Env, program: &AstNode) {
    // First pass: register funcs and classes
    for stmt in &program.body {
        if stmt.typ == NodeType::FuncDef {
            env.funcs.insert(stmt.name.clone().unwrap(), stmt.clone());
        } else if stmt.typ == NodeType::ClassDef {
            env.classes.insert(stmt.name.clone().unwrap(), stmt.clone());
        }
    }
    // Second pass: execute
    for stmt in &program.body {
        if stmt.typ != NodeType::FuncDef && stmt.typ != NodeType::ClassDef {
            if let Signal::Return(_) = visit_stmt(env, stmt, None) { break; }
        }
    }
}

// ============================================================
// PACKAGE SYSTEM
// ============================================================

fn detect_package(tokens: &[Token]) -> Option<String> {
    if tokens.len() >= 3
        && tokens[0].typ == TokenType::Package
        && tokens[2].typ == TokenType::Semicolon
    {
        Some(tokens[1].value.clone())
    } else {
        None
    }
}

/// Load a single .pj file that starts with `package <name>;` into env.
/// Used by pjrt (flat directory scan).
fn load_package_file(env: &mut Env, path: &str, debug: bool, target_pkg: &str) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => {
            if debug { println!("[pjrt] Skipping '{}' (cannot open)", path); }
            return;
        }
    };
    let tokens = tokenize(&src);
    // pjrt forbids imports inside any file it loads
    if tokens.iter().any(|t| t.typ == TokenType::Import) {
        eprintln!("pjrt Error: file '{}' contains an import statement. pjrt does not support imports.", path);
        process::exit(1);
    }
    let pkg_name = match detect_package(&tokens) {
        Some(p) => p,
        None => {
            if debug { println!("[pjrt] Skipping '{}' (no package declaration)", path); }
            return;
        }
    };
    if pkg_name != target_pkg {
        if debug { println!("[pjrt] Skipping '{}' (package '{}' != target '{}')", path, pkg_name, target_pkg); }
        return;
    }
    if debug { println!("[pjrt] Loading package '{}' from '{}'", pkg_name, path); }
    let mut parser = Parser::new(tokens);
    let pkg_ast = parser.parse_package_file();
    for node in &pkg_ast.body {
        if node.typ == NodeType::FuncDef {
            env.funcs.insert(node.name.clone().unwrap(), node.clone());
        } else if node.typ == NodeType::ClassDef {
            env.classes.insert(node.name.clone().unwrap(), node.clone());
        }
    }
}

/// Load a folder-based package: <pkg_name>/init.pj
/// Used by `import <pkg_name>;` in pjc mode.
/// If alias is Some("m"), functions/classes are NOT added to global env directly;
/// instead a special namespace object is stored under the alias name in scope.
/// If alias is None, everything is loaded flat into global env (original behaviour).
fn load_folder_package(env: &mut Env, pkg_name: &str, alias: Option<&str>, debug: bool) {
    // Built-in modules (math, io, sqllite3) don't live on disk.
    // - `import math as m;`  → alias already registered by register_builtin_modules under
    //   the canonical name; just add a second binding for the alias so m.sqrt() works.
    // - `import math;` / `from math import *;` → bare math functions are always available
    //   as globals (eval_builtin_or_func fallback), so nothing extra is needed.
    if matches!(pkg_name, "math"|"io"|"sqllite3"|"random"|"regex"|"gui") {
        if let Some(alias_name) = alias {
            // Retrieve the pre-registered object and bind it under the alias too.
            let obj_val = Scope::get(&env.scope, pkg_name);
            // Avoid re-declaring if alias == canonical name
            if alias_name != pkg_name {
                Scope::set_declare(&env.scope, alias_name, obj_val, false);
            }
            if debug { println!("[import] Builtin module '{}' aliased as '{}'", pkg_name, alias_name); }
        } else {
            if debug { println!("[import] Builtin module '{}' loaded flat (builtins always available)", pkg_name); }
        }
        return;
    }

    let init_path = format!("{}/init.pj", pkg_name);
    let src = fs::read_to_string(&init_path).unwrap_or_else(|_| {
        eprintln!("Import Error: cannot open '{}' for package '{}'", init_path, pkg_name);
        process::exit(1);
    });
    let tokens = tokenize(&src);
    // The init.pj must declare the correct package
    match detect_package(&tokens) {
        Some(ref p) if p == pkg_name => {}
        Some(ref p) => {
            eprintln!("Import Error: '{}' declares package '{}' but expected '{}'", init_path, p, pkg_name);
            process::exit(1);
        }
        None => {
            eprintln!("Import Error: '{}' has no package declaration", init_path);
            process::exit(1);
        }
    }
    if debug { println!("[import] Loading package '{}' from '{}'", pkg_name, init_path); }
    let mut parser = Parser::new(tokens);
    let pkg_ast = parser.parse_package_file();

    match alias {
        None => {
            // Flat load — functions and classes go straight into global env
            for node in &pkg_ast.body {
                if node.typ == NodeType::FuncDef {
                    env.funcs.insert(node.name.clone().unwrap(), node.clone());
                } else if node.typ == NodeType::ClassDef {
                    env.classes.insert(node.name.clone().unwrap(), node.clone());
                }
            }
        }
        Some(alias_name) => {
            // Namespaced load — build a PJObject whose fields are the callable names,
            // and whose class_def holds all the methods so member-call dispatch works.
            // Strategy: register all funcs/classes into env normally (so they work when
            // called internally), AND create a namespace object that re-exports them as
            // methods so `alias.func(args)` works via member-call dispatch.
            //
            // We create a synthetic class def node that has all the package functions
            // as methods, then instantiate it as the alias variable in scope.
            let mut synthetic_class = AstNode::new(NodeType::ClassDef);
            synthetic_class.name = Some(pkg_name.to_string());

            for node in &pkg_ast.body {
                if node.typ == NodeType::FuncDef {
                    // Also register globally so internal package calls work
                    env.funcs.insert(node.name.clone().unwrap(), node.clone());
                    // Add as method on the namespace object (mark as method)
                    let mut method = node.clone();
                    method.is_method = false; // no self param
                    synthetic_class.methods.push(method);
                } else if node.typ == NodeType::ClassDef {
                    env.classes.insert(node.name.clone().unwrap(), node.clone());
                }
            }

            // Create an instance of the synthetic class (no constructor, no fields)
            let ns_obj = Rc::new(RefCell::new(PJObject {
                class_name: pkg_name.to_string(),
                fields: Vec::new(),
                class_def: synthetic_class,
            }));
            // Declare alias variable in current scope
            Scope::set_declare(&env.scope, alias_name, Value::Object(ns_obj), false);
        }
    }
}

pub fn interpret_file(path: &str, debug: bool) {
    let src = fs::read_to_string(path).unwrap_or_else(|_| {
        eprintln!("Error: Cannot open file '{}'", path); process::exit(1);
    });
    let tokens = tokenize(&src);
    if let Some(pkg) = detect_package(&tokens) {
        eprintln!("Error: '{}' is a package file (package {}).\nCannot run a package with pjc — use pjrt instead.", path, pkg);
        process::exit(1);
    }
    if debug {
        println!("=== TOKENS ===");
        for (i, t) in tokens.iter().enumerate() {
            println!("  [{}] type={:?} value={}", i, t.typ, t.value);
        }
        println!("==============");
    }
    // Collect import names from token stream (before parsing) so we can
    // pre-load folder packages into env before the parser runs.
    let imports = collect_imports(&tokens);
    let mut env = Env::new();
    for (pkg_name, alias) in &imports {
        load_folder_package(&mut env, pkg_name, alias.as_deref(), debug);
    }
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program();
    if debug { println!("=== Parse OK ==="); }
    run(&mut env, &prog);
}
pub fn interpret(code:&str, debug: bool) {
    let tokens = tokenize(code);
    if let Some(pkg) = detect_package(&tokens) {
        eprintln!("Error: '{}' is a package file (package {}).\nCannot run a package with pjc — use pjrt instead.", code, pkg);
        process::exit(1);
    }
    if debug {
        println!("=== TOKENS ===");
        for (i, t) in tokens.iter().enumerate() {
            println!("  [{}] type={:?} value={}", i, t.typ, t.value);
        }
        println!("==============");
    }
    // Collect import names from token stream (before parsing) so we can
    // pre-load folder packages into env before the parser runs.
    let imports = collect_imports(&tokens);
    let mut env = Env::new();
    for (pkg_name, alias) in &imports {
        load_folder_package(&mut env, pkg_name, alias.as_deref(), debug);
    }
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program();
    if debug { println!("=== Parse OK ==="); }
    run(&mut env, &prog);
}

/// Scan a token stream and return the list of imported package names in order.
/// Supports:
///   import math;          -> ("math", None)        flat / bare-call mode
///   import math as m;     -> ("math", Some("m"))
///   from math import *;   -> ("math", None)        flat / bare-call mode
fn collect_imports(tokens: &[Token]) -> Vec<(String, Option<String>)> {
    let mut imports = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        // from <pkg> import * ;
        if tokens[i].typ == TokenType::Identifier && tokens[i].value == "from" {
            if i + 3 < tokens.len()
                && tokens[i + 2].typ == TokenType::Import
                && tokens[i + 3].typ == TokenType::Multiply
            {
                let pkg = tokens[i + 1].value.clone();
                imports.push((pkg, None));
                i += 4; // skip: from <pkg> import *
                if i < tokens.len() && tokens[i].typ == TokenType::Semicolon { i += 1; }
                continue;
            }
        }
        if tokens[i].typ == TokenType::Import {
            i += 1; // skip `import`
            if i < tokens.len() {
                let pkg = tokens[i].value.clone();
                i += 1;
                // optional `as <alias>`
                let alias = if i < tokens.len() && tokens[i].typ == TokenType::As {
                    i += 1; // skip `as`
                    let a = tokens[i].value.clone();
                    i += 1; // skip alias
                    Some(a)
                } else {
                    None
                };
                imports.push((pkg, alias));
            }
        } else {
            i += 1;
        }
    }
    imports
}

pub fn pjrt_run(debug: bool, pkgn: &str,skip:&str) {
    let mut env = Env::new();
    let entries = fs::read_dir(".").unwrap_or_else(|_| {
        eprintln!("pjrt Error: cannot open current directory"); process::exit(1);
    });
    let mut pkg_count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name==skip{continue;}
        if name == "main.pj" { continue; }
        if !name.ends_with(".pj") { continue; }
        let before = env.funcs.len() + env.classes.len();
        load_package_file(&mut env, &name, debug, pkgn);
        if env.funcs.len() + env.classes.len() > before { pkg_count +=1; }
    }
    if pkg_count == 0 {
        eprintln!("pjrt Error: package '{}' not found in current directory", pkgn);
        process::exit(1);
    }
    if debug { println!("[pjrt] Loaded file(s) for package: {}.", pkgn); }
    let src = fs::read_to_string("main.pj").unwrap_or_else(|_| {
        eprintln!("Error: Cannot open file 'main.pj'"); process::exit(1);
    });
    let tokens = tokenize(&src);
    if let Some(p) = detect_package(&tokens) {
        eprintln!("pjrt Error: main.pj has a package declaration (package {}).\nmain.pj must not declare a package.", p);
        process::exit(1);
    }
    if tokens.iter().any(|t| t.typ == TokenType::Import) {
        eprintln!("pjrt Error: main.pj contains an import statement. pjrt does not support imports — use pjc instead.");
        process::exit(1);
    }
    if debug {
        println!("=== TOKENS (main.pj) ===");
        for (i, t) in tokens.iter().enumerate() {
            println!("  [{}] type={:?} value={}", i, t.typ, t.value);
        }
        println!("========================");
    }
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program();
    if debug { println!("=== Parse OK (main.pj) ==="); }
    run(&mut env, &prog);
}
pub fn autorun(debug: bool,skip:&str) {
    let mut skips:Vec<String>=vec![];
    for c in skip.chars(){
      let mut a:String="".to_string();
      if c=='[' || c==']'{continue;}
      while c!=',' && c!='\0'{a+=&c.to_string();}
      skips.push(a);
     }
    let entries = fs::read_dir(".").unwrap_or_else(|_| {
        eprintln!("Error: cannot open current directory"); process::exit(1);
    });
    let mut i=0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name==skips[i]{if !i>=skips.len(){i+=1;}continue;}
        if name.ends_with(".pj") {
            interpret_file(&name, debug);
        }
    }
}

pub fn print_usage_pjc() {
    println!("pjc usage:");
    println!("  help              show this help page");
    println!("  autorun           autorun all pj files");
    println!("    -s <file>       skip <file> in autorun");
    println!("  -d <file>.pj      enable debug");
    println!("  <file>.pj         interpret <file>.pj");
}

pub fn print_usage_pjrt() {
    println!("pjrt (PayJar RunTime) usage:");
    println!("  help              show this help page");
    println!("  run <package>     load package from current dir and run main.pj");
    println!("  -d                enable debug mode");
    println!("  -s <file>         skip <file>");
    println!();
    println!("pjrt scans all .pj files in the current directory for 'package <n>;'");
    println!("declarations and loads them before running main.pj.");
    println!("Files without a package declaration are ignored.");
}