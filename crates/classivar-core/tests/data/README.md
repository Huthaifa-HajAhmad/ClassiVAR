# Benchmark data

`erepo-2026-11-21.tsv` holds expert-panel variant classifications from the
ClinGen Evidence Repository (ERepo). The ERepo benchmark test
(`crates/classivar-core/tests/erepo_benchmark.rs`) reads it, so the merge gate
runs offline and gives the same answer every time.

- **Source:** the Clinical Genome Resource (ClinGen), www.clinicalgenome.org.
  Export: `https://erepo.clinicalgenome.org/evrepo/api/classifications/all?format=tabbed`.
- **Accessed:** 2026-11-21.
- **Kept:** four columns (allele id, expert panel, class, met criteria codes);
  retracted rows dropped. Made by `scripts/erepo-snapshot.py`.
- **Terms:** ClinGen releases its curated content under the
  [CC0 1.0 Universal Public Domain Dedication](https://creativecommons.org/publicdomain/zero/1.0/)
  and asks for attribution with the date accessed
  ([ClinGen terms of use](https://clinicalgenome.org/docs/terms-of-use/)).
- **Attribution:** The expert-panel classifications in this file were obtained
  from the Clinical Genome Resource (ClinGen), www.clinicalgenome.org,
  accessed 2026-11-21. ClinGen is described in Rehm et al., N Engl J Med
  2015;372:2235-2242 (doi:10.1056/NEJMsr1406261).

ClinGen's classifications are for research use. ClassiVAR uses them only to
test its rules; it does not show them as answers.
