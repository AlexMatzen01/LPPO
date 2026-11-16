# LPPO Memory Safety Update

This document describes the compile-time memory safety system added to LPPO, modeled after Rust's borrow checker. LPPO now enforces memory safety at compile time without requiring a garbage collector.

## Overview

LPPO's memory safety system has three pillars:

1. **Ownership tracking** - Every value has a single owner. Moving a value transfers ownership; the source can no longer be accessed.
2. **Borrow checking** - References (`ref`) follow Rust's aliasing rules: at any time, either one mutable reference OR any number of immutable references, not both.
3. **Lifetime-based drop analysis** - Heap-allocated variables are automatically deallocated when they exit their scope. No garbage collector needed.

## Core Concepts

### Ownership

Every variable has an ownership kind: `own`, `ref`, or `borrow`. Only `own` values can be moved. When a value is moved, the source is marked as moved and any subsequent access is a compile-time error.

```
// Move transfers ownership
let a: Int64 = 42;
move b = a;      // a is now moved, cannot be used
out b;           // OK: b owns the value
// out a;        // ERROR: use of moved value 'a'
```

### Borrowing

Use `ref` to create a reference and `deref` to dereference it:

```
// Immutable borrow - multiple simultaneous borrows allowed
let x: Int64 = 42;
let r1: ref Int64 = ref x;
let r2: ref Int64 = ref x;
out deref r1;   // 42
out deref r2;   // 42

// Mutable borrow - exclusive access
let mut y: Int64 = 100;
let r3: ref Int64 = ref mut y;
out deref r3;   // 100
// out y;        // ERROR: cannot read 'y' while it is mutably borrowed
```

Borrow rules enforced at compile time:
- You cannot mutably borrow a value that is already borrowed (immutably or mutably).
- You cannot immutably borrow a value that is mutably borrowed.
- You cannot read a variable while it is mutably borrowed.
- You cannot write to a variable while it is borrowed.

### Scope-Based Drop Analysis

Heap-allocated variables are tracked per scope. When a scope exits, all heap-allocated variables declared in that scope are automatically deallocated:

```
scope {
    alloc <Arr<Int64>> buf on heap [1024];
    // buf is heap-allocated
}
// buf is automatically deallocated here
```

Stack-allocated variables do not need explicit deallocation:

```
scope {
    alloc <Int64> x on stack [0];
    // x is stack-allocated, no dealloc needed
}
// x is dropped automatically (no heap free needed)
```

### Explicit Deallocation

You can explicitly deallocate a heap variable before its scope exits:

```
scope {
    alloc <Arr<Int64>> buf on heap [256];
    dealloc buf;    // explicit free
    // buf cannot be used after dealloc
}
```

## Syntax Reference

### Memory Allocation

```
alloc <Type> var on heap [size];
alloc <Type> var on stack [size];
dealloc var;
```

- `alloc <Type> var on heap [size]` - Allocates `size` elements of `Type` on the heap. The variable holds a pointer to the allocated block.
- `alloc <Type> var on stack [size]` - Allocates on the stack. No address-space tracking, automatic drop at scope exit.
- `dealloc var` - Explicitly frees a heap-allocated variable.

### Scope Blocks

```
scope {
    // statements
}
```

Variables declared inside a `scope` block are dropped when the block exits. Heap-allocated variables are automatically deallocated.

### Unsafe Blocks

```
unsafe {
    // borrow checker is bypassed inside this block
}
```

Inside an `unsafe` block, the borrow checker is disabled. Use this when you need to perform operations that the borrow checker cannot verify as safe. You are responsible for ensuring memory safety.

### References

```
let r: ref Int64 = ref x;       // immutable reference
let rm: ref Int64 = ref mut y;  // mutable reference
let v: Int64 = deref r;         // dereference
```

## Compiler Enforcement

The compiler enforces memory safety in three phases:

1. **Type checking** (`src/typechecker.rs`) - Tracks ownership state, borrows, and scope levels. Rejects programs with use-after-move, borrow violations, and lifetime errors.

2. **Compilation** (`src/compiler.rs`) - Tracks heap-allocated variables per scope. Inserts `Dealloc` operations at scope boundaries for automatic cleanup.

3. **Runtime** (`src/runtime.rs`) - Executes `Alloc` and `Dealloc` operations, managing a simulated address space for heap memory.

## Error Types

| Error | Meaning |
|-------|---------|
| `MoveAfterMove` | Using a value after it has been moved |
| `OwnershipError` | Attempting to move a non-owned value |
| `BorrowError` | Violating borrow aliasing rules (read while mutably borrowed, write while borrowed, etc.) |
| `LifetimeError` | Variable used outside its lifetime |

## Examples

See `examples/memory/` for working examples:
- `memory_safety.lppo` - Scope, heap/stack alloc, explicit dealloc
- `borrow_checker.lppo` - Immutable and mutable references
- `unsafe_block.lppo` - Bypassing the borrow checker

## What's NOT Covered

- No borrow checker for data passed between procedures (procedure arguments are type-checked but not borrow-checked across call boundaries)
- No borrow checker for closures or async blocks
- `unsafe` blocks completely bypass all checks - use with care
