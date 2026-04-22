#!/bin/sh
# atlas installer — https://github.com/FriedScholvinck/atlas
#
# Usage:
#   curl -fsSL https://friedscholvinck.github.io/atlas/install.sh | sh
#
# Until a tagged release ships signed prebuilt binaries, this installs from
# source via `cargo install`. That puts the `atlas` binary in ~/.cargo/bin.

set -eu

REPO="https://github.com/FriedScholvinck/atlas.git"
BRANCH="main"

say() { printf '\033[36m==>\033[0m %s\n' "$1"; }
err() { printf '\033[31merror:\033[0m %s\n' "$1" >&2; exit 1; }

case "$(uname -s)" in
  Darwin) ;;
  *) err "atlas currently only supports macOS. Detected: $(uname -s)" ;;
esac

arch="$(uname -m)"
case "$arch" in
  arm64|x86_64) say "Detected macOS on $arch" ;;
  *) err "unsupported architecture: $arch" ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
  err "cargo not found. Install the Rust toolchain first:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  then re-run this installer."
fi

say "Building atlas from $REPO (branch: $BRANCH)"
say "This compiles locally — takes a minute or two on first install."

cargo install --git "$REPO" --branch "$BRANCH" --force atlas

bin="$(command -v atlas || true)"
if [ -z "$bin" ]; then
  bin="$HOME/.cargo/bin/atlas"
fi

printf '\n\033[32m✓\033[0m atlas installed: %s\n' "$bin"

case ":$PATH:" in
  *":$HOME/.cargo/bin:"*) ;;
  *) printf '\n\033[33mnote:\033[0m ~/.cargo/bin is not on your PATH. Add it:\n    export PATH="$HOME/.cargo/bin:$PATH"\n' ;;
esac

printf '\nRun: \033[36matlas tui\033[0m\n'
