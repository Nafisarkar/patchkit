# PatchKit — Review, Fixes & Beta 1 Release Report

## 1. Multi-agent code review

Four review agents ran in parallel over every module:

| Agent | Scope |
|---|---|
| tweaks backend | `src/tools/tweaks.rs` (all 40 rows), helper layer in `src/sys.rs` |
| store/downloader | `src/tools/downloader.rs`, `src/pages/store.rs`, `StoreApp`/`StoreScript` |
| main loop & pages | `src/main.rs`, `src/pages/*` (stopped early; key areas were covered by the others) |
| data model | `src/data/options.rs` (option tables, ~50 Store apps, docs) |

## 2. Bugs found and fixed

**Critical (both were actively broken):**

1. **Inverted Bing toggle** (`tweaks.rs`) — the branches set/removed the
   `DisableSearchBoxSuggestions` policy the wrong way around, so "enabling"
   Bing kept web results dead. Also the re-enable path's registry delete
   aborted the whole toggle when the value didn't exist. Both fixed.
2. **`reg_sz` couldn't read `REG_EXPAND_SZ`** — the Shortcut Arrow toggle
   writes that type, but the status reader only matched literal `REG_SZ`, so
   the column permanently showed "Shown" and toggling re-removed instead of
   restoring. The reader now handles both types via whole-token matching.
   Related: `toggle_explorer_home` restore no longer aborts when a value it
   wants to delete was never set (benign deletes ignored).

**Status column (correctness + false positives):**

3. Path markers are never matched by bare name on PATH — the stock Windows
   `wt.exe` alias made Windows Terminal report "Installed" before install.
4. WinRAR / Notepad++ GUI markers converted from bare names to quiet
   file-existence markers (`WinRAR/WinRAR.exe`, `Notepad++/notepad++.exe`),
   avoiding status probes that would open GUI windows.

**Performance (the big one):**

5. Status refresh used to spawn ~50 `where.exe` processes plus a `cmd /C …
   --version` per script app, **synchronously on the UI thread** — the TUI
   froze for seconds at startup and after every install. Replaced with an
   in-process PATH scan (`OnceLock`-cached, PATHEXT-aware) plus file checks.
   Measured: the Store render test went **4.4 s → 0.03 s** (~150×).

**Minor fixes:**
6. Installer failures now carry the app name in the status bar.
7. `7zr.exe` helper downloads to a fixed path (was self-checking after a
   Content-Disposition-dependent name).
8. "downloaded {file}" message now names the app; `r` refresh covers the
   Store tab too.
9. Dead code removed (`is_downloaded`, `version_ok`), clippy clean.

**Docs:**
10. Module doc, broken `STORE_OPTIONS` link, and the misattached
    `StoreApp`/`StoreScript` doc comments corrected.

## 3. GitHub CI failures — found and fixed

- **Windows PowerShell 5.1 doesn't support `||`** — `Test-Path … || exit 1`
  was a parse error. Replaced with an `if` statement.
- **NSIS `VIProductVersion` requires numeric `X.X.X`** — `v0.1.0-beta.1`
  tags crashed the build. The workflow now strips `-suffix` before passing
  `/DVERSION`.
- **Fresh runners have no `dist/` directory** — "Can't open output file".
  The workflow now creates it before makensis runs.

## 4. Installer & release

- `cargo build --release` + `tools/gen-assets` (icon/bitmap panels) →
  `makensis /DVERSION=0.1.0 installer/patchkit.nsi` → **dist/patchkit-setup.exe**
  built and verified locally (Start Menu + desktop shortcuts, Add/Remove
  Programs entry, PATH addition with proper uninstall-time removal).
- Pushed to `github.com/Nafisarkar/patchkit`; CI run **green** (3m07s).
- Tagged **`v0.1.0-beta.1`** → the workflow published **PatchKit v0.1.0-beta.1**
  with both assets:
  - `patchkit-setup.exe` — NSIS installer
  - `patchkit.exe` — standalone portable binary
  https://github.com/Nafisarkar/patchkit/releases/tag/v0.1.0-beta.1

## 5. Tests

All **12 unit tests pass** (2 new: PATH resolution incl. `.cmd` shims, and
installed-detection for unknown apps), clippy reports zero warnings, and the
whole suite runs in 0.03 s thanks to the subprocess removal.

## Suggested follow-ups (not done, low priority)

- Table-drive the 40-row name→status string matching (three duplicated
  maps) so adding a row means editing one file.
- Fold the ~12 "Craft" MSI entries into a generator-of-rows helper.
- Consider `cargo test` for `resolve_link` against a fixture JSON.
