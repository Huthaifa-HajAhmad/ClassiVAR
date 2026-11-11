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
crates/classivar-core/       engine, no UI: rules.rs (load+check), assess.rs (score), combining_table.rs (the 2015 table as rows), reevaluate.rs (reported variants), case.rs (file format)
crates/classivar-sources/    ClinGen, G2P, PanelApp clients; shared HTTP rules and caps
crates/classivar/            GPUI app: main.rs, view.rs (Workbench state), view/panels.rs, view/panels/helper_bar.rs, view/panels/helpers.rs, view/panels/evidence_stack.rs, view/panels/combining_table.rs, view/panels/point_axis.rs and view/panels/what_if.rs (drawing; the layout rules of the stack, the table and the axis are plain tested functions, and point_axis draws the axis the stack and the "what if" bars share), design.rs (the app's own colours), genes.rs, settings.rs
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
- Saved case files contain variant, gene and lab history. `*.classivar.json` is git-ignored; keep it that way. `CaseFile::from_json` caps field lengths and the record count; raise the caps together with the app's own typing caps.

## Verify a UI change

Tests do not draw the window, and a stack overflow or a bad binding only appears on screen. After a UI change:

1. `scripts/check.ps1`
2. `scripts/gui-smoke.ps1` (it never calls the live sources, so it runs offline; it refuses to act unless ClassiVAR is the foreground window; screenshots go to `%TEMP%\classivar-smoke`)
3. Click the changed path yourself if the smoke script does not cover it (Save/Open dialogs and keyboard are not covered).

## Gotchas

- Theme preference: `%APPDATA%\ClassiVAR\settings.txt` (`theme=dark|light`).
- A case saved under other rules says so on open; per-record rule sets are intentionally not saved.
- Live source tests hit real services; run them separately, not in the merge gate.
- CI (`.github/workflows/`) has never run: no remote existed. Treat "CI covers it" as false until a run URL is in `docs/ROADMAP.md`.
- The repository had no commits beyond the audit baseline when this file was written. Do not commit unless the user asks.

## When you change behaviour

Update, in the same change:

- `docs/ROADMAP.md`: what is done, what is next, and any decision the user must make.
- `docs/ARCHITECTURE.md`: layout, data flow, test counts, known problems.
- `docs/MVP.md` and `docs/design/design-brief.md` only if scope or the design claim changed.
- `rules/acmg-amp-points.toml` header/notes if clinical content changed.
