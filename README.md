# xkb-layout

A tiny, memory-safe utility that prints the active XKB keyboard layout on
Linux/X11 — for status bars (i3status-rust, i3blocks, polybar, …) and scripts.

```console
$ xkb-layout
us
```

It can also run as a long-lived watcher that prints a new line only when the
layout changes, so your status bar updates instantly without polling.

## Features

- Prints the current layout code (`us`, `ua`, `de`, …)
- `--watch` mode: event-driven, zero CPU while idle, instant updates
- Picks up layout list changes made with `setxkbmap` while watching
- Written in pure Rust with [`x11rb`](https://github.com/psychon/x11rb): no
  `libX11` needed, no `unsafe` code in the crate (`#![forbid(unsafe_code)]`)
- Static binaries for `x86_64` and `arm64` — work on any Linux distribution

## How it works

X11 does not store the active layout *name*. It stores:

1. the active **group index** in the XKB state (`0`, `1`, …), and
2. the configured **layout list** in the `_XKB_RULES_NAMES` property of the
   root window: `rules\0model\0layout\0variant\0options\0`, e.g.
   `evdev\0pc105\0us,ua\0,\0grp:alt_shift_toggle\0`.

`xkb-layout` reads both and prints `layouts[group]`. In `--watch` mode it
subscribes to XKB group-change events and to property changes on the root
window instead of re-querying.

## Installation

### Prebuilt binary

Installs the latest release into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/dsim01/xkb-layout-rust/main/scripts/install.sh | bash
```

The installer verifies the SHA-256 checksum before installing. Options are
passed as environment variables **to `bash`**:

```bash
# A specific version
curl -fsSL https://raw.githubusercontent.com/dsim01/xkb-layout-rust/main/scripts/install.sh | TAG=v0.1.0 bash

# A different directory
curl -fsSL https://raw.githubusercontent.com/dsim01/xkb-layout-rust/main/scripts/install.sh | INSTALL_DIR=/usr/local/bin sudo -E bash
```

Uninstall:

```bash
curl -fsSL https://raw.githubusercontent.com/dsim01/xkb-layout-rust/main/scripts/uninstall.sh | bash
```

### From source

Requires a recent stable [Rust toolchain](https://rustup.rs/).
No system libraries are needed.

```bash
cargo install --git https://github.com/dsim01/xkb-layout-rust
# or, from a local checkout:
cargo install --path .
```

## Usage

```text
Usage: xkb-layout [OPTIONS]

Options:
  -w, --watch  Keep running and print a new line every time the layout changes
  -h, --help   Print this help
```

Exit codes: `0` on success, `1` on an X11 error (printed to stderr with an
`X11:` prefix), `2` on an invalid argument.

## Status bar integration

### i3status-rust

A wrapper script that maps layout codes to labels, e.g.
`~/.local/bin/switch-language`:

```bash
#!/usr/bin/env bash
xkb-layout --watch | while read -r layout; do
  case "$layout" in
    us) echo "EN 🇺🇸" ;;
    ua) echo "UA 🇺🇦" ;;
    *)  echo "$layout" ;;
  esac
done
```

`~/.config/i3status-rust/config.toml`:

```toml
[[block]]
block = "custom"
command = "switch-language"
persistent = true
```

Or without a wrapper:

```toml
[[block]]
block = "custom"
command = "xkb-layout --watch"
persistent = true
```

### Polling (i3blocks, polybar, tmux, …)

Any bar that runs a command on an interval can call `xkb-layout` directly. It
starts, prints and exits in about a millisecond.

## Development

```bash
cargo build             # debug build
cargo build --release   # optimized, stripped build
cargo test              # unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## Releasing

1. Bump `version` in `Cargo.toml` and commit.
2. Tag and push:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

The [Release workflow](.github/workflows/release.yml) then:

1. checks that the tag matches the `Cargo.toml` version,
2. builds static musl binaries for `x86_64` and `arm64`,
3. publishes a GitHub release with `xkb-layout-<arch>.tar.gz`, their
   `.sha256` checksums, and the install/uninstall scripts.

Every push to `main` and every pull request runs formatting, clippy and tests
via the [CI workflow](.github/workflows/ci.yml).

## Limitations

- X11 only. Under a Wayland session the value seen through XWayland may not
  match the compositor's actual layout.
- Prints the layout, not the variant: `us` and `us(dvorak)` both print `us`.

## Acknowledgements

Rust rewrite of the original C implementation of `xkb-layout`.
