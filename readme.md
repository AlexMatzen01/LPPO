# LPPO - A Learning-Focused Programming Language

**Version:** 0.1.3-alpha.8 (alpha)

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

### Build options

The following options are accepted by `lppo project build`, `--compile-out`,
`--compile`, and `--compile-bare-metal` (where applicable):

| Option | Description |
|--------|-------------|
| `--release` | Build a release-mode bundle |
| `--entry <file>` | Entry source file (default: manifest `build.entry`) |
| `--output <path>` | Output executable path (default: manifest `build.output`) |
| `--strip` | Strip unreachable/dead procedures from the bytecode payload |
| `--obfuscate` | XOR-obfuscate the bytecode payload so plaintext strings are not stored verbatim in the output binary |
| `--linker <path>` | Linker to use (applied to bare-metal/native builds via `RUSTFLAGS`) |
| `--link-arg <arg>` | Additional argument forwarded to the linker (repeatable) |

`--strip` and `--obfuscate` can also be set as defaults in `lppo.toml` under
`[build]` (`strip = true`, `obfuscate = true`, `link_args = [...]`); CLI flags
override the manifest.

Examples:

```bash
# Compile a project with stripping and obfuscation
lppo project build --strip --obfuscate --output my_app

# Compile a single source file with obfuscation
lppo --compile-out my_app --obfuscate program.lppo

# Bare-metal build with a custom linker and linker args
lppo --compile-bare-metal kernel.bin --linker ld.lld --link-arg --strip-all program.lppo
```

## Standard library

### Collections

LPPO provides `Vec<T>` (backed by `Arr<T>`) and `Map<K, V>` (backed by `Object`) collections.
Element/value types are checked statically but manipulated dynamically at runtime.

```lppo
let own nums: Arr<Int64> = vec_new();
let own nums1 = vec_push(nums, 10);
let own nums2 = vec_push(nums1, 20);
out "len:", vec_len(nums2);
out "get[0]:", vec_get(nums2, 0);
let own nums3 = vec_set(nums2, 0, 99);
out "get[0] after set:", vec_get(nums3, 0);
let own v = vec_pop(nums3);

let own m: Map<Str, Int64> = map_new();
let own m1 = map_insert(m, "a", 1);
let own m2 = map_insert(m1, "b", 2);
out "map get a:", map_get(m2, "a");
out "map len:", map_len(m2);
out "map keys:", map_keys(m2);
let own m3 = map_remove(m2, "a");
```

Available built-ins: `vec_new`, `vec_push`, `vec_pop`, `vec_get`, `vec_set`, `vec_len`,
`map_new`, `map_insert`, `map_get`, `map_remove`, `map_len`, `map_keys`.

### Memory pools

A simple global byte pool is exposed via `pool_alloc(size)`, `pool_free(id)`, and
`pool_used()`.

```lppo
let own pid = pool_alloc(256);
out "pool used:", pool_used();
out "freed:", pool_free(pid);
out "pool used after free:", pool_used();
```

### Async/await

`async { expr }` evaluates `expr` in a background task and returns a `Future`. `await`
blocks until the future completes and yields its value. This is separate from `pause`
(user input waiting).

```lppo
let own f: Future = async { 42 };
out "launched";
let own v: Int64 = await f;
out "awaited:", v;
```

### Threading

`spawn { expr }` evaluates `expr` in a new OS thread and returns a `JoinHandle`. `await`
on a join handle blocks until the thread finishes.

```lppo
let own h: JoinHandle = spawn { 99 };
out "spawned";
let own v: Int64 = await h;
out "joined:", v;
```

`spawn { <statements> };` is also available as a fire-and-forget statement.

### Math

`eval(s)` parses and evaluates an integer math expression string with `+ - * / ^`
and parentheses (standard precedence). `sqrt(n)` returns the integer square root
(floor). `either(a, b)` returns `[a+b, a-b]` — handy for +/- pairs like the
quadratic formula.

```lppo
out "eval 2+3*4:", eval("2+3*4");   # 14
out "eval (2+3)*4:", eval("(2+3)*4"); # 20
out "eval 2^10:", eval("2^10");     # 1024
out "sqrt 16:", sqrt(16);           # 4
let own roots: Arr<Int64> = either(5, 1);
out "either 5, 1:", roots;          # [6, 4]
```

## Examples

See `examples/` for runnable programs demonstrating each feature:

- `examples/collections.lppo` - Vec and Map collections
- `examples/memory_pool.lppo` - pool_alloc / pool_free / pool_used
- `examples/async_await.lppo` - async blocks and await
- `examples/threading.lppo` - spawn, join, and fire-and-forget threads
- `examples/math.lppo` - eval, sqrt, either (including quadratic formula)

Run any example with:

```bash
cargo run -- examples/collections.lppo
```

## Package Management (`lppo pkg`)

LPPO includes an integrated Rust/Cargo package manager. The package library index
lives at `~/.LPPO/packages.json` and installed packages are tracked separately at
`~/.LPPO/installed.json`.

### Package library location

- `~/.LPPO/packages.json` — package library index (catalog of available packages).
- `~/.LPPO/installed.json` — installed package database (what is installed locally).
- `~/.LPPO/packages/<name>/` — retrieved package source checkouts.
- `~/.LPPO/cache/` — downloads, git checkouts, and build artifacts.

LPPO automatically creates `~/.LPPO/` and `packages.json` on first use. All writes
are atomic (temp file + rename) to prevent corruption.

### Package format

Every LPPO package is a genuine Rust/Cargo project with a `Cargo.toml`. Package
metadata is described in `packages.json` using Serde JSON, including name, version,
description, authors, license, repository, source (Git branch/tag/commit/subdirectory
or local path), LPPO version compatibility, dependencies on other LPPO packages
(machine-readable), and Cargo build configuration.

### CLI commands

```bash
lppo pkg search <query>    # search the package library
lppo pkg install <pkg>     # resolve deps, build with Cargo, track install
lppo pkg remove <pkg>      # remove an installed package
lppo pkg update [pkg]      # update a package (or all)
lppo pkg list              # list installed packages
lppo pkg info <pkg>        # show package metadata
lppo pkg build <pkg>       # build a package without marking installed
lppo pkg publish <path>    # validate and publish a Cargo package to the library
lppo pkg install <git-url> # install directly from a Git repository URL
```

Example:

```bash
lppo pkg install LPPX2D
lppo pkg list
lppo pkg remove LPPX2D
```

### Dependency resolution

LPPO resolves dependencies between LPPO packages (declared in `packages.json`)
separately from Cargo's Rust crate resolution (declared in each package's
`Cargo.toml`). It builds a directed dependency graph, detects circular
dependencies, resolves semantic version constraints, and installs dependencies
before their dependents.

### Using an installed package in LPPO

Installed Rust packages expose an LPPO-facing API (`lppo_interface.json`). Once
installed, a package can be imported and its functions called from LPPO:

```lppo
use LPPX2D;

pub proc main() -> Void {
    x2d_window_create("demo", 800, 600);
    x2d_begin_frame(1);
    x2d_canvas_clear(0, 0, 0, 1);
    x2d_draw_rectangle(100, 100, 200, 150, 255, 0, 0);
    x2d_end_frame(1);
}
```

Calls are dispatched through the compiler's native integration boundary (FFI
against the package's compiled `cdylib`).

### Graphics extensions (LPPX)

LPPX (Legible Programming Platform eXtensions) is the official extension platform
for LPPO. The initial extensions, maintained in the separate `LPPX` Git
repository, are:

- **LPPX2D** — 2D graphics framework (`lppo-x2d`).
- **LPPX3D** — 3D graphics framework (`lppo-x3d`).
- **LPPXCommon** — shared Rust crate (math, windowing, input, timing).

Both are independently installable, versioned, and built with Cargo. See the
`LPPX` repository for details.

## Version Tracking (`version-cli`)

LPPO ships with a dedicated version tracking CLI/TUI in `version-cli/`. It tracks
semantic versioning (major.minor.patch), an auto-incrementing build number, and a
release channel (alpha / beta / release). State is stored in `.version.json`.

Start state: **v1.0.1-beta.0**.

```bash
# Interactive TUI (recommended)
cargo run -p version-cli

# CLI
cargo run -p version-cli -- version          # print full version string
cargo run -p version-cli -- show             # show full state
cargo run -p version-cli -- set 1 2 3        # set major.minor.patch manually
cargo run -p version-cli -- bump-build       # auto-increment build number
cargo run -p version-cli -- channel release  # switch channel (alpha|beta|release)
cargo run -p version-cli -- reset-build      # reset build to 0
```

### Releasing

In the TUI, choose **Release & Push to GitHub**. This:

1. Saves `.version.json`
2. Updates `readme.md` (version line) and `CHANGELOG.md` (new release entry)
3. Commits with message `chore(release): <version> <commit message>`
4. Tags the commit `v<version>`
5. Pushes the branch and tags to `origin`
6. Creates a GitHub release via `gh release create` (prerelease for alpha/beta)

Requires `gh` to be authenticated (`gh auth login`) with `repo` scope.


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