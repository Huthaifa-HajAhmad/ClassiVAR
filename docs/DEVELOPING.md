# Developing ClassiVAR

This is the practical guide: how to build, test, change and review the app. The architecture itself is in `ARCHITECTURE.md`; current status is in `ROADMAP.md`.

## Prerequisites

- Rust **1.98.1** (pinned by `rust-toolchain.toml`; `rustup` installs it on the first `cargo` command).
- Windows: Visual Studio C++ build tools, Windows SDK, CMake. macOS: Xcode command-line tools (CI has since compiled there on a `macos-15-intel` runner; a local developer setup has not been tried).
- Network access for the first build (crates). The UI smoke test never calls the gene sources, so it runs offline; only `cargo test -p classivar-sources -- --ignored` and the weekly CI job use the network.

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

It finds controls by their accessible names (for example `PVS1:*not applied*`, `PVS1 Very strong:*`, `Theme:*Switch to dark*`). Text boxes carry an `aria_label`, which is their accessible name, so a box is found by that label (for example `Predictor score`, `De novo patients, highly-specific (parents confirmed)`); renaming one means updating the script.

A click on a button that is outside the visible part of the criteria list does nothing, and an open helper panel (it opens below the "Evidence helpers" bar) pushes criteria rows down. So the script scrolls first: `Scroll-To-Button` (for rows of the list, which can also hide under the pane header) and `Bring-Into-View` (a safety net for buttons below the window). When you add a panel above the list, expect to add `Scroll-To-Button` calls before the clicks that follow it. Plain text on screen is not visible to the script; check it in the screenshots.

It is Windows-only because it drives UI Automation. The paths it does **not** cover and that need a human click: Save/Open dialogs, keyboard-only use, macOS.

## Where things live

| You want to change… | Edit |
|---|---|
| criterion wording, points, caps, class limits, exclusivity, PVS1 guide | `rules/acmg-amp-points.toml` |
| the shape of the rule file (types) | `crates/classivar-core/src/rules/` (`basics.rs`, `criterion.rs`, `tables.rs`) |
| how rules are checked at load | `crates/classivar-core/src/rules/check*.rs` |
| how a score or count is read against a table | `crates/classivar-core/src/rules/read.rs` |
| scoring, conflicts, caps, warnings | `crates/classivar-core/src/assess/` (`mod.rs`, `item.rs`, `pairs.rs`) |
| the 2015 combining table | `crates/classivar-core/src/assess/classic.rs` |
| reported-variant comparison and verdict | `crates/classivar-core/src/reevaluate.rs` |
| saved-case JSON and version checks | `crates/classivar-core/src/case.rs` |
| ClinGen/G2P/PanelApp access | `crates/classivar-sources/src/` |
| screen state, its types and the constructor | `crates/classivar/src/view.rs` |
| what a click or key does, earlier records, gene lookups, save and open, the question window | `crates/classivar/src/view/` (`actions.rs`, `records.rs`, `gene.rs`, `case_io.rs`, `confirm.rs`) |
| bringing saved or copied evidence onto today's rules | `crates/classivar/src/view/evidence.rs` (`carry_forward`) |
| drawing (top bar, panels, criteria, result, history) | `crates/classivar/src/view/panels/` (one file per panel) |
| the Evidence helpers bar (which helpers show, and their buttons) | `crates/classivar/src/view/panels/helper_bar.rs` |
| an evidence helper: what its typed values mean, and its panel | `crates/classivar/src/view/panels/helpers/` (one file each); its open/typed state is in `crates/classivar/src/view/helper_state.rs` |
| the app's own colours and their contrast test | `crates/classivar/src/design.rs` |
| gene suggestions | `crates/classivar/src/genes.rs` |
| theme preference | `crates/classivar/src/settings.rs` |

## State in one screen

`Workbench` (in `view.rs`) owns:

- `mode`: New variant or Reported variant.
- `editing`: Today, or one earlier record by index.
- `today` / `records[i].evidence`: a map from criterion code to `Choice { strength, situation, reason }`. `situation` is a PVS1 guide id, not a sentence.
- `guide_open`: the one criterion whose guide is showing (Today only).
- `confirm`: the one pending question (clear, remove record, switch gene, open case, load example), with `confirm_focus` holding the keyboard while it is open and the page root blocking Tab behind it.
- `example`: on while the fictional example is on screen; it warns in a banner, refuses Save and makes a gene check ask first.
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

1. State in `view.rs` and `view/helper_state.rs`, actions in `view/*.rs`, drawing in `view/panels/`. Work out what a value means in a plain function with a test, and keep each drawing function small: the Windows debug stack is the reason.
2. Use theme tokens (`cx.theme()`) for chrome and the checked pairs for evidence colours. Add a contrast test if you add a new pair.
3. Give every button a stable id and, when the label is a glyph or an abbreviation, an `accessibility_label`; give every text box an `aria_label`. The smoke script finds both by those names.
4. Run `scripts/gui-smoke.ps1` and click the new path.

## Debugging

- **App closes when a screen opens**: stack overflow. Check the function that draws it; `build.rs` already asks for 16 MB, but keep render functions small. Reproduce by clicking, not by tests.
- **A backtrace stops at a dependency**: dependencies build without debug info (`[profile.dev.package."*"]` in `Cargo.toml`, which is what keeps `target/` near 2 GB instead of 5). You still get the crate and the function, not the file and line. To step into one, comment that block out or narrow it to the crate name; `CARGO_PROFILE_DEV_DEBUG` cannot override a package rule.
- **Theme resets**: the preference is `%APPDATA%\ClassiVAR\settings.txt` on Windows, `~/.config/ClassiVAR/settings.txt` elsewhere. Delete it to force light.
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

`.github/workflows/ci.yml` runs fmt (once), clippy, tests and build (each on Windows **and** macOS — both block a merge since the first green macOS build, run 42086149175) and `cargo audit` as separate jobs that run side by side, so a run takes as long as its slowest job. Only runs on `main` save the build cache; pull requests read it. `.github/workflows/live-sources.yml` runs the live source tests weekly. Run [42086149175](https://github.com/Huthaifa-HajAhmad/ClassiVAR/actions/runs/42086149175) on 2026-11-05 was the first fully green run; put the newest green run URL in `docs/ROADMAP.md` under D3.
