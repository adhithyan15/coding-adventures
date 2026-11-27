"""CV2-F17 / VER04r 2.1: ``Int`` never reaches a corpus.

``Int`` is model-internal: the ``type`` of a ``lit`` node and the intermediate type of ``Nat`` arithmetic.  Lowering
accepts a document that names it elsewhere (the committed ``data-forms-v1`` is a lowering-only fixture of such
documents); *generation* (``derive``, ``emit-v2``) refuses it as ``generation`` / ``W0701`` when it would be written
to a ``types`` row, an op signature or a binder.  A model signature, a ``lit`` and a ``none`` are not looked at.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.emit import emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

GEN = {"name": "test", "version": "0"}
CLAIM = "  claim c:\n    forall a: Bool. echo(a) == a\n"
ECHO = "  op echo(a: Bool) -> Bool\n"


def doc(decls: str = "", ops: str = ECHO, claim: str = CLAIM) -> bytes:
    return f"warrant t v1\n  error E = A | B\n{decls}{ops}{claim}".encode()


def gen(source: bytes):
    return derive(source, b"", spec_ref="t.warrant", generator=GEN)


def refused(source: bytes) -> str:
    with pytest.raises(OracleError) as err:
        gen(source)
    assert err.value.code == "generation" and err.value.message.startswith("W0701"), err.value
    # the same document lowers: only generation refuses
    lower_document(parse_source(source))
    return err.value.message


def test_a_document_without_int_in_a_corpus_position_derives():
    assert gen(doc()).corpus["types"]["E"]["kind"] == "error"


@pytest.mark.parametrize(
    "field",
    ["Int", "Option<Int>", "(Int,Bool)", "Result<Int,E>", "Option<Option<Int>>", "(Bool,(Nat,Option<Int>))"],
)
def test_a_record_field_of_int_at_any_depth_is_refused(field):
    assert "a record field" in refused(doc(f"  record R(i: {field})\n"))


@pytest.mark.parametrize("field", ["Int", "Option<Int>", "(Int,Bool)"])
def test_a_variant_field_of_int_at_any_depth_is_refused(field):
    assert "a variant field" in refused(doc(f"  variant V = A(i: {field}) | B\n"))


def test_a_record_field_reached_through_an_alias_is_refused():
    assert "a record field" in refused(doc("  model Z = Int\n  record R(i: Z)\n"))


@pytest.mark.parametrize(
    "op",
    [
        "  op echo(a: Bool, n: Int) -> Bool\n",
        "  op echo(a: Bool) -> Int\n",
        "  op echo(a: Bool, o: Option<Int>) -> Bool\n",
        "  op echo(a: Bool) -> Option<Int>\n",
        "  op echo(a: Bool) -> Result<Int,E>\n",
    ],
)
def test_an_op_parameter_or_result_of_int_is_refused(op):
    assert "an op signature" in refused(doc(ops=op, claim=CLAIM_FOR[op]))


CLAIM_FOR = {
    "  op echo(a: Bool, n: Int) -> Bool\n": "  claim c:\n    forall a: Bool, n: Int. echo(a, n) == echo(a, n)\n",
    "  op echo(a: Bool) -> Int\n": "  claim c:\n    forall a: Bool. echo(a) == echo(a)\n",
    "  op echo(a: Bool, o: Option<Int>) -> Bool\n": "  claim c:\n    forall a: Bool, o: Option<Int>. echo(a, o) == echo(a, o)\n",
    "  op echo(a: Bool) -> Option<Int>\n": "  claim c:\n    forall a: Bool. echo(a) == echo(a)\n",
    "  op echo(a: Bool) -> Result<Int,E>\n": "  claim c:\n    forall a: Bool. echo(a) == echo(a)\n",
}


def test_an_op_with_no_binder_of_int_still_refuses_on_its_signature():
    source = doc(ops="  op f(n: Int) -> Bool\n" + ECHO)
    assert "an op signature" in refused(source)


def test_a_binder_of_int_is_refused():
    # no op takes an Int, so the binder is the only position (an unused one: it is still enumerated)
    source = doc(claim="  claim c:\n    forall a: Bool, n: Int. echo(a) == a\n")
    assert "a binder" in refused(source)


def test_emit_v2_still_reports_per_suite_it_is_not_a_document_refusal():
    """``emit-v2`` reports each suite as at base; only ``derive`` refuses the document (as for the 8-binder cap)."""
    report = emit(doc("  record R(i: Int)\n"))
    assert report["suites"]["c"]["status"] == "ok"


def test_a_model_signature_lit_and_none_of_int_are_not_refused():
    models = (
        "  model sq(a: Int) -> Int = a * a\n"
        "  model big(a: Bool) -> Bool = sq(3) > 8 and a\n"
        "  model none_int(a: Bool) -> Option<Int> = None\n"
        "  model dflt(a: Bool) -> Bool = unwrap_or(none_int(a), 7) > 0\n"
    )
    out = gen(doc(models, ops=ECHO, claim="  claim c:\n    forall a: Bool. echo(a) == big(a)\n"))
    assert "sq" in out.corpus["models"]


# ---------------------------------------------------------------------------------------------------
# an expected value must be a value of its type (VER04r 2.3), and an Int value is never written
# ---------------------------------------------------------------------------------------------------

NAT_OP = "  op f(a: Nat) -> Nat\n"
NAT_CLAIM = "  claim c:\n    forall a: Nat. f(a) == a\n"


def nat_doc(example: str = "", claim: str = NAT_CLAIM) -> bytes:
    body = f"  example e:\n    {example}\n" if example else ""
    return f"warrant t v1\n{NAT_OP}{body}{claim}".encode()


def w0701(source: bytes) -> str:
    with pytest.raises(OracleError) as err:
        gen(source)
    assert err.value.code == "generation" and err.value.message.startswith("W0701"), err.value
    return err.value.message


def test_an_example_whose_expected_value_is_outside_its_type_is_refused():
    assert gen(nat_doc("f(3) == 2 + 1")).corpus["examples"][0]["expected"] == "3"
    assert "range of Nat" in w0701(nat_doc("f(3) == 2 - 5"))  # -3 was written as "expected": "-3"
    assert "range of Nat" in w0701(nat_doc("f(3) == 18446744073709551615 + 1"))  # 2^64


@pytest.mark.parametrize("expr", ["a - 1", "a * 18446744073709551615", "a + 18446744073709551615"])
def test_a_claim_whose_expected_value_leaves_the_result_type_is_refused_not_an_overflow_error(expr):
    assert "range of Nat" in w0701(nat_doc(claim=f"  claim c:\n    forall a: Nat. f(a) == {expr}\n"))


def test_an_int_value_has_no_json_tfv_or_stream_form_and_says_w0701():
    from coding_adventures_warrant_oracle import ast as A
    from coding_adventures_warrant_oracle import tfv
    from coding_adventures_warrant_oracle.corpus_v2 import value_json
    from coding_adventures_warrant_oracle.stream import make_encoder

    spec = lower_document(parse_source(doc()))
    for call in (
        lambda: value_json(5, A.Ty("Int")),
        lambda: tfv.value(5, A.Ty("Int")),
        lambda: make_encoder(spec, A.Ty("Int")),
    ):
        with pytest.raises(OracleError) as err:
            call()
        assert err.value.code == "generation" and err.value.message.startswith("W0701")


@pytest.mark.parametrize(
    ("head", "good", "bad"),
    [("U8", 255, -1), ("U16", 65535, 2**64), ("U64", 2**64 - 1, 2**64), ("Nat", 2**64 - 1, -1), ("I8", -128, 2**63), ("I8", 127, -(2**63) - 1), ("I64", -(2**63), 2**63)],
)
def test_the_stream_encoder_checks_the_range_of_every_scalar_it_can_state(head, good, bad):
    from coding_adventures_warrant_oracle import ast as A
    from coding_adventures_warrant_oracle.stream import make_encoder

    spec = lower_document(parse_source(doc()))
    encode = make_encoder(spec, A.Ty(head))
    assert len(encode(good)) == 9
    with pytest.raises(OracleError) as err:
        encode(bad)
    assert err.value.code == "generation" and err.value.message.startswith("W0701")


def test_an_example_value_is_range_checked_by_its_own_type_at_any_depth():
    from coding_adventures_warrant_oracle import ast as A
    from coding_adventures_warrant_oracle.corpus_v2 import _check_value

    spec = lower_document(parse_source(b"warrant t v1\n  record P(x: U8, y: I8)\n  op f(a: Bool) -> Bool\n  claim c:\n    forall a: Bool. f(a) == a\n"))
    _check_value((255, -128), A.Ty("P"), spec)
    for bad, ty in [
        ((256, 0), A.Ty("P")),
        ((0, 128), A.Ty("P")),
        (("some", 256), A.Ty("Option", (A.Ty("U8"),))),
        (("ok", (1, -1)), A.Ty("Result", (A.Ty("()", (A.Ty("U8"), A.Ty("Nat"))), A.Ty("E")))),
    ]:
        with pytest.raises(OracleError) as err:
            _check_value(bad, ty, spec)
        assert err.value.message.startswith("W0701")
    _check_value(("none",), A.Ty("Option", (A.Ty("U8"),)), spec)
    _check_value(("err", "BAD"), A.Ty("Result", (A.Ty("U8"), A.Ty("E"))), spec)
