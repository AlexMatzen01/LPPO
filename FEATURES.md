# LPPO Feature List

This document honestly describes what LPPO actually implements.

## Lexer & Parser

### Tokens Recognized
- Keywords: proc, pub, let, if, elif, else, while, for, in, loop, break, continue, return, out, input, call
- Types: Int64, Bool, Str, Arr, Void, Option, Result
- Qualifiers: own, mut, ref, borrow
- Operators: +, -, *, /, %, ==, !=, <, >, <=, >=, and, or, not
- Punctuation: (, ), {, }, [, ], :, ::, ,, ;, =, ->
- Literals: integers, strings, booleans (true/false)

### Parsing Support
- Procedure definitions with parameters
- Variable declarations
- Control flow statements (if/elif/else, while, for, loop)
- Expressions with operator precedence
- Array literals and indexing
- Basic module syntax (parsed but not fully implemented)
- Pattern matching syntax (parsed but limited runtime support)

## Type System

### Implemented
- Basic type checking for binary operations
- Type annotations required on all variables and parameters
- Array type checking
- Return type validation
- Ownership state tracking (move/copy enforcement)
- **Compile-time borrow checking** (Rust-like): immutable/mutable references, aliasing rules, scope-based drop analysis
- **Compile-time memory safety**: heap vs stack allocation enforcement, automatic deallocation at scope exit, no garbage collector needed

### Limitations
- No type inference
- Borrow checking does not cross procedure boundaries for arguments
- Limited generic support
- No trait system implementation

## Runtime

### Working Features
- Integer arithmetic (add, subtract, multiply, divide, modulo)
- Boolean logic (and, or, not)
- String literals
- Arrays (create, index, iterate)
- Procedure calls
- Variable scoping
- For loops over arrays
- While loops
- Conditionals (if/elif/else)
- Basic output (out statement)
- Basic input (input statement): reads from stdin and parses into Int64, Bool, Str, or Arr<T>
- Break/continue in loops
- **File I/O operations (v5.3)**:
  - `read_file(path)` - read file contents
  - `write_file(path, content)` - write to file
  - `append_file(path, content)` - append to file
  - `file_exists(path)` - check file existence
  - `delete_file(path)` - delete file
- **String utilities (v5.3)**:
  - `str_concat(s1, s2)` - concatenate strings
  - `int_to_str(num)` - integer to string conversion
  - `str_to_int(str)` - string to integer parsing
  - `str_len(str)` - string length
  - `str_upper(str)`, `str_lower(str)` - case conversion
  - `str_trim(str)` - whitespace trimming
  - `str_split(str, delim)` - string splitting
- **Math utilities (v5.3)**:
  - `abs(num)` - absolute value
  - `min(a, b)`, `max(a, b)` - min/max functions
  - `pow(base, exp)` - exponentiation
- **Random numbers**:
  - `random(min, max)` - pseudo-random integer in [min, max]
- **JSON support**:
  - `json_parse(str)` - parse a JSON string into an LPPO value
  - `json_stringify(val)` - serialize an LPPO value to a JSON string
- **Terminal commands**:
  - `system(cmd)` - run a shell command and return its output
- **ANSI colors**:
  - `color(text, color)` - wrap text in ANSI color codes (red, green, blue, yellow, magenta, cyan, white, black, bold, underline, reset, and bright variants)

### Limited Features
- Pattern matching (basic support only)
- Closures (parsed but limited execution)
- Ranges (basic support)
- Module system (parsed but not executed)
- Traits (parsed but not enforced)
- Generic implementations (parsed but limited typechecking)

### Recently Implemented
- Compile-time memory safety (Rust-like borrow checker): ownership tracking, borrow checking, scope-based auto-deallocation
- Manual memory allocation/deallocation with heap vs stack enforcement (`alloc <T> var on heap/stack [size]`, `dealloc var`)
- Scope-based automatic deallocation for heap variables (no garbage collector needed)
- `unsafe` blocks to bypass borrow checker
- References (`ref`, `ref mut`) and dereferencing (`deref`)
- Move/copy semantics with compile-time enforcement
- Standard library collections (Vec/Arr, HashMap/Map)
- Async/await (`async`, `await`, `spawn`, `join`)
- SIMD operations (`simd_add`, `simd_sub`, `simd_mul`, `simd_div`, `simd_splat`, `simd_extract`, `simd_insert`, `simd_shuffle`, `simd_reduce`)
- Direct memory addressing and pointers (`addr_of`, `ptr_read`, `ptr_write`, `ptr_offset`)
- Volatile memory operations (`volatile_read`, `volatile_write`)
- Inline assembly (`inline_asm`)
- Bitwise manipulation (`&`, `|`, `^`, `~`)
- Foreign function interface (via native packages / libloading)

### Not Implemented
- Networking (TCP/UDP)
- Macros (token only, no expansion)
- Effect system (parsed but not handled at runtime)
- JIT compilation
- GPU computing

## Error Handling

### Implemented
- Lexer errors with position
- Parser errors with position
- Type errors
- Runtime errors (division by zero, index out of bounds)
- Basic error messages

### Could Be Better
- Error recovery (fails fast)
- Suggestion system (partially implemented)
- Stack traces (basic only)

## Compiler/Interpreter

### Architecture
- Lexer: Tokenizes source code
- Parser: Builds AST
- Type Checker: Validates types and ownership
- Compiler: Converts AST to executable form
- Runtime: Executes compiled procedures

### Modes
- Interpret mode: Run .lppo files directly
- Compile mode: Create standalone executables (embeds bytecode)

## Testing

### Test Coverage
- 18 unit tests
- 3 integration tests
- Basic language feature tests
- Type checking tests
- Ownership tracking tests (basic)

### Not Tested
- Many edge cases
- Complex ownership scenarios
- Error message quality
- Performance characteristics

## Documentation

### Available
- This feature list
- README with examples
- BUGS.md with known issues
- Basic inline comments

### Missing
- API documentation
- Tutorial beyond examples
- Language specification
- Best practices guide

## Performance

### Characteristics
- Interpreted (not compiled to native code)
- No optimization passes
- Simple recursive evaluation
- Stack-based execution

### Known Issues
- Large error types cause compiler warnings (performance trade-off for better errors)
- No benchmarks
- No performance testing

## Summary

LPPO is a **basic educational interpreter** that:
- ✅ Parses a small language with explicit types
- ✅ Type checks basic operations
- ✅ Executes simple procedural programs
- ✅ Has readable syntax
- ❌ Is not production-ready
- ❌ Does not have most "advanced" features claimed in old docs
- ❌ Is not optimized for performance

For actual feature requests or to understand what works, see the test files in `tests/` directory.
