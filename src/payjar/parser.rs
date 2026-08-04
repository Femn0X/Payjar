use std::process;
use std::collections::HashMap;
use super::{Token, TokenType};
use super::ast::{AstNode, NodeType};

pub struct Parser {
    tokens: Vec<Token>,
    idx: usize,
    var_types: HashMap<String, String>,
}

pub fn to_str(a: NodeType) -> String {
    format!("{:#?}", a)
}

fn check_node_type(expected_type: &str, node: &AstNode) -> bool {
    let base_type = expected_type.trim_end_matches('*');
    if base_type == "Any" {
        return true;
    }
    match node.typ {
        NodeType::Int => base_type == "Int",
        NodeType::Float => base_type == "Float",
        NodeType::Str | NodeType::TemplateStr | NodeType::RegexString => base_type == "Str",
        NodeType::Imaginary => base_type == "Complex",
        // Bugfix: this used to compare `base_type` (the element type with
        // any trailing '*' already stripped off, e.g. "Int") against the
        // literal string "Array" -- which is never equal to "Int", "Float",
        // etc., so *every* properly-declared array/pointer-depth variable
        // (`let xs: Int* = [...]`) failed this check regardless of what it
        // was assigned. What actually needs checking here is just whether
        // the *declared* type is array-shaped at all (the bare `Array`
        // keyword, or any `Type*` pointer-depth suffix) -- the precise
        // element type and nesting depth are already checked properly,
        // with much better error messages, right after this call returns.
        NodeType::Array | NodeType::Generic | NodeType::Dict => {
            expected_type == "Array" || expected_type.ends_with('*')
        }
        _ => true, // Complex expressions/runtime-evaluated nodes bypass static parser-level checking
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, idx: 0, var_types: HashMap::new() }
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
    pub fn parse_program(&mut self) -> AstNode {
        let mut prog = AstNode::new(NodeType::MainDef);
        let mut found_main = false;
        loop {
            let cur_typ = self.cur().typ.clone();
            match cur_typ {
                TokenType::Eof => break,
                TokenType::Import => { self.parse_import(); }
                TokenType::Identifier if self.cur().value == "from" => {
                    self.advance();
                    self.advance();
                    self.eat(TokenType::Import);
                    self.eat(TokenType::Multiply);
                    self.eat(TokenType::Semicolon);
                }
                TokenType::Def => {
                    prog.body.push(self.parse_function_definition(false));
                }
                TokenType::Enum => {
                    prog.body.push(self.parse_enum_definition());
                }
                TokenType::Async => {
                    self.advance();
                    let mut f = self.parse_function_definition(false);
                    f.set_is_async(true);
                    prog.body.push(f);
                }
                TokenType::Public if self.peek(1).typ == TokenType::Class
                    && self.peek(2).typ == TokenType::Main =>
                {
                    if found_main {
                        eprintln!("Syntax Error: only one 'public class main(@self){{...}}' is allowed per file");
                        process::exit(1);
                    }
                    found_main = true;
                    self.advance(); // public
                    self.eat(TokenType::Class);
                    self.eat(TokenType::Main);
                    self.eat(TokenType::LParen);
                    self.eat(TokenType::At);
                    self.eat(TokenType::Self_);
                    self.eat(TokenType::RParen);
                    self.eat(TokenType::LBrace);
                    let mut inner = self.parse_body();
                    prog.body.append(&mut inner);
                    self.eat(TokenType::RBrace);
                }
                TokenType::Public => {
                    self.advance();
                    if self.cur().typ != TokenType::Class {
                        eprintln!("Syntax Error: expected 'class' after 'public'"); process::exit(1);
                    }
                    prog.body.push(self.parse_class_definition());
                }
                TokenType::Private => {
                    self.advance();
                    if self.cur().typ != TokenType::Class {
                        eprintln!("Syntax Error: expected 'class' after 'private'"); process::exit(1);
                    }
                    let mut c = self.parse_class_definition();
                    c.set_is_private(true);
                    prog.body.push(c);
                }
                TokenType::Class => {
                    eprintln!("Syntax Error: class declarations must start with 'public' or 'private', got bare 'class'");
                    process::exit(1);
                }
                _ => {
                    eprintln!(
                        "Syntax Error: expected 'import', a top-level 'func'/'class', or 'public class main(@self){{...}}', got {:?} ('{}')",
                        self.cur().typ, self.cur().value
                    );
                    process::exit(1);
                }
            }
        }
        if !found_main {
            eprintln!("Syntax Error: no 'public class main(@self){{...}}' entry point found in file");
            process::exit(1);
        }
        prog
    }

    fn parse_import(&mut self) -> AstNode {
        self.eat(TokenType::Import);
        let pack = self.cur().value.clone();
        self.advance();
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
        n.str_val = Some(pack);
        n.name = alias;
        n
    }

    pub fn parse_package_file(&mut self) -> AstNode {
        self.eat(TokenType::Package);
        self.advance();
        self.eat(TokenType::Semicolon);
        let mut pkg = AstNode::new(NodeType::MainDef);
        while self.cur().typ != TokenType::Eof {
            match self.cur().typ {
                TokenType::Def => { pkg.body.push(self.parse_function_definition(false)); }
                TokenType::Async => {
                    self.advance();
                    let mut f = self.parse_function_definition(false);
                    f.set_is_async(true);
                    pkg.body.push(f);
                }
                TokenType::Public => {
                    self.advance();
                    if self.cur().typ != TokenType::Class {
                        eprintln!("Package Error: expected 'class' after 'public'"); process::exit(1);
                    }
                    pkg.body.push(self.parse_class_definition());
                }
                TokenType::Private => {
                    self.advance();
                    if self.cur().typ != TokenType::Class {
                        eprintln!("Package Error: expected 'class' after 'private'"); process::exit(1);
                    }
                    let mut c = self.parse_class_definition();
                    c.set_is_private(true);
                    pkg.body.push(c);
                }
                TokenType::Class => {
                    eprintln!("Package Error: class declarations must start with 'public' or 'private', got bare 'class'");
                    process::exit(1);
                }
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
    fn parse_enum_definition(&mut self)->AstNode{
        self.eat(TokenType::Enum);
        let mut n=AstNode::new(NodeType::EnumDef);
        n.name=Some(self.cur().value.clone());
        self.eat(TokenType::Identifier);
        self.eat(TokenType::LBrace);
        // Bugfix: this used to assign the AstNode returned by an
        // EnumBody-wrapping helper straight into `n.body`, but `body` is a
        // `Vec<AstNode>` and the helper returned a single `AstNode` -- a
        // type mismatch that meant the whole file failed to build, so no
        // enum ever actually ran. Variant names are simple identifiers with
        // no sub-expressions, so they're stored directly as `n.params`
        // (a `Vec<String>`) instead of needing a wrapper node at all.
        n.params=self.parse_enum_variants();
        self.eat(TokenType::RBrace);
        n
    }
    fn parse_enum_variants(&mut self)->Vec<String>{
      let mut variants = Vec::new();
      while self.cur().typ!=TokenType::RBrace && self.cur().typ!=TokenType::Eof{
       variants.push(self.cur().value.clone());
       self.eat(TokenType::Identifier);
       // Trailing comma is optional: `enum X { A, B, C }` and
       // `enum X { A, B, C, }` are both valid.
       if self.cur().typ==TokenType::Comma{
           self.advance();
       } else {
           break;
       }
      }
      variants
    }
    fn parse_function_definition(&mut self, is_method: bool) -> AstNode {
        self.eat(TokenType::Def);
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::LParen);
        let mut n = AstNode::new(NodeType::FuncDef);
        let mut t: Vec<String> = vec![];
        n.name = Some(name);
        n.set_is_method(is_method);
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
        let ret_type = self.parse_type();
        self.eat(TokenType::LBrace);
        n.body = self.parse_body();
        self.eat(TokenType::RBrace);
        n.own_type = ret_type;
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
            let is_private = match self.cur().typ {
                TokenType::Public => { self.advance(); false }
                TokenType::Private => { self.advance(); true }
                _ => {
                    eprintln!(
                        "Syntax Error: class members must start with 'public' or 'private', got '{}'",
                        self.cur().value
                    );
                    process::exit(1);
                }
            };
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
                    let _typ = self.parse_type();
                    self.eat(TokenType::Semicolon);
                    let mut fd = AstNode::new(NodeType::FieldDecl);
                    fd.name = Some(fname);
                    fd.set_is_const(is_const);
                    fd.set_is_private(is_private);
                    fd.expr = fval;
                    n.fields.push(fd);
                }
                TokenType::Def => {
                    let mut method = self.parse_function_definition(true);
                    method.set_is_private(is_private);
                    if method.name.as_deref() == Some("init") {
                        n.constructor = Some(Box::new(method));
                    } else {
                        n.methods.push(method);
                    }
                }
                TokenType::Async => {
                    self.advance();
                    let mut method = self.parse_function_definition(true);
                    method.set_is_async(true);
                    method.set_is_private(is_private);
                    if method.name.as_deref() == Some("init") {
                        eprintln!("Syntax Error: constructor 'init' cannot be async");
                        process::exit(1);
                    }
                    n.methods.push(method);
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
            TokenType::ImaginaryLiteral => {
                let mut n = AstNode::new(NodeType::Imaginary);
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
            TokenType::RegexString => {
                let mut n = AstNode::new(NodeType::RegexString);
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
                        i += 1;
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
                // `new Color.RED` -- enum constants aren't constructed, so
                // this is just sugar for the plain `Color.RED` member-access
                // form (kept so existing `new Enum.Variant` call sites, and
                // anyone coming from a language where `new` is always
                // required, keep working).
                if self.cur().typ == TokenType::Dot {
                    self.advance();
                    let variant = self.cur().value.clone();
                    self.eat(TokenType::Identifier);
                    let mut obj = AstNode::new(NodeType::VarAccess);
                    obj.name = Some(class_name);
                    let mut ma = AstNode::new(NodeType::MemberAccess);
                    ma.object = Some(Box::new(obj));
                    ma.name = Some(variant);
                    return ma;
                }
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
                        ma.set_is_call(true);
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
            TokenType::Not | TokenType::Minus => {
                self.advance();
                let operand = self.parse_primary();
                let mut n = AstNode::new(NodeType::UnaryOp);
                n.str_val = Some(if cur_typ == TokenType::Not { "not".to_string() } else { "-".to_string() });
                n.left = Some(Box::new(operand));
                n
            }
            TokenType::Await => {
                self.advance();
                let operand = self.parse_primary();
                let mut n = AstNode::new(NodeType::Await);
                n.left = Some(Box::new(operand));
                n
            }
            TokenType::Identifier => {
                let name = cur_val.clone();
                self.advance();
                if self.cur().typ == TokenType::LParen {
                    self.advance();
                    let mut n = AstNode::new(NodeType::FuncCall);
                    n.name = Some(name.clone());
                    if self.cur().typ != TokenType::RParen {
                        n.args.push(self.parse_expression());
                        while self.cur().typ == TokenType::Comma {
                            self.advance();
                            n.args.push(self.parse_expression());
                        }
                    }
                    self.eat(TokenType::RParen);
                    let mut result = n;
                    while self.cur().typ == TokenType::Dot {
                        self.advance();
                        let member = self.cur().value.clone();
                        self.eat(TokenType::Identifier);
                        let mut ma = AstNode::new(NodeType::MemberAccess);
                        ma.object = Some(Box::new(result));
                        ma.name = Some(member);
                        if self.cur().typ == TokenType::LParen {
                            ma.set_is_call(true);
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
                        result = ma;
                    }
                    return result;
                }
                if self.cur().typ == TokenType::Dot {
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
                            ma.set_is_call(true);
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
                if self.cur().typ == TokenType::LBrack {
                    self.advance();
                    let idx = self.parse_expression();
                    self.eat(TokenType::RBrack);
                    let mut arr = AstNode::new(NodeType::VarAccess);
                    arr.name = Some(name);
                    let mut n = AstNode::new(NodeType::Index);
                    n.left = Some(Box::new(arr));
                    n.right = Some(Box::new(idx));
                    return n;
                }
                let mut n = AstNode::new(NodeType::VarAccess);
                n.name = Some(name);
                n
            }
            TokenType::LBrace => {
                self.advance();
                let mut n = AstNode::new(NodeType::Dict);
                while self.cur().typ != TokenType::RBrace && self.cur().typ != TokenType::Eof {
                    let key = self.cur().value.clone();
                    match self.cur().typ {
                        TokenType::Identifier | TokenType::StringLiteral => { self.advance(); }
                        _ => {
                            eprintln!("Syntax Error: dict key must be identifier or string, got '{:?}'", self.cur().typ);
                            process::exit(1);
                        }
                    }
                    self.eat(TokenType::Colon);
                    let val = self.parse_expression();
                    let mut key_node = AstNode::new(NodeType::Str);
                    key_node.str_val = Some(key);
                    n.args.push(key_node);
                    n.args.push(val);
                    if self.cur().typ == TokenType::Comma { self.advance(); }
                }
                self.eat(TokenType::RBrace);
                n
            }
            _ => {
                eprintln!("Syntax Error: unexpected token in expression: {:?} ('{}')",
                    cur_typ, cur_val);
                process::exit(1);
            }
        }
    }

    fn parse_factor(&mut self) -> AstNode {
        self.parse_primary()
    }

    fn parse_term(&mut self) -> AstNode {
        let mut left = self.parse_factor();
        while matches!(self.cur().typ,
            TokenType::Multiply | TokenType::Divide | TokenType::Modulo)
        {
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

    fn parse_additive(&mut self) -> AstNode {
        let mut left = self.parse_term();
        while matches!(self.cur().typ, TokenType::Plus | TokenType::Minus) {
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
        let mut left = self.parse_additive();
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
            n.right = Some(Box::new(self.parse_additive()));
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

    fn parse_type(&mut self) -> String {
        let mut re = String::new();
        match self.cur().typ {
            TokenType::Array => {
                re += "Array";
                self.eat(TokenType::Array);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            TokenType::Float => {
                re += "Float";
                self.eat(TokenType::Float);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            TokenType::Int => {
                re += "Int";
                self.eat(TokenType::Int);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            TokenType::Str => {
                re += "Str";
                self.eat(TokenType::Str);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            TokenType::Null => {
                re += "Null";
                self.eat(TokenType::Null);
                if self.cur().typ == TokenType::Multiply {
                    eprintln!("TypeError: Null cannot be a pointer type"); process::exit(1);
                }
            }
            
            TokenType::ComplexType => {
                re += "Complex";
                self.eat(TokenType::ComplexType);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            // Bugfix: a type annotation naming anything other than the
            // handful of builtins above -- including `Any` (which
            // `check_node_type` already special-cased, expecting this to be
            // reachable) and any user class or enum name (`Color`,
            // `BankAccount`, ...) -- used to fall into the `_` arm below and
            // hard-exit with "TypeError: ... is not a valid type" before the
            // program ever ran. Any identifier is now accepted as a type
            // name; `check_node_type` already treats non-builtin type names
            // permissively (custom types are checked at runtime instead).
            TokenType::Identifier => {
                re += &self.cur().value.clone();
                self.eat(TokenType::Identifier);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            _ => {
                eprintln!("TypeError: {:?} is not a valid type", self.cur().value);
                process::exit(1);
            }
        }
        re
    }

    fn parse_var_decl(&mut self) -> AstNode {
        let is_const = self.cur().typ == TokenType::Const;
        self.advance();
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::Colon);
        let var_type = self.parse_type();
        self.eat(TokenType::Equal);
        let val = self.parse_expression();

        if !check_node_type(&var_type, &val) {
            eprintln!("TypeError: Variable '{}' declared as '{}' but assigned incompatible expression", name, var_type);
            process::exit(1);
        }

        let declared_depth = var_type.chars().filter(|&c| c == '*').count();
        let is_dict_node = val.typ == NodeType::Dict;
        if var_type == "Array" && !is_dict_node && matches!(val.typ, NodeType::Array | NodeType::Generic) {
            // plain Array — ok
        } else if var_type == "Array" && !is_dict_node && !matches!(val.typ, NodeType::Array | NodeType::Generic) {
            eprintln!("TypeError: '{}' declared as 'Array' but assigned a non-collection", name);
            process::exit(1);
        } else if !is_dict_node {
            fn list_depth(node: &AstNode) -> usize {
                if !matches!(node.typ, NodeType::Array | NodeType::Generic) { return 0; }
                1 + node.args.iter().map(list_depth).max().unwrap_or(0)
            }
            let actual_depth = list_depth(&val);
            if declared_depth > 0 && actual_depth == 0 {
                eprintln!("TypeError: '{}' declared as pointer type '{}' but assigned a non-list value", name, var_type);
                process::exit(1);
            }
            if declared_depth == 0 && actual_depth > 0 {
                let stars: String = "*".repeat(actual_depth);
                eprintln!("TypeError: '{}' declared as '{}' but assigned a depth-{} list — did you mean '{}{}'?",
                    name, var_type, actual_depth, var_type, stars);
                process::exit(1);
            }
            if declared_depth > 0 && actual_depth > 0 && declared_depth != actual_depth {
                let stars: String = "*".repeat(actual_depth);
                eprintln!("TypeError: '{}' declared as '{}' (depth {}) but list has depth {} — did you mean '{}{}'?",
                    name, var_type, declared_depth, actual_depth,
                    var_type.trim_end_matches('*'), stars);
                process::exit(1);
            }
        }
        self.eat(TokenType::Semicolon);
        self.var_types.insert(name.clone(), var_type.clone());
        let mut n = AstNode::new(NodeType::VarDecl);
        n.name = Some(name);
        n.set_is_const(is_const);
        n.expr = Some(Box::new(val));
        n.var_type = var_type;
        n
    }

    fn parse_assign(&mut self) -> AstNode {
        let name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        self.eat(TokenType::Equal);
        let val = self.parse_expression();

        if let Some(expected_type) = self.var_types.get(&name).cloned() {
            if !check_node_type(&expected_type, &val) {
                eprintln!("TypeError: Cannot assign expression to variable '{}' of type '{}'", name, expected_type);
                process::exit(1);
            }
        }

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

    fn parse_perror(&mut self) -> AstNode {
        self.eat(TokenType::Perror);
        self.eat(TokenType::LParen);
        let e = self.parse_expression();
        self.eat(TokenType::RParen);
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Perror);
        n.expr = Some(Box::new(e));
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
            if self.cur().typ == TokenType::PlusPlus {
                self.advance(); self.advance();
                let mut one = AstNode::new(NodeType::Int); one.int_val = 1;
                let mut var = AstNode::new(NodeType::VarAccess); var.name = Some(name.clone());
                let mut add = AstNode::new(NodeType::BinaryOp); add.str_val = Some("+".to_string());
                add.left = Some(Box::new(var)); add.right = Some(Box::new(one));
                let mut n = AstNode::new(NodeType::Assign); n.name = Some(name); n.expr = Some(Box::new(add));
                return n;
            }
            if self.cur().typ == TokenType::MinusMinus {
                self.advance(); self.advance();
                let mut one = AstNode::new(NodeType::Int); one.int_val = 1;
                let mut var = AstNode::new(NodeType::VarAccess); var.name = Some(name.clone());
                let mut sub = AstNode::new(NodeType::BinaryOp); sub.str_val = Some("-".to_string());
                sub.left = Some(Box::new(var)); sub.right = Some(Box::new(one));
                let mut n = AstNode::new(NodeType::Assign); n.name = Some(name); n.expr = Some(Box::new(sub));
                return n;
            }
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
        if !matches!(self.cur().typ, TokenType::Let | TokenType::Const) {
            eprintln!("Syntax Error: for loop expects variable declaration"); process::exit(1);
        }
        self.advance();
        let var_name = self.cur().value.clone();
        self.eat(TokenType::Identifier);
        let mut n = AstNode::new(NodeType::For);
        n.name = Some(var_name);
        if self.cur().typ == TokenType::Colon || self.cur().typ == TokenType::In {
            n.set_is_method(true);
            if self.cur().typ == TokenType::Colon {
                self.eat(TokenType::Colon);
                let var_type = self.parse_type();
                n.var_type = var_type;
            }
            self.eat(TokenType::In);
            n.object = Some(Box::new(self.parse_expression()));
            self.eat(TokenType::RParen);
            self.eat(TokenType::LBrace);
            n.body = self.parse_body();
            self.eat(TokenType::RBrace);
        } else {
            n.set_is_method(false);
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

    /// `try { ... } except (e) { ... }` -- deliberately C-styled (like
    /// C++/Java's `try { } catch (e) { }`) rather than Python's
    /// `try: ... except ... as e:`: braces delimit both blocks and the
    /// caught-exception identifier is parenthesized, consistent with how
    /// every other block construct in this language (if/while/for) looks.
    fn parse_try(&mut self) -> AstNode {
        self.eat(TokenType::Try);
        self.eat(TokenType::LBrace);
        let try_body = self.parse_body();
        self.eat(TokenType::RBrace);
        let mut n = AstNode::new(NodeType::Try);
        n.body = try_body;
        self.eat(TokenType::Except);
        self.eat(TokenType::LParen);
        n.name = Some(self.cur().value.clone());
        self.eat(TokenType::Identifier);
        self.eat(TokenType::RParen);
        self.eat(TokenType::LBrace);
        n.else_body = self.parse_body();
        self.eat(TokenType::RBrace);
        if self.cur().typ == TokenType::Finally {
            self.advance();
            self.eat(TokenType::LBrace);
            n.finally_body = self.parse_body();
            self.eat(TokenType::RBrace);
        }
        n
    }

    fn parse_throw(&mut self) -> AstNode {
        self.eat(TokenType::Throw);
        let e = self.parse_expression();
        self.eat(TokenType::Semicolon);
        let mut n = AstNode::new(NodeType::Throw);
        n.expr = Some(Box::new(e));
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
            TokenType::Await => {
                let expr = self.parse_expression();
                self.eat(TokenType::Semicolon);
                expr
            }
            TokenType::Let | TokenType::Const => self.parse_var_decl(),
            TokenType::Def => self.parse_function_definition(false),
            TokenType::Async => {
                self.advance();
                let mut f = self.parse_function_definition(false);
                f.set_is_async(true);
                f
            }
            TokenType::Public => {
                self.advance();
                if self.cur().typ == TokenType::Class { return self.parse_class_definition(); }
                eprintln!("Syntax Error: expected 'class' after 'public'"); process::exit(1);
            }
            TokenType::Private => {
                self.advance();
                if self.cur().typ == TokenType::Class {
                    let mut c = self.parse_class_definition();
                    c.set_is_private(true);
                    return c;
                }
                eprintln!("Syntax Error: expected 'class' after 'private'"); process::exit(1);
            }
            TokenType::Class => {
                eprintln!("Syntax Error: class declarations must start with 'public' or 'private', got bare 'class'");
                process::exit(1);
            }
            TokenType::If => self.parse_if(),
            TokenType::While => self.parse_while(),
            TokenType::For => self.parse_for(),
            TokenType::Return => self.parse_return_stmt(),
            TokenType::Import => self.parse_import(),
            TokenType::Try => self.parse_try(),
            TokenType::Throw => self.parse_throw(),
            TokenType::Break => {
                self.advance();
                self.eat(TokenType::Semicolon);
                AstNode::new(NodeType::Break)
            }
            TokenType::Continue => {
                self.advance();
                self.eat(TokenType::Semicolon);
                AstNode::new(NodeType::Continue)
            }
            _ => {
                if cur_typ == TokenType::Identifier && self.peek(1).typ == TokenType::LBrack {
                    let obj_name = self.cur().value.clone();
                    self.advance();
                    self.advance();
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
                                ma.set_is_call(true);
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
                        if obj.typ == NodeType::MemberAccess && obj.is_call() {
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