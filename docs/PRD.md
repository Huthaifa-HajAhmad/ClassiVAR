# Product Requirements: ClassiVAR

Working title. Status: draft v0.1, 2026-11-03. Owner: the user (physician, MSc Genomic Medicine).
Background and decisions: see `docs/design/design-brief.md`.

## 1. Summary

A desktop app for variant curation. You enter a variant and the evidence, and the app shows how the classification is built: points, class, what is missing, and what would change it. It is meant for real work, with teaching built in that you can switch on.

## 2. Problem

A new curator can follow the ACMG/AMP criteria one by one but cannot yet see the whole picture. Two situations are especially hard:

- **A reported variant** is already classified VUS, VUS-low, Likely Pathogenic or Pathogenic. How did it get there? Does it still hold? Should it go up or down?
- **A never-reported variant** needs a different, evidence-first path.

Reading a paper does not show how evidence adds up. Seeing it does.

## 3. Users

1. **Primary:** the owner, learning curation while doing it.
2. **Later:** colleagues on the owner's team (Windows and macOS), after a qualified reviewer has checked the content.

## 4. Goals

- Show any classification as a visual breakdown: evidence blocks, points scale, class.
- Support two curation paths: new variant, and reported variant.
- Teach on the spot: a rule, its source and an example, one click away.
- Practice on solved cases using the same screen as real work.
- Check gene facts automatically (OMIM, DECIPHER, PanelApp, ClinGen), with manual override.
- Keep rules in data files with sources, so updates are data changes.
- Be polished enough to present from, and to share with a team.

## 5. Non-goals

- Not a validated clinical reporting system.
- Does not store patient names or identifiers.
- Does not replace the curator's judgment or silently classify.
- No accounts, server, or shared database in the first versions.

## 6. Safety principles (anti-goals made concrete)

The app fails if it silently classifies a variant, or gives a wrong answer with confidence. So:

1. **The curator applies every criterion.** The app adds up and explains. Suggestions are allowed later, but never applied automatically.
2. **Every result shows its basis:** the criteria, the rule-set version, the sources, and the review status ("UNVERIFIED" until a reviewer signs off).
3. **Missing, conflicting or out-of-scope evidence is shown as such.** The app says "needs expert judgment" instead of guessing.
4. **Fetched data shows its source and date.** A failed lookup is shown as a failure, never as "nothing found".
5. **It warns about common mistakes:** double-counting, conflicting criteria, discouraged criteria, changed strengths.
6. **No invented numbers.** VUS sub-tier cut-offs stay empty until official ranges exist (see section 9).

## 7. Functional requirements

| ID | Requirement | Phase |
|---|---|---|
| F1 | Load a rule set from a data file; reject rules without a source | MVP |
| F2 | Apply criteria with adjustable strength; compute points and class | MVP |
| F3 | Also show the ACMG/AMP 2015 combining-table class and flag disagreement | MVP |
| F4 | Points scale, evidence lanes, "what would change it", warnings | MVP |
| F5 | Learning layer: plain-language rule, note and source per criterion | MVP |
| F6 | Reported-variant view: record what each lab used, compare with today | MVP |
| F7 | Gene checks: gene-disease validity, existing gene-specific rule set; manual override | MVP |
| F8 | Practice shelf: solved cases, answer hidden until you commit | MVP |
| F9 | Import a ClinVar record | MVP |
| F10 | Gene-specific rule sets (ClinGen expert panels) layered over generic rules | v2 |
| F11 | Fetch population and prediction data | v2 |
| F12 | Saved re-check list with rule-set version and date | v2 |
| F13 | Slide and PDF export | v2 |
| F14 | VUS sub-tiers with official numeric ranges | when published |

## 8. Non-functional requirements

- **Correctness over speed.** The scoring is covered by automated tests, including the 2015 table rows.
- **Privacy.** Local files only. Lookups send a gene symbol (and panel names in the query), never the variant or patient data; the ClinGen lookup downloads one public reference file.
- **Offline.** Everything except lookups works without internet.
- **Platforms.** Windows and macOS. Linux later.
- **Accessibility.** Color is never the only clue. Keyboard use and visible focus. Light and dark themes.
- **Readable code and data.** A reviewer who is not a programmer can read and edit a rule file.

## 9. Open items

- **VUS sub-tier numbers:** both source papers (Bennett 2025, Mighton 2026) describe the tiers in words only. Point ranges will come with the revised ACMG standards (SVCv4.0, not yet published). The rule file has the slots; they stay empty until then.
- **OMIM data-service key:** requested by the owner, pending.
- **Content review:** the owner will have a qualified reviewer check the rules and lessons before team use.
- **Rule content from memory:** all criteria text, the points table, and the ClinGen refinements are drafted from memory and flagged unverified. The 2015 combining-table rows are covered by tests but still need source checking.
- **Practice case source:** needs a legal source (for example, published ClinGen examples).
- **SVCv4.0:** will probably change more than the VUS tiers. The engine must be able to hold two rule-set versions.

## 10. Success criteria

For the owner: after using it on real variants for a month, they can explain, for a reported variant, why it has its class and what would move it, without the app.
For the product: a reviewer signs off the rule set, and a colleague can install and use it unaided.
