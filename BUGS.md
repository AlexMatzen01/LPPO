# LPPO Bugs and Issues Documentation

This document lists all discovered bugs in the LPPO programming language implementation (version 5.2.0).

## Critical Bugs (Cause Panics/Crashes)

### BUG-001: Integer Overflow in Addition Not Handled
**Severity:** Critical  
**Location:** `src/runtime.rs:237`  
**Status:** Unfixed  

**Description:**  
Integer addition operations can cause a panic due to overflow when adding values near `i64::MAX`. The runtime uses Rust's default checked arithmetic which panics on overflow instead of handling it gracefully.

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own x: Int64 = 9223372036854775807;  // i64::MAX
    let own y: Int64 = x + 1;
    out y;
}
```

**Error Output:**
```
thread 'main' panicked at src/runtime.rs:237:84:
attempt to add with overflow
```

**Impact:** Any LPPO program that performs arithmetic leading to overflow will crash instead of producing an error message.

**Suggested Fix:** Use checked arithmetic methods like `checked_add()` and return a proper `LppoError` on overflow.

---

### BUG-002: Integer Overflow in Power Operation Not Handled
**Severity:** Critical  
**Location:** `src/runtime.rs:248`  
**Status:** Unfixed  

**Description:**  
The power operation (`**`) can cause a panic when:
1. The exponent is negative (causes attempt to convert negative to u32)
2. The result would overflow i64

**Reproduction:**
```lppo
// Test 1: Negative exponent
pub proc main() -> Void {
    let own x: Int64 = 2;
    let own y: Int64 = -1;
    let own z: Int64 = x ** y;
    out z;
}

// Test 2: Overflow
pub proc main() -> Void {
    let own x: Int64 = 2;
    let own y: Int64 = 100;
    let own z: Int64 = x ** y;
    out z;
}
```

**Error Output:**
```
thread 'main' panicked at /rustc/.../library/core/src/num/mod.rs:319:5:
attempt to multiply with overflow
```

**Impact:** Any program using power operations with negative exponents or large values will crash.

**Suggested Fix:**  
1. Check if exponent is negative and return an error
2. Use checked multiplication or implement a checked_pow function
3. Consider supporting negative exponents by returning floating point results

---

### BUG-003: Modulo by Zero Causes Panic
**Severity:** Critical  
**Location:** `src/runtime.rs:247`  
**Status:** Unfixed  

**Description:**  
The modulo operation (`%`) causes a panic when the divisor is zero, instead of returning a proper error.

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own x: Int64 = 10;
    let own y: Int64 = 0;
    let own z: Int64 = x % y;
    out z;
}
```

**Error Output:**
```
thread 'main' panicked at src/runtime.rs:247:84:
attempt to calculate the remainder with a divisor of zero
```

**Impact:** Any program performing modulo with a zero divisor will crash.

**Suggested Fix:** Add a check for zero divisor before performing modulo, similar to the division check.

---

## Major Bugs (Incorrect Behavior)

### BUG-004: String Indexing Type Mismatch
**Severity:** Major  
**Location:** `src/typechecker.rs` and `src/runtime.rs:305-315`  
**Status:** Unfixed  

**Description:**  
There's a discrepancy between the type checker and runtime regarding string indexing:
- The type checker rejects string indexing (`Str[Int64]`) at compile time
- The runtime actually supports string indexing and returns individual characters

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own s: Str = "hello";
    let own i: Int64 = 0;
    out s[i];  // Type checker rejects this
}
```

**Error Output:**
```
error: type error: expected Arr<T>, found Str: cannot index into Str
```

**Impact:** Users cannot access individual characters in strings through indexing, even though the runtime supports it.

**Suggested Fix:** Update the type checker to allow `Str[Int64]` indexing and specify the return type as `Str` (single character string).

---

### BUG-005: Negative Index Handling is Inconsistent
**Severity:** Major  
**Location:** `src/runtime.rs:296-310`  
**Status:** Partially Working  

**Description:**  
The runtime attempts to support Python-style negative indexing (where `-1` means last element), but:
1. It doesn't validate that the negative index is within bounds
2. Can result in array access with very large positive indices when negative index is too negative

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own arr: Arr<Int64> = [1, 2, 3];
    let own idx: Int64 = -10;  // Way beyond array bounds
    let own val: Int64 = arr[idx];  // Undefined behavior
    out val;
}
```

**Error Output:**
```
error: type error: index -10 out of bounds
```

**Impact:** Negative indexing works for small negative values but produces confusing errors for large negative values. The error message doesn't clarify the negative indexing behavior.

**Suggested Fix:**  
1. Validate that negative indices are within bounds: `-idx <= arr.len()`
2. Improve error messages to explain negative indexing
3. Consider if negative indexing should be supported at all (not documented in language spec)

---

### BUG-006: Float Literal Parsing Ambiguity
**Severity:** Major  
**Location:** `src/lexer.rs:973-995`  
**Status:** Unfixed  

**Description:**  
The lexer has ambiguous handling of dots after numbers. A number followed by a dot and no digit (like `1.`) is treated as a method call syntax, but causes a parse error.

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own x: Int64 = 1.;  // Ambiguous: float or method call?
    out x;
}
```

**Error Output:**
```
error: expected field or method name but found 'Semicolon' at line 2, column 26
```

**Impact:** Users might accidentally write invalid float literals thinking they're valid.

**Suggested Fix:**  
1. Decide whether `1.` should be a valid float literal (like in JavaScript) or rejected
2. Update lexer to handle this case consistently
3. Add better error message explaining the ambiguity

---

### BUG-007: Division by Zero Error Message Inconsistency
**Severity:** Minor  
**Location:** `src/runtime.rs:240-245`  
**Status:** Unfixed  

**Description:**  
Division by zero returns a "type error" instead of a more specific "DivisionByZero" error that exists in the error types.

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own x: Int64 = 10;
    let own y: Int64 = 0;
    let own z: Int64 = x / y;
    out z;
}
```

**Error Output:**
```
error: type error: division by zero
```

**Impact:** The error is caught but uses wrong error variant, making error handling inconsistent.

**Suggested Fix:** Use `LppoError::DivisionByZero` instead of `LppoError::type_error`.

---

## Minor Bugs (Missing Features or Edge Cases)

### BUG-008: Dead Code Warning for Borrowed Ownership State
**Severity:** Minor  
**Location:** `src/typechecker.rs:19`  
**Status:** Unfixed  

**Description:**  
The `Borrowed` variant of `OwnershipState` enum is never constructed, indicating that the borrow checking system is incomplete.

**Compiler Warning:**
```
warning: variant `Borrowed` is never constructed
  --> src/typechecker.rs:19:5
   |
16 | enum OwnershipState {
   |      -------------- variant in this enum
...
19 |     Borrowed { mutable: bool },
   |     ^^^^^^^^
```

**Impact:** The type system claims to support borrow checking but doesn't actually track borrowed state, making the ownership system incomplete.

**Suggested Fix:** Either implement full borrow tracking or remove the unused variant and update documentation to clarify limitations.

---

### BUG-009: Array Assignment Not Implemented
**Severity:** Minor  
**Location:** `src/compiler.rs:136-140`  
**Status:** Unfixed  

**Description:**  
Indexed assignment (`arr[i] = value`) is parsed but compiled to a no-op. The operation silently fails.

**Code Location:**
```rust
Statement::IndexedAssignment { target: _, index: _, value: _ } => {
    // Indexed assignment: arr[i] = expr
    // For now, treat as no-op since arrays are immutable in simplified runtime
    Operation::Nop
}
```

**Reproduction:**
```lppo
pub proc main() -> Void {
    let mut own arr: Arr<Int64> = [1, 2, 3];
    arr[0] = 42;  // Silently does nothing
    out arr[0];   // Still prints 1
}
```

**Impact:** Users might expect array mutation to work but it silently fails.

**Suggested Fix:**  
1. Either implement mutable array updates in the runtime
2. Or make the compiler/type checker reject indexed assignments with a clear error

---

### BUG-010: Method Call Support is Limited
**Severity:** Minor  
**Location:** `src/runtime.rs:319-416`  
**Status:** Partial Implementation  

**Description:**  
The runtime only supports a hardcoded set of methods on arrays and strings. Many expected methods are missing (e.g., `abs()` for integers, `reverse()` for arrays, etc.).

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own x: Int64 = -10;
    out x.abs();  // Method not found
}
```

**Error Output:**
```
error: type error: unknown method: abs
```

**Impact:** Limited standard library functionality.

**Suggested Fix:** Implement more standard methods or document which methods are supported.

---

### BUG-011: Procedure Call in Expressions Doesn't Execute Full Procedure
**Severity:** Major  
**Location:** `src/runtime.rs:532-545`  
**Status:** Unfixed  

**Description:**  
When calling a procedure from an expression context (like in `let x = call proc()`), the runtime only looks for explicit return statements and doesn't execute all operations in the procedure body.

**Code Location:**
```rust
// Execute the procedure
// For now, we need to extract the return value from the procedure execution
// Since procedures may use "return" statements, we need to handle ControlFlow
for op in &proc_body.operations {
    match op {
        Operation::Return(Some(ret_expr)) => {
            return eval_expr(ret_expr, &mut proc_env, procedures);
        }
        Operation::Return(None) => {
            return Ok(Value::Int(0)); // Unit/void return
        }
        _ => {
            // Execute other operations but ignore their results
            // This is a simplified implementation
        }
    }
}
```

**Impact:** Procedures with side effects (like `out`) won't execute when called from expressions unless they have an explicit return. This makes procedure behavior inconsistent.

**Suggested Fix:** Properly execute all operations in the procedure body before looking for returns.

---

### BUG-012: Float Type Not Fully Integrated
**Severity:** Minor  
**Location:** `src/runtime.rs:209-213`  
**Status:** Partial Implementation  

**Description:**  
Float literals are parsed and stored as strings, not actual floating-point values. All float operations are unimplemented.

**Code Location:**
```rust
Expr::Float(f) => {
    // Parse float string to f64 and store as string for display
    // For now, treat floats as strings in output
    Ok(Value::Str(f.clone()))
}
```

**Impact:** Cannot perform arithmetic on floats, making the Float type essentially useless.

**Suggested Fix:** Add a `Float(f64)` variant to the `Value` enum and implement float arithmetic.

---

### BUG-013: Closure Type Annotation Mismatch
**Severity:** Minor  
**Location:** Type system doesn't have proper closure types  
**Status:** Unfixed  

**Description:**  
Closures cannot be assigned to variables with type annotations because there's no closure type that can be used in variable declarations.

**Reproduction:**
```lppo
pub proc main() -> Void {
    let own f: ??? = |x: Int64| -> Int64 { return x + 1; };  // What type should f be?
    // ...
}
```

**Impact:** Closures can only be used inline, not stored in variables or passed as typed parameters.

**Suggested Fix:** Add a function/closure type syntax like `Fn(Int64) -> Int64` to the type system.

---

## Documentation/Design Issues

### BUG-014: Excessive Reserved Keywords
**Severity:** Minor  
**Location:** `src/lexer.rs:5-817`  
**Status:** Unfixed  

**Description:**  
The lexer reserves over 600 tokens/keywords, many of which are not implemented or used. This makes common identifiers unavailable.

Examples of over-reserved words that could be identifiers:
- `label`, `target` (commented out due to conflicts)
- `error`, `warning`, `info` (logging keywords that conflict with common variable names)
- `test`, `hash`, `random`, `json`, `csv`, etc.

**Impact:** Users cannot use many common variable names that should be available.

**Suggested Fix:** Review and remove unused keywords, or make them contextual keywords that are only reserved in specific contexts.

---

### BUG-015: Unused V4.0/V5.2 Features
**Severity:** Minor  
**Location:** Multiple files  
**Status:** Documentation Issue  

**Description:**  
The lexer and documentation claim support for 1,230+ features (V5.2), including:
- SIMD operations
- GPU computing
- JIT compilation
- Effect systems
- Dependent types

However, none of these features are actually implemented in the parser, compiler, or runtime.

**Impact:** Misleading documentation and excessive complexity in the lexer.

**Suggested Fix:** Either implement these features or remove them from the lexer and update documentation to reflect actual capabilities.

---

## Summary

**Total Bugs Found:** 15

**By Severity:**
- Critical (Panics/Crashes): 3 bugs
- Major (Incorrect Behavior): 6 bugs
- Minor (Missing Features): 6 bugs

**Priority Fixes:**
1. BUG-001, BUG-002, BUG-003: Add proper overflow and divide-by-zero handling
2. BUG-004: Fix string indexing type mismatch
3. BUG-011: Fix procedure call expression execution
4. BUG-009: Implement or properly reject array mutation
5. BUG-008: Complete or remove borrow checking

## Testing Methodology

Bugs were discovered through:
1. Static analysis using `cargo clippy`
2. Manual code review of critical sections (lexer, parser, runtime, typechecker)
3. Creating targeted test cases for edge cases
4. Testing arithmetic operations with boundary values
5. Testing type system with various combinations of types and operations

## Version Information

- LPPO Version: 5.2.0
- Rust Edition: 2021
- Date of Analysis: 2025-11-18
