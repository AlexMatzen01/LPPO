#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Building lppo..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

INSTALL_DIR="$HOME/.LPPO"
BIN_DIR="$INSTALL_DIR/bin"

mkdir -p "$BIN_DIR"

cp "$SCRIPT_DIR/repositories.json" "$INSTALL_DIR/repositories.json"
cp "$SCRIPT_DIR/target/release/lppo" "$BIN_DIR/lppo"

chmod +x "$BIN_DIR/lppo"

echo "Installed lppo to $BIN_DIR/lppo"
echo "Installed repositories.json to $INSTALL_DIR/repositories.json"
