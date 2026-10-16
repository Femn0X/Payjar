// src/payjar/mod.rs
// Sub-modules
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod builtins;
pub mod sqlite3;
pub mod gui_channel;
pub mod jit;

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
use std::sync::OnceLock;

fn jit_backend_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        if let Ok(value) = std::env::var("PAYJAR_BACKEND") {
            return value.eq_ignore_ascii_case("jit") || value.eq_ignore_ascii_case("auto");
        }
        fs::read_to_string("config.toml").ok().and_then(|s| {
            s.lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                if key.trim() == "backend" {
                    Some(value.trim().trim_matches('"').to_ascii_lowercase())
                } else {
                    None
                }
            })
        }).map(|v| v == "jit" || v == "auto").unwrap_or(false)
    })
}
// ============================================================
// TOKEN TYPES  (used by lexer, parser, and interpreter)
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Import, Public, Private, Class, Enum, Main, Self_, InnerSelf,
    Def, Print, Perror, Let, Const, New,
    Readln, Readi, Readf, Return, If, Else, While, For, In, Range,
    Package,Continue,Break,
    Async, Await,
    Try, Except, Finally, Throw,
    ComplexType, ImaginaryLiteral,
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
    Int, Float, Null, Str, Array,
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
        let real_part = if self.real == 0.0 { String::new() } else { self.real.to_string() };
        // Bugfix: this used to just concatenate the real part with
        // `format!("{}i", self.imag)` for any imaginary part other than
        // exactly +-1, so e.g. Complex{real:3, imag:2} printed as "32i"
        // instead of "3+2i" -- indistinguishable from thirty-two i's worth
        // of imaginary part. Now a positive imaginary part gets an explicit
        // '+' whenever there's a real part in front of it to separate from.
        let imag_part = if self.imag == 0.0 {
            String::new()
        } else if self.imag == 1.0 {
            if real_part.is_empty() { "i".to_string() } else { "+i".to_string() }
        } else if self.imag == -1.0 {
            "-i".to_string()
        } else if self.imag > 0.0 {
            if real_part.is_empty() { format!("{}i", self.imag) } else { format!("+{}i", self.imag) }
        } else {
            format!("{}i", self.imag)
        };
        if real_part.is_empty() && imag_part.is_empty() { "0".to_string() } else { format!("{}{}", real_part, imag_part) }
    }
    pub fn powc(&self, other: Complex) -> Complex {
        // z^w = exp(w * ln(z))
        let lnr = (self.real * self.real + self.imag * self.imag).sqrt().ln();
        let th = self.imag.atan2(self.real);
        let new_r = lnr * other.real - th * other.imag;
        let new_i = lnr * other.imag + th * other.real;
        let er = new_r.exp();
        Complex::new(er * new_i.cos(), er * new_i.sin())
    }
    pub fn sqrt(&self) -> Complex {
        let m = (self.real * self.real + self.imag * self.imag).sqrt();
        let sr = ((m + self.real) / 2.0).sqrt();
        let si = if self.imag < 0.0 { -((m - self.real) / 2.0).sqrt() } else { ((m - self.real) / 2.0).sqrt() };
        Complex::new(sr, si)
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
    Map(Rc<RefCell<rustc_hash::FxHashMap<String, Value>>>),
    Task(Rc<RefCell<TaskState>>),
    /// An enum constant, e.g. `Color.RED`. Holds (enum type name, variant name).
    Enum(String, String),
}

// ============================================================
// ASYNC TASKS
// ============================================================
//
// PayJar's interpreter is a single-threaded tree walker whose values
// (Rc<RefCell<..>>) are not `Send`, so true OS-thread concurrency isn't
// available. Instead, `async func` calls are lazy: calling one doesn't run
// the body — it captures the call and returns a `Task` handle immediately.
// The body only actually runs the first time the task is `await`ed (or
// never, if it's never awaited), and the result is cached so awaiting the
// same task again just returns the cached value instead of re-running it.
#[derive(Debug, Clone)]
pub enum TaskState {
    Pending {
        func: AstNode,
        args: Vec<Value>,
        self_obj: Option<Rc<RefCell<PJObject>>>,
    },
    Ready(Value),
}

/// Build a not-yet-run Task value for a call to an `async func`.
/// Java-style visibility check: is the code currently executing (identified
/// by its `self_obj`, i.e. which object's method is running, if any) allowed
/// to reach a `private` member of `target_class`? True if we're running
/// inside a method of that same class (any instance, not just the same
/// object — matching how `private` works between instances in Java).
fn same_class(self_obj: &Option<Rc<RefCell<PJObject>>>, target_class: &str) -> bool {
    self_obj.as_ref().map(|so| so.borrow().class_name == target_class).unwrap_or(false)
}

pub fn make_pending_task(func: AstNode, args: Vec<Value>, self_obj: Option<Rc<RefCell<PJObject>>>) -> Value {
    Value::Task(Rc::new(RefCell::new(TaskState::Pending { func, args, self_obj })))
}

/// `await` semantics: running a pending task's body (once, caching the
/// result) and returning its value. Awaiting a plain (non-task) value is
/// simply the identity function, matching how `await` behaves on
/// already-resolved values in most languages.
pub fn resolve_task(env: &mut Env, v: Value) -> Value {
    let task_rc = match v {
        Value::Task(ref t) => Rc::clone(t),
        _ => return v,
    };

    if let TaskState::Ready(ref ready) = *task_rc.borrow() {
        return ready.clone();
    }

    // Take the pending call out of the task before running it, so the
    // borrow doesn't stay held (and re-entrant/self-referential awaits on
    // this exact task can't deadlock on the RefCell).
    let taken = std::mem::replace(&mut *task_rc.borrow_mut(), TaskState::Ready(Value::Null));
    let (func, args, self_obj) = match taken {
        TaskState::Pending { func, args, self_obj } => (func, args, self_obj),
        TaskState::Ready(ready) => { *task_rc.borrow_mut() = TaskState::Ready(ready.clone()); return ready; }
    };

    let result = call_func_with_vals(env, &func, args, self_obj);
    *task_rc.borrow_mut() = TaskState::Ready(result.clone());
    result
}

impl Value {
    pub fn as_int(&self) -> i64 {
        match self {
            Value::Int(i)    => *i,
            Value::Float(f)  => *f as i64,
            Value::Comp(c)   =>  c.real as i64,
            _=>0,
        }
    }
    pub fn as_float(&self) -> f64 {
        match self {
            Value::Int(i)   => *i as f64,
            Value::Float(f) => *f,
            Value::Comp(c)  => c.real,
            _=>0.0,
        }
    }
    pub fn as_complex(&self) -> Complex {
        match self {
            Value::Int(i)    => Complex::new(*i as f64, 0.0),
            Value::Float(f)  => Complex::new(*f, 0.0),
            Value::Comp(c)   => Complex::new(c.real, c.imag),
            Value::List(a)   => {
             let vals = a.borrow();
             let len = vals.len();
             if len!=2{
              Complex::new(0.0,0.0) 
             }
             else {
              Complex::new(vals[0].as_float(),vals[1].as_float())
             }
            }
            _=>Complex::new(0.0,0.0),
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null      => false,
            Value::Int(i)    => *i != 0,
            Value::Float(f)  => *f != 0.0,
            Value::Str(s)    => !s.is_empty(),
            Value::Comp(c)   => c.real!=0.0||c.imag!=0.0,
            // Bugfix: objects/lists/maps/tasks used to fall through to the
            // wildcard below and were always falsy, so `if (someObject)` or
            // `if (someList)` could never be true even for non-empty values.
            Value::Object(_) => true,
            Value::List(_)   => true,
            Value::Map(_)    => true,
            Value::Task(_)   => true,
            Value::Enum(_, _) => true,
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
            Value::Null      => "Null".to_string(),
            // Bugfix: Comp had no arm here and silently fell through to the
            // wildcard as an empty string, so printing/toStr()-ing a complex
            // number produced nothing at all.
            Value::Comp(c)   => c.to_display(),
            Value::Task(_)   => "<task>".to_string(),
            Value::Enum(enum_name, variant) => format!("{}.{}", enum_name, variant),
            Value::Object(o) => format!("<{} object>", o.borrow().class_name),
            Value::List(l)   => {
                let items: Vec<String> = l.borrow().iter().map(|v| {
                    if let Value::Str(s) = v { format!("'{}'", s) }
                    else { v.to_string_repr() }
                }).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Map(m) => {
                let b = m.borrow();
                let mut pairs: Vec<String> = b.iter()
                    .map(|(k, v)| format!("'{}': {}", k, if let Value::Str(s) = v { format!("'{}'", s) } else { v.to_string_repr() }))
                    .collect();
                pairs.sort();
                format!("{{{}}}", pairs.join(", "))
            }
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
            // Bugfix: Comp fell through to the wildcard and printed nothing.
            Value::Comp(c)   => print!("{}", c.to_display()),
            Value::Task(_)   => print!("<task>"),
            Value::Enum(enum_name, variant) => print!("{}.{}", enum_name, variant),
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
            Value::Map(m) => {
                let b = m.borrow();
                let mut pairs: Vec<String> = b.iter()
                    .map(|(k, v)| format!("'{}': {}", k, if let Value::Str(s) = v { format!("'{}'", s) } else { v.to_string_repr() }))
                    .collect();
                pairs.sort();
                print!("{{{}}}", pairs.join(", "));
            }
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
            // Bugfix: Comp fell through to the wildcard and printed nothing.
            Value::Comp(c)   => eprint!("{}", c.to_display()),
            Value::Task(_)   => eprint!("<task>"),
            Value::Enum(enum_name, variant) => eprint!("{}.{}", enum_name, variant),
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
            Value::Map(m) => {
                let b = m.borrow();
                let mut pairs: Vec<String> = b.iter()
                    .map(|(k, v)| format!("'{}': {}", k, if let Value::Str(s) = v { format!("'{}'", s) } else { v.to_string_repr() }))
                    .collect();
                pairs.sort();
                eprint!("{{{}}}", pairs.join(", "));
            }
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
    pub is_private: bool,
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
        // Bugfix: redeclaring a variable in the same scope used to be a
        // fatal error that killed the whole process. That's especially bad
        // in the interactive CLI, where re-running a `let` line (extremely
        // common when experimenting) would silently end the entire session
        // and lose every variable saved so far. A const can't be
        // re-bound, but an ordinary variable is now simply updated in place.
        if let Some(existing) = s.vars.get(name) {
            if existing.is_const {
                eprintln!("Runtime Error: Cannot redeclare const '{}'", name);
                return;
            }
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
                        eprintln!("Runtime Error: Cannot assign to const '{}'", name);
                        return;
                    }
                    entry.val = val;
                    return;
                }
            }
            let parent = cur.borrow().parent.as_ref().map(Rc::clone);
            match parent {
                Some(p) => cur = p,
                None => {
                    // Bugfix: this used to process::exit(1), which meant a
                    // single typo anywhere would kill the whole CLI session.
                    eprintln!("Runtime Error: Assignment to undefined variable '{}'", name);
                    return;
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
                    // Bugfix: this used to process::exit(1); see set_assign above.
                    eprintln!("Runtime Error: Undefined variable '{}'", name);
                    return Value::Null;
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
    /// Enum type name -> ordered list of variant names.
    pub enums: HashMap<String, Vec<String>>,
    pub scope: Rc<RefCell<Scope>>,
}

impl Env {
    pub fn new() -> Self {
        install_quiet_panic_hook();
        let mut env = Env {
            funcs: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            scope: Scope::new(None),
        };
        register_builtin_modules(&mut env);
        env
    }
}

// ============================================================
// CATCHABLE RUNTIME ERRORS
// ============================================================
//
// `try { } except (e) { }` is implemented on top of Rust's own unwinding:
// a runtime error calls `runtime_error(...)`, which panics carrying the
// message as its payload. A `Try` node runs its body inside
// `std::panic::catch_unwind`; if nothing panicked, control continues
// normally, and if something did, the message is bound to the `except`
// variable and the except block runs instead. Nothing escapes the
// process anymore except an error that reaches the top of the whole
// program with no enclosing `try` -- at that point `run_catching` (see
// the RUN section) catches it once more, prints it exactly like the old
// eprintln!+exit did, and exits with status 1, so scripts with no
// try/except at all behave exactly as before.
//
// This relies on unwinding actually happening, so the release profile's
// `panic = "abort"` (see Cargo.toml) must not be used -- see the comment
// there.

static PANIC_HOOK_INIT: std::sync::Once = std::sync::Once::new();

/// Rust's default panic hook prints a "thread panicked at ..." + backtrace
/// notice to stderr for every panic, including ones we're about to catch
/// and handle ourselves via `try`/`except`. That default output would be
/// confusing noise for a caught, handled error, so it's replaced once (per
/// process) with a no-op hook. Uncaught errors are still printed -- just by
/// us, in our own "Runtime Error: ..." style, at the point they're caught
/// at the top level (see `run_catching`).
fn install_quiet_panic_hook() {
    PANIC_HOOK_INIT.call_once(|| {
        std::panic::set_hook(Box::new(|_info| {}));
    });
}

/// Raises a catchable runtime error: unwinds up to the nearest enclosing
/// `try` block (or, if there is none, all the way out of the program).
/// This replaces the old pattern of `eprintln!(...); process::exit(1);`,
/// which killed the whole process immediately and could never be caught.
pub fn runtime_error(msg: String) -> ! {
    panic!("{}", msg);
}

/// Recovers the `String` passed to `runtime_error` out of a caught panic's
/// payload. Falls back to a generic message for panics that didn't
/// originate from `runtime_error` (e.g. a genuine interpreter bug), so
/// `except` still gets *something* sensible to work with either way.
fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Runtime Error: an unknown internal error occurred".to_string()
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

/// Every builtin module name recognized as a bare identifier (e.g. `math`,
/// `cmath.sqrt(...)`) and as a valid target of `import`. Kept as a single
/// list so the three places that need to agree on it (module registration,
/// and the two import-whitelist checks below) can't drift apart the way
/// they had: `cmath`, `json`, `os`, `sys`, `time`, `base64`, and `hash` were
/// all dispatchable in eval_builtin_module_method but never registered as
/// accessible identifiers here, so e.g. `cmath.sqrt(...)` or `time.now()`
/// always failed with "Undefined variable" before ever reaching the
/// dispatch logic -- those modules were completely unreachable.
const BUILTIN_MODULE_NAMES: &[&str] = &[
    "math", "cmath", "io", "sqlite3", "sqllite3", "random", "regex", "gui",
    "net", "os","system", "time", "json", "base64", "hash",
];

fn register_builtin_modules(env: &mut Env) {
    for name in BUILTIN_MODULE_NAMES {
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
    Break,
    Continue,
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
    if func_def.is_method() && func_def.params.first().map(String::as_str) == Some("self") {
        if let Some(ref obj) = self_obj {
            Scope::set_declare(&env.scope, "self", Value::Object(Rc::clone(obj)), false);
        }
        param_offset = 1;
    }

    let expected = func_def.params.len() - param_offset;
    if expected != arg_vals.len() {
        runtime_error(format!("Runtime Error: Function '{}' expected {} args but got {}",
            func_def.name.as_deref().unwrap_or("?"), expected, arg_vals.len()));
    }
    for (i, v) in arg_vals.into_iter().enumerate() {
        Scope::set_declare(&env.scope, &func_def.params[i + param_offset], v, false);
    }

    let body = func_def.body.clone();
    let mut ret = Value::Null;
    for stmt in &body {
        match visit_stmt(env, stmt, self_obj.clone()) {
            Signal::Return(v) => { ret = v; break; }
            Signal::Break | Signal::Continue => {
                runtime_error(format!("Runtime Error: 'break'/'continue' used outside of a loop"));
            }
            Signal::None => {}
        }
    }
    env.scope = saved_scope;
    ret
}

/// Runs a `Try` node's `body`, catching any `runtime_error`/`throw` that
/// escapes it (from anywhere in its dynamic extent -- a builtin, a nested
/// call, a deeply-nested method -- not just its own top-level statements)
/// and routing it to the `except` block instead of letting it kill the
/// process. `finally`, if present, always runs last, whether the try body
/// finished normally, threw, or itself contained a `return`/`break`/
/// `continue` -- and a control-flow signal out of `finally` itself takes
/// priority over whatever the try/except part produced, matching how
/// finally works in Java/JS/Python.
fn visit_try(env: &mut Env, node: &AstNode, self_obj: Option<Rc<RefCell<PJObject>>>) -> Signal {
    let outer_scope = Rc::clone(&env.scope);

    env.scope = Scope::new(Some(Rc::clone(&outer_scope)));
    let try_body = &node.body;
    let try_result = {
        let payload = std::panic::AssertUnwindSafe((&mut *env, self_obj.clone(), try_body));
        std::panic::catch_unwind(move || {
            // Forces the closure to capture `payload` as a single unit
            // instead of each tuple field individually -- with the latter,
            // rustc's precise-capture analysis would see straight through
            // the AssertUnwindSafe wrapper to the non-UnwindSafe fields
            // inside it and refuse to compile.
            let payload = payload;
            let std::panic::AssertUnwindSafe((env, so, body)) = payload;
            let mut sig = Signal::None;
            for stmt in body {
                match visit_stmt(env, stmt, so.clone()) {
                    Signal::None => {}
                    s => { sig = s; break; }
                }
            }
            sig
        })
    };
    // Whatever happened inside -- clean finish, a Return/Break/Continue, or
    // a panic that unwound through several nested calls whose own scope
    // restores never got to run -- the try block's scope is done with now,
    // so it's reset directly to what it was before entering `try`.
    env.scope = Rc::clone(&outer_scope);

    let outcome = match try_result {
        Ok(sig) => sig,
        Err(payload) => {
            let msg = panic_payload_to_string(payload);
            env.scope = Scope::new(Some(Rc::clone(&outer_scope)));
            if let Some(var_name) = node.name.as_deref() {
                Scope::set_declare(&env.scope, var_name, Value::Str(msg), false);
            }
            let mut sig = Signal::None;
            for stmt in &node.else_body {
                match visit_stmt(env, stmt, self_obj.clone()) {
                    Signal::None => {}
                    s => { sig = s; break; }
                }
            }
            env.scope = Rc::clone(&outer_scope);
            sig
        }
    };

    if !node.finally_body.is_empty() {
        env.scope = Scope::new(Some(Rc::clone(&outer_scope)));
        let mut finally_sig = Signal::None;
        for stmt in &node.finally_body {
            match visit_stmt(env, stmt, self_obj.clone()) {
                Signal::None => {}
                s => { finally_sig = s; break; }
            }
        }
        env.scope = Rc::clone(&outer_scope);
        if !matches!(finally_sig, Signal::None) {
            return finally_sig;
        }
    }

    outcome
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
            Scope::set_declare(&env.scope, node.name.as_deref().unwrap(), v, node.is_const());
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
        NodeType::EnumDef => {
            env.enums.insert(node.name.clone().unwrap(), node.params.clone());
            Signal::None
        }
        NodeType::Try => visit_try(env, node, self_obj),
        NodeType::Throw => {
            let v = eval(env, node.expr.as_ref().unwrap(), self_obj);
            runtime_error(v.to_string_repr())
        }
        NodeType::If => {
            let cond = eval(env, node.expr.as_ref().unwrap(), self_obj.clone());
            if cond.truthy() {
                for stmt in &node.body {
                    match visit_stmt(env, stmt, self_obj.clone()) {
                        Signal::None => {}
                        s => return s,
                    }
                }
            } else {
                let mut handled = false;
                for (ei, ec) in node.else_if_conds.iter().enumerate() {
                    let ecv = eval(env, ec, self_obj.clone());
                    if ecv.truthy() {
                        for stmt in &node.else_if_bodies[ei] {
                            match visit_stmt(env, stmt, self_obj.clone()) {
                                Signal::None => {}
                                s => return s,
                            }
                        }
                        handled = true;
                        break;
                    }
                }
                if !handled {
                    for stmt in &node.else_body {
                        match visit_stmt(env, stmt, self_obj.clone()) {
                            Signal::None => {}
                            s => return s,
                        }
                    }
                }
            }
            Signal::None
        }
        NodeType::While => {
            let outer = Rc::clone(&env.scope);
            'while_loop: loop {
                let cond = eval(env, node.expr.as_ref().unwrap(), self_obj.clone());
                if !cond.truthy() { break; }
                env.scope = Scope::new(Some(Rc::clone(&outer)));
                for stmt in &node.body {
                    match visit_stmt(env, stmt, self_obj.clone()) {
                        Signal::Return(v) => { env.scope = Rc::clone(&outer); return Signal::Return(v); }
                        Signal::Break    => { env.scope = Rc::clone(&outer); break 'while_loop; }
                        Signal::Continue => { env.scope = Rc::clone(&outer); continue 'while_loop; }
                        Signal::None     => {}
                    }
                }
                env.scope = Rc::clone(&outer);
            }
            Signal::None
        }
        NodeType::For => {
            if node.is_method() {
                let iter = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
                let items = match iter {
                    Value::List(ref l) => l.borrow().clone(),
                    _ => { runtime_error(format!("Runtime Error: for-in requires a list")); }
                };
                'for_in: for item in items {
                    let saved = Rc::clone(&env.scope);
                    env.scope = Scope::new(Some(Rc::clone(&saved)));
                    Scope::set_declare(&env.scope, node.name.as_deref().unwrap(), item, false);
                    for stmt in &node.body {
                        match visit_stmt(env, stmt, self_obj.clone()) {
                            Signal::Return(v) => { env.scope = saved; return Signal::Return(v); }
                            Signal::Break     => { env.scope = saved; break 'for_in; }
                            Signal::Continue  => { env.scope = saved; continue 'for_in; }
                            Signal::None      => {}
                        }
                    }
                    env.scope = saved;
                }
            } else {
                let saved = Rc::clone(&env.scope);
                env.scope = Scope::new(Some(Rc::clone(&saved)));
                let init_val = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
                Scope::set_declare(&env.scope, node.name.as_deref().unwrap(), init_val, false);
                'for_c: loop {
                    let cond = eval(env, node.expr.as_ref().unwrap(), self_obj.clone());
                    if !cond.truthy() { break; }
                    let body_saved = Rc::clone(&env.scope);
                    env.scope = Scope::new(Some(Rc::clone(&body_saved)));
                    let mut do_continue = false;
                    for stmt in &node.body {
                        match visit_stmt(env, stmt, self_obj.clone()) {
                            Signal::Return(v) => {
                                env.scope = Rc::clone(&saved); return Signal::Return(v);
                            }
                            Signal::Break    => {
                                env.scope = Rc::clone(&saved); break 'for_c;
                            }
                            Signal::Continue => { do_continue = true; break; }
                            Signal::None     => {}
                        }
                    }
                    env.scope = body_saved;
                    // Always run the post-step, even on continue
                    visit_stmt(env, node.right.as_ref().unwrap(), self_obj.clone());
                    if !do_continue {
                        // normal iteration — post-step already ran above
                    }
                }
                env.scope = saved;
            }
            Signal::None
        }
        NodeType::MemberAssign => {
            let obj_val = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
            if let Value::Object(ref obj_rc) = obj_val {
                let caller = self_obj.clone();
                let new_val = eval(env, node.expr.as_ref().unwrap(), self_obj);
                let name = node.name.as_deref().unwrap();

                // Check visibility/existence with a short-lived immutable
                // borrow first, and let it fully drop before taking the
                // mutable borrow below -- `obj_rc` and `caller` can be the
                // very same object (e.g. inside `self.field = ...`), and
                // same_class() also needs to borrow it, so overlapping the
                // two would panic with "already mutably borrowed".
                let (class_name, is_private_field, field_exists) = {
                    let obj = obj_rc.borrow();
                    let class_name = obj.class_name.clone();
                    match obj.find_field(name) {
                        Some(f) => (class_name, f.is_private, true),
                        None => (class_name, false, false),
                    }
                };
                if !field_exists {
                    runtime_error(format!("Runtime Error: Field '{}' not found on '{}'", name, class_name));
                }
                if is_private_field && !same_class(&caller, &class_name) {
                    runtime_error(format!("Runtime Error: Field '{}' is private on '{}' and cannot be assigned from here",
                        name, class_name));
                }

                let mut obj = obj_rc.borrow_mut();
                if let Some(f) = obj.find_field_mut(name) {
                    if f.is_const {
                        runtime_error(format!("Runtime Error: Cannot assign to const field '{}'", name));
                    }
                    f.val = new_val;
                }
            } else {
                runtime_error(format!("Runtime Error: Member assignment on non-object"));
            }
            Signal::None
        }
        NodeType::IndexAssign => {
            let lst = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
            let idx = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            let newval = eval(env, node.expr.as_ref().unwrap(), self_obj);
            match lst {
                Value::List(ref l) => {
                    let mut items = l.borrow_mut();
                    let i = idx.as_int() as usize;
                    if i >= items.len() { runtime_error(format!("Runtime Error: List index out of range")); }
                    items[i] = newval;
                }
                Value::Map(ref m) => {
                    let key = idx.to_string_repr();
                    m.borrow_mut().insert(key, newval);
                }
                _ => { runtime_error(format!("Runtime Error: Cannot index-assign on non-list/map")); }
            }
            Signal::None
        }
        NodeType::Break    => Signal::Break,
        NodeType::Continue => Signal::Continue,
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
        NodeType::Imaginary => Value::Comp(Complex::new(0.0, node.fval)),
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

        NodeType::Await => {
            let v = eval(env, node.left.as_ref().unwrap(), self_obj);
            resolve_task(env, v)
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
                        runtime_error(format!("Runtime Error: List index {} out of range", i));
                    }
                    items[i as usize].clone()
                }
                Value::Str(ref s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let mut i = idx.as_int();
                    if i < 0 { i = chars.len() as i64 + i; }
                    if i < 0 || i >= chars.len() as i64 {
                        runtime_error(format!("Runtime Error: String index out of range"));
                    }
                    Value::Str(chars[i as usize].to_string())
                }
                Value::Map(ref m) => {
                    let key = idx.to_string_repr();
                    m.borrow().get(&key).cloned().unwrap_or_else(|| {
                        runtime_error(format!("Runtime Error: Map key '{}' not found", key));
                    })
                }
                _ => { runtime_error(format!("Runtime Error: Cannot index into that type")); }
            }
        }

        NodeType::BinaryOp => {
            // Conservative native fast path; unsupported trees retain interpreter semantics.
            if jit_backend_enabled() {
                if let Some(value) = jit::try_eval_integer_constant(node) {
                    return Value::Int(value);
                }
            }
            let lv = eval(env, node.left.as_ref().unwrap(), self_obj.clone());
            let rv = eval(env, node.right.as_ref().unwrap(), self_obj);
            let op = node.str_val.as_deref().unwrap();
            let is_comp = matches!(lv, Value::Comp(_)) || matches!(rv, Value::Comp(_));
            if op == "+" {
                if matches!(lv, Value::Str(_)) || matches!(rv, Value::Str(_)) {
                    return Value::Str(format!("{}{}", lv.to_string_repr(), rv.to_string_repr()));
                }
                if is_comp {
                    let (l, r) = (lv.as_complex(), rv.as_complex());
                    return Value::Comp(Complex::new(l.real + r.real, l.imag + r.imag));
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
                    if is_comp {
                        let (l, r) = (lv.as_complex(), rv.as_complex());
                        return Value::Comp(Complex::new(l.real - r.real, l.imag - r.imag));
                    }
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
                    if is_comp {
                        let (l, r) = (lv.as_complex(), rv.as_complex());
                        return Value::Comp(Complex::new(
                            l.real * r.real - l.imag * r.imag,
                            l.real * r.imag + l.imag * r.real,
                        ));
                    }
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
                    if is_comp {
                        let (l, r) = (lv.as_complex(), rv.as_complex());
                        let denom = r.real * r.real + r.imag * r.imag;
                        if denom == 0.0 { runtime_error(format!("Runtime Error: Division by zero")); }
                        return Value::Comp(Complex::new(
                            (l.real * r.real + l.imag * r.imag) / denom,
                            (l.imag * r.real - l.real * r.imag) / denom,
                        ));
                    }
                    if rv.as_float() == 0.0 { runtime_error(format!("Runtime Error: Division by zero")); }
                    Value::Float(lv.as_float() / rv.as_float())
                }
                "%" => {
                    if rv.as_int() == 0 { runtime_error(format!("Runtime Error: Modulo by zero")); }
                    Value::Int(lv.as_int() % rv.as_int())
                }
                // Bugfix: these used to always compare via as_int() (even
                // for two Floats), so e.g. `3.9 > 3.1` silently truncated
                // both sides to `3` first and evaluated to false. Now
                // Floats compare as floats, and Complex numbers -- which
                // have no natural total order -- support == / != but give
                // a clear error on </>/<=/>= instead of a meaningless
                // real-part-only comparison.
                "==" => {
                    if let (Value::Str(ls), Value::Str(rs)) = (&lv, &rv) {
                        Value::Int(if ls == rs { 1 } else { 0 })
                    } else if let (Value::Enum(lt, lvar), Value::Enum(rt, rvar)) = (&lv, &rv) {
                        Value::Int(if lt == rt && lvar == rvar { 1 } else { 0 })
                    } else if is_comp {
                        let (l, r) = (lv.as_complex(), rv.as_complex());
                        Value::Int(if l.real == r.real && l.imag == r.imag { 1 } else { 0 })
                    } else if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                        Value::Int(if lv.as_float() == rv.as_float() { 1 } else { 0 })
                    } else { Value::Int(if lv.as_int() == rv.as_int() { 1 } else { 0 }) }
                }
                "!=" => {
                    if let (Value::Str(ls), Value::Str(rs)) = (&lv, &rv) {
                        Value::Int(if ls != rs { 1 } else { 0 })
                    } else if let (Value::Enum(lt, lvar), Value::Enum(rt, rvar)) = (&lv, &rv) {
                        Value::Int(if lt != rt || lvar != rvar { 1 } else { 0 })
                    } else if is_comp {
                        let (l, r) = (lv.as_complex(), rv.as_complex());
                        Value::Int(if l.real != r.real || l.imag != r.imag { 1 } else { 0 })
                    } else if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                        Value::Int(if lv.as_float() != rv.as_float() { 1 } else { 0 })
                    } else { Value::Int(if lv.as_int() != rv.as_int() { 1 } else { 0 }) }
                }
                "<" | ">" | "<=" | ">=" => {
                    if is_comp {
                        runtime_error(format!("Runtime Error: '{}' is not supported between complex numbers (no natural ordering)", op));
                    }
                    let (l, r) = if matches!(lv, Value::Float(_)) || matches!(rv, Value::Float(_)) {
                        (lv.as_float(), rv.as_float())
                    } else {
                        (lv.as_int() as f64, rv.as_int() as f64)
                    };
                    let b = match op { "<" => l < r, ">" => l > r, "<=" => l <= r, _ => l >= r };
                    Value::Int(if b { 1 } else { 0 })
                }
                _    => { runtime_error(format!("Runtime Error: Unknown operator '{}'", op)); }
            }
        }

        NodeType::UnaryOp => {
            let v = eval(env, node.left.as_ref().unwrap(), self_obj);
            match node.str_val.as_deref().unwrap() {
                // Bugfix: this used to always do Value::Int(-v.as_int()),
                // so negating a Float truncated it to an integer first --
                // e.g. -3.5 evaluated to the Int -3 instead of the Float
                // -3.5. Now negation preserves the operand's type.
                "-" => match &v {
                    Value::Float(f) => Value::Float(-f),
                    Value::Comp(c)  => Value::Comp(Complex::new(-c.real, -c.imag)),
                    _ => Value::Int(-v.as_int()),
                },
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
                _ => { runtime_error(format!("Runtime Error: Unknown logical op")); }
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
                runtime_error(format!("Runtime Error: Undefined class '{}'", class_name));
            });
            let fields: Vec<PJField> = cls.fields.iter().map(|fd| {
                let fv = if let Some(ref e) = fd.expr { eval(env, e, None) } else { Value::Null };
                PJField { name: fd.name.clone().unwrap(), val: fv, is_const: fd.is_const(), is_private: fd.is_private() }
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
                if ctor.is_private() && !same_class(&self_obj, class_name) {
                    runtime_error(format!("Runtime Error: constructor of '{}' is private and cannot be called from here", class_name));
                }
                call_func_with_vals(env, ctor, arg_vals, Some(Rc::clone(&obj)));
            } else if !node.args.is_empty() {
                runtime_error(format!("Runtime Error: Class '{}' has no constructor but got args", class_name));
            }
            Value::Object(obj)
        }

        NodeType::MemberAccess => {
            // `EnumName.VARIANT` -- checked *before* evaluating `node.object`
            // as an ordinary expression, since an enum type name (e.g.
            // `Color`) isn't a variable and evaluating it as one would just
            // fail with "Undefined variable".
            if let NodeType::VarAccess = node.object.as_ref().unwrap().typ {
                let obj_name = node.object.as_ref().unwrap().name.as_deref().unwrap();
                if let Some(variants) = env.enums.get(obj_name).cloned() {
                    let member_name = node.name.as_deref().unwrap();
                    if !variants.iter().any(|v| v == member_name) {
                        runtime_error(format!(
                            "Runtime Error: enum '{}' has no variant '{}'", obj_name, member_name));
                    }
                    return Value::Enum(obj_name.to_string(), member_name.to_string());
                }
            }

            let obj_val = eval(env, node.object.as_ref().unwrap(), self_obj.clone());
            let member_name = node.name.as_deref().unwrap();
            match obj_val {
                // Java-style enum convenience methods: `Color.RED.name()` and
                // `Color.RED.ordinal()`.
                Value::Enum(ref enum_name, ref variant) => match member_name {
                    "name" => Value::Str(variant.clone()),
                    "ordinal" => {
                        let variants = env.enums.get(enum_name).cloned().unwrap_or_default();
                        let idx = variants.iter().position(|v| v == variant).unwrap_or(0);
                        Value::Int(idx as i64)
                    }
                    _ => runtime_error(format!(
                        "Runtime Error: enum '{}' has no member '{}'", enum_name, member_name)),
                },
                Value::List(ref l) => {
                    if node.is_call() {
                        let avals: Vec<Value> = node.args.iter()
                            .map(|a| eval(env, a, self_obj.clone()))
                            .collect();
                        match member_name {
                            "append" => {
                                if avals.len() != 1 { runtime_error(format!("append() takes 1 arg")); }
                                l.borrow_mut().push(avals.into_iter().next().unwrap());
                                Value::Null
                            }
                            "pop" => {
                                let mut items = l.borrow_mut();
                                if items.is_empty() { runtime_error(format!("Runtime Error: pop from empty list")); }
                                let idx = if !avals.is_empty() { avals[0].as_int() as usize } else { items.len() - 1 };
                                items.remove(idx)
                            }
                            "remove" => {
                                if avals.len() != 1 { runtime_error(format!("remove() takes 1 arg")); }
                                let mut items = l.borrow_mut();
                                let pos = items.iter().position(|v| match (v, &avals[0]) {
                                    (Value::Str(a), Value::Str(b)) => a == b,
                                    (a, b) => a.as_int() == b.as_int(),
                                });
                                match pos {
                                    Some(i) => { items.remove(i); Value::Null }
                                    None => { runtime_error(format!("Runtime Error: remove: value not in list")); }
                                }
                            }
                            "len" | "length" => Value::Int(l.borrow().len() as i64),
                            "contains" => {
                                if avals.len() != 1 { runtime_error(format!("contains() takes 1 arg")); }
                                let found = l.borrow().iter().any(|v| match (v, &avals[0]) {
                                    (Value::Str(a), Value::Str(b)) => a == b,
                                    (a, b) => a.as_int() == b.as_int(),
                                });
                                Value::Int(if found { 1 } else { 0 })
                            }
                            _ => { runtime_error(format!("Runtime Error: List has no method '{}'", member_name)); }
                        }
                    } else {
                        match member_name {
                            "len" | "length" => Value::Int(l.borrow().len() as i64),
                            _ => { runtime_error(format!("Runtime Error: List has no field '{}'", member_name)); }
                        }
                    }
                }
                Value::Object(ref obj_rc) => {
                    if node.is_call() {
                        let class_name = obj_rc.borrow().class_name.clone();
                        if let Some(v) = builtins::eval_builtin_module_method(
                            class_name.as_str(), member_name, &node.args, env, self_obj.clone())
                        {
                            return v;
                        }
                        let method = { let obj = obj_rc.borrow(); obj.find_method(member_name).cloned() };
                        let method = method.unwrap_or_else(|| {
                            runtime_error(format!("Runtime Error: Method '{}' not found on '{}'",
                                member_name, obj_rc.borrow().class_name));
                        });
                        if method.is_private() && !same_class(&self_obj, &class_name) {
                            runtime_error(format!("Runtime Error: Method '{}' is private on '{}' and cannot be called from here",
                                member_name, class_name));
                        }
                        let avals: Vec<Value> = node.args.iter()
                            .map(|a| eval(env, a, self_obj.clone()))
                            .collect();
                        if method.is_async() {
                            make_pending_task(method, avals, Some(Rc::clone(obj_rc)))
                        } else {
                            call_func_with_vals(env, &method, avals, Some(Rc::clone(obj_rc)))
                        }
                    } else {
                        let (field_val, is_private_field, class_name) = {
                            let obj = obj_rc.borrow();
                            let field = obj.find_field(member_name).unwrap_or_else(|| {
                                runtime_error(format!("Runtime Error: Field '{}' not found", member_name));
                            });
                            (field.val.clone(), field.is_private, obj.class_name.clone())
                        };
                        if is_private_field && !same_class(&self_obj, &class_name) {
                            runtime_error(format!("Runtime Error: Field '{}' is private on '{}' and cannot be accessed from here",
                                member_name, class_name));
                        }
                        field_val
                    }
                }
                _ => { runtime_error(format!("Runtime Error: Member access on non-object/list")); }
            }
        }


        NodeType::Dict => {
            use rustc_hash::FxHashMap;
            let mut map: FxHashMap<String, Value> = FxHashMap::default();
            // args are interleaved: [key_node, val_node, key_node, val_node, ...]
            let mut i = 0;
            while i + 1 < node.args.len() {
                let key = node.args[i].str_val.clone().unwrap_or_default();
                let val = eval(env, &node.args[i + 1], self_obj.clone());
                map.insert(key, val);
                i += 2;
            }
            Value::Map(Rc::new(RefCell::new(map)))
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
        } else if stmt.typ == NodeType::EnumDef {
            env.enums.insert(stmt.name.clone().unwrap(), stmt.params.clone());
        }
    }
    for stmt in &program.body {
        if stmt.typ != NodeType::FuncDef && stmt.typ != NodeType::ClassDef && stmt.typ != NodeType::EnumDef {
            if let Signal::Return(_) = visit_stmt(env, stmt, None) { break; }
        }
    }
}

/// Runs `program`, catching any runtime error that escapes every `try` in
/// the user's own code (i.e. one with no enclosing `try` at all). Returns
/// `Err(message)` in that case instead of letting the panic take the whole
/// process down uncontrolled, so callers can decide what "uncaught" should
/// mean for them: a script run from the command line still exits(1) just
/// like before `try`/`except` existed (see `interpret_file`), while the
/// interactive CLI can print the error and simply keep prompting instead of
/// dying on the first mistyped line (see `interpret_repl`).
fn run_catching(env: &mut Env, program: &AstNode) -> Result<(), String> {
    let payload = std::panic::AssertUnwindSafe((&mut *env, program));
    let result = std::panic::catch_unwind(move || {
        let payload = payload;
        let std::panic::AssertUnwindSafe((env, program)) = payload;
        run(env, program);
    });
    result.map_err(panic_payload_to_string)
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
        Err(_) => { if debug { println!("[import] Skipping '{}' (cannot open)", path); } return; }
    };
    let tokens = tokenize(&src);
    // Resolve any builtin imports declared at the top of the package file.
    for (pkg_name, alias) in collect_imports(&tokens) {
        if BUILTIN_MODULE_NAMES.contains(&pkg_name.as_str()) {
            load_folder_package(env, &pkg_name, alias.as_deref(), debug);
        } else {
            eprintln!("Import Error: package files may only import built-in modules, not '{}'", pkg_name);
            process::exit(1);
        }
    }
    let pkg_name = match detect_package(&tokens) {
        Some(p) => p,
        None => { if debug { println!("[import] Skipping '{}' (no package declaration)", path); } return; }
    };
    if pkg_name != target_pkg {
        if debug { println!("[import] Skipping '{}' (package '{}' != target '{}')", path, pkg_name, target_pkg); }
        return;
    }
    if debug { println!("[import] Loading package '{}' from '{}'", pkg_name, path); }
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
    if BUILTIN_MODULE_NAMES.contains(&pkg_name) {
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
                    method.set_is_method(false);
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
        eprintln!("Error: '{}' is a package file (package {}) and cannot be run directly.", path, pkg); process::exit(1);
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
    if let Err(msg) = run_catching(&mut env, &prog) {
        eprintln!("{}", msg);
        process::exit(1);
    }
}

pub fn interpret(code: &str, debug: bool) {
    let tokens = tokenize(code);
    if let Some(pkg) = detect_package(&tokens) {
        eprintln!("Error: code is a package file (package {}) and cannot be run directly.", pkg); process::exit(1);
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
    if let Err(msg) = run_catching(&mut env, &prog) {
        eprintln!("{}", msg);
        process::exit(1);
    }
}

/// Like `interpret`, but runs against an existing `Env` instead of creating
/// a fresh one, so variables, functions, and classes declared in one call
/// are still there for the next one. This is what makes the interactive
/// CLI able to "save" variables between lines: the caller keeps one `Env`
/// alive for the whole session and passes it to every line/block typed in.
pub fn interpret_repl(env: &mut Env, code: &str, debug: bool) {
    let tokens = tokenize(code);
    if let Some(pkg) = detect_package(&tokens) {
        eprintln!("Error: code is a package file (package {}) and cannot be run directly.", pkg);
        return;
    }
    if debug {
        println!("=== TOKENS ===");
        for (i, t) in tokens.iter().enumerate() { println!("  [{}] type={:?} value={}", i, t.typ, t.value); }
        println!("==============");
    }
    let imports = collect_imports(&tokens);
    for (pkg_name, alias) in &imports {
        load_folder_package(env, pkg_name, alias.as_deref(), debug);
    }
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program();
    if debug { println!("=== Parse OK ==="); }
    // Bugfix: this used to call `run` directly, so any runtime error typed
    // into the interactive CLI (dividing by zero, a typo'd variable, ...)
    // would process::exit(1) the *entire session* -- every variable typed
    // so far, gone, over one mistake. Now the error is caught, printed, and
    // the CLI just keeps prompting, exactly like a real REPL should.
    if let Err(msg) = run_catching(env, &prog) {
        eprintln!("{}", msg);
    }
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
    println!("  cli               open cli mode");
    println!("  version           show payjar version");
    println!("  help              show this help page");
    println!("  autorun           autorun all pj files");
    println!("    -s <file>       skip <file> in autorun");
    println!("  -d <file>.pj      enable debug");
    println!("  <file>.pj         interpret <file>.pj");
}
