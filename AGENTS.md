# AGENTS.md

Context for coding agents working in this repo. Read the [README](./README.md) for what Atlas *is*; read this for how to change it.

## Build / run / check

```sh
cargo build --release              # compile
./target/release/atlas tui         # launch TUI
./target/release/atlas doctor      # fast sanity check — works against cached snapshot
cargo check                        # type-check only, faster than build
```

There is no test suite yet. Verify changes by running the binary.

## Layout

```
src/
  main.rs        subcommand router (tui / scan / export / list / info / doctor)
  model.rs       SoftwareItem + enums (Source, Kind, Arch, Status)
  index.rs       snapshot load/save + merge engine
  lenses.rs      All / Outdated / Duplicates / Bloat / Stale / Rosetta / Unsigned
  actions.rs     owner-aware update + delete command builders
  manifest.rs    manifest.json / Brewfile / mas.txt export
  probe.rs       detect which installers are actually present
  cli.rs         read-only subcommand surface (list / info / doctor)
  scan/          per-source adapters: apps / brew / zerobrew / mas
  tui/           app state, event loop, rendering
docs/            GitHub Pages site (index.html, skills/, install.sh)
skills/          drop-in agent skills (mac-cleanup/SKILL.md)
Formula/         Homebrew tap
```

## Conventions

- **Adapters activate at runtime.** `probe::Available::detect()` is the gate — never call `brew` / `zb` / `mas` unconditionally.
- **Source preference is `zb > mas > brew > nix > pkg > manual`.** Merge logic in `src/index.rs` relies on this ordering. Preserve it unless you genuinely intend to change attribution.
- **Snapshot is in-memory `Vec<SoftwareItem>` + JSON on disk.** No SQLite, no migrations. If you find yourself reaching for a DB, stop and justify it.
- **Actions are source-routed.** `brew uninstall`, `zb uninstall`, App Store has no CLI path, manual `.app` → Finder Trash via `osascript`. Never `rm -rf` an app bundle.
- **CLI is strictly read-only** (`list` / `info` / `doctor`). Destructive verbs live in the TUI only, so scripted agents cannot uninstall.
- **Columns in the TUI are fixed-width.** If you add a column, pad every row so alignment holds — see `pad_display` in `src/tui/ui.rs`.

## When changing scan or merge logic

Rebuild then run `atlas scan` to regenerate the snapshot, or the TUI will keep showing stale data from the previous schema. The snapshot path is `~/Library/Application Support/dev.atlas.Atlas/index.json` — safe to delete.

## Don't

- Don't add a GUI, menubar, or background daemon. Out of scope for v0.
- Don't add network calls. Everything is local.
- Don't break the `--json` shape of `list` / `info` / `doctor`. Downstream agents and the `mac-cleanup` skill depend on it.
- Don't skip pre-commit hooks with `--no-verify`.

## Useful one-liners

```sh
# What does the inventory look like right now?
atlas doctor --json | jq

# Top disk hogs:
atlas list --sort size --limit 10 --json | jq '.[] | {name, size_bytes, source}'

# Find an item by bundle id:
atlas info com.apple.dt.Xcode --json
```

## Distribution

- Landing page: `docs/index.html`, served at `https://friedscholvinck.github.io/atlas/` from the `main` branch `/docs` folder.
- Install: `docs/install.sh` (cargo-install fallback) and `Formula/atlas.rb` (HEAD-only Homebrew tap).
- Agent skill: `skills/mac-cleanup/SKILL.md`, with a hosted dedicated page at `docs/skills/mac-cleanup.html`.

Changes to any of these need to stay in sync — if you bump a CLI flag, grep for it across `docs/` and `skills/` too.
