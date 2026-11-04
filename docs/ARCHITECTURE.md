# Architecture and framework

## Framework decision

| Choice | Value | Why |
|---|---|---|
| Language | Rust (1.98 installed, edition 2024) | Strong compile-time checks suit classification logic. |
| UI | **GPUI Kit 0.9** (`gpui-kit` from Longbridge), which re-exports Zed's GPUI | The owner chose GPUI to learn and enjoy it. Kit adds ready-made components (buttons, inputs, dialogs, tabs, charts, themes) so we do not build them. Chosen by the owner on 2026-11-03. |
| Rule data | TOML files | Readable by a reviewer, allows comments, easy to diff. |
| Platforms | Windows now (built and run), macOS next | Team uses both. GPUI Kit supports both; each needs its own build. |

**Risks of this choice.** GPUI Kit and GPUI are young and change quickly. The bundled guides we have describe 0.7, so every API is checked against the 0.9 source when used. Keep `Cargo.lock` committed and upgrade on purpose, not by accident.

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
├─ Cargo.lock              committed: reproducible builds
├─ rustfmt.toml
├─ rules/
│  └─ acmg-amp-points.toml the rule set (criteria, points, classes, sources)
├─ crates/
│  ├─ classivar-core/       rules engine, no screen code
│  │  └─ src/
│  │     ├─ model.rs       Strength, Direction, Category, Class
│  │     ├─ rules.rs       RuleSet: load + check the TOML
│  │     ├─ assess.rs      scoring, 2015 table, warnings, tests
│  │     └─ reevaluate.rs  reported variant: records vs today, reasons, verdict
│  └─ workbench/           the desktop app (GPUI Kit)
│     └─ src/
│        ├─ main.rs        starts the app and opens the window
│        └─ view.rs        the main screen
└─ docs/
   ├─ PRD.md
   ├─ MVP.md
   ├─ ARCHITECTURE.md      this file
   └─ design/design-brief.md   the agreed design, with decisions and open items
```

**Planned crates (not created yet; added when the work starts):**

| Crate | Purpose | Milestone |
|---|---|---|
| `evidence-sources` | Look up OMIM, DECIPHER, PanelApp, ClinGen, ClinVar; return a result with source and date, or a clear failure. Runs off the UI thread. | M4, M5 |
| `lessons` | Lesson and practice-case content as data files plus a loader. | M6 |
| `report` | Export a case to PDF or slides. | v2 |

## How data flows

```
rules/*.toml ──load+check──▶ RuleSet ──┐
                                       ├─▶ RuleSet::assess(&[Applied]) ─▶ Assessment ─▶ screen
curator clicks (Workbench state) ──────┘
```

- `Workbench` holds `applied: BTreeMap<code, Strength>`.
- On each draw it builds `Vec<Applied>` and calls `assess`. The result (points, classes, lanes, warnings, next steps) is plain data; the view only draws it.
- `Assessment.frameworks_agree()` says whether the points and the 2015 table give the same class. A difference is shown as information, not as an error.

## Data sources and how fresh they are

Checked on the web on 2026-11-03. The lookups in `classivar-sources` have live tests (`cargo test -p classivar-sources -- --ignored`) that call the real services; run them before each release and after any report of a wrong answer. A changed address or format is meant to show up as an error, never as "nothing found".

| Source | How we use it | Access | Notes |
|---|---|---|---|
| ClinGen gene-disease validity | one CSV download, kept in memory 6 h | public, no key | Official file at `search.clinicalgenome.org/kb/gene-validity/download`. Header rows first; the code fails loudly if the header row is missing. |
| Gene2Phenotype (EMBL-EBI) | `/gene2phenotype/api/gene/{symbol}/summary/` | public API | Returns 404 for a gene it does not hold. Same data DECIPHER shows. |
| PanelApp England | `/api/v1/genes/?entity_name=` | public API | Paged. |
| PanelApp Australia | same format, `panelapp-aus.org` | public API | Reported by the owner to update faster. |
| GenCC | not used yet | weekly CSV (about 27 MB, CC0, no API yet) | Would add Orphanet, Ambry, Labcorp and others. Plan: download once, cache on disk with its ETag, refresh weekly. |
| ClinGen rule sets (CSpec registry) | not used yet | web pages only; no documented API found | Today the app cannot tell whether a gene-specific rule set exists. Ask ClinGen about API access, or link the registry. |
| OMIM | not used yet | registered key (requested) | |
| DECIPHER | not used directly | its gene page shows G2P, ClinGen and GenCC data that we read from the sources themselves | |

## Rule content: what was checked against the web

On 2026-11-03 the builder (not a qualified reviewer) compared the rule file with: Richards 2015 Tables 3 to 5 (criteria wording and the combining table: matched); Tavtigian 2020 (1/2/4/8 points; Pathogenic 10 or more, Likely Pathogenic 6 to 9, Likely Benign -1 to -6, Benign -7 or less, via its published summary); the ClinGen PM2 recommendation of 4 September 2020 (PM2 at Supporting; frequency limits differ between gene-specific rule sets); the 2018 ClinGen proposal to stop using PP5 and BP6 (ACMG/AMP made no formal recommendation to remove them). Two status facts were also checked: the ClinGen SVI working group was retired in April 2025 and its recommendations now live on the ClinGen Variant Classification Guidance page; the revised standards (SVCv4.0) were still unpublished, in pilot, when checked.
Still to add and verify: the PVS1 decision tree (2018), calibrated PP3/BP4 thresholds (2022), functional evidence PS3/BS3 (2019), PP1/BS4/PP4 guidance (2023), splicing (2023), the BA1 exception list.

## Rule file format (summary)

`[meta]` (id, version, `status = unverified | reviewed`), `[[sources]]`, `[points]`, `[class_min_points]`, `[[vus_tiers]]` (optional `min_points`), `[[exclusive_pairs]]`, `[[criteria]]` (code, direction, default strength, category, title, summary, source, optional `discouraged` and `note`).
Loading fails if a criterion or tier cites an unknown source, a code is duplicated, thresholds are out of order, or an exclusive pair names a missing criterion.

## Supporting several rule-set versions (needed for SVCv4.0)

`RuleSet` is a value, not a global. `RuleSet::builtin()` is only the default. When SVCv4.0 arrives, add a second file and let the user pick; every saved result records the rule-set `id` and `version`.

## Testing strategy

- **Engine:** unit tests in `assess.rs`: class edges, the 2015 table rows, both-sides-met, points versus table disagreement, bad input, VUS tiers. Add golden cases from published ClinGen examples (M6).
- **Rule file:** a test loads the built-in file and checks counts (28 criteria: 16 pathogenic, 12 benign) and that every source resolves.
- **UI:** launched and clicked on Windows (done for the first slice). Next: automated GPUI UI tests (`gpui_kit::test`), keyboard and focus checks, dark theme, macOS.
- **Checks to run before every merge:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## Build and run

```
cargo run -p classivar            # run the app
cargo test --workspace            # tests
cargo clippy --workspace --all-targets -- -D warnings
```

First build compiles GPUI and takes a few minutes. Windows needs the Visual Studio C++ tools, a Windows SDK and CMake (all present on this machine).
Optional speed-up for dev runs: set `opt-level = 3` for the GPUI crates in `[profile.dev.package]` (see the GPUI Kit installation guide). Not applied yet.

## Distribution (later, M8)

Windows installer and a macOS app bundle, each built and run on its own platform. Signing and auto-update are decisions for M8. Single-binary replacement is not an update strategy.

## Implementation plan

1. **Done:** project, rules engine, rule file, first screen, reported-variant engine and view, PanelApp gene check (M0 to M4 first slices). The sources crate exists as `crates/classivar-sources`: blocking lookups, each returning `Fetched<T>` (source and UTC date) or a `SourceError`. The screen runs them on a background thread and ignores answers that arrive late.
2. Add automated UI tests, a keyboard pass, and a dark-theme pass. Try the build on macOS. The class colors on the points scale and the verdict card use fixed colors; check them against the dark theme.
3. Finish M3: a text box for lab names (the 0.9 input API differs from the 0.7 guides, so read its source first), VUS tier text per record, more sample variants.
4. Add sources to `classivar-sources` one at a time: PanelApp is done; next DECIPHER, ClinGen, ClinVar, then OMIM when the key arrives (M4, M5). Each gets offline tests on a saved sample response, plus an ignored live test.
5. Lesson and practice-case formats, ten solved cases, lessons 0 to 6 (M6).
6. Send the rule file and lessons to the reviewer; fix; mark `reviewed` (M7).
7. Packaging and team trial (M8).
