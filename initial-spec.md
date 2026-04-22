## Product

**Working name:** Atlas for Mac.

Atlas is a lightweight native macOS app with an optional TUI that builds a local software graph of the machine: apps, CLIs, services, runtimes, installers, package-manager ownership, update status, resource hints, provenance, and exportable rebuild state. It does not try to replace Homebrew, Zerobrew, Nix, or the App Store; it sits above them as a discovery, orchestration, and migration layer. [nix-darwin.github](https://nix-darwin.github.io/nix-darwin/manual/)

## Problem

macOS software is fragmented across App Store installs, `.app` bundles, `.pkg` receipts, Homebrew formulas and casks, Nix packages, launch agents, and language-specific ecosystems, so users never get one reliable view of “what is on my machine and how did it get there.” Current tools each stop early: Homebrew GUIs do not inventory the whole Mac, and declarative tools help rebuild but do not provide a polished, unified operational view. [github](https://github.com/nix-darwin/nix-darwin)

## Users

- Developers who use Homebrew, Zerobrew, `mas`, Nix, and direct installs side by side. [github](https://github.com/mas-cli/mas)
- Power users who want a clean migration path to a new Mac without rebuilding from memory. [sa.ndeep](https://sa.ndeep.me/post/how-to-backup-and-restore-your-homebrew-packages)
- Consultants and platform engineers who want visibility into background services, package provenance, and software sprawl on their own Macs. [danielcorin](https://www.danielcorin.com/til/nix-darwin/launch-agents/)

## Core promise

Atlas answers four questions fast:
- What is installed?
- Who installed it?
- What is running?
- Can I recreate this machine elsewhere? [nix-darwin.github](https://nix-darwin.github.io/nix-darwin/manual/)

## Scope

### In scope
- Unified inventory of GUI apps, CLI tools, services, package managers, runtimes, and receipts. [osxdaily](https://osxdaily.com/2015/05/25/list-all-applications-mac-os-x/)
- Provenance detection: App Store, Homebrew, Zerobrew, Nix, pkg receipt, standalone app, direct binary, language manager. [github](https://github.com/mas-cli/mas)
- Update visibility via adapters to package managers where possible. [corkmac](https://corkmac.app)
- Migration/export to a portable machine manifest, with optional GitHub sync. [zenn](https://zenn.dev/sei40kr/articles/macos-dotfiles-nix-darwin-home-manager-blueprint?locale=en)
- Read-mostly system intelligence plus carefully scoped actions like open, reveal, uninstall via owner, upgrade via owner, and export. [corkmac](https://corkmac.app)

### Out of scope v1
- Becoming a new package manager.
- Kernel extensions, MDM replacement, or enterprise policy engine. [kolide](https://www.kolide.com/features/device-inventory/properties/mac-apps)
- Perfect cross-language dependency introspection for every ecosystem.

## Experience

### App model
- Native macOS menu-bar + windowed app.
- Optional TUI for terminal-first users.
- Single local index with near-instant search.
- No mandatory cloud account; GitHub sync is optional.

### Main views
- **Overview:** installed count, outdated count, running background items, largest apps, unmanaged installs.
- **Inventory:** one searchable table/card list across all software.
- **Packages:** grouped by owner, Homebrew/Zerobrew/Nix/mas/pkg/manual.
- **Services:** launch agents, launch daemons, login items, menu bar extras, long-running CLIs. [danielcorin](https://www.danielcorin.com/til/nix-darwin/launch-agents/)
- **Migration:** export, compare, restore plan.
- **Drift:** items installed manually but absent from your declared stack. [zenn](https://zenn.dev/sei40kr/articles/macos-dotfiles-nix-darwin-home-manager-blueprint?locale=en)

## Data model

Each software item gets one canonical record:

| Field | Meaning |
|---|---|
| `id` | Stable local identifier |
| `name` | Human-readable name |
| `kind` | app, cli, service, runtime, library, manager |
| `source` | app_store, brew, zerobrew, nix, pkg, manual, npm, cargo, pipx, uv, gem, go |
| `install_path` | Main path or symlink target |
| `version` | Installed version |
| `latest_version` | Known available version if adapter can resolve it |
| `status` | installed, running, outdated, broken, orphaned |
| `ownership` | Which manager claims it |
| `receipt_id` | pkg receipt if relevant |
| `bundle_id` | app bundle id if relevant |
| `signing` | signed, notarized, team id, developer |
| `license_guess` | open-source, closed-source, unknown |
| `runtime` | native, Java, Python, Node, Electron, Rust, Go, other |
| `arch` | arm64, x86_64, universal |
| `size_bytes` | disk usage estimate |
| `memory_hint` | current or recent process RSS summary if running |
| `autostart` | yes/no/source |
| `rebuild_recipe` | exact reinstall expression if derivable |

That structure maps well onto the real fragmentation in macOS packaging and lets Atlas unify what today is split across Homebrew GUIs, App Store tooling, and declarative config systems. [nix-darwin.github](https://nix-darwin.github.io/nix-darwin/manual/)

## Detection layer

Atlas should use adapters, not one giant scanner.

### System adapters
- **Applications:** `/Applications`, `~/Applications`, Spotlight/LaunchServices metadata, bundle ids, versions. [stackoverflow](https://stackoverflow.com/questions/78357623/how-to-get-all-installed-applications-and-their-detailed-info-on-mac-not-just)
- **pkg receipts:** `pkgutil` receipts and file lists for installer-based software. [discussions.apple](https://discussions.apple.com/thread/255610114)
- **Processes/services:** `launchctl`, login items, running processes, background items. [danielcorin](https://www.danielcorin.com/til/nix-darwin/launch-agents/)
- **Signatures:** `codesign`, notarization metadata, Team ID.

### Package-manager adapters
- **Homebrew:** `brew list --formula --versions`, `brew list --cask --versions`, `brew outdated --quiet`.
- **Zerobrew:** `zb list` and `zb outdated` — **not** a brew drop-in. Confirmed in v0: `zb` rejects `--formula`/`--versions` flags; its `list` is a bare `name version` per line and `outdated` emits `"<name> (<current>) < <latest>"`. Adapter owns its own parser. Today zb is formula-only (no cask surface). [mintlify](https://www.mintlify.com/lucasgelfond/zerobrew/quickstart)
- **mas:** installed and outdated App Store apps via `mas`. [x-cmd](https://www.x-cmd.com/install/mas/)
- **Nix / nix-darwin:** profiles, generations, installed packages, optional parse of flake-based rebuild inputs. [davi](https://davi.sh/blog/2024/01/nix-darwin/)
- **Language managers:** opportunistic adapters for `cargo install --list`, `npm -g`, `pnpm -g`, `pipx list`, `uv tool list`, `gem list`, Go install bins.

### Adapter activation rule
Every adapter declares `is_available()` and only runs when its owning CLI is actually on `PATH`. Detection happens at startup via a cheap `which`-style probe. Consequences:
- Fresh Mac with only brew installed → only brew adapter runs; zb/mas/nix adapters dormant.
- Machine with both brew *and* zb → **zb attribution wins** in the merge (see Merge logic). This reflects the user intent: "zerobrew over homebrew, but not zerobrew when not installed."
- Nothing ever prompts the user to install a package manager Atlas wants.

## Key features

### Unified inventory
Search “postgres” once and see:
- Postgres.app
- `postgresql@16` from Homebrew
- `psql` from Zerobrew
- `libpq` via Nix
- LaunchAgent running status
- disk usage and binary location

That is the key missing UX today; current tools expose only a single package universe at a time. [milanvarady.github](https://milanvarady.github.io/Applite/)

### Provenance
Every item gets badges:
- Owner: Homebrew / Zerobrew / App Store / Nix / Manual
- Type: app / cli / service
- Runtime: native / Electron / Java / Python / Node / Rust / Go
- Trust: signed / notarized / unknown
- License: OSS / proprietary / unknown

### Update center
A single “Outdated” page aggregates manager adapters, but execution still calls the native owner tool under the hood rather than reimplementing package resolution. [github](https://github.com/mas-cli/mas)

### Running state
For anything with a matching process or launchd entry:
- running now
- auto-start enabled
- memory usage
- binary path
- start owner

### Migration
Export a machine manifest with:
- apps
- CLIs
- services
- managers in use
- optional settings/dotfile references
- restore recipe per item if available

Then generate:
- Brewfile for brew-managed items. [sa.ndeep](https://sa.ndeep.me/post/how-to-backup-and-restore-your-homebrew-packages)
- `mas` install list for App Store apps. [github](https://github.com/mas-cli/mas)
- Nix snippets if already using nix-darwin/Home Manager. [carlosvaz](https://carlosvaz.com/posts/declarative-macos-management-with-nix-darwin-and-home-manager/)
- A fallback “manual reinstall checklist” for unmanaged apps.

## Restore flows

### Fast restore
User signs into GitHub or opens a local manifest, Atlas generates a restore plan and executes manager-native commands in dependency order where safe. [sa.ndeep](https://sa.ndeep.me/post/how-to-backup-and-restore-your-homebrew-packages)

### Safe restore
Dry-run only:
- show what can be fully restored
- show items needing credentials
- show items needing App Store login
- show items that are deprecated or unavailable

## UX principles

- Read-fast, write-carefully.
- Everything local first.
- One row per thing, not one row per manager.
- Lightweight: launch in under 500ms target, idle near-zero CPU.
- No background daemon in v1 unless the user enables monitoring.
- "Explain why" on every status badge.

### Visual style (v0 shipped)
- **Rounded corners** on every bordered pane and modal (`BorderType::Rounded`).
- **Selection is a glyph, not a colour block.** Inverse-background selection is loud and dates the UI — we use a thin `▎` left-edge bar in accent colour on the focused pane, and bold the name. Unfocused panes show the cursor position only by the saved index, not a visible highlight (follows lazygit).
- **Quiet metadata.** Version, arch, and size on each row are rendered in `DarkGray` — they recede until the user looks at them. Only state-changing glyphs (`↑` outdated, `●` running, `✗` broken) and the `rosetta` marker get colour accents.
- **Single-line title bar** separated by `·` dots — no background fills, no icons. Shows only what is true: app name, item count, active installers, active filter (if any), outdated count (if non-zero).
- **One accent colour** (cyan) used sparingly: pane focus, selection bar, active filter label, destructive-modal is the only exception (red).
- **Responsive layout** — details pane auto-hides when width < 110 cols, collapsing to a two-pane view.

## Architecture

### Suggested stack
- **Core:** Rust library for scanners, adapters, index, manifest generation.
- **Native UI:** SwiftUI shell over Rust core, or Tauri-style bridge if you want faster iteration.
- **TUI:** Ratatui on top of same Rust core.
- **Storage:** ~~SQLite~~ → **in-memory `Vec<SoftwareItem>` + JSON snapshot** at `~/Library/Application Support/dev.atlas.Atlas/index.json`. See v0 decision note.
- **File watching:** FSEvents only where useful; otherwise scheduled rescans.

This split matches your instinct about performance and gives one engine for GUI, TUI, and eventual CLI automation. [corkmac](https://corkmac.app/Stripe/)

### Storage: why not SQLite (v0 decision)
For typical Macs the full inventory is a few hundred to a few thousand items — well under a MB of JSON. The access pattern is "load everything, filter in memory" (lenses, search), so SQLite's query optimizer earns nothing. In-memory + JSON snapshot gives: smaller dep tree, instant load, human-debuggable, trivially swappable to MessagePack (`rmp-serde`) or `postcard` if the snapshot ever grows. Revisit only if we add time-series history or cross-machine federation.

### Internal modules
- `atlas-core`: canonical models, merge engine
- `atlas-scan-system`: apps, bundles, receipts, launchd, codesign
- `atlas-adapter-brew`
- `atlas-adapter-zb`
- `atlas-adapter-mas`
- `atlas-adapter-nix`
- `atlas-adapter-lang`
- `atlas-manifest`
- `atlas-ui-macos`
- `atlas-tui`

## Merge logic

The hard product value is entity resolution:
- detect that `Visual Studio Code.app`, `code` CLI, and a cask/package relation belong together when confidence is high
- keep ambiguous items separate when confidence is low
- show the confidence/explanation

A simple scoring model works for v1:
- exact bundle id match
- same executable path
- same package metadata name
- same receipt owner
- same launchd plist target

### Source preference ordering (v0)
When two candidate records for the same entity merge, the attribution winner is picked by a fixed rank:

`Zerobrew → AppStore → Brew → Nix → Pkg → Manual → (lang managers) → Unknown`

Rationale: if a user has opted into zerobrew or the App Store as an installer, those tools carry more intent than a generic brew fallback. Non-source fields (version, path, size, last_used, bundle_id) are merged field-wise — first value wins per field — so nothing is lost when a brew formula and a zb formula collapse into one row.

## Permissions

Atlas should avoid scary permissions by default.
- No full disk access required for base inventory if you stay within standard locations where possible.
- Optional elevated mode only for deeper receipts, cleanup, or uninstall flows.
- Never send inventory to the cloud by default.

## v1 actions

- Open app
- Reveal in Finder
- Copy install path
- Show owner tool
- Upgrade item
- Uninstall via owner
- Disable/enable autostart where safely supported
- Export manifest
- Generate restore plan

### Action model (v0 shipped)

Atlas performs every mutating action through the **owner installer**, never by poking files directly when a package manager is in charge. Each action resolves to a concrete `ShellCmd { exe, args, display }` before the user sees the confirmation modal, so nothing runs that the user hasn't read verbatim.

Keybindings:
- `u` — upgrade the selected item via its owner
- `U` — upgrade *everything* (runs the chain `zb upgrade` → `brew upgrade` → `mas upgrade`, only including installers that are present)
- `d` — delete the selected item via its owner (for `.app` manual bundles, "delete" means *move to Trash* via `osascript → Finder → delete POSIX file …`, which is reversible from the Finder trash)

Dispatch table, per source:

| source    | delete                          | update                        | update-all        |
|-----------|---------------------------------|-------------------------------|-------------------|
| Brew      | `brew uninstall [--cask] <n>`   | `brew upgrade [--cask] <n>`   | `brew upgrade`    |
| Zerobrew  | `zb uninstall <n>`              | `zb upgrade <n>`              | `zb upgrade`      |
| AppStore  | *disabled v0* (needs sudo)      | *disabled v0* (needs bundle id) | `mas upgrade`   |
| Manual    | Finder Trash via `osascript`    | *n/a*                         | *n/a*             |
| Nix / pkg / lang managers | *deferred*         | *deferred*                    | *deferred*        |

Execution model (lazygit-style):
1. User selects an item, presses `d` or `u`. A confirmation modal renders with the *literal* command(s) that will run. Destructive actions use a red-toned border.
2. On `y` / `⏎`: Atlas drops out of raw mode and the alt-screen, then runs the command with **inherited stdio** so the user sees real installer output (progress bars, prompts, etc.). No captured-and-replayed buffer — what the installer says, the user sees live.
3. On completion, Atlas prints a coloured pass/fail marker, waits for `⏎`, restores the alt-screen, and triggers a rescan so the item's state reflects the new reality.
4. On `n` / `esc`: modal dismisses, nothing runs, status line shows "cancelled".

### Filter model

Two orthogonal narrowing dimensions, both live-composable:
1. **Lens** (`j/k` in the Lenses pane) — answers a semantic question ("what's stale?", "what's x86_64 only?", …).
2. **Source filter** (`]` / `[` step forward / backward through `all → brew → zb → mas → manual`) — answers the provenance question.

A text search (`/` *or* `f`) layers on top, matching name, bundle id, or source label. Both keys open the same search prompt so users reach for whichever is closer to home-row. The three narrowing dimensions are ANDed: e.g., *Rosetta lens + source=manual + query "adobe"* returns exactly the Adobe Intel-only `.app` bundles. The active source filter is always visible in the title bar.

## Lenses — "cleaner, faster, more secure"

Atlas exposes recurring machine-hygiene questions as **lenses** — one-keystroke filters over the same unified inventory, each with a plain-English hint. Each lens answers a single question the user already asks themselves occasionally.

### v0 lens set (shipping)
- **All** — unfiltered inventory.
- **Outdated** — items where an adapter reports an upgrade, or where `version != latest_version`.
- **Duplicates** — same normalized tool name present under ≥ 2 different sources (e.g., `python@3.12` in brew *and* `python` in pipx). Name normalization strips version suffixes like `@3.12` and case.
- **Bloat** — top 50 by `size_bytes`, so the user sees where disk actually goes, not where they *think* it goes.
- **Stale** — `.app` bundles whose `kMDItemLastUsedDate` (Spotlight) is > 90 days ago. This is the hardest signal to fake and the most productive for cleanup.
- **Rosetta** — apps whose `LSArchitecturePriority` / `lipo -archs` resolves to `x86_64` only on an arm64 host. Candidates for "is there an Apple Silicon build yet?"
- **Unsigned** — items where a `codesign` probe reports no valid signature. Run on-demand only (codesign is 10–50 ms per app), not on startup.

### Lens ideas queued for v0.x / v1
- **Launchd noise** — login items + launch agents + launch daemons sorted by "most recently added" with a one-click *disable autostart*.
- **Duplicate binaries on PATH** — e.g., two `node`s, two `python`s, two `jq`s, highlighting which one actually runs first (`which -a` ordering).
- **Electron tax** — items with an Electron framework bundled, summed disk + (if running) summed RSS across helper processes.
- **Unopened since install** — apps whose `kMDItemLastUsedDate` is null *and* whose install mtime is > 30 days old.
- **Kext / system extension residents** — `systemextensionsctl list` + legacy kext probes.
- **Signed by expired / unknown team** — codesign Team ID not in a small allowlist the user curates.
- **Background memory hogs** — running launchd jobs whose RSS > threshold.
- **Rosetta-only CLI** — the binaries under PATH, not just apps.
- **Brew bottle vs source** — formulas compiled from source when a bottle was available (user pays CPU tax for no reason).
- **Orphan receipts** — `pkgutil --pkgs` entries whose payload files no longer exist.

## Nice-to-have later

- Disk cleanup suggestions (tied to Bloat lens + explicit "safe to remove" heuristics).
- SBOM-style export (CycloneDX/SPDX) — lets Atlas talk to security tooling downstream.
- Vulnerability feed enrichment (OSV/NVD lookups for detected formulas and languages).
- Team/fleet mode via osquery/Kolide-style extensions. [kolide](https://www.kolide.com/features/device-inventory/properties/mac-apps)
- Homebrew "would a cask serve you better than this manual install?" detector.

## Competitive positioning

Position Atlas as:
- **Not Cork:** broader than Homebrew GUIs. [corkmac](https://corkmac.app)
- **Not Applite:** more than a cask browser. [milanvarady.github](https://milanvarady.github.io/Applite/)
- **Not nix-darwin:** easier visibility, optional declarative export, no Nix commitment. [github](https://github.com/nix-darwin/nix-darwin)
- **Not mas:** cross-source, not App Store-only. [github](https://github.com/mas-cli/mas)

## MVP

### Phase 0 (shipped, this repo) — thin vertical slice
- `.app` bundle scan (`/Applications`, `/System/Applications`, `~/Applications`) with Info.plist, bundle id, version, arch, size, Spotlight last-used.
- Adapter probe → only run brew / zb / mas when their CLI is on PATH.
- brew adapter (formula + cask, outdated).
- zb adapter (native `zb list` / `zb outdated` parsing — not brew-compatible).
- mas adapter (stubbed, activates if installed).
- Merge engine with zb-over-brew attribution preference.
- In-memory index + JSON snapshot.
- Seven lenses: All, Outdated, Duplicates, Bloat, Stale, Rosetta, Unsigned.
- Owner-aware actions with confirmation modal and lazygit-style suspended-stdio execution: `u` update one, `U` update all, `d` delete / Trash.
- Source filter (`]` / `[`) cycling `all → brew → zb → mas → manual`, visible in the title bar, composable with lens + search.
- lazygit-style TUI (Ratatui): rounded borders, subtle `▎` selection bar (no inverse-colour blocks), left lens sidebar, centre inventory, right detail+insights, bottom help bar, status line, `?` modal help. Responsive collapse under 110 cols.
- CLI: `atlas tui | scan | export` — export writes `./atlas-export/{manifest.json,Brewfile,mas.txt}`.
- Baseline: 1.2 MB release binary, ~10 s cold scan on ~400-item machine (dominated by `mdls` + dir-walk).

### Phase 1
- Inventory apps, CLIs, brew, mas, pkg, launchd
- Unified search
- Outdated aggregation for brew and mas
- Export manifest + Brewfile + `mas` list
- Native app + TUI

### Phase 2
- Zerobrew adapter
- Nix/nix-darwin parsing
- runtime classification
- open/closed-source heuristics
- migration planner

### Phase 3
- restore execution
- GitHub sync
- drift detection
- setup profiles for “new work Mac”, “consulting Mac”, “AI laptop”

## Risks

- “Cover everything” can spiral into a systems-management product.
- Some metadata like license, runtime, or update availability will always be heuristic for manually installed apps. [stackoverflow](https://stackoverflow.com/questions/78357623/how-to-get-all-installed-applications-and-their-detailed-info-on-mac-not-just)
- Zerobrew compatibility may shift quickly because it is still experimental. [github](https://github.com/lucasgelfond/zerobrew)

## Success metrics

- Time to answer “what is installed?” under 2 seconds on first scan after warm cache.
- 90%+ of visible user software categorized by source.
- Restore manifest generated with actionable recipes for 80%+ of inventoried items.
- Users remove duplicate/unmanaged software after seeing provenance and drift.

## Distribution (v0)

Distribution predates a tagged release, so everything flows through source builds:

- **Landing page** at `https://friedscholvinck.github.io/atlas` — single-file `docs/index.html`, served by GitHub Pages from `/docs` on `main`. Dark/minimal theme; matches the TUI's visual language.
- **curl installer** at `docs/install.sh` — detects macOS + arm64/x86_64, checks `cargo` is present, runs `cargo install --git <repo> --branch main atlas`, nudges PATH if `~/.cargo/bin` is missing. Invoked as `curl -fsSL https://friedscholvinck.github.io/atlas/install.sh | sh`.
- **Homebrew** via an ad-hoc tap. The formula lives at `Formula/atlas.rb` in the primary repo (not a separate `homebrew-atlas` tap), so the tap incantation is `brew tap friedscholvinck/atlas https://github.com/FriedScholvinck/atlas.git && brew install --HEAD atlas`. HEAD-only until the first release is cut.
- **No menubar, no native Mac app** for v0. Scope is strictly the TUI + CLI. Native GUI was explicitly deferred.

Once a versioned binary ships:
- Tag `v0.1.0`, attach signed `atlas-aarch64-apple-darwin.tar.gz` + `atlas-x86_64-apple-darwin.tar.gz` to the release.
- Rewrite `install.sh` to fetch the archive and skip the `cargo install` path.
- Drop `--HEAD` from the formula and point `url`/`sha256` at the release tarball.

## One-line pitch

**Atlas is the native control plane for your Mac’s software stack: inventory, provenance, updates, running state, and one-click rebuild across every install path.** [nix-darwin.github](https://nix-darwin.github.io/nix-darwin/manual/)
