"""Slice S10t-1: evaluating ``ir.tuple`` (values, the stream, TFV, domains, call counting).

The references are written in this file and share nothing with the production code: the stream
bytes of tag ``64`` are spelled out, the digests of a tuple suite are rebuilt by brute force, and
the call counts of a ``match`` are worked out by hand from VER04r 4.4 (a ``match`` arm is a guard).
"""

from __future__ import annotations

import hashlib
import itertools
import json
import pathlib

import pytest
from conftest import write_lf

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import patterns as P
from coding_adventures_warrant_oracle import tfv
from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import canonical, derive, value_json
from coding_adventures_warrant_oracle.domains import descriptor_for
from coding_adventures_warrant_oracle.emit import emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import (
    Program,
    admit_check_cost,
    compile_admit_check,
    compile_calls,
    compile_trace,
    fixed_arith_over_subject,
    has_subject,
    may_wrap,
    walk_scoped,
)
from coding_adventures_warrant_oracle.irtree import (
    children,
    free_vars,
    is_node,
    match_arms,
    pattern_binds,
)
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.stream import (
    HEADER_V1,
    HEADER_V2,
    context_prefix,
    in_v1_set,
    make_encoder,
    suite_header,
    u32,
)
from coding_adventures_warrant_oracle.suites import SuitePlan
from coding_adventures_warrant_oracle.verify_v2 import verify_v2

GEN = {"name": "test", "version": "0"}
T = A.Ty


def ty(text: str) -> A.Ty:
    """Parse a type through the real parser (a throwaway op)."""
    doc = parse_source(f"warrant t v1\n  error E = A | B\n  op f() -> {text}\n")
    return lower_document(doc).ops["f"].ret


def le(n: int, width: int = 8) -> bytes:
    return n.to_bytes(width, "little")


SPEC = lower_document(parse_source(b"warrant s v1\n  error E = A | B\n  op f(a: Nat) -> Nat\n"))


def enc(type_text: str, value) -> bytes:
    return make_encoder(SPEC, ty(type_text))(value)


# -- stream tag 64 ---------------------------------------------------------------------------------


def test_a_tuple_value_is_tag_64_a_u32_count_and_its_components():
    assert enc("(U8,Bool)", (1, True)) == b"\x64" + u32(2) + b"\x10" + le(1) + b"\x01"
    assert enc("(Bool,Bool,Bool)", (True, False, True)) == b"\x64" + u32(3) + b"\x01\x00\x01"
    # the spec's worked example is a record; a tuple of two U8 has the same bytes
    assert enc("(U8,U8)", (1, 2)) == bytes.fromhex("64 02000000 10 0100000000000000 10 0200000000000000")


def test_nested_tuples_are_self_delimiting():
    inner = b"\x64" + u32(2) + b"\x01\x00"
    assert enc("((Bool,Bool),Bool)", ((True, False), True)) == b"\x64" + u32(2) + inner + b"\x01"
    # two different nestings of the same leaves serialise differently
    assert enc("((Bool,Bool),Bool)", ((True, False), True)) != enc("(Bool,(Bool,Bool))", (True, (False, True)))


def test_a_tuple_inside_a_result_option_and_bytes_component():
    assert enc("Result<(U8,Nat),E>", ("ok", (7, 3))) == (
        b"\x50" + b"\x64" + u32(2) + b"\x10" + le(7) + b"\x10" + le(3)
    )
    assert enc("Result<(U8,Nat),E>", ("err", "B")) == b"\x51" + u32(1) + b"B"
    assert enc("(Bytes<2>,Nat)", (b"\x01\x02", 4)) == (
        b"\x64" + u32(2) + b"\x20" + u32(2) + b"\x01\x02" + b"\x10" + le(4)
    )


def test_a_suite_with_a_tuple_type_has_the_v2_header_and_the_context_names_the_type():
    assert not in_v1_set(ty("(U8,U8)")) and not in_v1_set(ty("Result<(U8,Nat),E>"))
    assert suite_header([ty("(U8,Bool)")], T("Bool")) == HEADER_V2
    assert suite_header([T("U8")], ty("(U8,U8)")) == HEADER_V2
    assert suite_header([T("U8")], T("Bool")) == HEADER_V1
    prefix = context_prefix("s", [ty("(U8,Bool)")], ty("Result<(U8,Nat),E>"), HEADER_V2)
    assert b"\x09\x00\x00\x00(U8,Bool)" in prefix
    assert b"\x12\x00\x00\x00Result<(U8,Nat),E>" in prefix


# -- values: JSON and TFV ---------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "type_text,value,as_json,as_tfv",
    [
        ("(U8,Bool)", (1, True), ["1", True], "L2 I1 T"),
        ("(Bool,Bool,Bool)", (True, False, True), [True, False, True], "L3 T F T"),
        ("((Bool,Bool),Bool)", ((True, False), True), [[True, False], True], "L2 L2 T F T"),
        ("(Bytes,Nat)", (b"\x0a\xff", 9), ["0aff", "9"], "L2 X0aff I9"),
        ("(I8,I8)", (-1, 5), ["-1", "5"], "L2 I-1 I5"),
        ("Result<(U8,Nat),E>", ("ok", (1, 2)), {"ok": ["1", "2"]}, "M1 S6f6b L2 I1 I2"),
        ("Result<(U8,Nat),E>", ("err", "A"), {"err": "A"}, "M1 S657272 S41"),
        ("(Result<Bool,E>,Bool)", (("ok", True), False), [{"ok": True}, False], "L2 M1 S6f6b T F"),
    ],
)
def test_json_and_tfv_forms_of_a_tuple_value(type_text, value, as_json, as_tfv):
    assert value_json(value, ty(type_text)) == as_json
    assert tfv.value(value, ty(type_text)) == as_tfv


def test_a_tuple_record_line():
    line = tfv.record("x", "f", [("t", ty("(U8,Bool)"))], [(3, False)], None, (4, True), ty("(U8,Bool)"))
    assert line == "x R3 Kop S66 Kt L2 I3 F Kexpect L2 I4 T"


# -- domains: the descriptor `tuple` ---------------------------------------------------------------------


def test_the_tuple_descriptor_is_the_product_with_the_last_component_fastest():
    spec = SPEC
    d = descriptor_for(spec, ty("(Bool,U8)"))
    assert d.to_json() == {
        "d": "tuple",
        "of": [{"d": "bool", "size": 2}, {"d": "u8_all", "size": 256}],
        "size": 512,
    }
    assert [d.nth(i) for i in range(3)] == [(False, 0), (False, 1), (False, 2)]
    assert d.nth(255) == (False, 255) and d.nth(256) == (True, 0) and d.nth(511) == (True, 255)
    nested = descriptor_for(spec, ty("((Bool,Bool),Bool)"))
    assert nested.size() == 8
    assert [nested.nth(i) for i in range(4)] == [
        ((False, False), False),
        ((False, False), True),
        ((False, True), False),
        ((False, True), True),
    ]
    # eight components, each a Bool: 256 values
    assert descriptor_for(spec, ty("(" + ",".join(["Bool"] * 8) + ")")).size() == 256


def test_a_result_binder_of_a_tuple_enumerates_ok_values_then_each_error_case():
    d = descriptor_for(SPEC, ty("Result<(Bool,Bool),E>"))
    assert d.size() == 4 + 2
    assert [d.nth(i) for i in range(6)] == [
        ("ok", (False, False)),
        ("ok", (False, True)),
        ("ok", (True, False)),
        ("ok", (True, True)),
        ("err", "A"),
        ("err", "B"),
    ]


# -- a tuple suite, digests rebuilt by brute force -------------------------------------------------------------

PAIRS = b"""warrant pairs v1
  error E = BAD | WORSE
  op swap_op(t: (Bool,U8)) -> (U8,Bool)
  model swap_model(t: (Bool,U8)) -> (U8,Bool) = (t.1, t.0)
  op case_op(r: Result<(Bool,Bool),E>) -> Nat
  model case_model(r: Result<(Bool,Bool),E>) -> Nat = match r with Ok((a, _)) -> (if a then 1 else 2) | Err(E.BAD) -> 3 | Err(_) -> 4
  claim swap_claim:
    forall t: (Bool,U8). swap_op(t) == swap_model(t)
  claim case_claim:
    forall r: Result<(Bool,Bool),E>. case_op(r) == case_model(r)
"""


def plan_for(source: bytes, claim: str, **kw):
    spec = lower_document(parse_source(source))
    prog = build_program(spec)
    suite = next(s for s in spec.suites if s.claim == claim)
    return SuitePlan(spec, prog, suite, **kw), spec


def test_the_digests_of_a_tuple_suite_equal_a_brute_force_stream():
    plan, spec = plan_for(PAIRS, "swap_claim")
    assert plan.header == HEADER_V2 and plan.product == 512
    prefix = context_prefix(plan.suite.id, [ty("(Bool,U8)")], ty("(U8,Bool)"), HEADER_V2)
    assert plan.prefix == prefix
    h_in, h_ex = hashlib.sha256(prefix + u32(0)), hashlib.sha256(prefix + u32(0))
    for flag, byte in itertools.product([False, True], range(256)):
        h_in.update(b"\x64" + u32(2) + (b"\x01" if flag else b"\x00") + b"\x10" + le(byte))
        h_ex.update(b"\x64" + u32(2) + b"\x10" + le(byte) + (b"\x01" if flag else b"\x00"))
    chunk = plan.run_chunk(0)
    assert (chunk.cases, chunk.calls) == (512, 512)
    assert chunk.inputs == h_in.digest() and chunk.expected == h_ex.digest()


def test_a_match_in_the_expected_side_is_evaluated_per_case():
    plan, spec = plan_for(PAIRS, "case_claim")
    assert plan.product == 6
    prefix = plan.prefix
    h_in, h_ex = hashlib.sha256(prefix + u32(0)), hashlib.sha256(prefix + u32(0))
    values = [("ok", (a, b)) for a in (False, True) for b in (False, True)] + [("err", "BAD"), ("err", "WORSE")]
    for v in values:
        payload = b"\x50" + b"\x64" + u32(2) + b"".join(b"\x01" if x else b"\x00" for x in v[1]) if v[0] == "ok" else b"\x51" + u32(len(v[1])) + v[1].encode()
        h_in.update(payload)
        want = (1 if v[1][0] else 2) if v[0] == "ok" else (3 if v[1] == "BAD" else 4)
        h_ex.update(b"\x10" + le(want))
    chunk = plan.run_chunk(0)
    assert chunk.inputs == h_in.digest() and chunk.expected == h_ex.digest()
    # worst-path costs (section 4.2 rule 6), by hand.  The model body is the match: 1 + cost(r) +
    # the worst arm = 1 + 1 + (1 + 1 + max(1, 1) for the `if`) = 5, so the model call costs
    # 1 + 1 + 5 = 7 and the subject call 1 + 1 = 2.  admit 1 + body (eq: 1 + 2 + 7 = 10) + expected 7.
    assert plan.ir_cost == 1 + 10 + 7


def test_the_derived_corpus_of_a_tuple_document_has_computed_requires_and_levels():
    out = derive(PAIRS, b"", spec_ref="p.warrant", generator=GEN)
    suites = {s["claim"]: s for s in out.corpus["suites"]}
    assert suites["swap_claim"]["requires"] == ["ir.core", "ir.tuple"]
    assert suites["swap_claim"]["stream"] == "warrant-stream-v2"
    assert suites["swap_claim"]["binders"][0] == [
        "t",
        {
            "type": "(Bool,U8)",
            "domain": {
                "d": "tuple",
                "of": [{"d": "bool", "size": 2}, {"d": "u8_all", "size": 256}],
                "size": 512,
            },
        },
    ]
    assert suites["swap_claim"]["result_type"] == "(U8,Bool)"
    assert out.corpus["levels"] == ["ir.core", "ir.tuple"]
    assert out.corpus["ops"]["swap_op"]["requires"] == ["ir.core", "ir.tuple"]
    assert "v1_inputs_sha256" not in suites["swap_claim"]  # a v2-header suite has no v1 digest


def test_emit_reports_a_tuple_suite_live_and_names_the_implemented_levels():
    tree = emit(PAIRS)
    assert tree["levels_implemented"] == ["ir.core", "ir.data", "ir.fixed", "ir.tuple"]
    assert {r["status"] for r in tree["suites"].values()} == {"ok"}
    assert tree["suites"]["swap_claim"]["stream_header"] == "warrant-stream-v2"


# -- verify-v2 on a derived tuple corpus -----------------------------------------------------------------------------


def write_corpus(tmp_path, source: bytes, profile: bytes) -> pathlib.Path:
    out = derive(source, profile, spec_ref="p.warrant", generator=GEN)
    root = tmp_path / "corpus"
    (root / "chunks").mkdir(parents=True)
    (root / "corpus.json").write_bytes(canonical(out.corpus))
    (root / "cases.tfv").write_bytes(out.tfv)
    for name, text in out.chunk_files.items():
        (root / "chunks" / name).write_bytes(text)
    return root


def test_verify_v2_accepts_a_derived_tuple_corpus_and_rejects_a_tampered_one(tmp_path):
    source = PAIRS + b"  example e:\n    swap_op((true, 3u8)) == (3u8, true)\n"
    root = write_corpus(tmp_path, source, b"")
    warrant, profile = tmp_path / "p.warrant", tmp_path / "corpus.profile"
    warrant.write_bytes(source)
    write_lf(profile, "")
    report, _ = verify_v2(root, warrant, profile, repo_root=tmp_path)
    assert report.ok, report.mismatches
    corpus = json.loads((root / "corpus.json").read_bytes())
    suite = next(s for s in corpus["suites"] if s["claim"] == "swap_claim")
    suite["requires"] = ["ir.core"]  # a lane that lacks ir.tuple would then run it: refused
    (root / "corpus.json").write_bytes(canonical(corpus))
    report, _ = verify_v2(root, warrant, profile, repo_root=tmp_path)
    assert not report.ok


def test_the_cli_emits_a_tuple_document(tmp_path, capsys):
    doc = tmp_path / "p.warrant"
    doc.write_bytes(PAIRS)
    out = tmp_path / "o.json"
    assert run(["emit-v2", "--spec", str(doc), "--out", str(out), "--depth", "full"]) == 0
    tree = json.loads(out.read_text())
    assert tree["suites"]["swap_claim"]["stream_header"] == "warrant-stream-v2"
    assert tree["levels_implemented"] == ["ir.core", "ir.data", "ir.fixed", "ir.tuple"]
    del capsys


# -- call counting: a `match` arm is a guard (VER04r 4.4) -------------------------------------------------------

COUNT = """warrant counts v1
  error E = A | B
  op h(a: Bool) -> Result<Bool,E>
  op inc(a: U8) -> U8
  op small(a: U8 | a < 100u8) -> U8
  op both(a: U8, b: U8) -> U8
"""


def calls_of(claim_body: str, *, binders: str = "r: Result<U8,E>"):
    spec = lower_document(parse_source(COUNT + f"  claim c:\n    forall {binders}. {claim_body}\n"))
    prog = build_program(spec)
    suite = spec.suites[0]
    tenv = dict(suite.binders)
    return compile_calls(prog, suite.body, tenv)


def test_a_subject_call_as_the_scrutinee_counts_once_per_case():
    fn = calls_of("match h(a) with Ok(x) -> x | Err(_) -> true", binders="a: Bool")
    assert fn == 1


def test_a_subject_call_in_an_arm_counts_only_when_the_arm_is_taken():
    fn = calls_of("match r with Ok(x) -> inc(x) == x + 1u8 | Err(_) -> true")
    assert callable(fn)
    assert fn({"r": ("ok", 3)}) == 1 and fn({"r": ("err", "A")}) == 0


def test_the_arm_taken_is_the_first_pattern_that_matches_and_the_binders_are_visible_to_guards():
    fn = calls_of(
        "match r with Ok(x) -> (if x > 5u8 then inc(x) == x else both(x, x) == x) | Err(E.A) -> inc(0u8) == 1u8 | Err(_) -> true"
    )
    assert fn({"r": ("ok", 9)}) == 1 and fn({"r": ("ok", 2)}) == 1  # one call on either branch
    assert fn({"r": ("err", "A")}) == 1 and fn({"r": ("err", "B")}) == 0
    nested = calls_of(
        "match r with Ok(x) -> (match r with Ok(y) -> inc(y) == y | Err(_) -> true) | Err(_) -> true"
    )
    assert nested({"r": ("ok", 1)}) == 1 and nested({"r": ("err", "A")}) == 0


def test_a_subject_call_in_an_arm_under_an_impure_scrutinee_is_w0701():
    with pytest.raises(OracleError) as err:
        calls_of("match h(true) with Ok(_) -> inc(1u8) == 1u8 | Err(_) -> true", binders="a: Bool")
    assert err.value.code == "generation" and "W0701" in err.value.message


def test_an_impure_scrutinee_with_pure_arms_is_fine_and_counts_the_scrutinee_only():
    fn = calls_of("match h(a) with Ok(x) -> x | Err(_) -> a", binders="a: Bool")
    assert fn == 1
    # a subject call in an argument of the scrutinee
    fn = calls_of("match h(inc(a) == 0u8) with Ok(x) -> x | Err(_) -> true", binders="a: U8")
    assert fn == 2


def test_the_trace_of_wraps_follows_the_taken_arm():
    spec = lower_document(
        parse_source(
            COUNT
            + "  claim c:\n    forall r: Result<U8,E>. match r with Ok(x) -> inc(x + 1u8) == 0u8 | Err(_) -> true\n"
        )
    )
    prog = build_program(spec)
    suite = spec.suites[0]
    trace = compile_trace(prog, suite.body, dict(suite.binders))
    stats = prog.stats
    stats.counting = True
    before = stats.wraps
    trace({"r": ("ok", 255)})  # x + 1u8 wraps
    trace({"r": ("ok", 3)})  # does not
    trace({"r": ("err", "A")})  # no arm with a node on the trace
    assert stats.wraps - before == 1


def test_wraps_and_the_pure_trace_over_a_whole_suite_with_a_match_arm():
    plan, _ = plan_for(
        COUNT + "  claim c:\n    forall r: Result<U8,E>. match r with Ok(x) -> inc(x + 1u8) == 0u8 | Err(_) -> true\n",
        "c",
    )
    result = plan.run("full")
    assert result.wraps == 1 and result.expected_calls == 256 and result.cases == 258


def test_a_match_in_a_claim_admit_and_free_variables():
    plan, _ = plan_for(
        COUNT + "  claim c:\n    forall r: Result<U8,E> | match r with Ok(x) -> x < 3u8 | Err(_) -> false. match r with Ok(x) -> inc(x) == x | Err(_) -> true\n",
        "c",
    )
    assert plan.admit_last == 0  # admit reads r only: the pattern binder x is not a binder
    assert plan.total_cases() == 3


def test_the_admit_of_a_callee_is_checked_inside_a_match_arm():
    ok = COUNT + (
        "  claim c:\n    forall r: Result<U8,E> | match r with Ok(x) -> x < 100u8 | Err(_) -> true."
        " match r with Ok(x) -> small(x) == x | Err(_) -> true\n"
    )
    plan, _ = plan_for(ok, "c")
    plan.run("full")  # every admitted case calls small inside its admit
    bad = COUNT + (
        "  claim c:\n    forall r: Result<U8,E>. match r with Ok(x) -> small(x) == x | Err(_) -> true\n"
    )
    plan, _ = plan_for(bad, "c")
    with pytest.raises(OracleError) as err:
        plan.run("full")
    assert err.value.code == "generation" and "outside the op's admit" in err.value.message
    assert "admitted case 100" in err.value.message  # the first offender: Ok(100) is case 100


def test_the_admit_check_of_an_impure_scrutinee_with_subject_arms_is_w0701():
    spec = lower_document(
        parse_source(
            COUNT + "  claim c:\n    forall a: Bool. match h(a) with Ok(_) -> small(1u8) == 1u8 | Err(_) -> true\n"
        )
    )
    prog = build_program(spec)
    suite = spec.suites[0]
    with pytest.raises(OracleError) as err:
        compile_admit_check(prog, suite.body, dict(suite.binders))
    assert "W0701" in err.value.message
    with pytest.raises(OracleError):
        compile_trace(prog, suite.body, dict(suite.binders))


def test_the_check_cost_walks_into_arms_with_their_binders_in_scope():
    spec = lower_document(
        parse_source(
            COUNT + "  claim c:\n    forall r: Result<U8,E>. match r with Ok(x) -> small(x + 1u8) == x | Err(_) -> true\n"
        )
    )
    prog = build_program(spec)
    suite = spec.suites[0]
    cost = admit_check_cost(prog, suite.body, dict(suite.binders))
    assert cost > 0  # the cost of `a < 100u8` plus the argument `x + 1u8`, typed with x: U8


# -- v0 rule 3, may_wrap, has_subject and the walkers --------------------------------------------------------


def test_fixed_arithmetic_above_a_subject_call_is_found_inside_a_match_arm():
    """VER04r 3.3 rule 3 (a v0 document): a fixed-width ``+`` above a subject call is refused
    statically, and the walk must see it inside a ``match`` arm (with the binder typed) and inside
    the body of a model called over a subject."""
    from coding_adventures_warrant_oracle.ir_eval import ModelDef
    from coding_adventures_warrant_oracle.lower import OpSig

    u8 = ["lit", {"type": "U8", "value": "0"}]
    r_ty = A.Ty("Result", (T("U8"), T("E")))
    f = OpSig("f", (), [("a", T("U8"))], T("U8"), False)
    g = OpSig("g", (), [], r_ty, False)
    errors = {"E": ("A", "B")}
    prog = Program({}, {"f": f, "g": g}, errors=errors)
    plus = lambda e: ["add", e, ["lit", {"type": "U8", "value": "1"}]]  # noqa: E731
    call = ["subject_op", "f", [["var", "x"]]]
    over = ["match", ["var", "r"], [[["ok", ["bind", "x"]], plus(call)], [["err_any"], u8]]]
    assert fixed_arith_over_subject(prog, over, {"r": r_ty}) is True
    safe = ["match", ["var", "r"], [[["ok", ["bind", "x"]], ["var", "x"]], [["err_any"], call]]]
    assert fixed_arith_over_subject(prog, safe, {"r": r_ty}) is False
    body = ["match", ["var", "r"], [[["ok", ["bind", "x"]], plus(["var", "x"])], [["err_any"], u8]]]
    prog = Program({"m": ModelDef("m", [("r", r_ty)], T("U8"), body)}, {"f": f, "g": g}, errors=errors)
    assert fixed_arith_over_subject(prog, ["model_call", "m", [["subject_op", "g", []]]], {}) is True


def test_has_subject_and_may_wrap_look_inside_tuples_and_match_arms():
    prog = Program({}, {})
    call = ["subject_op", "f", []]
    assert has_subject(["tuple", [["lit", True], call]])
    assert has_subject(["project", ["tuple", [call, call]], 0])
    assert has_subject(["match", ["var", "r"], [[["wild"], call]]])
    assert not has_subject(["match", ["var", "r"], [[["ok", ["bind", "x"]], ["var", "x"]], [["wild"], ["lit", True]]]])
    one = ["lit", {"type": "U8", "value": "1"}]
    assert may_wrap(prog, ["tuple", [["lit", True], ["add", one, one]]])
    assert may_wrap(prog, ["match", ["var", "r"], [[["wild"], ["add", one, one]]]])
    assert not may_wrap(prog, ["tuple", [one, one]])


def test_walk_scoped_yields_every_node_with_the_scope_it_is_typed_in():
    prog = Program({}, {}, errors={"E": ("A",)})
    r_ty = A.Ty("Result", (A.Ty("()", (T("Bool"), T("Nat"))), T("E")))
    ir = ["match", ["var", "r"], [[["ok", ["tuple", [["bind", "a"], ["bind", "n"]]]], ["tuple", [["var", "n"], ["var", "a"]]]], [["err_any"], ["tuple", [["lit", True], ["lit", {"type": "Nat", "value": "0"}]]]]]]
    seen = {}
    for node, types in walk_scoped(prog, ir, {"r": r_ty}):
        if node[0] == "var" and node[1] in ("a", "n"):
            seen[node[1]] = types[node[1]]
    assert seen == {"a": T("Bool"), "n": T("Nat")}
    # no pattern is ever yielded as an expression
    tags = [node[0] for node, _ in walk_scoped(prog, ir, {"r": r_ty})]
    assert "bind" not in tags and "ok" not in tags and "err_any" not in tags


# -- irtree --------------------------------------------------------------------------------------------------------


def test_children_of_every_node_shape():
    a, b = ["var", "a"], ["lit", True]
    assert children(a) == [] and children(["err", "X"]) == [] and children(["max_value", "U8"]) == []
    assert children(["add", a, b]) == [a, b]
    assert children(["widen", "U32", a]) == [a]  # the static type argument is not a child
    assert children(["subject_op", "f", [a, b], [3]]) == [a, b]
    assert children(["model_call", "m", [a]]) == [a]
    assert children(["tuple", [a, b]]) == [a, b]
    assert children(["project", a, 1]) == [a]
    assert children(["match", a, [[["ok", ["bind", "x"]], b], [["wild"], a]]]) == [a, b, a]
    with pytest.raises(OracleError):
        children(["match", a])
    with pytest.raises(OracleError):
        match_arms(["match", a, []])
    with pytest.raises(OracleError):
        match_arms(["match", a, [["wild"]]])
    assert is_node(["x"]) and not is_node([]) and not is_node([["x"]]) and not is_node("x")


def test_free_vars_and_pattern_binds():
    ir = [
        "match",
        ["var", "r"],
        [
            [["ok", ["tuple", [["bind", "a"], ["wild"], ["bind", "b"]]]], ["and", ["var", "a"], ["var", "c"]]],
            [["err", "A"], ["var", "b"]],
            [["bind", "whole"], ["var", "whole"]],
        ],
    ]
    assert free_vars(ir) == {"r", "c"}
    assert pattern_binds(["ok", ["tuple", [["bind", "a"], ["wild"], ["bind", "b"]]]]) == ["a", "b"]
    assert pattern_binds(["err_any"]) == [] and pattern_binds(["bind", "z"]) == ["z"]
    assert pattern_binds("junk") == [] and pattern_binds(["bind", 3]) == []
    assert free_vars(["tuple", [["var", "a"], ["var", "b"]]]) == {"a", "b"}


# -- patterns: matching and the usefulness check -----------------------------------------------------------


def test_compiled_patterns_match_values_and_bind():
    def run(pattern, value):
        binds: dict = {}
        return P.compile_pattern(pattern)(value, binds), binds

    assert run(["wild"], 1) == (True, {})
    assert run(["bind", "x"], 7) == (True, {"x": 7})
    assert run(["ok", ["bind", "x"]], ("ok", 3)) == (True, {"x": 3})
    assert run(["ok", ["bind", "x"]], ("err", "A"))[0] is False
    assert run(["err_any"], ("err", "Z"))[0] is True and run(["err_any"], ("ok", 1))[0] is False
    assert run(["err", "A"], ("err", "A"))[0] is True and run(["err", "A"], ("err", "B"))[0] is False
    assert run(["err", "A"], ("ok", 1))[0] is False
    assert run(["tuple", [["bind", "a"], ["bind", "b"]]], (1, 2)) == (True, {"a": 1, "b": 2})
    nested = ["ok", ["tuple", [["tuple", [["bind", "a"], ["wild"]]], ["bind", "c"]]]]
    assert run(nested, ("ok", ((1, 2), 3))) == (True, {"a": 1, "c": 3})


def test_the_usefulness_check_directly():
    errors = {"E": ("A", "B")}
    rt = A.Ty("Result", (A.Ty("()", (T("Bool"), T("Bool"))), T("E")))
    P.check_arms([["ok", ["wild"]], ["err_any"]], rt, errors)
    P.check_arms([["ok", ["tuple", [["wild"], ["wild"]]]], ["err", "A"], ["err", "B"]], rt, errors)
    with pytest.raises(OracleError):  # an ok(tuple) after an ok(wild)
        P.check_arms([["ok", ["wild"]], ["ok", ["tuple", [["wild"], ["wild"]]]], ["err_any"]], rt, errors)
    with pytest.raises(OracleError):
        P.check_arms([["ok", ["wild"]], ["err", "A"]], rt, errors)
    with pytest.raises(OracleError) as err:  # no arms at all
        P.check_arms([], rt, errors)
    assert err.value.code == "limit"
    with pytest.raises(OracleError) as err:
        P.check_arms([["wild"]] * 257, rt, errors)
    assert err.value.code == "limit"


def test_the_usefulness_check_has_a_work_budget(monkeypatch):
    monkeypatch.setattr(P, "WORK_BUDGET", 3)
    rt = A.Ty("Result", (A.Ty("Bool"), T("E")))
    with pytest.raises(OracleError) as err:
        P.check_arms([["ok", ["wild"]], ["err", "A"], ["err", "B"], ["err", "C"]], rt, {"E": ("A", "B", "C")})
    assert err.value.code == "limit"


def test_type_pattern_depth_limit_and_tuple_arity():
    deep: object = ["wild"]
    for _ in range(9):
        deep = ["ok", deep]
    ty_deep = A.Ty("Bool")
    for _ in range(9):
        ty_deep = A.Ty("Result", (ty_deep, T("E")))
    with pytest.raises(OracleError) as err:
        P.type_pattern(deep, ty_deep, {}, [])
    assert err.value.code == "limit"
    pair = A.Ty("()", (T("Bool"), T("Bool")))
    with pytest.raises(OracleError):
        P.type_pattern(["tuple", [["wild"], ["wild"], ["wild"]]], pair, {}, [])
    with pytest.raises(OracleError):
        P.type_pattern(["tuple", [["wild"]] * 9], pair, {}, [])
