use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I64,
    F64,
    Str,
    Bool,
    None,
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Unit,
    Unknown,
    Any,
    Value,
    Result(Box<Type>),
    FnPtr(Vec<Type>, Box<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I64 => write!(f, "i64"),
            Type::F64 => write!(f, "f64"),
            Type::Str => write!(f, "String"),
            Type::Bool => write!(f, "bool"),
            Type::None => write!(f, "()"),
            Type::List(t) => write!(f, "Vec<{}>", t),
            Type::Dict(k, v) => write!(f, "std::collections::HashMap<{}, {}>", k, v),
            Type::Tuple(ts) => {
                let inner: Vec<String> = ts.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", inner.join(", "))
            }
            Type::Unit => write!(f, "()"),
            Type::Unknown => write!(f, "_"),
            Type::Any => write!(f, "Box<dyn std::any::Any>"),
            Type::Value => write!(f, "serde_json::Value"),
            Type::Result(t) => write!(f, "Result<{}, String>", t),
            Type::FnPtr(params, ret) => {
                let inner: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                write!(f, "fn({}) -> {}", inner.join(", "), ret)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    In,
    NotIn,
    Is,
    IsNot,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum ComprehensionKind {
    List,
    Dict,
}

#[derive(Debug, Clone)]
pub struct Comprehension {
    pub kind: ComprehensionKind,
    pub element: Box<Expr>,
    pub key: Option<Box<Expr>>,
    pub generators: Vec<CompGenerator>,
}

#[derive(Debug, Clone)]
pub struct CompGenerator {
    pub var: String,
    pub iter: Box<Expr>,
    pub cond: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),
    BoolLit(bool),
    NoneLit,
    Ident(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    FuncCall(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Attribute(Box<Expr>, String),
    Subscript(Box<Expr>, Box<Expr>),
    Slice(Option<Box<Expr>>, Option<Box<Expr>>, Option<Box<Expr>>),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Tuple(Vec<Expr>),
    Starred(Box<Expr>),
    Comp(Comprehension),
    Lambda(Vec<String>, Box<Expr>),
    FStr(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub exception: String,
    pub var: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Assign(String, Expr, bool),
    AssignTuple(Vec<String>, Expr),
    AugAssign(String, BinOp, Expr),
    If(Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    For(Vec<String>, Expr, Vec<Stmt>),
    FuncDef {
        name: String,
        params: Vec<(String, Option<Type>)>,
        body: Vec<Stmt>,
        return_type: Option<Type>,
        decorators: Vec<String>,
    },
    ClassDef {
        name: String,
        bases: Vec<String>,
        body: Vec<Stmt>,
        decorators: Vec<String>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Pass,
    Import(String, Option<String>),
    FromImport(String, Vec<(String, Option<String>)>),
    Try {
        body: Vec<Stmt>,
        handlers: Vec<Handler>,
        else_body: Option<Vec<Stmt>>,
        finally_body: Option<Vec<Stmt>>,
    },
    Raise(Option<Expr>),
    With {
        expr: Expr,
        var: Option<String>,
        body: Vec<Stmt>,
    },
    Delete(Vec<String>),
    EmbedRust(String),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}
