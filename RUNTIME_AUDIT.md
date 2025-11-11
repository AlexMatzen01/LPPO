# Runtime Audit — `src/runtime.rs`

Date: 2025-11-11

This document summarizes issues, bugs, correctness concerns, performance problems, API/UX suggestions, testing gaps, and prioritized recommended fixes for `src/runtime.rs` (Executor + evaluator). The goal is to provide a clear, actionable plan to improve correctness, safety, maintainability, and performance.

---

## Quick summary (high level)

- The runtime is largely functional and readable, but there are several correctness bugs (string/char vs byte inconsistency, exponent handling, serde error mapping), safety/robustness issues (panics via `.expect()` inside threads, excessive cloning, error surface mapping), concurrency behavior ambiguities (Parallel clones env and silently panics on eval error), and API/design gaps (poor error variant choice for serde, unclear method semantics like `push` mutability).
- Tests are minimal; add unit tests for arithmetic edge-cases, string indexing, parallel error handling, method calls, and control-flow (return/break/continue) semantics.
- Improvements fall into three priority bands: Critical (fix correctness/unsound behavior / panics), High (error handling, API clarity, tests), and Medium (performance, docstrings, style/clippy).

---

## Concrete issues (by priority)

### Critical

1) Binary exponent operation uses `i64.pow(b as u32)` with no check for negative exponent or huge exponents.
   - Problem: casting a negative `b` to `u32` produces a large exponent; behavior is wrong for negative exponents. Large exponents can overflow or produce surprising results.
   - Impact: incorrect computation or panic/overflow depending on build profile.
   - Fix: Reject negative exponents with a `TypeError` (or support floats) and limit exponent range. Example: check `if b < 0 { return Err(LppoError::TypeError("negative exponent".into())); }` then use `a.pow(b as u32)` with a safe bound.

2) `Operation::Parallel` uses `expect("eval in thread")` inside thread closures.
   - Problem: any evaluation error inside a parallel thread will cause that thread to panic; `expect` will panic, which in scoped threads will unwind to the main thread and cause test or process failure. Errors are not surfaced as `LppoResult`.
   - Impact: runtime crash on user code errors in parallel expressions.
   - Fix: evaluate expressions in the thread as `match eval_expr(...) { Ok(v) => println!(...), Err(e) => eprintln!(...) /* or send back via channel */ }`. Prefer returning Result from each spawned job and aggregate results. Consider collecting thread results and converting into a `LppoResult` for the caller.

3) Inconsistent string semantics: `len` method uses `s.len()` (byte length) while indexing uses `s.chars().nth(i)` (char index).
   - Problem: For unicode strings, `len()` returns bytes but indexing uses character positions — inconsistent API that will confuse users and cause off-bytes errors.
   - Impact: wrong behavior for multibyte characters; indexing and length mismatch.
   - Fix: choose a policy: either use bytes everywhere (then indexing by bytes) or chars (prefer char count/characters). Document choice and implement `len` as `s.chars().count()` if the intended indexing is by characters.

4) Error mapping for serde operations uses `LppoError::UnknownProcedure`.
   - Problem: `serialize_procedures` and `executor_from_bytes` map any serde error to `UnknownProcedure`, which is misleading and loses important error context.
   - Impact: poor diagnostics when loading/serializing compiled procedures.
   - Fix: Add or use a `LppoError::Serialization(String)` (or `LppoError::Io`/`SerdeError`) variant and map serde errors to that.

5) `BinaryOp::Div` and `Mod`: `Mod` implementation uses `%` without handling negative modulo semantics if relevant. `Div` rejects division by zero with `TypeError` — consider using a dedicated error variant for arithmetic errors.
   - Fix: consider `LppoError::Arithmetic(String)` for arithmetic operations.


### High

6) `MethodCall` semantics for `push` / `pop` operate on moved `Value`s and return new `Value` rather than mutating variables.
   - Problem: Users may expect `a.push(x)` to mutate `a` in-place. Current code moves target and returns a new array (and doesn't update caller's variable), which can be surprising.
   - Impact: surprising semantics and API inconsistency.
   - Fix: Decide on functional vs in-place semantics. If in-place mutation is desired, introduce mutable storage semantics (e.g., `Value::Array(Rc<RefCell<Vec<Value>>>)`) or add specific `set` operations that can update variable binding.

7) `Index` negative index handling computes `i = (len as i64 + idx) as usize` then uses `arr.get(i)`. While bounds fall-through is safe (get returns None), the cast from negative to usize can produce huge usize; prefer an explicit bounds check for clarity and avoid potential logic traps.
   - Fix: compute signed index as i64, check 0 <= i < len as i64, then cast to usize.

8) `value_to_string` and `Display`/`ToString` implementations: no `Display` impl for `Value`, forcing manual conversions across codebase.
   - Fix: implement `impl Display for Value` to standardize stringization.

9) Excessive cloning of `env`, `Array`, and `Expr` in `Parallel` and other places.
   - Problem: cloning whole environment per thread is heavy and can be surprising in memory-constrained contexts.
   - Fix: Use references where possible, or `Arc`/`Rc` for shared readonly data, and `Arc<Mutex<...>>` if mutation across threads is required. Document `Parallel` as a read-only evaluation snapshot.

10) `serialize_procedures` returns `Vec<u8>` but the error variant is wrong (see #4). Also `ProcedureBody` contains `Expr`/`Operation` which are `Serialize`/`Deserialize` — be sure their serde config is stable and documented.


### Medium / Nice-to-have

11) Add more unit tests: arithmetic (overflow, negative pow, division/mod by negative), Indexing (negative indices, out-of-bounds), MethodCall edge cases (split with wrong arg count), Parallel error handling, Control flow (return inside nested blocks), Call arity mismatch, recursion depth.

12) Add `#[doc]` comments for all public API: `Executor`, `ProcedureBody`, `Operation`, `serialize_procedures`, `executor_from_bytes`.

13) Use `clippy` and `cargo fmt`, add CI to run them. Fix common clippy lints (e.g., ownership, unnecessary clones).

14) Consider enabling `serde` feature flags to ensure deterministic serialization ordering if needed (HashMap order isn't stable across runs unless using BTreeMap). If cross-process determinism matters, serialize a BTreeMap or sort keys.

15) More descriptive error types and `Display` / `std::error::Error` impl for `LppoError` to improve user diagnostics.

16) Avoid printing directly from runtime (`println!`) — instead surface output via an I/O trait or callback so embedding programs can capture/redirect output (helps testing). Provide an `Executor::with_output` or take an `impl Write` or callback in `Executor::run`.

17) `Parallel` prints results in arbitrary order — document that or provide an ordered parallel mode.

18) Replace `HashMap<String, Value>` env passing with a small `Env` wrapper type implementing utilities (scoping, shadowing, cloning-on-write), and document variable scoping behavior.


## Suggested prioritized plan

1. (Critical) Fix panics and error mapping:
   - Replace `expect` in `Parallel` with error-returning behavior or proper handling. Aggregate thread results or at least print errors instead of panicking.
   - Add `LppoError::Serialization` (or `Serde`), and map serde_json errors to that in `serialize_procedures` and `executor_from_bytes`.
   - Fix `Pow` negative exponent handling.

2. (High) Make string semantics consistent:
   - Decide on char-based string indexing or byte-based. Implement `len` accordingly and add tests.
   - Implement explicit index bounds checks.

3. (High) Improve API semantics for methods:
   - Decide whether method calls mutate or return new values. If mutation, use `Rc<RefCell<...>>` for arrays/strings where mutation is allowed, and update language docs.

4. (High) Add tests for all control flow constructs and edge cases.

5. (Medium) Replace prints with an injectable `Output` interface and make `Executor::run` accept an output sink.

6. (Medium) Profile and reduce cloning where it is costly; consider `Arc` for shared immutable data.

7. (Nice) Add docs, clippy, CI, and implement `Display` for `Value`.


## Example code snippets (illustrative)

- Replace `expect` in `Parallel` with error-safe handling (sketch):

```rust
// inside Operation::Parallel branch
let (tx, rx) = std::sync::mpsc::channel();
std::thread::scope(|scope| {
    for e in exprs.iter().cloned() {
        let tx = tx.clone();
        let mut snapshot = env.clone();
        scope.spawn(move || {
            let res = eval_expr(&e, &mut snapshot);
            tx.send(res).expect("send to main thread");
        });
    }
});
// collect results and return error if any
for r in rx.try_iter() {
    match r {
        Ok(v) => println!("{}", value_to_string(&v)),
        Err(err) => return Err(err),
    }
}
```

- Check negative exponent explicitly:

```rust
(Value::Int(a), Value::Int(b), BinaryOp::Pow) => {
    if b < 0 {
        return Err(LppoError::TypeError("negative exponent".into()));
    }
    let e = b as u32;
    Ok(Value::Int(a.pow(e)))
}
```

- String `len` as characters:

```rust
(Value::Str(s), "len") => Ok(Value::Int(s.chars().count() as i64)),
```


## Tests to add (minimum set)

- pow_negative_exponent -> assert error
- index_negative_with_unicode -> correct char indexing
- method_push_mutation -> clarify and assert chosen semantics
- parallel_eval_error -> thread errors are surfaced as LppoError not panic
- serialize_roundtrip -> serialize then deserialize and run a simple procedure
- control_flow_return_in_nested_block -> ensure return bubbles out


## Tooling & maintenance

- Add `cargo fmt` and `cargo clippy` checks to CI.
- Add unit tests under `src/runtime.rs`'s `#[cfg(test)]` with a test harness helper to build small `ProcedureBody`s.
- Consider adding a small `examples/` runtime sample that exercises parallel and complex string/array behaviour.


## Next steps I can take (I can implement any of these on request)

- Implement the critical fixes (pow negative check, serde error variant, replace `expect` in parallel with robust aggregation).
- Add the recommended unit tests and run `cargo test`.
- Introduce an `Output` trait to decouple printing from runtime.
- Run `cargo clippy` and fix top lints.

If you want, I'll start with the critical fixes and tests now (I can open PR-style edits). Tell me which fixes you want first or say "fix critical" and I'll implement them and run the test/build cycle.

---

End of audit.
