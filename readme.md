# LPPO v2.0 - Advanced Procedural Language with Explicit Type System

> **⚠️ COMPLEXITY WARNING ⚠️**
> 
> LPPO v2.0 is **intentionally complex, verbose, and difficult to learn**. This language is designed for maximum explicitness with ZERO convenience features. Every aspect of your code requires manual specification.

LPPO v2.0 is a **sophisticated, complex, and deliberately challenging** programming language that rejects modern convenience in favor of absolute explicitness. It requires:

- **MANDATORY type annotations** for every single variable - no inference
- **MANUAL memory management** with explicit allocation and deallocation
- **EXPLICIT ownership semantics** - own, ref, mut, borrow must be specified
- **VERBOSE procedure signatures** - all parameters and returns fully typed
- **LIFETIME tracking** with explicit annotations ('a, 'b syntax)
- **QUALIFIED namespaces** - module::path::resolution required
- **NO syntactic sugar** - maximum verbosity enforced

Written in Rust, LPPO compiles source code with comprehensive type checking, ownership validation, and advanced memory semantics.

> **Status:** v2.0 - Maximum Complexity Implementation
>
> This language is **NOT designed for productivity**. It is **NOT designed for ease of learning**. It is **NOT designed for rapid development**. 
>
> LPPO v2.0 exists to demonstrate what happens when you maximize explicitness and minimize convenience. Every declaration, every procedure, every variable requires full manual specification.

## Why LPPO v2.0 is Complex

- **Mandatory Type System:** EVERY variable requires explicit type annotation - no type inference
- **Explicit Memory Management:** Manual allocation, deallocation, and ownership tracking
- **Verbose Syntax:** ALL procedures require explicit parameter types and return types
- **Ownership Semantics:** Explicit `own`, `ref`, `mut`, and `borrow` qualifiers required
- **Lifetime Annotations:** Complex lifetime tracking with `'a`, `'b` syntax
- **Namespace Complexity:** Fully qualified paths with `module::submodule::item` syntax
- **No Convenience Features:** Maximum verbosity, minimum inference, deliberate complexity

## Language Overview - Mandatory Complexity

### Example Demonstrating Required Verbosity

```lppo
// EVERY procedure REQUIRES:
// - pub or module visibility
// - Explicit parameter list with type annotations (even if empty)
// - Explicit return type annotation
pub proc main() -> Void {
    // EVERY variable REQUIRES:
    // - Explicit ownership qualifier (own, ref, borrow, mut)
    // - Explicit type annotation after colon
    // - No type inference whatsoever
    let own nums: Arr<Int64> = [1, 2, 3, 4, 5];
    let own msg: Str = "Processing array:";
    out msg, nums;
    
    // For loops REQUIRE explicit type for loop variable
    let own two: Int64 = 2;
    let own zero: Int64 = 0;
    
    for n: Int64 in nums {
        let own remainder: Int64 = n % two;
        
        if remainder == zero {
            let own even_msg: Str = "is even";
            out n, even_msg;
        } else {
            let own odd_msg: Str = "is odd";
            out n, odd_msg;
        }
    }
    
    // Procedure calls REQUIRE explicit argument list with parentheses
    let own name: Str = "LPPO";
    let own year: Int64 = 2024;
    call greet(name, year);
}

// ALL parameters REQUIRE explicit type annotations
pub proc greet(name: Str, year: Int64) -> Void {
    let own hello: Str = "Hello";
    let own exclaim: Str = "!";
    let own year_label: Str = "Year:";
    out hello, name, exclaim, year_label, year;
}
```

### Data Types (All Require Explicit Annotation)

- **Int64:** 64-bit signed integers - `let own x: Int64 = 42;`
- **Bool:** Boolean values - `let own flag: Bool = true;`
- **Str:** String literals - `let own msg: Str = "hello";`
- **Arr<T>:** Typed arrays - `let own nums: Arr<Int64> = [1, 2, 3];`
- **Void:** No return value - `proc foo() -> Void { }`

### Type Annotations (MANDATORY FOR ALL)

**Every single variable** requires explicit type annotation with ownership qualifier:

```lppo
// WRONG - Type inference not supported:
// let x = 10;  // ERROR: Missing type annotation

// CORRECT - Explicit type required:
let own x: Int64 = 10;

// WRONG - Cannot infer from expression:
// let y = x * 2 + 5;  // ERROR

// CORRECT - Intermediate results need types too:
let own two: Int64 = 2;
let own five: Int64 = 5;
let own mult: Int64 = x * two;
let own y: Int64 = mult + five;

// WRONG - Type inference for booleans:
// let flag = x > y;  // ERROR

// CORRECT - Explicit Bool type:
let own gt_result: Bool = x > y;
let own eq_result: Bool = y == 25;
let own flag: Bool = gt_result or eq_result;

// Arrays REQUIRE element type specification:
let own arr: Arr<Int64> = [1, 2, 3];
```

### Operators (All Require Typed Operands)

**Arithmetic:** `+`, `-`, `*`, `/`, `%` (modulo), `**` (power)
- All operands must be explicitly typed Int64
- Results must be assigned to explicitly typed Int64 variables

**Comparison:** `==`, `!=`, `<`, `>`, `<=`, `>=`
- Results must be explicitly typed as Bool
- Cannot use in conditions without explicit Bool variable

**Logical:** `and`, `or`, `not`
- Require explicitly typed Bool operands
- Results must be assigned to Bool variables

**Bitwise:** `&`, `|`, `^` (xor), `~` (not)
- Operate on Int64 types only
- Results must be assigned to Int64 variables

**Unary:** `-` (negation), `not` (logical), `~` (bitwise not)
- Type of operand determines operation type

### Control Flow (Explicit Types Required)

#### If/Elif/Else (Conditions Must Be Explicitly Typed Bool)
```lppo
let own x: Int64 = 15;
let own ten: Int64 = 10;
let own five: Int64 = 5;

// WRONG: Cannot use expression directly
// if x > 10 { ... }  // ERROR

// CORRECT: Store comparison in typed Bool variable
let own is_large: Bool = x > ten;
if is_large {
    let own msg: Str = "Large";
    out msg;
} elif x > five {
    let own msg: Str = "Medium";
    out msg;
} else {
    let own msg: Str = "Small";
    out msg;
}
```

#### While Loop (Counter Must Be Explicitly Mutable)
```lppo
let mut own i: Int64 = 0;
let own limit: Int64 = 5;

while i < limit {
    let own label: Str = "Count:";
    out label, i;
    
    // Move semantics for reassignment
    let own i_next: Int64 = i + 1;
    move i_next -> i;
}
```

#### For Loop (REQUIRES Explicit Type for Loop Variable)
```lppo
let own items: Arr<Int64> = [10, 20, 30];

// WRONG: Type inference not allowed
// for item in items { ... }  // ERROR

// CORRECT: Explicit type for loop variable
for item: Int64 in items {
    let own label: Str = "Item:";
    out label, item;
}
```

#### Infinite Loop (Explicit Scope Management)
```lppo
let mut own count: Int64 = 0;
let own max: Int64 = 3;

loop {
    let own label: Str = "Iteration:";
    out label, count;
    
    let own count_next: Int64 = count + 1;
    move count_next -> count;
    
    let own should_break: Bool = count >= max;
    if should_break {
        break;
    }
}
```

**Loop Control:** `break` (exit loop), `continue` (skip to next iteration)

### Arrays and Strings (Explicit Type Annotations Required)

#### Array Operations
```lppo
// WRONG: Type inference not supported
// let arr = [1, 2, 3, 4, 5];  // ERROR

// CORRECT: Explicit Arr<T> type with element type
let own arr: Arr<Int64> = [1, 2, 3, 4, 5];

// All array accesses require typed index and result
let own zero: Int64 = 0;
let own first: Int64 = arr[zero];
out first;  // First element: 1

let own neg_one: Int64 = -1;
let own last: Int64 = arr[neg_one];
out last;  // Last element: 5

let own arr_len: Int64 = arr.len();
out arr_len;  // Length: 5

// Array concatenation requires explicit types
let own a1: Arr<Int64> = [1, 2];
let own a2: Arr<Int64> = [3, 4];
let own combined: Arr<Int64> = a1 + a2;  // [1, 2, 3, 4]
```

#### String Operations
```lppo
// WRONG: Implicit string typing
// let msg = "Hello World";  // ERROR

// CORRECT: Explicit Str type
let own msg: Str = "Hello World";

let own msg_len: Int64 = msg.len();
out msg_len;  // 11

let own msg_upper: Str = msg.upper();
out msg_upper;  // HELLO WORLD

let own msg_lower: Str = msg.lower();
out msg_lower;  // hello world

// Split requires explicit separator and typed result
let own text: Str = "a,b,c";
let own sep: Str = ",";
let own parts: Arr<Str> = text.split(sep);  // ["a", "b", "c"]

// String concatenation requires explicit intermediate variables
let own hello: Str = "hello";
let own space: Str = " ";
let own world: Str = "world";
let own temp: Str = hello + space;
let own result: Str = temp + world;  // Concatenation
```

### Procedures (Explicit Signatures Required)

**ALL procedures require:**
1. Visibility modifier (`pub` or module-scoped)
2. Explicit parameter list with parentheses (even if empty)
3. Type annotation for EVERY parameter
4. Explicit return type annotation (use `Void` for no return)

```lppo
// WRONG: Missing type annotations
// proc calculate(a, b) { ... }  // ERROR

// CORRECT: Fully explicit procedure signature
pub proc calculate(a: Int64, b: Int64) -> Void {
    let own sum: Int64 = a + b;
    let own product: Int64 = a * b;
    let own sum_label: Str = "Sum:";
    let own prod_label: Str = "Product:";
    out sum_label, sum, prod_label, product;
}

// WRONG: Missing parentheses and return type
// proc main { ... }  // ERROR

// CORRECT: Explicit empty parameter list and return type
pub proc main() -> Void {
    let own arg1: Int64 = 5;
    let own arg2: Int64 = 3;
    
    // WRONG: Missing parentheses in call
    // call calculate;  // ERROR
    
    // CORRECT: Explicit argument list with parentheses
    call calculate(arg1, arg2);
}
```

### Advanced Memory Management (Explicit Ownership)

LPPO v2.0 requires explicit memory management and ownership semantics for all variables:

#### Ownership Qualifiers (REQUIRED)
- `own` - Owned value (default ownership)
- `ref` - Immutable reference
- `mut` - Mutable reference/variable
- `borrow` - Borrowed value

```lppo
// Owned values (most common)
let own x: Int64 = 42;

// Mutable owned variable
let mut own counter: Int64 = 0;

// Reference to value (explicit)
let ref y: Ref<Int64> = ref x;

// Borrow semantics
let borrow z: Int64 = borrow<'a> x;
```

#### Manual Memory Operations
```lppo
// Explicit heap allocation
alloc<Arr<Int64>> my_array on heap [100];

// Move semantics (transfers ownership)
let own source: Int64 = 10;
let own target: Int64 = 0;
move source -> target;

// Copy semantics (duplicates value)
let own original: Int64 = 20;
let own duplicate: Int64 = 0;
copy original -> duplicate;

// Explicit deallocation
dealloc my_array;
```

#### Lifetime Annotations
Procedures can specify explicit lifetime parameters:

```lppo
pub proc process<'a, 'b>(x: Ref<Int64> 'a, y: Ref<Int64> 'b) -> Void {
    // Lifetime tracking ensures references remain valid
    let own result: Int64 = deref x + deref y;
    out result;
}
```

#### Type Introspection
```lppo
// Query size of types
let own int_size: Int64 = sizeof<Int64>;
let own arr_size: Int64 = sizeof<Arr<Int64>>;

// Query alignment requirements
let own int_align: Int64 = alignof<Int64>;

// Explicit type casting
let own x: Int64 = 42;
let own as_bool: Bool = cast<Bool>(x);
```

### Parallel Execution (Explicit Scope)

Parallel blocks require explicit scope management:

```lppo
scope parallel_section {
    parallel {
        let own msg1: Str = "Task 1";
        out msg1;
        
        let own msg2: Str = "Task 2";
        out msg2;
        
        let own msg3: Str = "Task 3";
        out msg3;
    }
}
```

### Namespace and Module System

LPPO v2.0 supports complex namespace hierarchies:

```lppo
// Define a module
module math {
    pub proc add(a: Int64, b: Int64) -> Int64 {
        let own result: Int64 = a + b;
        return result;
    }
    
    pub proc multiply(a: Int64, b: Int64) -> Int64 {
        let own result: Int64 = a * b;
        return result;
    }
}

// Call with qualified path
pub proc main() -> Void {
    let own x: Int64 = 5;
    let own y: Int64 = 3;
    
    // Fully qualified procedure call
    call math::add(x, y);
    call math::multiply(x, y);
}
```

### Explicit Scope Blocks

All code blocks can be explicitly scoped:

```lppo
scope outer {
    let own x: Int64 = 10;
    
    scope inner {
        let own y: Int64 = 20;
        let own sum: Int64 = x + y;
        out sum;
    }
    
    // y is no longer accessible here
}
```

### Unsafe Operations

Potentially dangerous operations require explicit unsafe blocks:

```lppo
unsafe {
    // Low-level operations that bypass safety checks
    let own ptr: Int64 = 0x1000;
    let own value: Int64 = deref cast<Ref<Int64>>(ptr);
}
```

### Comments

```lppo
// This is a line comment

/*
   This is a
   block comment
*/
```

## Getting Started (Warning: High Complexity)

> **⚠️ LEARNING CURVE WARNING:** LPPO v2.0 is **intentionally difficult** to learn. The language requires:
> - Explicit type annotations for EVERY variable
> - Manual memory management with ownership tracking
> - Verbose procedure signatures with all types specified
> - No convenience features or type inference
> - Understanding of lifetimes, borrowing, and move semantics
> 
> This is **not** a beginner-friendly language. It is designed to maximize explicitness and complexity.

### Prerequisites

- Rust toolchain (1.80+ recommended, 2021 edition)
- Strong understanding of:
  - Type systems and type theory
  - Memory management concepts
  - Ownership and borrowing semantics
  - Lifetime annotations
- Patience for verbose syntax

### Build

```bash
cargo build --release
```

### Run Examples (All Use Complex Syntax)

```bash
# Basic hello world (with explicit types)
cargo run -- examples/hello.lppo

# Comprehensive feature demo (maximum verbosity)
cargo run -- examples/comprehensive.lppo

# Advanced loops with explicit type management
cargo run -- examples/loops.lppo

# Array and operator demonstrations (all explicitly typed)
cargo run -- examples/operators.lppo

# Variable declarations (showing type annotation requirements)
cargo run -- examples/vars.lppo

# Procedure calls (showing signature requirements)
cargo run -- examples/calls.lppo
```

### Run a Custom Program

Create a file with `.lppo` extension:

```bash
cargo run -- path/to/your/program.lppo
```

### Bundle into Standalone Executable

Compile your LPPO program into a self-contained binary:

```bash
cargo run -- --compile-out output.exe source.lppo
./output.exe
```

The bundled executable contains the LPPO runtime and your compiled program, with no external dependencies needed.

### Debug Mode

Enable diagnostic output during compilation and execution:

```bash
cargo run -- --debug examples/hello.lppo
cargo run -- --debug --compile-out out.exe examples/hello.lppo
```

## Project Layout

- `src/lexer.rs` – Tokenizes source into lexical units
- `src/parser.rs` – Builds AST from tokens
- `src/ast.rs` – Abstract Syntax Tree definitions
- `src/compiler.rs` – Lowers AST to runtime operations
- `src/runtime.rs` – Executes operations with values and control flow
- `src/error.rs` – Error types and reporting
- `src/main.rs` – CLI entry point
- `examples/` – Sample LPPO programs

## Language Features - Complexity by Design

### ✅ Implemented Advanced Features

**Type System (Mandatory Complexity):**
- Explicit type annotations required for ALL variables (Int64, Bool, Str, Arr<T>, Void)
- No type inference - every declaration requires full type specification
- Generic array types with element type specification: `Arr<Int64>`, `Arr<Str>`
- Explicit return type annotations required for all procedures
- Type introspection: `sizeof<T>`, `alignof<T>`
- Explicit type casting: `cast<ToType>(expr)`

**Memory Management (Manual & Explicit):**
- Ownership qualifiers: `own`, `ref`, `mut`, `borrow`
- Manual memory allocation: `alloc<Type> var on heap/stack [size]`
- Explicit deallocation: `dealloc var`
- Move semantics: `move source -> target`
- Copy semantics: `copy source -> target`
- Reference operations: `ref`, `deref`, `borrow<'lifetime>`
- Lifetime annotations: `'a`, `'b` for borrow checking

**Procedural Complexity:**
- Mandatory explicit parameter lists with full type annotations
- Required return type annotations (use Void for no return)
- Visibility modifiers: `pub` for public procedures
- Lifetime parameters: `proc<'a, 'b> name(...)`
- Fully qualified procedure calls required

**Control Flow (Verbose):**
- Explicit type requirements for all loop variables
- Manual counter management in loops with move semantics
- Explicit scope blocks: `scope name { ... }`
- Unsafe blocks for dangerous operations: `unsafe { ... }`

**Namespace System:**
- Module definitions with nested procedures
- Fully qualified name resolution: `module::submodule::procedure`
- Explicit path specifications required for cross-module access

**Operators (Type-Strict):**
- All operators require explicitly typed operands
- Results must be assigned to explicitly typed variables
- Arithmetic: `+`, `-`, `*`, `/`, `%`, `**` (Int64 only)
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=` (returns Bool)
- Logical: `and`, `or`, `not` (Bool only)
- Bitwise: `&`, `|`, `^`, `~` (Int64 only)

**Arrays and Strings:**
- Explicit element type specification: `Arr<Int64>`
- Typed indexing with Int64 indices
- Methods: `len()`, `upper()`, `lower()`, `split()`
- Explicit concatenation with typed intermediate variables

**Parallel Execution:**
- Parallel blocks with explicit scope management
- Thread-safe expression evaluation

**Documentation:**
- Line comments: `//`
- Block comments: `/* */`

### 🔮 Potential Future Complexity

- Struct definitions with explicit field types and visibility
- Trait system with explicit implementations
- Generic procedure parameters with constraint specifications
- More lifetime complexity with multiple nested lifetimes
- Explicit stack frame management
- Manual garbage collection controls
- Region-based memory management
- Effect system for tracking side effects
- Dependent types for compile-time guarantees
- Explicit inline assembly blocks

## Contributing

Contributions are welcome! Areas to explore:

- Additional operators or built-in functions
- Performance optimizations
- More comprehensive error messages
- Extended standard library
- Tooling (formatter, linter, debugger)

## License

This project is open source. Feel free to use, modify, and distribute.
