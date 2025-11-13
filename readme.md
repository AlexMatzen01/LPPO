# LPPO v3.0 - Ultra-Low-Level Assembly-Like Language with Cryptic Syntax

> **⚠️ EXTREME COMPLEXITY WARNING ⚠️**
> 
> LPPO v3.0 is **EXTREMELY complex, cryptic, and intentionally hostile to beginners**. This language uses assembly-like abbreviations and machine-level terminology. It is designed for MAXIMUM difficulty with ZERO readability. Every aspect requires obscure machine-level knowledge.

LPPO v3.0 is an **ultra-low-level, cryptic, and deliberately obfuscated** programming language that rejects all human readability in favor of machine-like terseness. It requires:

- **CRYPTIC ABBREVIATIONS** - All keywords replaced with assembly-like mnemonics
- **MANDATORY type annotations** with machine-level bit specifications (I64, BOOL8, STRPTR)
- **MANUAL memory management** with HEAPALLOC, HEAPFREE, MEMXFER operations
- **EXPLICIT ownership semantics** - OWNR, MEMREF, MUTBL, BORROW must be specified
- **VERBOSE procedure signatures** - PROCDEF with all parameters and returns fully typed
- **STACK FRAME tracking** - STKFRAME and FRAMEEND for explicit scope boundaries
- **QUALIFIED namespaces** - module::path::resolution required with MODDEF
- **NO syntactic sugar** - maximum cryptic verbosity enforced
- **ASSEMBLY-LIKE syntax** - CJMP, JMPDEF, WHLLP, ITRLP, INFLP for control flow

Written in Rust, LPPO compiles source code with comprehensive type checking, ownership validation, and advanced memory semantics.

> **Status:** v3.0 - EXTREME Complexity with Ultra-Low-Level Cryptic Syntax
>
> This language is **INTENTIONALLY HOSTILE** to human readability. It is **NOT designed for productivity**. It is **NOT designed for ease of learning**. It is **NOT designed for rapid development**. 
>
> LPPO v3.0 exists to demonstrate what happens when you maximize cryptic complexity and minimize human comprehension. Every declaration uses machine-level abbreviations requiring deep understanding of low-level computing.

## 🎉 What's New in v3.0

### Ultra-Low-Level Cryptic Syntax Revolution

LPPO v3.0 **COMPLETELY OVERHAULS** the syntax to be maximally cryptic and assembly-like:

#### Cryptic Keyword Replacements (No More Conversational Keywords!)

| V2.0 (Old)  | V3.0 (New)    | Description                           |
|-------------|---------------|---------------------------------------|
| `proc`      | `PROCDEF`     | Procedure definition                  |
| `let`       | `DECLVAR`     | Variable declaration                  |
| `out`       | `EMIT`        | Output/emission instruction           |
| `call`      | `PROCCALL`    | Procedure call instruction            |
| `if`        | `CJMP`        | Conditional jump                      |
| `elif`      | `CJMPALT`     | Alternative conditional jump          |
| `else`      | `JMPDEF`      | Default jump target                   |
| `while`     | `WHLLP`       | While loop construct                  |
| `for`       | `ITRLP`       | Iterator loop                         |
| `loop`      | `INFLP`       | Infinite loop                         |
| `break`     | `LPEXIT`      | Loop exit instruction                 |
| `continue`  | `LPSKIP`      | Loop skip to next iteration           |
| `return`    | `RETVL`       | Return value instruction              |
| `and`       | `LOGAND`      | Logical AND operation                 |
| `or`        | `LOGOR`       | Logical OR operation                  |
| `not`       | `LOGNOT`      | Logical NOT operation                 |
| `scope`     | `STKFRAME`    | Stack frame boundary                  |
| `parallel`  | `PARLBLK`     | Parallel execution block              |
| `input`     | `STDIN`       | Standard input read                   |
| `move`      | `MEMXFER`     | Memory transfer operation             |
| `copy`      | `MEMCPY`      | Memory copy operation                 |
| `ref`       | `MEMREF`      | Memory reference                      |
| `deref`     | `MEMDEREF`    | Memory dereference                    |
| `alloc`     | `HEAPALLOC`   | Heap allocation                       |
| `dealloc`   | `HEAPFREE`    | Heap deallocation                     |

#### Machine-Level Type Names (Explicit Bit Specifications!)

| V2.0 (Old)  | V3.0 (New)    | Description                           |
|-------------|---------------|---------------------------------------|
| `Int64`     | `I64`         | 64-bit signed integer                 |
| `Bool`      | `BOOL8`       | 8-bit boolean value                   |
| `Str`       | `STRPTR`      | String pointer type                   |
| `Arr`       | `ARRPTR`      | Array pointer type                    |
| `Void`      | `VOID0`       | Zero-byte void type                   |
| `own`       | `OWNR`        | Ownership qualifier                   |
| `mut`       | `MUTBL`       | Mutable qualifier                     |
| `pub`       | `PUBEXP`      | Public export visibility              |
| `module`    | `MODDEF`      | Module definition                     |

### V3.0 Syntax Philosophy

- **Cryptic abbreviations**: Replace all readable keywords with terse mnemonics
- **Assembly-like**: Control flow uses jump-style terminology (CJMP, JMPDEF)
- **Machine-level types**: All types specify bit sizes or pointer nature
- **Memory operations**: Explicit MEMXFER, MEMCPY, HEAPALLOC terminology
- **Stack awareness**: STKFRAME and FRAMEEND for explicit scope management
- **Zero readability**: Intentionally hostile to casual reading

### Enhanced Examples

- **[Hello V3.0](examples/hello_v3.lppo)**: Demonstrating new cryptic syntax
- **[Comprehensive V3.0](examples/comprehensive_v3.lppo)**: All features with ultra-low-level syntax
- **Backward Compatibility**: V2.0 examples still work during transition

### Future-Ready AST

- Struct definitions with fields and methods (STRUCTDEF)
- Enum types with pattern matching (ENUMDEF, PATMCH)
- Trait system with implementations (TRAITDEF, IMPLBLK)
- Generic types and constraints
- Advanced type annotations (OPT, RES, Tuple types)

All groundwork laid for future feature implementation!

## Why LPPO v3.0 is EXTREMELY Complex

- **Cryptic Abbreviations:** ALL keywords replaced with assembly-like mnemonics (PROCDEF, DECLVAR, CJMP)
- **Mandatory Type System:** EVERY variable requires explicit machine-level type with bit specs (I64, BOOL8, STRPTR)
- **Explicit Memory Management:** Manual HEAPALLOC, HEAPFREE, MEMXFER operations
- **Ultra-Verbose Syntax:** ALL procedures use PROCDEF with PUBEXP visibility and explicit types
- **Ownership Semantics:** Explicit OWNR, MEMREF, MUTBL, and BORROW qualifiers required
- **Stack Frame Management:** Explicit STKFRAME and FRAMEEND for all scope boundaries
- **Assembly-Like Control Flow:** CJMP (if), CJMPALT (elif), JMPDEF (else), WHLLP (while), ITRLP (for), INFLP (loop)
- **Low-Level Operations:** MEMXFER (move), MEMCPY (copy), MEMDEREF (deref), HEAPSEG, STKSEG
- **Namespace Complexity:** Fully qualified paths with MODDEF::submodule::item syntax
- **No Readability:** Maximum cryptic verbosity, zero human-friendly syntax, intentional obfuscation

## Language Overview - Ultra-Low-Level Cryptic Complexity

### V3.0 Example Demonstrating EXTREME Cryptic Verbosity

```lppo
// EVERY procedure REQUIRES:
// - PUBEXP visibility modifier (replaces "pub")
// - PROCDEF keyword (replaces "proc")
// - Explicit parameter list with machine-level type annotations
// - Explicit return type with bit specification (VOID0, I64, etc.)
PUBEXP PROCDEF main() -> VOID0 {
    // EVERY variable REQUIRES:
    // - DECLVAR keyword (replaces "let")
    // - Explicit ownership qualifier (OWNR, MEMREF, BORROW, MUTBL)
    // - Explicit machine-level type annotation (I64, STRPTR, ARRPTR)
    // - No type inference whatsoever
    DECLVAR OWNR nums: ARRPTR<I64> = [1, 2, 3, 4, 5];
    DECLVAR OWNR msg: STRPTR = "Processing array:";
    EMIT msg, nums;
    
    // For loops use ITRLP (iterator loop) with INSET keyword
    DECLVAR OWNR two: I64 = 2;
    DECLVAR OWNR zero: I64 = 0;
    
    ITRLP n: I64 INSET nums {
        DECLVAR OWNR remainder: I64 = n % two;
        
        // Conditional uses CJMP (conditional jump) instead of "if"
        // Alternative paths use CJMPALT and JMPDEF
        CJMP remainder == zero {
            DECLVAR OWNR even_msg: STRPTR = "is even";
            EMIT n, even_msg;
        } JMPDEF {
            DECLVAR OWNR odd_msg: STRPTR = "is odd";
            EMIT n, odd_msg;
        }
    }
    
    // Procedure calls use PROCCALL with explicit argument list
    DECLVAR OWNR name: STRPTR = "LPPO";
    DECLVAR OWNR year: I64 = 2024;
    PROCCALL greet(name, year);
}

// ALL parameters REQUIRE explicit machine-level type annotations
PUBEXP PROCDEF greet(name: STRPTR, year: I64) -> VOID0 {
    DECLVAR OWNR hello: STRPTR = "Hello";
    DECLVAR OWNR exclaim: STRPTR = "!";
    DECLVAR OWNR year_label: STRPTR = "Year:";
    EMIT hello, name, exclaim, year_label, year;
}
```

### V2.0 Compatibility

V2.0 syntax still works for backward compatibility during the transition period:

```lppo
// V2.0 style (still supported)
pub proc main() -> Void {
    let own msg: Str = "Hello from V2.0";
    out msg;
}
```

### Data Types (V3.0 Machine-Level Specifications)

#### V3.0 Ultra-Low-Level Types (Cryptic Abbreviations)

- **I64:** 64-bit signed integers - `DECLVAR OWNR x: I64 = 42;`
- **BOOL8:** 8-bit boolean values - `DECLVAR OWNR flag: BOOL8 = BOOLT;`
- **STRPTR:** String pointer type - `DECLVAR OWNR msg: STRPTR = "hello";`
- **ARRPTR<T>:** Array pointer with element type - `DECLVAR OWNR nums: ARRPTR<I64> = [1, 2, 3];`
- **VOID0:** Zero-byte void return - `PROCDEF foo() -> VOID0 { }`

#### V2.0 Backward Compatible Types (Still Supported)

- **Int64:** Maps to I64 internally
- **Bool:** Maps to BOOL8 internally
- **Str:** Maps to STRPTR internally
- **Arr<T>:** Maps to ARRPTR<T> internally
- **Void:** Maps to VOID0 internally

### Type Annotations (V3.0 CRYPTIC MANDATORY SPECIFICATION)

**Every single variable** requires explicit machine-level type annotation with ownership qualifier:

```lppo
// WRONG - Type inference not supported:
// DECLVAR x = 10;  // ERROR: Missing type annotation

// CORRECT - Explicit I64 type required:
DECLVAR OWNR x: I64 = 10;

// WRONG - Cannot infer from expression:
// DECLVAR y = x * 2 + 5;  // ERROR

// CORRECT - Intermediate results need machine-level types:
DECLVAR OWNR two: I64 = 2;
DECLVAR OWNR five: I64 = 5;
DECLVAR OWNR mult: I64 = x * two;
DECLVAR OWNR y: I64 = mult + five;

// WRONG - Type inference for booleans:
// DECLVAR flag = x > y;  // ERROR

// CORRECT - Explicit BOOL8 type with LOGAND/LOGOR:
DECLVAR OWNR gt_result: BOOL8 = x > y;
DECLVAR OWNR eq_result: BOOL8 = y == 25;
DECLVAR OWNR flag: BOOL8 = gt_result LOGOR eq_result;

// Arrays REQUIRE ARRPTR with element type specification:
DECLVAR OWNR arr: ARRPTR<I64> = [1, 2, 3];
```

#### V2.0 Style (Still Supported for Transition)

```lppo
// Old V2.0 syntax still works:
let own x: Int64 = 10;
let own flag: Bool = x > 5;
let own arr: Arr<Int64> = [1, 2, 3];
```

### Operators (V3.0 Machine-Level Typed Operands)

**Arithmetic:** `+`, `-`, `*`, `/`, `%` (modulo), `**` (power)
- All operands must be explicitly typed I64
- Results must be assigned to explicitly typed I64 variables

**Comparison:** `==`, `!=`, `<`, `>`, `<=`, `>=`
- Results must be explicitly typed as BOOL8
- Cannot use in conditions without explicit BOOL8 variable

**Logical (V3.0 Cryptic):** `LOGAND`, `LOGOR`, `LOGNOT`
- Require explicitly typed BOOL8 operands
- Results must be assigned to BOOL8 variables
- V2.0 syntax (`and`, `or`, `not`) still supported

**Bitwise:** `&`, `|`, `^` (xor), `~` (not)
- Operate on I64 types only
- Results must be assigned to I64 variables

**Unary:** `-` (negation), `LOGNOT` (logical), `~` (bitwise not)
- Type of operand determines operation type

### Control Flow (V3.0 Assembly-Like Jump Instructions)

#### CJMP/CJMPALT/JMPDEF (Conditional/Alternative/Default Jumps)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
DECLVAR OWNR x: I64 = 15;
DECLVAR OWNR ten: I64 = 10;
DECLVAR OWNR five: I64 = 5;

// WRONG: Cannot use expression directly
// CJMP x > 10 { ... }  // ERROR

// CORRECT: Store comparison in typed BOOL8 variable
DECLVAR OWNR is_large: BOOL8 = x > ten;
CJMP is_large {
    DECLVAR OWNR msg: STRPTR = "Large";
    EMIT msg;
} CJMPALT x > five {
    DECLVAR OWNR msg: STRPTR = "Medium";
    EMIT msg;
} JMPDEF {
    DECLVAR OWNR msg: STRPTR = "Small";
    EMIT msg;
}
```

**V2.0 Backward Compatible Syntax:**
```lppo
let own x: Int64 = 15;
let own is_large: Bool = x > 10;
if is_large {
    let own msg: Str = "Large";
    out msg;
} elif x > 5 {
    let own msg: Str = "Medium";
    out msg;
} else {
    let own msg: Str = "Small";
    out msg;
}
```

#### WHLLP Loop (While Loop with Mutable Counter)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
DECLVAR MUTBL OWNR i: I64 = 0;
DECLVAR OWNR limit: I64 = 5;

WHLLP i < limit {
    DECLVAR OWNR label: STRPTR = "Count:";
    EMIT label, i;
    
    // MEMXFER (memory transfer) for reassignment
    DECLVAR OWNR i_next: I64 = i + 1;
    MEMXFER i_next -> i;
}
```

**V2.0 Backward Compatible Syntax:**
```lppo
let mut own i: Int64 = 0;
let own limit: Int64 = 5;

while i < limit {
    let own label: Str = "Count:";
    out label, i;
    
    let own i_next: Int64 = i + 1;
    move i_next -> i;
}
```

#### ITRLP Loop (Iterator Loop with INSET)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
DECLVAR OWNR items: ARRPTR<I64> = [10, 20, 30];

// WRONG: Type inference not allowed
// ITRLP item INSET items { ... }  // ERROR

// CORRECT: Explicit I64 type for loop variable with INSET keyword
ITRLP item: I64 INSET items {
    DECLVAR OWNR label: STRPTR = "Item:";
    EMIT label, item;
}
```

**V2.0 Backward Compatible Syntax:**
```lppo
let own items: Arr<Int64> = [10, 20, 30];

for item: Int64 in items {
    let own label: Str = "Item:";
    out label, item;
}
```

#### INFLP Loop (Infinite Loop with LPEXIT)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
DECLVAR MUTBL OWNR count: I64 = 0;
DECLVAR OWNR max: I64 = 3;

INFLP {
    DECLVAR OWNR label: STRPTR = "Iteration:";
    EMIT label, count;
    
    DECLVAR OWNR count_next: I64 = count + 1;
    MEMXFER count_next -> count;
    
    DECLVAR OWNR should_break: BOOL8 = count >= max;
    CJMP should_break {
        LPEXIT;  // Exit loop instruction
    }
}
```

**V2.0 Backward Compatible Syntax:**
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

**Loop Control (V3.0):** `LPEXIT` (exit loop), `LPSKIP` (skip to next iteration)

**Loop Control (V2.0):** `break` (exit loop), `continue` (skip to next iteration)

### User Input (Explicit Type Specification Required)

LPPO provides the `input` statement for reading user input from standard input. Like all LPPO constructs, input requires **explicit type annotation** for the variable being read.

#### Syntax
```lppo
input variable_name: Type;                    // Basic input
input variable_name: Type with prompt_expr;   // Input with prompt
```

#### Supported Input Types

**String Input**
```lppo
pub proc main() -> Void {
    input name: Str with "Enter your name:";
    let own greeting: Str = "Hello,";
    out greeting, name;
}
```

**Integer Input**
```lppo
pub proc main() -> Void {
    input age: Int64 with "Enter your age:";
    let own double: Int64 = age + age;
    out "Double your age:", double;
}
```

**Boolean Input**
```lppo
pub proc main() -> Void {
    input is_member: Bool with "Are you a member? (true/false):";
    let own status: Str = "Membership status:";
    out status, is_member;
}
```

**Accepted boolean inputs:** `true`, `false`, `yes`, `no`, `y`, `n`, `1`, `0` (case-insensitive)

**Array Input**
```lppo
pub proc main() -> Void {
    // Comma-separated format: 1,2,3,4,5
    input numbers: Arr<Int64> with "Enter numbers (comma-separated):";
    out "You entered:", numbers;
    
    // Also supports bracket format: [1, 2, 3, 4, 5]
    input more_numbers: Arr<Int64>;
    out "Additional numbers:", more_numbers;
}
```

**Input Processing Example**
```lppo
pub proc main() -> Void {
    input nums: Arr<Int64> with "Enter numbers to sum:";
    
    let mut own total: Int64 = 0;
    let own zero: Int64 = 0;
    
    for num: Int64 in nums {
        let own new_total: Int64 = total + num;
        move new_total -> total;
    }
    
    out "Sum:", total;
}
```

**Input Validation:**
- Type mismatches produce clear error messages
- Invalid integers: `"Invalid Int64 input: 'abc'"`
- Invalid booleans: `"Invalid Bool input: 'maybe'. Expected: true/false, yes/no, 1/0"`
- Invalid array elements: `"Invalid Int64 in array: 'xyz'"`

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

### Quick Start

See our **[Tutorial](docs/TUTORIAL.md)** for a step-by-step learning guide!

### Run Examples (All Use Complex Syntax)

```bash
# Basic hello world (with explicit types)
cargo run -- examples/hello.lppo

# Comprehensive feature demo (maximum verbosity)
cargo run -- examples/comprehensive.lppo

# Advanced math library with modules
cargo run -- examples/math_library.lppo

# Complex features showcase
cargo run -- examples/advanced_features.lppo

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

### Source Code
- `src/lexer.rs` – Tokenizes source into lexical units
- `src/parser.rs` – Builds AST from tokens  
- `src/ast.rs` – Abstract Syntax Tree definitions (with struct/enum/trait support)
- `src/compiler.rs` – Lowers AST to runtime operations
- `src/runtime.rs` – Executes operations with values and control flow
- `src/error.rs` – Enhanced error types with detailed reporting
- `src/main.rs` – CLI entry point with bundled executable support

### Documentation (2000+ lines!)
- `docs/LANGUAGE_SPEC.md` – Complete language specification
- `docs/TUTORIAL.md` – Beginner to advanced tutorial
- `docs/ARCHITECTURE.md` – Implementation details and design
- `docs/BEST_PRACTICES.md` – Coding standards and patterns

### Examples
- `examples/hello.lppo` – Simple hello world
- `examples/comprehensive.lppo` – Feature showcase
- `examples/math_library.lppo` – Advanced module organization
- `examples/advanced_features.lppo` – Complex patterns
- `examples/loops.lppo`, `operators.lppo`, `vars.lppo`, etc.

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
- **NEW**: 2000+ lines of comprehensive documentation
- **NEW**: Language specification, tutorial, architecture guide, best practices

**Error Handling (NEW in v2.0):**
- Detailed error messages with source location
- Line and column tracking
- Source code context display
- Helpful suggestions for fixes
- Error severity levels (Info, Warning, Error, Fatal)
- Stack traces for runtime errors

### 🔮 Potential Future Complexity

**In Progress (AST Support Added):**
- Struct definitions with explicit field types and visibility
- Enum types with pattern matching capabilities
- Trait system with explicit implementations
- Generic procedure parameters with constraint specifications
- Advanced type annotations: Option<T>, Result<T,E>, Tuple types

**Planned:**
- More lifetime complexity with multiple nested lifetimes
- Explicit stack frame management
- Manual garbage collection controls
- Region-based memory management
- Effect system for tracking side effects
- Dependent types for compile-time guarantees
- Explicit inline assembly blocks

## Key Improvements in v2.0

### 📚 Documentation Excellence
- **4 comprehensive guides** totaling 2000+ lines
- **Complete language specification** with every syntax detail
- **Step-by-step tutorial** from beginner to advanced
- **Architecture deep-dive** for contributors
- **Best practices** and coding standards

### 🛠️ Enhanced Error System
- **Rich error messages** with precise locations
- **Source context** showing exactly where errors occur
- **Helpful suggestions** for common mistakes
- **Severity levels** for appropriate responses
- **Stack traces** for debugging runtime issues

### 📦 Advanced Examples
- **Math library** showcasing complex module organization
- **Advanced features** demonstrating memory management
- **Working code** for every documented feature
- **Real-world patterns** you can learn from

### 🏗️ Future-Ready Architecture
- **AST support** for structs, enums, and traits
- **Type system extensions** for generics and advanced types
- **Clean separation** of concerns in compiler stages
- **Extensible design** for adding new features

## Contributing

Contributions are welcome! Areas to explore:

- **Parser Implementation**: Complete struct/enum/trait parsing
- **Runtime Support**: Implement advanced type system features
- **Standard Library**: File I/O, networking, collections
- **Tooling**: Formatter, linter, language server, debugger
- **Performance**: Optimization passes, JIT compilation
- **Testing**: Comprehensive test suite with coverage
- **Documentation**: More examples and use cases
- **Error Recovery**: Better parser error handling

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for implementation details.

## Resources

- **[Language Specification](docs/LANGUAGE_SPEC.md)** - Complete syntax reference
- **[Tutorial](docs/TUTORIAL.md)** - Learn LPPO step by step
- **[Architecture](docs/ARCHITECTURE.md)** - Implementation details
- **[Best Practices](docs/BEST_PRACTICES.md)** - Coding guidelines

## License

This project is open source. Feel free to use, modify, and distribute.
