// src/payjar/mod.rs
// Sub-modules
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod builtins;
pub mod sqlite3;
pub mod gui_channel;

// Re-export everything callers need
pub use ast::{AstNode, NodeType};
pub use lexer::tokenize;
pub use parser::Parser;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process;
use std::rc::Rc;

// ============================================================
// TOKEN TYPES  (used by lexer, parser, and interpreter)
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Import, Public, Private, Class, Enum, Main, Self_, InnerSelf,
    Def, Print, Perror, Let, Const, Var, New,
    Readln, Readi, Readf, Return, If, Else, While, For, In, Range,
    Package,
    And, Or, Not,
    PlusEqual, MinusEqual,
    Identifier, Number, FloatLiteral, StringLiteral, BacktickString, RegexString,
    Plus, Minus, Multiply, Divide, Modulo,
    Equal, EqualEqual, NotEqual,
    LessThan, GreaterThan, LessEqual, GreaterEqual,
    LParen, RParen, LBrack, RBrack, LBrace, RBrace,
    Line, Amp, QM, Caret,
    Semicolon, Comma, Dot, Colon, At, As,
    Eof,
    Int, Float, Null, Str, Any, Array,
    PlusPlus, MinusMinus, Arrow,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub typ: TokenType,
    pub value: String,
}

impl Token {
    pub fn new(typ: TokenType, value: impl Into<String>) -> Self {
        Token { typ, value: value.into() }
    }
}

// ============================================================
// COMPLEX  (used by runtime)
// ============================================================

#[derive(Debug, Clone)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

impl Complex {
    pub fn new(r: f64, i: f64) -> Self { Self { real: r, imag: i } }
    pub fn to_display(&self) -> String {
        let l = match self.real { 0.0 => "".to_string(), _ => self.real.to_string() };
        let r = match self.imag {
            0.0  => "".to_string(),
            1.0  => "+i".to_string(),
            -1.0 => "-i".to_string(),
            _    => format!("{}i", self.imag),
        };
        format!("{}{}", l, r)
    }
    pub fn powc(&mut self, other: Complex) -> Complex {
        if other.imag != 0.0 { return Complex::new(0.0, 0.0); }
        Complex::new(self.real.powf(other.real), self.imag.powf(other.imag))
    }
    pub fn sqrt(&self) -> Complex {
        if self.real < 0.0 { Complex::new(0.0, self.real.abs()) }
        else { Complex::new(self.real.sqrt(), 0.0) }
    }
}

// ============================================================
// RUNTIME VALUES
// ============================================================

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Comp(Complex),
    Null,
    Object(Rc<RefCell<PJObject>>),
    List(Rc<RefCell<Vec<Value>>>),
}

impl Value {
    pub fn as_int(&self) -> i64 {
        match self {
            Value::Null      => 0,
            Value::Int(i)    => *i,
            Value::Float(f)  => *f as i64,
            Value::Str(_)    => 0,
            _ => 0,
        }
    }
    pub fn as_float(&self) -> f64 {
        match self {
            Value::Null     => 0.0,
            Value::Int(i)   => *i as f64,
            Value::Float(f) => *f,
            Value::Str(_)   => 0.0,
            _ => 0.0,
        }
    }
    pub fn as_complex(&self) -> Complex {
        match self {
            Value::Null      => Complex::new(0.0, 0.0),
            Value::Int(i)    => Complex::new(*i as f64, 0.0),
            Value::Float(f)  => Complex::new(*f, 0.0),
            Value::Comp(c)   => Complex::new(c.real, c.imag),
            _ => Complex::new(0.0, 0.0),
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null      => false,
            Value::Int(i)    => *i != 0,
            Value::Float(f)  => *f != 0.0,
            Value::Str(s)    => !s.is_empty(),
            _ => true,
        }
    }
    pub fn to_string_repr(&self) -> String {
        match self {
            Value::Int(i)    => i.to_string(),
            Value::Float(f)  => {
                if f.fract() == 0.0 && f.abs() < 1e15 { format!("{}", *f as i64) }
                else { format!("{}", f) }
            }
            Value::Str(s)    => s.clone(),
            Value::Null      => "null".to_string(),
            Value::Object(o) => format!("<{} object>", o.borrow().class_name),
            Value::List(l)   => {
                let items: Vec<String> = l.borrow().iter().map(|v| {
                    if let Value::Str(s) = v { format!("'{}'", s) }
                    else { v.to_string_repr() }
                }).collect();
                format!("[{}]", items.join(", "))
            }
            _ => "".to_string(),
        }
    }
    pub fn print(&self) {
        match self {
            Value::Int(i)   => print!("{}", i),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 { print!("{}", *f as i64); }
                else { print!("{}", f); }
            }
            Value::Str(s)    => print!("{}", s),
            Value::Null      => print!("null"),
            Value::Object(o) => print!("<{} object>", o.borrow().class_name),
            Value::List(l)   => {
                print!("[");
                let items = l.borrow();
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { print!(", "); }
                    if let Value::Str(s) = v { print!("'{}'", s); } else { v.print(); }
                }
                print!("]");
            }
            _ => {}
        }
    }
    pub fn perror(&self) {
        match self {
            Value::Int(i)   => eprint!("{}", i),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 { eprint!("{}", *f as i64); }
                else { eprint!("{}", f); }
            }
            Value::Str(s)    => eprint!("{}", s),
            Value::Null      => eprint!("null"),
            Value::Object(o) => eprint!("<{} object>", o.borrow().class_name),
            Value::List(l)   => {
                eprint!("[");
                let items = l.borrow();
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { eprint!(", "); }
                    if let Value::Str(s) = v { eprint!("'{}'", s); } else { v.perror(); }
                }
                eprint!("]");
            }
            _ => {}
        }
    }
}

// ============================================================
// OBJECT / FIELD
// ============================================================

#[derive(Debug, Clone)]
pub struct PJField {
    pub name: String,
    pub val: Value,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct PJObject {
    pub class_name: String,
    pub fields: Vec<PJField>,
    pub class_def: AstNode,
}

impl PJObject {
    pub fn find_field(&self, name: &str) -> Option<&PJField> {
        self.fields.iter().find(|f| f.name == name)
    }
    pub fn find_field_mut(&mut self, name: &str) -> Option<&mut PJField> {
        self.fields.iter_mut().find(|f| f.name == name)
    }
    pub fn find_method(&self, name: &str) -> Option<&AstNode> {
        self.class_def.methods.iter().find(|m| m.name.as_deref() == Some(name))
    }
}

// ============================================================
// SCOPE
// ============================================================

#[derive(Debug, Clone)]
pub struct VarEntry {
    pub val: Value,
    pub is_const: bool,
}

#[derive(Debug)]
pub struct Scope {
    vars: HashMap<String, VarEntry>,
    parent: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
    pub fn new(parent: Option<Rc<RefCell<Scope>>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Scope { vars: HashMap::new(), parent }))
    }
    pub fn set_declare(scope: &Rc<RefCell<Scope>>, name: &str, val: Value, is_const: bool) {
        let mut s = scope.borrow_mut();
        if s.vars.contains_key(name) {
            eprintln!("Runtime Error: Redeclaration of '{}'", name); process::exit(1);
        }
        s.vars.insert(name.to_string(), VarEntry { val, is_const });
    }
    pub fn set_assign(scope: &Rc<RefCell<Scope>>, name: &str, val: Value) {
        let mut cur = Rc::clone(scope);
        loop {
            {
                let mut s = cur.borrow_mut();
                if let Some(entry) = s.vars.get_mut(name) {
                    if entry.is_const {
                        eprintln!("Runtime Error: Cannot assign to const '{}'", name); process::exit(1);
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
    pub fn get(scope: &Rc<RefCell<Scope>>, name: &str) -> Value {
        let mut cur = Rc::clone(scope);
        loop {
            {
                let s = cur.borrow();
                if let Some(entry) = s.vars.get(name) { return entry.val.clone(); }
            }
            let parent = cur.borrow().parent.as_ref().map(Rc::clone);
            match parent {
                Some(p) => cur = p,
                None => {
                    eprintln!("Runtime Error: Undefined variable '{}'", name); process::exit(1);
                }
            }
        }
    }
}

// ============================================================
// ENV / INTERPRETER STATE
// ============================================================

pub struct Env {
    pub funcs: HashMap<String, AstNode>,
    pub classes: HashMap<String, AstNode>,
    pub scope: Rc<RefCell<Scope>>,
}

impl Env {
    pub fn new() -> Self {
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
    for name in &["math", "io", "sqlite3", "sqllite3", "random", "regex", "gui"] {
        let obj = create_builtin_module(name);
        Scope::set_declare(&env.scope, name, Value::Object(obj), false);
    }
}

// ============================================================
// SIGNAL (return value propagation)
// ============================================================

enum Signal {
    None,
    Return(Value),
}

// ============================================================
// FUNCTION CALL
// ============================================================

pub fn call_func_with_vals(
    env: &mut Env,
    func_def: &AstNode,
    arg_vals: Vec<Value>,
    self_obj: Option<Rc<RefCell<PJObject>>>,
) -> Value {
    let saved_scope = Rc::clone(&env.scope);
    env.scope = Scope::new(Some(Rc::clone(&saved_scope)));

    let mut param_offset = 0;
    if func_def.is_method && func_def.params.first().map(String::as_str) == Some("self") {
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

// ============================================================
// STATEMENT VISITOR
// ============================================================

fn visit_stmt(env: &mut Env, node: &AstNode, self_obj: Option<Rc<RefCell<PJObject>>>) -> Signal {
    match node.typ {
        NodeType::Print => {
            let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            v.print(); println!();
            Signal::None
        }
        NodeType::Perror => {
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
                            env.scope = saved; return s;
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
                            env.scope = body_saved; env.scope = saved; return s;
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
                        eprintln!("Runtime Error: Cannot assign to const field '{}'", name); process::exit(1);
                    }
                    f.val = new_val;
                } else {
                    eprintln!("Runtime Error: Field '{}' not found on '{}'", name, obj.class_name); process::exit(1);
                }
            } else {
                eprintln!("Runtime Error: Member assignment on non-object"); process::exit(1);
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
                if i >= items.len() { eprintln!("Runtime Error: List index out of range"); process::exit(1); }
                items[i] = newval;
            } else {
                eprintln!("Runtime Error: Cannot index-assign on non-list"); process::exit(1);
            }
            Signal::None
        }
        NodeType::Import  => Signal::None,
        NodeType::FieldDecl => Signal::None,
        _ => { eval(env, node, self_obj); Signal::None }
    }
}

// ============================================================
// EXPRESSION EVALUATOR
// ============================================================

pub fn eval(env: &mut Env, node: &AstNode, self_obj: Option<Rc<RefCell<PJObject>>>) -> Value {
    match node.typ {
        NodeType::Int   => Value::Int(node.int_val),
        NodeType::Float => Value::Float(node.fval),
        NodeType::Str   => Value::Str(node.str_val.clone().unwrap_or_default()),

        NodeType::TemplateStr => {
            let mut result = String::new();
            for part in &node.template_parts {
                result.push_str(&eval(env, part, self_obj.clone()).to_string_repr());
            }
            Value::Str(result)
        }

        NodeType::VarAccess => {
            let name = node.name.as_deref().unwrap();
            if name == "self" {
                if let Some(ref obj) = self_obj { return Value::Object(Rc::clone(obj)); }
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
            Value::Int(line.trim().parse().unwrap_or(0))
        }

        NodeType::Readf => {
            let prompt = eval(env, node.expr.as_ref().unwrap(), self_obj);
            print!("{}", prompt.to_string_repr());
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).ok();
            Value::Float(line.trim().parse().unwrap_or(0.0))
        }

        NodeType::Index => {
            let lst = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            let idx = eval(env, node.right.as_ref().unwrap(), self_obj);
            match lst {
                Value::List(ref l) => {
                    let items = l.borrow();
                    let mut i = idx.as_int();
                    if i < 0 { i = items.len() as i64 + i; }
                    if i < 0 || i >= items.len() as i64 {
                        eprintln!("Runtime Error: List index {} out of range", i); process::exit(1);
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
            if op == "+" {
                if matches!(lv, Value::Str(_)) || matches!(rv, Value::Str(_)) {
                    return Value::Str(format!("{}{}", lv.to_string_repr(), rv.to_string_repr()));
                }
                if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                    return Value::Float(lv.as_float() + rv.as_float());
                }
                let (l, r) = (lv.as_int(), rv.as_int());
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
                    let (l, r) = (lv.as_int(), rv.as_int());
                    match l.checked_sub(r) {
                        Some(v) => Value::Int(v),
                        None    => Value::Float(l as f64 - r as f64),
                    }
                }
                "*" => {
                    if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                        return Value::Float(lv.as_float() * rv.as_float());
                    }
                    let (l, r) = (lv.as_int(), rv.as_int());
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
                "-"       => Value::Int(-v.as_int()),
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
            builtins::eval_builtin_or_func(env, name, arg_vals, self_obj)
        }

        NodeType::ObjectCreation => {
            let class_name = node.name.as_deref().unwrap();
            let cls = env.classes.get(class_name).cloned().unwrap_or_else(|| {
                eprintln!("Runtime Error: Undefined class '{}'", class_name); process::exit(1);
            });
            let fields: Vec<PJField> = cls.fields.iter().map(|fd| {
                let fv = if let Some(ref e) = fd.expr { eval(env, e, None) } else { Value::Null };
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
                                let pos = items.iter().position(|v| match (v, &avals[0]) {
                                    (Value::Str(a), Value::Str(b)) => a == b,
                                    (a, b) => a.as_int() == b.as_int(),
                                });
                                match pos {
                                    Some(i) => { items.remove(i); Value::Null }
                                    None => { eprintln!("Runtime Error: remove: value not in list"); process::exit(1); }
                                }
                            }
                            "len" | "length" => Value::Int(l.borrow().len() as i64),
                            "contains" => {
                                if avals.len() != 1 { eprintln!("contains() takes 1 arg"); process::exit(1); }
                                let found = l.borrow().iter().any(|v| match (v, &avals[0]) {
                                    (Value::Str(a), Value::Str(b)) => a == b,
                                    (a, b) => a.as_int() == b.as_int(),
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
                        if let Some(v) = builtins::eval_builtin_module_method(
                            class_name.as_str(), member_name, &node.args, env, self_obj.clone())
                        {
                            return v;
                        }
                        let method = { let obj = obj_rc.borrow(); obj.find_method(member_name).cloned() };
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
                                eprintln!("Runtime Error: Field '{}' not found", member_name); process::exit(1);
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

        NodeType::FuncDef | NodeType::ClassDef => Value::Null,
        _ => Value::Null,
    }
}

// ============================================================
// RUN
// ============================================================

fn run(env: &mut Env, program: &AstNode) {
    for stmt in &program.body {
        if stmt.typ == NodeType::FuncDef {
            env.funcs.insert(stmt.name.clone().unwrap(), stmt.clone());
        } else if stmt.typ == NodeType::ClassDef {
            env.classes.insert(stmt.name.clone().unwrap(), stmt.clone());
        }
    }
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

fn collect_imports(tokens: &[Token]) -> Vec<(String, Option<String>)> {
    let mut imports = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].typ == TokenType::Identifier && tokens[i].value == "from" {
            if i + 3 < tokens.len()
                && tokens[i + 2].typ == TokenType::Import
                && tokens[i + 3].typ == TokenType::Multiply
            {
                let pkg = tokens[i + 1].value.clone();
                imports.push((pkg, None));
                i += 4;
                if i < tokens.len() && tokens[i].typ == TokenType::Semicolon { i += 1; }
                continue;
            }
        }
        if tokens[i].typ == TokenType::Import {
            i += 1;
            if i < tokens.len() {
                let pkg = tokens[i].value.clone();
                i += 1;
                let alias = if i < tokens.len() && tokens[i].typ == TokenType::As {
                    i += 1;
                    let a = tokens[i].value.clone();
                    i += 1;
                    Some(a)
                } else { None };
                // skip the trailing semicolon
                if i < tokens.len() && tokens[i].typ == TokenType::Semicolon { i += 1; }
                imports.push((pkg, alias));
            }
        } else { i += 1; }
    }
    imports
}

fn load_package_file(env: &mut Env, path: &str, debug: bool, target_pkg: &str) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => { if debug { println!("[pjrt] Skipping '{}' (cannot open)", path); } return; }
    };
    let tokens = tokenize(&src);
    if tokens.iter().any(|t| t.typ == TokenType::Import) {
        eprintln!("pjrt Error: file '{}' contains an import statement.", path); process::exit(1);
    }
    let pkg_name = match detect_package(&tokens) {
        Some(p) => p,
        None => { if debug { println!("[pjrt] Skipping '{}' (no package declaration)", path); } return; }
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

fn load_folder_package(env: &mut Env, pkg_name: &str, alias: Option<&str>, debug: bool) {
    if matches!(pkg_name, "math"|"io"|"sqlite3"|"sqllite3"|"random"|"regex"|"gui"|"os"|"sys") {
        if let Some(alias_name) = alias {
            let obj_val = Scope::get(&env.scope, pkg_name);
            if alias_name != pkg_name {
                Scope::set_declare(&env.scope, alias_name, obj_val, false);
            }
            if debug { println!("[import] Builtin module '{}' aliased as '{}'", pkg_name, alias_name); }
        } else if debug {
            println!("[import] Builtin module '{}' loaded flat", pkg_name);
        }
        return;
    }

    let init_path = format!("{}/init.pj", pkg_name);
    let src = fs::read_to_string(&init_path).unwrap_or_else(|_| {
        eprintln!("Import Error: cannot open '{}' for package '{}'", init_path, pkg_name); process::exit(1);
    });
    let tokens = tokenize(&src);
    match detect_package(&tokens) {
        Some(ref p) if p == pkg_name => {}
        Some(ref p) => {
            eprintln!("Import Error: '{}' declares package '{}' but expected '{}'", init_path, p, pkg_name);
            process::exit(1);
        }
        None => {
            eprintln!("Import Error: '{}' has no package declaration", init_path); process::exit(1);
        }
    }
    if debug { println!("[import] Loading package '{}' from '{}'", pkg_name, init_path); }
    let mut parser = Parser::new(tokens);
    let pkg_ast = parser.parse_package_file();

    match alias {
        None => {
            for node in &pkg_ast.body {
                if node.typ == NodeType::FuncDef {
                    env.funcs.insert(node.name.clone().unwrap(), node.clone());
                } else if node.typ == NodeType::ClassDef {
                    env.classes.insert(node.name.clone().unwrap(), node.clone());
                }
            }
        }
        Some(alias_name) => {
            let mut synthetic_class = AstNode::new(NodeType::ClassDef);
            synthetic_class.name = Some(pkg_name.to_string());
            for node in &pkg_ast.body {
                if node.typ == NodeType::FuncDef {
                    env.funcs.insert(node.name.clone().unwrap(), node.clone());
                    let mut method = node.clone();
                    method.is_method = false;
                    synthetic_class.methods.push(method);
                } else if node.typ == NodeType::ClassDef {
                    env.classes.insert(node.name.clone().unwrap(), node.clone());
                }
            }
            let ns_obj = Rc::new(RefCell::new(PJObject {
                class_name: pkg_name.to_string(),
                fields: Vec::new(),
                class_def: synthetic_class,
            }));
            Scope::set_declare(&env.scope, alias_name, Value::Object(ns_obj), false);
        }
    }
}

// ============================================================
// PUBLIC API
// ============================================================

pub fn interpret_file(path: &str, debug: bool) {
    let src = fs::read_to_string(path).unwrap_or_else(|_| {
        eprintln!("Error: Cannot open file '{}'", path); process::exit(1);
    });
    let tokens = tokenize(&src);
    if let Some(pkg) = detect_package(&tokens) {
        eprintln!("Error: '{}' is a package file (package {}). Use pjrt.", path, pkg); process::exit(1);
    }
    if debug {
        println!("=== TOKENS ===");
        for (i, t) in tokens.iter().enumerate() { println!("  [{}] type={:?} value={}", i, t.typ, t.value); }
        println!("==============");
    }
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

pub fn interpret(code: &str, debug: bool) {
    let tokens = tokenize(code);
    if let Some(pkg) = detect_package(&tokens) {
        eprintln!("Error: code is a package file (package {}). Use pjrt.", pkg); process::exit(1);
    }
    if debug {
        println!("=== TOKENS ===");
        for (i, t) in tokens.iter().enumerate() { println!("  [{}] type={:?} value={}", i, t.typ, t.value); }
        println!("==============");
    }
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

pub fn pjrt_run(debug: bool, pkgn: &str, skip: &str) {
    let mut env = Env::new();
    let entries = fs::read_dir(".").unwrap_or_else(|_| {
        eprintln!("pjrt Error: cannot open current directory"); process::exit(1);
    });
    let mut pkg_count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == skip || name == "main.pj" || !name.ends_with(".pj") { continue; }
        let before = env.funcs.len() + env.classes.len();
        load_package_file(&mut env, &name, debug, pkgn);
        if env.funcs.len() + env.classes.len() > before { pkg_count += 1; }
    }
    if pkg_count == 0 {
        eprintln!("pjrt Error: package '{}' not found in current directory", pkgn); process::exit(1);
    }
    if debug { println!("[pjrt] Loaded file(s) for package: {}.", pkgn); }
    let src = fs::read_to_string("main.pj").unwrap_or_else(|_| {
        eprintln!("Error: Cannot open file 'main.pj'"); process::exit(1);
    });
    let tokens = tokenize(&src);
    if let Some(p) = detect_package(&tokens) {
        eprintln!("pjrt Error: main.pj declares package {}. main.pj must not declare a package.", p);
        process::exit(1);
    }
    if tokens.iter().any(|t| t.typ == TokenType::Import) {
        eprintln!("pjrt Error: main.pj contains an import statement. pjrt does not support imports.");
        process::exit(1);
    }
    if debug {
        println!("=== TOKENS (main.pj) ===");
        for (i, t) in tokens.iter().enumerate() { println!("  [{}] type={:?} value={}", i, t.typ, t.value); }
        println!("========================");
    }
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program();
    if debug { println!("=== Parse OK (main.pj) ==="); }
    run(&mut env, &prog);
}

pub fn autorun(debug: bool, skip: &str) {
    let skips: Vec<&str> = skip.trim_matches(|c| c == '[' || c == ']').split(',').collect();
    let entries = fs::read_dir(".").unwrap_or_else(|_| {
        eprintln!("Error: cannot open current directory"); process::exit(1);
    });
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if skips.contains(&name.as_str()) { continue; }
        if name.ends_with(".pj") { interpret_file(&name, debug); }
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
}