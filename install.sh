#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
EXT_DIR="$SCRIPT_DIR/vscode-mltv"

NO_DESTRUCT=false
for arg in "$@"; do
    if [ "$arg" == "--no-destruct" ]; then
        NO_DESTRUCT=true
    fi
done

# --- Build ---
echo "Building mltv..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
BINARY="$SCRIPT_DIR/target/release/mltv"

echo "Building mpm..."
"$BINARY" deploy "$SCRIPT_DIR/mpm.mltv" -o "$SCRIPT_DIR/mpm"
MPM_BINARY="$SCRIPT_DIR/mpm"

# --- Install binaries ---
if [ "$(id -u)" -eq 0 ]; then
    DEST="/usr/local/bin"
else
    DEST="${HOME}/.local/bin"
    mkdir -p "$DEST"
    # Add to PATH if missing
    case ":$PATH:" in
        *":$DEST:"*) ;;
        *) echo "export PATH=\"\$PATH:$DEST\"" >> "$HOME/.bashrc"
           echo "export PATH=\"\$PATH:$DEST\"" >> "$HOME/.profile"
           echo "Added $DEST to PATH in ~/.bashrc and ~/.profile" ;;
    esac
fi

cp "$BINARY" "$DEST/mltv"
cp "$MPM_BINARY" "$DEST/mpm"
echo "Installed mltv and mpm to $DEST"

# --- Install VS Code extension ---
install_extension() {
    local ext_dir="$1"
    mkdir -p "$ext_dir"
    if [ -d "$ext_dir/molotov-language-1.1" ]; then
        rm -rf "$ext_dir/molotov-language-1.1"
    fi
    cp -r "$EXT_DIR" "$ext_dir/molotov-language-1.1"
    echo "Installed VS Code extension to $ext_dir/molotov-language-1.1"
}

if [ -d "$HOME/.vscode/extensions" ]; then
    install_extension "$HOME/.vscode/extensions"
fi

if [ -d "$HOME/.vscode-oss/extensions" ]; then
    install_extension "$HOME/.vscode-oss/extensions"
fi

if [ -d "$HOME/.vscode-server/extensions" ]; then
    install_extension "$HOME/.vscode-server/extensions"
fi

# Antigravity / Codium / any vscodium variant
if [ -d "$HOME/.vscodium/extensions" ]; then
    install_extension "$HOME/.vscodium/extensions"
fi

echo ""
echo "mltv and mpm installed! Restart your terminal, then:"
echo "  mltv deploy myfile.mltv -o myprogram"
echo "  mpm install mltv-lang/sample-lib"

# --- Self-destruct ---
if [ "$NO_DESTRUCT" = false ]; then
    echo ""
    echo "Self-destructing in 3 seconds..."
    _SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
    _OTHER="$(cd "$(dirname "$0")" && pwd)/install.ps1"
    ( sleep 3 && rm -f "$_SELF" "$_OTHER" ) &
fi
