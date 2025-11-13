# LPPO Language Specification v2.0

## Table of Contents

1. [Introduction](#introduction)
2. [Lexical Structure](#lexical-structure)
3. [Type System](#type-system)
4. [Memory Management](#memory-management)
5. [Expressions](#expressions)
6. [Statements](#statements)
7. [Procedures](#procedures)
8. [Modules and Namespaces](#modules-and-namespaces)
9. [Advanced Features](#advanced-features)
10. [Error Handling](#error-handling)

## Introduction

LPPO (Low-level Procedural Programming with Ownership) is a deliberately complex, verbose programming language designed to maximize explicitness and minimize convenience. Every aspect of the language requires manual specification, with no type inference or syntactic sugar.

### Design Philosophy

- **Absolute Explicitness**: Every variable, parameter, and return type must be explicitly annotated
- **Manual Memory Management**: Explicit ownership tracking and lifetime annotations
- **Maximum Verbosity**: No shortcuts or convenience features
- **Zero Inference**: The compiler makes no assumptions about types or ownership

### Hello World Example

```lppo
pub proc main() -> Void {
    let own msg: Str = "Hello, World!";
    out msg;
}
```

## Lexical Structure

### Keywords

LPPO has the following reserved keywords:

**Control Flow**: `if`, `elif`, `else`, `while`, `for`, `in`, `loop`, `break`, `continue`, `return`, `match`

**Declarations**: `proc`, `let`, `struct`, `enum`, `trait`, `impl`, `module`, `use`

**Visibility**: `pub`

**Memory**: `alloc`, `dealloc`, `move`, `copy`, `ref`, `deref`, `borrow`, `own`, `mut`, `heap`, `stack`, `sizeof`, `alignof`, `cast`

**Types**: `Int64`, `Bool`, `Str`, `Arr`, `Void`, `Option`, `Result`

**Operations**: `out`, `call`, `parallel`, `scope`, `unsafe`, `new`, `end`, `on`

**Boolean**: `and`, `or`, `not`

### Identifiers

Identifiers must:
- Start with a letter (a-z, A-Z) or underscore (_)
- Contain only letters, digits, or underscores
- Not be a reserved keyword

### Literals

**Integer Literals**: Decimal integers (e.g., `42`, `-10`, `0`)

**Boolean Literals**: `true`, `false`

**String Literals**: Enclosed in double quotes with escape sequences
```lppo
let own msg: Str = "Hello\nWorld";  // \n, \r, \t, \\, \"
```

**Array Literals**: Comma-separated values in square brackets
```lppo
let own nums: Arr<Int64> = [1, 2, 3, 4, 5];
let own words: Arr<Str> = ["hello", "world"];
```

### Comments

**Line Comments**: From `//` to end of line
```lppo
// This is a line comment
```

**Block Comments**: Between `/*` and `*/`
```lppo
/*
 * This is a
 * block comment
 */
```

### Operators

**Arithmetic**: `+`, `-`, `*`, `/`, `%` (modulo), `**` (power)

**Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`

**Logical**: `and`, `or`, `not`

**Bitwise**: `&`, `|`, `^` (xor), `~` (bitwise not)

**Other**: `=` (assignment), `:` (type annotation), `::` (namespace), `->` (return type), `.` (member access)

## Type System

### Primitive Types

**Int64**: 64-bit signed integer
```lppo
let own x: Int64 = 42;
let own y: Int64 = -100;
```

**Bool**: Boolean value (true or false)
```lppo
let own flag: Bool = true;
let own result: Bool = false;
```

**Str**: String type (immutable)
```lppo
let own msg: Str = "Hello";
```

**Void**: No return value for procedures
```lppo
pub proc do_nothing() -> Void {
    // No return statement needed
}
```

### Compound Types

**Arr<T>**: Array of elements of type T
```lppo
let own numbers: Arr<Int64> = [1, 2, 3];
let own words: Arr<Str> = ["one", "two"];
```

**Custom Types**: User-defined structs and enums (planned)
```lppo
// Planned feature
struct Point {
    pub x: Int64,
    pub y: Int64,
}
```

**Option<T>**: Optional value (planned)
```lppo
// Planned feature
let own maybe: Option<Int64> = Some(42);
```

**Result<T, E>**: Result type for error handling (planned)
```lppo
// Planned feature
let own result: Result<Int64, Str> = Ok(42);
```

### Type Annotations

**Every variable** must have an explicit type annotation:

```lppo
// WRONG - No type annotation:
// let own x = 10;  // ERROR

// CORRECT - Explicit type:
let own x: Int64 = 10;
```

**Intermediate values** also need explicit types:

```lppo
// WRONG - Cannot infer:
// let own y = x * 2 + 5;  // ERROR

// CORRECT - Explicit intermediates:
let own two: Int64 = 2;
let own five: Int64 = 5;
let own mult: Int64 = x * two;
let own y: Int64 = mult + five;
```

### References and Ownership Types

**Ref<T>**: Immutable reference with optional lifetime
```lppo
let ref x_ref: Ref<Int64> = ref x;
```

**Own<T>**: Owned value (default)
```lppo
let own value: Own<Int64> = 42;
```

**Mut<T>**: Mutable reference
```lppo
let mut own counter: Mut<Int64> = 0;
```

## Memory Management

LPPO requires explicit memory management with ownership tracking.

### Ownership Qualifiers

**own**: Owned value (most common)
```lppo
let own x: Int64 = 42;
```

**mut**: Mutable variable
```lppo
let mut own counter: Int64 = 0;
// Can be reassigned with move
let own new_val: Int64 = counter + 1;
move new_val -> counter;
```

**ref**: Immutable reference
```lppo
let own x: Int64 = 42;
let ref y: Ref<Int64> = ref x;
```

**borrow**: Borrowed value with lifetime
```lppo
let borrow z: Int64 = borrow<'a> x;
```

### Manual Allocation

**Heap Allocation**:
```lppo
alloc<Arr<Int64>> my_array on heap [100];
// ... use my_array ...
dealloc my_array;
```

**Stack Allocation**:
```lppo
alloc<Int64> temp on stack [1];
```

### Move Semantics

Transfer ownership from one variable to another:
```lppo
let own source: Int64 = 10;
let own target: Int64 = 0;
move source -> target;
// source is now invalid
```

### Copy Semantics

Duplicate a value (requires Copy trait):
```lppo
let own original: Int64 = 20;
let own duplicate: Int64 = 0;
copy original -> duplicate;
// Both original and duplicate are valid
```

### Lifetime Annotations

Procedures can specify explicit lifetime parameters:
```lppo
pub proc process<'a, 'b>(x: Ref<Int64> 'a, y: Ref<Int64> 'b) -> Void {
    let own result: Int64 = deref x + deref y;
    out result;
}
```

### Type Introspection

**sizeof**: Query the size of a type
```lppo
let own int_size: Int64 = sizeof<Int64>;
let own arr_size: Int64 = sizeof<Arr<Int64>>;
```

**alignof**: Query the alignment of a type
```lppo
let own int_align: Int64 = alignof<Int64>;
```

**cast**: Explicit type casting
```lppo
let own x: Int64 = 42;
let own as_bool: Bool = cast<Bool>(x);
```

## Expressions

### Literal Expressions

```lppo
let own num: Int64 = 42;
let own flag: Bool = true;
let own text: Str = "hello";
let own arr: Arr<Int64> = [1, 2, 3];
```

### Arithmetic Expressions

```lppo
let own a: Int64 = 10;
let own b: Int64 = 3;

let own sum: Int64 = a + b;        // Addition
let own diff: Int64 = a - b;       // Subtraction
let own prod: Int64 = a * b;       // Multiplication
let own quot: Int64 = a / b;       // Division
let own rem: Int64 = a % b;        // Modulo
let own pow: Int64 = a ** b;       // Power
```

### Comparison Expressions

```lppo
let own x: Int64 = 10;
let own y: Int64 = 20;

let own eq: Bool = x == y;         // Equal
let own ne: Bool = x != y;         // Not equal
let own lt: Bool = x < y;          // Less than
let own le: Bool = x <= y;         // Less or equal
let own gt: Bool = x > y;          // Greater than
let own ge: Bool = x >= y;         // Greater or equal
```

### Logical Expressions

```lppo
let own a: Bool = true;
let own b: Bool = false;

let own and_result: Bool = a and b;   // Logical AND
let own or_result: Bool = a or b;     // Logical OR
let own not_result: Bool = not a;     // Logical NOT
```

### Bitwise Expressions

```lppo
let own x: Int64 = 12;   // 0b1100
let own y: Int64 = 10;   // 0b1010

let own and_bits: Int64 = x & y;      // 0b1000 = 8
let own or_bits: Int64 = x | y;       // 0b1110 = 14
let own xor_bits: Int64 = x ^ y;      // 0b0110 = 6
let own not_bits: Int64 = ~x;         // Bitwise NOT
```

### Array Expressions

**Indexing**: Access array elements
```lppo
let own arr: Arr<Int64> = [1, 2, 3, 4, 5];

// Positive indexing (0-based)
let own zero: Int64 = 0;
let own first: Int64 = arr[zero];     // 1

// Negative indexing (from end)
let own neg_one: Int64 = -1;
let own last: Int64 = arr[neg_one];   // 5
```

**Length**: Get array length
```lppo
let own len: Int64 = arr.len();
```

**Concatenation**: Combine arrays
```lppo
let own a1: Arr<Int64> = [1, 2];
let own a2: Arr<Int64> = [3, 4];
let own combined: Arr<Int64> = a1 + a2;  // [1, 2, 3, 4]
```

### String Expressions

**Length**: Get string length
```lppo
let own msg: Str = "Hello";
let own len: Int64 = msg.len();  // 5
```

**Case conversion**:
```lppo
let own upper: Str = msg.upper();  // "HELLO"
let own lower: Str = msg.lower();  // "hello"
```

**Split**: Split string into array
```lppo
let own text: Str = "a,b,c";
let own sep: Str = ",";
let own parts: Arr<Str> = text.split(sep);  // ["a", "b", "c"]
```

**Concatenation**:
```lppo
let own hello: Str = "hello";
let own space: Str = " ";
let own world: Str = "world";
let own temp: Str = hello + space;
let own result: Str = temp + world;  // "hello world"
```

## Statements

### Variable Declaration

```lppo
let own variable_name: Type = expression;
```

**Ownership qualifiers**: `own`, `mut`, `ref`, `borrow`

Examples:
```lppo
let own x: Int64 = 42;
let mut own counter: Int64 = 0;
let ref y: Ref<Int64> = ref x;
```

### Output Statement

Emit values to standard output:
```lppo
out expr1, expr2, expr3;
```

Examples:
```lppo
let own x: Int64 = 42;
let own msg: Str = "Value:";
out msg, x;
```

### Input Statement

Read user input from standard input with explicit type specification:

**Syntax:**
```lppo
input variable_name: Type;                    // Basic input
input variable_name: Type with prompt_expr;   // Input with prompt
```

**Supported Types:** `Int64`, `Bool`, `Str`, `Arr<T>`

**Examples:**

String input:
```lppo
input name: Str with "Enter your name:";
out "Hello,", name;
```

Integer input:
```lppo
input age: Int64 with "Enter your age:";
let own double: Int64 = age + age;
out "Double:", double;
```

Boolean input (accepts: true/false, yes/no, y/n, 1/0):
```lppo
input flag: Bool with "Continue? (yes/no):";
if flag {
    out "Continuing...";
} else {
    out "Stopping...";
}
```

Array input (comma-separated or bracket notation):
```lppo
// Accepts: "1,2,3" or "[1, 2, 3]"
input numbers: Arr<Int64> with "Enter numbers:";
out "You entered:", numbers;
```

**Type Validation:**
- Input is automatically parsed according to the specified type
- Type mismatches produce descriptive error messages
- Invalid input terminates the program with an error

### If Statement

```lppo
let own condition: Bool = x > 10;

if condition {
    // then branch
} elif other_condition {
    // else-if branch
} else {
    // else branch
}
```

**Note**: Conditions must be explicitly typed Bool variables, not inline expressions.

### While Loop

```lppo
let mut own i: Int64 = 0;
let own limit: Int64 = 5;

while i < limit {
    out i;
    let own i_next: Int64 = i + 1;
    move i_next -> i;
}
```

### For Loop

**Requires explicit type** for loop variable:
```lppo
let own items: Arr<Int64> = [10, 20, 30];

for item: Int64 in items {
    out item;
}
```

### Infinite Loop

```lppo
let mut own count: Int64 = 0;
let own max: Int64 = 3;

loop {
    out count;
    let own count_next: Int64 = count + 1;
    move count_next -> count;
    
    let own should_break: Bool = count >= max;
    if should_break {
        break;
    }
}
```

### Break and Continue

```lppo
break;      // Exit current loop
continue;   // Skip to next iteration
```

### Return Statement

```lppo
return;              // For Void procedures
return expression;   // For procedures with return value
```

### Procedure Call

```lppo
call procedure_name(arg1, arg2);
```

Examples:
```lppo
call greet("Alice", 25);
call math::add(5, 3);  // Qualified name
```

### Scope Block

Explicit scope management:
```lppo
scope block_name {
    let own x: Int64 = 10;
    // x is only valid within this scope
}
// x is no longer accessible
```

### Unsafe Block

For operations that bypass safety checks:
```lppo
unsafe {
    // Low-level operations
    let own ptr: Int64 = 0x1000;
    let own value: Int64 = deref cast<Ref<Int64>>(ptr);
}
```

### Parallel Block

Execute expressions in parallel:
```lppo
scope parallel_section {
    parallel {
        out "Task 1";
        out "Task 2";
        out "Task 3";
    }
}
```

## Procedures

### Procedure Definition

**All procedures require**:
1. Visibility modifier (`pub` or module-scoped)
2. Explicit parameter list (even if empty)
3. Type annotation for every parameter
4. Explicit return type annotation

```lppo
pub proc procedure_name(param1: Type1, param2: Type2) -> ReturnType {
    // body
}
```

### Examples

**No parameters, no return**:
```lppo
pub proc main() -> Void {
    let own msg: Str = "Hello!";
    out msg;
}
```

**With parameters**:
```lppo
pub proc greet(name: Str, age: Int64) -> Void {
    let own hello: Str = "Hello";
    out hello, name, "Age:", age;
}
```

**With return value** (planned):
```lppo
pub proc add(a: Int64, b: Int64) -> Int64 {
    let own result: Int64 = a + b;
    return result;
}
```

### Lifetime Parameters

Procedures can have explicit lifetime parameters:
```lppo
pub proc process<'a, 'b>(x: Ref<Int64> 'a, y: Ref<Int64> 'b) -> Void {
    // Lifetime tracking ensures references remain valid
}
```

### Visibility

- `pub`: Public procedure, accessible from outside module
- No modifier: Module-scoped, only accessible within module

## Modules and Namespaces

### Module Definition

```lppo
module module_name {
    pub proc public_function() -> Void {
        // accessible from outside
    }
    
    proc private_function() -> Void {
        // only accessible within module
    }
}
```

### Qualified Names

Access items from modules using `::`:
```lppo
call math::add(5, 3);
call utils::string::format(msg);
```

### Nested Modules

```lppo
module parent {
    module child {
        pub proc helper() -> Void {
            // ...
        }
    }
}

// Call with fully qualified path
call parent::child::helper();
```

## Advanced Features

### Structs (Planned)

```lppo
pub struct Point {
    pub x: Int64,
    pub y: Int64,
}

// Instantiation
let own p: Point = Point {
    x: 10,
    y: 20,
};

// Field access
let own x_val: Int64 = p.x;
```

### Enums (Planned)

```lppo
pub enum Option<T> {
    Some(T),
    None,
}

let own maybe: Option<Int64> = Some(42);
```

### Pattern Matching (Planned)

```lppo
match value {
    Pattern1 => {
        // body
    }
    Pattern2 if guard => {
        // body with guard
    }
    _ => {
        // default case
    }
}
```

### Traits (Planned)

```lppo
pub trait Display {
    pub proc show(self: Ref<Self>) -> Str;
}

impl Display for Point {
    pub proc show(self: Ref<Point>) -> Str {
        // implementation
    }
}
```

## Error Handling

### Error Types

LPPO provides detailed error messages with:
- Line and column numbers
- Source code context
- Helpful suggestions
- Severity levels (Error, Warning, Info, Fatal)

### Common Errors

**Type Errors**:
```
error: type error: expected Int64, found Str
  --> line 5, column 10
   |
5  | let own x: Int64 = "hello";
   |                    ^^^^^^^
   = help: use an Int64 literal like 42
```

**Unknown Variable**:
```
error: unknown variable 'foo'
  --> line 8, column 5
   |
8  | out foo;
   |     ^^^
   = help: check that the variable is declared before use
```

**Division by Zero**:
```
error: division by zero
  --> line 12, column 15
```

### Runtime Errors

Runtime errors include stack traces:
```
runtime error: index 5 out of bounds for array of length 3
  --> line 20, column 10

Stack trace:
  0 at procedure 'process_array'
  1 at procedure 'main'
```

## Best Practices

1. **Always use explicit types**: Never rely on type inference (it doesn't exist)

2. **Manage memory explicitly**: Use `alloc`/`dealloc` and track ownership

3. **Use qualified names**: For clarity, use module::procedure syntax

4. **Explicit intermediate variables**: Break complex expressions into steps

5. **Document lifetime requirements**: Use lifetime annotations for borrowed values

6. **Scope management**: Use explicit scope blocks for clarity

7. **Error handling**: Check for potential errors before they occur

## Language Comparison

### LPPO vs Rust

LPPO is inspired by Rust but removes all convenience:
- No type inference
- No pattern matching sugar
- Explicit everything
- Maximally verbose

### LPPO vs C

LPPO adds:
- Memory safety tracking
- Ownership system
- Lifetime annotations
- Module system

But requires:
- More verbosity
- More explicit type annotations
- More boilerplate

## Implementation Notes

LPPO v2.0 is implemented in Rust and consists of:

- **Lexer**: Tokenizes source code
- **Parser**: Builds abstract syntax tree
- **Compiler**: Lowers AST to runtime operations
- **Runtime**: Executes operations with interpreter
- **Error System**: Provides detailed diagnostic messages

The language supports:
- Compilation to standalone executables
- Debug mode for diagnostics
- Embedded program execution
