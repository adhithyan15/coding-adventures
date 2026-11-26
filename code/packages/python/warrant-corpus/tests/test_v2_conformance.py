"""Run the whole v2 conformance kit through this lane's own library (slice S6a).

``code/specs/fixtures/warrant-corpus-conformance-v1`` holds 1,419 v2 cases (VER04r section 11): 1,243 in the six
categories this lane runs, of which S10d's 461 add ``ir.fixed`` and ``ir.tuple`` vectors (400 of them skipped here by
their ``requires``, 61 that need ``ir.core`` alone run), and the 176 of ``v2-binding`` (S5d),
which it indexes and digest-checks but skips whole until S6b.  This lane runs every case whose ``requires`` it
implements, in a category it knows, and skips the rest; per category, **run plus skipped equals the index
count**, and every case of the six whose ``requires`` it implements runs: 843 run, 0 failed.  The ``also_sequential`` chunk
cases are walked sequentially too ("chunk ``c`` alone equals sequential").

The harness itself is tested here (digests, accounting, skipping, failure
naming); its falsification by wrong evaluators is ``test_v2_falsification.py``.
"""

from __future__ import annotations

import hashlib
import json

import pytest
from helpers import REPO_ROOT
from test_v2_chunks import A_AND_B, REFINED, sealed

from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.v2 import STANDARD, run_kit
from coding_adventures_warrant_corpus.v2 import chunks as chunk_lib
from coding_adventures_warrant_corpus.v2.kit import (
    RUNNERS,
    _run_case,
    _SequentialWalks,
    canonical,
    load_family,
    outcome_chunk,
)

FIXTURE = REPO_ROOT / "code" / "specs" / "fixtures" / "warrant-corpus-conformance-v1"
V2 = ("v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault")
#: S5d commits ``v2-binding.json``: indexed and digest-checked, and skipped whole (S6b runs it).
BINDING = "v2-binding"
V2_INDEXED = (*V2, BINDING)


@pytest.fixture(scope="module")
def full_report():
    """ONE complete run, sequential walks included (about a minute)."""
    return run_kit(FIXTURE)


def test_every_case_runs_and_passes(full_report):
    assert full_report.ok, full_report.lines()
    assert list(full_report.categories) == list(V2_INDEXED)
    assert sum(c.run for c in full_report.categories.values()) == 843
    assert all(not c.failed for c in (full_report.categories[n] for n in V2))
    # S10d's cases that need ir.fixed or ir.tuple are skipped by their requires: 400 of the 461 it added.
    skipped = {n: len(full_report.categories[n].skipped) for n in V2}
    assert skipped == {"v2-value": 39, "v2-arith": 0, "v2-ir": 246, "v2-domain": 43, "v2-chunk": 15, "v2-fault": 57}
    binding = full_report.categories[BINDING]
    assert (binding.indexed, binding.run, len(binding.skipped), binding.failed) == (176, 0, 176, {})
    assert binding.accounted and binding.ok


@pytest.mark.parametrize("category", V2)
def test_run_plus_skipped_equals_the_index_count(full_report, category):
    row = full_report.categories[category]
    index = json.loads((FIXTURE / "index.json").read_text())
    indexed = next(r["cases"] for r in index["files"] if r["category"] == category)
    assert row.indexed == indexed
    assert row.run + len(row.skipped) == indexed
    assert row.accounted and row.ok


def test_report_lines_summarise_each_category(full_report):
    lines = full_report.lines()
    assert lines[0] == "v2-value: run 138, skipped 39, failed 0 (indexed 177)"
    assert all(not line.startswith("not ok") for line in lines)


# -- the harness's own contract ----------------------------------------------
def make_family(directory, categories):
    """Write a scratch fixture family: one file per category and a matching index."""
    rows = []
    for name, cases in categories.items():
        raw = (json.dumps({"cases": cases}, sort_keys=True) + "\n").encode()
        (directory / f"{name}.json").write_bytes(raw)
        rows.append(
            {
                "cases": len(cases),
                "category": name,
                "path": f"{name}.json",
                "sha256": hashlib.sha256(raw).hexdigest(),
            }
        )
    total = sum(r["cases"] for r in rows)
    index = {
        "files": rows,
        "schema": "warrant-corpus-conformance-v1",
        "total_cases": total,
    }
    (directory / "index.json").write_text(json.dumps(index))
    return directory


def arith_case(slug, expect, requires=("ir.core",), **fields):
    base = {"type": "U8", "op": "add", "lhs": "255", "rhs": "1"}
    return {
        "id": f"v2-arith/{slug}",
        "summary": slug,
        "requires": list(requires),
        "expect": expect,
        **(base | fields),
    }


def test_a_case_that_needs_a_missing_level_is_skipped_not_dropped(tmp_path):
    family = make_family(
        tmp_path,
        {
            "v2-arith": [
                arith_case("core", {"type": "U8", "value": "0"}),
                arith_case(
                    "fixed",
                    {"type": "U8", "value": "0"},
                    requires=("ir.core", "ir.fixed"),
                ),
            ],
            "v2-binding": [
                {"id": "v2-binding/x", "requires": ["ir.core"], "expect": {}}
            ],
        },
    )
    report = run_kit(family)
    arith, binding = report.categories["v2-arith"], report.categories["v2-binding"]
    assert (arith.run, arith.skipped, arith.accounted) == (1, ["v2-arith/fixed"], True)
    # a category this runner does not know is skipped whole, never run
    assert (binding.run, binding.skipped, binding.accounted) == (
        0,
        ["v2-binding/x"],
        True,
    )
    assert report.ok


def test_a_runner_with_no_levels_skips_everything_and_still_accounts(tmp_path):
    report = run_kit(FIXTURE, levels=frozenset(), categories=("v2-value", "v2-fault"))
    assert report.ok
    assert all(c.run == 0 and c.accounted for c in report.categories.values())
    assert report.categories["v2-value"].skipped[0] == "v2-value/bool-true"


def test_a_wrong_expectation_is_reported_by_id_and_outcome_only(tmp_path):
    wrong = arith_case("wrong", {"type": "U8", "value": "1"})
    report = run_kit(make_family(tmp_path, {"v2-arith": [wrong]}))
    assert not report.ok
    assert report.failed_ids == ["v2-arith/wrong"]
    line = report.lines()[-1]
    assert line.startswith("not ok v2-arith/wrong: outcome ")
    assert '"value":"0"' in line and '"value":"1"' in line


def test_a_long_outcome_is_abbreviated_in_the_failure_line(tmp_path):
    case = {
        "id": "v2-value/long",
        "summary": "s",
        "requires": ["ir.core"],
        "type": "Bytes",
        "types": {},
        "input_hex": bytes.fromhex("22").hex() + ("61" * 600) + "22",
        "expect": {"outcome": "reject", "error": "value"},
    }
    report = run_kit(make_family(tmp_path, {"v2-value": [case]}))
    assert report.failed_ids == ["v2-value/long"]
    assert report.lines()[-1].count("...") == 1


def test_a_broken_case_is_a_kit_error_and_a_crash_is_a_failure_not_an_exception(
    tmp_path,
):
    bad_node = arith_case("bad-op", {"error": "ir-overflow"}, op="frobnicate")
    no_fields = {"id": "v2-arith/crash", "requires": ["ir.core"], "expect": {}}
    negative = {
        "id": "v2-domain/negative",
        "requires": ["ir.core"],
        "expect": {},
        "descriptor": {"d": "bool", "size": 2},
        "type": "Bool",
        "index": -1,
    }
    report = run_kit(
        make_family(
            tmp_path, {"v2-arith": [bad_node, no_fields], "v2-domain": [negative]}
        )
    )
    failed = report.categories["v2-arith"].failed
    assert failed["v2-arith/bad-op"].startswith("kit error: a v2-arith node is refused")
    assert failed["v2-arith/crash"] == "crashed with KeyError"
    assert (
        report.categories["v2-domain"]
        .failed["v2-domain/negative"]
        .startswith("kit error")
    )


def test_only_and_categories_restrict_what_is_counted(tmp_path):
    report = run_kit(
        FIXTURE,
        categories=("v2-arith",),
        only=lambda case: case["id"].endswith("-wrap"),
    )
    arith = report.categories["v2-arith"]
    assert list(report.categories) == ["v2-arith"]
    assert 0 < arith.run < 129 and arith.accounted and report.ok


def test_the_canonical_form_does_not_confuse_true_with_one():
    assert canonical({"value": True}) != canonical({"value": 1})


# -- the kit is read with digests, counts and totals checked -------------------
def test_a_tampered_category_file_is_refused(tmp_path):
    family = make_family(
        tmp_path, {"v2-arith": [arith_case("a", {"type": "U8", "value": "0"})]}
    )
    (family / "v2-arith.json").write_bytes(b'{"cases": []}\n')
    with pytest.raises(CorpusError, match="index digest"):
        load_family(family)


def test_a_wrong_case_count_or_total_is_refused(tmp_path):
    family = make_family(
        tmp_path, {"v2-arith": [arith_case("a", {"type": "U8", "value": "0"})]}
    )
    index = json.loads((family / "index.json").read_text())
    index["files"][0]["cases"] = 2
    (family / "index.json").write_text(json.dumps(index))
    with pytest.raises(CorpusError, match="case count"):
        load_family(family)
    index["files"][0]["cases"] = 1
    index["total_cases"] = 9
    (family / "index.json").write_text(json.dumps(index))
    with pytest.raises(CorpusError, match="total_cases"):
        load_family(family)


def test_a_malformed_index_is_refused(tmp_path):
    (tmp_path / "index.json").write_text('{"files": 3}')
    with pytest.raises(CorpusError, match="malformed"):
        load_family(tmp_path)


def test_v1_rows_are_counted_in_the_total_but_never_run_here():
    family = load_family(FIXTURE)
    assert set(family.cases) == set(V2_INDEXED)
    assert family.index["total_cases"] == sum(r["cases"] for r in family.index["files"])
    assert set(RUNNERS) == set(V2)


# -- "chunk c alone equals sequential", as the harness checks it -----------------------
def sequential_case(chunk):
    corpus, row, data = sealed(A_AND_B, **REFINED)
    case = {
        "id": "v2-chunk/test",
        "requires": ["ir.core"],
        "corpus": corpus,
        "suite": row,
        "chunk_file_hex": data.hex(),
        "chunk": chunk,
        "also_sequential": True,
    }
    case["expect"] = outcome_chunk(case, STANDARD)
    return case


def test_a_sequential_case_passes_and_the_walk_is_shared_across_cases():
    cases = [sequential_case(2), sequential_case(9)]
    walks = _SequentialWalks(STANDARD)
    walks.plan(cases)
    assert [_run_case(c, outcome_chunk, STANDARD, walks) for c in cases] == [None, None]
    assert len(walks._walks) == 1 and len(next(iter(walks._walks.values()))) == 10


def test_a_sequential_walk_that_disagrees_with_the_file_is_reported_as_the_failure():
    case = sequential_case(3)
    walks = _SequentialWalks(STANDARD)
    suite = chunk_lib.build_suite(case["corpus"], case["suite"])
    wrong = chunk_lib.run_sequential(suite, 3)
    wrong[0] = chunk_lib.ChunkResult(1, 1, b"\x00" * 32, b"\x00" * 32)
    walks._walks[case["suite"]["chunks_sha256"]] = wrong
    problem = _run_case(case, outcome_chunk, STANDARD, walks)
    assert problem == "chunk 0: the sequential walk disagrees with the chunk file"


def test_a_malformed_env_value_and_an_undetermined_result_are_kit_errors(tmp_path):
    env_case = {
        "id": "v2-ir/bad-env",
        "requires": ["ir.core"],
        "ir": ["lit", True],
        "env": [["a", {"type": "U8", "value": "300"}]],
        "levels": ["ir.core"],
        "types": {},
        "ops": {},
        "models": {},
        "model_order": [],
        "subjects": {},
        "expect": {},
    }
    hole = env_case | {
        "id": "v2-ir/hole",
        "ir": ["ok", ["lit", True]],
        "env": [],
        "types": {"E": {"kind": "error", "cases": ["B"]}},
    }
    range_case = {
        "id": "v2-domain/range",
        "requires": ["ir.core"],
        "descriptor": {"d": "bool", "size": 2},
        "type": "Bool",
        "range": [0],
        "expect": {},
    }
    report = run_kit(
        make_family(tmp_path, {"v2-ir": [env_case, hole], "v2-domain": [range_case]})
    )
    failed = {
        **report.categories["v2-ir"].failed,
        **report.categories["v2-domain"].failed,
    }
    assert failed["v2-ir/bad-env"].startswith("kit error: an env value is malformed")
    assert failed["v2-ir/hole"].startswith("kit error: R41")
    assert failed["v2-domain/range"] == "kit error: range is a pair [lo, hi]"
