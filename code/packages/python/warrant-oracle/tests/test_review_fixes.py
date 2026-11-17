"""Regression tests for the review of S4 part 1 (findings 1-10 and the cost amendment)."""

from __future__ import annotations

import copy
import json
import os

import pytest
from conftest import SPECS, V1_CORPUS, adopter_bytes, write_lf

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v1 import build_program, verify_v1
from coding_adventures_warrant_oracle.emit import emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import ModelDef, Program, compile_calls
from coding_adventures_warrant_oracle.lower import OpSig, lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.profile import parse_profile
from coding_adventures_warrant_oracle.stream import make_encoder
from coding_adventures_warrant_oracle.suites import SuitePlan, Work

T = A.Ty
SPEC = adopter_bytes("ct-compare")
CORPUS = json.loads(V1_CORPUS.read_bytes())


def gate(mutate, digests=False):
    corpus = copy.deepcopy(CORPUS)
    mutate(corpus)
    return verify_v1(SPEC, json.dumps(corpus).encode(), digests=digests)[0]


# -- 1: type-strict comparison ------------------------------------------------------


@pytest.mark.parametrize(
    "mutate",
    [
        lambda c: (
            c["models"]["same"]["body"].__setitem__(0, "eq")
            or c["suites"][0].__setitem__("discarded", False)
        ),
        lambda c: c["suites"][0].__setitem__("cases", float(c["suites"][0]["cases"])),
        lambda c: c["ops"]["select_bytes"].__setitem__("fresh", 1),
        lambda c: c["suites"][0].__setitem__("admit", True),
        lambda c: c["suites"][0].__setitem__("expected_calls", True),
    ],
)
def test_bool_int_float_confusions_are_mismatches(mutate):
    assert not gate(mutate, digests=True).ok


def test_true_is_not_the_literal_one():
    def mutate(c):
        c["suites"][0]["admit"] = ["lit", 1]

    assert not gate(mutate).ok


# -- 2: closed suite keys, ordered faults and controls --------------------------------


def test_extra_and_missing_suite_keys_are_refused():
    def extra(c):
        c["suites"][0]["smuggled"] = 1

    def missing(c):
        del c["suites"][0]["domain_text"]

    for m in (extra, missing):
        r = gate(m)
        assert not r.ok and any(x["code"] == "suite-keys" for x in r.mismatches)


def test_reordered_faults_and_controls_are_mismatches():
    def faults(c):
        c["faults"].reverse()

    def controls(c):
        c["controls"].reverse()

    assert not gate(faults).ok and not gate(controls).ok


# -- 3: a model the oracle cannot check never counts as agreement -----------------------


def test_unsupported_models_are_reported_not_agreed():
    src = b"warrant ct_compare v1\n  op f(a: U8) -> U8\n  model m(a: U8) -> U8 = bit_not(a)\n  claim c:\n    forall a: U8. f(a) == a\n"
    import hashlib

    corpus = copy.deepcopy(CORPUS)
    corpus["spec_sha256"] = hashlib.sha256(src).hexdigest()
    report, _ = verify_v1(src, json.dumps(corpus).encode(), digests=False)
    assert any(x["code"] == "level" and x["where"] == "models.m" for x in report.mismatches)


# -- 10: non-dict rows ---------------------------------------------------------------------


@pytest.mark.parametrize("bad", [5, "x", None, [1]])
def test_non_dict_rows_are_clean_refusals(bad):
    corpus = copy.deepcopy(CORPUS)
    corpus["suites"][0] = bad
    with pytest.raises(OracleError) as e:
        verify_v1(SPEC, json.dumps(corpus).encode(), digests=False)
    assert e.value.code == "corpus"
    corpus = copy.deepcopy(CORPUS)
    corpus["models"] = [1]
    r, _ = verify_v1(SPEC, json.dumps(corpus).encode(), digests=False)
    assert not r.ok


# -- 4: subject calls inside model_call arguments -----------------------------------------


def test_subject_calls_in_model_arguments_are_counted():
    op = OpSig("bump", (), [("a", T("U8"))], T("U8"), False)
    inc = ModelDef(
        "inc",
        [("x", T("U8"))],
        T("U8"),
        ["add", ["var", "x"], ["lit", {"type": "U8", "value": "1"}]],
    )
    prog = Program({"inc": inc}, {"bump": op})
    ir = [
        "eq",
        ["model_call", "inc", [["subject_op", "bump", [["var", "a"]]]]],
        ["lit", {"type": "U8", "value": "3"}],
    ]
    assert compile_calls(prog, ir, {"a": T("U8")}) == 1


# -- 5: wraps are independent of memoisation and order, and see subject arguments -------


WRAPS = b"""warrant w v1
  op f(a: U8) -> U8
  model plus(a: U8) -> U8 = a + 200u8
  claim one:
    forall a: U8. f(a) == plus(a)
  claim two:
    forall a: U8. f(a) == plus(a)
  claim arg:
    forall a: U8. f(a + 200u8) == a
"""


def plan(src, claim, prog=None, **kw):
    spec = lower_document(parse_source(src))
    return SuitePlan(
        spec, prog or build_program(spec), next(s for s in spec.suites if s.claim == claim), **kw
    )


def test_wraps_do_not_depend_on_memo_hits_or_suite_order():
    spec = lower_document(parse_source(WRAPS))
    prog = build_program(spec)
    got = {}
    for name in ("two", "one", "one"):  # a shared program: later suites hit the memo
        s = next(x for x in spec.suites if x.claim == name)
        got.setdefault(name, []).append(SuitePlan(spec, prog, s).run("full").wraps)
    assert got["one"] == [200, 200] and got["two"] == [200]
    fresh = build_program(spec, memo=False)
    assert SuitePlan(spec, fresh, spec.suites[0]).run("full").wraps == 200


def test_wraps_include_pure_subject_arguments():
    assert plan(WRAPS, "arg").run("full").wraps == 200


def test_wraps_do_not_change_with_depth():
    assert plan(WRAPS, "one").run("slim").wraps == plan(WRAPS, "one").run("full").wraps


# -- 6: recursion is a clean refusal ---------------------------------------------------------


def test_four_hundred_chained_models_are_a_limit_error():
    lines = ["warrant deep", "  op f(a: U8) -> U8", "  model m0(a: U8) -> U8 = a"]
    lines += [f"  model m{i}(a: U8) -> U8 = m{i - 1}(a)" for i in range(1, 400)]
    lines += ["  claim c:", "    forall a: U8. f(a) == m399(a)"]
    spec = lower_document(parse_source("\n".join(lines) + "\n"))
    with pytest.raises(OracleError) as e:
        SuitePlan(spec, build_program(spec), spec.suites[0])
    assert e.value.code == "limit"


def test_two_thousand_term_sum_is_a_limit_error():
    body = " + ".join(["1"] * 2000)
    with pytest.raises(OracleError) as e:
        lower_document(parse_source(f"warrant s\n  model m() -> Int = {body}\n"))
    assert e.value.code == "limit"


def test_cli_converts_recursion_to_exit_2(tmp_path, capsys):
    body = " + ".join(["1"] * 2000)
    p = tmp_path / "s.warrant"
    write_lf(p, f"warrant s\n  model m() -> Int = {body}\n")
    assert run(["check", "--spec", str(p)]) == 2


# -- 7: emit exit status and output path -------------------------------------------------------


def test_emit_with_not_live_or_refused_suites_exits_3_unless_allowed(tmp_path):
    out = tmp_path / "o.json"
    spec = SPECS / "lzw.warrant"
    assert run(["emit-v2", "--spec", str(spec), "--out", str(out)]) == 3
    assert out.exists()  # the partial result is still written
    assert run(["emit-v2", "--spec", str(spec), "--out", str(out), "--allow-partial"]) == 0
    assert run(["emit-v2", "--spec", str(SPECS / "ct-compare.warrant"), "--out", str(out)]) == 0


def test_out_may_not_overwrite_an_input_or_land_in_the_spec_tree(tmp_path):
    spec = tmp_path / "x.warrant"
    spec.write_bytes(SPEC)
    assert run(["emit-v2", "--spec", str(spec), "--out", str(spec)]) == 2
    assert spec.read_bytes() == SPEC
    assert run(["emit-v2", "--spec", str(spec), "--out", str(SPECS / "oops.json")]) == 2
    assert not (SPECS / "oops.json").exists()
    link = tmp_path / "link.json"
    link.symlink_to(tmp_path / "target.json")
    assert run(["emit-v2", "--spec", str(spec), "--out", str(link)]) == 2
    assert (
        run(["verify-v1", "--spec", str(spec), "--corpus", str(V1_CORPUS), "--emit-v2", str(spec)])
        == 2
    )


# -- 8: total work -------------------------------------------------------------------------------


def test_the_total_work_bound_is_enforced_across_suites():
    src = b"warrant big\n  op f(a: U8, b: U8, c: U8) -> U8\n  claim c3:\n    forall a: U8, b: U8, c: U8 | c == a. f(a, b, c) == f(a, b, c)\n"
    spec = lower_document(parse_source(src))
    work = Work("slim", cases=100_000)  # the slim chunks (about 983,000 ordinals) exceed it
    with pytest.raises(OracleError) as e:
        SuitePlan(spec, build_program(spec), spec.suites[0], work=work).run("slim")
    assert e.value.code == "limit" and "work" in e.value.message


def test_ir_work_bound_is_enforced():
    work = Work("slim", ir_work=1000)
    with pytest.raises(OracleError):
        plan(WRAPS, "one", work=work).run("full")


def test_work_limits_are_the_spec_values():
    from coding_adventures_warrant_oracle import suites

    assert (suites.MAX_TOTAL_WORK_SLIM, suites.MAX_TOTAL_WORK_FULL) == (32_000_000, 1 << 32)
    assert (suites.MAX_IR_WORK_SLIM, suites.MAX_IR_WORK_FULL) == (64_000_000_000, 8_589_934_592_000)


# -- 9: stream tags --------------------------------------------------------------------------------


def test_tuple_record_and_variant_use_the_registry_tags():
    spec = lower_document(
        parse_source(
            b"warrant s v1\n  record P(x: U8)\n  variant V = Q | R(v: U8)\n  op f(a: Nat) -> Nat\n"
        )
    )
    assert make_encoder(spec, T("P"))((1,))[0] == 0x64
    assert make_encoder(spec, T("()", (T("Bool"), T("Bool"))))((True, True))[0] == 0x64
    assert make_encoder(spec, T("V"))(("Q", ()))[0] == 0x65


# -- cost amendment: worst path, profile, ir_cost ---------------------------------------------------


GF = adopter_bytes("gf256")


def test_default_bound_is_2000_on_the_worst_path_and_gf256_needs_a_profile():
    tree = emit(GF)
    refused = [k for k, v in tree["suites"].items() if v["status"] == "refused"]
    assert len(refused) == 4 and tree["max_ir_cost"] == 2000
    tree = emit(GF, profile=parse_profile(b"max_ir_cost 32768\n"))
    assert tree["summary"]["ok"] == 38 and tree["max_ir_cost"] == 32768
    assert max(r["ir_cost"] for r in tree["suites"].values()) >= 26_210


def test_ir_cost_is_the_sum_of_admit_body_and_expected():
    s = plan(WRAPS, "one").run("full")
    spec = lower_document(parse_source(WRAPS))
    prog = build_program(spec)
    su = spec.suites[0]
    tenv = dict(su.binders)
    want = sum(prog.compile(x, tenv).cost for x in (su.admit, su.body, su.expected))
    assert s.ir_cost == want


def test_profile_parser_is_strict():
    p = parse_profile(
        b"budget 1024\nchunk_size 1024\nslim_stride auto\nmax_ir_cost 4096\nfault multiply x\n"
    )
    assert (p.budget, p.chunk_size, p.max_ir_cost) == (1024, 1024, 4096)
    for bad in (
        b"max_ir_cost 1999\n",
        b"max_ir_cost 65537\n",
        b"max_ir_cost 5000\nmax_ir_cost 5000\n",
        b"chunk_size 1000\n",
        b"bogus 1\n",
        b"budget\n",
        b"budget x\n",
        b"\xff",
        b"budget 1\r\n",
    ):
        with pytest.raises(OracleError):
            parse_profile(bad)
    assert parse_profile(b"").max_ir_cost == 2000


def test_the_cli_has_no_cost_override(capsys):
    with pytest.raises(SystemExit):
        run(["emit-v2", "--spec", str(SPECS / "gf256.warrant"), "--out", "x", "--max-ir-cost", "9"])
    with pytest.raises(SystemExit):
        run(
            [
                "emit-v2",
                "--spec",
                str(SPECS / "gf256.warrant"),
                "--out",
                "x",
                "--cost-model",
                "taken-path",
            ]
        )


def test_analysis_walks_are_not_charged_to_the_slim_case_budget():
    # 2^20 ordinals, a pure guard (so the call count needs a full-product walk) and a
    # budget that fits the one hashed pass but not the hashed pass plus the walk.
    src = (
        b"warrant g\n  op f(a: U8, b: U8, p: Bool, q: Bool, r: Bool, s: Bool) -> U8\n"
        b"  claim c:\n    forall a: U8, b: U8, p: Bool, q: Bool, r: Bool, s: Bool."
        b" if p then true else f(a, b, p, q, r, s) == f(a, b, p, q, r, s)\n"
    )
    work = Work("slim", cases=1_500_000)
    p = plan(src, "c", work=work)
    assert callable(p.calls)
    r = p.run("slim")
    assert r.cases == 1 << 20 and r.expected_calls == 2 * (1 << 19)
    assert work.cases == 1 << 20 and work.walked >= 1 << 20


def test_a_hard_link_to_an_input_is_refused(tmp_path):
    spec = tmp_path / "x.warrant"
    spec.write_bytes(SPEC)
    link = tmp_path / "hard.json"
    try:
        os.link(spec, link)
    except OSError:  # pragma: no cover - filesystem without hard links
        pytest.skip("hard links unavailable")
    assert run(["emit-v2", "--spec", str(spec), "--out", str(link)]) == 2
    assert spec.read_bytes() == SPEC
