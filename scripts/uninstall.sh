#!/usr/bin/env bash
# Removes xkb-layout from ~/.local/bin (or $INSTALL_DIR).

set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
TARGET="$INSTALL_DIR/xkb-layout"

if [ -e "$TARGET" ]; then
    rm -f "$TARGET"
    echo "Removed $TARGET"
else
    echo "Nothing to remove: $TARGET does not exist"
fi
