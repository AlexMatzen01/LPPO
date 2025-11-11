# LPPO — Repository Audit & Recommended Improvements

Date: 2025-11-11

This document records the automated review I ran over the LPPO repository, the issues I found, risks, and prioritized remediation suggestions with concrete examples. It is intended as a developer-friendly checklist you can act on incrementally.

---

## Quick summary

- `cargo check` completed successfully but reported 4 warnings (unused imports/variables).
- I read core files: `Cargo.toml`, `readme.md`, `src/{lib.rs,main.rs,compiler.rs,parser.rs,lexer.rs,runtime.rs,error.rs}`.
- Main pain points: a few correctness risks in runtime (exponent handling, parallel error handling), lexer/tokenization quirks, and error reporting/serialization mapping issues.

Severity legend:
- P0 — High priority / correctness or crash risk
- P1 — Medium priority / developer experience
- P2 — Design/feature improvements

---

## Build output (summary)

- Warnings observed from `cargo check`:
  - `src/compiler.rs`: unused import `Expr`.
  - `src/lexer.rs`: unused variable `found_end` and an unused assignment to it.
  - `src/runtime.rs`: unused variable `target` in `Expr::FieldAccess` match arm.

(Full `cargo check` output is in the CI logs; these are small but worth fixing to keep the tree clean.)

---

## Detailed issues and recommendations

The following list groups issues by priority. Each item shows: location, what is wrong, why it's risky, and a recommended fix (concrete when possible).

### A — Correctness / Crash Risks (P0/P1)

1) Unsafe/incorrect exponentiation handling (P0)
- Location: `src/runtime.rs` — `Expr::BinaryOp`, `BinaryOp::Pow` branch.
- Problem: code uses `a.pow(b as u32)` where `b` is `i64`. Negative exponents cast to huge `u32` values and will either compute wrong results or panic/overflow. Very large exponents cause extreme CPU or overflow.
- Fix: reject negative exponents and limit exponent size. Use `checked_pow` and return a `LppoError::TypeError` when invalid.
  - Example:
    - if `b < 0` => Err(TypeError("negative exponent not supported"))
    - else use `a.checked_pow(b as u32)` and return TypeError on overflow or if `b` too large (e.g. > 63).

2) Parallel branch may panic and abort process (P0)
- Location: `src/runtime.rs` — `Operation::Parallel` handler uses `expect("eval in thread")`.
- Problem: `expect` in spawned thread causes panic (which may abort the process depending on configuration) instead of returning a recoverable `LppoError`. Also concurrent `println!` calls can interleave output.
- Fix: evaluate branches with proper error propagation. Collect `Result<Value, LppoError>`, return the first error (or aggregate). Use a `Mutex`/channel for ordered printing or serialize output after collection. Avoid `expect`.

3) Serialization/Deserialization map to wrong error (P1)
- Location: `src/runtime.rs` — `serialize_procedures` and `executor_from_bytes` map serde errors to `LppoError::UnknownProcedure`.
- Problem: hides the real cause (serde error) and mislabels it.
- Fix: add error variants such as `SerializationError(String)` and `DeserializationError(String)`, map serde errors to these.

4) Lexer comment handling and `position` semantics (P1)
- Location: `src/lexer.rs` — `skip_whitespace_and_comments`.
- Problems:
  - `position` is updated to `self.current` while skipping whitespace/comments. This makes token positions point at the token end rather than the start, reducing error-report quality.
  - `found_end` is assigned but never read.
- Fix:
  - Set `self.position` to the start-of-token before lexing a token, not during whitespace skipping. Remove or use `found_end` properly.

5) `!` handling in lexer (P1)
- Location: `src/lexer.rs` — handling of `b'!'` returns error unless followed by `=`.
- Problem: The language currently uses `not` keyword; however `!` is a common unary operator. Rejecting `!` can be surprising. If `!` is intended to be invalid, document it. Otherwise adjust to emit `Token::Not` when not followed by `=`.
- Fix: decide syntax; implement `Token::Not` on standalone `!`.

6) Boolean literal tokenization (P1)
- Location: `src/lexer.rs` — `true`/`false` are returned as `Token::Identifier("true")` rather than dedicated boolean tokens.
- Problem: relying on identifier token handling is fragile.
- Fix: add `Token::Bool(bool)` and emit it for `true`/`false`, or let parser convert identifiers to boolean Expr explicitly and clearly.

7) Index arithmetic and bounds logic (P1)
- Location: `src/runtime.rs` — `Expr::Index` handling for arrays and strings.
- Problems:
  - Negative indexing is converted via `(arr.len() as i64 + idx) as usize`, which may yield incorrect results if `idx` is less than `-(arr.len() as i64)` and yields confusing casts.
  - String indexing uses `s.chars().nth(i)` (codepoint-based) — acceptable, but should be documented and performance noted.
- Fix: perform safe checked arithmetic for negative indices, explicitly check for out-of-range and return a clear error.

8) Field access unimplemented but unused variable warning (P1)
- Location: `src/runtime.rs` — `Expr::FieldAccess { target, field }` arm returns TypeError and `target` is unused.
- Fix: rename to `_target` to silence warning or implement struct support.

9) Compiler unused import (P2)
- Location: `src/compiler.rs` — unused `Expr` import.
- Fix: remove the unused import.

10) Error positions are raw byte offsets (P1)
- Location: `src/error.rs` & `src/lexer.rs`.
- Problem: `LppoError` variants use `position: usize` but do not provide line/column, which users expect.
- Fix: track line & column in the lexer or provide a helper to map offsets to (line, column) and include that in error prints.


### B — Developer experience & quality (P1)

1) No CI/workflow or formatting/lint enforcement
- Add a GitHub Actions workflow to run `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, `cargo test`, and `cargo check`.

2) Missing `LICENSE` and crate metadata
- Add a `LICENSE` file (MIT or Apache-2.0 preferred) and update `Cargo.toml` with `license`, `description`, `repository`, and `authors` fields.

3) Tests: limited coverage
- Add unit tests covering exponent edge cases, array/string indexing, `parallel` error behavior, and serialization roundtrip.

4) Serialization format/versioning
- Add a version byte or schema version in the serialized payload for forward/backward compatibility.

5) README vs docs
- `readme.md` exists; consider adding uppercase `README.md` or linking it from `docs/index.html`. Ensure docs are discoverable.

6) Logging
- Consider `log`/`tracing` integration for structured debug output; replace ad-hoc `--debug` prints.


### C — Design & feature suggestions (P2/P3)

- Define procedure return semantics and add tests (how nested calls return values; stack discipline).
- Standard library: add small builtins (IO, math) implemented in Rust for performance and to keep the runtime small.
- Parallel semantics: document determinism guarantees (if any), and consider a task runner abstraction.
- Add LSP server later for editor integration.

---

## Concrete quick fixes (I can apply these)

The following changes are low-risk and recommended to implement immediately. If you want I can apply these now and run tests.

1. Remove unused import in `src/compiler.rs`:
   - change `use crate::ast::{Procedure, Program, Statement, Expr};` to
     `use crate::ast::{Procedure, Program, Statement};`.

2. Fix the `found_end` unused variable in `src/lexer.rs`.
   - remove the variable or rename to `_found_end`.

3. Rename `target` to `_target` in `Expr::FieldAccess` arm in `src/runtime.rs` to silence the warning.

4. Add proper serialization/deserialization error mapping in `src/error.rs` and use it in `src/runtime.rs`.

5. Replace `expect("eval in thread")` in `Operation::Parallel` with a join that propagates `LppoError`.

6. Add `LICENSE` and `README.md` (capitalized) or symlink `readme.md` -> `README.md`.

7. Add a minimal GitHub Actions workflow (format + clippy + test).

---

## Prioritized action plan (short list)

Immediate (apply now — I can do these):
- [ ] Fix warnings (unused imports/vars) — low risk.
- [ ] Add `SerializationError`/`DeserializationError` and update usage — low risk.
- [ ] Replace `expect` in parallel branch with error propagation — medium risk.
- [ ] Add `LICENSE` file and update `Cargo.toml` metadata — trivial.

Short-term (after immediate):
- [ ] Add unit tests for exponent, indexing, parallel error propagation, and serialize/deserialize roundtrip.
- [ ] Add CI (GitHub Actions) with formatting/linting/testing.
- [ ] Improve lexer position to track line/column and update error formatting.

Mid-term:
- [ ] Harden numeric operations (checked ops), define semantics for overflow.
- [ ] Improve runtime concurrency (ordered/structured output).
- [ ] Document serialized format version and add backwards compatibility support.

Long-term:
- [ ] LSP server, stdlib, packaging improvements, performance profiling.

---

## Suggested next step (pick one)

1. I apply the low-risk fixes listed above (I will update files and run `cargo check` and unit tests).
2. I generate a PR with the fixes and a GitHub Actions workflow.
3. You prefer a different set of changes — tell me which and I will implement them.

Tell me which option you want. If you pick (1), I'll implement the small code edits now and run `cargo check` + `cargo test` where applicable. I will update the todo list and report results.

---

## Appendix — quick references

- Key files examined:
  - `Cargo.toml`
  - `readme.md` (project README)
  - `src/lib.rs`, `src/main.rs`, `src/compiler.rs`, `src/parser.rs`, `src/lexer.rs`, `src/runtime.rs`, `src/error.rs`
- Build command used:

```bash
cargo check
```

- Helpful small commands to run locally:

```bash
# format and check
cargo fmt --all -- --check
# lints
cargo clippy -- -D warnings
# build & tests
cargo test
```


---

End of audit.
