/// Represents a complete LPPO program consisting of zero or more modules and procedures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub modules: Vec<Module>,
    pub procedures: Vec<Procedure>,
}

/// Represents a module for namespace organization
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub procedures: Vec<Procedure>,
}

/// Type annotations required for all variables and parameters
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypeAnnotation {
    Int64,
    Bool,
    Str,
    Arr(Box<TypeAnnotation>), // Array of specific type
    Void,
    Ref(Box<TypeAnnotation>, Option<String>), // Reference with optional lifetime
    Own(Box<TypeAnnotation>), // Owned value
    Mut(Box<TypeAnnotation>), // Mutable reference
}

/// Parameter with explicit type and ownership annotations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub typ: TypeAnnotation,
    pub lifetime: Option<String>,
}

/// Represents a user-defined procedure with explicit return type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Procedure {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: TypeAnnotation,
    pub lifetimes: Vec<String>, // Lifetime parameters like 'a, 'b
    pub statements: Vec<Statement>,
    pub is_public: bool,
    pub is_unsafe: bool,
}

/// Represents an executable statement within a procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// Emits a textual message when executed.
    /// Emit one or more expressions (string or numeric) to output.
    Emit(Vec<Expr>),
    /// Executes several emit expressions in parallel.
    Parallel(Vec<Expr>),
    /// Calls another procedure by name (possibly qualified), with optional argument expressions.
    Call(QualifiedName, Vec<Expr>),
    /// Declare a local variable with EXPLICIT type: let name: Type = expr;
    Let {
        name: String,
        typ: TypeAnnotation,
        expr: Expr,
        is_mut: bool,
        ownership: OwnershipKind,
    },
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
        typ: TypeAnnotation,
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
    /// Memory allocation: alloc<Type> on heap/stack
    Alloc {
        var: String,
        typ: TypeAnnotation,
        location: MemoryLocation,
        size: Expr, // For arrays
    },
    /// Memory deallocation: dealloc var;
    Dealloc(String),
    /// Move ownership: move var to target
    Move {
        source: String,
        target: String,
    },
    /// Copy value: copy var to target (requires Copy trait)
    Copy {
        source: String,
        target: String,
    },
    /// Explicit scope block
    Scope {
        name: Option<String>,
        body: Vec<Statement>,
    },
    /// Unsafe block
    Unsafe {
        body: Vec<Statement>,
    },
}

/// Qualified name for namespaced identifiers like module::submodule::name
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualifiedName {
    pub path: Vec<String>,
    pub name: String,
}

/// Memory location specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryLocation {
    Heap,
    Stack,
}

/// Ownership kind for variables
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipKind {
    Own,     // Owned value
    Ref,     // Immutable reference
    MutRef,  // Mutable reference
    Borrow,  // Borrowed value
}

/// Expression nodes for simple arithmetic and string literals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    Var(String),
    /// Qualified variable access: module::submodule::var
    QualifiedVar(QualifiedName),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// Array literal with explicit type: [expr, expr, ...]: Arr<Type>
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
    /// Reference operation: ref var or ref<'a> var
    Ref {
        target: Box<Expr>,
        lifetime: Option<String>,
        is_mut: bool,
    },
    /// Dereference operation: deref expr
    Deref(Box<Expr>),
    /// Size of type: sizeof<Type>
    SizeOf(TypeAnnotation),
    /// Alignment of type: alignof<Type>
    AlignOf(TypeAnnotation),
    /// Type cast: cast<ToType>(expr)
    Cast {
        expr: Box<Expr>,
        to_type: TypeAnnotation,
    },
    /// Borrow operation: borrow<'a> var
    Borrow {
        target: Box<Expr>,
        lifetime: Option<String>,
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
