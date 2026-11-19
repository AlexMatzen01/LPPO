/// Represents a complete LPPO program consisting of zero or more modules and procedures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub ring0: bool,
    pub use_declarations: Vec<UseDeclaration>,
    pub type_aliases: Vec<TypeAlias>,
    pub modules: Vec<Module>,
    pub procedures: Vec<Procedure>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub traits: Vec<TraitDef>,
    pub impls: Vec<ImplBlock>,
}

/// Use declaration for importing items
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDeclaration {
    pub path: Vec<String>,
    pub items: UseItems,
}

/// Items that can be imported
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseItems {
    /// Import all items from a path: use std::collections::*;
    All,
    /// Import specific items: use std::collections::{Vec, HashMap};
    Items(Vec<String>),
    /// Import a single item: use std::collections::Vec;
    Single(String),
}

/// Type alias definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    pub name: String,
    pub typ: TypeAnnotation,
    pub is_public: bool,
    pub generic_params: Vec<String>,
}

/// Represents a module for namespace organization
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub procedures: Vec<Procedure>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
}

/// Struct definition with explicit fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub methods: Vec<Procedure>,
    pub is_public: bool,
    pub generic_params: Vec<String>,
}

/// Struct field with type and visibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub typ: TypeAnnotation,
    pub is_public: bool,
}

/// Enum definition with variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub is_public: bool,
    pub generic_params: Vec<String>,
}

/// Enum variant with optional associated data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeAnnotation>, // Tuple-style fields
}

/// Trait definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethod>,
    pub is_public: bool,
}

/// Trait method signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: TypeAnnotation,
}

/// Implementation block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplBlock {
    pub trait_name: Option<String>, // None for inherent impl
    pub type_name: String,
    pub methods: Vec<Procedure>,
}

/// Type annotations required for all variables and parameters
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypeAnnotation {
    Int64,
    Bool,
    Float,
    Float32,
    Float64,
    Char,
    Byte,
    Str,
    Arr(Box<TypeAnnotation>), // Array of specific type
    Void,
    Ref(Box<TypeAnnotation>, Option<String>), // Reference with optional lifetime
    Own(Box<TypeAnnotation>),                 // Owned value
    Mut(Box<TypeAnnotation>),                 // Mutable reference
    Custom(String),                           // User-defined types (structs, enums)
    Generic(String, Vec<TypeAnnotation>),     // Generic types like HashMap<Str, Int64>
    Tuple(Vec<TypeAnnotation>),               // Tuple types
    Option(Box<TypeAnnotation>),              // Option type
    Result(Box<TypeAnnotation>, Box<TypeAnnotation>), // Result type
    Map(Box<TypeAnnotation>, Box<TypeAnnotation>),    // Map<K, V> hash map
    Set(Box<TypeAnnotation>),                 // Set<T>
    Fn {
        params: Vec<TypeAnnotation>,
        ret: Box<TypeAnnotation>,
    }, // Function type: Fn(Int64, Str) -> Bool
}

impl TypeAnnotation {
    /// Resolve keyword aliases (`F64`, `Vec<T>`, `HashSet<T>`, ...) into canonical types.
    pub fn normalized(&self) -> TypeAnnotation {
        use TypeAnnotation::*;
        match self {
            Custom(name) => match name.as_str() {
                "I64" | "Int64" | "Int" => Int64,
                "BOOL8" | "Bool" => Bool,
                "STRPTR" | "Str" | "String" => Str,
                "F64" | "Float64" | "Float" | "Double" => Float64,
                "F32" | "Float32" => Float32,
                "Char" => Char,
                "Byte" | "U8" | "UInt8" => Byte,
                "Vec" | "Array" | "Arr" | "Slice" => Arr(Box::new(Int64)),
                "HashMap" | "Map" | "BTreeMap" => Map(Box::new(Str), Box::new(Int64)),
                "HashSet" | "Set" | "BTreeSet" => Set(Box::new(Int64)),
                "Tuple" => Tuple(Vec::new()),
                other => Custom(other.to_string()),
            },
            Generic(name, params) => match (name.as_str(), params.len()) {
                ("Vec" | "Array" | "Arr" | "Slice", 1) => Arr(Box::new(params[0].normalized())),
                ("HashSet" | "Set" | "BTreeSet", 1) => Set(Box::new(params[0].normalized())),
                ("HashMap" | "Map" | "BTreeMap", 2) => Map(
                    Box::new(params[0].normalized()),
                    Box::new(params[1].normalized()),
                ),
                ("Option", 1) => Option(Box::new(params[0].normalized())),
                ("Result", 2) => Result(
                    Box::new(params[0].normalized()),
                    Box::new(params[1].normalized()),
                ),
                _ => Generic(
                    name.clone(),
                    params.iter().map(|p| p.normalized()).collect(),
                ),
            },
            Arr(t) => Arr(Box::new(t.normalized())),
            Set(t) => Set(Box::new(t.normalized())),
            Map(k, v) => Map(Box::new(k.normalized()), Box::new(v.normalized())),
            Option(t) => Option(Box::new(t.normalized())),
            Result(ok, err) => Result(Box::new(ok.normalized()), Box::new(err.normalized())),
            Tuple(items) => Tuple(items.iter().map(|t| t.normalized()).collect()),
            Ref(t, lifetime) => Ref(Box::new(t.normalized()), lifetime.clone()),
            Mut(t) => Mut(Box::new(t.normalized())),
            Own(t) => Own(Box::new(t.normalized())),
            Fn { params, ret } => Fn {
                params: params.iter().map(|t| t.normalized()).collect(),
                ret: Box::new(ret.normalized()),
            },
            other => other.clone(),
        }
    }

    /// Human-readable spelling used in diagnostics and printed output.
    pub fn display_name(&self) -> String {
        match self.normalized() {
            TypeAnnotation::Int64 => "Int64".into(),
            TypeAnnotation::Bool => "Bool".into(),
            TypeAnnotation::Float | TypeAnnotation::Float64 => "Float64".into(),
            TypeAnnotation::Float32 => "Float32".into(),
            TypeAnnotation::Char => "Char".into(),
            TypeAnnotation::Byte => "Byte".into(),
            TypeAnnotation::Str => "Str".into(),
            TypeAnnotation::Void => "Void".into(),
            TypeAnnotation::Arr(t) => format!("Arr<{}>", t.display_name()),
            TypeAnnotation::Set(t) => format!("Set<{}>", t.display_name()),
            TypeAnnotation::Map(k, v) => {
                format!("Map<{}, {}>", k.display_name(), v.display_name())
            }
            TypeAnnotation::Option(t) => format!("Option<{}>", t.display_name()),
            TypeAnnotation::Result(ok, err) => {
                format!("Result<{}, {}>", ok.display_name(), err.display_name())
            }
            TypeAnnotation::Tuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(|t| t.display_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TypeAnnotation::Fn { params, ret } => format!(
                "Fn({}) -> {}",
                params
                    .iter()
                    .map(|t| t.display_name())
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.display_name()
            ),
            TypeAnnotation::Ref(t, _) => format!("Ref<{}>", t.display_name()),
            TypeAnnotation::Mut(t) => format!("Mut<{}>", t.display_name()),
            TypeAnnotation::Own(t) => format!("Own<{}>", t.display_name()),
            TypeAnnotation::Custom(n) => n,
            TypeAnnotation::Generic(n, params) => format!(
                "{}<{}>",
                n,
                params
                    .iter()
                    .map(|t| t.display_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// Parameter with explicit type and ownership annotations
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Read input from stdin: input var: Type with prompt;
    Input {
        var: String,
        typ: TypeAnnotation,
        prompt: Option<Expr>,
    },
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
    While { cond: Expr, body: Vec<Statement> },
    /// For-in loop: for item in expr { ... }
    For {
        var: String,
        typ: TypeAnnotation,
        iterable: Expr,
        body: Vec<Statement>,
    },
    /// Infinite loop: loop { ... }
    Loop { body: Vec<Statement> },
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
    Move { source: String, target: String },
    /// Copy value: copy var to target (requires Copy trait)
    Copy { source: String, target: String },
    /// Explicit scope block
    Scope {
        name: Option<String>,
        body: Vec<Statement>,
    },
    /// Unsafe block
    Unsafe { body: Vec<Statement> },
    /// Match expression for pattern matching
    Match { expr: Expr, arms: Vec<MatchArm> },
    /// Struct instantiation: let x: Point = Point { x: 10, y: 20 };
    StructInit {
        name: String,
        typ: TypeAnnotation,
        fields: Vec<(String, Expr)>,
    },
    /// Expression statement: expr;
    ExprStatement(Expr),
    /// Assignment: var = expr;
    Assignment { target: String, value: Expr },
    /// Indexed assignment: arr[i] = expr;
    IndexedAssignment {
        target: Box<Expr>,
        index: Box<Expr>,
        value: Expr,
    },
    /// Pause execution until the user presses Enter or other condition
    Pause {
        /// Optional prompt expression displayed before waiting
        prompt: Option<Expr>,
        /// Optional timeout in milliseconds (Int expression)
        timeout_ms: Option<Expr>,
        /// Optional until-expression: wait until input equals this expression's string
        until: Option<Expr>,
        /// Optional variable name to assign the received input to
        assign_to: Option<String>,
        /// Optional expression to evaluate on mismatch (when input != until)
        on_mismatch: Option<Expr>,
        /// Optional expression to evaluate on timeout
        on_timeout: Option<Expr>,
        /// Async flag: if true, schedule remaining ops in background and return immediately
        is_async: bool,
    },
    /// Spawn a block of statements in a new thread (fire-and-forget threading).
    Spawn { body: Vec<Statement> },
}

/// Qualified name for namespaced identifiers like module::submodule::name
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualifiedName {
    pub path: Vec<String>,
    pub name: String,
}

/// Memory location specification
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemoryLocation {
    Heap,
    Stack,
}

/// Ownership kind for variables
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipKind {
    Own,    // Owned value
    Ref,    // Immutable reference
    MutRef, // Mutable reference
    Borrow, // Borrowed value
}

/// Match arm for pattern matching
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: MatchArmBody,
}

/// Body of a `match` arm: either an expression value or an `out ...` emit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MatchArmBody {
    Expr(Expr),
    Emit(Vec<Expr>),
}

/// Pattern for matching
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Pattern {
    /// Wildcard pattern _
    Wildcard,
    /// Literal pattern (number, string, bool)
    Literal(Expr),
    /// Variable binding
    Variable(String),
    /// Tuple pattern
    Tuple(Vec<Pattern>),
    /// Struct pattern
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    /// Enum variant pattern
    EnumVariant {
        enum_name: String,
        variant: String,
        fields: Vec<Pattern>,
    },
    /// Or pattern (multiple alternatives)
    Or(Vec<Pattern>),
}

/// Expression nodes for simple arithmetic and string literals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    Int(i64),
    Float(String), // Store as string to maintain Eq
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
    /// Closure/Lambda expression: |params| -> return_type { body }
    Closure {
        params: Vec<Parameter>,
        return_type: Option<TypeAnnotation>,
        body: Box<Expr>,
    },
    /// Function call: func(args)
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    /// Struct instantiation expression: Player { hp: 100, name: "Hero" }
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    /// Range expression: start..end
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },
    /// Match expression: match expr { arms }
    MatchExpr {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// Await expression: await expr
    Await(Box<Expr>),
    /// Async block expression: async { expr } - evaluates in a background task, returns a Future.
    Async { body: Box<Expr> },
    /// Spawn expression: spawn { expr } - evaluates in a new thread, returns a JoinHandle.
    Spawn { body: Box<Expr> },
    /// Join a spawned thread / future handle: await also works on handles.
    Join(Box<Expr>),
    /// Handle expression for effect handling: handle expr with type { handlers }
    Handle {
        expr: Box<Expr>,
        handler_type: TypeAnnotation,
        handlers: Vec<(String, Box<Expr>)>, // (handler_name, handler_body)
    },
    /// Character literal: 'a'
    Char(char),
    /// Byte literal: b'A'
    Byte(u8),
    /// Tuple literal: (a, b)
    Tuple(Vec<Expr>),
    /// Enum / Option / Result variant construction: Color::Red, Some(1), Ok(v)
    EnumVariant {
        enum_name: String,
        variant: String,
        args: Vec<Expr>,
    },
    /// Early return used in `match` arm bodies: `=> return expr`
    Return(Option<Box<Expr>>),
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
