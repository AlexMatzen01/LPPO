# LPPO Language Support

Syntax highlighting, formatting, and language support for the LPPO programming language in Visual Studio Code.

## Features

- Syntax highlighting for `.lppo` and `.lpph` files
- LPPO formatter (built-in and external binary)
- Symbol navigation: document outline (Outline view), Go to Definition, Find References, and Hover info for classes, structs, enums, traits, modules, impl blocks, procedures, methods, fields, and enum variants
- Run LPPO scripts directly from VS Code

## Requirements

- **LPPO executable** (required to run scripts): Set the path to your built `lppo` binary in Settings → Extensions → LPPO Language Support → `lppo.executablePath`.
  Example: `/home/user/lppo/target/release/lppo`

## Extension Settings

| Setting | Description | Default |
|---|---|---|
| `lppo.executablePath` | Path to the LPPO executable (built binary) | `""` |
| `lppo.runInTerminal` | Run LPPO scripts in the integrated terminal | `true` |

## Commands

| Command | Description |
|---|---|
| **LPPO: Run LPPO Script** | Runs the current `.lppo` file using the configured executable |

## Installation

### From VSIX

Download `lppo.vsix` and install:

```bash
code --install-extension lppo.vsix
```

### From VS Code Marketplace

Search for **"LPPO Language Support"** in the Extensions tab (Ctrl+Shift+X / Cmd+Shift+X) and click Install.

## File Icons

`.lppo` files show the LPPO icon in the Explorer. Built-in file icon themes that support language mode icons will display it automatically.

## Publishing

This extension is ready to publish to the VS Code Marketplace:

```bash
npx vsce login lppo       # enter your Azure DevOps PAT
npx vsce publish
```

## License

MIT
