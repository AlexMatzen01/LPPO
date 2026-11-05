# LPPO Ring-0 Advanced Syntax Plan

**Plan file:** `.kilo/plans/1793909576-ring0-syntax-plan.md`
**Timestamp:** 1793909576
**Status:** Implementation-ready

## Goal

Add an advanced syntax feature to lppo: a `ring 0;` declaration that must appear first in a source file, which gates access to ring-0 (kernel privilege) operations such as port I/O, MSR reads/writes, CPUID, CLI/STI, and HLT.

## Decisions (resolved)

| Decision | Resolution |
|---|---|
| Declaration syntax | `ring 0;` file-level prefix directive, must be the **first** declaration (before any `use`, `proc`, etc.) |
| Operation exposure | Built-in functions callable as normal expressions, only registered/type-checked when `ring 0` is declared |
| Host (interpreted/bundled) runtime | Ring-0 built-in calls return a clear type/runtime error (port I/O unavailable from userspace Linux) |
| Bare metal runtime | Implement via `core::arch::asm!` inline x86 assembly (`out`, `in`, `rdmsr`, `wrmsr`, `cpuid`, `cli`, `sti`, `hlt`) |
| Typechecker gate | Register ring-0 built-ins only when `Program.ring0 == true`; calling without the declaration → unknown-procedure type error |

## Affected files

1. `src/lexer.rs` — add `Token::Ring` keyword; map `"ring"` → `Token::Ring`
2. `src/ast.rs` — add `pub ring0: bool` field to `Program`
3. `src/parser.rs` — parse `ring 0;` as the first directive in `parse_program`; enforce first-position; set `program.ring0`
4. `src/lib.rs` — thread `ring0` through `resolve_program`/`merge_program` so imports preserve the flag
5. `src/typechecker.rs` — register ring-0 built-ins conditionally on `program.ring0`
6. `src/runtime.rs` — add ring-0 built-in fast-path in `eval_expr` (`Expr::Call`) returning a user-mode error
7. `src/compiler.rs` — no change needed (built-ins are regular `Expr::Call`; compile as-is)
8. `lppo-bare-rt/src/main.rs` — add ring-0 built-in handling in bare-metal `eval_expr` using inline asm; add `"Call"` case for built-in `Expr::Var` names
9. `examples/ring0.lppo` (new) — example demonstrating `ring 0;` + port I/O
10. `tests/` (new) — unit/integration tests for parse position gate, type-check gating, and runtime error

## Ring-0 built-in signatures

| Function | Signature | Effect |
|---|---|---|
| `outb` | `(Int64, Int64) -> Void` | x86 `outb` port byte write |
| `outw` | `(Int64, Int64) -> Void` | x86 `outw` port word write |
| `outl` | `(Int64, Int64) -> Void` | x86 `outl` port dword write |
| `inb` | `(Int64) -> Int64` | x86 `inb` port byte read |
| `inw` | `(Int64) -> Int64` | x86 `inw` port word read |
| `inl` | `(Int64) -> Int64` | x86 `inl` port dword read |
| `rdmsr` | `(Int64) -> Int64` | x86 `rdmsr` MSR read |
| `wrmsr` | `(Int64, Int64) -> Void` | x86 `wrmsr` MSR write |
| `cpuid` | `(Int64) -> Arr<Int64>` | x86 `cpuid` → `[eax, ebx, ecx, edx]` |
| `cli` | `() -> Void` | x86 `cli` (disable interrupts) |
| `sti` | `() -> Void` | x86 `sti` (enable interrupts) |
| `hlt` | `() -> Void` | x86 `hlt` (halt) |

## Data flow

1. Source `ring 0; ... proc main() { outb(0x3f8, 65); }` → lexer tokens `Ring, Number(0), Semicolon, ...`
2. Parser parses `ring 0;` first, sets `Program.ring0 = true`, then parses procedures normally
3. `resolve_program` / `merge_program` carry `ring0` across imported modules
4. Typechecker registers ring-0 built-ins only if `program.ring0`; type-checks calls against signatures
5. Compiler emits regular `Operation::Call` (no new op variants) → serialized to JSON bytecode
6. Host runtime `eval_expr`: matches ring-0 built-in names → returns `type_error("ring-0 operation not available in user mode")`
7. Bare metal runtime `eval_expr`: matches ring-0 built-in names → executes inline x86 asm

## Failure modes & handling

- `ring 0;` not first → parser error: `"ring 0 must be declared before any other declaration"`
- Ring-0 built-in called without `ring 0;` → typechecker unknown-procedure error (not registered)
- Ring-0 built-in called in host/bundled mode → runtime error: not available in user mode
- Bare metal: `cli`/`sti`/`hlt`/`out`/`in`/`rdmsr`/`wrmsr`/`cpuid` require ring 0; executing at lower privilege faults (expected, hardware-enforced)

## Validation

- `cargo build` — workspace compiles (lppo + lppo-bare-rt)
- `cargo test` — all existing + new tests pass
- `cargo clippy` — no new warnings (if clippy installed)
- `cargo fmt --check` — formatted
- New tests:
  - `ring 0;` at top parses and sets `ring0 = true`
  - `ring 0;` after a `proc` → parser error
  - `outb(...)` without `ring 0;` → typechecker error
  - `outb(...)` with `ring 0;` → type-checks OK
  - Host runtime calling `outb(...)` → runtime error (user mode)
- Example: `examples/ring0.lppo` compiles and runs (demonstrates syntax; host run errors gracefully, bare-metal build produces `x86_64-unknown-none` binary)

## Open questions

None — all design decisions resolved. Implementation can proceed directly.
