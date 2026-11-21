# Variant curation workbench: design

A shared application title bar surrounds the opening screen and case workspace. The opening screen shows a fictional example scored with the loaded rules.

The current workbench puts the explanation on the left and the evidence editor on the right. Expand explanation gives the chart the full workspace. Compare reports starts with the selected report and the current evidence. See [the evidence workbench design](evidence-workbench.md).

Date: 2026-11-03. Status: draft, waiting for the user's approval. Working title only.

Labels used below: **[User]** a decision the user made. **[My call]** my proposal, not yet agreed. **[Builder]** left to whoever plans and builds it.

## 1. Intent and why

The user is a physician with an MSc in Genomic Medicine. They have started variant curation (deciding if a DNA change causes disease) and want to understand the whole classification system: the ACMG/AMP 2015 criteria, the ClinGen refinements, the points system, and the newer VUS sub-tiers.

Their main sticking point: when a variant is already classified (VUS, "VUS-low", Likely Pathogenic or Pathogenic), they cannot tell how it got that class, whether it still holds today, or when to upgrade or downgrade it. A never-reported variant needs a different path. They want to **see** the evidence, not only read it.

The product is one desktop app that is used for real work, with learning built in. Later it may be shared with their team and used for presentations.

## 2. Goals, non-goals, anti-goals

**Goals [User]**
- Break any variant into its evidence, with points and a class, shown visually.
- Support three uses in version 1: (1) a new, never-reported variant, (2) a reported variant ("does this classification still hold?"), (3) practice on solved cases.
- Learning is available on the spot, if the user switches it on.
- Fetch the gene checks automatically (see section 6).
- Polished visuals: charts, plots and diagrams.
- Strong enough to share with the user's team.

**Non-goals for version 1 [My call]**
- Saved re-check list, slide/PDF export, shared team cases, web version, user accounts.
- Fetching population and prediction data automatically (stage 2, see section 9).

**Anti-goals (the app fails even if it "works") [User]**
- It silently classifies a variant.
- It gives a wrong answer with confidence.

## 3. Approaches considered

1. **One hand-written HTML file.** Fastest, but gets fragile and hard to test. Rejected.
2. **Modular project that builds into one HTML file.** Was recommended, then the user chose a native app (below).
3. **Hosted app with server and database.** Adds patient-data and security duties. Rejected for now.
4. **Native desktop app in Rust with GPUI, with the rules engine as a separate library. [User chose this].** The user wants to learn and enjoy building with GPUI. Trade-offs: team sharing needs an installer or .exe; GPUI is young and its interface changes; far fewer ready-made parts than the web; charts, formatted text and slide-like layouts need more work. Benefits: direct access to databases with no browser limits, strong compile-time checks for the classification logic, a fast native feel.

The rules engine is kept separate from the screen code **[My call]** so it can be tested alone, and so a web build stays possible later.

## 4. Product design

**One working screen, a learning layer, and a practice shelf [User + My call for details].**
- Working screen: as in the approved sketch (section 7).
- Learning layer: a switch. When on, every criterion gets a "why?" panel with the rule in plain words, its source, and a worked example. When off, the screen is clean. *(Status 2026-11-04: summary, note and source are shown; worked examples are not built yet.)*
- Practice shelf: solved cases open in the same working screen. The expert answer stays hidden until the user commits their own.

**Flow A, new variant [My call].** Gene checks, then evidence in a fixed order (variant type, population, computational, functional, family and case data, benign evidence), then points and class.

**Flow B, reported variant [My call].** The user enters or imports the ClinVar record (class, submitters, cited criteria). The app shows a side-by-side history of what each lab used against today's rules and evidence. Verdict: "holds", "upgrade candidate" or "downgrade candidate", each with reasons. Typical causes of drift to detect: strength of a criterion changed in later guidance (for example PM2 became *Supporting* in the ClinGen SVI recommendation of 2020), new evidence appeared, or the original lab double-counted.

**Safety principles for the anti-goals [My call, from the user's anti-goals].**
- The user applies each criterion. The app may suggest one with its evidence, but never applies it alone.
- The final class is always shown with the full breakdown, the rule-set version and the sources.
- If evidence is missing, conflicting, or outside the app's rules, the app says "needs expert judgment". It never fills the gap.
- Fetched data shows its source and fetch date. A failed fetch is shown as a failure, never as "nothing found".
- The app warns about double-counting (for example two predictors counted as two criteria).
- It is labeled decision support and a learning aid, not a validated reporting system.

## 5. The rules engine

- All rules live in data files, each with a citation, so a rule change is a file edit. A check rejects any rule without a source. **[My call]**
- Default scoring: the points system (Tavtigian et al., 2018 and 2020). Supporting, moderate, strong and very strong evidence are worth 1, 2, 4 and 8 points; benign evidence counts as negative points. Class thresholds as the builder confirms them from the source paper. The classic ACMG/AMP 2015 combining table is also shown, and the app flags when the two disagree. **[My call]**
- Coverage: all 28 ACMG/AMP 2015 criteria plus the ClinGen Sequence Variant Interpretation (SVI) refinements. *(Status 2026-11-04: 2015 criteria plus PM2, PP5/BP6, the PVS1 situation guide, BP4/PP3 ceilings and the PP1+PP4 combined cap; the remaining refinements below are noted on screen but not built in.)* Examples: PM2 lowered to Supporting; the PVS1 decision tree (Abou Tayoun, 2018); calibrated thresholds for computational predictors (Pejaver, 2022); functional-evidence guidance (Brnich, 2019). **[Builder verifies each against the source and the current version.]**
- Gene-specific rule sets (ClinGen expert-panel specifications, VCEPs) override generic rules where they exist. **[User wants ClinGen calibration rules.]**
- VUS sub-tiers are a clearly labeled layer. **[User wants these; the user named two papers as the sources.]**
  - Bennett et al., *Genet Med* 2025 (doi 10.1016/j.gim.2025.101400): four laboratories each used three tiers, **VUS-high** (evidence leans pathogenic, but not enough for Likely Pathogenic), **VUS-mid** (evidence equivocal, balanced, or absent) and **VUS-low** (evidence leans benign, but not enough for Likely Benign). Each lab built its own scoring rubric, and the paper gives **no numeric cut-offs**. Its main finding: VUS-low variants were never reclassified to Likely Pathogenic or Pathogenic, and VUS-high variants were far more likely to move up.
  - Mighton et al., ACMG statement "Points to consider for the reporting of VUS in germline genetic and genomic testing", *Genet Med* 2026 (doi 10.1016/j.gim.2026.102583), read in full by the author of this document. It defines the tiers in words: **VUS-low** is the lowest level of pathogenic evidence within the VUS range, **VUS-mid** is equivocal evidence, **VUS-high** is the upper fraction of VUS that borders Likely Pathogenic. It states that the ACMG/AMP 2015 guidelines already allow extra sub-classes, and that the VUS range spans roughly 10% to 90% probability of being causal. **It gives no point ranges.** It says the upcoming revised standards (**SVCv4.0**, not yet published) will set the point ranges and standard names, and that labs which have not yet adopted sub-classes may reasonably wait for them.
  - **Consequence:** there is no official numeric definition of VUS-low, VUS-mid or VUS-high yet. The app therefore shows the three tiers as plain-language descriptions and assigns **no numeric cut-offs of its own**. *(Status 2026-11-04: a reported VUS sub-tier can be entered per record, and the definitions with their sources are shown on screen and in the verdict. The app still names no tier for new evidence.)* **[My call, from the anti-goal "never a confident wrong answer".]**
  - The cut-offs live in a rule file. When SVCv4.0 is published, filling them in is a data change. Until then the user can load an optional, **clearly labeled lab-defined rubric** (each of the four labs in Bennett et al. used its own), shown as "not the official standard". **[My call]**
  - The ACMG statement also gives **reporting guidance** that fits the learning layer: report VUS-high more readily in symptomatic testing, consider leaving VUS-low off or in a supplement for low-yield testing (such as most germline cancer testing), and VUS are generally not returned in asymptomatic testing. Most VUS that are reclassified move toward benign (75% to 95%). **[My call: include as lessons, not as app decisions.]**

## 6. Gene checks and data sources

Before scoring, the app asks: is the gene known to cause this disease, and does a gene-specific rule set exist? It fetches the answers from OMIM, DECIPHER (assumed to mean its gene-disease data) and PanelApp **[User]**. Adding ClinGen gene-disease validity and its rule-set registry is **[My call]**. Every fetch can be overridden by hand, and the app still works offline.

Each source's access terms must be checked before use. OMIM's data service needs a registered key **[Builder]**.

## 7. Visuals

The approved sketch is the reference for layout: a points scale from Benign to Pathogenic with the variant marked; evidence lanes with one block per criterion, one block-width per point; empty lanes that show where evidence is missing; a "what would change this" panel; a side-by-side history for reported variants. **[User approved the sketch.]**

More visuals to add **[My call]**: allele frequency against its threshold, a protein diagram with variants marked, the PVS1 decision tree, and a class timeline.

Rules **[My call]**: color is never the only clue (every mark also carries a sign and a word); the palette is color-blind-safe and checked; light and dark themes; hover shows the source and the rule. Domain-specific diagrams are drawn by hand in GPUI. A chart library is used only where it fits.

## 8. Privacy and sharing

- No patient names or identifiers are stored by default. Files stay on the user's computer. **[My call]**
- Fetches send only gene names and variant coordinates. **[My call]**
- Platforms: Windows (the user's machine) and macOS (some colleagues use Mac). **[User]** GPUI supports both, but each needs its own build, installer and (later) signing. Linux later. **[My call]**
- Status label: research and learning use only until the content has been reviewed by a qualified curator. The user plans to share the content with a reviewer later. **[User]**

## 9. Order of work [My call]

1. **Spike (throwaway):** GPUI on the user's Windows machine, drawing one real chart (the points scale). Learn what is hard before committing.
2. **Version 1:** rules engine with tests, working screen, learning layer, practice shelf, gene-check fetching, ClinVar record import, the three uses.
3. **Version 2:** automatic fetching of population and prediction data, saved re-check list, slide and PDF export.
4. **Later:** web build, shared cases.

## 10. Open items

- **Numeric VUS tier definitions:** resolved as "not yet published". Both papers were read in full. The numbers wait for SVCv4.0. The builder must watch for its release and then update the rule file and the lessons.
- **SVCv4.0 in general:** the revised standards will probably change more than the VUS tiers (the ACMG statement says they move to a points-based system). The rules engine must make it cheap to add a second rule-set version beside the current one, and every saved result must record which version it used.
- OMIM data-service key: the user has requested one (pending). Until it arrives, the OMIM check uses manual entry.
- Content review: the user will share the rules and lessons with a reviewer later. Until then the app is labeled research and learning only. Updates after that need an owner; the tool must never present a stale rule as current.
- Practice cases need a legal source (for example published ClinGen examples). **[Builder checks licenses.]**

## 11. Left to the builder

Project layout, Rust and GPUI versions, test strategy and sample cases, the exact schema for rule files, caching and offline behavior, how the practice shelf stores progress, and the final look inside the approved sketch's direction. Also whether to adopt a GPUI component kit; ask the user first.
