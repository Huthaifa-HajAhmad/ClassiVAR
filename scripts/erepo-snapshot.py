"""Make the dated ClinGen Evidence Repository snapshot the engine benchmark reads.

Usage: python scripts/erepo-snapshot.py <erepo-export.tsv> <out.tsv>
The export is https://erepo.clinicalgenome.org/evrepo/api/classifications/all?format=tabbed
Keeps four columns (allele, expert panel, class, met codes) and drops retracted rows.
ClinGen releases curated content under CC0 1.0 and asks for attribution with the
date accessed: see crates/classivar-core/tests/data/README.md.
"""
import csv
import sys

csv.field_size_limit(10**9)
source, target = sys.argv[1], sys.argv[2]
with open(source, encoding="utf-8", newline="") as f:
    rows = list(csv.reader(f, delimiter="\t"))
header = rows[0]
at = {name.lstrip("#"): i for i, name in enumerate(header)}
need = ["Allele Registry Id", "Expert Panel", "Assertion", "Applied Evidence Codes (Met)", "Retracted"]
missing = [n for n in need if n not in at]
if missing:
    sys.exit(f"the export has no column {missing}")
kept = 0
with open(target, "w", encoding="utf-8", newline="\n") as out:
    out.write("allele\tpanel\tclass\tmet\n")
    for row in rows[1:]:
        if len(row) < len(header) or row[at["Retracted"]].strip().lower() == "true":
            continue
        cells = [row[at[n]].replace("\t", " ").replace("\n", " ").strip() for n in need[:4]]
        out.write("\t".join(cells) + "\n")
        kept += 1
print(f"{kept} classifications written to {target}")
