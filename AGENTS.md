# AGENTS.md

Working notes for AI agents and new contributors. Humans: `docs/DEVELOPING.md` has the longer version.

## What this is

ClassiVAR is a Windows/macOS desktop app that explains ACMG/AMP variant classifications. It adds up the criteria a curator chooses and explains the class. Version 1.0.0; release steps in `docs/RELEASE.md`. It does **not** decide criteria, and it is **not** a clinical reporting system.

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
cargo run -p classivar -- --open path\to\case.classivar.json   # debug only: launch directly into a saved case
powershell -ExecutionPolicy Bypass -File scripts/prototype.ps1   # live-reload HTML/CSS screen prototypes
cargo test --workspace --locked         # all offline tests
cargo test -p classivar-sources -- --ignored           # live service tests (network)
powershell -ExecutionPolicy Bypass -File scripts/check.ps1   # the merge gate
cargo check -p classivar --release --locked            # after editing #[cfg(debug_assertions)] code
powershell -ExecutionPolicy Bypass -File scripts/gui-smoke.ps1   # Windows UI smoke test
cargo run -p classivar-core --example review_packet > docs/review/REVIEW-PACKET.md   # after editing rules or sign-offs
cargo run -p classivar-core --example erepo_benchmark > docs/review/BENCHMARK-erepo.md # after a clinical rule change
powershell -ExecutionPolicy Bypass -File scripts/third-party-notices.ps1   # after changing Cargo.lock (needs cargo-about)
cargo packager -p classivar --release -f nsis   # Windows installer in target/packages (needs cargo-packager)
```

`scripts/check.sh` is the POSIX equivalent of `check.ps1`. The UI smoke test is Windows-only and needs a built `target\debug\classivar.exe`.

## Map

```
rules/acmg-amp-points.toml   all clinical content: criteria, points, classes, sources, guides, caps
rules/review-signoff.toml    one sign-off per rule entry (kind ai or human); the test `rules::review` checks it covers every entry
crates/classivar-core/       engine, no UI: rules/ (load + check; check*.rs, read.rs for the helper tables, population.rs for BA1/BS1/PM2, path.rs and facts.rs for the guided review order, fact questions and hints, review.rs for the review packet), assess/ (score; gene_limit.rs for a gene of uncertain significance), combining_table.rs (the 2015 table as rows, with marked additions), reevaluate.rs (reported variants, condition-aware), case.rs (file format; case/review.rs and case/progress.rs for review decisions and progress), condition.rs (condition, inheritance, how two conditions compare), variant/ (notation parser, build table, the question for the Allele Registry, the saved identity), benchmark.rs + tests/erepo_benchmark.rs (expert-panel classifications, dated snapshot in tests/data)
crates/classivar-sources/    ClinGen, G2P, PanelApp, ClinGen Allele Registry (allele_registry.rs), ClinVar (clinvar.rs) clients; shared HTTP rules and caps; fixtures in tests/fixtures
crates/classivar/            GPUI app: main.rs, view.rs (Workbench state), view/*.rs (actions, records, gene lookups, save/open, unsaved.rs, helper_state.rs), view/panels/ (drawing, one file per panel) and view/panels/helpers/ (one file per evidence helper), design.rs (the app's own colours), genes.rs, settings.rs
                             The pictures that explain a class are panels too: track.rs (the interactive evidence track; evidence_stack.rs holds its step layout), combining_table.rs, what_if.rs, timeline.rs and strength_ladder.rs. point_axis.rs holds the axis the track, the "what if" bars and the timeline share. Their layout rules are plain tested functions; the drawing on top is thin.
                             The window: a rail (reports over time, `rail.rs`), the stage, and the editor sheet. The class is drawn as a dial, column or track (`picture.rs`, `dial.rs` + `dial_geometry.rs`, `column.rs`, `track.rs`); the choice is saved in settings. `look.rs` is the design system (type roles, panels, segmented control).
                             view/panels/identity_card.rs: the checked variant as a card (`identity_view` is the tested words). view/real_case.rs (tests only): the published RYR1 case the window tests load.
                             view/types.rs: the plain screen types (Choice, Record, Lookup, Boxes, Handles, StageMemory). view/variant.rs + variant_state.rs: the variant check, condition and ClinVar import; view/clinvar_records.rs: submissions to records (tested); view/rules_window.rs: Rules… opens another rule file in a new window; rule_file.rs: reading a rule file (start-up and Rules…)
                             view/guided.rs + view/panels/guided_page.rs, guided_facts.rs, guided_row.rs, guided_strip.rs, helpers/apply.rs: the guided review (rule-file steps, optional fact questions and sourced hints, decisions, progress, Replace/Keep/Reconsider for helper readings); view/headless_guided.rs tests it
                             view/headless.rs: tests that draw the real window in GPUI's test platform (see "Verify a UI change"); view/headless_case.rs: the variant, condition, ClinVar, PVS1 gene-level and note paths; view/headless_keys.rs: the same window with real key events (Tab, Enter, Escape)
                             dev_reload.rs, view/reload.rs: #[cfg(debug_assertions)] live reload for assets/classivar-theme.json (with WCAG contrast warnings) and --rules; view/headless_dev.rs: headless tests for live reload and --open
docs/                        ROADMAP.md is the current status; ARCHITECTURE.md is the map; AUDIT is historical; design/prototype/ (tokens.css, starter.html, README.md) is the HTML/CSS prototype kit
```

Key types: `RuleSet` → `Applied` → `Assessment` (`class()`, `points_class`, `classic_class`, `conflicts`, `both_sides_met`) → `Reevaluation`/`Verdict` → `CaseFile`.

## Invariants to preserve

- `Assessment::class()` is `None` when there is no counted evidence, a contradictory pair is applied, both sides of the 2015 table are met while the points would give a confident class, or a combined limit is exceeded. Do not make the UI invent a class in those states.
- Earlier records are scored with `Counting::AsRecorded`, so a strength that is capped today still opens. Never "repair" an old record silently.
- A new assessment of a guided criterion (PVS1) must carry a guide answer id; the engine rejects a bare strength. Keep guide ids stable: saved cases refer to them. Rebuilding a missing answer is only allowed when exactly one guide answer sets the strength (`unique_situation_for`); otherwise ask, never guess. The criteria the helpers set (`HELPER_CODES`) refuse a guide at load time.
- `start_from_latest` must lower copied strengths to today's limits, fill the matching guide answer, and say so.
- `Assessment::class()` also withholds when a PP1+PP4-style combined limit is exceeded; do not make the UI invent a class.
- A variant is never guessed: a bare position needs the build the curator chose, several alleles for one id wait for a choice, and a failed lookup says it failed. The saved identity is what the Allele Registry returned, nothing filled in.
- A record for a different condition (different MONDO ids) is never compared with today's class. Names alone can show sameness, never difference.
- With the gene-disease link marked doubtful, a pathogenic class is shown as VUS (`Assessment::in_gene_of_uncertain_significance`); the points are not changed.
- Today's evidence is added only through `Workbench::count_today`, which ends the criterion's "does not apply" or "deferred" decision. A review decision or hint never counts as evidence.
- PVS1's gene-level answer (`gene_level` on `Applied`) is required for a new assessment and is never assumed when a case opens; keep its ids stable like guide ids.
- Criteria from ClinVar are never applied for the curator; a lab's codes stay text in its record's source.
- An AI sign-off never makes the rules "reviewed". `status = "reviewed"` also needs a human sign-off on every entry of `rules/review-signoff.toml`.
- The ERepo benchmark floors (98% agreement on rows with evidence, no class on the other side of VUS) hold in the merge gate. A rule change that lowers them must be explained in `docs/ROADMAP.md`.
- Every new button needs a stable id, a visible label, and `accessibility_label` when the label is a glyph or abbreviation. Every text box gets an `aria_label`.
- There is no demo or fictional case in the app, and no learning mode: rule text opens on demand ("More"). The window tests use a real published case instead (`view/real_case.rs`: RYR1 CA024770 with two ClinGen expert-panel classifications); keep test fixtures real.
- Colours: use theme tokens for chrome, and the checked WCAG pairs in `design.rs` (`class_fill`/`class_ink`, `direction_colour`/`direction_ink`, warning pair) for meaning. Do not write a raw `rgb(...)` in a view file. Extend the contrast test for both themes if you add a pair.
- The debug build once overflowed the 1 MB Windows stack. Keep render functions small; `build.rs` requests 16 MB as a second safety net.
- Saved case files contain variant, gene and lab history. `*.classivar.json` is git-ignored; keep it that way. `CaseFile::from_json` caps field lengths and the record count. The variant, lab-name and year limits are public constants in `case.rs` that the app types against, so there is one number to change. The gene symbol shape is `classivar_sources::is_gene_symbol`.
- Keep dev helpers out of release builds: `--open`, `--theme`, `dev_reload.rs` and `view/reload.rs` stay gated behind `#[cfg(debug_assertions)]`. Release builds accept only `--rules` and run no file watchers.
- No user-overridable runtime UI shell: keep the production UI in tested Rust/GPUI rather than a runtime script layer, so `UNVERIFIED` status, withheld classes, WCAG contrast and `accessibility_label`/`aria_label` hooks remain compile-time and test-enforced.
- Prototype major UI changes first: start new screens or large layout changes from `docs/design/prototype/starter.html` (using real RYR1 data, never fictional cases) and verify Light/Dark, 1240×900/960×600, Empty/Withheld states and the `UNVERIFIED` pill before porting to Rust/GPUI.

## Verify a UI change

The unit tests do not draw the window, and a stack overflow or a bad binding only appears on screen (the headless tests catch a panic in a render function, not a wrong binding). After a UI change:

1. `scripts/check.ps1`. It includes the headless window tests (`crates/classivar/src/view/headless.rs`): they draw the real window in GPUI's test platform with no screen, mouse or keyboard, so run them freely, also while the user is working. Most set state directly and do not click. `headless_keys.rs` sends key events and reads which element has the focus: use it for any keyboard path. A button answers Enter on key release, so send both the press and the release (see `press_enter` there).
2. `scripts/gui-smoke.ps1`. **It takes over the user's mouse and keyboard and must not run while the user is using the machine** (it also fails when they touch it). Ask first, or run it only when the user says the machine is free. (it never calls the live sources, so it runs offline; it refuses to act unless ClassiVAR is the foreground window; screenshots go to `%TEMP%\classivar-smoke`)
3. Click the changed path yourself if the smoke script does not cover it (Save/Open dialogs and keyboard are not covered).
4. If you touched `#[cfg(debug_assertions)]` code, also run `cargo check -p classivar --release --locked` (the merge gate builds debug only, so release-mode unused imports or dead code would otherwise be missed).
5. When creating or editing a prototype in `docs/design/prototype/`, verify both Light and Dark themes and avoid `vh` units or `height: 100%` on `html`/`body` so browser and inline previews render cleanly.

## Gotchas

- Theme preference: `%APPDATA%\ClassiVAR\settings.txt` (`theme=dark|light`).
- A case saved under other rules says so on open; per-record rule sets are intentionally not saved.
- Live source tests hit real services; run them separately, not in the merge gate.
- CI (`.github/workflows/`) runs the merge gate on Windows and macOS plus `cargo audit`; the first fully green run is linked in `README.md`. It has run with the headless window tests and passed on Windows and macOS (run URL in `docs/ROADMAP.md`, Phase E item 7). The Windows UI smoke test is not part of CI: run it yourself after a UI change.
- Do not commit unless the user asks.
- Size limits: files of 300 lines (a test block in the same file may go over) and functions of 50 lines hold in all three crates. Keep it that way. A new helper is one state type in `view/helper_state.rs`, one file in `view/panels/helpers/` with a tested function that reads the typed values, and one entry in `view/panels/helper_bar.rs`.
- Saved text with non-ASCII characters (`•`, `·`, `→`) is easy to damage when a script reads a file with the wrong encoding. After a scripted edit, search for `â€` and `Â`.

## When you change behaviour

Update, in the same change:

- `docs/ROADMAP.md`: what is done, what is next, and any decision the user must make.
- `docs/ARCHITECTURE.md`: layout, data flow, test counts, known problems.
- `docs/MVP.md` and `docs/design/design-brief.md` only if scope or the design claim changed.
- `docs/design/prototype/tokens.css` (and the inline fallback block in `starter.html`) when colours or layout tokens change in `assets/classivar-theme.json`, `design.rs` or `look.rs`.
- `rules/acmg-amp-points.toml` header/notes if clinical content changed.
