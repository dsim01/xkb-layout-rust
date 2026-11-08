#!/usr/bin/env bash
# Installs the xkb-layout release binary into ~/.local/bin (or $INSTALL_DIR).
#
#   curl -fsSL https://raw.githubusercontent.com/dsim01/xkb-layout-rust/main/scripts/install.sh | bash
#   curl -fsSL .../install.sh | TAG=v0.1.0 bash

set -euo pipefail

REPO="${REPO:-dsim01/xkb-layout-rust}"
TAG="${TAG:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN="xkb-layout"

die() { echo "error: $*" >&2; exit 1; }

for cmd in curl tar sha256sum; do
    command -v "$cmd" >/dev/null || die "'$cmd' is required"
done

case "$(uname -s)" in
    Linux) ;;
    *) die "only Linux is supported" ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) ARCH="x86_64" ;;
    aarch64 | arm64) ARCH="arm64" ;;
    *) die "unsupported architecture: $(uname -m)" ;;
esac

ASSET="$BIN-$ARCH.tar.gz"
if [ "$TAG" = "latest" ]; then
    BASE_URL="https://github.com/$REPO/releases/latest/download"
else
    BASE_URL="https://github.com/$REPO/releases/download/$TAG"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading $ASSET ($TAG) from $REPO..."
curl -fsSL "$BASE_URL/$ASSET" -o "$TMP/$ASSET" \
    || die "download failed: release '$TAG' or architecture '$ARCH' may not exist"
curl -fsSL "$BASE_URL/$ASSET.sha256" -o "$TMP/$ASSET.sha256" \
    || die "checksum download failed"

(cd "$TMP" && sha256sum --check --quiet "$ASSET.sha256") \
    || die "checksum mismatch"

tar -xzf "$TMP/$ASSET" -C "$TMP"
install -D -m 755 "$TMP/$BIN" "$INSTALL_DIR/$BIN"

echo "Installed $INSTALL_DIR/$BIN"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "note: $INSTALL_DIR is not in your PATH" ;;
esac
