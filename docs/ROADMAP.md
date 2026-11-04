# Roadmap

Written 2026-11-04, after the audit in `docs/AUDIT-2026-11-04.md`. It replaces the "Implementation plan" list in `ARCHITECTURE.md`.

## Where the project stands

- **Done in the audit pass (2026-11-04):** the findings in stages 0 to 3 of the audit, plus most UI findings. Details are in the table below.
- **Content is still UNVERIFIED.** No qualified curator has reviewed the rules. The program says so on every class.
- **Built since the audit:** saving and opening a case (phase A), reported VUS sub-tiers (phase B), and a redesigned screen. The screen was checked by clicking through the real window on 2026-11-04.
- **Latest (2026-11-04):** the Phase C safety pass (PVS1 situation guide with the answer saved, strength caps, PM4/PVS1, both-sides rule, PP1+PP4 cap, BP4 ceiling), a light/dark switch, gene suggestions, and fixes from a second three-part senior review (Phase E).
- **Not built:** ClinVar import, lessons, practice shelf, a macOS build, a licence.

## What each audit finding became

| Finding | Status | Note |
|---|---|---|
| D1 | Done | Git repo, baseline tag `audit-baseline-2026-11-04`. |
| C1 | Done | PVS1 no longer defaults to Very Strong for your own evidence: it asks which situation fits, using a shortened form of the ClinGen tree (checked against Abou Tayoun 2018). The chosen situation is saved and shown. Earlier records keep what the lab used. |
| C2, C3, C4, C5, C6, C7, C9, C10, C11 | Done | Tests cover C2, C3, C4, C7. C4: every criterion except PVS1, PS2 and PS3 now has a `max_strength` (PP5/BP6 are not counted). These are cautious safeguards chosen for this program, not published limits, and need review. BA1 has only its stand-alone level. Limits do not apply to earlier records. |
| C8 | Partly | Notes added and the screen now says which defaults differ from 2015 (PVS1, PM2, PP5/BP6). The remaining refinements are still phase C. |
| E1, E2, E3, E4, E5, E6, E7, E9, E10, E11, E12 | Done | E6: rules id and version are on every `Assessment` and in every saved case file. A case saved under other rules says so when opened. Per-record rules ids are not saved (the rules used by an outside lab are usually unknown). |
| E8 | Partly | `reviewed` now needs a reviewer and a date. A content hash is not done. |
| S1 to S8 | Done | Live tests pass against the real services (2026-11-04). |
| U1, U3 to U12, U14 | Done | The main flows were clicked through the real window on 2026-11-04. The Save/Open dialogs, keyboard use and macOS are still unchecked. |
| U2 | Done | A variant name box is shown beside every class and is saved in the case file. |
| U9, U13 | Partly | Horizontal scroll, a window minimum size, pressed state for assistive technology and accessible names are in. The pinned Today column and the full-row click target are not. |
| D2 | Partly | `--rules <file>` loads a rule file at run time. There is no in-app file picker. |
| D3 | Written, never run | CI workflows exist but have never run: they were untracked, and the repo had no remote, on 2026-11-04. |
| D7 | Done | Docs updated 2026-11-04; `DEVELOPING.md`, `AGENTS.md` and the scripts are linked from the README and the counts were refreshed. |
| D5 | Done (phase A, 2026-11-04) | Save and open a case file. The Save and Open dialogs were not clicked by hand. |
| D4, D6 | Open | See phase F and "Needs your decision". |

## Phases

Each phase ends with a state you can check. Sizes are rough.

**Next three steps (in order):**
1. **Phase C, item 4 onward** (the calibrated PP3/BP4 thresholds themselves, then PS3/BS3 and splicing). The safety items are done.
2. **Phase E**, the UI follow-up from the code review: a lab-name box, a real modal confirmation, and the Save/Open hand check.
3. **Phase D**, teaching, once the rules stop moving under it.

### Phase A: make the case real (DONE 2026-11-04, except the hand check)
- Save and load a case as a JSON file: variant, gene, evidence, records, the gene decision, rules id and version, app version, time stamp (D5, U2, E6).
- The case file holds the rules id and version for the whole case. A per-record version was dropped: for a record from an outside lab the rules used are rarely known.
- A case with an unknown criterion or a strength the rules cannot read is refused with a message that names it. A strength the current rules cap opens, is lowered to today's limit, and the screen says which criteria changed. Opening such a file in a viewer-only mode is not built.
- Done when: a case closed and reopened gives the same class and verdict, and a file saved under older rules says so. The file round trip is covered by tests in `classivar-core`; the Save and Open buttons need one hand check by clicking.

### Phase B: the question that started the project (DONE 2026-11-04)
- Let the curator enter a reported VUS sub-tier ("VUS-low") per record, and show the tier definitions with their sources (C9, E10).
- Show a reason when the tier changes (the engine already supports it). It cannot fire yet: it needs official tier point ranges, which are not published. The verdict says so instead.
- Built: a "Tier it named" row for each record that reported a VUS; the tier shows in the history table, in the verdict (with its definition and source), in a short reference list, and in the saved case file. For a VUS result the screen shows where in the VUS range the points sit, without naming a tier.
- Done when: "how did this variant end up VUS-low?" can be answered on screen. Yes: a lab's tier is recorded and explained, and the evidence behind it is in the history table.

### Phase C: rule content (1 to 2 weeks, one rule per step)
Add each as a note, then as logic, with a source and a worked example:
1. **Done.** PVS1 strength guide (Abou Tayoun 2018): nine situations with short labels, including "does not apply". The chosen situation is saved in the case file and shown on the row. The engine refuses a new PVS1 with no guide answer. Still shortened: it does not walk the tree question by question or apply the gene-level downgrade (Table 1).
2. **Done, needs review.** Strength limits on weak evidence, all recorded as program safeguards. No Supporting-default criterion reaches Very Strong; benign criteria stop at Strong (BP4 at Moderate per Pejaver 2022); BA1 has only stand-alone; PM3 and PM6 stop at Strong. Limits do not apply to earlier records.
3. **Done, with a curator's nuance.** When both sides of the 2015 table are met: if the points also say VUS, the class is VUS (the ordinary conflicting-evidence case); if the points would be confident, the class is withheld with "needs expert judgment" and the criteria on each side are listed. BA1 against evidence that itself reaches Pathogenic is withheld, because known common pathogenic variants exist (the ClinGen exception list).
4. PP3 and BP4 calibrated thresholds (Pejaver 2022). The ceilings are in (PP3 Strong, BP4 Moderate); the score tables are not.
5. PS3 and BS3 (Brnich 2019), and add its citation to `[[sources]]` (it is quoted in two notes but not listed).
6. PP1, BS4, PP4 (2023 and 2024 guidance). Done: the PP1+PP4 combined cap of 5 points, with the class withheld when exceeded. Not done: the full points table and BS4.
7. Splicing (Walker 2023), the BA1 exception list, PS2 and PM6, PM3 point tables. A warning now flags PVS1 + PP3 as possible double counting for a splice variant.
8. A required reason when a curator raises a strength.
9. The PVS1 guide's situation is part of the saved case; changing an `id` in the rule file would break old cases, so keep ids stable and bump the rule version when wording changes.
- Done when: each item has a test with a published worked example.

### Phase D: teaching (1 to 2 weeks)
- Per-criterion panel: rule in plain words, worked example, common mistake, "why this might not apply here".
- Practice shelf: 10 solved cases from a source we may legally use. The answer stays hidden until the learner commits.
- Lessons 0 to 6, tied to the criteria they explain.
- Done when: a new curator can finish a practice case and see the expert answer next to their own.

### Phase E: UI follow-up from the 2026-11-04 code review (2 to 3 days)
The second three-part review (core/sources, GUI, docs/CI) is recorded in this phase. Fixed already in the same pass:
- BA1 has one level only; a single click can no longer turn a >5% variant into Likely benign. Benign criteria stop at Strong, and their levels are the ones the framework defines.
- PVS1's answer is saved (stable id + short label), shown under the criterion, and marked when the guide is reopened. A new assessment cannot score PVS1 without answering the guide; old cases get the matching answer on open.
- Clicking an applied PVS1 reopens the guide instead of deleting it; Remove is a separate action in "More"; Cancel sits at the top and the options have distinct accessible names.
- Strength caps appear in the strength tooltip; BP4 is capped at Moderate per Pejaver 2022; PM3/PM6 at Strong.
- PP1 + PP4 are capped jointly at 5 points (ClinGen 2024) and the class is withheld with the criteria named when exceeded.
- Both-sides withholding now only applies when the points would be confident; a true VUS is reported as VUS. The banner lists the criteria on each side and explains the BA1 exception.
- Typing a variant repaints the result immediately.
- "Remove record" asks first, stays next to the nearest record, and labels never repeat.
- The points scale fits the 960 px minimum window, and the result now comes first in Reported mode with "Result for …" above it.
- Opening/clearing a case resets the guide and questions; the guide and its answer no longer leak into the next case.
- A guide answer no longer appears as a curator "strength change"; a record's note no longer claims a change nobody made.
- A second question while one is open now says "Finish the question above first" instead of dropping the action silently.
- The theme button has an accessible name and a pressed state; the result comes first on both paths and says which record it is for.
- Docs, AGENTS.md, DEVELOPING.md and the check/smoke scripts are linked from the README; the smoke script starts from a known theme and re-checks focus before every synthetic click.

Still open, in rough order:
1. A lab-name box per record (M3). Every record is still "Record N".
2. The confirmation as a real modal: the banner is better than before, but the rest of the screen stays clickable, and a refused question is silent.
3. Record rule provenance: `Reason::RuleSetChanged` cannot be produced from the UI because per-record rule sets are not saved.
4. Keyboard: the criteria list, points scale and history table scroll only with the mouse; the guide has no Escape.
5. A record-count cap and an in-app message when a case file is too large or from a newer layout.
6. Hand checks: Save/Open dialogs, dark theme on macOS, keyboard-only use.
7. Automated GPUI UI tests; splitting `view/panels.rs` by region and making the state machine testable without a window.
8. One colour for one job (success and failure currently share the warning yellow), and a recoverable error screen.

### Phase F: before a colleague uses it (1 week)
- Run CI on GitHub and fix what it finds (D3). The workflows need to be committed and a remote added first. `permissions`, a timeout and concurrency are in; add caching, pin `cargo-audit` and the actions, and consider `cargo deny` for advisories and licences.
- Build on macOS. This has never been tried (D4). Budget a spike first. Also an installer for each platform.
- Choose a licence and add third-party notices (D6). About 800 crates ship in the binary.
- Put a contact address or project link in the HTTP user-agent (E, S8): the services expect one.
- Send the rules and lessons to a qualified curator. Set `status = "reviewed"` only with reviewer and date, and add the content hash (E8).
- Add ClinVar import (MVP item 4).
- Done when: a colleague installs the app unaided and the rule file has a named reviewer.

### Later
Gene-specific rule sets from the ClinGen CSpec API (it works: checked 2026-11-04), OMIM, SVCv4.0 when published, population and prediction data, PDF and slide export.

## Needs your decision
- **Licence (D6).** I did not pick one. "Internal proprietary" is the simplest if the code stays inside your team.
- **BA1 against Pathogenic evidence: decided, please confirm.** The program withholds the class when BA1 and evidence that itself reaches a Pathogenic rule are both applied. The reasoning: a known pathogenic variant can be common (founder effect, reduced penetrance), which is exactly what ClinGen's BA1 exception list is for. BA1 with weaker pathogenic evidence stays Benign.
- **Commit.** Everything since the audit is still uncommitted on the baseline tag. I have not committed; say the word and I will, after one final `scripts/check.ps1` and smoke run.
