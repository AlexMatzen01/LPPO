# LPPO Architecture Documentation

## Overview

LPPO v2.0 is implemented as a multi-stage compiler and interpreter written in Rust. This document describes the internal architecture, design decisions, and implementation details.

## Architecture Diagram

```
┌─────────────┐
│ Source Code │
│  (.lppo)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Lexer     │  Tokenization
│ (lexer.rs)  │
└──────┬──────┘
       │
       ▼  [Token Stream]
┌─────────────┐
│   Parser    │  Syntax Analysis
│ (parser.rs) │
└──────┬──────┘
       │
       ▼  [AST]
┌─────────────┐
│  Compiler   │  Lowering
│(compiler.rs)│
└──────┬──────┘
       │
       ▼  [Operations]
┌─────────────┐
│  Runtime    │  Execution
│ (runtime.rs)│
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Output    │
└─────────────┘
```

## Core Components

### 1. Lexer (src/lexer.rs)

**Purpose**: Transform raw source code into a stream of tokens.

**Key Responsibilities**:
- Character-by-character scanning
- Token recognition
- String literal parsing with escape sequences
- Comment handling (line and block)
- Keyword identification
- Number literal parsing
- Error reporting for invalid characters

**Design Decisions**:
- Single-pass scanning for efficiency
- Byte-based processing for performance
- Position tracking for error reporting
- Eager tokenization (no lookahead buffer)

**Token Categories**:
- Keywords: `proc`, `let`, `if`, `while`, `for`, etc.
- Types: `Int64`, `Bool`, `Str`, `Arr`, `Void`
- Operators: `+`, `-`, `*`, `/`, `==`, `!=`, etc.
- Delimiters: `{`, `}`, `(`, `)`, `[`, `]`
- Literals: Numbers, strings, booleans
- Identifiers: Variable and procedure names

**Example**:
```rust
let lexer = Lexer::new("let own x: Int64 = 42;");
// Produces: [Let, Own, Identifier("x"), Colon, TypeInt64, 
//            Equals, Number(42), Semicolon]
```

### 2. Parser (src/parser.rs)

**Purpose**: Build an Abstract Syntax Tree (AST) from token stream.

**Key Responsibilities**:
- Recursive descent parsing
- Syntax validation
- AST construction
- Precedence handling for expressions
- Error recovery (planned)

**Design Decisions**:
- Top-down recursive descent (simple, maintainable)
- LL(1) grammar with explicit lookahead
- Explicit operator precedence tables
- No left recursion

**Parsing Strategy**:
```
Program := (Module | Procedure)*
Module := 'module' Identifier '{' Procedure* '}'
Procedure := 'pub'? 'proc' Identifier '(' Parameters? ')' '->' Type '{' Statement* '}'
Statement := Let | If | While | For | Loop | Call | Out | etc.
Expression := LogicalOr
LogicalOr := LogicalAnd ('or' LogicalAnd)*
LogicalAnd := Comparison ('and' Comparison)*
Comparison := BitwiseOr (CompOp BitwiseOr)?
...
```

**Error Handling**:
- Descriptive error messages with position
- Contextual suggestions
- Future: Error recovery to continue parsing

### 3. Abstract Syntax Tree (src/ast.rs)

**Purpose**: Internal representation of program structure.

**Key Structures**:

**Program**: Top-level container
```rust
pub struct Program {
    pub modules: Vec<Module>,
    pub procedures: Vec<Procedure>,
    pub structs: Vec<StructDef>,     // Planned
    pub enums: Vec<EnumDef>,         // Planned
    pub traits: Vec<TraitDef>,       // Planned
    pub impls: Vec<ImplBlock>,       // Planned
}
```

**Type System**:
```rust
pub enum TypeAnnotation {
    Int64,
    Bool,
    Str,
    Arr(Box<TypeAnnotation>),
    Void,
    Ref(Box<TypeAnnotation>, Option<String>),  // References
    Own(Box<TypeAnnotation>),                  // Owned
    Mut(Box<TypeAnnotation>),                  // Mutable
    Custom(String),                            // User-defined
    Generic(String, Vec<TypeAnnotation>),      // Generics
    Tuple(Vec<TypeAnnotation>),                // Tuples
    Option(Box<TypeAnnotation>),               // Option<T>
    Result(Box<TypeAnnotation>, Box<TypeAnnotation>), // Result<T,E>
}
```

**Statements**:
- Let: Variable declarations
- If/While/For/Loop: Control flow
- Call: Procedure invocation
- Emit: Output
- Parallel: Concurrent execution
- Scope/Unsafe: Explicit blocks
- Match: Pattern matching (planned)
- StructInit: Struct creation (planned)

**Expressions**:
- Literals: Numbers, booleans, strings, arrays
- Binary operations: Arithmetic, comparison, logical, bitwise
- Unary operations: Negation, logical not, bitwise not
- Method calls: Array/string methods
- Field access (planned)
- References and dereferencing

### 4. Compiler (src/compiler.rs)

**Purpose**: Lower AST to executable operations.

**Key Responsibilities**:
- AST traversal
- Operation generation
- Namespace resolution
- Duplicate detection

**Design Decisions**:
- Direct AST-to-operations mapping (no IR)
- Procedure-level compilation
- Minimal optimization (explicit is better)

**Compilation Process**:
```rust
fn compile_program(program: Program) -> Result<HashMap<String, ProcedureBody>>
  → For each module and procedure:
    → Check for duplicates
    → Compile procedure body
    → Store in procedure map with qualified name
```

**Operation Types**:
```rust
pub enum Operation {
    Emit(Vec<Expr>),              // Output
    Parallel(Vec<Expr>),          // Concurrent execution
    Call(String, Vec<Expr>),      // Procedure call
    Let(String, Expr),            // Variable binding
    If { ... },                   // Conditional
    While { ... },                // Loop
    For { ... },                  // Iteration
    Loop { ... },                 // Infinite loop
    Break,                        // Exit loop
    Continue,                     // Next iteration
    Return(Option<Expr>),         // Return from procedure
    Scope { ... },                // Explicit scope
    Unsafe { ... },               // Unsafe operations
    Nop,                          // No-op placeholder
}
```

### 5. Runtime (src/runtime.rs)

**Purpose**: Execute compiled operations.

**Key Responsibilities**:
- Operation interpretation
- Environment management (variable scopes)
- Expression evaluation
- Control flow handling
- Parallel execution
- Error detection and reporting

**Design Decisions**:
- Tree-walking interpreter (simple, debuggable)
- HashMap-based environments
- Stack-based control flow
- Thread-based parallelism

**Executor Structure**:
```rust
pub struct Executor {
    procedures: HashMap<String, ProcedureBody>,
}

impl Executor {
    pub fn run(&self, entry: &str) -> Result<()>
    fn execute_procedure_with_env(...)
    fn execute_ops(...) -> Result<ControlFlow>
    fn eval_expr(...) -> Result<Value>
}
```

**Value Types**:
```rust
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Arr(Vec<Value>),
    Void,
}
```

**Control Flow**:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlFlow {
    None,      // Continue normally
    Break,     // Exit loop
    Continue,  // Next iteration
    Return,    // Exit procedure
}
```

**Execution Model**:
1. Lookup entry procedure
2. Create empty environment
3. Execute operations sequentially
4. Evaluate expressions as needed
5. Handle control flow
6. Manage variable scopes

**Parallel Execution**:
- Uses Rust's thread spawning
- Each expression evaluated in separate thread
- Results collected and output in arrival order
- Thread-safe due to immutable values

### 6. Error System (src/error.rs)

**Purpose**: Comprehensive error reporting.

**Error Categories**:

**Lexical Errors**:
- Unexpected character
- Unterminated string

**Parse Errors**:
- Unexpected token
- Unexpected end of input

**Semantic Errors**:
- Type mismatch
- Unknown variable
- Unknown procedure
- Duplicate procedure

**Runtime Errors**:
- Division by zero
- Index out of bounds
- Stack overflow
- Memory errors

**Error Structure**:
```rust
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub position: usize,
    pub length: usize,
}

pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

pub enum LppoError {
    // Each variant includes:
    // - Location information
    // - Contextual data
    // - Optional suggestion
}
```

**Error Formatting**:
```
error: unknown variable 'foo'
  --> line 8, column 5
   |
8  | out foo;
   |     ^^^
   = help: check that the variable is declared before use
```

## Data Flow

### Compilation Flow

```
Source → Lexer → Tokens → Parser → AST → Compiler → Operations → Bytecode
```

### Execution Flow

```
Entry Point → Executor → Procedure → Operations → Expressions → Values → Output
```

### Environment Management

```
Global Procedures Map
  └─> Procedure Name → Operations
        └─> Local Environment (HashMap<String, Value>)
              └─> Variable Name → Value
```

## Memory Management

### Ownership Tracking

LPPO tracks ownership through explicit qualifiers:
- `own`: Owned value
- `mut`: Mutable reference
- `ref`: Immutable reference
- `borrow`: Borrowed value

Currently tracked at:
- **Parse time**: In AST nodes
- **Future**: Runtime enforcement

### Lifetime Annotations

Syntax exists for lifetime parameters:
```lppo
pub proc process<'a>(x: Ref<Int64> 'a) -> Void { }
```

Currently:
- **Parsed**: Stored in AST
- **Future**: Validation and enforcement

### Memory Operations

**Allocation**:
```lppo
alloc<Type> var on heap/stack [size];
```

**Deallocation**:
```lppo
dealloc var;
```

**Move**:
```lppo
move source -> target;
```

**Copy**:
```lppo
copy source -> target;
```

Currently: Parsed but not fully enforced at runtime.

## Parallel Execution Model

### Implementation

Uses Rust's `std::thread`:
```rust
let handles: Vec<_> = expressions
    .iter()
    .map(|expr| {
        let expr_clone = expr.clone();
        thread::spawn(move || evaluate(expr_clone))
    })
    .collect();

for handle in handles {
    let result = handle.join()?;
    println!("{}", result);
}
```

### Characteristics

- **True Parallelism**: Uses OS threads
- **Unordered Output**: Results appear as they complete
- **Thread-Safe**: Values are immutable
- **No Shared State**: Each thread has own environment

## Extensibility Points

### Adding New Types

1. Add variant to `TypeAnnotation`
2. Update parser to recognize type
3. Add to `Value` enum
4. Implement operations in runtime

### Adding New Statements

1. Add variant to `Statement` in AST
2. Add parsing logic in parser
3. Add compilation in compiler
4. Add execution in runtime

### Adding New Operations

1. Add token if needed
2. Parse expression
3. Compile to operation
4. Evaluate in runtime

## Performance Considerations

### Current Status

- **Interpreted**: Tree-walking interpreter (slow)
- **No Optimization**: Direct execution
- **Memory**: Cloning values frequently

### Future Improvements

- JIT compilation
- Bytecode generation
- Register-based VM
- Value sharing with reference counting
- Constant folding
- Dead code elimination

## Testing Strategy

### Current

- Manual testing with example programs
- Build validation

### Planned

- Unit tests for each module
- Integration tests for features
- Property-based testing
- Fuzzing
- Benchmark suite
- Performance regression tests

## Build and Deployment

### Compilation

```bash
cargo build --release
```

### Bundled Executables

LPPO can compile programs to standalone executables:

1. Serialize compiled program to JSON
2. Append to copy of compiler executable
3. Mark with magic bytes and length
4. Set executable permissions

**Format**:
```
[Compiler Binary]
[Magic: "LPPO-EMBED-1\n"]
[Length: 8 bytes little-endian]
[Payload: JSON-serialized procedures]
```

**Execution**:
- Check for embedded program
- Deserialize and execute
- Fall back to file input if no payload

## Dependencies

**Direct Dependencies**:
- `serde`: Serialization for bundled executables
- `serde_json`: JSON encoding

**Why Minimal?**

- Keep complexity low
- Easy to audit
- Fast compilation
- Minimal attack surface

## Design Philosophy

### Explicit Over Implicit

Every aspect requires manual specification:
- Types
- Ownership
- Memory
- Control flow

### Complexity as Feature

Deliberately verbose to teach:
- Type systems
- Memory management
- Ownership tracking
- Explicit programming

### Safety Through Explicitness

Making everything explicit:
- Reduces bugs
- Makes intent clear
- Forces thinking
- Educational value

## Future Directions

### Type System Enhancements

- Full struct implementation
- Enum pattern matching
- Trait system with implementations
- Generic functions and types
- Type aliases
- Associated types

### Standard Library

- File I/O
- Network operations
- Regular expressions
- JSON parsing
- Collections (HashMap, HashSet, etc.)
- Math library

### Tooling

- Formatter
- Linter
- Language server
- Debugger
- REPL

### Performance

- JIT compilation
- Bytecode VM
- Optimization passes
- Profile-guided optimization

### Language Features

- Closures
- Async/await
- Macros
- Inline assembly
- FFI (Foreign Function Interface)

## Contributing

Areas for contribution:
1. Additional error recovery in parser
2. More comprehensive runtime checks
3. Performance optimizations
4. Standard library functions
5. Documentation improvements
6. Example programs
7. Testing infrastructure

## Conclusion

LPPO's architecture prioritizes:
- **Clarity**: Easy to understand
- **Explicitness**: No hidden behavior
- **Extensibility**: Easy to add features
- **Educational Value**: Teaching tool for language implementation

The multi-stage design allows for clear separation of concerns and makes the language easier to maintain and extend.
