# Architecture and framework

## Framework decision

| Choice | Value | Why |
|---|---|---|
| Language | Rust (1.98 installed, edition 2024) | Strong compile-time checks suit classification logic. |
| UI | **GPUI Kit 0.9** (`gpui-kit` from Longbridge), which re-exports Zed's GPUI | The owner chose GPUI to learn and enjoy it. Kit adds ready-made components (buttons, inputs, dialogs, tabs, charts, themes) so we do not build them. Chosen by the owner on 2026-11-03. |
| Rule data | TOML files | Readable by a reviewer, allows comments, easy to diff. |
| Platforms | Windows now (built and run), macOS next | Team uses both. GPUI Kit supports both; each needs its own build. |

**Risks of this choice.** GPUI Kit and GPUI are young and change quickly. The bundled guides we have describe 0.7, so every API is checked against the 0.9 source when used. Keep `Cargo.lock` in version control and upgrade on purpose, not by accident. The toolchain is pinned in `rust-toolchain.toml`.

## Principles

1. **The rules engine knows nothing about the screen.** It lives in `classivar-core` and is tested alone. A web or command-line front end could reuse it later.
2. **Rules are data.** Changing a rule means editing `rules/*.toml`. Every rule has a `source`, checked at load time.
3. **Validate once, at the edge.** The rule file is checked when loaded. User input is checked when it enters the engine (`AssessError`). Inside, values are trusted.
4. **The curator decides.** The engine never picks criteria.
5. **One owner for each piece of state.** In the UI, the `Workbench` view owns the applied criteria. Parts that only display take values and callbacks.

## Folder layout

```
ClassiVAR/
├─ Cargo.toml              workspace
├─ Cargo.lock              tracked in git: reproducible builds
├─ rust-toolchain.toml     pinned Rust version
├─ .github/workflows/      ci.yml (fmt, clippy, test, build, audit); live-sources.yml (weekly)
├─ rustfmt.toml
├─ AGENTS.md               how an AI agent or new contributor works here
├─ scripts/                check.ps1, check.sh, gui-smoke.ps1 (Windows)
├─ rules/
│  └─ acmg-amp-points.toml the rule set (criteria, points, classes, sources, guides, caps)
├─ crates/
│  ├─ classivar-core/       rules engine, no screen code
│  │  └─ src/
│  │     ├─ model.rs       Strength, Direction, Category, Class
│  │     ├─ rules.rs       RuleSet: load + check the TOML (also when read through serde)
│  │     ├─ assess.rs      scoring, 2015 table, conflicts, warnings, tests
│  │     ├─ case.rs        the saved-case file: JSON, versioned, checked once on opening
│  │     └─ reevaluate.rs  reported variant: records vs today, reasons, verdict
│  ├─ classivar-sources/   lookups: ClinGen, G2P, PanelApp (source + date, or a clear failure)
│  │  └─ src/              lib.rs (shared HTTP rules), clingen.rs, g2p.rs, panelapp.rs
│  └─ classivar/           the desktop app (GPUI Kit)
│     ├─ build.rs       asks the Windows linker for a 16 MB main-thread stack (see Known problems)
│     └─ src/
│        ├─ main.rs        checks the rules, starts the app, opens the window
│        ├─ genes.rs       the gene-symbol fill-in list (typing aid, not a check)
│        ├─ settings.rs    the one saved preference: light or dark theme
│        ├─ view.rs        the state and the actions of the main screen (the view type is called `Workbench`)
│        ├─ view/panels.rs the drawing: top bar, case panel, criteria list, result, verdict
│        └─ view/panels/helpers.rs the criteria helpers: predictor score, functional assay, family and phenotype points, non-segregation, reason prompt
└─ docs/
   ├─ PRD.md
   ├─ MVP.md
   ├─ ARCHITECTURE.md      this file
   ├─ DEVELOPING.md        build, test, UI smoke, recipes and debugging
   ├─ ROADMAP.md           what is built and what is next
   ├─ AUDIT-2026-11-04.md  the audit
   └─ design/design-brief.md   the design (draft), with decisions and open items
```

**Planned crates (not created yet; added when the work starts).** The sources crate already exists as `classivar-sources`; ClinVar and OMIM will be added to it.

| Crate | Purpose | Milestone |
|---|---|---|
| `lessons` | Lesson and practice-case content as data files plus a loader. | M6 |
| `report` | Export a case to PDF or slides. | v2 |

## How data flows

```
rules/*.toml ──load+check──▶ RuleSet ──┐
                                       ├─▶ RuleSet::assess(&[Applied]) ─▶ Assessment ─▶ screen
curator clicks (Workbench state) ──────┘
```

- `Workbench` holds today's evidence and the records, each a `BTreeMap<code, Choice>` (`Choice` = strength + optional guide-answer id).
- On each draw it builds `Vec<Applied>` and calls `assess` (and, in the reported path, `reevaluate`). The result (points, classes, lanes, conflicts, combined-limit breaches, warnings, next steps) is plain data; the view only draws it. If a result cannot be scored, the screen shows the error instead of stopping.
- `Assessment::class()` is `None` when no criterion is counted, when two criteria contradict each other, when both sides of the 2015 table are met but the points disagree (a confident class would hide the conflict), or when a pair exceeds its combined limit. The screen then shows no class and says which criteria caused it.
- `Assessment.frameworks_agree()` says whether the points and the 2015 table give the same class. A difference is shown as information, not as an error.

## Data sources and how fresh they are

Checked on the web on 2026-11-03. The lookups in `classivar-sources` have live tests (`cargo test -p classivar-sources -- --ignored`) that call the real services; run them before each release and after any report of a wrong answer. A changed address or format is meant to show up as an error, never as "nothing found". Requests identify the program, try again after timeouts, rate limits and server errors (three tries, with waits of 1 and 2 seconds), and cap the size of every answer. The user-agent does **not** carry a contact address yet; EBI-style services expect one, so add it before the team trial.

| Source | How we use it | Access | Notes |
|---|---|---|---|
| ClinGen gene-disease validity | one CSV download, kept in memory 6 h | public, no key | Official file at `search.clinicalgenome.org/kb/gene-validity/download`. Header rows first. Columns are read by name; a missing header row or column, an unknown rating, or a file with no data rows is an error, and such a file is never cached. |
| Gene2Phenotype (EMBL-EBI) | `/gene2phenotype/api/gene/{symbol}/summary/` | public API | Answers 404 with a JSON `error` message for a gene it does not hold; any other 404 is treated as an error. Same data DECIPHER shows. |
| PanelApp England | `/api/v1/genes/?entity_name=` | public API | Paged. Next-page links must stay on the same host, and the number of entries received must equal the service's own `count`. |
| PanelApp Australia | same format, `panelapp-aus.org` | public API | Reported by the owner to update faster. |
| GenCC | not used yet | weekly CSV (about 27 MB, CC0, no API yet) | Would add Orphanet, Ambry, Labcorp and others. Plan: download once, cache on disk with its ETag, refresh weekly. |
| ClinGen rule sets (CSpec registry) | not used yet | JSON-LD API, public (checked 2026-11-04: `/cspec/api/SequenceVariantInterpretation/id/GN016` returns 200) | Today the app cannot tell whether a gene-specific rule set exists. Next step: use the API to list the rule sets for a gene. |
| OMIM | not used yet | registered key (requested) | |
| DECIPHER | not used directly | its gene page shows G2P, ClinGen and GenCC data that we read from the sources themselves | |

## Rule content: what was checked against the web

On 2026-11-03 the builder (not a qualified reviewer) compared the rule file with: Richards 2015 Tables 3 to 5 (the combining table matched; several criteria summaries are paraphrases, and PVS1, PS4 and BP1 wording was corrected on 2026-11-04 after the audit); Tavtigian 2020 (1/2/4/8 points; Pathogenic 10 or more, Likely Pathogenic 6 to 9, Likely Benign -1 to -6, Benign -7 or less, via its published summary); the ClinGen PM2 recommendation of 4 September 2020 (PM2 at Supporting; frequency limits differ between gene-specific rule sets); the 2018 ClinGen proposal to stop using PP5 and BP6 (ACMG/AMP made no formal recommendation to remove them). Two status facts were also checked: the ClinGen SVI working group was retired in April 2025 and its recommendations now live on the ClinGen Variant Classification Guidance page; the revised standards (SVCv4.0) were still unpublished, in pilot, when checked.
Added after the audit of 2026-11-04: the ClinGen 2020 combining addition (one Very Strong + one Supporting = Likely pathogenic), withholding the class when contradictory criteria are applied, discouraged criteria listed but not counted in new assessments, and stand-alone benign evidence (BA1) fixed at Benign rather than allowed to be outvoted by points.
Added in the Phase C pass: the PVS1 situation guide from Abou Tayoun 2018 (nine situations, including "does not apply", with the chosen id saved in the case file); PVS1 and PM4 cannot both count; a strength cap on weak evidence (Supporting-default and benign criteria) that does not apply to earlier records; the both-sides rule (a VUS when both methods agree, a withheld class when the points would be confident); and the ClinGen 2024 cap of 5 points on PP1 + PP4. BP4 is now capped at Moderate and PP3 at Strong, per Pejaver 2022. Rule version 0.6.0 (2026-11-04) added the Brnich 2019 functional-evidence limits (`read_functional_controls`, `read_oddspath`); an assay with no validation controls gives no evidence (`controls_supporting_min = 1`); a case file now also carries an optional `reason` for each criterion. Rule version 0.5.0 (2026-11-04) added the calibrated PP3/BP4 score limits for REVEL, BayesDel (no allele frequency), MutPred2 and VEST4 (Pejaver 2022, Table 2, read from the open-access paper) and the Pejaver limit of Strong on PP3 + PM1 together. The engine reads a score with `RuleSet::read_predictor`; a published benign level above Moderate is lowered to the BP4 ceiling and shown as such. The helper writes the tool, score and source into the criterion's `reason`, which is saved with the case. Rule version 0.7.0 (2026-11-04) added the ClinGen 2024 PP1/PP4 points arithmetic (Tables 2 to 4, read twice from the open-access paper): `RuleSet::pp4_points` (which divides the yield, never the points, when variants share an allele), `pp1_points` and `read_locus` (which also lists only PP4 choices when one gene causes the phenotype) give the points, the total cut to 5 and the strength choices; a helper in the criteria list applies one choice to PP1 and/or PP4 with the points (and the sum before the limit) saved as the reason. The helper asks whether other genes can cause the phenotype, and does not count PP1 until that is answered or when only one gene does (the paper's locus homogeneity). The points are fractions, which the engine's whole-number scoring does not hold, so the helper only does the paper's arithmetic and the curator applies a whole-number strength from Table 4. Rule version 0.8.0 (2026-11-05) added the BS4 situations from the same paper: `RuleSet::read_non_segregation` gives BS4 Strong only for the situations where the paper supports it (its −4 points equal BS4's existing default, so no points changed), and a helper shows the paper's reason where it does not. Still to add and verify: splicing (2023), the BA1 exception list, and PVS1's gene-level downgrade (Table 1 of Abou Tayoun 2018).

## Rule file format (summary)

`[meta]` (id, version, `status = unverified | reviewed`; `reviewed` also needs `reviewed_by` and `reviewed_on`), `[[sources]]`, `[points]`, `[class_min_points]`, `[[vus_tiers]]` (optional `min_points`), `[[exclusive_pairs]]`, `[[combination_caps]]` (two criteria, `max_points`, reason, source), `[locus_evidence]` (source, `cap_points`, `yield_text_minimum`, `homogeneity_text`, `notes`, a `pp4_yield` table, `segregation` kinds with `points_each`, and Table 4 `bands` whose options name a PP1 and/or PP4 strength; the loader refuses an option worth more program points than its band's lower edge), `[non_segregation]` (source, the BS4 `strength`, and `[[non_segregation.situations]]` each with an id, label, `applies` and text; at least one must apply), `[functional]` (source, `controls_moderate_min`, OddsPath `pathogenic` and `benign` limit lists, crossed strictly), `[[predictors]]` (id, name, source, optional note, `pathogenic` and `benign` lists of `{ strength, at }` score limits, weakest first; load fails when limits run the wrong way, overlap, or cite an unknown source), `[[criteria]]` (code, direction, default strength, category, title, summary, source, optional `max_strength`, `original_strength`, `strength_source`, `discouraged`, `note`, and a `guide_source` with `[[criteria.guide]]` entries: stable `id`, `short` label, `situation` text, and an optional `strength`; no strength means "does not apply").
Loading fails if a criterion or tier cites an unknown source, a code is duplicated, thresholds are out of order or VUS does not start at 0, the points are not strictly increasing or BA1 alone cannot reach Benign, the file has no pathogenic or no benign criteria, a default strength is above its `max_strength` (or a cap cannot be placed), a guide has no source, an empty/duplicate situation or id, no answer at all, or a strength the criterion may not use, `original_strength` repeats today's default, an exclusive pair repeats itself or names a missing criterion, a combination cap is below the pair's default total or names a missing criterion, a `reviewed` file has no reviewer or date, or VUS tier ranges are partial, repeated or outside the VUS range. The same checks run when a `RuleSet` is read through serde. A reviewer hash is not implemented yet.

## Supporting several rule-set versions (needed for SVCv4.0)

`RuleSet` is a value, not a global. `RuleSet::builtin()` is only the default (it returns an error instead of panicking, and the program checks it at start-up). The program can also load a file with `--rules`. Every `Assessment` carries the `id` and `version` of the rules that made it (`RuleSetRef`), and a record can carry one too, so a changed rule set can show up as a reason. `CaseFile` writes the case's rule reference and version, and a case saved under other rules says so when it is opened. Per-record rule references are not saved: for a record from an outside lab the rules used are usually unknown. When SVCv4.0 arrives, add a second file and let the user pick.

## Testing strategy

- **Engine** (103 tests at 2026-11-05): class edges, the 2015 table rows and the ClinGen 2020 addition, both-sides (VUS when the methods agree, withheld when the points would be confident), contradictory pairs, combined-limit pairs, strength caps, stand-alone BA1, guided criteria (a new assessment must answer the guide), discouraged criteria, points versus table disagreement, bad input, VUS tiers, the reported-variant verdicts, the rule-file checks, the calibrated PP3/BP4 score tables (every REVEL limit and the capped benign levels), the PM1+PP3 limit, the PS3/BS3 OddsPath and control-count limits (every limit crossed strictly), the saved reason for a strength, the PP4 yield table (all 23 rows against an independent copy, and just under each row), the PP1 co-segregation table and the Table 4 strength choices of ClinGen 2024 (every band, the 5 point limit, allele sharing by dividing the yield with the paper's 95.8% example, PP4-only choices when one gene causes the phenotype, points that are negative or not numbers refused, a sum that lands on a band edge only after rounding, a choice worth more points than its band, a limit that differs from the PP1/PP4 combination cap, or a non-standard point scale is refused, and a rule file without the table still loads), BS4 (applies only in the situation the paper supports; its strength is the paper's −4 points; a table that could never offer BS4 is refused), and the case file round trip. Saved-case tests also check unchanged assessments after reopening, older files without reasons, and the 500-character reason limit in both current and earlier evidence, including non-ASCII text.
- **Rule file:** a test loads the built-in file and checks counts (28 criteria: 16 pathogenic, 12 benign) and that every source resolves; malformed guides, caps, pairs and original strengths are rejected.
- **Sources** (29 offline tests, 5 live): sample responses for every parser, including the failure cases (an empty footer row in the ClinGen CSV is skipped instead of failing every lookup). The live tests run weekly in CI (best effort: GitHub disables schedules after 60 days of inactivity) and before each release.
- **UI** (14 tests): colour contrast for the class fills and their ink in both themes, the warning colours, file names (including Windows device names), the points scale against the rule set, the example data, the gene fill-in list, the theme-file parser, and bringing old evidence onto today's caps. The Windows smoke script (`scripts/gui-smoke.ps1`) drives the real window: light → dark → light with the saved choice, gene suggestions, the PVS1 guide and Escape closing it, the predictor score helper (apply, between-limits, capped benign), the functional helper (both routes), the family and phenotype helper (PP1 withheld until the gene question is answered and when only one gene causes the phenotype, the 5 point cut, applying one Table 4 choice, an allele shared by two variants, a yield below the table or above 100%), the non-segregation helper (BS4 offered for one situation and withheld for the other four), the reason prompt for a raised strength, Page Up and Page Down, renaming a record, the confirmation window (and Escape cancelling it), and clearing the gene box. Not yet checked by hand: the Save and Open dialogs, keyboard use, macOS. Next: automated GPUI UI tests.
- **Checks to run before every merge:** `scripts/check.ps1` (Windows) or `scripts/check.sh`, which run fmt, clippy, tests and build. CI runs the same commands (`.github/workflows/ci.yml`, never run on GitHub yet).

## Build and run

```
cargo run -p classivar            # run the app
scripts/check.ps1                 # the merge gate (scripts/check.sh on POSIX)
scripts/gui-smoke.ps1             # Windows: drive the real window and save screenshots
cargo run -p classivar -- --rules path/to/file.toml   # try another rule file
```

Full instructions, recipes and debugging: `docs/DEVELOPING.md`.

First build compiles GPUI and takes a few minutes. Windows needs the Visual Studio C++ tools, a Windows SDK and CMake (all present on this machine). macOS has not been built yet.
Case files are ignored by git (`*.classivar.json`), and the save dialog starts in Documents, so a saved case cannot be committed by accident.
Optional speed-up for dev runs: set `opt-level = 3` for the GPUI crates in `[profile.dev.package]` (see the GPUI Kit installation guide). Not applied yet.

## Distribution (later, M8)

Windows installer and a macOS app bundle, each built and run on its own platform. Signing and auto-update are decisions for M8. Single-binary replacement is not an update strategy.

## Implementation plan

The phases are now in `docs/ROADMAP.md`. This list is kept for history.

1. **Done:** project, rules engine, rule file, first screen, reported-variant engine and view, four gene checks (M0 to M4 first slices). The sources crate exists as `crates/classivar-sources`: blocking lookups, each returning `Fetched<T>` (source and UTC date) or a `SourceError`. The screen runs them on a background thread and ignores answers that arrive late.
2. Add automated UI tests, a keyboard pass and a macOS build. Dark mode is built (a top-bar switch, saved between runs) with theme-aware class colours; a hand pass in the real window on 2026-11-04 found it readable.
3. Finish M3: a text box for lab names (the 0.9 input API differs from the 0.7 guides, so read its source first), more sample variants. VUS tier text per record is done.
4. Add sources to `classivar-sources` one at a time: PanelApp is done; next DECIPHER, ClinGen, ClinVar, then OMIM when the key arrives (M4, M5). Each gets offline tests on a saved sample response, plus an ignored live test.
5. Lesson and practice-case formats, ten solved cases, lessons 0 to 6 (M6).
6. Send the rule file and lessons to the reviewer; fix; mark `reviewed` (M7).
7. Packaging and team trial (M8).

## Known problems and how they were handled

- **Stack overflow on Windows (found 2026-11-04).** Clicking "Reported variant" closed the app with "thread 'main' has overflowed its stack". Windows gives the main thread 1 MB, and the debug build of the old, very large `render` needed more. Two changes: `crates/classivar/build.rs` asks the linker for 16 MB, and the screen is now built from many small functions that each return an `AnyElement`. Tests did not catch it because no test draws the window. `scripts/gui-smoke.ps1` and a real click-through do.
- **A case saved under wider rules (Phase C).** A strength that was legal when a case was saved must still open. Opening validates today's block as recorded, then lowers a capped strength to today's limit and says which criteria changed. A guided criterion with no saved answer gets the matching guide option, so old cases are not rejected.
- **PVS1 is a guide, not a default (Phase C).** A new assessment cannot apply PVS1 without answering which situation fits; the engine rejects it. Earlier records keep the strength the laboratory used, with no guide answer required.
