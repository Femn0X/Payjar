/// All node kinds the parser can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Import, MainDef, FuncDef, ClassDef,EnumDef,EnumBody,
    VarDecl, Assign, CompoundAssign, FieldDecl,
    Print, Perror, Return,
    If, While,
    FuncCall, MemberAccess, MemberAssign,
    NewExpr, ObjectCreation,
    BinaryOp, UnaryOp, LogicalOp, Index, IndexAssign,
    Int, Float, RegexString, Str, Array, Null, TemplateStr,
    VarAccess, Readln, Readi, Readf, For, Generic,
    Break, Continue,
    Dict,
    Await,
    Imaginary,
    /// `try { ... } except (e) { ... }` — C-style exception handling.
    /// `body` holds the try block, `else_body` holds the except/catch
    /// block, and `name` holds the caught-exception variable's identifier.
    Try,
    /// `throw expr;` — raises `expr` as a catchable exception.
    Throw,
}

#[derive(Debug, Clone)]
pub struct AstNode {
    pub typ: NodeType,
    pub int_val: i64,
    pub fval: f64,
    /// Flags packed: bit0=is_method, bit1=is_const, bit2=is_call, bit3=is_async, bit4=is_private
    pub flags: u8,
    pub own_type: String,
    pub var_type: String,
    pub str_val: Option<String>,
    pub name: Option<String>,
    pub left: Option<Box<AstNode>>,
    pub right: Option<Box<AstNode>>,
    pub expr: Option<Box<AstNode>>,
    pub object: Option<Box<AstNode>>,
    pub constructor: Option<Box<AstNode>>,
    pub body: Vec<AstNode>,
    pub args: Vec<AstNode>,
    pub else_if_conds: Vec<AstNode>,
    pub else_if_bodies: Vec<Vec<AstNode>>,
    pub else_body: Vec<AstNode>,
    /// `finally` block statements for a `Try` node (empty if there is none).
    pub finally_body: Vec<AstNode>,
    pub fields: Vec<AstNode>,
    pub methods: Vec<AstNode>,
    pub template_parts: Vec<AstNode>,
    pub params: Vec<String>,
}

impl AstNode {
    #[inline]
    pub fn new(typ: NodeType) -> Self {
        AstNode {
            typ,
            int_val: 0,
            fval: 0.0,
            flags: 0,
            own_type: String::new(),
            var_type: String::new(),
            str_val: None,
            name: None,
            left: None,
            right: None,
            expr: None,
            object: None,
            constructor: None,
            body: Vec::new(),
            args: Vec::new(),
            else_if_conds: Vec::new(),
            else_if_bodies: Vec::new(),
            else_body: Vec::new(),
            finally_body: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            template_parts: Vec::new(),
            params: Vec::new(),
        }
    }

    // ── flag accessors ────────────────────────────────────────
    #[inline(always)] pub fn is_method(&self) -> bool { self.flags & 1 != 0 }
    #[inline(always)] pub fn is_const(&self)  -> bool { self.flags & 2 != 0 }
    #[inline(always)] pub fn is_call(&self)   -> bool { self.flags & 4 != 0 }
    #[inline(always)] pub fn is_async(&self)  -> bool { self.flags & 8 != 0 }
    #[inline(always)] pub fn is_private(&self) -> bool { self.flags & 16 != 0 }

    #[inline(always)] pub fn set_is_method(&mut self, v: bool) { if v { self.flags |= 1 } else { self.flags &= !1 } }
    #[inline(always)] pub fn set_is_const(&mut self,  v: bool) { if v { self.flags |= 2 } else { self.flags &= !2 } }
    #[inline(always)] pub fn set_is_call(&mut self,   v: bool) { if v { self.flags |= 4 } else { self.flags &= !4 } }
    #[inline(always)] pub fn set_is_async(&mut self,  v: bool) { if v { self.flags |= 8 } else { self.flags &= !8 } }
    #[inline(always)] pub fn set_is_private(&mut self, v: bool) { if v { self.flags |= 16 } else { self.flags &= !16 } }
}
