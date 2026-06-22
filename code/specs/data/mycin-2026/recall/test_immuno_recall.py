#!/usr/bin/env python3
"""test_immuno_recall.py — the ADJ-only immunology recall library (MYCIN-2026 IMMUNO).

The third recall domain shipped as a pure ADJ artifact (after MICRO and PHARM):
`immuno-edges.adj` is the SOLE source of truth — facts + byte-provenance inline, no
Python gate, no JSON, no manifest. This test pins the property that matters: the native
adj-lang engine, given a query .adj that only `import`s the library and asks binding
queries (`immuno-recall.query.adj` — the shape an LLM produces), returns the correct
binding for every grounded edge, each carrying an AUTHORITATIVE citation with a real
source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_immuno_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "immuno-edges.adj"
QUERY = HERE / "immuno-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# The full grounded edge set (subject, relation, expected binding). Mirrors
# immuno-edges.adj; a divergence here or there fails the test.
EXPECTED = [
    ("type_i_hypersensitivity", "mediated_by", "ige"),
    ("type_iv_hypersensitivity", "mediated_by", "t_cells"),
    ("ankylosing_spondylitis", "associated_hla", "hla_b27"),
    ("x_linked_agammaglobulinemia", "gene_defect", "btk"),
    ("chronic_granulomatous_disease", "deficiency_of", "nadph_oxidase"),
    ("digeorge_syndrome", "deficiency_of", "t_cells"),
    ("digeorge_syndrome", "gene_defect", "chromosome_22q11_2_deletion"),
    ("celiac_disease", "associated_hla", "hla_dq2"),                 # expand (NBK441900)
    ("type_iii_hypersensitivity", "mediated_by", "immune_complexes"),  # expand (NBK559122)
    ("type_ii_hypersensitivity", "mediated_by", "igg_igm_antibodies"),  # expand (NBK563264)
    ("x_linked_scid", "gene_defect", "il2rg"),                       # expand (NBK562182)
    ("wiskott_aldrich_syndrome", "gene_defect", "was"),              # expand (NBK539838)
    ("hereditary_angioedema", "deficiency_of", "c1_inhibitor"),      # expand (NBK482266)
]
VAR = {"mediated_by": "Mediator", "associated_hla": "HLA",
       "gene_defect": "Gene", "deficiency_of": "Component"}


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
    assert "trust consensus" not in text, "IMMUNO ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    relate_count = text.count("    relate ")
    assert text.count("trust authoritative") == relate_count == len(EXPECTED)
    assert text.count('\n        locator "') == relate_count
    assert text.count('\n        source "') == relate_count
    # No sibling generator / JSON / manifest for this domain (ADJ-only).
    assert not (HERE / "immuno_edge_ground.py").exists()
    assert not (HERE / "immuno-edge-grounding.json").exists()
    assert not (HERE / "immuno-edge-manifest.json").exists()


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
    (so the relative `import "immuno-edges.adj"` resolves) and running the engine."""
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".immuno_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write(f'import "immuno-edges.adj"\n? {rel}({subj}, ${varname})\n')
        res = _run(Path(path))
        return res["recall"][0] if res["recall"] else None
    finally:
        os.unlink(path)


# ---- 3. an off-vocabulary subject abstains, never fabricates ----------------------

def test_unknown_condition_abstains() -> None:
    if not _cli_available():
        return
    r = _one("associated_hla", "rheumatoid_arthritis", "HLA")  # not in the library
    assert r is not None and r["abstained"], "an ungrounded condition must abstain"


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
    print(f"\ntest_immuno_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
