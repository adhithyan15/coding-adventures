#!/usr/bin/env python3
"""test_genetics_recall.py — the ADJ-only genetics recall library (MYCIN-2026 GENETICS).

The fourth recall domain shipped as a pure ADJ artifact (completing Tier-1 foundational
recall: MICRO, PHARM, IMMUNO, GENETICS). `genetics-edges.adj` is the SOLE source of
truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest. This test
pins the property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`genetics-recall.query.adj` — the shape
an LLM produces), returns the correct binding for every grounded edge, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ; the
engine answers. (`gene_defect` is shared with the IMMUNO library; the disorders are
disjoint, and the combined import resolves both — see board_eval's merged store.)

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_genetics_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "genetics-edges.adj"
QUERY = HERE / "genetics-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# The full grounded edge set (subject, relation, expected binding). Mirrors
# genetics-edges.adj; a divergence here or there fails the test.
EXPECTED = [
    ("huntington_disease", "inheritance", "autosomal_dominant"),
    ("huntington_disease", "gene_defect", "htt"),
    ("huntington_disease", "trinucleotide_repeat", "cag"),
    ("marfan_syndrome", "inheritance", "autosomal_dominant"),
    ("fragile_x_syndrome", "trinucleotide_repeat", "cgg"),
    ("fragile_x_syndrome", "gene_defect", "fmr1"),
    ("prader_willi_syndrome", "imprinting", "paternal"),
    ("cystic_fibrosis", "inheritance", "autosomal_recessive"),       # expand (NBK493206)
    ("cystic_fibrosis", "gene_defect", "cftr"),                      # expand (NBK493206)
    ("duchenne_muscular_dystrophy", "gene_defect", "dystrophin"),    # expand (NBK482346)
    ("myotonic_dystrophy", "trinucleotide_repeat", "ctg"),           # expand (NBK557446)
    ("myotonic_dystrophy", "gene_defect", "dmpk"),                   # expand (NBK557446)
    ("friedreich_ataxia", "gene_defect", "fxn"),                     # expand (NBK563199)
    ("marfan_syndrome", "gene_defect", "fbn1"),                      # expand (NBK537339)
    ("angelman_syndrome", "imprinting", "maternal"),                 # expand (NBK560870)
    ("angelman_syndrome", "gene_defect", "ube3a"),                   # expand (NBK560870)
    ("friedreich_ataxia", "inheritance", "autosomal_recessive"),     # expand (NBK563199)
    ("friedreich_ataxia", "trinucleotide_repeat", "gaa"),            # expand (NBK563199)
    ("wilson_disease", "inheritance", "autosomal_recessive"),        # expand (NBK441990)
    ("wilson_disease", "gene_defect", "atp7b"),                      # expand (NBK441990)
    ("hereditary_hemochromatosis", "inheritance", "autosomal_recessive"),  # expand (NBK430862)
    ("hereditary_hemochromatosis", "gene_defect", "hfe"),            # expand (NBK430862)
    ("tay_sachs_disease", "inheritance", "autosomal_recessive"),     # expand (NBK564432)
    ("tay_sachs_disease", "gene_defect", "hexa"),                    # expand (NBK564432)
    ("phenylketonuria", "gene_defect", "pah"),                       # expand (NBK535378)
    # --- BATCH: single-gene disorders + lysosomal storage diseases ---
    ("achondroplasia", "gene_defect", "fgfr3"),                      # expand (NBK559263)
    ("achondroplasia", "inheritance", "autosomal_dominant"),         # expand (NBK559263)
    ("gaucher_disease", "gene_defect", "gba1"),                      # expand (NBK448080)
    ("gaucher_disease", "inheritance", "autosomal_recessive"),       # expand (NBK448080)
    ("fabry_disease", "inheritance", "x_linked"),                    # expand (NBK435996)
    ("fabry_disease", "gene_defect", "gla"),                         # expand (NBK435996)
    ("pompe_disease", "inheritance", "autosomal_recessive"),         # expand (NBK470558)
    ("sickle_cell_disease", "gene_defect", "hbb"),                   # expand (NBK482164)
    ("retinoblastoma", "gene_defect", "rb1"),                        # expand (NBK545276)
    ("retinoblastoma", "inheritance", "autosomal_dominant"),         # expand (NBK545276)
    ("von_hippel_lindau", "gene_defect", "vhl"),                     # expand (NBK459242)
    ("von_hippel_lindau", "inheritance", "autosomal_dominant"),      # expand (NBK459242)
    ("hemophilia_a", "gene_defect", "factor_viii"),                  # expand (NBK470265)
    ("hemophilia_a", "inheritance", "x_linked"),                     # expand (NBK470265)
    ("alpha_1_antitrypsin_deficiency", "inheritance", "autosomal_codominant"),  # expand (NBK442030)
    ("tuberous_sclerosis", "gene_defect", "tsc1_tsc2"),              # expand (NBK538492)
    ("tuberous_sclerosis", "inheritance", "autosomal_dominant"),     # expand (NBK538492)
]
VAR = {"inheritance": "Pattern", "gene_defect": "Gene",
       "trinucleotide_repeat": "Repeat", "imprinting": "Parent"}


def _cli_available() -> bool:
    return CLI.exists()


def _run(program: Path) -> dict:
    out = subprocess.run([str(CLI), str(program)], capture_output=True, text=True,
                         cwd=str(HERE), timeout=60)
    assert out.returncode == 0, f"adj-lang-cli failed: {out.stderr}"
    return json.loads(out.stdout)


# ---- 1. the ADJ file is the artifact — every clause grounded, no authored-debt ----

def test_library_is_pure_adj_and_fully_grounded() -> None:
    text = ADJ.read_text()
    assert "trust consensus" not in text, "GENETICS ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    relate_count = text.count("    relate ")
    assert text.count("trust authoritative") == relate_count == len(EXPECTED)
    assert text.count('\n        locator "') == relate_count
    assert text.count('\n        source "') == relate_count
    # No sibling generator / JSON / manifest for this domain (ADJ-only).
    assert not (HERE / "genetics_edge_ground.py").exists()
    assert not (HERE / "genetics-edge-grounding.json").exists()
    assert not (HERE / "genetics-edge-manifest.json").exists()


# ---- 2. the engine resolves every grounded edge with an authoritative citation ----

def test_engine_resolves_every_edge_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for subj, rel, expected in EXPECTED:
        q = f"{rel}({subj}, {VAR[rel]})"
        r = by_query.get(q) or _one(rel, subj, VAR[rel])
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        ans = r["answers"][0]
        assert ans["bindings"][VAR[rel]] == expected, f"{q} -> {ans['bindings']}"
        cite = ans["citations"][0]
        assert cite["trust"] == "authoritative", f"{q} citation not authoritative"
        assert cite["source"], f"{q} citation has no source span"
        assert cite["locator"].startswith("https://www.ncbi.nlm.nih.gov/"), cite["locator"]


def _one(rel: str, subj: str, varname: str) -> dict | None:
    """Resolve a single edge by writing a throwaway one-line query .adj in this dir
    (so the relative `import "genetics-edges.adj"` resolves) and running the engine."""
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".genetics_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write(f'import "genetics-edges.adj"\n? {rel}({subj}, ${varname})\n')
        res = _run(Path(path))
        return res["recall"][0] if res["recall"] else None
    finally:
        os.unlink(path)


# ---- 3. an off-vocabulary subject abstains, never fabricates ----------------------

def test_unknown_disorder_abstains() -> None:
    if not _cli_available():
        return
    r = _one("inheritance", "ehlers_danlos_syndrome", "Pattern")  # not in the library
    assert r is not None and r["abstained"], "an ungrounded disorder must abstain"


def _run_all() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_genetics_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
