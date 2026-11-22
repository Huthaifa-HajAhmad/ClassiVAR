# Changelog

## 1.0.0 (2026-11-22)

The first release with installers. ClassiVAR explains how the ACMG/AMP criteria
you choose lead to a variant classification, and how today's evidence compares
with earlier lab reports.

**Research use only. Not a clinical reporting system.** The rules (v0.14.0-draft)
are still **UNVERIFIED**: no qualified human curator has signed them off, and
every class on screen says so.

### What it does

- Explains a class with a dial, a column or a track, a breakdown by kind of
  evidence, the 2015 combining table, and what would change the class.
- Checks a variant with the ClinGen Allele Registry (HGVS, VCF, SPDI, ClinVar
  VCV, dbSNP rsID or ClinGen CA id), and shows the identity it returned: MANE
  Select name, protein change, both genome builds and the ids.
- Checks a gene with ClinGen gene-disease validity, Gene2Phenotype and
  PanelApp (England and Australia), one line per source.
- Imports earlier lab reports from ClinVar, and compares each with today's
  evidence under today's rules.
- Saves and opens case files (`*.classivar.json`).

### Changes since the development builds

- An explanation tool, not a teaching tool: the Learning switch is gone, each
  criterion's rule text opens on demand ("More"), and the fictional demo case
  is removed. The opening screen shows the empty dial.
- The window opens maximized. The reports rail and the editor sheet grow with
  the window, and the explanation sections sit side by side when there is room.
- Clearer results after a check: the checked variant and the gene sources
  each have their own card; the editor is grouped into Variant, Gene,
  Condition and Earlier reports.
- Each report in the rail marks its own class on its scale.
- A two-part Light / Dark switch.
- An app icon, a release build without a console window, and the licence texts
  of every bundled package (`THIRD-PARTY-LICENSES.html`).

### Known limits

- The installers are unsigned unless the release was built with signing
  certificates (see `docs/RELEASE.md`). Windows then shows a SmartScreen
  warning; macOS asks you to allow the app in System Settings.
- The macOS app has been built in CI but not yet run by a person on a Mac.
- Copy-number, mitochondrial, somatic and low-penetrance variants use other
  frameworks and are out of scope.
