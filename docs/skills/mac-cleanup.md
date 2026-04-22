---
name: mac-cleanup
description: Use this skill when the user wants to clean up their Mac — free disk space, find apps they never open, spot Rosetta-only x86_64 apps, deduplicate brew/zerobrew/mas installs, or audit what's installed. Also use when the user says "what should I uninstall", "why is my disk full", "what haven't I opened in a while", or asks for a storage / software audit of their machine. Works via the `atlas` CLI, which is a local macOS software-graph tool. Everything is local — nothing is uploaded.
---

# Mac Cleanup with Atlas

Atlas is a local macOS tool that builds a unified inventory of everything installed — `.app` bundles, Homebrew formulas and casks, zerobrew, and App Store apps — and exposes it as a read-only JSON CLI. Use it to help the user audit and clean up their machine without ever uploading their software list.

## When to use this skill

Trigger on requests like:

- "What's eating my disk?" / "my disk is full"
- "Which apps do I never open?" / "what can I uninstall?"
- "What's still running under Rosetta?"
- "Show me duplicates between brew and zerobrew."
- "Audit my Mac."
- "What do I have installed?"

## Prerequisites — check first

Before running anything, confirm atlas is installed:

```sh
command -v atlas || echo "MISSING"
```

If missing, tell the user to install via one of:

```sh
curl -fsSL https://friedscholvinck.github.io/atlas/install.sh | sh
# or
brew tap friedscholvinck/atlas https://github.com/FriedScholvinck/atlas.git
brew install --HEAD atlas
```

Do not try to install it without their permission — it compiles from source.

## Core workflow

### 1. Start with the summary

Always begin a cleanup session with `atlas doctor --json`. It's cheap (uses the cached snapshot), returns total counts, per-source breakdown, and aggregate storage — enough to orient before drilling in.

```sh
atlas doctor --json
```

Sample response shape:

```json
{
  "generated_at": "2026-04-22T10:01:16Z",
  "total": 427,
  "by_source": { "brew": 194, "manual": 193, "zb": 40 },
  "by_kind":   { "app": 193, "cask": 12, "formula": 222 },
  "outdated": 74,
  "duplicates": 6,
  "rosetta": 79,
  "stale": 36,
  "total_size_bytes": 83150000000,
  "installers_available": ["brew", "zb"]
}
```

If `generated_at` is older than ~a day, run `atlas scan --json` to refresh before making recommendations.

### 2. Pick the right lens for the user's question

| User asks | Command |
|---|---|
| "What's huge?" | `atlas list --sort size --limit 20 --json` |
| "What haven't I opened?" | `atlas list --lens stale --sort old --json` |
| "Rosetta apps?" | `atlas list --lens rosetta --json` |
| "Duplicates across installers?" | `atlas list --lens duplicates --json` |
| "What's outdated?" | `atlas list --lens outdated --json` |
| "What do I actually use?" | `atlas list --sort frequent --limit 20 --json` |
| "Info on one app" | `atlas info "<name or bundle-id>" --json` |

### 3. Synthesize, don't dump

Never paste the raw JSON back to the user. Parse it, pick the 5–10 most interesting rows, and present them as a decision list. For each candidate:

- Name and size (humanized, e.g. `2.1 GiB`)
- Last opened, formatted conversationally (e.g. `3 years ago`, `never opened`)
- Why you're flagging it (`largest app not opened in 2+ years`, `duplicate of a brew formula`)
- The exact uninstall path (see below)

### 4. Uninstall routing — never guess

The CLI is read-only by design. When the user decides to uninstall something, generate the right command from the item's `source`:

| Source | Command |
|---|---|
| `brew` | `brew uninstall <name>` (formula) or `brew uninstall --cask <name>` |
| `zerobrew` | `zb uninstall <name>` |
| `app_store` | App Store uninstalls via Launchpad — tell the user (no CLI path) |
| `manual` | Move `.app` to Trash: `osascript -e 'tell application "Finder" to delete POSIX file "<install_path>"'` |

Always show the exact command and wait for the user to run it. Never chain uninstalls without confirmation. For manual `.app` bundles, prefer the Finder-trash route over `rm -rf` — reversible, and some apps leave LaunchAgents behind that need a separate cleanup.

## Good cleanup patterns

### "Free the most GiB with the least regret"

```sh
atlas list --sort size --limit 30 --json
```

Intersect with `--lens stale` to find big + unused:

```sh
atlas list --lens stale --sort size --limit 20 --json
```

These are the highest-leverage uninstalls.

### "Apple Silicon check"

```sh
atlas list --lens rosetta --sort size --json
```

For each, check if an arm64 version exists (the user should do this manually — don't guess or fabricate).

### "Duplicate cleanup"

```sh
atlas list --lens duplicates --json
```

When the same tool is in brew and zerobrew, the user almost always wants to keep one. `zb` is typically preferred on Apple Silicon; brew is preferred when the user has established brew workflows. Ask — don't decide.

### "Storage freed estimate"

After identifying candidates, sum their `size_bytes` and report:

> "Uninstalling these 8 apps would free ~14.3 GiB."

Round to one decimal; don't over-promise because some apps leave support files behind.

## What NOT to do

- Do not parse `atlas list` plain-text output — always use `--json`.
- Do not run `atlas scan` every time — it takes 10-30s. Only re-scan if `doctor`'s `generated_at` is stale or the user says "rescan".
- Do not suggest uninstalling system apps (`/System/Applications/*`) or Xcode without checking first.
- Do not fabricate data — every recommendation must come from a real CLI response. If a field is `null` (e.g. `last_used`), say "no Spotlight data" rather than inventing a date.
- Do not run destructive commands. Generate them for the user to execute.
- Do not upload the inventory anywhere.

## Example session

**User:** "My Mac is full, help me clean up."

**You:**

```
[run: atlas doctor --json]
```

→ "You have 427 items installed, taking 77 GiB. 36 apps haven't been opened in 90+ days, and 79 run under Rosetta. Let me find the highest-leverage targets."

```
[run: atlas list --lens stale --sort size --limit 10 --json]
```

→ "Top cleanup candidates:

1. **InDesign** — 1.78 GiB, last opened 2023-10-23. Adobe app you haven't touched in 18 months.
2. **Tableau** — 1.73 GiB, last opened 2023-11-03.
3. **DBeaver Community** — 154 MiB, last opened 2023-10-26.

Uninstalling these three frees ~3.6 GiB. Want the exact commands?"

Wait for confirmation before generating `osascript ... delete ...` lines.

## Exit criteria

A cleanup session is done when:
- The user has a clear list of what to remove and how, OR
- You've confirmed nothing obvious stands out (a healthy Mac with no bloat lens hits is fine).

Don't loop indefinitely. If the user hasn't replied in a few turns, summarize what you found and stop.
