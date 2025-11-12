# LPPO Best Practices Guide

## Introduction

This guide provides recommendations for writing clean, maintainable, and efficient LPPO code. While LPPO is deliberately verbose and explicit, following these practices will help you write better programs.

## Table of Contents

1. [Code Organization](#code-organization)
2. [Naming Conventions](#naming-conventions)
3. [Type Management](#type-management)
4. [Memory Management](#memory-management)
5. [Error Handling](#error-handling)
6. [Control Flow](#control-flow)
7. [Procedures](#procedures)
8. [Documentation](#documentation)
9. [Performance](#performance)
10. [Common Pitfalls](#common-pitfalls)

## Code Organization

### Module Structure

**DO**: Organize related functionality into modules
```lppo
module math {
    pub proc add(a: Int64, b: Int64) -> Void { }
    pub proc multiply(a: Int64, b: Int64) -> Void { }
}

module string_utils {
    pub proc upper(s: Str) -> Void { }
    pub proc lower(s: Str) -> Void { }
}
```

**DON'T**: Put everything in the global namespace
```lppo
pub proc add(a: Int64, b: Int64) -> Void { }
pub proc multiply(a: Int64, b: Int64) -> Void { }
pub proc upper(s: Str) -> Void { }
pub proc lower(s: Str) -> Void { }
```

### File Organization

**Recommended Structure**:
```
project/
├── main.lppo          # Entry point
├── math.lppo          # Math utilities
├── string.lppo        # String utilities
└── utils.lppo         # General utilities
```

### Visibility

**DO**: Use `pub` for public API, omit for internal procedures
```lppo
module utils {
    // Public API
    pub proc process_data(data: Arr<Int64>) -> Void {
        call validate_data(data);
        call transform_data(data);
    }
    
    // Internal helpers
    proc validate_data(data: Arr<Int64>) -> Void { }
    proc transform_data(data: Arr<Int64>) -> Void { }
}
```

## Naming Conventions

### Variables

**DO**: Use descriptive snake_case names
```lppo
let own user_count: Int64 = 100;
let own max_retries: Int64 = 3;
let own is_valid: Bool = true;
```

**DON'T**: Use single letters or unclear abbreviations
```lppo
let own x: Int64 = 100;
let own n: Int64 = 3;
let own flg: Bool = true;
```

### Procedures

**DO**: Use verb-noun patterns for procedures
```lppo
pub proc calculate_sum(arr: Arr<Int64>) -> Void { }
pub proc validate_input(data: Str) -> Void { }
pub proc process_array(items: Arr<Int64>) -> Void { }
```

**DON'T**: Use vague names
```lppo
pub proc do_stuff(arr: Arr<Int64>) -> Void { }
pub proc handle(data: Str) -> Void { }
```

### Modules

**DO**: Use lowercase names describing the domain
```lppo
module math_operations { }
module string_utilities { }
module data_processing { }
```

### Boolean Variables

**DO**: Use is_, has_, should_ prefixes
```lppo
let own is_valid: Bool = true;
let own has_errors: Bool = false;
let own should_continue: Bool = true;
```

### Constants

**DO**: Use descriptive names for magic numbers
```lppo
let own max_array_size: Int64 = 1000;
let own default_timeout: Int64 = 30;
let own min_password_length: Int64 = 8;
```

**DON'T**: Use magic numbers directly
```lppo
// What does 1000 mean?
while count < 1000 {
    // ...
}
```

## Type Management

### Explicit Types Everywhere

**DO**: Always declare types explicitly
```lppo
let own count: Int64 = 0;
let own message: Str = "Hello";
let own items: Arr<Int64> = [1, 2, 3];
```

**Reminder**: LPPO has NO type inference

### Break Down Complex Expressions

**DO**: Use intermediate variables
```lppo
let own a: Int64 = 10;
let own b: Int64 = 5;
let own c: Int64 = 3;

let own temp1: Int64 = a * b;
let own temp2: Int64 = temp1 + c;
let own result: Int64 = temp2 / c;
```

**DON'T**: Try to do everything in one line
```lppo
// This doesn't work in LPPO anyway
// let own result = (a * b + c) / c;
```

### Ownership Qualifiers

**DO**: Use appropriate qualifiers
```lppo
let own immutable: Int64 = 42;        // Can't change
let mut own mutable: Int64 = 0;       // Can change via move
let ref reference: Ref<Int64> = ref x; // Reference
```

**Best Practice**: Prefer `own` unless you need mutability

## Memory Management

### Variable Lifetime

**DO**: Limit variable scope
```lppo
pub proc process_data() -> Void {
    scope validation {
        let own temp_data: Arr<Int64> = [1, 2, 3];
        // Use temp_data
    }
    // temp_data no longer accessible
}
```

### Move vs Copy

**DO**: Understand when to use each
```lppo
// Move: Transfer ownership (original becomes invalid)
let own source: Int64 = 42;
let own target: Int64 = 0;
move source -> target;
// source is now invalid

// Copy: Duplicate value (both remain valid)
let own original: Int64 = 42;
let own duplicate: Int64 = 0;
copy original -> duplicate;
// Both are valid
```

**Best Practice**: Use `move` for mutable variables, `copy` for immutable

### Memory Allocation

**DO**: Explicitly manage memory when needed
```lppo
alloc<Arr<Int64>> large_array on heap [10000];
// ... use array ...
dealloc large_array;
```

**DON'T**: Forget to deallocate
```lppo
alloc<Arr<Int64>> large_array on heap [10000];
// ... use array ...
// Memory leak! No dealloc
```

## Error Handling

### Check Bounds

**DO**: Validate array access
```lppo
let own arr: Arr<Int64> = [1, 2, 3];
let own len: Int64 = arr.len();
let own index: Int64 = 5;

let own is_valid: Bool = index < len;
if is_valid {
    let own value: Int64 = arr[index];
    out value;
} else {
    out "Index out of bounds";
}
```

### Avoid Division by Zero

**DO**: Check divisor before division
```lppo
let own numerator: Int64 = 10;
let own denominator: Int64 = 0;
let own zero: Int64 = 0;

let own is_zero: Bool = denominator == zero;
if is_zero {
    out "Error: Division by zero";
} else {
    let own result: Int64 = numerator / denominator;
    out result;
}
```

### Meaningful Error Messages

**DO**: Provide context in error messages
```lppo
if error_condition {
    let own msg: Str = "Failed to process user data: invalid format";
    out msg;
}
```

**DON'T**: Use vague errors
```lppo
if error_condition {
    out "Error";
}
```

## Control Flow

### Explicit Conditions

**DO**: Store conditions in named variables
```lppo
let own age: Int64 = 18;
let own min_age: Int64 = 18;
let own is_adult: Bool = age >= min_age;

if is_adult {
    out "Access granted";
} else {
    out "Access denied";
}
```

**Benefits**: Clearer intent, easier to debug

### Loop Counters

**DO**: Use clear counter management
```lppo
let mut own i: Int64 = 0;
let own limit: Int64 = 10;
let own one: Int64 = 1;

while i < limit {
    out i;
    
    let own i_next: Int64 = i + one;
    move i_next -> i;
}
```

### Break and Continue

**DO**: Use for early exit when appropriate
```lppo
for item: Int64 in items {
    let own zero: Int64 = 0;
    let own is_negative: Bool = item < zero;
    
    if is_negative {
        out "Found negative, stopping";
        break;
    }
    
    let own two: Int64 = 2;
    let own rem: Int64 = item % two;
    let own is_even: Bool = rem == zero;
    
    if is_even {
        continue;  // Skip even numbers
    }
    
    out item;
}
```

## Procedures

### Single Responsibility

**DO**: Each procedure should do one thing
```lppo
pub proc calculate_sum(arr: Arr<Int64>) -> Void {
    let mut own sum: Int64 = 0;
    
    for num: Int64 in arr {
        let own temp: Int64 = sum + num;
        move temp -> sum;
    }
    
    out sum;
}

pub proc calculate_average(arr: Arr<Int64>) -> Void {
    // Call calculate_sum then divide by length
}
```

**DON'T**: Mix multiple responsibilities
```lppo
pub proc process_data(arr: Arr<Int64>) -> Void {
    // Validates
    // Transforms
    // Calculates
    // Formats
    // Outputs
}
```

### Parameter Naming

**DO**: Use descriptive parameter names
```lppo
pub proc greet_user(user_name: Str, user_age: Int64) -> Void {
    out "Hello", user_name;
    out "Age:", user_age;
}
```

**DON'T**: Use generic names
```lppo
pub proc greet_user(s: Str, n: Int64) -> Void {
    out s, n;
}
```

### Keep Procedures Short

**Best Practice**: Aim for procedures under 50 lines

**DO**: Split long procedures
```lppo
pub proc process_data(data: Arr<Int64>) -> Void {
    call validate_data(data);
    call transform_data(data);
    call output_results(data);
}

proc validate_data(data: Arr<Int64>) -> Void { }
proc transform_data(data: Arr<Int64>) -> Void { }
proc output_results(data: Arr<Int64>) -> Void { }
```

## Documentation

### Comments

**DO**: Explain why, not what
```lppo
// Multiply by negative one to handle edge case in protocol
let own neg_one: Int64 = -1;
let own result: Int64 = value * neg_one;
```

**DON'T**: State the obvious
```lppo
// Add one to counter
let own one: Int64 = 1;
let own temp: Int64 = counter + one;
```

### Procedure Documentation

**DO**: Document purpose and parameters
```lppo
// Calculate the factorial of a number iteratively
// Parameters:
//   n: The number to calculate factorial for
// Outputs: The factorial result
pub proc factorial(n: Int64) -> Void {
    // Implementation
}
```

### Module Documentation

**DO**: Describe module purpose at the top
```lppo
// Math utilities module
// Provides common mathematical operations including:
// - Basic arithmetic helpers
// - Statistical calculations
// - Array operations
module math {
    // Procedures
}
```

## Performance

### Minimize Moves

**DO**: Avoid unnecessary moves
```lppo
let mut own counter: Int64 = 0;
let own one: Int64 = 1;

// GOOD: Reuse one
let own temp1: Int64 = counter + one;
move temp1 -> counter;

let own temp2: Int64 = counter + one;
move temp2 -> counter;
```

**DON'T**: Create new literals each time
```lppo
// BAD: Creates new Int64 each iteration
loop {
    let own one: Int64 = 1;
    let own temp: Int64 = counter + one;
    move temp -> counter;
}
```

### Array Operations

**DO**: Consider array size
```lppo
// For large arrays, be aware of copying
let own large_array: Arr<Int64> = [/* 10000 elements */];

// Operations may copy the entire array
let own another_array: Arr<Int64> = [/* more */];
let own combined: Arr<Int64> = large_array + another_array;
```

### Parallel Execution

**DO**: Use for independent, compute-heavy tasks
```lppo
scope parallel_section {
    parallel {
        out "Computing expensive operation 1";
        out "Computing expensive operation 2";
        out "Computing expensive operation 3";
    }
}
```

**DON'T**: Use for trivial operations
```lppo
// Overkill for simple outputs
scope parallel_section {
    parallel {
        out "a";
        out "b";
    }
}
```

## Common Pitfalls

### Forgetting Type Annotations

**Problem**:
```lppo
// This doesn't work
let own x = 10;  // ERROR: Missing type
```

**Solution**:
```lppo
let own x: Int64 = 10;
```

### Using Expressions in Conditions

**Problem**:
```lppo
// This doesn't work
if x > 10 {  // ERROR: Expression not allowed
}
```

**Solution**:
```lppo
let own ten: Int64 = 10;
let own condition: Bool = x > ten;
if condition {
}
```

### Forgetting Loop Variable Types

**Problem**:
```lppo
// This doesn't work
for item in array {  // ERROR: Missing type
}
```

**Solution**:
```lppo
for item: Int64 in array {
}
```

### Using Moved Values

**Problem**:
```lppo
let own source: Int64 = 42;
let own target: Int64 = 0;
move source -> target;
out source;  // ERROR: source is invalid
```

**Solution**: Don't use moved variables

### Not Breaking Complex Expressions

**Problem**: Trying to do too much at once

**Solution**: Break into steps
```lppo
// Step by step
let own a: Int64 = 10;
let own b: Int64 = 5;
let own sum: Int64 = a + b;
let own two: Int64 = 2;
let own result: Int64 = sum * two;
```

## Code Review Checklist

Before committing code, verify:

- [ ] All variables have explicit type annotations
- [ ] All ownership qualifiers are present
- [ ] Complex expressions are broken down
- [ ] Conditions are stored in Bool variables
- [ ] Loop variables have explicit types
- [ ] No magic numbers (use named constants)
- [ ] Descriptive variable and procedure names
- [ ] Comments explain why, not what
- [ ] Procedures have single responsibility
- [ ] Proper error checking
- [ ] Memory allocation paired with deallocation
- [ ] No unused variables
- [ ] Consistent formatting

## Style Guide Summary

**Indentation**: 4 spaces (no tabs)

**Braces**: Opening brace on same line
```lppo
pub proc example() -> Void {
    if condition {
        // body
    }
}
```

**Spacing**: Space after comma and around operators
```lppo
let own sum: Int64 = a + b;
out value1, value2, value3;
```

**Line Length**: Aim for under 100 characters

**Blank Lines**: Use to separate logical sections

## Conclusion

LPPO's explicit nature forces you to think carefully about:
- Types
- Memory
- Ownership
- Control flow

Following these best practices will make your LPPO code:
- More maintainable
- Easier to understand
- Less error-prone
- Better organized

Remember: Verbosity is a feature, not a bug!
