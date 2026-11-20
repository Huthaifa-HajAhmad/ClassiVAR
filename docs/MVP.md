# MVP

The workbench explains how chosen evidence leads to a classification and how that evidence compares with an earlier report.

The opening screen offers both workflows and a separate fictional case.

## In scope

Status on 2026-11-04 is in `docs/ROADMAP.md` and in `docs/AUDIT-2026-11-04.md` section 4. In short: items 1 and 2 work and a case can be saved and opened, 3 is partly done, and 4, 6 and most of 5 are not built yet.

1. **New-variant path:** apply criteria, change strengths, see points, class, lanes, warnings, and what would change it.
2. **Reported-variant path:** enter the existing class and what each lab used; see a side-by-side history against today's rules, with a verdict ("holds", "upgrade candidate", "downgrade candidate") and reasons.
3. **Gene checks:** gene-disease validity and gene-specific rule-set lookup (OMIM, DECIPHER, PanelApp, ClinGen) with manual override and "source and date" on every answer.
4. **ClinVar record import** for the reported-variant path.
5. **Learning layer** on every criterion, and a short lesson set (see below).
6. **Practice shelf** with a small set of solved cases.
7. **Windows and macOS builds.**

## Out of scope for the MVP

Population and prediction fetching, gene-specific rule overlays, the saved re-check list, slide and PDF export, shared cases, web build, VUS tier numbers, accounts.

## Lesson set for the MVP (proposal)

0. The framework and its vocabulary
1. Evidence types and where to find them
2. Strength and points
3. Combining evidence (points and the 2015 table)
4. ClinGen refinements (PM2, PVS1 tree, predictor thresholds, functional evidence)
5. Re-evaluating a reported variant
6. VUS sub-tiers and reporting (Bennett 2025, Mighton 2026)

## Milestones

| # | Milestone | Done when |
|---|---|---|
| M0 | Spike: GPUI Kit runs and draws on Windows | **Done 2026-11-03.** Builds, opens, draws the points scale; clicks work. macOS not yet tried. |
| M1 | Rules engine with tests and the built-in rule file | **Done 2026-11-03, hardened 2026-11-04** after the audit and the Phase C safety pass. 73 engine tests at the time (133 at 2026-11-08), 28 criteria, 2015 table rows, the ClinGen 2020 addition and the PVS1 guide covered. Content still UNVERIFIED. |
| M2 | New-variant screen | **Done 2026-11-03; redesigned and audit-fixed 2026-11-04** (variant name, class withheld when criteria contradict, no class without evidence, contrast, confirm steps, saving and opening a case, light/dark switch, PVS1 situation guide). Needs: automated UI tests and a keyboard pass. |
| M3 | Reported-variant view | **Done 2026-11-03; phase B 2026-11-04.** Engine (`reevaluate`, 16 tests), records strip, history table, reasons and verdict work in the real app with a fictional example, and each VUS record can name its sub-tier with the definition and source shown. Remove now asks first, and record labels stay unique. Still needed: lab names typed by hand (text box), ClinVar fills (M5), three more sample variants. |
| M4 | Gene checks | **Four sources done 2026-11-03** in `classivar-sources`: ClinGen gene-disease validity, Gene2Phenotype, PanelApp England, PanelApp Australia. 29 offline tests, 5 live tests run and passing (2026-11-04). Shown in the app, each with its own date; a failure is shown as a failure; "not found" only when the service says so; your own yes/no answer on the gene-disease link, and a warning stays until you confirm. Still needed: GenCC (bulk file, no API yet), gene-specific rule-set (VCEP) lookup (the ClinGen CSpec registry has a public JSON API that is not used yet), OMIM (key pending), saving downloads to disk for offline use, smarter choice of which panels to show. |
| M5 | ClinVar import | A ClinVar record fills the reported-variant view |
| M6 | Lessons and practice shelf | Lessons 0-6 written with sources; at least 10 solved cases; answer stays hidden until commit |
| M7 | Content review | A qualified reviewer has checked the rule file and lessons; status changes to "reviewed" |
| M8 | Packaging | Installer for Windows and a macOS app that a colleague can run unaided |

## Acceptance for the MVP

- Every number and rule on screen traces to a source in the rule file.
- The app never classifies without the curator choosing criteria.
- All automated tests pass; the app was launched and used on both platforms.
- The rule set is marked "reviewed" by someone other than the author, or the app still says UNVERIFIED everywhere it shows a class.
