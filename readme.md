# LPPO - A Learning-Focused Programming Language

## Overview

LPPO (Legible Programming Paradigm Optimizer) is an educational programming language designed to teach systems programming concepts with explicit, readable syntax. It's implemented as an interpreter in Rust.

## What LPPO Actually Is

LPPO is a **small, educational language** that demonstrates:
- Explicit type annotations (no type inference)
- Manual ownership tracking
- Clear, readable keywords
- Basic procedural programming

**LPPO is NOT:**
- A production-ready language
- Feature-complete or industrial-strength  
- Comparable to Rust, C++, or other established languages

## Core Features

### Actually Implemented

✅ **Basic Data Types**
- `Int64` - 64-bit signed integers
- `Bool` - Boolean values
- `Str` - String literals
- `Arr<T>` - Arrays

✅ **Control Flow**
- `if`/`elif`/`else` - Conditionals
- `while` - While loops
- `for...in` - For loops
- `loop` - Infinite loops with `break`/`continue`

✅ **Procedures**
- Function definitions with `proc`
- Explicit parameter and return types
- Public/private visibility with `pub`

✅ **Type System**
- Mandatory type annotations
- Basic type checking
- Ownership qualifiers (`own`, `mut`, `ref`)

✅ **Basic Operations**
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `and`, `or`, `not`
- Output: `out` statement
- Input: `input var: Type [with prompt];` - reads a line from stdin and parses it into the given type (`Int64`, `Bool`, `Str`, `Arr<T>`)

✅ **Random numbers**
- `random(min, max)` - pseudo-random integer in the inclusive range `[min, max]`

✅ **JSON support**
- `json_parse(str)` - parse a JSON string into an LPPO value
- `json_stringify(val)` - serialize an LPPO value to a JSON string

✅ **Terminal commands**
- `system(cmd)` - run a shell command via `sh -c` and return its output

✅ **ANSI colors**
- `color(text, color)` - wrap text in ANSI color codes. Supported colors: `reset`, `bold`, `underline`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `black`, and `bright_*` variants

✅ **File I/O** (New in v5.3)
- `read_file(path)` - Read file contents as string
- `write_file(path, content)` - Write content to file
- `append_file(path, content)` - Append content to file
- `file_exists(path)` - Check if file exists (returns Bool)
- `delete_file(path)` - Delete a file

✅ **String Utilities** (New in v5.3)
- `str_concat(s1, s2)` - Concatenate two strings
- `int_to_str(num)` - Convert integer to string
- `str_to_int(str)` - Convert string to integer
- `str_len(str)` - Get string length
- `str_upper(str)` - Convert to uppercase
- `str_lower(str)` - Convert to lowercase
- `str_trim(str)` - Trim whitespace
- `str_split(str, delim)` - Split string into array

✅ **Math Utilities** (New in v5.3)
- `abs(num)` - Absolute value
- `min(a, b)` - Minimum of two numbers
- `max(a, b)` - Maximum of two numbers
- `pow(base, exp)` - Exponentiation

### File I/O Example

```lppo
pub proc main() -> Void {
    let own path: Str = "example.txt";
    let own content: Str = "Hello, World!";
    
    let own write_result: Bool = write_file(path, content);
    let own file_content: Str = read_file(path);
    out "File contents:", file_content;
    
    let own exists: Bool = file_exists(path);
    if exists {
        let own delete_result: Bool = delete_file(path);
    }
}
```

### String Utilities Example

```lppo
pub proc main() -> Void {
    let own name: Str = "Alice";
    let own age: Int64 = 30;
    
    // Concatenate strings
    let own greeting: Str = str_concat("Hello, ", name);
    
    // Convert and build message
    let own age_str: Str = int_to_str(age);
    let own message: Str = str_concat(name, str_concat(" is ", str_concat(age_str, " years old")));
    out message;
    
    // Split string
    let own csv: Str = "apple,banana,orange";
    let own fruits: Arr<Str> = str_split(csv, ",");
    for fruit: Str in fruits {
        out fruit;
    }
}
```

### Input Example

```lppo
pub proc main() -> Void {
    input name: Str with "Enter your name:";
    out "Hello,", name;

    input age: Int64 with "Enter your age:";
    out "You are", age, "years old.";

    input active: Bool with "Are you active? (true/false):";
    out "Active:", active;
}
```

The `input` statement reads one line from stdin, trims whitespace, and parses it according to the declared type:
- `Int64` - parses an integer
- `Bool` - accepts `true/false`, `yes/no`, `y/n`, `1/0`
- `Str` - stores the raw line as a string
- `Arr<T>` - parses `[1, 2, 3]` or comma-separated values into an array

### Pause / Timers

LPPO includes a `pause` statement useful for simple timers and input-with-timeout patterns.

- Synchronous pause (blocks the current procedure until the timeout expires):

```lppo
// sleep for 1000 ms
pause { timeout 1000; }
```

- Asynchronous pause (schedules the remaining operations to run after timeout, returns immediately):

```lppo
// schedule remaining operations after 1s, return immediately
pause { async; timeout 1000; }
```

Notes:
- The `async` form runs the remaining operations in a background worker using a snapshot of the current environment (cloned variables and procedures).
- By default the interpreter waits for background continuations to finish before exiting so outputs from async continuations are not lost. To change to fire-and-forget behavior you can modify the runtime to not join background handles.

### Partially Implemented

⚠️ **Advanced Features** (syntax exists but may be incomplete)
- Module system (`module`, `use`)
- Pattern matching (`match`)
- Closures and lambdas
- Move semantics
- Some ownership tracking

### Not Implemented

❌ **Features in Documentation But Not in Code**
- Async/await (separate from pause)
- SIMD operations
- Standard library collections (HashMap, Vec, etc.)
- Effect system
- JIT compilation
- GPU computing
- Memory pools
- Networking (TCP/UDP)
- And many others claimed in older docs

## Quick Start

### Hello World

```lppo
pub proc main() -> Void {
    let own message: Str = "Hello, World!";
    out message;
}
```

### Basic Arithmetic

```lppo
pub proc main() -> Void {
    let own x: Int64 = 10;
    let own y: Int64 = 20;
    let own sum: Int64 = x + y;
    out "Sum:", sum;
}
```

### Arrays and Loops

```lppo
pub proc main() -> Void {
    let own numbers: Arr<Int64> = [1, 2, 3, 4, 5];
    
    for n: Int64 in numbers {
        out "Number:", n;
    }
}
```

### Conditionals

```lppo
pub proc main() -> Void {
    let own x: Int64 = 42;
    
    if x > 50 {
        out "Large";
    } elif x > 25 {
        out "Medium";
    } else {
        out "Small";
    }
}
```

## Building and Running

```bash
# Build the interpreter
cargo build --release

# Run a program
cargo run -- your_program.lppo

# Compile to a standalone runnable native executable (ELF on Linux, .exe on Windows)
cargo run --release -- --compile out your_program.lppo   # shorthand for --compile-out
./out

# Compile to a true bare metal executable (x86_64-unknown-none, no OS)
cargo run --release -- --compile-bare-metal kernel.bin your_program.lppo

# Run tests
cargo test
```

## Design Philosophy

LPPO emphasizes:

1. **Explicitness** - No hidden behavior or type inference
2. **Readability** - Clear keywords over cryptic abbreviations
3. **Education** - Teaching systems concepts, not production use
4. **Simplicity** - Limited feature set, well-implemented

## Language Spec

### Keywords

- **Declarations**: `proc`, `let`, `pub`, `module`, `use`
- **Types**: `Int64`, `Bool`, `Str`, `Arr`, `Void`
- **Qualifiers**: `own`, `mut`, `ref`, `borrow`
- **Control Flow**: `if`, `elif`, `else`, `while`, `for`, `in`, `loop`, `break`, `continue`
- **Operations**: `out`, `call`, `return`, `move`, `copy`

### Syntax Rules

- All variables require explicit type annotations
- All procedures require parameter types and return types
- All ownership must be explicit with qualifiers
- No type inference - every type must be stated

## Known Limitations

- No standard library (minimal built-ins only)
- Limited error messages and debugging info
- Performance not optimized
- Many advanced features incomplete
- No IDE support or tooling

## Development Status

**Current Version**: 5.3 (file I/O system added)
**Status**: Educational prototype with growing practical features
**Stability**: Experimental - APIs may change

## Documentation

- **[FEATURES.md](FEATURES.md)** - Complete list of implemented features
- **[REBUILD_NOTES.md](REBUILD_NOTES.md)** - Details about the rebuild process
- **[SUMMARY.md](SUMMARY.md)** - Quick overview of changes
- **[BUGS.md](BUGS.md)** - Known issues and limitations

## Contributing

This is an educational project. Contributions welcome for:
- Bug fixes
- Better error messages
- Documentation improvements
- Test coverage
- Example programs

See [BUGS.md](BUGS.md) for known issues.

## License

Open source - free to use, modify, and distribute.

## Acknowledgments

LPPO is a learning project exploring language design concepts. It's inspired by Rust's ownership system and modern language design, adapted for educational purposes.
