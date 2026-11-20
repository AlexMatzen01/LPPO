# LPPO — Legible Programming Platform eXtensions

**Version:** 0.1.10-alpha.4 (alpha)

LPPO is a statically typed systems programming language with zero-cost abstractions, explicit memory/ownership, and first-class concurrency. It compiles to portable bytecode executed by an embedded interpreter, transpiles to C for native executables, and targets bare-metal x86_64 kernels.

## Language at a glance

- File extensions: `.lppo`, `.lpph`, `.lph`
- Entry point: `pub proc main() -> Void { ... }`
- Output: `out "Hello";`
- Input: `input name: Str;`
- Variables: `let x: Int64 = 42;`, `let mut y: Int64 = 1;`
- Memory: `alloc`/`dealloc`, `ref`/`deref`, `move`/`copy`, `borrow`/`own`, `mut`, heap/stack
- Control flow: `if`/`elif`/`else`, `while`, `for ... in`, `loop`, `break`, `continue`, `match`
- Types: `Int64`, `Bool`, `Str`, `Arr<T>`, `Void`, `Option<T>`, `Result<T, E>`, `Map<K, V>`, `Set<T>`, `Fn(...) -> R`, structs, enums, traits, generics, lifetimes
- Concurrency: `async`/`await`, `spawn`/`join`, `parallel`

## Example

```lppo
pub proc main() -> Void {
    out "Name: ";
    input name: Str;
    out "Hello, ", name, "!";
}
```

## Toolchain

| Command | Purpose |
|---------|---------|
| `lppo <source.lppo>` | Run a source file via the bytecode interpreter |
| `lppo run <source.lppo>` | Explicit run mode |
| `lppo build <source.lppo> -o <out>` | Transpile to C and produce a native executable |
| `lppo compile <out> <source.lppo>` | Bundle bytecode into a self-contained executable |
| `lppo compile-bare-metal <out> <source.lppo>` | Compile to a bare-metal x86_64 kernel |
| `lppo project init <name>` | Scaffold a new project (`lppo.toml`, `src/main.lppo`) |
| `lppo project build` / `run` / `format` / `debug` | Project-level commands |
| `lppo pkg install|search|list|info|remove|build|update` | Package management |
| `lppo fmt` | Format source files |
| `lppo version` / `--version` | Print version |

## Runtimes

- `lppo-rt` — embedded C FFI runtime (print, input, heap, arrays, maps, sets, JSON, strings, math, color, file I/O, system)
- `lppo-bare-rt` — bare-metal kernel runtime for x86_64 (VGA, serial, keyboard, paging, GDT/IDT, port I/O)

## Installation

```sh
cargo build --release
# copies the binary to ~/.LPPO/bin/lppo and adds it to PATH on first run
```

## Documentation

See `docs/index.html` for the full language reference, standard library, and interactive guides.

## License

See the repository for license information.
