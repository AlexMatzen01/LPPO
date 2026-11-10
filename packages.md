# LPPO Package System

LPPO uses a repository-driven package system. Instead of hardcoding package
paths in a central registry, you declare repositories in `~/.LPPO/repositories.json`,
and each package carries its own `package.json` manifest.

## Directory Layout

```
~/.LPPO/                  # Global LPPO package data
  repositories.json       # Declared package repositories
  packages.json           # Generated index of available packages
  installed.json          # Tracked installed packages
  lppo_modules/           # Installed package source code & builds
    <package-name>/       # Each installed package lives here
      Cargo.toml
      src/
      package.json        # The package manifest (copied from source)
      target/             # Build artifacts
```

<project>/
  .lppo/                  # Project-local package data (when inside an LPPO project)
    lppo_modules/         # Project-local installed packages

## `repositories.json`

Declares where LPPO looks for packages. Each entry has:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique repository identifier |
| `url` | string | Git URL or filesystem path |
| `type` | `"git"` or `"path"` | Repository kind |
| `description` | string | Human-readable description |

Example:

```json
{
  "schema_version": 1,
  "repositories": [
    {
      "name": "lppo-official",
      "url": "https://github.com/lppo-lang/packages",
      "type": "git",
      "description": "Official LPPO package repository"
    },
    {
      "name": "local",
      "url": "/home/alex/Documents/lppo-packages",
      "type": "path",
      "description": "Local package directory"
    }
  ]
}
```

Manage repositories with:

```sh
lppo pkg repo list
lppo pkg repo add <name> <url> <git|path>
lppo pkg repo remove <name>
```

## `package.json` (package manifest)

Every LPPO package must include a `package.json` at its root. It declares:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `identifier` | string | Yes | Unique package name (used as import key) |
| `display_name` | string | Yes | Human-readable display name |
| `version` | string | Yes | SemVer version (e.g. `1.0.0`) |
| `author` | string | Yes | Name and email |
| `description` | string | No | Short description |
| `license` | string | No | SPDX license identifier |
| `repository` | string | No | Source repository URL |
| `lppo_version` | string | No | Minimum LPPO version (e.g. `>=5.0.0`) |
| `dependencies` | object | No | Map of package identifier -> version req |
| `build` | object | No | Build configuration |

Example:

```json
{
  "schema_version": 1,
  "package": {
    "identifier": "mypackage",
    "display_name": "My Package",
    "version": "1.0.0",
    "author": "Your Name <you@example.com>",
    "description": "A short description of your package",
    "license": "MIT",
    "repository": "https://github.com/yourname/mypackage",
    "lppo_version": ">=5.0.0",
    "dependencies": {},
    "build": {
      "system": "cargo",
      "manifest": "Cargo.toml",
      "features": [],
      "release": true
    }
  }
}
```

## How Packages Are Resolved

1. `lppo pkg install <identifier>` reads `~/.LPPO/repositories.json`.
2. For each repository, LPPO checks whether a package with that `identifier`
   exists (by reading its `package.json`).
3. The package source is cloned/copied into `~/.LPPO/lppo_modules/<identifier>/`.
4. An entry is written to `~/.LPPO/packages.json` recording the install path.
5. The package is built with Cargo and tracked in `~/.LPPO/installed.json`.

For project-local installs (`lppo project pkg install`), packages go into
`<project>/.lppo/lppo_modules/` instead.

## Creating a Package

1. Initialize a Cargo project:

   ```sh
   cargo new mypackage
   cd mypackage
   ```

2. Add `package.json` at the project root (see example above).

3. Make sure your `Cargo.toml` produces a `cdylib` or `rlib` that LPPO can load.

4. Publish the package to a Git repository or filesystem path.

5. Add the repository to your LPPO config:

   ```sh
   lppo pkg repo add myrepo https://github.com/you/mypackages.git git
   ```

6. Install and use:

   ```sh
   lppo pkg install mypackage
   ```
