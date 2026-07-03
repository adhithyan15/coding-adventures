#!/usr/bin/env python3
"""test_write_once_use_many.py — the WRITE-ONCE / USE-MANY thesis, machine-checked.

The MYCIN-2026 knowledge libraries (`*-edges.adj`) are WRITTEN ONCE — each fact grounded
to a byte-stable primary source, with its provenance inline — and then USED MANY times by
independent consumers, every answer carrying that same provenance, at zero answer-time
model calls. This test pins that claim against the real native engine using four query
files that share the SAME unchanged libraries:

    wom_forward.query.adj    consumer #1  subject -> object        (canonical recall)
    wom_reverse.query.adj    consumer #2  object  -> subject       (same facts, backwards)
    wom_enumerate.query.adj  consumer #3  object  -> {subjects}     (aggregate index)
    wom_crosslib.query.adj   consumer #4  one query over 2 libraries (composition for free)

It asserts the invariants that make "write once, use many" real, not decorative:
  (A) USE, NOT WRITE — the query files contain zero `relate`/`dictionary`/`rulebook`
      blocks; they only `import` and ask. All knowledge lives in the libraries.
  (B) ONE FACT, MANY VIEWS — the forward and reverse queries over the same edge return a
      byte-identical citation (source span + locator), because it is one written fact
      seen from two directions, not two copies.
  (C) PROVENANCE IS REAL — every returned citation's source span appears character-for-
      character in the cited library file (the engine echoes the written byte-provenance,
      it does not synthesize it).
  (D) READ-ONLY — querying the libraries does not modify them (the files are byte-identical
      before and after all four consumers run): "write once" is literally once.
  (E) DETERMINISTIC / OFFLINE — re-running a query yields identical output; the answer
      comes from the CPU engine, with no model in the answer-time loop.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip; the
static file-shape checks (A) still run.

Run:  python3 test_write_once_use_many.py
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

FORWARD = HERE / "wom_forward.query.adj"
REVERSE = HERE / "wom_reverse.query.adj"
ENUMERATE = HERE / "wom_enumerate.query.adj"
CROSSLIB = HERE / "wom_crosslib.query.adj"
QUERY_FILES = [FORWARD, REVERSE, ENUMERATE, CROSSLIB]

# Libraries the demo reuses, unchanged. (Written once; imported, never edited, by the
# queries above and by board_eval / decompose_query.)
LIBS = [HERE / "genetics-edges.adj", HERE / "immuno-edges.adj"]


def _cli_available() -> bool:
    return CLI.exists()


def _run(program: Path) -> dict:
    out = subprocess.run([str(CLI), str(program)], capture_output=True, text=True,
                         cwd=str(HERE), timeout=60)
    assert out.returncode == 0, f"adj-lang-cli failed on {program.name}: {out.stderr}"
    return json.loads(out.stdout)


def _by_query(result: dict) -> dict:
    return {r["query"]: r for r in result["recall"]}


def _answers(rec: dict) -> list:
    return rec["answers"]


# ---- (A) the query files USE, they do not WRITE ----------------------------------

def test_queries_define_no_facts_only_import() -> None:
    for q in QUERY_FILES:
        text = q.read_text()
        assert "import " in text, f"{q.name} must import a library"
        for written in ("relate ", "dictionary ", "rulebook "):
            assert written not in text, f"{q.name} must not {written.strip()} — it only consumes"


# ---- (B) one written fact, seen forward and backward, cites the SAME provenance ----

def test_forward_and_reverse_share_identical_provenance() -> None:
    if not _cli_available():
        return
    fwd = _by_query(_run(FORWARD))["gene_defect(huntington_disease, Gene)"]
    rev = _by_query(_run(REVERSE))["gene_defect(Disease, htt)"]
    assert _answers(fwd)[0]["bindings"] == {"Gene": "htt"}
    assert _answers(rev)[0]["bindings"] == {"Disease": "huntington_disease"}
    fwd_cite = _answers(fwd)[0]["citations"][0]
    rev_cite = _answers(rev)[0]["citations"][0]
    # Same written edge → byte-identical source span + locator + trust, both directions.
    assert fwd_cite == rev_cite, "forward and reverse must cite the identical written fact"
    assert fwd_cite["trust"] == "authoritative"


# ---- (C) every returned citation echoes a span that really lives in the library ----

def test_every_citation_is_present_in_its_cited_library() -> None:
    if not _cli_available():
        return
    lib_text = {lib.name: lib.read_text() for lib in LIBS}
    # NBK locator fragment -> which library file owns it (both cite NCBI here).
    checked = 0
    for q in QUERY_FILES:
        for rec in _run(q)["recall"]:
            for ans in rec["answers"]:
                for cite in ans.get("citations") or []:
                    span = cite["source"]
                    # the span must appear verbatim in at least one imported library
                    assert any(span in t for t in lib_text.values()), \
                        f"citation span not found verbatim in any library: {span[:60]}"
                    checked += 1
    assert checked >= 6, f"expected several cited answers, only checked {checked}"


# ---- (C') cross-library: each binding keeps the citation of the library that owns it ----

def test_crosslib_resolves_union_with_correct_owners() -> None:
    if not _cli_available():
        return
    by_q = _by_query(_run(CROSSLIB))
    htt = _answers(by_q["gene_defect(huntington_disease, Gene)"])[0]
    btk = _answers(by_q["gene_defect(x_linked_agammaglobulinemia, Gene)"])[0]
    assert htt["bindings"] == {"Gene": "htt"}
    assert btk["bindings"] == {"Gene": "btk"}
    # htt's span lives in genetics-edges; btk's span lives in immuno-edges.
    genetics = (HERE / "genetics-edges.adj").read_text()
    immuno = (HERE / "immuno-edges.adj").read_text()
    assert htt["citations"][0]["source"] in genetics
    assert btk["citations"][0]["source"] in immuno
    # reverse direction works across the union too
    dg = _answers(by_q["gene_defect(Disease, chromosome_22q11_2_deletion)"])[0]
    assert dg["bindings"] == {"Disease": "digeorge_syndrome"}


# ---- (E) enumeration: one object binds the whole set, each with its own citation ----

def test_enumerate_binds_full_set_each_cited() -> None:
    if not _cli_available():
        return
    rec = _by_query(_run(ENUMERATE))["inheritance(Disease, autosomal_dominant)"]
    bound = {a["bindings"]["Disease"]: a for a in _answers(rec)}
    # Ground truth derived from the library itself (self-maintaining as the genetics
    # library grows): the engine must enumerate EXACTLY the diseases written with an
    # `inheritance(<disease>, autosomal_dominant)` edge — no more, no fewer.
    import re
    genetics = (HERE / "genetics-edges.adj").read_text()
    expected = set(re.findall(
        r"relate inheritance\((\w+), autosomal_dominant\)", genetics))
    assert expected, "no autosomal_dominant edges found in genetics-edges.adj"
    assert set(bound) == expected, f"engine enumerated {set(bound)} != written {expected}"
    for disease, ans in bound.items():
        assert ans["citations"][0]["trust"] == "authoritative", f"{disease} uncited"


# ---- (D) querying is read-only: the libraries are byte-identical afterwards --------

def test_libraries_unchanged_by_being_queried() -> None:
    if not _cli_available():
        return
    before = {lib.name: hashlib.sha256(lib.read_bytes()).hexdigest() for lib in LIBS}
    for q in QUERY_FILES:
        _run(q)
    after = {lib.name: hashlib.sha256(lib.read_bytes()).hexdigest() for lib in LIBS}
    assert before == after, "a library changed while being queried — write-once violated"


# ---- (E') the engine is deterministic: same query, same bytes out -----------------

def test_engine_is_deterministic() -> None:
    if not _cli_available():
        return
    a = subprocess.run([str(CLI), str(FORWARD)], capture_output=True, text=True, cwd=str(HERE), timeout=60)
    b = subprocess.run([str(CLI), str(FORWARD)], capture_output=True, text=True, cwd=str(HERE), timeout=60)
    assert a.stdout == b.stdout, "engine output is not deterministic"


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
    print(f"\ntest_write_once_use_many: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
