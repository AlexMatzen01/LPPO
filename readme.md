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

### Partially Implemented

⚠️ **Advanced Features** (syntax exists but may be incomplete)
- Module system (`module`, `use`)
- Pattern matching (`match`)
- Closures and lambdas
- Move semantics
- Some ownership tracking

### Not Implemented

❌ **Features in Documentation But Not in Code**
- Async/await
- SIMD operations
- Standard library collections
- Effect system
- JIT compilation
- GPU computing
- Memory pools
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

**Current Version**: 5.2 (documentation cleaned up)
**Status**: Educational prototype
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
