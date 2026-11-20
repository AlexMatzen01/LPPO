# LPPO Packages

LPPO ships with a package manager (`lppo pkg`) that resolves, installs, builds, and links LPPO packages. Packages are Rust crates exposing an LPPO-facing FFI interface, registered in a repository index and installed into a local package store.

## Package layout

A package is a directory containing:

- `package.json` — package manifest (identifier, version, dependencies, build config)
- `Cargo.toml` — Rust crate manifest (build system: cargo)
- LPPO-facing native functions compiled into a dynamic/static library

## package.json manifest

```json
{
  "schema_version": 1,
  "package": {
    "identifier": "mypackage",
    "display_name": "My Package",
    "version": "1.0.0",
    "author": "Name <email>",
    "description": "A useful LPPO package",
    "license": "MIT",
    "repository": "https://github.com/user/mypackage",
    "lppo_version": ">=5.0.0",
    "dependencies": {
      "otherpkg": "^1.0.0"
    },
    "ports": [],
    "build": {
      "system": "cargo",
      "manifest": "Cargo.toml",
      "features": [],
      "release": true
    }
  }
}
```

Fields:
- `identifier` — unique package ID used as the import/install key
- `version` — SemVer version string
- `dependencies` — package identifier → version requirement (e.g. `^1.0.0`, `>=1.2.3`)
- `build.release` — build in release mode (default `true`)

## Repositories

Repositories declare where LPPO finds packages. Stored in `~/.LPPO/repositories.json` (or project-local `.lppo/repositories.json`).

```json
{
  "schema_version": 1,
  "repositories": [
    { "name": "official", "url": "https://github.com/lppo/packages.git", "type": "git", "description": "Official LPPO packages" }
  ]
}
```

Repository types: `git` or `path`.

## CLI reference

`lppo pkg <subcommand> [args]`

| Subcommand | Description |
|------------|-------------|
| `search <query>` | Search available packages |
| `install <package>` | Install a package (and regenerate the index) |
| `remove <name>` | Remove an installed package |
| `list` | List installed packages |
| `info <name>` | Show package details (installed or available) |
| `build <name>` | Build a package from source |
| `update [name]` | Update a package (or all) |
| `repo list` | List configured repositories |
| `repo add <name> <url> <git\|path>` | Add a repository |
| `repo remove <name>` | Remove a repository |

### Examples

```sh
# Add the official repository
lppo pkg repo add official https://github.com/lppo/packages.git git

# Search for a package
lppo pkg search crypto

# Install a package
lppo pkg install sha256

# List installed packages
lppo pkg list

# View package info
lppo pkg info sha256

# Remove a package
lppo pkg remove sha256
```

## Importing packages in LPPO source

Packages exposing native functions resolve against the installed package database. Use `use <package>;` to bring a package's interface into scope (the first path segment is checked against installed packages; on-disk file resolution is skipped for installed packages).

```lppo
use sha256;

pub proc main() -> Void {
    out sha256("hello world");
}
```

## Native package interface

Native packages declare exported functions with LPPO-facing types:

| LPPO type | FFI representation |
|-----------|-------------------|
| `Int64` | `i64` |
| `Bool` | `i1` (zero-extended to `i64` in FFI) |
| `Str` | `*mut i8` (null-terminated, heap-allocated) |
| `Void` | `()` |

Up to two string parameters are supported when `string_args` is true; they occupy the first two argument slots using the `NativeFnString` calling convention.

## Package store

- Global: `~/.LPPO/`
  - `packages.json` — package library index
  - `installed.json` — local installation database
  - `repositories.json` — configured repositories
  - `bin/lppo` — the LPPO binary (auto-installed on first run)
- Project-local: `<project>/.lppo/`
  - `repositories.json` — copied from global on `lppo project init`

Schema versions: package library `1`, installed database `1`.

## Version requirements

Version requirements follow SemVer ranges:
- `*` or empty — any version
- `=1.2.3` or `1.2.3` — exact
- `^1.2.3` — `>=1.2.3, <2.0.0` (major locked when major > 0)
- `~1.2.3` — `>=1.2.3, <1.3.0`
- `>=1.2.3` / `>1.2.3` / `<=1.2.3` / `<1.2.3` — comparison

## Project integration

`lppo.toml` manifest:

```toml
[project]
name = "myapp"
version = "0.1.0"
edition = "2021"

[build]
entry = "src/main.lppo"
output = "target/{name}"
strip = false
obfuscate = false
native = true
opt_level = 2

[dependencies]
path = ["../somepkg"]
```

`lppo project init <name>` scaffolds a new project with `lppo.toml`, `src/main.lppo`, `.gitignore`, `.lppo/` (with `repositories.json`), and a git repository.
