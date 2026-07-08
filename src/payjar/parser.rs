use std::process;
use super::{Token, TokenType};
use super::ast::{AstNode, NodeType};

pub struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

pub fn to_str(a: NodeType) -> String {
    format!("{:#?}", a)
}

pub fn all_same<T: PartialEq>(slice: &[T]) -> bool {
    if let Some(first) = slice.first() {
        slice.iter().all(|x| x == first)
    } else {
        true
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
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

    // Top-level programs may now freely mix `import` statements, top-level
    // `def` function definitions, top-level `class` (or `public class`)
    // definitions, and exactly one `public class main(@self){ ... }` entry
    // point — in any order, before or after `main`. Everything is flattened
    // into a single body; `run()` already hoists FuncDef/ClassDef nodes
    // before executing the remaining statements, so order doesn't matter.
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
                TokenType::Class => {
                    prog.body.push(self.parse_class_definition());
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
                        eprintln!("Syntax Error: expected 'class' after 'public'");
                        process::exit(1);
                    }
                    prog.body.push(self.parse_class_definition());
                }
                _ => {
                    eprintln!(
                        "Syntax Error: expected 'import', top-level 'def'/'class', or 'public class main(@self){{...}}', got {:?} ('{}')",
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
                TokenType::Import => { self.parse_import(); }
                TokenType::Identifier if self.cur().value == "from" => {
                    self.advance();
                    self.advance();
                    self.eat(TokenType::Import);
                    self.eat(TokenType::Multiply);
                    self.eat(TokenType::Semicolon);
                }
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
                    eprintln!("Package Error: only 'import' and func/class definitions allowed in package, got '{}'",
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
            TokenType::RegexString => {
                // §"pattern" — stored as a RegexString node; evaluates to a string value
                // that the regex module recognises as a pre-compiled pattern.
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
                        // ${...} can contain nested braces (e.g. a call with a
                        // block-like arg is unlikely, but nested {} in general
                        // expressions isn't, so just track depth defensively).
                        let mut depth = 0i32;
                        while i < len && !(chars[i] == '}' && depth == 0) {
                            if chars[i] == '{' { depth += 1; }
                            if chars[i] == '}' { depth -= 1; }
                            i += 1;
                        }
                        let varexpr: String = chars[vstart..i].iter().collect();
                        i += 1;
                        // Parse the interpolated text as a full expression, so
                        // things like `${square(4)}`, `${a.b}`, `${a+b}` all work,
                        // not just bare variable/single-field access.
                        let sub_tokens = super::tokenize(&varexpr);
                        let mut sub_parser = Parser::new(sub_tokens);
                        let part_node = sub_parser.parse_expression();
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
            TokenType::Identifier => {
                let name = cur_val.clone();
                self.advance();
                // function call
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
                    // chain member calls: func().method()
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
                // member access chain: obj.field / obj.method()
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
                // index: arr[i]
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
                // plain variable
                let mut n = AstNode::new(NodeType::VarAccess);
                n.name = Some(name);
                n
            }
            TokenType::LBrace => {
                // Dict literal: { key: expr, key: expr, ... }
                self.advance(); // consume '{'
                let mut n = AstNode::new(NodeType::Dict);
                while self.cur().typ != TokenType::RBrace && self.cur().typ != TokenType::Eof {
                    // key: identifier or string literal
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
                    // store key as a Str node, value as the expr; interleaved in args
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
            TokenType::Any => {
                re += "Any";
                self.eat(TokenType::Any);
                while self.cur().typ == TokenType::Multiply { self.advance(); re += "*"; }
            }
            TokenType::Identifier => {
                // A user-defined class name used as a type, e.g. `let r:Rect = new Rect(10,10);`
                re += &self.cur().value;
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
        let declared_depth = var_type.chars().filter(|&c| c == '*').count();
        let is_dict_node = val.typ == NodeType::Dict;
        // For Array (no stars) allow a dict literal
        if var_type == "Array" && !is_dict_node && matches!(val.typ, NodeType::Array | NodeType::Generic) {
            // plain Array — ok, no depth to verify
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
        if !matches!(self.cur().typ, TokenType::Let | TokenType::Const | TokenType::Var) {
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
                // index assign: name[expr] = expr;
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