/// All node kinds the parser can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Import, MainDef, FuncDef, ClassDef,
    VarDecl, Assign, CompoundAssign, FieldDecl,
    Print, Perror, Return,
    If, While,
    FuncCall, MemberAccess, MemberAssign,
    NewExpr, ObjectCreation,
    BinaryOp, UnaryOp, LogicalOp, Index, IndexAssign,
    Int, Float, RegexString, Str, Array, Null, TemplateStr,
    VarAccess, Readln, Readi, Readf, For, Generic,
}

#[derive(Debug, Clone)]
pub struct AstNode {
    pub own_type: String,
    pub var_type: String,
    pub typ: NodeType,
    pub int_val: i64,
    pub fval: f64,
    pub str_val: Option<String>,
    pub left: Option<Box<AstNode>>,
    pub right: Option<Box<AstNode>>,
    pub expr: Option<Box<AstNode>>,
    pub object: Option<Box<AstNode>>,
    pub body: Vec<AstNode>,
    pub args: Vec<AstNode>,
    pub else_if_conds: Vec<AstNode>,
    pub else_if_bodies: Vec<Vec<AstNode>>,
    pub else_body: Vec<AstNode>,
    pub fields: Vec<AstNode>,
    pub methods: Vec<AstNode>,
    pub template_parts: Vec<AstNode>,
    pub params: Vec<String>,
    pub is_method: bool,
    pub is_const: bool,
    pub is_call: bool,
    pub name: Option<String>,
    pub constructor: Option<Box<AstNode>>,
}

impl AstNode {
    pub fn new(typ: NodeType) -> Self {
        AstNode {
            own_type: "".to_string(),
            var_type: "".to_string(),
            typ,
            int_val: 0,
            fval: 0.0,
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
