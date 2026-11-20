"""Slice S11c-1: the committed dumps of the synthetic ``ir.data`` evaluation document (``tests/fixtures/data-eval-v1``).

The directory holds what the Rust generator's slice S11c-2 must reproduce byte for byte (VER04r 13, 16.14 [D47] to
[D54], R37): the generated document, its profile, the lowering, evaluation, fault, value, domain and refusal dumps.
The tests below pin the bytes three ways (the script, the oracle, canonical JSON) and read the dumps for coverage:
a suite on each stream header, every demotion path of section 5.3, the exact JSON of the two data faults, the two
gates, the three value encodings and the typing rows of R67 and R94.
"""

from __future__ import annotations

import importlib.util
import json
import math
import pathlib
import subprocess
import sys
from collections import Counter

import pytest

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.conformance import Refused, decode_value, parse_type
from coding_adventures_warrant_oracle.corpus_v2 import _types, canonical
from coding_adventures_warrant_oracle.domains import descriptor_size
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.lowering_dump import SCHEMA, dump
from coding_adventures_warrant_oracle.parity_dumps import EVALUATION_SCHEMA, FAULT_SCHEMA
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.stream import HEADER_V1, HEADER_V2, make_encoder

FIXTURES = pathlib.Path(__file__).resolve().parent / "fixtures" / "data-eval-v1"
MAKER = FIXTURES / "make_data_eval.py"
NAME = "data_eval_v1"
FILES = {
    FIXTURES / f"{NAME}{suffix}"
    for suffix in (
        ".warrant",
        ".profile",
        ".oracle-lowering.json",
        ".oracle-evaluation.json",
        ".oracle-faults.json",
        ".oracle-values.json",
        ".oracle-domains.json",
        ".oracle-refusals.json",
    )
}


def maker():
    spec = importlib.util.spec_from_file_location("make_data_eval", MAKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load(suffix: str):
    return json.loads((FIXTURES / f"{NAME}{suffix}").read_bytes())


@pytest.fixture(scope="module")
def built():
    return maker().build()


@pytest.fixture(scope="module")
def evaluation():
    return load(".oracle-evaluation.json")


@pytest.fixture(scope="module")
def faults():
    return load(".oracle-faults.json")


@pytest.fixture(scope="module")
def spec():
    return lower_document(parse_source((FIXTURES / f"{NAME}.warrant").read_bytes()))


# -- the bytes ------------------------------------------------------------------------------------------------------


def test_every_committed_file_is_what_the_script_writes(built):
    assert set(built) == FILES
    for path, data in built.items():
        assert path.read_bytes() == data, path.name


def test_the_script_check_mode_passes_and_detects_a_difference(tmp_path, built):
    module = maker()
    module.build = lambda: built  # the derivation is the expensive part: it ran once, above
    assert module.run(["--check"]) == 0
    module.HERE, saved = tmp_path, module.HERE
    # build() keyed its paths on the real directory: re-key them to the empty one
    module.build = lambda: {tmp_path / p.name: data for p, data in built.items()}
    try:
        assert module.run(["--check"]) == 1  # nothing committed there: every file differs
        assert module.run([]) == 0 and (tmp_path / f"{NAME}.warrant").exists()
        assert module.run(["--check"]) == 0
    finally:
        module.HERE = saved


def test_the_script_is_a_program_whose_exit_status_is_the_check():
    # The BUILD line runs ``make_data_eval.py --check`` as a program (about 30 s: it derives the whole fixture), so a
    # test does not derive it a third time; it checks the entry point and, on a tiny argument, the program's plumbing.
    text = MAKER.read_text(encoding="utf-8")
    assert text.rstrip().endswith('if __name__ == "__main__":\n    raise SystemExit(run(sys.argv[1:]))')
    done = subprocess.run(
        [
            sys.executable,
            "-c",
            "import sys; g={'__name__': 'maker', '__file__': sys.argv[1]}; "
            "exec(compile(open(sys.argv[1]).read(), sys.argv[1], 'exec'), g); "
            "g['build']=lambda: {}; sys.exit(g['run'](['--check']))",
            str(MAKER),
        ],
        capture_output=True,
        text=True,
    )
    assert done.returncode == 0 and "ok: 0 files" in done.stdout, done.stderr


def test_the_dumps_are_canonical_json_with_their_schema():
    schemas = {
        ".oracle-lowering.json": SCHEMA,
        ".oracle-evaluation.json": EVALUATION_SCHEMA,
        ".oracle-faults.json": FAULT_SCHEMA,
        ".oracle-values.json": "warrant-data-values-parity-v1",
        ".oracle-domains.json": "warrant-data-domain-parity-v1",
        ".oracle-refusals.json": "warrant-data-faults-refusals-v1",
    }
    for suffix, schema in schemas.items():
        raw = (FIXTURES / f"{NAME}{suffix}").read_bytes()
        tree = json.loads(raw)
        assert raw == canonical(tree), suffix
        assert tree["schema"] == schema
        assert b"\r" not in raw and raw.endswith(b"\n") and not raw.endswith(b"\n\n") and raw.isascii()
    assert (FIXTURES / ".gitattributes").read_text() == "* -text\n"
    assert dump((FIXTURES / f"{NAME}.warrant").read_bytes()) == (FIXTURES / f"{NAME}.oracle-lowering.json").read_bytes()


def test_the_dump_command_checks_the_committed_lowering_dump(capsys, tmp_path):
    spec_path, dump_path = FIXTURES / f"{NAME}.warrant", FIXTURES / f"{NAME}.oracle-lowering.json"
    assert run(["dump-lowering", "--spec", str(spec_path), "--check", str(dump_path)]) == 0
    assert "dump-lowering: ok" in capsys.readouterr().out
    other = tmp_path / "other.json"
    other.write_bytes(dump_path.read_bytes() + b" ")
    assert run(["dump-lowering", "--spec", str(spec_path), "--check", str(other)]) == 1


# -- the document and the evaluation dump ----------------------------------------------------------------------------


def test_the_document_declares_every_data_kind_and_every_suite_needs_ir_data_or_is_the_r29_control(evaluation):
    types = load(".oracle-lowering.json")["types"]
    kinds = Counter(row["kind"] for row in types.values())
    assert kinds == {"record": 9, "variant": 4}
    assert evaluation["levels"] == ["ir.core", "ir.data", "ir.fixed", "ir.tuple"]
    requires = {name: tuple(s["requires"]) for name, s in evaluation["suites"].items()}
    # R29: unwrap_or directly over a producer adds NO ir.data; a producer anywhere else, and unwrap_or over a binder, do
    assert requires["c_inter_add"] == ("ir.core", "ir.fixed")
    assert requires["c_prod_eq"] == requires["c_prod_try"] == ("ir.core", "ir.data", "ir.fixed")
    assert requires["c_unwrap_binder"] == ("ir.core", "ir.data", "ir.fixed")
    assert requires["c_pr"] == ("ir.core", "ir.tuple")  # (Bytes,Bool) inside a Result: a tuple, no data
    assert requires["c_m_res_opt"] == ("ir.core", "ir.data", "ir.tuple")  # a Result with a data pattern: both
    assert sum("ir.data" in r for r in requires.values()) >= 60


def test_there_is_a_suite_on_each_stream_header(evaluation):
    headers = Counter(s["stream"] for s in evaluation["suites"].values())
    assert set(headers) == {"warrant-stream-v1", "warrant-stream-v2"}
    assert HEADER_V1.rstrip(b"\0").decode() == "warrant-stream-v1" and HEADER_V2.rstrip(b"\0").decode() == "warrant-stream-v2"
    suites = evaluation["suites"]
    # tri_state's shape (Option<U8> in a Result, U8 binders) keeps the v1 header and still has the v1 digests
    for name in ("c_tri", "c_pick", "c_some_u8", "c_opt_eq", "c_m_res_opt", "c_inter_add"):
        assert suites[name]["stream"] == "warrant-stream-v1", name
        assert "v1_inputs_sha256" in suites[name], name
    # every record, variant and tuple suite is the v2 header and has no v1 digest
    for name in ("c_mk_point", "c_adder", "c_alu", "c_dir_res", "c_m_shape", "c_m_msg", "c_pr", "c_holder", "c_opt_pair"):
        assert suites[name]["stream"] == "warrant-stream-v2", name
        assert "v1_inputs_sha256" not in suites[name], name


def test_the_descriptors_the_demotions_and_the_paths_of_section_5_3(evaluation):
    suites = evaluation["suites"]
    assert suites["c_wide_w"]["demoted"] == ["0.0"]  # field 0 of a record
    assert suites["c_m_msg"]["demoted"] == ["0.Done.1.some"]  # the Option that is field 1 of Done (the spec's example)
    assert suites["c_mk_wide"]["demoted"] == ["1"]  # a binder
    assert all(s["demoted"] == [] for n, s in suites.items() if n not in ("c_wide_w", "c_m_msg", "c_mk_wide"))
    assert suites["c_m_msg"]["product"] == 18588 and suites["c_wide_w"]["product"] == 18265
    kinds = Counter(b[1]["domain"]["d"] for s in suites.values() for b in s["binders"])
    assert {"tuple", "variant", "option", "result", "fx_edge", "bytes_d1"} <= set(kinds)
    msg = suites["c_m_msg"]["binders"][0][1]
    assert msg["type"] == "Msg" and msg["domain"]["d"] == "variant"
    done = dict(msg["domain"]["ctors"])["Done"]
    assert done[1] == {"d": "option", "of": {"d": "fx_edge", "t": "U16", "size": 281}, "size": 282}


def test_every_suite_encodes_what_the_domain_enumerates_and_the_chunk_lines_cover_the_cases(evaluation):
    for name, suite in evaluation["suites"].items():
        lines = suite["chunk_lines"]
        assert len(lines) == suite["chunks"], name
        assert sum(int(line.split()[1]) for line in lines) == suite["cases"], name
        assert suite["cases"] + suite["discarded"] == suite["product"], name


def test_wraps_are_counted_inside_data_matches(evaluation):
    suites = evaluation["suites"]
    assert suites["c_m_point"]["wraps"] == suites["c_m_shape"]["wraps"] == 32640  # x + y over U8 x U8
    assert suites["c_mk_point"]["wraps"] == 0


def test_examples_carry_json_and_tfv_of_every_data_kind(evaluation):
    by = {e["id"].rsplit("/", 1)[1]: e for e in evaluation["examples"]}
    assert by["e_some"]["expected"] == {"some": "7"} and by["e_none"]["expected"] == {"none": True}
    assert by["e_point"]["expected"] == ["1", "2"]
    assert by["e_circle"]["expected"] == {"ctor": "Circle", "fields": ["3"]}
    assert by["e_dot"]["expected"] == {"ctor": "Dot", "fields": []}
    assert by["e_ok_record"]["expected"] == {"ok": ["01", "3", True]}
    assert by["e_err"]["expected"] == {"err": "Bad"}
    assert by["e_slot"]["expected"] == [{"some": "9"}, [True, "2"]]
    assert by["e_arg_option"]["args"] == [{"some": "8"}] and by["e_arg_variant"]["args"] == [
        {"ctor": "Rect", "fields": ["1", "2"]}
    ]
    tfv = {line.split()[0].rsplit("/", 1)[1]: line for line in evaluation["cases_tfv"].splitlines()[1:]}
    assert tfv["e_none"].endswith("Kexpect L0") and tfv["e_some"].endswith("Kexpect L1 I7")
    assert tfv["e_circle"].endswith("Kexpect L2 S436972636c65 L1 I3")
    assert tfv["e_dot"].endswith("Kexpect L2 S446f74 L0")
    assert tfv["e_ok_record"].endswith("Kexpect M1 S6f6b L3 X01 I3 T")
    assert evaluation["examples"][0]["id"] == "warrant/data_eval/example/e_some"


# -- the fault dump -----------------------------------------------------------------------------------------------------


R = ["var", "result"]


def test_the_fault_rows_are_the_json_of_section_8_2(faults):
    rows = {(r["op"], r["fault"]): r for r in faults["faults"]}
    assert len(rows) == 12
    sum_ok = [
        "match", R,
        [
            [["ok", ["bind", "ok_payload"]],
             ["ok", ["ctor", "Sum", [["field", ["var", "ok_payload"], 0], ["lit", {"type": "U8", "value": "0"}], ["field", ["var", "ok_payload"], 2]]]]],
            [["err_any"], R],
        ],
    ]  # fmt: skip
    assert rows["op_adder", "field-zero"]["expr"] == sum_ok and rows["op_adder", "field-zero"]["requires"] == ["ir.data", "ir.tuple"]
    assert rows["op_adder_n", "field-zero"]["expr"] == sum_ok
    assert rows["op_mk_point", "field-zero"]["expr"] == [
        "ctor", "Point", [["lit", {"type": "U8", "value": "0"}], ["field", R, 1]],
    ]  # fmt: skip
    assert rows["op_mk_point", "field-zero"]["requires"] == ["ir.data"] and rows["op_mk_point", "field-zero"]["kind"] == "post"
    assert rows["op_mk_wide", "field-zero"]["expr"][2][0] == ["lit", {"type": "U16", "value": "0"}]
    z = ["lit", {"type": "U8", "value": "0"}]
    expected_payload = {
        ("op_tri", "missing-error"): ["none", "U8"],
        ("op_pick", "missing-error"): ["var", "o"],
        ("op_adder", "missing-error"): ["ctor", "Sum", [["var", "a"], z, ["lit", False]]],
        ("op_adder_n", "missing-error"): ["ctor", "Sum", [["var", "a"], ["var", "n"], ["lit", False]]],
        ("op_alu", "missing-error"): ["ctor", "AluOut", [["var", "a"], *[["lit", False]] * 4]],
        ("op_dir_res", "missing-error"): ["var", "d"],
    }
    for key, payload in expected_payload.items():
        assert rows[key]["expr"] == ["if", ["is_err", R], ["ok", payload], R] and rows[key]["kind"] == "replace", key
    # R67: the zero literal of a component declared plain Bytes is the empty literal, in a tuple and in a record
    empty = ["lit", {"type": "Bytes", "value": ""}]
    assert rows["op_pr", "missing-error"]["expr"][2] == ["ok", ["tuple", [empty, ["lit", False]]]]
    assert rows["op_holder", "missing-error"]["expr"][2] == ["ok", ["ctor", "Holder", [empty, ["tuple", [empty, ["lit", False]]]]]]
    assert rows["op_holder", "missing-error"]["requires"] == ["ir.data", "ir.tuple"]
    assert rows["op_pr", "missing-error"]["requires"] == ["ir.tuple"]
    assert rows["op_tri", "missing-error"]["requires"] == ["ir.data"]
    assert rows["op_pick", "missing-error"]["requires"] == ["ir.core", "ir.data"]


def test_the_two_gates_of_the_fault_dump(faults):
    assert faults["gate_shipped"] == ["ir.core", "ir.fixed", "ir.tuple"]
    assert faults["gate_on"] == ["ir.core", "ir.data", "ir.fixed", "ir.tuple"]
    # before this slice every ir.data fault is deferred; ir.tuple-only faults were already live
    deferred = {c["id"] for c in faults["controls_shipped"] if any(w.get("deferred") == "ir.data" for w in c["slim_witness"])}
    live = {c["id"] for c in faults["controls_shipped"] if c["requires"] == ["ir.tuple"]}
    assert len(deferred) == 11 and len(live) == 1
    for control in faults["controls_gate_on"]:
        assert all("deferred" not in w for w in control["slim_witness"]), control["id"]
    by = {(c["op"], c["fault"]): c for c in faults["controls_gate_on"]}
    # the states: witnessed (chunk), unproven (a computed argument), blind (a claim that only sees Err)
    adder = by["op_adder", "field-zero"]
    assert [w.get("chunk", w.get("unproven")) for w in adder["slim_witness"]] == [0, True]
    assert adder["blind_claims"] == ["warrant/data_eval/exhaustive/c_adder_err_only"]
    assert faults["accepted_blind_faults"] == []
    assert faults["levels_gate_on"] == faults["levels_shipped"] == ["ir.core", "ir.data", "ir.fixed", "ir.tuple"]


# -- the value, domain and refusal tables --------------------------------------------------------------------------------


def test_every_value_row_reads_back_and_the_stream_is_the_encoder_of_section_2_4(spec):
    values = load(".oracle-values.json")
    types = _types(spec)
    assert len(values["values"]) == 28
    seen_tags = set()
    for row in values["values"]:
        ty = parse_type(row["type"], types)
        value = decode_value(row["json"], ty, types)
        assert make_encoder(spec, ty)(value).hex() == row["stream"], row
        seen_tags.add(row["stream"][:2])
    assert {"40", "41", "64", "65", "50", "51"} <= seen_tags  # Option none/some, record, variant, Ok, Err
    assert {"Option<U8>", "Option<Option<Bool>>", "Point", "Wide", "Shape", "Msg", "Result<Sum,DataError>"} <= {
        r["type"] for r in values["values"]
    }
    assert len(values["rejected"]) == 23
    for row in values["rejected"]:
        with pytest.raises(Refused) as err:
            decode_value(row["json"], parse_type(row["type"], types), types)
        assert err.value.code == "value" and err.value.why == row["message"], row


def test_the_domain_table_demotions_and_consumer_verdicts():
    domains = load(".oracle-domains.json")
    assert domains["domains"]["Dir"]["size"] == 7 and len(domains["domains"]["Dir"]["values"]) == 7
    assert domains["domains"]["Option<Bool>"]["values"] == [{"none": True}, {"some": False}, {"some": True}]
    assert domains["domains"]["Point"]["head"][0] == ["0", "0"] and domains["domains"]["Point"]["tail"][-1] == ["255", "255"]
    paths = {tuple(d["binders"]): d["demoted"] for d in domains["demotions"]}
    assert paths == {
        ("Wide",): ["0.0"],
        ("Msg",): ["0.Done.1.some"],
        ("Option<U16>", "Nat"): ["0.some"],
        ("Nat", "U16"): ["1"],
        ("Result<Wide,DataError>",): ["0.ok.0"],
        ("Dir", "Slot"): [],
    }
    for row in domains["demotions"]:
        assert math.prod(row["sizes"]) <= row["budget"]
    accepted = [v for v in domains["consumer"] if v.get("accepted")]
    rejected = [v for v in domains["consumer"] if not v.get("accepted")]
    assert len(accepted) == 5 and len(rejected) == 11
    for v in accepted:
        assert descriptor_size(v["descriptor"]) == v["size"]
    for v in rejected:
        with pytest.raises(OracleError) as err:
            descriptor_size(v["descriptor"])
        assert (err.value.code, err.value.message) == (v["oracle_code"], v["message"])


def test_the_refusal_dump_pins_the_data_fault_signatures_and_the_typing_of_r67_and_r94():
    refusals = load(".oracle-refusals.json")
    assert len(refusals["refusals"]) == 16
    for row in refusals["refusals"]:
        assert row["oracle_code"] == "generation" and row["message"].startswith("fault:"), row["name"]
        assert row["source"].startswith("warrant refusals v1\n")
    names = {r["name"] for r in refusals["refusals"]}
    assert "field-zero/a-record-with-no-numeric-field" in names and "wrong-ctor/is-not-in-the-registry" in names
    typing = {t["name"]: t for t in refusals["typing"]}
    assert typing["r67/a-tuple-payload-with-a-plain-bytes-zero"]["accepted"]
    assert typing["r67/a-tuple-payload-with-a-plain-bytes-zero"]["type"] == "Result<(Bytes,Bool),E>"  # R98b: holes filled, never `?`
    assert typing["r67/a-record-field-that-is-a-tuple-with-a-plain-bytes-zero"]["accepted"]
    assert not typing["r67/a-one-byte-literal-in-a-bytes-2-component"].get("accepted")
    assert "no common type" in typing["r67/a-one-byte-literal-in-a-bytes-2-component"]["message"]
    assert typing["r67/a-one-byte-literal-in-a-record-field-of-bytes-2"]["oracle_code"] == "ir"
    assert typing["r94/plain-bytes-against-a-declared-bytes-2"]["type"] == "Bytes"
    assert typing["r94/an-int-expression-against-a-declared-nat-result"]["type"] == "Int"
    assert [t["name"] for t in refusals["typing"] if not t.get("accepted")] == [
        "r67/a-one-byte-literal-in-a-bytes-2-component",
        "r67/a-one-byte-literal-in-a-record-field-of-bytes-2",
        "r94/a-bool-against-a-declared-bytes-result",
        "r94/a-bytes-against-a-declared-u8-result",
        "r94/a-bool-ok-in-the-else-branch-of-a-u8-result",
        "r94/a-bool-ok-in-the-second-match-arm-of-a-u8-result",
    ]
    assert all("?" not in t["type"] for t in refusals["typing"] if t.get("accepted"))
