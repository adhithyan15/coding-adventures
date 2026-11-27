"""The committed v2 corpora, read by the v2 library (S6a review: B2, [D64]).

``corpus.json`` is the authority; this lane has no corpus reader yet (S6b), so the
tests cut the six-key ``corpus`` part and each suite row out of it by hand and
hand them to :func:`build_suite`, then run every chunk and compare cases, calls and
both digests with the committed chunk file.
"""

from __future__ import annotations

import hashlib
import json

import pytest
from helpers import REPO_ROOT

from coding_adventures_warrant_corpus.v2 import chunks as chunk_lib
from coding_adventures_warrant_corpus.v2.errors import Refusal

FIXTURES = REPO_ROOT / "code" / "specs" / "fixtures"
CORE = frozenset({"ir.core"})


def load(name: str) -> dict:
    return json.loads((FIXTURES / f"{name}-warrant-v2" / "corpus.json").read_bytes())


def part(corpus: dict) -> dict:
    keys = ("chunk_size", "max_ir_cost", "types", "ops", "models", "model_order")
    return {k: corpus[k] for k in keys}


def chunk_file(name: str, row: dict) -> bytes:
    stem = row["claim"].replace("/", ".")
    return (FIXTURES / f"{name}-warrant-v2" / "chunks" / f"{stem}.txt").read_bytes()


def run_whole_suite(name: str, corpus: dict, row: dict) -> int:
    """Build the suite, validate its chunk file, rerun every chunk; return the cases."""
    suite = chunk_lib.build_suite(part(corpus), row, levels=CORE)
    lines = chunk_lib.check_chunk_file(chunk_file(name, row), suite)
    cases = 0
    for chunk, line in enumerate(lines):
        got = chunk_lib.run_chunk(suite, chunk)
        assert (got.cases, got.calls, got.inputs.hex()) == (
            line.cases,
            line.calls,
            line.inputs,
        ), f"{row['id']} chunk {chunk}"
        assert (got.expected.hex() if got.expected else None) == line.expected
        cases += got.cases
    assert cases == row["cases"]
    # the suite digests are the framing of the chunk digests (section 5.4)
    framed = chunk_lib.chunk_digest_framing([bytes.fromhex(x.inputs) for x in lines])
    assert framed == row["inputs_sha256"]
    return cases


def test_ct_compare_loads_and_every_suite_reproduces():
    """B2: ``eq_fixed`` has ``Bytes<N>`` with ``width_params: ["N"]``; all nine run."""
    corpus = load("ct-compare")
    assert corpus["ops"]["eq_fixed"]["width_params"] == ["N"]
    total = sum(run_whole_suite("ct-compare", corpus, r) for r in corpus["suites"])
    assert len(corpus["suites"]) == 9
    assert total == 572_109


@pytest.mark.parametrize(
    ("name", "live"), [("lzw", 4), ("x25519", 6), ("wasm-leb128", 4)]
)
def test_the_ir_core_suites_of_a_mixed_corpus_run(name, live):
    """[D64]: models and ops of other levels are not reached, so nothing is refused."""
    corpus = load(name)
    runnable = [r for r in corpus["suites"] if set(r["requires"]) <= CORE]
    assert len(runnable) == live
    for row in runnable:
        run_whole_suite(name, corpus, row)


@pytest.mark.parametrize("name", ["lzw", "x25519", "wasm-leb128"])
def test_a_suite_that_reaches_a_missing_level_is_level_missing_not_corpus(name):
    corpus = load(name)
    refused = 0
    for row in corpus["suites"]:
        if set(row["requires"]) <= CORE:
            continue
        with pytest.raises(Refusal) as caught:
            chunk_lib.build_suite(part(corpus), row, levels=CORE)
        assert caught.value.code == "level-missing", row["id"]
        refused += 1
    assert refused


def test_a_row_whose_requires_lies_about_its_reach_is_a_corpus_error():
    """R69 e: ``requires`` inside the lane's levels, yet the suite needs another."""
    corpus = load("lzw")
    row = next(r for r in corpus["suites"] if "ir.fixed" in r["requires"])
    lying = {**row, "requires": ["ir.core"]}
    with pytest.raises(Refusal) as caught:
        chunk_lib.build_suite(part(corpus), lying, levels=CORE)
    assert caught.value.code == "corpus"


def test_the_digest_of_the_committed_chunk_file_is_the_row_digest():
    corpus = load("ct-compare")
    row = corpus["suites"][0]
    data = chunk_file("ct-compare", row)
    assert hashlib.sha256(data).hexdigest() == row["chunks_sha256"]


def test_a_mixed_corpus_is_classified_and_its_ir_core_suites_run():
    """The S6a acceptance for mixed corpora: library level, no corpus reader (S6b).

    Over the models, ops and suite rows of lzw, the ``ir.core`` suites compile and
    enumerate, every other suite needs a missing level, and a model that needs one
    is classified (it has no row of its own, [D65]).
    """
    from coding_adventures_warrant_corpus.v2.ir import Program

    corpus = load("lzw")
    program = Program(
        types=corpus["types"],
        ops=corpus["ops"],
        models=corpus["models"],
        model_order=corpus["model_order"],
        levels=CORE,
        max_ir_cost=corpus["max_ir_cost"],
    )
    assert program.not_live_models and len(program.models) + len(
        program.not_live_models
    ) == len(corpus["models"])
    live, not_live = [], []
    for row in corpus["suites"]:
        try:
            suite = chunk_lib.build_suite(part(corpus), row, levels=CORE)
        except Refusal as refusal:
            assert refusal.code == "level-missing"
            not_live.append(row["id"])
        else:
            chunk_lib.run_chunk(suite, 0)
            live.append(row["id"])
    assert (len(live), len(not_live)) == (4, 17)


@pytest.mark.parametrize(
    ("name", "ok", "not_live"),
    [("ct-compare", 7, 1), ("lzw", 2, 3), ("wasm-leb128", 2, 18), ("x25519", 2, 3)],
)
def test_every_fault_row_of_a_committed_corpus_compiles_or_is_not_live(
    name, ok, not_live
):
    """N1: a fault may change a length (``drop-last-result-byte``); none is ill-typed."""
    from coding_adventures_warrant_corpus.v2.faults import compile_fault

    corpus = load(name)
    counts = {"ok": 0, "not-live": 0}
    for fault in corpus["faults"]:
        try:
            compile_fault(fault, corpus["ops"][fault["op"]], corpus["types"], CORE)
        except Refusal as refusal:
            assert refusal.code == "level-missing", (fault["fault"], refusal)
            counts["not-live"] += 1
        else:
            counts["ok"] += 1
    assert counts == {"ok": ok, "not-live": not_live}


FULL = frozenset({"ir.core", "ir.fixed", "ir.tuple"})


@pytest.mark.parametrize(
    "name", ["ct-compare", "lzw", "wasm-leb128", "x25519", "logic-gates-basic"]
)
def test_every_suite_and_fault_of_every_committed_corpus_builds_at_the_lanes_levels(
    name,
):
    """P8: no committed suite is a corpus error at ``ir.core`` + ``ir.fixed`` + ``ir.tuple``.

    A suite that needs a level the lane lacks (``ir.data``) is ``level-missing``, never
    ``corpus``; every other suite builds.  (The wasm-leb128 length formulas have an
    ``Int`` ``expected`` against a ``Nat`` ``result_type``: R42 and R51's unify.)
    """
    from coding_adventures_warrant_corpus.v2.faults import compile_fault

    corpus = load(name)
    built = 0
    for row in corpus["suites"]:
        try:
            chunk_lib.build_suite(part(corpus), row, levels=FULL)
        except Refusal as refusal:
            assert refusal.code == "level-missing", (row["id"], refusal)
            assert "ir.data" in row["requires"] or "ir.list" in row["requires"]
        else:
            built += 1
    assert built
    for fault in corpus["faults"]:
        try:
            compile_fault(fault, corpus["ops"][fault["op"]], corpus["types"], FULL)
        except Refusal as refusal:
            assert refusal.code == "level-missing", (fault["fault"], refusal)


def test_the_wasm_leb128_length_formulas_build():
    corpus = load("wasm-leb128")
    rows = [r for r in corpus["suites"] if r["claim"].endswith("_length_formula")]
    assert len(rows) >= 2
    for row in rows:
        chunk_lib.build_suite(part(corpus), row, levels=FULL)
