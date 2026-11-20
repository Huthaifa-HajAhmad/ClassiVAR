# AGENTS.md

Working notes for AI agents and new contributors. Humans: `docs/DEVELOPING.md` has the longer version.

## What this is

ClassiVAR is a Windows/macOS desktop app for learning and practising ACMG/AMP variant curation. It adds up the criteria a curator chooses and explains the class. It does **not** decide criteria, and it is **not** a clinical reporting system.

Non-negotiables:

- Every class shown must carry its review status (`UNVERIFIED rules vX`) and rule version. Never hide or weaken that.
- The app never classifies without a counted criterion chosen by the user.
- A failed lookup is never shown as "nothing found".
- Do not change a clinical rule (points, caps, classes, exclusivity, guide strengths) without a cited source, a test, and a note in `docs/ROADMAP.md`.
- `status = "reviewed"` requires a named reviewer and a `YYYY-MM-DD` date.

## Commands

```powershell
cargo run -p classivar                  # run the app
cargo run -p classivar -- --rules path\to\file.toml   # try another rule file
cargo test --workspace --locked         # all offline tests
cargo test -p classivar-sources -- --ignored           # live service tests (network)
powershell -ExecutionPolicy Bypass -File scripts/check.ps1   # the merge gate
powershell -ExecutionPolicy Bypass -File scripts/gui-smoke.ps1   # Windows UI smoke test
```

`scripts/check.sh` is the POSIX equivalent of `check.ps1`. The UI smoke test is Windows-only and needs a built `target\debug\classivar.exe`.

## Map

```
rules/acmg-amp-points.toml   all clinical content: criteria, points, classes, sources, guides, caps
crates/classivar-core/       engine, no UI: rules/ (load + check; check*.rs, read.rs for the helper tables), assess/ (score), combining_table.rs (the 2015 table as rows), reevaluate.rs (reported variants), case.rs (file format)
crates/classivar-sources/    ClinGen, G2P, PanelApp clients; shared HTTP rules and caps
crates/classivar/            GPUI app: main.rs, view.rs (Workbench state), view/*.rs (actions, records, gene lookups, save/open, helper_state.rs), view/panels/ (drawing, one file per panel) and view/panels/helpers/ (one file per evidence helper), design.rs (the app's own colours), genes.rs, settings.rs
                             The pictures that explain a class are panels too: track.rs (the interactive evidence track; evidence_stack.rs holds its step layout), combining_table.rs, what_if.rs, timeline.rs and strength_ladder.rs. point_axis.rs holds the axis the track, the "what if" bars and the timeline share. Their layout rules are plain tested functions; the drawing on top is thin.
                             The window: a rail (reports over time, `rail.rs`), the stage, and the editor sheet. The class is drawn as a dial, column or track (`picture.rs`, `dial.rs` + `dial_geometry.rs`, `column.rs`, `track.rs`); the choice is saved in settings. `look.rs` is the design system (type roles, panels, segmented control).
                             view/headless.rs: tests that draw the real window in GPUI's test platform (see "Verify a UI change")
docs/                        ROADMAP.md is the current status; ARCHITECTURE.md is the map; AUDIT is historical
```

Key types: `RuleSet` → `Applied` → `Assessment` (`class()`, `points_class`, `classic_class`, `conflicts`, `both_sides_met`) → `Reevaluation`/`Verdict` → `CaseFile`.

## Invariants to preserve

- `Assessment::class()` is `None` when there is no counted evidence, a contradictory pair is applied, both sides of the 2015 table are met while the points would give a confident class, or a combined limit is exceeded. Do not make the UI invent a class in those states.
- Earlier records are scored with `Counting::AsRecorded`, so a strength that is capped today still opens. Never "repair" an old record silently.
- A new assessment of a guided criterion (PVS1) must carry a guide answer id; the engine rejects a bare strength. Keep guide ids stable: saved cases refer to them. Rebuilding a missing answer is only allowed when exactly one guide answer sets the strength (`unique_situation_for`); otherwise ask, never guess. The criteria the helpers set (`HELPER_CODES`) refuse a guide at load time.
- `start_from_latest` must lower copied strengths to today's limits, fill the matching guide answer, and say so.
- `Assessment::class()` also withholds when a PP1+PP4-style combined limit is exceeded; do not make the UI invent a class.
- Every new button needs a stable id, a visible label, and `accessibility_label` when the label is a glyph or abbreviation. Every text box gets an `aria_label`.
- The fictional example never names a variant or gene on its own: saving is refused and checking a gene asks first, so a demo case cannot be attributed to a real variant (`Workbench.example`).
- Colours: use theme tokens for chrome, and the checked WCAG pairs in `design.rs` (`class_fill`/`class_ink`, `direction_colour`/`direction_ink`, warning pair) for meaning. Do not write a raw `rgb(...)` in a view file. Extend the contrast test for both themes if you add a pair.
- The debug build once overflowed the 1 MB Windows stack. Keep render functions small; `build.rs` requests 16 MB as a second safety net.
- Saved case files contain variant, gene and lab history. `*.classivar.json` is git-ignored; keep it that way. `CaseFile::from_json` caps field lengths and the record count. The variant, lab-name and year limits are public constants in `case.rs` that the app types against, so there is one number to change. The gene symbol shape is `classivar_sources::is_gene_symbol`.

## Verify a UI change

The unit tests do not draw the window, and a stack overflow or a bad binding only appears on screen (the headless tests catch a panic in a render function, not a wrong binding). After a UI change:

1. `scripts/check.ps1`. It includes the headless window tests (`crates/classivar/src/view/headless.rs`): they draw the real window in GPUI's test platform with no screen, mouse or keyboard, so run them freely, also while the user is working. They set state directly; they do not click.
2. `scripts/gui-smoke.ps1`. **It takes over the user's mouse and keyboard and must not run while the user is using the machine** (it also fails when they touch it). Ask first, or run it only when the user says the machine is free. (it never calls the live sources, so it runs offline; it refuses to act unless ClassiVAR is the foreground window; screenshots go to `%TEMP%\classivar-smoke`)
3. Click the changed path yourself if the smoke script does not cover it (Save/Open dialogs and keyboard are not covered).

## Gotchas

- Theme preference: `%APPDATA%\ClassiVAR\settings.txt` (`theme=dark|light`).
- A case saved under other rules says so on open; per-record rule sets are intentionally not saved.
- Live source tests hit real services; run them separately, not in the merge gate.
- CI (`.github/workflows/`) runs the merge gate on Windows and macOS plus `cargo audit`; the first fully green run is linked in `README.md`. It has not run since the headless window tests were added, so treat "CI covers them" as unverified until a newer run URL is in `docs/ROADMAP.md`. The Windows UI smoke test is not part of CI: run it yourself after a UI change.
- Do not commit unless the user asks.
- Size limits: files of 300 lines (a test block in the same file may go over) and functions of 50 lines hold in all three crates. Keep it that way. A new helper is one state type in `view/helper_state.rs`, one file in `view/panels/helpers/` with a tested function that reads the typed values, and one entry in `view/panels/helper_bar.rs`.
- Saved text with non-ASCII characters (`•`, `·`, `→`) is easy to damage when a script reads a file with the wrong encoding. After a scripted edit, search for `â€` and `Â`.

## When you change behaviour

Update, in the same change:

- `docs/ROADMAP.md`: what is done, what is next, and any decision the user must make.
- `docs/ARCHITECTURE.md`: layout, data flow, test counts, known problems.
- `docs/MVP.md` and `docs/design/design-brief.md` only if scope or the design claim changed.
- `rules/acmg-amp-points.toml` header/notes if clinical content changed.
