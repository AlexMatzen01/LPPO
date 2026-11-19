# LPPO Bytecode Reference

**Source of truth:** `src/runtime.rs` (`Operation` enum), `src/ast.rs` (`Expr`, `TypeAnnotation`, `BinaryOp`, `UnaryOp`), `src/compiler.rs` (AST → `ProcedureBody`).

This document is the semantic specification for the LPPO bytecode instruction set. It is the stable contract between the frontend (lexer/parser/typechecker/compiler) and the execution backends (interpreter and LLVM AOT codegen). The interpreter in `src/runtime.rs` is the reference implementation; the LLVM backend must produce functionally equivalent native code.

---

## 1. Program Structure

A compiled LPPO program is a map of procedure names to `ProcedureBody`:

```
ProcedureBody {
    params: Vec<String>,        // parameter names (types are compile-time only)
    operations: Vec<Operation>, // flat bytecode instruction stream
}
```

Bytecode is serialized as JSON (`runtime::serialize_procedures` / `executor_from_bytes`) and may be XOR-obfuscated (`OBFUSCATION_MARKER = b"LPPO-OBF-1\n"`).

The entry point is the procedure named `"main"`. Execution begins with an empty environment `HashMap<String, Value>`.

---

## 2. Value Representation (Runtime `Value` enum)

| Variant | LLVM type (AOT) | Description |
|---|---|---|
| `Value::Int(i64)` | `i64` | 64-bit signed integer |
| `Value::Float(f64)` | `double` | 64-bit IEEE float |
| `Value::Bool(bool)` | `i1` | Boolean |
| `Value::Str(String)` | `i8*` | Pointer to null-terminated UTF-8 string (heap-allocated) |
| `Value::Array(Vec<Value>)` | `%lppo.array = { i8*, i64 }` | `{ data_ptr, len }` struct |
| `Value::Object(HashMap<String,Value>)` | opaque `i8*` | Hash map handle (runtime-managed) |
| `Value::Closure {..}` | opaque `i8*` | Closure object handle |
| `Value::Future { id }` | `i64` | Future handle id |
| `Value::JoinHandle { id }` | `i64` | Thread join handle id |

All scalar values are passed/returned in registers per the System V AMD64 ABI. Strings, arrays, objects, closures are heap-allocated and passed by pointer.

Truthiness (`is_truthy`): `Bool(b)=b`, `Int(i)=i!=0`, `Float(f)=f!=0.0`, `Str(s)=!s.is_empty()`, `Array/Object=non-empty`, `Closure/Future/JoinHandle=always true`.

---

## 3. Instruction Reference

### 3.1 `Emit(Vec<Expr>)`

| Field | Type | Meaning |
|---|---|---|
| `exprs` | `Vec<Expr>` | One or more expressions to evaluate |

**Semantics:** Evaluate each expression in order, convert to string via `value_to_string`, join with spaces, print followed by newline. Returns `()` (no value).

**LLVM mapping:** For each `expr`, codegen to an LLVM value, then call extern `lppo_rt_print_value(type_tag, value)` per operand, followed by `lppo_rt_print_newline()`.

**Control flow:** Falls through to next instruction.

---

### 3.2 `Parallel(Vec<Expr>)`

| Field | Type | Meaning |
|---|---|---|
| `exprs` | `Vec<Expr>` | Expressions to evaluate concurrently |

**Semantics:** Evaluate each expression in a separate OS thread (via `std::thread::scope`), each with a snapshot of the current environment, print each result. All threads joined before continuing.

**LLVM mapping (native):** Deferred to full threading support. Early phases emit a clear "not yet implemented in native mode" error.

**Control flow:** Falls through after all threads complete.

---

### 3.3 `Input { var, typ, prompt }`

| Field | Type | Meaning |
|---|---|---|
| `var` | `String` | Variable name to bind input to |
| `typ` | `TypeAnnotation` | Expected type (`Int64`, `Bool`, `Str`, `Arr<T>`) |
| `prompt` | `Option<Expr>` | Optional prompt expression |

**Semantics:** Evaluate `prompt` (if any) and print without newline. Read one line from stdin, trim whitespace, parse according to `typ`, bind result to `var`.

Parsing rules:
- `Int64`: parse `i64`
- `Bool`: `true/t/yes/y/1` → true, `false/f/no/n/0` → false
- `Str`: raw trimmed line
- `Arr<T>`: `[1, 2, 3]` or comma-separated, element type per `T`

**LLVM mapping:** Call extern `lppo_rt_input(type_tag, prompt_ptr)` which returns a `Value`; store result into local `var`.

**Control flow:** Falls through; blocks on stdin.

---

### 3.4 `Alloc { var, location, size }`

| Field | Type | Meaning |
|---|---|---|
| `var` | `String` | Variable to bind address/handle to |
| `location` | `MemoryLocation` | `Heap` or `Stack` |
| `size` | `Expr` | Allocation size in bytes (heap only) |

**Semantics:**
- `Heap`: Allocate `size` bytes in the simulated address space, initialize to zero, bind `var` to starting address (`Value::Int(addr)`). Track for auto-deallocation at scope/proc exit.
- `Stack`: Bind `var` to `Value::Int(0)` (no actual stack reservation in simplified runtime).

**LLVM mapping:**
- `Heap`: Call extern `lppo_rt_alloc(size)` returning `i64` address; store into local `var` (alloca `i8*`).
- `Stack`: Store `0`/`nullptr` into local `var`.

**Control flow:** Falls through.

---

### 3.5 `Dealloc(var)`

| Field | Type | Meaning |
|---|---|---|
| `var` | `String` | Variable whose memory to free |

**Semantics:** Free the memory block at `var`'s address, remove `var` from environment.

**LLVM mapping:** Call extern `lppo_rt_dealloc(ptr)` where `ptr` is loaded from local `var`.

**Control flow:** Falls through.

---

### 3.6 `Call(String, Vec<Expr>)`

| Field | Type | Meaning |
|---|---|---|
| `name` | `String` | Procedure name to call |
| `args` | `Vec<Expr>` | Argument expressions |

**Semantics:** Evaluate all arguments in current environment, look up procedure by name, check arity, bind parameters in a fresh environment, execute procedure body, propagate `Return`/`Break`/`Continue` control flow.

Built-in calls (fast-path in `eval_expr`): `println`, `print`, `eprintln`, `read_file`, `write_file`, `append_file`, `file_exists`, `delete_file`, `str_concat`, `int_to_str`, `str_to_int`, `str_len`, `str_upper`, `str_lower`, `str_trim`, `str_split`, `abs`, `min`, `max`, `pow`, `buf_*`, `random`, `eval`, `sqrt`, `either`, `json_parse`, `json_stringify`, `system`, `color`, `tui_*`, `pool_*`, `vec_*`, `map_*`.

**LLVM mapping:**
- User proc: `call` LLVM function `@name(args...)`.
- Built-in: `call` extern `lppo_rt_<name>(args...)`.
- Arity mismatch → codegen/runtime error.

**Control flow:** Falls through after call returns (unless call is last and procedure returns).

---

### 3.7 `Let(String, Expr)`

| Field | Type | Meaning |
|---|---|---|
| `name` | `String` | Variable name |
| `expr` | `Expr` | Expression to evaluate and bind |

**Semantics:** Evaluate `expr` in current environment, bind result to `name` (overwrites any existing binding). Used for `let`, `move`, `copy`, and `assignment` statements.

**LLVM mapping:** Codegen `expr` to LLVM value, `store` into alloca slot for `name`.

**Control flow:** Falls through.

---

### 3.8 `If { cond, then_ops, elifs, else_ops }`

| Field | Type | Meaning |
|---|---|---|
| `cond` | `Expr` | Condition expression |
| `then_ops` | `Vec<Operation>` | Then-branch operations |
| `elifs` | `Vec<(Expr, Vec<Operation>)>` | elif condition + operations pairs |
| `else_ops` | `Option<Vec<Operation>>` | Optional else operations |

**Semantics:** Evaluate `cond`; if truthy, execute `then_ops`. Else evaluate each elif condition in order; first truthy branch executes. Else execute `else_ops` if present. Propagate `Return`/`Break`/`Continue`.

**LLVM mapping:** Create basic blocks: `entry` (eval cond), `then`, `elif_N*` (each), `else` (optional), `merge`. Use `br i1 cond, then, next`. Chained elifs fall through. `br` to `merge` at end of each branch. Phi nodes only if a value is defined in multiple branches and used after merge (LPPO uses alloca-based locals, so usually unnecessary).

**Control flow:** Branches to one of several blocks, reconverges at `merge`.

---

### 3.9 `While { cond, ops }`

| Field | Type | Meaning |
|---|---|---|
| `cond` | `Expr` | Loop condition |
| `ops` | `Vec<Operation>` | Loop body |

**Semantics:** While `cond` is truthy, execute `ops`. `Break` exits, `Continue` jumps to next condition check, `Return` propagates out.

**LLVM mapping:** Blocks: `cond`, `body`, `exit`. `br i1 cond, body, exit`. At end of `body`, `br cond` (back-edge). `Break` → `br exit`. `Continue` → `br cond`.

**Control flow:** Loop with back-edge to condition block.

---

### 3.10 `For { var, iterable, ops }`

| Field | Type | Meaning |
|---|---|---|
| `var` | `String` | Loop variable name |
| `iterable` | `Expr` | Expression evaluating to an `Array` |
| `ops` | `Vec<Operation>` | Loop body |

**Semantics:** Evaluate `iterable` to an array; for each element, bind `var` to element, execute `ops`. `Break`/`Continue`/`Return` propagate as with `While`.

**LLVM mapping:** Evaluate iterable to array struct `{ptr, len}`. Blocks: `init` (eval iterable), `cond` (index < len), `body`, `increment` (index++), `exit`. Load element from array each iteration, store into `var` alloca.

**Control flow:** Loop with index comparison and back-edge.

---

### 3.11 `Loop { ops }`

| Field | Type | Meaning |
|---|---|---|
| `ops` | `Vec<Operation>` | Infinite loop body |

**Semantics:** Execute `ops` forever unless `Break` or `Return`. `Continue` restarts body.

**LLVM mapping:** Blocks: `body`, `exit`. Unconditional `br body` at end of body (back-edge). `Break` → `br exit`. `Continue` → `br body`.

**Control flow:** Infinite loop with conditional exit.

---

### 3.12 `Break`

**Semantics:** Exit the innermost enclosing loop (`While`/`For`/`Loop`).

**LLVM mapping:** `br` to the loop's exit block (tracked via label stack in codegen context).

**Control flow:** Unconditional branch to loop exit.

---

### 3.13 `Continue`

**Semantics:** Jump to the next iteration of the innermost enclosing loop (condition check for `While`/`For`, body restart for `Loop`).

**LLVM mapping:** `br` to the loop's continue target block (tracked via label stack).

**Control flow:** Unconditional branch to loop continue target.

---

### 3.14 `Return(Option<Expr>)`

| Field | Type | Meaning |
|---|---|---|
| `expr` | `Option<Expr>` | Optional return value |

**Semantics:** Evaluate `expr` (if any), return control to caller with the value. `None` returns `()`/`Void`.

**LLVM mapping:** `ret <type>` or `ret void`. Value loaded from expr codegen result.

**Control flow:** Terminates current procedure; transfers to caller.

---

### 3.15 `Scope { ops }`

| Field | Type | Meaning |
|---|---|---|
| `ops` | `Vec<Operation>` | Operations in the scope |

**Semantics:** Execute `ops` in a new scope. Heap-allocated variables in this scope are deallocated at scope exit (inserted by compiler as `Dealloc` ops).

**LLVM mapping:** Execute `ops` inline; LLVM alloca scoping handles local variable lifetime. `Dealloc` ops emit `lppo_rt_dealloc`.

**Control flow:** Falls through; propagates `Return`/`Break`/`Continue`.

---

### 3.16 `Unsafe { ops }`

| Field | Type | Meaning |
|---|---|---|
| `ops` | `Vec<Operation>` | Operations in unsafe block |

**Semantics:** Treated identically to `Scope` at runtime; bypasses borrow checking at compile time only.

**LLVM mapping:** Same as `Scope`.

**Control flow:** Falls through.

---

### 3.17 `Nop`

**Semantics:** No operation. Placeholder for unimplemented statements (e.g. `Match`, `StructInit` placeholders).

**LLVM mapping:** No IR emitted.

**Control flow:** Falls through.

---

### 3.18 `Pause { ... }`

| Field | Type | Meaning |
|---|---|---|
| `prompt` | `Option<Expr>` | Optional prompt |
| `timeout_ms` | `Option<Expr>` | Optional timeout |
| `until` | `Option<Expr>` | Wait until input equals this |
| `assign_to` | `Option<String>` | Variable to store input |
| `on_mismatch` | `Option<Expr>` | Handler on until-mismatch |
| `on_timeout` | `Option<Expr>` | Handler on timeout |
| `is_async` | `bool` | Schedule remainder in background |

**Semantics:** Wait for user input (or timeout), optionally match against `until`, run handlers on mismatch/timeout. Async form schedules remaining ops in a background thread.

**LLVM mapping (native):** Deferred to full runtime threading support. Early phases emit a clear "not yet implemented in native mode" error.

**Control flow:** May suspend execution; async form returns immediately and runs remainder in background.

---

### 3.19 `ExprStatement(Expr)`

| Field | Type | Meaning |
|---|---|---|
| `expr` | `Expr` | Expression to evaluate for side effects |

**Semantics:** Evaluate `expr` and discard the result. Used for method calls with side effects (e.g. `arr.push(x)`).

**LLVM mapping:** Codegen `expr` (side effects occur during evaluation), discard result.

**Control flow:** Falls through.

---

### 3.20 `Spawn { ops }`

| Field | Type | Meaning |
|---|---|---|
| `ops` | `Vec<Operation>` | Operations to run in new thread |

**Semantics:** Snapshot current environment, run `ops` in a new OS thread (fire-and-forget). Returns a `JoinHandle`.

**LLVM mapping (native):** Deferred to full threading support. Early phases emit a clear "not yet implemented in native mode" error.

**Control flow:** Falls through immediately; spawned thread runs concurrently.

---

## 4. Expression Reference

Expressions are evaluated to produce `Value`s. They appear as operands to `Let`, `Call`, `Emit`, `Input`, `Return`, `Alloc`, `If/While/For` conditions, etc.

### 4.1 Literals

| Expr | LLVM IR | Notes |
|---|---|---|
| `Expr::Int(i64)` | `i64` constant | |
| `Expr::Float(String)` | `double` constant | Parsed at codegen time |
| `Expr::Str(String)` | `i8*` global constant | Stored in `.rodata` as null-terminated string |
| `Expr::Bool(bool)` | `i1` constant `true`/`false` | |

### 4.2 Variables

| Expr | LLVM IR | Notes |
|---|---|---|
| `Expr::Var(name)` | `load` from alloca slot `name` | Local variable |
| `Expr::QualifiedVar(qn)` | `load` from alloca slot `module::name` | Namespaced variable |

### 4.3 Binary Operations

`Expr::BinaryOp { left, op, right }`

| Op | LLVM IR (Int) | LLVM IR (Float) | LLVM IR (Str) | LLVM IR (Bool) |
|---|---|---|---|---|
| `Add` | `add` | `fadd` | call `lppo_rt_str_concat` | — |
| `Sub` | `sub` | `fsub` | — | — |
| `Mul` | `mul` | `fmul` | — | — |
| `Div` | `sdiv` (check div0) | `fdiv` (check div0) | — | — |
| `Mod` | `srem` | — | — | — |
| `Pow` | call `lppo_rt_pow` | — | — | — |
| `Eq` | `icmp eq` | `fcmp oeq` | call `lppo_rt_str_eq` | `icmp eq` |
| `Ne` | `icmp ne` | `fcmp one` | call `lppo_rt_str_ne` | `icmp ne` |
| `Lt` | `icmp slt` | `fcmp olt` | call `lppo_rt_str_lt` | — |
| `Le` | `icmp sle` | `fcmp ole` | call `lppo_rt_str_le` | — |
| `Gt` | `icmp sgt` | `fcmp ogt` | call `lppo_rt_str_gt` | — |
| `Ge` | `icmp sge` | `fcmp oge` | call `lppo_rt_str_ge` | — |
| `And` | `and` (on i1) | — | — | `and` |
| `Or` | `or` (on i1) | — | — | `or` |
| `BitAnd` | `and` (on i64) | — | — | — |
| `BitOr` | `or` (on i64) | — | — | — |
| `BitXor` | `xor` (on i64) | — | — | — |

Comparison results are `i1`; logical ops expect `i1` operands.

### 4.4 Unary Operations

`Expr::UnaryOp { op, operand }`

| Op | LLVM IR (Int) | LLVM IR (Float) | LLVM IR (Bool) |
|---|---|---|---|
| `Neg` | `sub 0, x` | `fsub 0.0, x` | — |
| `Not` | — | — | `xor true, x` / `not` |
| `BitNot` | `xor -1, x` / `not` | — | — |

### 4.5 Compound Expressions

| Expr | LLVM IR | Notes |
|---|---|---|
| `Expr::Array(elements)` | Call `lppo_rt_array_make(n)` then set each element | Array struct `{ptr, len}` |
| `Expr::Index { target, index }` | Call `lppo_rt_array_get(arr, idx)` | Supports negative indexing |
| `Expr::MethodCall { target, method, args }` | Dispatch per method name to `lppo_rt_*` | `push`, `pop`, `len`, `iter`, `map`, `filter`, `first`, etc. |
| `Expr::FieldAccess { target, field }` | Object field lookup | Opaque object handle |
| `Expr::SizeOf(typ)` | Constant per type | `Int64`=8, `Bool`=1, `Str`=16, `Arr`=24 |
| `Expr::AlignOf(typ)` | Constant per type | `Int64`=8, `Bool`=1, default=8 |
| `Expr::Cast { expr, to_type }` | Type conversion call | `Int`↔`Bool`, `Int`↔`Str`, etc. |
| `Expr::Closure {..}` | Heap-allocated closure struct | Deferred (full closure support) |
| `Expr::Call { func, args }` | `call` function pointer or proc | Higher-order calls deferred |

---

## 5. Type Lowering (`TypeAnnotation` → LLVM)

| LPPO Type | LLVM Type | AOT Representation |
|---|---|---|
| `Int64` | `i64` | Register / alloca |
| `Bool` | `i1` | Register / alloca |
| `Float` | `double` | Register / alloca |
| `Str` | `i8*` | Pointer to null-terminated string |
| `Arr<T>` | `%lppo.array = { i8*, i64 }` | `{ data_ptr, len }` |
| `Void` | `void` | No return value |
| `Ref<T>` / `Own<T>` / `Mut<T>` | Same as `T` | Ownership is compile-time only |
| `Custom(String)` | Opaque struct | User-defined struct layout |
| `Map<K,V>` | Opaque `i8*` | Runtime hash map handle |
| `Option<T>` / `Result<T,E>` | `{ i1, T }` or `{ i1, T, E }` | Tagged union |

Ownership qualifiers (`own`, `mut`, `ref`, `borrow`) and lifetimes are compile-time only; they do not affect LLVM IR generation.

---

## 6. Built-in Function ABI

Each built-in is exposed as an `extern "C"` function in the LPPO runtime (`lppo-rt`). Naming convention: `lppo_rt_<name>`. Arguments and return values follow the System V AMD64 ABI (integers in `rdi, rsi, rdx, rcx, r8, r9`; floats in `xmm0-xmm7`; return in `rax`/`xmm0`).

Examples:
- `lppo_rt_print_value(i64 type_tag, ...value)` → `void`
- `lppo_rt_input(i64 type_tag, i8* prompt)` → `i8*` (string result) / `i64` (int result) — tagged union via out-param or struct return
- `lppo_rt_add(i64, i64)` → `i64`
- `lppo_rt_str_concat(i8*, i8*)` → `i8*`
- `lppo_rt_alloc(i64 size)` → `i64` (address)
- `lppo_rt_dealloc(i64 addr)` → `void`
- `lppo_rt_array_make(i64 len)` → `%lppo.array*`
- `lppo_rt_array_get(%lppo.array*, i64 idx)` → `i64` (boxed value)

Error convention: Unrecoverable errors (type mismatch, div by zero, OOM) call `lppo_rt_panic(i8* message)` which aborts the process. Recoverable errors (e.g. `file_exists`) return a tagged value.

---

## 7. Control Flow Summary

| Construct | LLVM Blocks | Branch Instruction |
|---|---|---|
| Straight-line | single block | fall-through |
| `If` | entry, then, elif*, else, merge | `br i1` |
| `While` | cond, body, exit | `br i1` (cond→body/exit), `br` (body→cond) |
| `For` | init, cond, body, incr, exit | `icmp` + `br` |
| `Loop` | body, exit | `br` (unconditional back-edge) |
| `Break` | — | `br` to loop exit |
| `Continue` | — | `br` to loop cond/body |
| `Return` | — | `ret void` / `ret <type>` |
| `Proc call` | — | `call` + fall-through |

---

## 8. Frontend-to-Bytecode Guarantees

The compiler in `src/compiler.rs` performs these transformations, which the bytecode preserves:

1. **Types are erased:** `TypeAnnotation`, `OwnershipKind`, lifetimes are compile-time only; `Operation` carries no type info.
2. **Heap tracking:** `CompileCtx` tracks heap-allocated vars per scope; inserts `Dealloc` at scope/proc exit.
3. **Header-only procs skipped:** Procedures with empty bodies are not compiled into bytecode.
4. **Qualified names flattened:** `module::proc` → `"module::proc"` string key.
5. **Move/Copy → Let:** Both compile to `Operation::Let(target, Expr::Var(source))`.
6. **IndexedAssignment → Nop:** Arrays are immutable in the simplified runtime.

The LLVM AOT backend must read `ProcedureBody` directly and produce IR matching these semantics exactly.

---

## 9. Validation Checklist

For each `Operation` variant, the differential test suite must confirm:
- [ ] Interpreter output == Native executable output
- [ ] Edge cases (empty arrays, div by zero, negative indexing, empty strings)
- [ ] Control flow propagation (Break/Continue/Return through If/While/For/Loop/Scope)
- [ ] Type correctness (Int vs Float vs Bool vs Str operations)

No bytecode instruction may be silently skipped or approximated by the native backend; unsupported instructions must emit a clear codegen error.
