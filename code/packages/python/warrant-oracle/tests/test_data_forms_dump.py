"""Slice S11a: the committed dumps of the synthetic ``ir.data`` document (``tests/fixtures/data-forms-v1``).

The directory holds what the Rust generator's slice S11b must reproduce byte for byte (VER04r 13, 16.14 R35, R37):
the generated document, its lowering dump and its refusals dump.  **Appendix A.2 stops being provisional when
these files are committed** ([D47]): the tests below read the dump for *coverage* of every form of the appendix,
for the level of every unit (the R29 invariant), and for the refusal order of R35, beside pinning the bytes three
ways (the script, the oracle, canonical JSON), as ``tuple-forms-v1`` does.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import subprocess
import sys
from collections import Counter

import pytest

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.irtree import children, is_node
from coding_adventures_warrant_oracle.lowering_dump import SCHEMA, dump

FIXTURES = pathlib.Path(__file__).resolve().parent / "fixtures" / "data-forms-v1"
MAKER = FIXTURES / "make_data_forms.py"
SOURCE = FIXTURES / "data_forms_v1.warrant"
LOWERING = FIXTURES / "data_forms_v1.oracle-lowering.json"
REFUSALS = FIXTURES / "data_forms_v1.oracle-refusals.json"
FILES = [SOURCE, LOWERING, REFUSALS]


def maker():
    spec = importlib.util.spec_from_file_location("make_data_forms", MAKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def built():
    return maker().build()


@pytest.fixture(scope="module")
def lowering():
    return json.loads(LOWERING.read_bytes())


@pytest.fixture(scope="module")
def refusals():
    return json.loads(REFUSALS.read_bytes())["refusals"]


def nodes(tree):
    """Every IR node of a tree (patterns excluded), pre-order."""
    stack = [tree]
    while stack:
        node = stack.pop()
        if is_node(node):
            yield node
            stack.extend(reversed(children(node)))


def patterns(tree):
    """Every pattern of every ``match`` in a tree, at any depth."""
    for node in nodes(tree):
        if node[0] == "match":
            for pattern, _arm in node[2]:
                stack = [pattern]
                while stack:
                    p = stack.pop()
                    yield p
                    if p[0] in ("ok", "some"):
                        stack.append(p[1])
                    elif p[0] == "tuple":
                        stack.extend(p[1])
                    elif p[0] == "ctor":
                        stack.extend(p[2])


# -- the bytes -----------------------------------------------------------------------------------------------


def test_every_committed_file_is_what_the_script_writes(built):
    assert {p.name for p in built} == {p.name for p in FILES}
    for path, data in built.items():
        assert path.read_bytes() == data, path.name


def test_the_script_check_mode_passes_and_detects_a_difference(tmp_path, built):
    module = maker()
    assert module.run(["--check"]) == 0
    module.HERE, saved = tmp_path, module.HERE  # point the maker at an empty directory
    try:
        assert module.run(["--check"]) == 1  # nothing committed there: every file differs
        assert module.run([]) == 0 and (tmp_path / "data_forms_v1.warrant").exists()
        assert module.run(["--check"]) == 0
    finally:
        module.HERE = saved


def test_the_script_runs_as_a_program():
    done = subprocess.run([sys.executable, str(MAKER), "--check"], capture_output=True, text=True)
    assert done.returncode == 0, done.stderr


def test_the_dumps_are_canonical_json_with_their_schema():
    for path, schema in ((LOWERING, SCHEMA), (REFUSALS, "warrant-refusals-parity-v1")):
        raw = path.read_bytes()
        tree = json.loads(raw)
        assert raw == canonical(tree), path.name
        assert tree["schema"] == schema
        assert b"\r" not in raw and raw.endswith(b"\n") and not raw.endswith(b"\n\n") and raw.isascii()
    assert dump(SOURCE.read_bytes()) == LOWERING.read_bytes()
    assert (FIXTURES / ".gitattributes").read_text() == "* -text\n"


def test_the_dump_command_checks_the_committed_dump(capsys, tmp_path):
    assert run(["dump-lowering", "--spec", str(SOURCE), "--check", str(LOWERING)]) == 0
    assert "dump-lowering: ok" in capsys.readouterr().out
    stale = tmp_path / "stale.json"
    stale.write_bytes(LOWERING.read_bytes() + b" ")
    assert run(["dump-lowering", "--spec", str(SOURCE), "--check", str(stale)]) == 1


def test_the_dumps_carry_no_generator_name_or_version():
    for path in (LOWERING, REFUSALS):
        assert b"warrant-corpus-gen" not in path.read_bytes()


# -- the lowering dump: coverage of Appendix A.2 --------------------------------------------------------------


def test_the_document_has_every_form_of_appendix_a2(lowering):
    tags = Counter()
    pats = Counter()
    trees = [m["body"] for m in lowering["models"].values()]
    trees += [t for s in lowering["suites"] for t in (s["body"], s["admit"])]
    trees += [x["body"] for x in lowering["examples"]]
    for tree in trees:
        tags.update(n[0] for n in nodes(tree))
        pats.update(p[0] for p in patterns(tree))
    for op in lowering["ops"].values():
        if "admit" in op:
            tags.update(n[0] for n in nodes(op["admit"]))
            pats.update(p[0] for p in patterns(op["admit"]))
    for tag in ("some", "none", "ctor", "field", "match", "unwrap_or", "eq", "ne"):
        assert tags[tag] >= 3, tag
    assert tags["project"] >= 1
    for pattern in ("some", "none", "ctor", "bool", "ok", "err", "err_any", "tuple", "bind", "wild"):
        assert pats[pattern] >= 1, pattern
    # a typed none of every shape the document writes
    nones = {n[1] for m in lowering["models"].values() for n in nodes(m["body"]) if n[0] == "none"}
    assert {"U8", "Bytes", "Bytes<2>"} <= nones


def test_the_types_rows_are_the_corpus_rows(lowering):
    types = lowering["types"]
    assert list(types) == ["Blob", "Mode", "Msg", "Point", "Shape", "Slot", "Wrapped"]  # canonical key order
    assert types["Msg"] == {
        "kind": "variant",
        "ctors": [["Short", []], ["Done", ["Nat", "Bytes"]], ["Opt", ["Option<U8>"]]],
    }
    assert types["Wrapped"] == {"kind": "record", "fields": [["p", "Point"], ["tag", "Bool"]]}
    assert types["Slot"]["fields"] == [["o", "Option<U8>"], ["t", "(Bool,Nat)"]]


def test_every_unit_lowered_and_the_level_of_each_follows_r29(lowering):
    # nothing is refused: every model, claim and example of the document is in the dump
    assert len(lowering["models"]) == 95 and len(lowering["suites"]) == 103 and len(lowering["examples"]) == 11
    by_claim = {s["claim"]: tuple(s["requires"]) for s in lowering["suites"]}
    # the intermediate use of unwrap_or keeps ir.fixed only: no `requires` moved (R29 (f))
    for claim in ("c_inter_add", "c_inter_at", "c_inter_div_nat"):
        assert by_claim[claim] == ("ir.core", "ir.fixed"), claim
    # every other Option brings ir.data
    for claim in ("c_unwrap_binder", "c_prod_eq", "c_prod_in_ok", "c_prod_in_if", "c_prod_try"):
        assert by_claim[claim] == ("ir.core", "ir.data", "ir.fixed"), claim
    # a match on a Result is ir.tuple, and a data pattern inside it adds ir.data
    assert by_claim["c_m_res_plain"] == ("ir.core", "ir.tuple")
    assert by_claim["c_m_res_bool"] == by_claim["c_m_res_opt"] == ("ir.core", "ir.data", "ir.tuple")
    # a match on a bare tuple is ir.data by its scrutinee and ir.tuple by its type
    assert by_claim["c_m_tuple"] == ("ir.core", "ir.data", "ir.tuple")
    assert by_claim["c_m_bool"] == ("ir.core", "ir.data")
    assert by_claim["c_mk_point"] == ("ir.core", "ir.data")
    assert Counter(by_claim.values()) == {
        ("ir.core", "ir.data"): 81,
        ("ir.core", "ir.data", "ir.tuple"): 10,
        ("ir.core", "ir.data", "ir.fixed"): 7,
        ("ir.core", "ir.fixed"): 3,
        ("ir.core", "ir.tuple"): 2,
    }


def test_costs_follow_section_4_2_rule_6(lowering):
    models = lowering["models"]
    assert models["some_u8"]["ir_cost"] == 2  # some = 1 + operand
    assert models["none_bytes"]["ir_cost"] == 1  # none = 1
    assert models["mk_wrapped"]["ir_cost"] == 5  # ctor(1) + ctor(1 + 2) + var
    assert models["wrapped_y"]["ir_cost"] == 3  # field(field(var))
    # match = 1 + scrutinee + the worst arm, whatever the scrutinee (the longest arm costs 3 here)
    assert models["m_point"]["ir_cost"] == 5  # 1 + 1 + (1 + 1 + 1)
    assert models["m_tuple"]["ir_cost"] == 3


def test_the_ops_rows_carry_data_types_and_a_data_admit(lowering):
    ops = lowering["ops"]
    assert ops["op_some_only"]["requires"] == ["ir.core", "ir.data"]
    assert ops["op_some_only"]["admit"][0] == "match"
    assert ops["op_mk_point"]["ret"] == "Point" and ops["op_mk_point"]["requires"] == ["ir.core", "ir.data"]
    assert ops["op_m_opt_nested"]["params"] == [["o", "Option<Option<U8>>"]]
    assert ops["op_inter_add"]["requires"] == ["ir.core"]  # a scalar op: untouched by the level


# -- the refusals dump ----------------------------------------------------------------------------------------


def test_the_refusals_dump_has_every_group_and_each_row_is_what_the_oracle_says(refusals):
    assert len(refusals) == 121
    assert Counter((r["oracle_code"], r["oracle_level"]) for r in refusals) == {
        ("type", ""): 50,
        ("corpus", ""): 28,
        ("level", "ir.list"): 22,
        ("level", "ir.seq"): 10,
        ("limit", ""): 4,
        ("level", "fixed-cast"): 4,
        ("level", "err-value"): 3,
    }
    module = maker()
    for name, source, expected in module.refusal_documents():
        row = next(r for r in refusals if r["name"] == name)
        assert row["source"] == source
        code, _, level = expected.partition(":")
        assert row["oracle_code"] == code and (not level or row["oracle_level"] == level), name
    # the generator code of each: ir for a corpus error, its own name for a level and a limit
    for row in refusals:
        assert row["generator_code"] == {"corpus": "ir", "level": "level", "limit": "limit"}.get(
            row["oracle_code"], ""
        )
    assert len({r["name"] for r in refusals}) == len(refusals)


def test_no_refusal_is_ever_named_ir_data(refusals):
    """After S11a the name ``ir.data`` is not a *lowering* refusal of the oracle (R28)."""
    assert all(r["oracle_level"] != "ir.data" for r in refusals)
    assert not any("ir.data" in r["message"] for r in refusals)


def test_the_refusal_order_is_pinned_by_the_order_rows(refusals):
    by = {r["name"]: r for r in refusals}
    # (1) limits first
    assert by["order-limit-before-a-level"]["oracle_code"] == "limit"
    # (2) a level or feature over a type error of the same unit; the first in source order
    assert by["order-level-over-a-type-error-of-the-same-unit"]["oracle_level"] == "fixed-cast"
    assert by["order-the-first-level-in-source-order-list-first"]["oracle_level"] == "ir.list"
    assert by["order-the-first-level-in-source-order-seq-first"]["oracle_level"] == "ir.seq"
    # (3) patterns before arm bodies; structure before the bind rules
    assert by["order-a-later-pattern-before-an-earlier-arm-body"]["message"].endswith("unknown constructor in a pattern")
    assert "unknown constructor" in by["order-an-unknown-constructor-before-a-duplicate-bind"]["message"]
    assert "wrong number" in by["order-a-field-count-before-a-shadowing-bind"]["message"]
    assert "twice" in by["order-a-duplicate-bind-after-a-sound-structure"]["message"]
    # (4) unreachable before fall-through, on every scrutinee type
    for name in (
        "order-unreachable-before-fall-through-on-a-bool",
        "order-unreachable-before-fall-through-on-an-option",
        "order-unreachable-before-fall-through-on-a-variant",
    ):
        assert "no value can reach" in by[name]["message"], name
    # (5) units in declaration order
    assert by["order-an-earlier-level-before-a-later-type-error"]["oracle_code"] == "level"
    assert by["order-an-earlier-type-error-before-a-later-level"]["oracle_code"] == "type"
    assert by["order-a-type-declaration-before-a-later-model"]["oracle_level"] == "ir.list"
    assert by["order-a-model-before-a-later-type-declaration"]["oracle_level"] == "fixed-cast"
    assert by["order-a-signature-before-a-later-model"]["oracle_level"] == "ir.list"
    assert by["order-a-model-before-a-later-signature"]["oracle_level"] == "ir.seq"


def test_the_tuple_forms_refusal_row_that_this_slice_regenerated():
    """``a-projection-of-a-non-tuple`` became a type error (R39 (c)); only the oracle's code class moved."""
    tuple_dir = FIXTURES.parent / "tuple-forms-v1"
    rows = json.loads((tuple_dir / "tuple_forms_v1.oracle-refusals.json").read_bytes())["refusals"]
    row = next(r for r in rows if r["name"] == "a-projection-of-a-non-tuple")
    assert (row["oracle_code"], row["oracle_level"], row["generator_code"]) == ("type", "", "")
