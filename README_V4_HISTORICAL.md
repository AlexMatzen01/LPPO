# ⚠️ HISTORICAL DOCUMENT - V4.0 ERA (DISCONTINUED)

> **🛑 THIS DOCUMENTATION IS DEPRECATED AND IRRELEVANT**
>
> This document describes V4.0 and earlier versions of LPPO which had an "intentionally hostile" design philosophy. These versions and their documentation are **discontinued** as of LPPO V5.1.
>
> **For current documentation, see:**
> - [LPPO V5.1 README](readme.md) - Current version
> - [V5.1 Release Notes](V5.1_RELEASE_NOTES.md) - Full feature list
> - [V5.1 Migration Guide](V5.1_MIGRATION_GUIDE.md) - How to upgrade
>
> This file is kept for historical reference only.

---

# LPPO v5.0 - Complex but Comprehensible Programming Language (Historical)

> **📚 BALANCED COMPLEXITY APPROACH**
> 
> LPPO v5.0 represents a **major philosophy shift** from V4.0's "intentionally hostile" design. V5.0 is **complex but comprehensible** - maintaining substantial type system complexity and explicit control while using **human-readable syntax**.

LPPO v5.0 is a **statically-typed, explicitly-managed** programming language that balances complexity with readability. It features:

- **READABLE KEYWORDS** - Clear, descriptive names (proc, let, out, if, while) instead of cryptic abbreviations
- **EXPLICIT TYPE SYSTEM** - Mandatory type annotations for all variables and procedures
- **OWNERSHIP SEMANTICS** - Manual memory management with clear ownership qualifiers
- **OPTIONAL ADVANCED FEATURES** - Use complexity when you need it, not mandatory everywhere
- **STRUCTURED CONTROL FLOW** - Familiar constructs with explicit type requirements
- **MODULE SYSTEM** - Clear namespace organization with qualified paths
- **PARALLEL EXECUTION** - Built-in support for concurrent operations
- **NO MAGIC** - Everything is explicit, no hidden behaviors

Written in Rust, LPPO compiles source code with strong type checking and explicit memory management.

> **Status:** v5.0 - BALANCED COMPLEXITY - Complex but Comprehensible
>
> LPPO v5.0 is **NOT Python** - it maintains significant complexity through explicit typing and manual memory management.
>
> LPPO v5.0 is **NOT V4.0** - it removes the intentional hostility and cryptic syntax that made code unreadable.
>
> LPPO v5.0 achieves the "Goldilocks zone" - complex enough to teach systems programming concepts, readable enough to be practical.

## 🎯 What's New in v5.0 - HUMAN READABILITY RESTORED

LPPO v5.0 **pulls back** from V4.0's extreme hostility while **maintaining** meaningful complexity. V5.0 makes code comprehensible without sacrificing type safety or explicit control.

### Readable Keywords (NEW in V5.0!)

V5.0 **emphasizes** clear, descriptive keywords:

```lppo
pub proc main() -> Void {
    let own msg: Str = "Readable!";
    out msg;
}
```

### Optional Advanced Features (NEW in V5.0!)

V5.0 makes complexity **optional** rather than mandatory:

```lppo
// Simple when you don't need complexity
let own x: Int64 = 42;

// Complex when you need control
alloc<Arr<Int64>> buffer on heap [100];
```

### No Mandatory Annotations (NEW in V5.0!)

V5.0 removes mandatory verbose markers - use them only when needed:

```lppo
// V5.0: Simple and clean
let own x: Int64 = 42;

// V4.0 required: DECLVAR OWNR MEMALIGN 8 x: I64 = 42; TYPECHK x: I64;
```

### V5.0 Core Feature Set

#### 1. Readable Type System
- `Int64` - 64-bit signed integer (not `I64`)
- `Bool` - Boolean value (not `BOOL8`)
- `Str` - String type (not `STRPTR`)
- `Arr<T>` - Array type (not `ARRPTR<T>`)
- `Void` - Void return (not `VOID0`)
- `Option<T>` - Optional values
- `Result<T,E>` - Result types

#### 2. Clear Control Flow
- `if` / `elif` / `else` - Conditionals (not `CJMP`/`JMPDEF`)
- `while` - While loops (not `WHLLP`)
- `for` / `in` - For loops (not `ITRLP`/`INSET`)
- `loop` - Infinite loops (not `INFLP`)
- `break` / `continue` - Loop control (not `LPEXIT`/`LPSKIP`)
- `return` - Return values (not `RETVL`)

#### 3. Understandable Keywords
- `proc` - Procedure definition (not `PROCDEF`)
- `let` - Variable declaration (not `DECLVAR`)
- `out` - Output statement (not `EMIT`)
- `call` - Procedure call (not `PROCCALL`)
- `module` - Module definition (not `MODDEF`)
- `pub` - Public visibility (not `PUBEXP`)

#### 4. Explicit Memory Management
- `own` - Owned value (not `OWNR`)
- `mut` - Mutable qualifier (not `MUTBL`)
- `ref` - Reference (not `MEMREF`)
- `alloc` / `dealloc` - Heap allocation (not `HEAPALLOC`/`HEAPFREE`)
- `move` - Move semantics (not `MEMXFER`)
- `copy` - Copy semantics (not `MEMCPY`)
- `borrow` - Borrowing (explicit when needed)

#### 5. Structured Organization
- `scope` - Explicit scope blocks (not `STKFRAME`)
- `parallel` - Parallel execution (not `PARLBLK`)
- `module` - Namespace organization
- `use` - Import modules
- Qualified paths: `module::function`

#### 6. Optional Advanced Features (Not Mandatory!)
- `sizeof<T>` - Type size queries (not `TYPESZ`)
- `alignof<T>` - Alignment queries (not `TYPEALN`)
- `cast<T>` - Type casting (not `TYPECAST`)
- `unsafe` - Unsafe blocks (not `UNSAFEBLK`)
- Lifetime annotations: `'a`, `'b` when needed

#### 7. Logical Operators
- `and` - Logical AND (not `LOGAND`)
- `or` - Logical OR (not `LOGOR`)
- `not` - Logical NOT (not `LOGNOT`)
- Boolean values: `true` / `false` (not `BOOLT`/`BOOLF`)
### V5.0 Example (Readable and Clear)

```lppo
// V5.0: Clean, readable syntax with maintained complexity
pub proc main() -> Void {
    // Type annotations still required (complexity maintained)
    let own greeting: Str = "Hello from LPPO V5.0!";
    let own numbers: Arr<Int64> = [1, 2, 3, 4, 5];
    
    out greeting;
    out "Numbers:", numbers;
    
    // Explicit control flow with readable keywords
    for num: Int64 in numbers {
        let own doubled: Int64 = num * 2;
        out "Doubled:", doubled;
    }
    
    // Parallel execution - clear but not simplified
    parallel {
        out "Parallel task 1";
        out "Parallel task 2";
    }
}
```

### V5.0 vs V4.0 vs V3.0 vs V2.0

| Feature | V2.0 | V3.0 | V4.0 | V5.0 |
|---------|------|------|------|------|
| Keywords | 50+ | 70+ (cryptic) | 170+ (ultra-verbose) | **50+ (readable)** |
| Block markers | Optional | Optional | Mandatory | **Optional** |
| Memory alignment | Implicit | Implicit | Explicit required | **Optional** |
| Type annotations | Required | Required | Required | **Required** |
| Readability | Medium | Very Low | None | **High** |
| Complexity | High | Extreme | Ultimate | **Balanced** |
| Philosophy | Verbose | Cryptic | Hostile | **Comprehensible** |

## Why LPPO V5.0 is Still Complex (Not Python!)

V5.0 pulls back from V4.0's hostility but **maintains meaningful complexity**:

- **Mandatory Type Annotations:** EVERY variable requires explicit type specification (Int64, Bool, Str, Arr<T>, etc.)
- **Explicit Memory Management:** Manual allocation, deallocation, move/copy semantics
- **Ownership System:** Explicit ownership qualifiers (own, mut, ref, borrow) required
- **No Type Inference:** Cannot infer types from expressions - all must be declared
- **Manual Control Flow:** Explicit loop management with typed counters
- **Lifetime Annotations:** References require explicit lifetime tracking when needed
- **Structured Scope:** Explicit scope blocks and stack frame management
- **Qualified Paths:** Full module::function paths for organization
- **No Garbage Collection:** Manual memory management throughout
- **No Magic:** Everything is explicit - no hidden behaviors or automatic conversions

## Language Overview - Balanced Readable Complexity

### V5.0 Example Demonstrating Readable Complexity

```lppo
// V5.0: Clear keywords with maintained complexity
pub proc main() -> Void {
    // Type annotations still required (no inference)
    let own numbers: Arr<Int64> = [1, 2, 3, 4, 5];
    let own message: Str = "Processing array:";
    out message, numbers;
    
    // Explicit types in loops
    let own two: Int64 = 2;
    let own zero: Int64 = 0;
    
    for n: Int64 in numbers {
        let own remainder: Int64 = n % two;
        
        // Clear control flow
        if remainder == zero {
            let own even_msg: Str = "is even";
            out n, even_msg;
        } else {
            let own odd_msg: Str = "is odd";
            out n, odd_msg;
        }
    }
    
    // Explicit procedure calls
    let own name: Str = "LPPO";
    let own year: Int64 = 2025;
    call greet(name, year);
}

// Procedures require full type signatures
pub proc greet(name: Str, year: Int64) -> Void {
    let own hello: Str = "Hello";
    let own exclaim: Str = "!";
    let own year_label: Str = "Year:";
    out hello, name, exclaim, year_label, year;
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

### User Input (V3.0 STDIN Instruction)

#### V3.0 Ultra-Low-Level Syntax

LPPO V3.0 uses the `STDIN` instruction for reading user input. Like all V3.0 constructs, input requires **explicit machine-level type annotation**.

**Syntax:**
```lppo
STDIN variable_name: Type;                    // Basic input
STDIN variable_name: Type with prompt_expr;   // Input with prompt
```

**String Input:**
```lppo
PUBEXP PROCDEF main() -> VOID0 {
    STDIN name: STRPTR with "Enter your name:";
    DECLVAR OWNR greeting: STRPTR = "Hello,";
    EMIT greeting, name;
}
```

**Integer Input:**
```lppo
PUBEXP PROCDEF main() -> VOID0 {
    STDIN age: I64 with "Enter your age:";
    DECLVAR OWNR double: I64 = age + age;
    EMIT "Double your age:", double;
}
```

**Boolean Input:**
```lppo
PUBEXP PROCDEF main() -> VOID0 {
    STDIN is_member: BOOL8 with "Are you a member? (BOOLT/BOOLF):";
    DECLVAR OWNR status: STRPTR = "Membership status:";
    EMIT status, is_member;
}
```

**Accepted boolean inputs:** `BOOLT`, `BOOLF`, `true`, `false`, `yes`, `no`, `y`, `n`, `1`, `0` (case-insensitive)

**Array Input:**
```lppo
PUBEXP PROCDEF main() -> VOID0 {
    // Comma-separated format: 1,2,3,4,5
    STDIN numbers: ARRPTR<I64> with "Enter numbers (comma-separated):";
    EMIT "You entered:", numbers;
    
    // Also supports bracket format: [1, 2, 3, 4, 5]
    STDIN more_numbers: ARRPTR<I64>;
    EMIT "Additional numbers:", more_numbers;
}
```

**Input Processing Example:**
```lppo
PUBEXP PROCDEF main() -> VOID0 {
    STDIN nums: ARRPTR<I64> with "Enter numbers to sum:";
    
    DECLVAR MUTBL OWNR total: I64 = 0;
    DECLVAR OWNR zero: I64 = 0;
    
    ITRLP num: I64 INSET nums {
        DECLVAR OWNR new_total: I64 = total + num;
        MEMXFER new_total -> total;
    }
    
    EMIT "Sum:", total;
}
```

#### V2.0 Backward Compatible Syntax

```lppo
pub proc main() -> Void {
    input name: Str with "Enter your name:";
    let own greeting: Str = "Hello,";
    out greeting, name;
}
```

**Input Validation:**
- Type mismatches produce clear error messages
- Invalid integers: `"Invalid I64 input: 'abc'"`
- Invalid booleans: `"Invalid BOOL8 input: 'maybe'. Expected: BOOLT/BOOLF, true/false, yes/no, 1/0"`
- Invalid array elements: `"Invalid I64 in array: 'xyz'"`

### Arrays and Strings (V3.0 ARRPTR and STRPTR Operations)

#### V3.0 Array Operations

**V3.0 Ultra-Low-Level Syntax:**
```lppo
// WRONG: Type inference not supported
// DECLVAR arr = [1, 2, 3, 4, 5];  // ERROR

// CORRECT: Explicit ARRPTR<T> type with element type
DECLVAR OWNR arr: ARRPTR<I64> = [1, 2, 3, 4, 5];

// All array accesses require typed index and result
DECLVAR OWNR zero: I64 = 0;
DECLVAR OWNR first: I64 = arr[zero];
EMIT first;  // First element: 1

DECLVAR OWNR neg_one: I64 = -1;
DECLVAR OWNR last: I64 = arr[neg_one];
EMIT last;  // Last element: 5

DECLVAR OWNR arr_len: I64 = arr.len();
EMIT arr_len;  // Length: 5

// Array concatenation requires explicit ARRPTR types
DECLVAR OWNR a1: ARRPTR<I64> = [1, 2];
DECLVAR OWNR a2: ARRPTR<I64> = [3, 4];
DECLVAR OWNR combined: ARRPTR<I64> = a1 + a2;  // [1, 2, 3, 4]
```

**V2.0 Backward Compatible:**
```lppo
let own arr: Arr<Int64> = [1, 2, 3, 4, 5];
let own zero: Int64 = 0;
let own first: Int64 = arr[zero];
out first;
```

#### V3.0 String Operations

**V3.0 Ultra-Low-Level Syntax:**
```lppo
// WRONG: Implicit string typing
// DECLVAR msg = "Hello World";  // ERROR

// CORRECT: Explicit STRPTR type
DECLVAR OWNR msg: STRPTR = "Hello World";

DECLVAR OWNR msg_len: I64 = msg.len();
EMIT msg_len;  // 11

DECLVAR OWNR msg_upper: STRPTR = msg.upper();
EMIT msg_upper;  // HELLO WORLD

DECLVAR OWNR msg_lower: STRPTR = msg.lower();
EMIT msg_lower;  // hello world

// Split requires explicit separator and typed result
DECLVAR OWNR text: STRPTR = "a,b,c";
DECLVAR OWNR sep: STRPTR = ",";
DECLVAR OWNR parts: ARRPTR<STRPTR> = text.split(sep);  // ["a", "b", "c"]

// String concatenation requires explicit intermediate variables
DECLVAR OWNR hello: STRPTR = "hello";
DECLVAR OWNR space: STRPTR = " ";
DECLVAR OWNR world: STRPTR = "world";
DECLVAR OWNR temp: STRPTR = hello + space;
DECLVAR OWNR result: STRPTR = temp + world;  // Concatenation
```

**V2.0 Backward Compatible:**
```lppo
let own msg: Str = "Hello World";
let own msg_len: Int64 = msg.len();
let own msg_upper: Str = msg.upper();
out msg_upper;
```

### Procedures (V3.0 PROCDEF with Explicit Signatures)

#### V3.0 Ultra-Low-Level Syntax

**ALL procedures require:**
1. Visibility modifier (`PUBEXP` or module-scoped)
2. `PROCDEF` keyword (replaces `proc`)
3. Explicit parameter list with parentheses (even if empty)
4. Machine-level type annotation for EVERY parameter
5. Explicit return type annotation (use `VOID0` for no return)

```lppo
// WRONG: Missing type annotations
// PROCDEF calculate(a, b) { ... }  // ERROR

// CORRECT: Fully explicit PROCDEF signature
PUBEXP PROCDEF calculate(a: I64, b: I64) -> VOID0 {
    DECLVAR OWNR sum: I64 = a + b;
    DECLVAR OWNR product: I64 = a * b;
    DECLVAR OWNR sum_label: STRPTR = "Sum:";
    DECLVAR OWNR prod_label: STRPTR = "Product:";
    EMIT sum_label, sum, prod_label, product;
}

// WRONG: Missing parentheses and return type
// PROCDEF main { ... }  // ERROR

// CORRECT: Explicit empty parameter list and return type
PUBEXP PROCDEF main() -> VOID0 {
    DECLVAR OWNR arg1: I64 = 5;
    DECLVAR OWNR arg2: I64 = 3;
    
    // WRONG: Missing parentheses in call
    // PROCCALL calculate;  // ERROR
    
    // CORRECT: PROCCALL with explicit argument list
    PROCCALL calculate(arg1, arg2);
}
```

#### V2.0 Backward Compatible Syntax

```lppo
pub proc calculate(a: Int64, b: Int64) -> Void {
    let own sum: Int64 = a + b;
    out sum;
}

pub proc main() -> Void {
    let own arg1: Int64 = 5;
    let own arg2: Int64 = 3;
    call calculate(arg1, arg2);
}
```

### Advanced Memory Management (V3.0 Explicit Low-Level Operations)

#### V3.0 Ultra-Low-Level Syntax

LPPO v3.0 requires explicit machine-level memory management and ownership semantics:

**Ownership Qualifiers (REQUIRED):**
- `OWNR` - Owned value (default ownership) - replaces `own`
- `MEMREF` - Immutable reference - replaces `ref`
- `MUTBL` - Mutable reference/variable - replaces `mut`
- `BORROW` - Borrowed value - replaces `borrow`

```lppo
// Owned values (most common)
DECLVAR OWNR x: I64 = 42;

// Mutable owned variable
DECLVAR MUTBL OWNR counter: I64 = 0;

// Reference to value (explicit)
DECLVAR MEMREF y: Ref<I64> = MEMREF x;

// Borrow semantics
DECLVAR BORROW z: I64 = BORROW<'a> x;
```

**Manual Memory Operations:**
```lppo
// Explicit heap allocation with HEAPALLOC
HEAPALLOC<ARRPTR<I64>> my_array MEMSEG HEAPSEG [100];

// MEMXFER (memory transfer) - replaces "move"
DECLVAR OWNR source: I64 = 10;
DECLVAR OWNR target: I64 = 0;
MEMXFER source -> target;

// MEMCPY (memory copy) - replaces "copy"
DECLVAR OWNR original: I64 = 20;
DECLVAR OWNR duplicate: I64 = 0;
MEMCPY original -> duplicate;

// Explicit deallocation with HEAPFREE
HEAPFREE my_array;
```

#### V2.0 Backward Compatible Syntax

```lppo
// Old V2.0 syntax still works:
let own x: Int64 = 42;
let mut own counter: Int64 = 0;

alloc<Arr<Int64>> my_array on heap [100];
move source -> target;
copy original -> duplicate;
dealloc my_array;
```

#### Lifetime Annotations

**V3.0 Ultra-Low-Level Syntax:**
```lppo
PUBEXP PROCDEF process<'a, 'b>(x: Ref<I64> 'a, y: Ref<I64> 'b) -> VOID0 {
    // Lifetime tracking ensures references remain valid
    DECLVAR OWNR result: I64 = MEMDEREF x + MEMDEREF y;
    EMIT result;
}
```

**V2.0 Backward Compatible:**
```lppo
pub proc process<'a, 'b>(x: Ref<Int64> 'a, y: Ref<Int64> 'b) -> Void {
    let own result: Int64 = deref x + deref y;
    out result;
}
```

#### Type Introspection

**V3.0 Ultra-Low-Level Syntax:**
```lppo
// Query size of types with TYPESZ
DECLVAR OWNR int_size: I64 = TYPESZ<I64>;
DECLVAR OWNR arr_size: I64 = TYPESZ<ARRPTR<I64>>;

// Query alignment requirements with TYPEALN
DECLVAR OWNR int_align: I64 = TYPEALN<I64>;

// Explicit type casting with TYPECAST
DECLVAR OWNR x: I64 = 42;
DECLVAR OWNR as_bool: BOOL8 = TYPECAST<BOOL8>(x);
```

**V2.0 Backward Compatible:**
```lppo
let own int_size: Int64 = sizeof<Int64>;
let own int_align: Int64 = alignof<Int64>;
let own as_bool: Bool = cast<Bool>(x);
```

### Parallel Execution (V3.0 PARLBLK with STKFRAME)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
STKFRAME parallel_section {
    PARLBLK {
        DECLVAR OWNR msg1: STRPTR = "Task 1";
        EMIT msg1;
        
        DECLVAR OWNR msg2: STRPTR = "Task 2";
        EMIT msg2;
        
        DECLVAR OWNR msg3: STRPTR = "Task 3";
        EMIT msg3;
    }
}
```

**V2.0 Backward Compatible:**
```lppo
scope parallel_section {
    parallel {
        let own msg1: Str = "Task 1";
        out msg1;
    }
}
```

### Namespace and Module System (V3.0 MODDEF)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
// Define a module with MODDEF
MODDEF math {
    PUBEXP PROCDEF add(a: I64, b: I64) -> I64 {
        DECLVAR OWNR result: I64 = a + b;
        RETVL result;
    }
    
    PUBEXP PROCDEF multiply(a: I64, b: I64) -> I64 {
        DECLVAR OWNR result: I64 = a * b;
        RETVL result;
    }
}

// Call with qualified path
PUBEXP PROCDEF main() -> VOID0 {
    DECLVAR OWNR x: I64 = 5;
    DECLVAR OWNR y: I64 = 3;
    
    // Fully qualified procedure call
    call math::add(x, y);
    call math::multiply(x, y);
}
```

### Explicit Scope Blocks (V3.0 STKFRAME)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
STKFRAME outer {
    DECLVAR OWNR x: I64 = 10;
    
    STKFRAME inner {
        DECLVAR OWNR y: I64 = 20;
        DECLVAR OWNR sum: I64 = x + y;
        EMIT sum;
    }
    
    // y is no longer accessible here
}
```

**V2.0 Backward Compatible:**
```lppo
scope outer {
    let own x: Int64 = 10;
    
    scope inner {
        let own y: Int64 = 20;
        let own sum: Int64 = x + y;
        out sum;
    }
}
```

### Unsafe Operations (V3.0 UNSAFEBLK)

**V3.0 Ultra-Low-Level Syntax:**
```lppo
UNSAFEBLK {
    // Low-level operations that bypass safety checks
    DECLVAR OWNR ptr: I64 = 0x1000;
    DECLVAR OWNR value: I64 = MEMDEREF TYPECAST<Ref<I64>>(ptr);
}
```

**V2.0 Backward Compatible:**
```lppo
unsafe {
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

## Getting Started (Warning: EXTREME Complexity)

> **⚠️ EXTREME LEARNING CURVE WARNING:** LPPO v3.0 is **INTENTIONALLY HOSTILE** to beginners. The language requires:
> - Understanding cryptic assembly-like mnemonics (PROCDEF, DECLVAR, CJMP, MEMXFER)
> - Explicit machine-level type annotations (I64, BOOL8, STRPTR, ARRPTR)
> - Manual memory management with low-level operations (HEAPALLOC, MEMXFER, MEMCPY)
> - Ultra-verbose procedure signatures with PUBEXP and PROCDEF
> - Deep understanding of stack frames (STKFRAME), jump instructions, and memory segments
> - No convenience features, no type inference, maximum cryptic complexity
> - Understanding of lifetimes, borrowing, and memory transfer semantics
> 
> This is **ACTIVELY HOSTILE** to casual users. It is designed to maximize cryptic difficulty and eliminate readability.

### Prerequisites

- Rust toolchain (1.80+ recommended, 2021 edition)
- **Expert-level** understanding of:
  - Assembly language and machine code concepts
  - Type systems and type theory
  - Manual memory management and memory layouts
  - Stack frames and calling conventions
  - Ownership and borrowing semantics
  - Lifetime annotations and reference tracking
  - Jump instructions and control flow at machine level
- Extreme patience for cryptic, assembly-like syntax
- Willingness to decipher mnemonics and abbreviations

### Build

```bash
cargo build --release
```

### Quick Start

See our **[Tutorial](docs/TUTORIAL.md)** for a step-by-step learning guide!

### Run Examples

#### V3.0 Ultra-Low-Level Cryptic Examples

```bash
# V3.0 hello world (ultra-cryptic PROCDEF, DECLVAR, EMIT syntax)
cargo run -- examples/hello_v3.lppo

# V3.0 comprehensive feature demo (maximum cryptic complexity)
cargo run -- examples/comprehensive_v3.lppo
```

#### V2.0 Backward Compatible Examples (Still Work!)

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
