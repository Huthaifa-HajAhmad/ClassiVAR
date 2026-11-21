# Scientific audit, 2026-11-21

An outside review of ClassiVAR's clinical content and of how a variant enters a case. It judges the program as a senior clinical genomics scientist would: against ACMG/AMP 2015, the ClinGen SVI recommendations and the points system (Tavtigian 2018/2020).

Nothing in the code or the rule file was changed. Every finding below is a proposal until the owner approves it. Any change to a clinical rule still needs a cited source, a test and a note in `ROADMAP.md`.

Baseline on the day: `cargo test --workspace --locked` passes (174 app, 145 engine, 32 offline source tests; 5 live tests skipped as designed). Rules `0.13.0-draft`, status `unverified`.

## 1. What ClassiVAR claims, in short

A learning workbench. The curator chooses the ACMG/AMP criteria; the program adds the points, shows the class with its review status, compares the result with earlier reported records, and explains it in pictures. It does not decide criteria and is not a reporting system. ClinVar import, practice cases, a macOS run and installers are not built. No qualified reviewer has checked the rules.

## 2. What was checked and holds

| Area | Result |
|---|---|
| Points per strength (1/2/4/8) and class limits (P ≥ 10, LP 6–9, VUS 0–5, LB −1 to −6, B ≤ −7) | Match Tavtigian 2020. `rules/acmg-amp-points.toml:123-136` |
| 2015 combining table, plus ClinGen 2020 "PVS1 + 1 Supporting = LP" | Every row matches Richards 2015 Table 5. `crates/classivar-core/src/combining_table.rs` |
| Class withheld for contradictions, combined limits, and both sides met with a confident points class | Sound. A true VUS stays VUS. `crates/classivar-core/src/assess/mod.rs:168` |
| BA1 overrides points, withheld when pathogenic evidence meets a 2015 row | Sound and well argued (known common pathogenic variants exist). |
| PP3/BP4 limits for REVEL, BayesDel (no AF), MutPred2, VEST4 | Match Pejaver 2022 Table 2 as I know it. Not re-read today. |
| PS3/BS3 OddsPath limits (2.1 / 4.3 / 18.7 / 350; 0.48 / 0.23 / 0.053) and 11 controls for Moderate | Match Brnich 2019. |
| De novo points (Table 1) and thresholds (0.5 / 1 / 2 / 4) | Match ClinGen SVI v1.1. |
| PM3 points | **Checked against the ClinGen PDF today.** Table 1: P or LP confirmed in trans 1.0; phase unknown 0.5 (P) / 0.25 (LP); homozygous 0.5, at most 1.0; VUS in trans 0.25, at most 0.5, unknown phase 0. The rule file matches exactly. |
| BA1 exception list (nine entries) | Matches the ClinGen July 2018 list. |
| PP5/BP6 listed but not counted | Matches ClinGen 2018. |

The engine is careful. The problems are mostly around the engine: what a case is about, and which strengths a curator may choose.

## 3. Findings, most serious first

### Critical

**C1. The variant is free text that nothing checks.**
`crates/classivar/src/view/actions.rs:8` trims and cuts the text to 300 characters. That is all. There is no genome build, no transcript, no check against the reference sequence, no normalisation and no lookup. The gene is typed in its own box (`case.rs:79-80`), so the case can say a BRCA1 variant in BRCA2 without any warning.
*Why it matters:* the reported-variant path exists to compare records **of the same variant**. With free text, two records about different variants can be compared without anyone noticing: another transcript version, GRCh37 against GRCh38 coordinates, or a legacy name. Several PVS1 answers (last exon, the NMD boundary, "exon in the relevant transcripts") also depend on the transcript, which is never recorded.
*Fix:* see section 4.

**C2. A case has no condition and no mode of inheritance.**
ACMG/AMP classifies a variant **for a condition**. ClinVar stores a lab's class per condition too (each lab submission is called an SCV). ClassiVAR's case holds only a variant, a gene and a yes/no answer on the gene–disease link.
*Why it matters:* PM3, BS2, BP2, BS1, PM2 (recessive) and the recessive step in PS2 all depend on the inheritance mode. Without a condition, records for different diseases look like disagreements. Example: a CFTR variant classified for cystic fibrosis by one lab and for CBAVD (a milder, male-only CFTR condition) by another. Another example: a gene that has a dominant disease and a recessive disease.
*Fix:* add a condition (a MONDO id plus a name) and an inheritance mode to the case, chosen from the ClinGen or GenCC gene–disease result the app already shows. Show them with every class. Give each record its own condition, and say "different condition" in the comparison instead of "upgrade" or "downgrade". Existing files would open with these fields empty.

**C3. A doubtful gene–disease link still gives "Pathogenic".**
`crates/classivar/src/view/panels/lanes.rs:51` only adds a note. The engine ignores `GeneDecision` completely.
*Why it matters:* the framework assumes the gene causes the disease. A learner who marks the link "doubtful" and still sees "Pathogenic" learns the wrong lesson. Clinical practice does not report P/LP variants in genes with Limited, Disputed or Refuted validity.
*Fix (a clinical rule, needs a source):* while the link is "not established" or "not decided", withhold P and LP and say why. Better still, take the ClinGen validity level of the chosen gene–disease pair. Before building, find the exact wording in Richards 2015 and in ClinGen's gene curation guidance and cite it.

**C4. The content has grown faster than it has been reviewed.**
13 rule versions, 10 evidence helpers, and still no qualified reviewer. Each new helper adds more unreviewed text and numbers. That is the main risk of the project, more than any single number.
*Fix:* see recommendations R1 and R2.

### Major

**M1. There is no population-frequency helper, and the BA1 text is the 2015 text.**
PM2, BS1 and BA1 are among the most used criteria, and they are where learners make the most mistakes. BA1's summary (`rules/acmg-amp-points.toml:836`) says only "more than 5%". The ClinGen update (Ghosh 2018) adds: in a continental population with at least 2,000 observed alleles, and only where no gene-specific limit exists. BS1 has no method at all. The usual method is the maximum credible allele frequency (Whiffin 2017), which needs prevalence, penetrance and the inheritance mode, so C2 is needed first. PM2 has no warning about poor coverage or about using the filtering allele frequency.
*Fix:* a "Population frequency" helper. Inputs: the frequency, the allele number, the population group, the inheritance mode, and an optional gene-specific limit. Output: the PM2, BS1 or BA1 eligibility, with the source quoted.

**M2. Strength caps allow raises that no source supports.**
PP2, BP1, BP2, BP3, BP5 and BP7 may be raised to Moderate. PM1, PM4 and PM5 may be raised to Strong. A free-text reason is the only control. The rule file says these caps are the program's own choice (`toml:294-301`). For a teaching tool, this teaches that strength is a free choice.
*Fix:* make each generic cap the highest strength a published source supports, which is usually the 2015 default. A stronger level should come only from a gene-specific rule file (a VCEP specification, called a "CSpec" at ClinGen) loaded with `--rules`. Later it can come from the CSpec registry, which the roadmap already says is reachable.

**M3. PVS1 does not check the disease mechanism, and its guide is shortened.**
PVS1 applies only where loss of function is a known disease mechanism. The program never asks. The guide also leaves out the in-frame splicing branch (exon skipping that keeps the reading frame) and does not state the NMD rule (no decay is predicted for a stop in the last exon or in the last 50–55 nucleotides of the second-to-last exon). The gene-level downgrade (Abou Tayoun 2018, Table 1) still waits for a decision.
*Fix:* before the guide, ask whether loss of function is an established mechanism, and show the ClinGen Dosage Sensitivity haploinsufficiency score as evidence. Add the two missing branches. My answers on the pending gene-level decision are in section 5.

**M4. ClinVar import (MVP item 4) is not built, and its limits are not written down.**
A ClinVar SCV holds the lab, the date last evaluated, the class, the condition, the review status and sometimes a free-text comment. It rarely lists the criteria in a structured form. An import can fill the record's lab, year, class and condition. It cannot honestly fill the criteria.
*Fix:* build the import on the variant identity layer (section 4). Fill the facts ClinVar holds. Leave the criteria to the curator, marked "not stated in ClinVar". If a comment names ACMG codes, offer them as "read from the lab's comment, check", never apply them silently.

**M5. The predictor list is from 2022.**
ClinGen has since calibrated more tools. I believe this includes AlphaMissense (Bergquist et al., Genet Med 2025), but **check the citation before using it**. Walker 2023 also says how SpliceAI and a missense tool should be used together for one variant, and the roadmap already lists this as open.
*Fix:* add newer tools only from the calibration papers, each with a test at its limits. Settle the SpliceAI-with-missense rule.

**M6. Some double-counting checks may be missing.**
The program warns for PVS1+PP3, PS2+PM6 and PP4+PS4. Possible further pairs, **each needing a source before it is built**:
- PVS1 + PS3, when the assay only shows the loss of function that PVS1 already counts.
- PS2/PM6 + PP4, because the de novo points already score how well the phenotype fits.
- PM1 + PP2, which are often both based on missense constraint.

**M7. The scope does not say what is excluded.**
Copy-number variants (Riggs 2020 CNV points), mitochondrial variants (McCormick 2020), somatic variants (AMP/ASCO/CAP 2017; ClinGen/CGC/VICC 2022) and low-penetrance or risk alleles use other frameworks. The PVS1 answer "Whole-gene deletion" (`toml:332`) is a CNV question. Its own text admits the points disagree with the source: PVS1 alone gives LP, while Abou Tayoun says Pathogenic.
*Fix:* state these exclusions on the welcome screen and in `MVP.md`. Keep the `whole_gene_deletion` id (saved cases use it), but change its text to point to the CNV framework.

### Minor

- **m1.** The PS1/PM5 exclusive pair cites Richards 2015 (`toml:170-174`), but the 2015 paper does not say this. It is VCEP practice. Fix the citation or drop the pair.
- **m2.** The functional helper counts all validation controls together. Brnich 2019 asks for both pathogenic and benign controls. Ask for the two counts separately.
- **m3.** Walker 2023's PS1 for splicing variants and its BP7 for deep intronic variants are not built. Add them with the splicing assays, as the roadmap plans.
- **m4.** A raised strength needs a reason, but an applied criterion carries no evidence record (for example "gnomAD v4.1, frequency 0, read on date X"). For learning and for review, every applied criterion should be able to hold a short "what I looked at" note, not only raised ones.

## 4. How a new variant is entered: today and proposed

**Today:** one text box, "Variant, e.g. NM_007294.4:c.5266dup" (`crates/classivar/src/view.rs:269`). Any text up to 300 characters is accepted, saved and shown. Nothing is parsed or looked up. Genomic coordinates, ClinVar ids, rsIDs and CAids are all just text.

**Proposed: one box that recognises the form, a lookup the curator starts, and a stored identity.**

| Form typed | Example | Accept? | Note |
|---|---|---|---|
| HGVS c. with a versioned transcript | `NM_007294.4:c.5266dup` | Yes, main form | Suggest the MANE Select transcript if another one is used. |
| HGVS g. on a chromosome accession | `NC_000017.11:g.43057063dup` | Yes | The accession already says the build. |
| Chromosome, position, ref, alt | `17-43057062-T-TG` | Yes, **only with a chosen build** | Never guess GRCh37 or GRCh38: ask. |
| SPDI | `NC_000017.11:43057062:T:TG` | Yes | |
| ClinVar VariationID or VCV | `VCV000017677` | Yes | An RCV or SCV leads back to its VCV. |
| ClinGen Allele Registry id (CAid) | `CA…` | Yes | Best canonical id. |
| dbSNP rsID | `rs80357906` | Yes, **with an alt allele chosen** | One rsID can stand for several alleles. |
| HGVS p. alone, gene + c. without a transcript, legacy names (`5382insC`) | | **No** as identity | Kept as free-text "also known as". p. alone is ambiguous: several DNA changes give the same protein change, which is exactly the PS1 question. |

*(The example ids are only for showing the format. Check each one before putting it in a test.)*

**How it resolves:** the ClinGen Allele Registry takes HGVS, coordinates and dbSNP or ClinVar ids. It returns the CAid, the GRCh37 and GRCh38 positions, the transcript forms and the ClinVar and dbSNP links. VariantValidator checks HGVS against the reference sequence and normalises it (the 3′ rule). One or both can go in `classivar-sources`, with the same rules the gene lookups already follow: a failure is shown as a failure, "not found" only when the service says so, and source and date on every answer.

**What the case file stores:** what the curator typed; the CAid; the GRCh38 and GRCh37 positions; the transcript and HGVS c./p.; the gene **taken from the variant** (a typed gene that disagrees is flagged); the ClinVar VCV; and the service and date of the lookup. A case can still be saved without a lookup, marked "identity not checked". Offline use stays possible.

**Privacy:** a lookup sends only the variant, never the case, the labs or the evidence. It starts with a button, like "Check gene". The fictional example keeps its guard and never looks anything up.

## 5. Answers to the decisions already waiting in the roadmap

| Decision | Recommendation | Why |
|---|---|---|
| PVS1 gene-level downgrade: build it? | **Yes**, with the design already written in Phase C item 7. | Without it, PVS1 overstates evidence in genes where loss of function is uncertain. |
| (a) "more than 10%" or "at least 10%" | Follow the table: more than 10%. | Tables are what curators apply. The difference is tiny. |
| (b) A downgrade below Supporting | No PVS1, shown as such. | It matches the paper's "no PVS1" row. |
| (c) Lower the flowchart's strength, or only Very Strong? | Lower the strength the flowchart gave. | The paper's examples do this. |
| PM6 cap: Strong or Very Strong? | **Raise it to Very Strong**, as SVI Table 2 allows. | The SVI points already halve unconfirmed parentage. Capping again counts the same weakness twice, and the cap has no source. |
| PP4 from 19.1% or 20% | Keep the table (19.1%) and the warning. | The table is the calibrated value. |
| BA1 with pathogenic evidence | Confirm the withholding. | Clinically right. |
| De novo "many genes" row at most 1 point | Confirm this reading. | It fits the table's footnote. |

## 6. Bold recommendations

**R1. Freeze new clinical helpers until review starts.** The next milestone should be M7 (content review), not helper number 11. Generate a "review packet" from the rule file: each entry with its number, the source sentence or table it comes from, and the test that pins it. A reviewer can then sign off line by line in hours, not days. Record the sign-off per entry, not only for the file.

**R2. Measure the engine against expert classifications.** The ClinGen Evidence Repository (ERepo) publishes expert panel classifications with the criteria codes and strengths they met. Feed those criteria into the engine, compare its class with the panel's, and publish the agreement rate with every disagreement explained. Expected sources of difference: gene-specific rules, points against the 2015 table, and PP5/BP6. This turns "UNVERIFIED" into a measured statement. It also makes a strong teaching set without writing any cases, which answers the parked practice shelf's licensing problem.

**R3. Define a case as variant + condition + inheritance mode, with a checked identity** (C1, C2, section 4). This is the base that ClinVar import, the population helper and fair report comparison all need. Build it before anything else on the reported-variant path.

**R4. Gene-specific rules as the only way above the published strengths** (M2). Lower the generic caps. Make loading a CSpec (VCEP) rule file a first-class action, not a command-line flag.

**R5. Plan for the next ACMG/ClinGen standard (SVC v4).** It is expected but not published. Keep evidence and points in data, as now, so a new framework means a new rule file and not a new engine. Avoid building more per-criterion helpers that the new standard may replace.

## 7. Proposed roadmap change (not applied; needs approval)

Insert before Phase F:

- **Phase H: what a case is about.** (1) Variant identity: parse the forms in section 4, look them up in the Allele Registry, store the canonical identity, take the gene from the variant. (2) Condition and inheritance mode per case and per record. (3) Gene–disease validity limits P/LP, once sourced (C3). (4) ClinVar import on top of (1) and (2) (M4). Done when: a VCV id fills a case with its identity, its condition and each lab's class, and two records for different conditions are never compared as one.
- **Phase I: evidence that the content is right.** (1) The review packet (R1). (2) The ERepo benchmark in the merge gate, offline, on a dated snapshot (R2). (3) Caps lowered to published strengths (M2). (4) A population-frequency helper (M1). (5) The PVS1 mechanism question and the gene-level step (M3, section 5). Done when: a named reviewer signs off, and the benchmark's agreement is published with every disagreement explained.
- **Scope:** add the exclusions in M7 to `MVP.md` and to the welcome screen.
- **Paused:** new helpers, other than those listed above, until Phase I step 1 is done.

## 8. Remediation log

The owner asked for every finding to be addressed, with Claude as the interim reviewer. Claude does not set `status = "reviewed"`: an AI review is not the qualified human review that status promises. Sign-off per entry is recorded separately (see R1).

**Stage 1 — rule content (rules 0.14.0-draft), done 2026-11-21, merge gate passed.**

- M2: every `max_strength` is now the highest level a cited general source supports. PM1, PM4 and PM5 stop at Moderate; PP2, BP1, BP2, BP3 and BP5 at Supporting. BP7 keeps a sourced route to Strong (Walker 2023, "BP7_Strong (RNA)") through a new `levels` field, so the menu offers Supporting or Strong and never an unsourced Moderate. The test `no_criterion_can_be_raised_above_its_published_ceiling` names the source for every limit. Screen text now says "these rules stop at X" instead of "this program".
- PM6 may reach Very Strong, as SVI de novo Table 2 lists (section 5 decision).
- M1 (part): BA1 uses the Ghosh 2018 wording: above 0.05 in a continental population dataset of at least 2,000 alleles, with the founder-population caution.
- M3 (part): the PVS1 guide states the NMD rule (no decay for a stop in the last exon or the last 50 nucleotides of the second-to-last exon) and covers in-frame exon skipping, cryptic splice sites and exon deletions (Abou Tayoun 2018, Table 1 and footnotes). The note quotes Richards 2015 on the null-variant mechanism (the MYH7 example) and the ±20-nucleotide check.
- M7 (part): the whole-gene deletion answer points to the CNV framework (Riggs 2020). Its id is unchanged.
- m1: the PS1/PM5 pair is named as this program's safeguard, not a 2015 rule.
- m2, corrected after reading Brnich 2019 again: the audit understated it. Without statistics, Supporting needs normal and abnormal **experimental** controls and replicates, and allows "10 or fewer" validation controls, none included. Moderate needs at least 11 validation controls "including a mix of benign and pathogenic". The old helper refused 0 validation controls and never asked about experimental controls. The helper now asks for the confirmation and for the two counts separately.
- New, found while reading Pejaver 2022: BP4 at Moderate had no row in the 2015 table, so the table gave no class. Pejaver's interim rule lets it invoke the Likely benign rules. It is now a marked table row ("Pejaver 2022 addition"), applied to any Moderate benign evidence. **Decision for the human reviewer:** that generalisation (to BS3_Moderate, for example) is Claude's judgment: Moderate is two Supporting on Tavtigian's scale. A consequence: BP4_Moderate against two Strong pathogenic criteria now meets both sides of the table, so the class is withheld for expert judgment.
- **Correction to section 4:** the example g. position for BRCA1 c.5266dup on GRCh38 is `NC_000017.11:g.43057065dup` (ClinGen Allele Registry CA003404), not …063.

**Stages 2 to 9, done 2026-11-21, merge gate passed (192 app, 195 engine, 2 benchmark, 42 offline source tests; both new live tests pass).**

| Finding | Status | Where |
|---|---|---|
| C1 variant identity | Done | `classivar-core/src/variant/` (parser ported from Missense-Structural-Explorer and widened), `classivar-sources/src/allele_registry.rs`, "Check variant" in the case form |
| C2 condition and inheritance | Done | `condition.rs`, case fields, ClinGen curation as a one-click condition, per-record condition box, "different condition" never compared |
| C3 gene of uncertain significance | Done | `assess/gene_limit.rs`; Richards 2015 Special Considerations |
| C4 review | Done as far as an AI can | `rules/review-signoff.toml` (55 entries), `docs/review/REVIEW-PACKET.md`; status stays `unverified` |
| M1 population | Done | `rules/population.rs`, Population frequency helper |
| M2 caps | Done (stage 1) | rule file, `levels` |
| M3 PVS1 | Done | gene-level question, guide text |
| M4 ClinVar import | Done | `classivar-sources/src/clinvar.rs`, `view/clinvar_records.rs` |
| M5 predictors | Done, ESM1b not added | Bergquist 2025 (AlphaMissense, VARITY_R); ESM1b's scale runs the other way |
| M6 double counting | PVS1 + PS3 added; PS2/PM6 + PP4 and PM1 + PP2 not added | no source says so |
| M7 scope | Done | welcome screen, `MVP.md` |
| m1 to m4 | Done | see stage 1 and the ROADMAP section |
| R1 review packet | Done | generator `examples/review_packet.rs` |
| R2 benchmark | Done | `benchmark.rs`, `tests/erepo_benchmark.rs`, `docs/review/BENCHMARK-erepo.md` |
| R3 case = variant + condition + inheritance | Done | C1 and C2 |
| R4 gene-specific rules as the route above the caps | Done | Rules… opens a rule file in a new window |
| R5 SVC v4 readiness | Unchanged by design | rules stay data; no new helpers beyond the findings |

**Decisions closed 2026-11-21** (the owner asked for a decision as a panel of domain experts would make it; AI judgment with sources, status stays `unverified`):

1. *Moderate benign in the 2015 table:* kept for any Moderate benign criterion, not only BP4. Richards 2015 combines by strength, not by code; Brnich 2019 defines BS3_Moderate, which would otherwise meet no row; on the points scale −2 equals the existing two-Supporting row.
2. *Benign results in a gene of uncertain significance:* kept as they are. Richards 2015 limits what a doubtful gene can make *pathogenic*; a common variant is still benign. The note on the result now says that BS1, BS2, BS4 and BP5 assume the gene causes the disease.
3. *Whiffin's ½:* kept. The paper's worked example and Table 1 footnote use it (a share of people becomes a share of gene copies). The BS1 finding now names the formula with its ½.
4. *Two Very Strong criteria:* **new table row "≥2 Very strong → Pathogenic", marked "Tavtigian 2020 points addition".** No paper prints it: in 2015 PVS1 was the only Very Strong criterion. Two are 16 points (Pathogenic from 10), and all 58 ERepo classifications with two Very Strong criteria are Pathogenic. It closes a safety gap: PVS1 + PM3_Very Strong + BS1 + BS2 used to show a confident Likely pathogenic; it now meets both sides and is withheld. Benchmark unchanged (98.3%, none across VUS).
5. *ERepo data in a public repository:* kept. ClinGen releases curated content under CC0 1.0 and asks for attribution with the access date (terms page read 2026-11-21); `crates/classivar-core/tests/data/README.md` gives it.

**What the benchmark changed.** Scoring 13,864 expert-panel classifications showed two rules that disagreed with practice or safety: (1) 8 panels apply PS1 and PM5 together (18 classifications, 17 of them P or LP), so the pair is now a double-counting caution, not a withheld class; (2) two Very Strong criteria meet no 2015 row, so BA1 with PVS1 and PM3 at Very Strong was shown as Benign; BA1 is now also withheld when the pathogenic points alone reach Likely pathogenic. After both changes, 98.3% of classifications with evidence agree, and none crosses VUS.

**Decisions for the human reviewer.**
1. Moderate benign evidence (not only BP4) meets the Pejaver Likely benign row.
2. In a gene of uncertain significance, benign results are left as they are (the source speaks about candidate variants).
3. Whiffin 2017's printed dominant formula has no ½; its worked example does. The example is followed.
4. Should two Very Strong criteria meet a Pathogenic row of the 2015 table? All 58 such panel classifications are Pathogenic; the points already say so, so only the table view and the both-sides rule are affected.
5. Every sign-off marked "not checked" (points, class limits, PP1/PP4 and BS4 tables, PP5/BP6, PVS1/PM4, PP1+PP4 cap, VUS tiers).
6. The ERepo snapshot (1.2 MB, public ClinGen data) is in the repository as a test fixture; confirm the data terms before publishing the repository.

**Not built, and why.** ESM1b (needs a "lower is more damaging" field); PVS1 strengths from RNA assay results (Walker 2023, beyond BP7_Strong (RNA)); counting a stronger criterion toward a weaker 2015 row (decision 4); a CSpec registry lookup (Rules… loads a file the curator has).
