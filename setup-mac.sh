#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_MANIFEST="$SCRIPT_DIR/Cargo.toml"
INSTALL_DIR="$HOME/.LPPO"
BIN_DIR="$INSTALL_DIR/bin"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo not found. Install Rust via https://rustup.rs" >&2
  exit 1
fi

ARCH="$(uname -m)"
echo "Building lppo for macOS ($ARCH)..."
cargo build --release --manifest-path "$CARGO_MANIFEST"

# Ensure liblppo_rt.a exists (check debug first to preserve original behavior, fallback to release)
RT_LIB=""
if [[ -f "$SCRIPT_DIR/target/debug/liblppo_rt.a" ]]; then
  RT_LIB="$SCRIPT_DIR/target/debug/liblppo_rt.a"
elif [[ -f "$SCRIPT_DIR/target/release/liblppo_rt.a" ]]; then
  RT_LIB="$SCRIPT_DIR/target/release/liblppo_rt.a"
else
  echo "Error: liblppo_rt.a not found in target/debug or target/release." >&2
  echo "If it's a separate crate or build target, build it before running this script." >&2
  exit 1
fi

if [[ ! -f "$SCRIPT_DIR/repositories.json" ]]; then
  echo "Error: $SCRIPT_DIR/repositories.json not found." >&2
  exit 1
fi

mkdir -p "$BIN_DIR"

cp "$SCRIPT_DIR/repositories.json" "$INSTALL_DIR/repositories.json"
cp "$SCRIPT_DIR/target/release/lppo" "$BIN_DIR/lppo"
cp "$RT_LIB" "$BIN_DIR/liblppo_rt.a"

chmod +x "$BIN_DIR/lppo"

# Ad-hoc sign on macOS so local execution never chokes
if command -v codesign >/dev/null 2>&1; then
  codesign --force --sign - "$BIN_DIR/lppo" >/dev/null 2>&1 || true
fi

echo "Installed lppo ($ARCH) to $BIN_DIR/lppo"
echo "Installed liblppo_rt.a to $BIN_DIR/liblppo_rt.a"
echo "Installed repositories.json to $INSTALL_DIR/repositories.json"

# Check if BIN_DIR is in PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  SHELL_RC="$HOME/.zshrc"
  [[ "${SHELL:-}" == */bash ]] && SHELL_RC="$HOME/.bash_profile"
  echo ""
  echo "Note: $BIN_DIR is not currently in your PATH."
  echo "Add it by running:"
  echo "  echo 'export PATH=\"\$HOME/.LPPO/bin:\$PATH\"' >> \"$SHELL_RC\" && source \"$SHELL_RC\""
fi