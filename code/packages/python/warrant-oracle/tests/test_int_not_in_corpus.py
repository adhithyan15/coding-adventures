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


def test_emit_refuses_the_same_documents():
    with pytest.raises(OracleError) as err:
        emit(doc("  record R(i: Int)\n"))
    assert err.value.code == "generation" and err.value.message.startswith("W0701")


def test_a_model_signature_lit_and_none_of_int_are_not_refused():
    models = (
        "  model sq(a: Int) -> Int = a * a\n"
        "  model big(a: Bool) -> Bool = sq(3) > 8 and a\n"
        "  model none_int(a: Bool) -> Option<Int> = None\n"
        "  model dflt(a: Bool) -> Bool = unwrap_or(none_int(a), 7) > 0\n"
    )
    out = gen(doc(models, ops=ECHO, claim="  claim c:\n    forall a: Bool. echo(a) == big(a)\n"))
    assert "sq" in out.corpus["models"]
