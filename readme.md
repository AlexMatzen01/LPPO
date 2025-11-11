# LPPO - Lightweight Procedural & Parallel Output

LPPO is a feature-rich programming language that combines procedural structure with parallel execution, comprehensive control flow, dynamic arrays, and expressive operators. Written in Rust, LPPO compiles source code into an efficient instruction graph for execution.

> **Status:** Feature-complete v1.0. Supports variables, expressions, control flow (if/elif/else, while/for/loop), arrays with methods, string operations, arithmetic and bitwise operators, procedure parameters, parallel execution, and more.

## Why LPPO?

- **Expressive:** Full-featured language with loops, conditionals, arrays, and operators
- **Procedural:** Clean procedure-based design with parameters and arguments
- **Parallel:** Native `parallel { }` blocks for concurrent execution
- **Fast:** Compiles to optimized instruction graph, optional binary bundling

## Language Overview

### Quick Example

```lppo
proc main {
    let nums = [1, 2, 3, 4, 5];
    out "Processing array:", nums;
    
    for n in nums {
        if n % 2 == 0 {
            out n, "is even";
        } else {
            out n, "is odd";
        }
    }
    
    call greet("LPPO", 2024);
}

proc greet(name, year) {
    out "Hello", name, "!", "Year:", year;
}
```

### Data Types

- **Integers:** `42`, `-17`, `0`
- **Booleans:** `true`, `false`
- **Strings:** `"hello"`, `"world\n"`
- **Arrays:** `[1, 2, 3]`, `["a", "b", "c"]`

### Variables and Expressions

```lppo
let x = 10;
let y = x * 2 + 5;
let flag = x > y or y == 25;
let arr = [1, 2, 3];
```

### Operators

**Arithmetic:** `+`, `-`, `*`, `/`, `%` (modulo), `**` (power)
**Comparison:** `==`, `!=`, `<`, `>`, `<=`, `>=`
**Logical:** `and`, `or`, `not`
**Bitwise:** `&`, `|`, `^` (xor), `~` (not)
**Unary:** `-` (negation), `!` (logical not), `~` (bitwise not)

### Control Flow

#### If/Elif/Else
```lppo
if x > 10 {
    out "Large";
} elif x > 5 {
    out "Medium";
} else {
    out "Small";
}
```

#### While Loop
```lppo
let i = 0;
while i < 5 {
    out "Count:", i;
    let i = i + 1;
}
```

#### For Loop
```lppo
let items = [10, 20, 30];
for item in items {
    out "Item:", item;
}
```

#### Infinite Loop
```lppo
let count = 0;
loop {
    out "Iteration:", count;
    let count = count + 1;
    if count >= 3 {
        break;
    }
}
```

**Loop Control:** `break` (exit loop), `continue` (skip to next iteration)

### Arrays and Strings

#### Array Operations
```lppo
let arr = [1, 2, 3, 4, 5];
out arr[0];        // First element: 1
out arr[-1];       // Last element: 5
out arr.len();     // Length: 5

let combined = [1, 2] + [3, 4];  // [1, 2, 3, 4]
```

#### String Operations
```lppo
let msg = "Hello World";
out msg.len();      // 11
out msg.upper();    // HELLO WORLD
out msg.lower();    // hello world

let parts = "a,b,c".split(",");  // ["a", "b", "c"]
let text = "hello" + " " + "world";  // Concatenation
```

### Procedures

```lppo
proc calculate(a, b) {
    let sum = a + b;
    let product = a * b;
    out "Sum:", sum, "Product:", product;
}

proc main {
    call calculate(5, 3);
}
```

### Parallel Execution

```lppo
parallel {
    out "Task 1";
    out "Task 2";
    out "Task 3";
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

## Getting Started

### Prerequisites

- Rust toolchain (1.80+ recommended, 2021 edition)

### Build

```bash
cargo build --release
```

### Run Examples

```bash
# Basic hello world
cargo run -- examples/hello.lppo

# Comprehensive feature demo
cargo run -- examples/comprehensive.lppo

# Advanced loops with break/continue
cargo run -- examples/loops.lppo

# Array and operator demonstrations
cargo run -- examples/operators.lppo
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

## Language Features

### ✅ Implemented

- Variables with `let` bindings
- Data types: integers, booleans, strings, arrays
- Arithmetic operators: `+`, `-`, `*`, `/`, `%`, `**`
- Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical operators: `and`, `or`, `not`
- Bitwise operators: `&`, `|`, `^`, `~`
- Unary operators: `-`, `!`, `~`
- Control flow: `if`/`elif`/`else`
- Loops: `while`, `for`-in, `loop` (infinite)
- Loop control: `break`, `continue`
- Procedures with parameters and arguments
- Array indexing (positive and negative indices)
- Array methods: `len()`, concatenation with `+`
- String methods: `len()`, `upper()`, `lower()`, `split()`
- String concatenation
- Parallel execution blocks
- Line comments (`//`) and block comments (`/* */`)
- Binary bundling for distribution

### 🔮 Future Enhancements

- Return values from procedures
- Structs and custom types
- First-class functions
- Import/module system
- More collection types (maps, sets)
- File I/O operations
- Error handling constructs
- Memory safety analysis
- Optimizing compiler passes
- Language server protocol support
- Standard library

## Contributing

Contributions are welcome! Areas to explore:

- Additional operators or built-in functions
- Performance optimizations
- More comprehensive error messages
- Extended standard library
- Tooling (formatter, linter, debugger)

## License

This project is open source. Feel free to use, modify, and distribute.
