#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Building mltv..."
cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml"
MLTV_BINARY="$SCRIPT_DIR/target/debug/mltv"

echo "Building mpm..."
"$MLTV_BINARY" deploy "$SCRIPT_DIR/mpm.mltv" -o "$SCRIPT_DIR/mpm"
MPM_BINARY="$SCRIPT_DIR/mpm"

if [ "$(id -u)" -eq 0 ]; then
    DEST="/usr/local/bin"
else
    DEST="${HOME}/.local/bin"
    mkdir -p "$DEST"
    case ":$PATH:" in
        *":$DEST:"*) ;;
        *) echo "export PATH=\"\$PATH:$DEST\"" >> "$HOME/.bashrc"
           echo "export PATH=\"\$PATH:$DEST\"" >> "$HOME/.profile"
           echo "Added $DEST to PATH in ~/.bashrc and ~/.profile" ;;
    esac
fi

cp "$MLTV_BINARY" "$DEST/mltv"
cp "$MPM_BINARY" "$DEST/mpm"
echo "Installed mltv and mpm to $DEST"

echo ""
echo "mltv and mpm installed! Restart your terminal, then:"
echo "  mltv deploy myfile.mltv -o myprogram"
echo "  mpm install mltv-lang/sample-lib"
