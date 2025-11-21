# LPPO Feature List

This document honestly describes what LPPO actually implements.

## Lexer & Parser

### Tokens Recognized
- Keywords: proc, pub, let, if, elif, else, while, for, in, loop, break, continue, return, out, call
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
- Ownership state tracking (basic)

### Limitations
- No type inference
- Ownership tracking incomplete (borrow checking not fully implemented)
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

### Limited Features
- Pattern matching (basic support only)
- Closures (parsed but limited execution)
- Ranges (basic support)
- Move semantics (tracked but not enforced)
- Module system (parsed but not executed)

### Not Implemented
- Standard library collections (Vec, HashMap, etc.)
- Networking (TCP/UDP)
- Async/await (separate from pause statement)
- Manual memory allocation/deallocation
- Heap vs stack management
- Foreign function interface
- Macros
- Traits
- Generic implementations

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
