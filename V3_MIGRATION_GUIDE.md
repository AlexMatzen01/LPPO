# LPPO V3.0 Migration Guide

## Quick Reference: V2.0 to V3.0 Syntax

This guide helps you understand the cryptic V3.0 syntax and migrate from V2.0.

### Core Concept

V3.0 replaces **ALL** human-readable keywords with **assembly-like mnemonics** to maximize complexity and eliminate readability.

## Complete Keyword Mapping

### Control Flow

| V2.0 Keyword | V3.0 Keyword | Meaning | Example (V3.0) |
|--------------|--------------|---------|----------------|
| `proc` | `PROCDEF` | Procedure Definition | `PUBEXP PROCDEF main() -> VOID0 { }` |
| `let` | `DECLVAR` | Declare Variable | `DECLVAR OWNR x: I64 = 42;` |
| `out` | `EMIT` | Emit Output | `EMIT "Hello";` |
| `call` | `PROCCALL` | Procedure Call | `PROCCALL foo(x, y);` |
| `if` | `CJMP` | Conditional Jump | `CJMP condition { }` |
| `elif` | `CJMPALT` | Alternative Conditional | `CJMPALT other_condition { }` |
| `else` | `JMPDEF` | Default Jump | `JMPDEF { }` |
| `while` | `WHLLP` | While Loop | `WHLLP i < 10 { }` |
| `for` | `ITRLP` | Iterator Loop | `ITRLP item: I64 INSET array { }` |
| `in` | `INSET` | In Set/Collection | `ITRLP x: I64 INSET [1,2,3] { }` |
| `loop` | `INFLP` | Infinite Loop | `INFLP { }` |
| `break` | `LPEXIT` | Loop Exit | `LPEXIT;` |
| `continue` | `LPSKIP` | Loop Skip | `LPSKIP;` |
| `return` | `RETVL` | Return Value | `RETVL result;` |

### Type System

| V2.0 Type | V3.0 Type | Bits | Description |
|-----------|-----------|------|-------------|
| `Int64` | `I64` | 64 | Signed integer |
| `Bool` | `BOOL8` | 8 | Boolean value |
| `Str` | `STRPTR` | ptr | String pointer |
| `Arr<T>` | `ARRPTR<T>` | ptr | Array pointer |
| `Void` | `VOID0` | 0 | Zero-byte void |
| `true` | `BOOLT` | - | Boolean true literal |
| `false` | `BOOLF` | - | Boolean false literal |

### Memory Management

| V2.0 Keyword | V3.0 Keyword | Description | Example (V3.0) |
|--------------|--------------|-------------|----------------|
| `move` | `MEMXFER` | Memory Transfer | `MEMXFER src -> dst;` |
| `copy` | `MEMCPY` | Memory Copy | `MEMCPY src -> dst;` |
| `ref` | `MEMREF` | Memory Reference | `DECLVAR MEMREF x: Ref<I64> = MEMREF y;` |
| `deref` | `MEMDEREF` | Memory Dereference | `DECLVAR val: I64 = MEMDEREF ptr;` |
| `alloc` | `HEAPALLOC` | Heap Allocation | `HEAPALLOC<ARRPTR<I64>> arr MEMSEG HEAPSEG [100];` |
| `dealloc` | `HEAPFREE` | Heap Deallocation | `HEAPFREE arr;` |

### Ownership & Visibility

| V2.0 Keyword | V3.0 Keyword | Description | Example (V3.0) |
|--------------|--------------|-------------|----------------|
| `own` | `OWNR` | Owner | `DECLVAR OWNR x: I64 = 42;` |
| `mut` | `MUTBL` | Mutable | `DECLVAR MUTBL OWNR x: I64 = 0;` |
| `pub` | `PUBEXP` | Public Export | `PUBEXP PROCDEF main() { }` |
| `module` | `MODDEF` | Module Definition | `MODDEF math { }` |
| `use` | `MODIMP` | Module Import | `MODIMP std::io;` |

### Scope & Execution

| V2.0 Keyword | V3.0 Keyword | Description | Example (V3.0) |
|--------------|--------------|-------------|----------------|
| `scope` | `STKFRAME` | Stack Frame | `STKFRAME block { }` |
| `end` | `FRAMEEND` | Frame End | `FRAMEEND` |
| `parallel` | `PARLBLK` | Parallel Block | `PARLBLK { }` |
| `unsafe` | `UNSAFEBLK` | Unsafe Block | `UNSAFEBLK { }` |
| `input` | `STDIN` | Standard Input | `STDIN x: I64 with "Enter:";` |

### Logical Operations

| V2.0 Operator | V3.0 Operator | Description | Example (V3.0) |
|---------------|---------------|-------------|----------------|
| `and` | `LOGAND` | Logical AND | `DECLVAR result: BOOL8 = a LOGAND b;` |
| `or` | `LOGOR` | Logical OR | `DECLVAR result: BOOL8 = a LOGOR b;` |
| `not` | `LOGNOT` | Logical NOT | `DECLVAR result: BOOL8 = LOGNOT a;` |

### Type Operations

| V2.0 Keyword | V3.0 Keyword | Description | Example (V3.0) |
|--------------|--------------|-------------|----------------|
| `sizeof` | `TYPESZ` | Type Size | `DECLVAR size: I64 = TYPESZ<I64>;` |
| `alignof` | `TYPEALN` | Type Alignment | `DECLVAR align: I64 = TYPEALN<I64>;` |
| `cast` | `TYPECAST` | Type Cast | `DECLVAR b: BOOL8 = TYPECAST<BOOL8>(x);` |
| `heap` | `HEAPSEG` | Heap Segment | `MEMSEG HEAPSEG` |
| `stack` | `STKSEG` | Stack Segment | `MEMSEG STKSEG` |
| `on` | `MEMSEG` | Memory Segment | `HEAPALLOC x MEMSEG HEAPSEG [10];` |

### Advanced Constructs

| V2.0 Keyword | V3.0 Keyword | Description |
|--------------|--------------|-------------|
| `struct` | `STRUCTDEF` | Structure Definition |
| `enum` | `ENUMDEF` | Enumeration Definition |
| `trait` | `TRAITDEF` | Trait Definition |
| `impl` | `IMPLBLK` | Implementation Block |
| `match` | `PATMCH` | Pattern Match |
| `new` | `INSTCRT` | Instance Create |

## Side-by-Side Examples

### Hello World

**V2.0 (Readable):**
```lppo
pub proc main() -> Void {
    let own msg: Str = "Hello, World!";
    out msg;
}
```

**V3.0 (Cryptic):**
```lppo
PUBEXP PROCDEF main() -> VOID0 {
    DECLVAR OWNR msg: STRPTR = "Hello, World!";
    EMIT msg;
}
```

### Variables and Types

**V2.0:**
```lppo
let own x: Int64 = 42;
let own flag: Bool = true;
let own text: Str = "hello";
let own numbers: Arr<Int64> = [1, 2, 3];
```

**V3.0:**
```lppo
DECLVAR OWNR x: I64 = 42;
DECLVAR OWNR flag: BOOL8 = BOOLT;
DECLVAR OWNR text: STRPTR = "hello";
DECLVAR OWNR numbers: ARRPTR<I64> = [1, 2, 3];
```

### Control Flow

**V2.0:**
```lppo
if x > 10 {
    out "large";
} elif x > 5 {
    out "medium";
} else {
    out "small";
}
```

**V3.0:**
```lppo
CJMP x > 10 {
    EMIT "large";
} CJMPALT x > 5 {
    EMIT "medium";
} JMPDEF {
    EMIT "small";
}
```

### Loops

**V2.0:**
```lppo
for item: Int64 in array {
    out item;
}

while i < 10 {
    let own next: Int64 = i + 1;
    move next -> i;
}
```

**V3.0:**
```lppo
ITRLP item: I64 INSET array {
    EMIT item;
}

WHLLP i < 10 {
    DECLVAR OWNR next: I64 = i + 1;
    MEMXFER next -> i;
}
```

### Procedures

**V2.0:**
```lppo
pub proc calculate(a: Int64, b: Int64) -> Int64 {
    let own result: Int64 = a + b;
    return result;
}

pub proc main() -> Void {
    call calculate(5, 3);
}
```

**V3.0:**
```lppo
PUBEXP PROCDEF calculate(a: I64, b: I64) -> I64 {
    DECLVAR OWNR result: I64 = a + b;
    RETVL result;
}

PUBEXP PROCDEF main() -> VOID0 {
    PROCCALL calculate(5, 3);
}
```

### Memory Operations

**V2.0:**
```lppo
alloc<Arr<Int64>> my_array on heap [100];
move source -> target;
copy original -> duplicate;
dealloc my_array;
```

**V3.0:**
```lppo
HEAPALLOC<ARRPTR<I64>> my_array MEMSEG HEAPSEG [100];
MEMXFER source -> target;
MEMCPY original -> duplicate;
HEAPFREE my_array;
```

### Logical Operations

**V2.0:**
```lppo
let own result: Bool = a and b;
let own result2: Bool = x or y;
let own result3: Bool = not flag;
```

**V3.0:**
```lppo
DECLVAR OWNR result: BOOL8 = a LOGAND b;
DECLVAR OWNR result2: BOOL8 = x LOGOR y;
DECLVAR OWNR result3: BOOL8 = LOGNOT flag;
```

## Migration Strategy

### Step 1: Test Your V2.0 Code
V2.0 code continues to work! Test it first:
```bash
cargo run -- your_program.lppo
```

### Step 2: Gradual Migration
You can mix V2.0 and V3.0 syntax in the same file:
```lppo
// This works!
pub proc helper() -> Void {  // V2.0 style
    let own x: Int64 = 10;   // V2.0 style
    out x;                    // V2.0 style
}

PUBEXP PROCDEF main() -> VOID0 {    // V3.0 style
    DECLVAR OWNR y: I64 = 20;       // V3.0 style
    EMIT y;                          // V3.0 style
    call helper();                   // V2.0 style
}
```

### Step 3: Learn the Mnemonics
Focus on the most common ones first:
- `PROCDEF` (proc), `DECLVAR` (let), `EMIT` (out)
- `I64` (Int64), `BOOL8` (Bool), `STRPTR` (Str)
- `CJMP` (if), `JMPDEF` (else)
- `OWNR` (own), `MUTBL` (mut)

### Step 4: Use the Reference
Keep this migration guide handy when writing V3.0 code!

## Why V3.0?

V3.0 exists to demonstrate what happens when you:
- ✅ Maximize cryptic complexity
- ✅ Eliminate all human-friendly syntax
- ✅ Use assembly-like mnemonics
- ✅ Make the language hostile to casual reading
- ✅ Require deep low-level computing knowledge

**V3.0 is intentionally difficult!** It's designed to be challenging, not productive.

## Need Help?

- Check examples in `examples/*_v3.lppo`
- Review the [Language Specification](docs/LANGUAGE_SPEC.md)
- See the [CHANGELOG](CHANGELOG.md) for details

## Backward Compatibility Guarantee

**100% of V2.0 programs will continue to work!**

Both syntaxes are permanently supported. You can:
- Keep using V2.0 syntax forever
- Gradually migrate to V3.0
- Mix both styles in the same file
- Choose based on your needs

There are **NO breaking changes** in V3.0!
