/// Represents a complete LPPO program consisting of zero or more procedures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub procedures: Vec<Procedure>,
}

/// Represents a user-defined procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Procedure {
    pub name: String,
    pub params: Vec<String>,
    pub statements: Vec<Statement>,
}

/// Represents an executable statement within a procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// Emits a textual message when executed.
    /// Emit one or more expressions (string or numeric) to output.
    Emit(Vec<Expr>),
    /// Executes several emit expressions in parallel.
    Parallel(Vec<Expr>),
    /// Calls another procedure by name, with optional argument expressions.
    Call(String, Vec<Expr>),
    /// Declare a local variable: let name = expr;
    Let(String, Expr),
    /// Conditional branching: if/elif/else
    If {
        cond: Expr,
        then_branch: Vec<Statement>,
        elifs: Vec<(Expr, Vec<Statement>)>,
        else_branch: Option<Vec<Statement>>,
    },
    /// While loop: while cond { ... }
    While {
        cond: Expr,
        body: Vec<Statement>,
    },
    /// For-in loop: for item in expr { ... }
    For {
        var: String,
        iterable: Expr,
        body: Vec<Statement>,
    },
    /// Infinite loop: loop { ... }
    Loop {
        body: Vec<Statement>,
    },
    /// Break out of loop
    Break,
    /// Continue to next loop iteration
    Continue,
    /// Return from procedure with optional value
    Return(Option<Expr>),
}

/// Expression nodes for simple arithmetic and string literals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    Var(String),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// Array literal: [expr, expr, ...]
    Array(Vec<Expr>),
    /// Array/string indexing: expr[index]
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    /// Method call: expr.method(args)
    MethodCall {
        target: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    /// Field access: expr.field
    FieldAccess {
        target: Box<Expr>,
        field: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
    BitNot,
}
