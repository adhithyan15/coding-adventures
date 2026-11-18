"""Slice S10a: the committed lowering dump of the synthetic ``ir.fixed`` document.

``tests/fixtures/fixed-forms-v1/fixed_forms_v1.oracle-lowering.json`` is what slice S10b (the
Rust generator) must reproduce byte for byte.  The tests pin it three ways: the document is
regenerated from ``make_fixed_forms.py``, the dump is regenerated from the document, and the
dump is read for *coverage* (every one of the 35 built-ins, at every width the spec allows).
"""

from __future__ import annotations

import importlib.util
import json
import pathlib

import pytest

from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import walk
from coding_adventures_warrant_oracle.lowering_dump import SCHEMA, dump, view

FIXTURES = pathlib.Path(__file__).resolve().parent / "fixtures" / "fixed-forms-v1"
SOURCE = FIXTURES / "fixed_forms_v1.warrant"
DUMP = FIXTURES / "fixed_forms_v1.oracle-lowering.json"
MAKER = FIXTURES / "make_fixed_forms.py"

TYPES = ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"]
SIGNED = [t for t in TYPES if t[0] == "I"]
UNSIGNED = [t for t in TYPES if t[0] == "U"]


def _maker():
    spec = importlib.util.spec_from_file_location("make_fixed_forms", MAKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_the_committed_document_is_what_the_generator_script_writes():
    assert SOURCE.read_bytes() == _maker().main().encode("utf-8")


def test_the_committed_dump_is_what_the_oracle_writes():
    assert DUMP.read_bytes() == dump(SOURCE.read_bytes())


def test_the_dump_is_canonical_json_and_deterministic():
    raw = DUMP.read_bytes()
    tree = json.loads(raw)
    assert raw == (json.dumps(tree, sort_keys=True, indent=2, ensure_ascii=True) + "\n").encode()
    assert dump(SOURCE.read_bytes()) == dump(SOURCE.read_bytes())
    assert b"\r" not in raw and raw.endswith(b"\n") and not raw.endswith(b"\n\n")
    assert tree["schema"] == SCHEMA == "warrant-lowering-parity-v1"


def _all_nodes(tree: dict):
    for m in tree["models"].values():
        yield from walk(m["body"])
    for s in tree["suites"]:
        yield from walk(s["admit"])
        yield from walk(s["body"])
    for e in tree["examples"]:
        yield from walk(e["body"])
    for o in tree["ops"].values():
        if "admit" in o:
            yield from walk(o["admit"])


def test_the_dump_covers_every_one_of_the_35_built_ins():
    tree = json.loads(DUMP.read_bytes())
    seen = {n[0] for n in _all_nodes(tree)}
    assert seen >= L.FIXED_TAGS, sorted(L.FIXED_TAGS - seen)
    assert len(L.FIXED_TAGS) == 35


def _param_types(model: dict) -> list:
    return [t for _, t in model["params"]]


def test_every_width_bearing_built_in_is_covered_at_every_allowed_width():
    models = json.loads(DUMP.read_bytes())["models"]
    for t in TYPES:
        s = t.lower()
        for name in ("bit_and", "bit_or", "bit_xor", "bit_not", "shl", "shr", "rotl", "rotr",
                     "popcount", "clz", "ctz", "bit", "max_value", "min_value", "to_int",
                     "checked_add", "checked_sub", "checked_mul", "be_bytes", "le_bytes",
                     "from_be", "from_le"):
            assert f"m_{name}_{s}" in models, (name, t)
    for t in SIGNED:
        for name in ("sar", "div_trunc", "rem_trunc"):
            assert f"m_{name}_{t.lower()}" in models
    for t in UNSIGNED:
        for name in ("to_nat", "div", "rem"):
            assert f"m_{name}_{t.lower()}" in models
    # signed widths never get div/rem, nor unsigned ones sar/div_trunc: those are refused shapes
    assert not any(f"m_div_{t.lower()}" in models for t in SIGNED)
    assert not any(f"m_sar_{t.lower()}" in models for t in UNSIGNED)
    # conversions: widen for every source whose range is inside the target (26 pairs),
    # wrap and try for every source (8 fixed + Int + Nat) and every target
    widen = [n for n in models if n.startswith("m_widen_")]
    assert len(widen) == 26
    for dst in TYPES:
        for src in TYPES + ["Int", "Nat"]:
            assert f"m_wrap_{src.lower()}_{dst.lower()}" in models
            assert f"m_try_{src.lower()}_{dst.lower()}" in models
    assert sum(n.startswith("m_wrap_") for n in models) == 80 + 1  # + m_wrap_around
    # endian: the width follows the type
    for t in TYPES:
        n = int(t[1:]) // 8
        assert models[f"m_be_bytes_{t.lower()}"]["ret"] == f"Bytes<{n}>"
        assert _param_types(models[f"m_from_le_{t.lower()}"]) == [f"Bytes<{n}>"]
    # div/rem on Int and Nat (R7), imin/imax on every pair, at on Bytes and Bytes<N>
    for tag in ("div", "rem", "imin", "imax"):
        for a in ("nat", "int"):
            for b in ("nat", "int"):
                assert f"m_{tag}_{a}_{b}" in models
    assert {"m_at_nat", "m_at_int", "m_at_fixed", "m_at_literal_index"} <= set(models)
    # a bare literal on the left of a Nat operand, imin with a literal, from_be over an if
    assert {"m_div_literal_left", "m_rem_literal_left", "m_imin_literal", "m_from_be_if"} <= set(models)


def test_the_dump_records_computed_requires_and_costs():
    tree = json.loads(DUMP.read_bytes())
    by = {s["claim"]: s for s in tree["suites"]}
    assert by["c_core_only"]["requires"] == ["ir.core"]
    assert by["c_closure_only"]["requires"] == ["ir.core", "ir.fixed"]  # through the closure
    assert by["c_low_u8"]["requires"] == ["ir.core", "ir.fixed"]
    assert all(set(s["requires"]) <= {"ir.core", "ir.fixed"} for s in tree["suites"])
    assert tree["ops"]["op_low_u8"]["requires"] == ["ir.core", "ir.fixed"]
    assert tree["ops"]["op_and_u8"]["requires"] == ["ir.core"]
    assert tree["models"]["m_max_value_u8"]["ir_cost"] == 1
    assert tree["models"]["m_bit_and_u8"]["ir_cost"] == 3
    assert tree["models"]["m_use_use"]["ir_cost"] == 17
    # admit (``true``, 1) + body (``op(a, b) == bit_and(a, b)``: 1 + 3 + 3) + expected (3)
    assert by["c_and_u8"]["ir_cost"] == 1 + (1 + 3 + 3) + 3
    assert "requires" not in tree["models"]["m_rotl_u32"]  # a model has none of its own (1.4)


def test_every_view_row_has_the_keys_the_generator_will_compare():
    tree = view(SOURCE.read_bytes())
    assert set(tree) == {
        "schema", "module", "document", "errors", "ops", "model_order", "models", "suites",
        "examples",
    }
    assert set(tree["suites"][0]) == {
        "id", "claim", "kind", "binders", "admit", "body", "subject", "expected",
        "result_type", "ops", "requires", "ir_cost",
    }
    assert tree["model_order"] == list(tree["models"])


def test_the_dump_refuses_a_document_it_cannot_show():
    base = "warrant d v1\n  op f(a: U8) -> U8\n"
    for body, level in (
        ("  model m(a: Int) -> Bool = div(a, a) == div(a, a)\n", "ir.data"),
        ("  claim c:\n    forall a: U8. f(a) == [a]\n", "ir.data"),
        ("  op g(a: U8) -> Option<U8>\n", "ir.data"),
    ):
        with pytest.raises(OracleError) as err:
            dump((base + body).encode())
        assert err.value.code == "level" and err.value.level == level, body


def test_the_dump_covers_core_documents_too_and_costs_are_bounded():
    out = json.loads(dump(b"warrant c v1\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8. f(a) == a\n"))
    assert out["suites"][0]["requires"] == ["ir.core"]
    assert out["suites"][0]["ir_cost"] == 1 + (1 + 2 + 1) + 1  # admit, body, expected
    with pytest.raises(OracleError) as err:  # an expression above the bound is a limit error
        dump(SOURCE.read_bytes(), max_cost=2)
    assert err.value.code == "limit"


# -- the CLI --------------------------------------------------------------------------------------


def test_dump_lowering_check_passes_for_the_committed_dump(capsys):
    assert run(["dump-lowering", "--spec", str(SOURCE), "--check", str(DUMP)]) == 0
    assert "dump-lowering: ok" in capsys.readouterr().out


def test_dump_lowering_check_fails_when_the_dump_differs(tmp_path, capsys):
    stale = tmp_path / "stale.json"
    stale.write_bytes(DUMP.read_bytes() + b" ")
    assert run(["dump-lowering", "--spec", str(SOURCE), "--check", str(stale)]) == 1
    assert "DIFFERS" in capsys.readouterr().err


def test_dump_lowering_writes_the_same_bytes(tmp_path, capsys):
    out = tmp_path / "out.json"
    assert run(["dump-lowering", "--spec", str(SOURCE), "--out", str(out)]) == 0
    assert out.read_bytes() == DUMP.read_bytes()
    assert "wrote" in capsys.readouterr().out


def test_dump_lowering_refuses_to_overwrite_its_input_and_bad_bounds(tmp_path, capsys):
    copy = tmp_path / "doc.warrant"
    copy.write_bytes(SOURCE.read_bytes())
    assert run(["dump-lowering", "--spec", str(copy), "--out", str(copy)]) == 2
    assert run(["dump-lowering", "--spec", str(SOURCE), "--max-ir-cost", "10", "--out", str(tmp_path / "x")]) == 2
    err = capsys.readouterr().err
    assert "input file" in err and "max_ir_cost" in err


def test_dump_lowering_reports_a_document_beyond_ir_fixed(tmp_path, capsys):
    doc = tmp_path / "d.warrant"
    doc.write_text("warrant d v1\n  model m(a: U8) -> Bool = [a] == [a]\n")
    assert run(["dump-lowering", "--spec", str(doc), "--out", str(tmp_path / "o.json")]) == 2
    assert "beyond ir.tuple" in capsys.readouterr().err
