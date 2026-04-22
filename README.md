# atlas

Native control plane for your Mac's software stack — inventory, provenance, updates, running state, and one-click rebuild across every install path.

**Landing page · install instructions:** <https://friedscholvinck.github.io/atlas>

## What it does

Atlas builds a unified local graph of everything installed on your Mac — `.app` bundles, brew formulas/casks, zerobrew, App Store apps, and more — and exposes it through a fast, keyboard-first TUI. It sits *above* your package managers rather than replacing them.

- **One row per thing**, not one row per manager. A formula in brew *and* zerobrew collapses into a single row attributed to whichever installer you actually use.
- **Lenses** — `Outdated`, `Duplicates`, `Bloat`, `Stale`, `Rosetta`, `Unsigned` — each answers a question you already ask yourself occasionally.
- **Owner-aware actions** — `u` update, `U` update everything, `d` uninstall (via the actual installer, or Finder Trash for manual `.app` bundles).
- **Migration export** — `manifest.json`, `Brewfile`, `mas.txt` so a new Mac is reproducible.

## Install

```sh
# curl
curl -fsSL https://friedscholvinck.github.io/atlas/install.sh | sh

# homebrew
brew tap friedscholvinck/atlas https://github.com/FriedScholvinck/atlas.git
brew install --HEAD atlas
```

Requires macOS 13+ and (for now) a working Rust toolchain for the build step. A signed prebuilt binary ships once the first release is tagged.

## Build from source

```sh
git clone https://github.com/FriedScholvinck/atlas.git
cd atlas
cargo build --release
./target/release/atlas tui
```

## For scripts & AI agents

Atlas exposes a read-only JSON query surface so LLM agents, shell scripts, or CI checks can inspect what's on the machine without attaching a TTY:

```sh
atlas doctor --json                            # total / outdated / rosetta / by-source / by-kind
atlas list --lens outdated --json              # every upgradeable item, full records
atlas list --source brew --filter ripgrep      # plain-text for piping
atlas info com.apple.dt.Xcode --json           # one item by bundle-id, name, or id
```

Lenses: `all · outdated · duplicates · bloat · stale · rosetta · unsigned`.
Sources: `brew · zb · mas · nix · pkg · manual`. Add `--rescan` to bypass the cached snapshot.

## Keys

```
j / k         nav
h / l / tab   switch pane
/ or f        search name · bundle id
] / [         cycle installer filter
u             update selected via its installer
U             update everything
d             delete (or move .app to Trash)
r             rescan    e  export    ?  help    q  quit
```

## Design

Design doc and rolling decision log: [`initial-spec.md`](./initial-spec.md).

## License

MIT — see [LICENSE](./LICENSE).
