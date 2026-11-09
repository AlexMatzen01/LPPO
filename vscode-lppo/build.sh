#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "[build] Installing dependencies..."
npm install --silent

echo "[build] Compiling TypeScript..."
npm run compile

echo "[build] Packaging VSIX..."
npx vsce package -o lppo.vsix

echo "[build] Done: $SCRIPT_DIR/lppo.vsix"
