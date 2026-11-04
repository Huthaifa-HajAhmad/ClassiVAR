# Developing ClassiVAR

This is the practical guide: how to build, test, change and review the app. The architecture itself is in `ARCHITECTURE.md`; current status is in `ROADMAP.md`.

## Prerequisites

- Rust **1.98.1** (pinned by `rust-toolchain.toml`; `rustup` installs it on the first `cargo` command).
- Windows: Visual Studio C++ build tools, Windows SDK, CMake. macOS: Xcode command-line tools (the app has not been built on macOS yet).
- Network access for a first build (crates); the UI smoke test clicks "Check gene", which also calls the live gene sources.

## First run

```powershell
cargo build -p classivar
cargo run -p classivar
```

The first build compiles GPUI and takes several minutes. The app opens at 1240×900 with a 960×600 minimum.

Try a corrected rule file without rebuilding:

```powershell
cargo run -p classivar -- --rules path\to\acmg-amp-points.toml
```

The file is checked at start-up; an invalid file stops the program with the reason.

## The merge gate

```powershell
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

```sh
./scripts/check.sh
```

Both run, in order: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo build --workspace --locked`. CI runs the same set and adds `cargo audit`.

Targeted runs:

```powershell
cargo test -p classivar-core            # engine, rules, case files, reevaluation
cargo test -p classivar-sources         # parsers, offline samples
cargo test -p classivar-sources -- --ignored   # real ClinGen/G2P/PanelApp calls
cargo test -p classivar                 # app-level: colours, scale, file names, gene list, settings
```

The live tests are excluded from the merge gate on purpose: a service outage must not block a merge. Run them before a release and after any report of a wrong gene answer.

## Verify the window, not just the tests

Unit tests do not draw a screen. A stack overflow, a bad binding, or an unreadable dark theme only shows up in a real window.

```powershell
cargo build -p classivar
powershell -ExecutionPolicy Bypass -File scripts/gui-smoke.ps1
```

`scripts/gui-smoke.ps1`:

- refuses to click or capture unless ClassiVAR is the foreground window;
- captures only the window's client area (an always-on-top window could still overlap it, so do not run it over one);
- starts from the light theme, tests light → dark → light with the saved choice, gene fill-in suggestions, the PVS1 guide, and clearing the gene box;
- restores the theme preference it found and writes screenshots to `%TEMP%\classivar-smoke`.

It finds controls by their accessible names (for example `PVS1:*not applied*`, `PVS1 Very strong:*`, `Theme:*Switch to dark*`), so renaming a button means updating the script.

It is Windows-only because it drives UI Automation. The paths it does **not** cover and that need a human click: Save/Open dialogs, keyboard-only use, macOS.

## Where things live

| You want to change… | Edit |
|---|---|
| criterion wording, points, caps, class limits, exclusivity, PVS1 guide | `rules/acmg-amp-points.toml` |
| how rules are loaded/validated | `crates/classivar-core/src/rules.rs` |
| scoring, 2015 table, conflicts, both-sides rule | `crates/classivar-core/src/assess.rs` |
| reported-variant comparison and verdict | `crates/classivar-core/src/reevaluate.rs` |
| saved-case JSON and version checks | `crates/classivar-core/src/case.rs` |
| ClinGen/G2P/PanelApp access | `crates/classivar-sources/src/` |
| screen state and actions | `crates/classivar/src/view.rs` |
| drawing (top bar, panels, criteria, result) | `crates/classivar/src/view/panels.rs` |
| gene suggestions | `crates/classivar/src/genes.rs` |
| theme preference | `crates/classivar/src/settings.rs` |

## State in one screen

`Workbench` (in `view.rs`) owns:

- `mode`: New variant or Reported variant.
- `editing`: Today, or one earlier record by index.
- `today` / `records[i].evidence`: a map from criterion code to `Choice { strength, situation }`. `situation` is a PVS1 guide id, not a sentence.
- `guide_open`: the one criterion whose guide is showing (Today only).
- `confirm`: the one pending question (clear, remove record, switch gene, open case, load example).
- `gene_link`, `case_gene`, `gene` lookups: the gene decision, the gene actually checked, and the four answers.
- `next_record`: the next record number, so a removal cannot create two "Record 2"s.

Rules that are easy to break: an index in `Editing::Record` must always be `< records.len()`; a guided criterion in Today must have a `situation`; `has_case_data` includes the typed variant and gene, not only criteria.

## Recipes

### Add or change a clinical rule

1. Edit `rules/acmg-amp-points.toml`. Every criterion, guide, exclusive pair and tier needs a `source` that exists under `[[sources]]`.
2. If you add a field, add it to `RuleSetFile`/`Criterion` and validate it in `RuleSet::check` (fail at load, not at the first click).
3. Add a test with the published worked example in `classivar-core`.
4. Update the rule file's header note, `docs/ROADMAP.md`, and the "Rule content" section of `docs/ARCHITECTURE.md`.
5. If it can change a class, add a note to "Needs your decision" unless the source is unambiguous.

Strength caps are **cautious safeguards chosen for this program**, not published ACMG/ClinGen limits. Say so wherever you add one.

### Add a strength guide (a PVS1-style question)

Add `guide_source` and one or more `[[criteria.guide]]` entries. Each needs a stable `id` (saved with the case; keep it when rewording), a `short` label, `situation` text, and an optional `strength`; no `strength` means "does not apply". The loader checks the source, unique ids and texts, at least one answer, and that every offered strength is allowed for the criterion. The UI draws the guide from the rule file; no Rust change is needed.

### Add a data source

1. New module in `classivar-sources`; use `crate::get_ok` so timeouts, retries, size caps and the user-agent stay consistent.
2. Return `Fetched<T>` with the source name and UTC date, or a `SourceError`. Never turn a failure into an empty list.
3. Offline test on a saved sample response, plus an `#[ignore]` live test.
4. Add the endpoint, terms and freshness to `ARCHITECTURE.md`.

### Change the screen

1. State and actions in `view.rs`, drawing in `view/panels.rs`. Keep functions small: the Windows debug stack is the reason.
2. Use theme tokens (`cx.theme()`) for chrome and the checked pairs for evidence colours. Add a contrast test if you add a new pair.
3. Give every button a stable id and, when the label is a glyph or an abbreviation, an `accessibility_label`.
4. Run `scripts/gui-smoke.ps1` and click the new path.

## Debugging

- **App closes when a screen opens**: stack overflow. Check the function that draws it; `build.rs` already asks for 16 MB, but keep render functions small. Reproduce by clicking, not by tests.
- **Theme resets**: the preference is `%APPDATA%\ClassiVAR\settings.txt`. Delete it to force light.
- **A saved case will not open**: the file may be from a newer layout or use a criterion the current rules reject. The message names the reason; `case.rs` has the format and version.
- **A gene answer looks stale**: each lookup is generation-counted; starting a new check drops late answers. Answers are not saved with a case (the curator re-checks after opening).
- **A source test fails**: check the endpoint with the live test first; the parser is meant to fail loudly, never to return a shorter list.

## Review checklist

Before asking for review:

- [ ] `scripts/check.ps1` passes.
- [ ] `scripts/gui-smoke.ps1` passes if the screen changed; save screenshots of new states.
- [ ] New clinical content has a source in `[[sources]]`, a test, and a roadmap note.
- [ ] Docs that state counts, phases or behaviour are updated.
- [ ] No case file, `target/`, or screenshot is staged.
- [ ] New UI text is plain language, says what is unknown, and never implies clinical validation.

## Continuous integration

`.github/workflows/ci.yml` runs fmt, clippy, tests and build on Windows (blocking) and macOS (non-blocking until the first green run), plus `cargo audit`. `.github/workflows/live-sources.yml` runs the live source tests weekly. As of 2026-11-04 these have never run: the repository had no remote. Put the first green run URL in `docs/ROADMAP.md` under D3.
