"""``omit_example`` (CV2-S10-1; VER04r 8.4, owner decisions [D22] and [D34], refinement R4).

An ``omit_example <id>`` profile line removes one example from ``corpus.json`` when the tool can
decide that it is inexpressible, and records the *computed* reason in the conditional key
``omitted_examples``.  The reason is ``not-op-eq-pure`` (shape, tested first) or
``value-over-limit`` (the normative five-step order of 8.4).  The committed corpus under test is
``fixtures/omit-example-v2``.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import shutil

import pytest
from conftest import write_lf

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v2 import REASONS, canonical, derive
from coding_adventures_warrant_oracle.errors import OracleError, SizeError
from coding_adventures_warrant_oracle.profile import parse_profile
from coding_adventures_warrant_oracle.verify_v2 import verify_v2

HERE = pathlib.Path(__file__).resolve().parent / "fixtures" / "omit-example-v2"
WARRANT = HERE / "omit.warrant"
PROFILE = HERE / "corpus.profile"
CORPUS = HERE / "corpus"
GEN = {"name": "test", "version": "0"}
CACHE: dict = {}

BIG = "warrant/omitx/example/echo_big_value"
SIZE = "warrant/omitx/example/echo_size_error"
TWO = "warrant/omitx/example/echo_two_sided"


# -- a small document builder ------------------------------------------------------------------


def nest(n: int, inner: str) -> str:
    for _ in range(n):
        inner = f"x2({inner})"
    return inner


GROW13 = nest(13, 'b"aa"')  # 2 * 2^13 = 16,384 bytes: over 4,096, under MAX_IR_VALUE
GROW17 = nest(17, 'b"aa"')  # 2 * 2^17 = 262,144 bytes: past MAX_IR_VALUE (a SizeError)

OPS = [
    ("echo", "op echo(a: Bytes) -> Bytes", "ab"),
    ("lo", "op lo(a: U8 | a < 10u8, b: Bytes) -> Bytes", "ab"),
    ("wr", "op wr(a: U8, b: Bytes) -> Bytes", "ab"),
    ("nn", "op nn(a: Bytes, b: Bytes) -> Bytes", "ab"),
    ("nb", "op nb(b: Bytes, a: Bytes) -> Bytes", "ab"),
    ("res", "op res(a: Bytes) -> Result<Bytes, E>", "a"),
]


def doc(examples: str, version: str = "v1") -> bytes:
    s = "warrant omitx" + ("" if version == "v0" else " v1") + "\n  error E = BAD\n"
    s += "  model x2(a: Bytes) -> Bytes = concat(a, a)\n"
    s += "  model same(a: Bytes, b: Bytes) -> Bool = a == b\n"
    s += "  model sq(a: Int) -> Int = a * a\n"
    # `Int` is model-internal (VER04r 2.1): an op signature may not name it, so the exact-window overflow is
    # raised inside a model whose result is Bytes.
    s += "  model ov(a: Bytes) -> Bytes = if sq(sq(sq(sq(10000000000)))) > 0 then a else a\n"
    for _, line, _ in OPS:
        s += f"  {line}\n"
    s += "  assume platform:\n    none\n  assume environment:\n    none\n"
    for op, _, params in OPS:
        s += f"  labels {op}:\n" + "".join(f"    {p} secret\n" for p in params)
    for op, _, params in OPS:
        for p in params:
            s += f"  input {op}.{p}:\n    hostile\n    ensures no_panic, terminates\n"
    for op, _, _ in OPS:
        s += f"  bound {op}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n"
    s += "  claim c:\n    forall a: Bytes<1>. echo(a) == a\n"
    return (s + examples).encode()


def ex(name: str, body: str) -> str:
    return f"  example {name}:\n    {body}\n"


def gen(examples: str, profile: str, version: str = "v1"):
    return derive(
        doc(examples, version), profile.encode(), spec_ref="o.warrant", generator=GEN
    )


def omit(name: str) -> str:
    return f"omit_example warrant/omitx/example/{name}\n"


def reasons(derived) -> dict:
    return {r["id"].rsplit("/", 1)[1]: r["reason"] for r in derived.corpus.get("omitted_examples", [])}


def refused(examples: str, names: str, code: str, version: str = "v1") -> str:
    profile = "".join(omit(n) for n in names.split())
    with pytest.raises(OracleError) as err:
        gen(examples, profile, version)
    assert err.value.code == code, err.value
    return err.value.message


# -- the profile reader -------------------------------------------------------------------------


def test_omit_example_lines_parse_in_order():
    p = parse_profile(b"omit_example warrant/m/example/b\nomit_example warrant/m/example/a\n")
    assert p.omit_examples == ["warrant/m/example/b", "warrant/m/example/a"]
    assert parse_profile(b"").omit_examples == []
    assert parse_profile(b"fault a b\n").omit_examples == []


@pytest.mark.parametrize(
    "text",
    [
        b"omit_example\n",
        b"omit_example a b\n",
        b"omit_example  a\n",  # an empty token
        b"omit_example a \n",
        b"omit_example\xc3\xa9\n",
        b"omit_examples a\n",
        b"omit_example\ta\n",
    ],
)
def test_a_malformed_omit_example_line_is_a_profile_error(text):
    with pytest.raises(OracleError) as err:
        parse_profile(text)
    assert err.value.code == "corpus"


def test_a_repeated_omit_example_line_is_a_profile_error():
    with pytest.raises(OracleError) as err:
        parse_profile(b"omit_example a\nomit_example b\nomit_example a\n")
    assert err.value.code == "corpus" and "repeated" in err.value.message


@pytest.mark.parametrize("line", [" ", "  ", "\t", "omit_example a\n \n"])
def test_a_whitespace_only_line_is_refused_beside_omit_example(line):
    with pytest.raises(OracleError) as err:
        parse_profile(f"omit_example a\n{line}\n".encode())
    assert err.value.code == "corpus"


def test_an_unknown_id_is_a_profile_error():
    msg = refused(ex("e", 'echo(b"aa") == b"aa"'), "nope", "corpus")
    assert "names no example" in msg
    # the id of another module is not an example of this one either
    with pytest.raises(OracleError) as err:
        gen(ex("e", 'echo(b"aa") == b"aa"'), "omit_example warrant/other/example/e\n")
    assert err.value.code == "corpus"
    # an id is the full id, never the bare name
    with pytest.raises(OracleError):
        gen(ex("e", 'echo(b"aa") != echo(b"bb")'), "omit_example e\n")


def test_an_emittable_example_is_a_stale_profile_and_the_message_names_only_the_id():
    msg = refused(ex("small", 'echo(b"ab") == b"ab"'), "small", "corpus")
    assert "emittable" in msg and "warrant/omitx/example/small" in msg
    assert "6162" not in msg and "b\"ab\"" not in msg  # an id, never a value


def test_a_document_with_zero_examples_left_is_accepted():
    out = gen(ex("t", "echo(b\"aa\") != echo(b\"bb\")"), omit("t"))
    assert out.corpus["examples"] == [] and out.tfv == b"tfv 1\n"
    assert reasons(out) == {"t": "not-op-eq-pure"}


# -- the two reasons -----------------------------------------------------------------------------


def test_not_op_eq_pure_is_decided_from_the_shape():
    shapes = {
        "ne": 'echo(b"aa") != echo(b"bb")',
        "two": 'echo(b"aa") == echo(b"aa")',  # a subject on both sides
        "pure": 'b"aa" == b"aa"',  # no subject at all
        "nested": 'x2(echo(b"aa")) == b"aaaa"',  # the subject is not the top of its side
        "inner": 'echo(echo(b"aa")) == b"aa"',  # a subject call inside an argument
        "inner_right": 'b"aa" == echo(echo(b"aa"))',
        "inner_two": 'lo(1u8, echo(b"aa")) == b"aa"',
        "lt": 'x2(b"aa") != echo(b"aa")',  # not an equality
        "boolean": 'same(echo(b"aa"), b"aa")',  # a Bool call with a subject inside
    }
    examples = "".join(ex(n, b) for n, b in shapes.items())
    out = gen(examples, "".join(omit(n) for n in shapes))
    assert reasons(out) == dict.fromkeys(shapes, "not-op-eq-pure")


def test_a_shape_that_is_not_op_eq_pure_without_the_line_is_the_generators_w0701():
    for body in ('echo(echo(b"aa")) == b"aa"', 'echo(b"aa") != echo(b"bb")', 'echo(b"aa") == echo(b"aa")'):
        msg = refused(ex("s", body), "", "generation")
        assert msg.startswith("W0701") and "one subject call" in msg


def test_value_over_limit_by_a_completed_evaluation_with_a_final_value_over_4096_bytes():
    out = gen(
        ex("argbig", f"echo({GROW13}) == b\"aa\"") + ex("expbig", f"echo(b\"aa\") == {GROW13}"),
        omit("argbig") + omit("expbig"),
    )
    assert reasons(out) == {"argbig": "value-over-limit", "expbig": "value-over-limit"}


def test_the_limit_is_4096_bytes_inclusive():
    at, over = "61" * 4096, "61" * 4097
    refused(ex("at", f'echo(b"{at}") == b"{at}"'), "at", "corpus")  # emittable: stale
    out = gen(ex("over", f'echo(b"{over}") == b"aa"'), omit("over"))
    assert reasons(out) == {"over": "value-over-limit"}


def test_a_bytes_value_under_ok_counts_toward_the_limit():
    out = gen(ex("r", f"res(b\"aa\") == Ok({GROW13})"), omit("r"))
    assert reasons(out) == {"r": "value-over-limit"}
    small = ex("r", 'res(b"aa") == Ok(b"aa")')
    refused(small, "r", "corpus")
    refused(ex("r", "res(b\"aa\") == Err(E.BAD)"), "r", "corpus")


def test_both_reasons_apply_so_the_shape_wins():
    body = f"echo({GROW17}) != echo({GROW13})"  # a size error *and* a shape that is not op == pure
    out = gen(ex("both", body), omit("both"))
    assert reasons(out) == {"both": "not-op-eq-pure"}


def test_an_omitted_example_is_not_admit_checked_wrap_checked_or_encoded():
    # in a v0 document: outside the callee admit AND a U8 wrap, but the shape excuses both
    examples = ex("e", 'lo(200u8, b"aa") != wr(255u8 + 2u8, b"aa")')
    out = gen(examples, omit("e"), version="v0")
    assert reasons(out) == {"e": "not-op-eq-pure"}
    assert out.corpus["examples"] == [] and out.tfv == b"tfv 1\n"


# -- the normative order of the value-over-limit decision (8.4, R4) ----------------------------


OVER = 'ov(b"aa")'  # evaluates 10^160: past the 128-bit exact window, not a size error


def test_step_2_an_error_that_is_not_a_size_error_refuses():
    with pytest.raises(OracleError) as err:
        gen(ex("e", f'nn({OVER}, b"aa") == b"aa"'), omit("e"))
    assert not isinstance(err.value, SizeError) and err.value.code == "ir-overflow"
    assert "128-bit" in err.value.message


def test_step_3_a_size_error_after_a_non_size_error_refuses():
    # argument 1 fails with the window overflow, argument 2 would be a size error
    with pytest.raises(OracleError) as err:
        gen(ex("e", f'nn({OVER}, {GROW17}) == b"aa"'), omit("e"))
    assert not isinstance(err.value, SizeError) and "128-bit" in err.value.message
    # the pure side is evaluated after the arguments, so an argument's error comes first
    with pytest.raises(OracleError) as err:
        gen(ex("e", f'nn({OVER}, b"aa") == {GROW17}'), omit("e"))
    assert "128-bit" in err.value.message


def test_step_3_a_size_error_that_is_the_first_error_qualifies():
    out = gen(ex("e", f'echo({GROW17}) == b"aa"'), omit("e"))
    assert reasons(out) == {"e": "value-over-limit"}
    # on the pure side, after arguments that evaluated fine
    out = gen(ex("p", f'echo(b"aa") == {GROW17}'), omit("p"))
    assert reasons(out) == {"p": "value-over-limit"}
    # in argument 2, after argument 1 evaluated fine
    out = gen(ex("q", f'nn(b"aa", {GROW17}) == b"aa"'), omit("q"))
    assert reasons(out) == {"q": "value-over-limit"}
    # first means first: evaluation stops there, so a later non-size error is never met
    out = gen(ex("r", f'nb({GROW17}, {OVER}) == b"aa"'), omit("r"))
    assert reasons(out) == {"r": "value-over-limit"}


def test_a_size_error_in_a_literal_is_a_size_error_too():
    huge = "61" * 70_000  # a literal past MAX_IR_VALUE: found while compiling its argument
    out = gen(ex("lit", f'echo(b"{huge}") == b"aa"'), omit("lit"))
    assert reasons(out) == {"lit": "value-over-limit"}


def test_step_4_a_v0_wrap_plus_an_over_limit_value_still_refuses():
    body = f"wr(255u8 + 2u8, {GROW13}) == b\"aa\""
    msg = refused(ex("w", body), "w", "generation", version="v0")
    assert msg.startswith("W0701") and "`w`" in msg and "wraps" in msg
    # the same example in a v1 document does not wrap, so the line excuses it
    assert reasons(gen(ex("w", body), omit("w"), version="v1")) == {"w": "value-over-limit"}


def test_step_4_an_out_of_admit_call_plus_an_over_limit_value_still_refuses():
    body = f'lo(20u8, {GROW13}) == b"aa"'
    msg = refused(ex("a", body), "a", "generation")
    assert msg.startswith("W0701") and "outside the op's admit" in msg
    # inside the admit, the over-limit value qualifies
    assert reasons(gen(ex("a", f'lo(5u8, {GROW13}) == b"aa"'), omit("a"))) == {
        "a": "value-over-limit"
    }


def test_a_wrap_while_evaluating_the_admit_still_refuses():
    # a callee whose own admit wraps in a v0 document (D18 applied to the admit)
    OPS.append(("wa", "op wa(a: U8 | a + 250u8 > 3u8, b: Bytes) -> Bytes", "ab"))
    try:
        msg = refused(ex("e", f'wa(10u8, {GROW13}) == b"aa"'), "e", "generation", version="v0")
    finally:
        OPS.pop()
    assert "admit of op `wa`" in msg


def test_a_size_error_inside_a_callees_admit_refuses_and_is_not_value_over_limit():
    # CV2-F1 (VER04r 8.4 step 3): only a size error in the arguments or the pure side qualifies;
    # the [D19] check evaluates the callee's admit, and a size error there refuses.
    OPS.append(("big", f"op big(a: Bytes | len({nest(17, 'a')}) > 0, b: Bytes) -> Bytes", "ab"))
    try:
        for names in ("big", ""):  # with the line and without it, the same refusal
            msg = refused(ex("big", 'big(b"aa", b"bb") == b"bb"'), names, "ir-overflow")
            assert "admit of op `big`" in msg
        with pytest.raises(OracleError) as err:
            gen(ex("big", 'big(b"aa", b"bb") == b"bb"'), omit("big"))
        assert not isinstance(err.value, SizeError)
        # the same size error in an *argument* still qualifies
        out = gen(ex("arg", f'big({GROW17}, b"bb") == b"bb"'), omit("arg"))
        assert reasons(out) == {"arg": "value-over-limit"}
    finally:
        OPS.pop()


def test_any_other_check_failure_still_refuses():
    # an example with an op that does not exist is a type error: the line never rescues it
    refused(ex("s", f'nosuch({GROW17}) == b"aa"'), "s", "type")


# -- a value over 4,096 bytes in an emitted example is a generation error (CV2-F1; VER04r 2.3, 14) --


def test_a_non_omitted_example_with_a_bytes_value_over_4096_is_a_limit_error():
    over = "61" * 4097
    for body in (
        f'echo(b"{over}") == b"aa"',  # an argument
        f'echo(b"aa") == b"{over}"',  # the pure side
        f"echo({GROW13}) == b\"aa\"",  # computed, under MAX_IR_VALUE
        f"res(b\"aa\") == Ok({GROW13})",  # under an ok
    ):
        msg = refused(ex("big", body), "", "limit")
        assert "over 4096 bytes" in msg
        assert "6161" not in msg  # never a value
    # exactly 4,096 bytes is emitted
    at = "61" * 4096
    out = gen(ex("at", f'echo(b"{at}") == b"{at}"'), "")
    assert [e["id"] for e in out.corpus["examples"]] == ["warrant/omitx/example/at"]
    # the line turns the refusal into the omission
    assert reasons(gen(ex("big", f'echo(b"{over}") == b"aa"'), omit("big"))) == {
        "big": "value-over-limit"
    }


def test_the_order_of_the_checks_is_unchanged_a_wrap_or_admit_failure_comes_first():
    body = f"wr(255u8 + 2u8, {GROW13}) == b\"aa\""
    msg = refused(ex("w", body), "", "generation", version="v0")
    assert msg.startswith("W0701") and "wraps" in msg
    msg = refused(ex("a", f'lo(20u8, {GROW13}) == b"aa"'), "", "generation")
    assert "outside the op's admit" in msg


def test_removing_the_omit_line_of_the_fixture_example_is_now_a_refusal(tmp_path):
    profile = tmp_path / "corpus.profile"
    write_lf(
        profile,
        "".join(
            f"{ln}\n" for ln in PROFILE.read_text().splitlines() if "echo_big_value" not in ln
        ),
    )
    with pytest.raises(OracleError) as err:
        check(profile=profile)
    assert err.value.code == "limit" and "echo_big_value" in err.value.message


# -- the key ---------------------------------------------------------------------------------


def test_the_key_is_sorted_by_id_and_written_only_when_non_empty():
    examples = ex("zz", 'echo(b"aa") != echo(b"bb")') + ex("aa", 'echo(b"cc") != echo(b"dd")')
    out = gen(examples, omit("zz") + omit("aa"))
    assert [r["id"].rsplit("/", 1)[1] for r in out.corpus["omitted_examples"]] == ["aa", "zz"]
    assert all(set(r) == {"id", "reason"} and r["reason"] in REASONS for r in out.corpus["omitted_examples"])
    plain = gen(ex("keep", 'echo(b"aa") == b"aa"'), "")
    assert "omitted_examples" not in plain.corpus and plain.corpus["examples"]


def test_the_omitted_example_is_not_written_to_the_corpus_or_the_tfv():
    out = derive(WARRANT.read_bytes(), PROFILE.read_bytes(), spec_ref="s", generator=GEN)
    assert [e["id"] for e in out.corpus["examples"]] == ["warrant/omitx/example/echo_small"]
    assert out.tfv.count(b"\n") == 2 and b"echo_big" not in out.tfv


def test_without_the_lines_nothing_changes_for_an_existing_fixture():
    for name in ("synth-v2", "accept-blind-v2"):
        corpus = json.loads((HERE.parent / name / "corpus" / "corpus.json").read_text("utf-8"))
        assert "omitted_examples" not in corpus


# -- verify-v2 -------------------------------------------------------------------------------


def check(root=CORPUS, profile=PROFILE, **kw):
    return verify_v2(root, WARRANT, profile, cache=CACHE, **kw)


def codes(root, **kw):
    report, _ = check(root, **kw)
    return {(m["code"], m["where"]) for m in report.mismatches}


def messages(root, **kw):
    report, _ = check(root, **kw)
    return [m["message"] for m in report.mismatches]


def test_the_committed_fixture_passes_and_is_canonical():
    report, derived = check()
    assert report.ok, report.mismatches
    assert (CORPUS / "corpus.json").read_bytes() == canonical(derived.corpus)
    assert report.notes[0] == "omitted_examples 3"


def test_the_committed_fixture_is_what_make_fixture_writes():
    spec = importlib.util.spec_from_file_location("make_omit_fixture", HERE / "make_fixture.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.main(["--check"]) == 0
    assert all(path.read_bytes() == data for path, data in module.build().items())


def _fresh(tmp_path: pathlib.Path) -> pathlib.Path:
    dest = tmp_path / "corpus"
    shutil.copytree(CORPUS, dest)
    return dest


def _edit(root: pathlib.Path, mutate) -> None:
    corpus = json.loads((root / "corpus.json").read_text(encoding="utf-8"))
    mutate(corpus)
    (root / "corpus.json").write_bytes(canonical(corpus))


def row(id_: str, reason: str) -> dict:
    return {"id": id_, "reason": reason}


GOOD = [row(BIG, "value-over-limit"), row(SIZE, "value-over-limit"), row(TWO, "not-op-eq-pure")]


@pytest.mark.parametrize(
    ("mutate", "fragment"),
    [
        (lambda c: c.update(omitted_examples=[]), "non-empty list"),
        (lambda c: c.update(omitted_examples=None), "non-empty list"),
        (lambda c: c.update(omitted_examples={}), "non-empty list"),
        (lambda c: c.update(omitted_examples=[GOOD[1], GOOD[0], GOOD[2]]), "not sorted"),
        (lambda c: c.update(omitted_examples=[GOOD[0], GOOD[0], GOOD[1], GOOD[2]]), "twice"),
        (lambda c: c.update(omitted_examples=[{**GOOD[0], "why": "x"}, *GOOD[1:]]), "exactly"),
        (lambda c: c.update(omitted_examples=[{"id": BIG}, *GOOD[1:]]), "exactly"),
        (lambda c: c.update(omitted_examples=["x", *GOOD[1:]]), "exactly"),
        (lambda c: c.update(omitted_examples=[row(BIG, "too-big"), *GOOD[1:]]), "closed set"),
        (lambda c: c.update(omitted_examples=[row(BIG, 7), *GOOD[1:]]), "closed set"),
        (lambda c: c.update(omitted_examples=[row(1, "value-over-limit"), *GOOD[1:]]), "closed set"),
    ],
)
def test_a_malformed_omitted_examples_key_fails_with_its_own_message(tmp_path, mutate, fragment):
    root = _fresh(tmp_path)
    _edit(root, mutate)
    report, _ = check(root)
    assert not report.ok
    assert any(fragment in m["message"] for m in report.mismatches), report.mismatches
    assert any(m["where"].startswith("omitted_examples") for m in report.mismatches)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda c: c["omitted_examples"].pop(),  # an omission dropped
        lambda c: c["omitted_examples"].__setitem__(0, row(BIG, "not-op-eq-pure")),  # wrong reason
        lambda c: c["omitted_examples"].__setitem__(2, row(TWO, "value-over-limit")),
        lambda c: c["omitted_examples"].append(row("warrant/omitx/example/echo_small", "value-over-limit")),
        lambda c: c["omitted_examples"].__setitem__(0, row(BIG + "x", "value-over-limit")),
        lambda c: c.pop("omitted_examples"),  # the profile has the lines, the corpus lost the key
        lambda c: c["examples"].append(
            {"id": BIG, "op": "echo", "args": [], "expected": "", "result_type": "Bytes"}
        ),  # an omitted example written anyway
    ],
)
def test_a_wrong_omitted_examples_key_fails_against_the_recomputed_one(tmp_path, mutate):
    root = _fresh(tmp_path)
    _edit(root, mutate)
    assert not check(root)[0].ok


def test_a_corpus_is_bound_to_its_profile(tmp_path):
    # this profile no longer omits echo_two_sided, which is then a shape refusal at generation
    profile = tmp_path / "corpus.profile"
    write_lf(profile, "".join(f"{ln}\n" for ln in PROFILE.read_text().splitlines() if "two_sided" not in ln))
    with pytest.raises(OracleError) as err:
        check(profile=profile)
    assert err.value.code == "generation" and "W0701" in err.value.message
    # a profile that is only reordered keeps the omissions but moves profile_sha256
    reordered = tmp_path / "reordered.profile"
    write_lf(reordered, "".join(f"{ln}\n" for ln in reversed(PROFILE.read_text().splitlines())))
    report, _ = check(profile=reordered)
    assert {(m["code"], m["where"]) for m in report.mismatches} == {("header", "profile_sha256")}


def test_the_key_on_a_corpus_whose_profile_omits_nothing_is_an_unexpected_key(tmp_path):
    synth = HERE.parent / "synth-v2"
    root = tmp_path / "corpus"
    shutil.copytree(synth / "corpus", root)
    _edit(root, lambda c: c.update(omitted_examples=[row(BIG, "value-over-limit")]))
    report, _ = verify_v2(root, synth / "synth.warrant", synth / "corpus.profile", cache=CACHE)
    assert not report.ok
    assert any(m["code"] == "header" and "unexpected top-level key" in m["message"] for m in report.mismatches)


def test_a_profile_that_omits_less_than_the_corpus_records_refuses(tmp_path):
    profile = tmp_path / "corpus.profile"
    write_lf(profile, f"omit_example {TWO}\n")
    with pytest.raises(OracleError):
        check(profile=profile)  # the other two examples are emittable-by-shape but refuse


def test_cli_prints_the_omitted_count(capsys):
    argv = ["verify-v2", str(CORPUS), "--warrant", str(WARRANT), "--profile", str(PROFILE)]
    assert run(argv) == 0
    assert capsys.readouterr().out.strip().endswith("1 suites, 3 omitted examples")
    synth = HERE.parent / "synth-v2"
    argv = [
        "verify-v2",
        str(synth / "corpus"),
        "--warrant",
        str(synth / "synth.warrant"),
        "--profile",
        str(synth / "corpus.profile"),
    ]
    assert run(argv) == 0
    assert "omitted" not in capsys.readouterr().out
