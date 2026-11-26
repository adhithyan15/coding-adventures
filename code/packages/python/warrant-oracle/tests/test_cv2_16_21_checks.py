"""CV2-F17 / VER04r 16.21: [D73] chunk ``calls`` sum, R106 ``Bytes<N>`` length, R108 consumer-side ``types`` checks.

* [D73]: ``check_chunk_file`` refuses ``chunk-file`` a file whose chunks' ``calls`` do not sum to the row's
  ``expected_calls``, even when ``chunks_sha256`` was recomputed for the altered file.
* R106: a value written for a ``Bytes<N>`` position whose length is not ``N`` is ``generation`` /
  ``W0701: a stream value does not have the declared Bytes<N> length``, at any depth, for a claim's expected stream
  (``stream.make_encoder``) and for an example's arguments and expected value (``corpus_v2._check_value``).
* R108: ``check_type_table`` refuses a recursive table, a repeated field or constructor name and an undeclared
  field type with the code of the context (``type`` in ``v2-value``, ``corpus`` in ``v2-chunk``), and the two
  crashes the reviewer found (``RecursionError``; an uncaught ``Refused`` from ``_encoder_spec``) are outcomes.
"""

from __future__ import annotations

import hashlib
import pathlib
import types as pytypes

import pytest

from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import corpus_v2 as V
from coding_adventures_warrant_oracle.ast import Ty
from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.stream import BYTES_LENGTH_W0701, make_encoder

GEN = {"name": "test", "version": "0"}
W0701 = "W0701: a stream value does not have the declared Bytes<N> length"


def gen(source: str):
    return derive(source.encode(), b"", spec_ref="t.warrant", generator=GEN)


def refused_length(source: str) -> None:
    with pytest.raises(OracleError) as err:
        gen(source)
    assert err.value.code == "generation" and err.value.message == W0701


# =========================================================== [D73]


@pytest.fixture(scope="module")
def small_kit():
    here = pathlib.Path(__file__).resolve().parent / "fixtures" / "conformance-v2"
    return C.build_kit((here / "kit-small.warrant").read_bytes(), (here / "kit-small.profile").read_bytes())


def _sealed(data: bytes, row: dict) -> dict:
    return {**row, "chunks_sha256": hashlib.sha256(data).hexdigest()}


def _bump_calls(data: bytes, line_no: int, delta: int) -> bytes:
    lines = data.decode().split("\n")
    parts = lines[line_no].split(" ")
    parts[3] = str(int(parts[3]) + delta)
    lines[line_no] = " ".join(parts)
    return "\n".join(lines).encode()


def test_d73_calls_that_do_not_sum_to_expected_calls_are_chunk_file(small_kit):
    row = small_kit.row("ragged")
    good = small_kit.chunk_file("ragged")
    assert sum(int(line.split(" ")[3]) for line in good.decode().split("\n")[:-1]) == row["expected_calls"]
    assert len(C.check_chunk_file(good, row, 17, 1024)) == 17
    # one line's calls is one more, the file's digest recomputed: the sum is the only check that fails
    for delta in (1, 1000):
        bad = _bump_calls(good, 3, delta)
        with pytest.raises(C.Refused) as exc:
            C.check_chunk_file(bad, _sealed(bad, row), 17, 1024)
        assert exc.value.code == "chunk-file" and "expected_calls" in str(exc.value)
    # the row disagreeing the other way (file right, row's expected_calls changed) is refused too
    with pytest.raises(C.Refused) as exc:
        C.check_chunk_file(good, {**row, "expected_calls": row["expected_calls"] + 1}, 17, 1024)
    assert exc.value.code == "chunk-file"


def test_d73_a_v2_chunk_case_with_a_wrong_calls_sum_is_chunk_file(small_kit):
    row = small_kit.row("ragged")
    good = small_kit.chunk_file("ragged")
    bad = _bump_calls(good, 0, 1)
    case = {
        "corpus": small_kit.corpus_part(),
        "suite": _sealed(bad, row),
        "chunk": 1,
        "chunk_file_hex": bad.hex(),
    }
    assert C.derive_chunk(case, small_kit) == {"error": "chunk-file"}
    case["suite"], case["chunk_file_hex"] = row, good.hex()
    assert "error" not in C.derive_chunk(case, small_kit)


# =========================================================== R106


def test_r106_claim_expected_value_of_another_length_is_refused():
    refused_length('warrant t v1\n  op f(a: U8) -> Bytes<2>\n  claim c:\n    forall a: U8. f(a) == b"aabbcc"\n')


def test_r106_claim_equal_length_derives():
    assert gen('warrant t v1\n  op f(a: U8) -> Bytes<2>\n  claim c:\n    forall a: U8. f(a) == b"aabb"\n')


def test_r106_claim_under_option_is_refused_and_equal_length_derives():
    head = "warrant t v1\n  op f(a: U8) -> Option<Bytes<2>>\n  claim c:\n    forall a: U8. f(a) == Some("
    refused_length(head + 'b"aabbcc")\n')
    assert gen(head + 'b"aabb")\n')


def test_r106_example_argument_and_expected_forms():
    refused_length('warrant t v1\n  op f(a: U8) -> Bytes<2>\n  example e:\n    f(1u8) == b"aabbcc"\n')
    assert gen('warrant t v1\n  op f(a: U8) -> Bytes<2>\n  example e:\n    f(1u8) == b"aabb"\n')


def test_r106_named_width_after_substitution():
    base = "warrant t v1\n  op f<N: Nat>(a: Bytes<N>) -> Bytes<N>\n"
    refused_length(base + '  claim c:\n    forall a: Bytes<4>. f<4>(a) == b"aabbcc"\n')
    refused_length(base + '  example e:\n    f<4>(b"aabbccdd") == b"aabbcc"\n')
    # N is the substituted width: 3 here, and both sides are 3 bytes
    assert gen(base + '  example e:\n    f<3>(b"aabbcc") == b"aabbcc"\n')


def test_r106_plain_bytes_is_unaffected():
    assert gen('warrant t v1\n  op f(a: Bytes) -> Bytes\n  example e:\n    f(b"aabbcc") == b"aa"\n')
    assert make_encoder(_spec(), Ty("Bytes"))(b"abc") == b"\x20\x03\x00\x00\x00abc"


def _spec(records=None, variants=None):
    return pytypes.SimpleNamespace(records=records or {}, variants=variants or {}, errors={})


def test_r106_encoder_at_every_depth():
    b2 = Ty("Bytes", (2,))
    spec = _spec(
        records={"R": [("x", b2), ("y", Ty("Bool"))]},
        variants={"V": [("A", [("f0", b2)]), ("B", [])]},
    )
    wrong, right = b"abc", b"ab"
    cases = [
        (b2, wrong, right),
        (Ty("Option", (b2,)), ("some", wrong), ("some", right)),
        (Ty("Result", (b2, Ty("E"))), ("ok", wrong), ("ok", right)),
        (Ty("()", (Ty("Bool"), b2)), (True, wrong), (True, right)),
        (Ty("R"), (wrong, True), (right, True)),
        (Ty("V"), ("A", (wrong,)), ("A", (right,))),
        (Ty("List", (b2,)), ("list", (right, wrong)), ("list", (right, right))),
    ]
    for ty, bad, good in cases:
        enc = make_encoder(spec, ty)
        assert enc(good)
        with pytest.raises(OracleError) as err:
            enc(bad)
        assert err.value.code == "generation" and err.value.message == BYTES_LENGTH_W0701 == W0701
    # the omit decision comes first: an absent value (none) and an error case are never length-checked
    assert make_encoder(spec, Ty("Option", (b2,)))(("none",)) == b"\x40"
    assert make_encoder(spec, Ty("Result", (b2, Ty("E"))))(("err", "X"))


def test_r106_check_value_at_every_depth_and_a_still_named_width():
    b2 = Ty("Bytes", (2,))
    spec = _spec(records={"R": [("x", b2)]}, variants={"V": [("A", [("f0", b2)])]})
    for ty, bad in [
        (b2, b"abc"),
        (Ty("Option", (b2,)), ("some", b"")),
        (Ty("R"), (b"abc",)),
        (Ty("V"), ("A", (b"a",))),
        (Ty("()", (Ty("Bool"), b2)), (True, b"abc")),
    ]:
        with pytest.raises(OracleError) as err:
            V._check_value(bad, ty, spec)
        assert err.value.message == W0701
    V._check_value(b"ab", b2, spec)
    V._check_value(b"abcdef", Ty("Bytes"), spec)
    V._check_value(b"abcdef", Ty("Bytes", ("N",)), spec)  # a named width not substituted is plain here


def test_r106_a_suite_reference_stream_nested_under_a_record_is_refused():
    # the record case reaches make_encoder through a binder-free reference suite's result type
    spec = _spec(records={"R": [("x", Ty("Bytes", (4,)))]})
    with pytest.raises(OracleError):
        make_encoder(spec, Ty("Option", (Ty("R"),)))(("some", (b"abc",)))


# =========================================================== R108

ERR = {"E": {"kind": "error", "cases": ["A", "B"]}}


def rec(*fields):
    return {"kind": "record", "fields": [list(f) for f in fields]}


def var(*ctors):
    return {"kind": "variant", "ctors": [[n, list(ts)] for n, ts in ctors]}


BAD_TABLES = {
    "self": {"R": rec(("a", "R"))},
    "self-option": {"R": rec(("a", "Option<R>"))},
    "self-result": {**ERR, "R": rec(("a", "Result<R,E>"))},
    "self-tuple": {"R": rec(("a", "(Bool,R)"))},
    "self-list": {"R": rec(("a", "List<R>"))},
    "mutual": {"R": rec(("a", "Option<S>")), "S": rec(("b", "R"))},
    "mutual-variant": {"R": rec(("a", "V")), "V": var(("K", ["Option<R>"]))},
    "variant-self": {"V": var(("K", ["V"]))},
    "dup-field": {"R": rec(("a", "Bool"), ("a", "Nat"))},
    "dup-ctor": {"V": var(("K", []), ("K", ["Bool"]))},
    "undeclared": {"R": rec(("a", "Missing"))},
    "undeclared-nested": {"R": rec(("a", "Option<(Bool,Missing)>"))},
    "not-text": {"R": rec(("a", 7))},
    "not-a-table": [],
    "malformed-row": {"R": {"kind": "record"}},
}
GOOD_TABLES = {
    "empty": {},
    "diamond": {"A": rec(("x", "Bool")), "B": rec(("l", "A"), ("r", "A")), "C": rec(("p", "B"), ("q", "A"))},
    "shared-names-across-rows": {"R": rec(("a", "Bool")), "S": rec(("a", "Bool")), "V": var(("a", []), ("b", []))},
    **{"errors": ERR},
}


@pytest.mark.parametrize("name", sorted(BAD_TABLES))
@pytest.mark.parametrize("code", ["type", "corpus"])
def test_r108_bad_table_is_refused_with_the_context_code(name, code):
    with pytest.raises(C.Refused) as exc:
        C.check_type_table(BAD_TABLES[name], code)
    assert exc.value.code == code


@pytest.mark.parametrize("name", sorted(GOOD_TABLES))
def test_r108_good_tables_pass(name):
    C.check_type_table(GOOD_TABLES[name], "type")


def test_r108_a_very_long_chain_does_not_recurse():
    n = 3000  # no type-depth limit applies to a chain of rows; the walk is iterative
    table = {f"T{i}": rec(("a", f"T{i + 1}")) for i in range(n)}
    table[f"T{n}"] = rec(("a", "Bool"))
    C.check_type_table(table, "type")
    table[f"T{n}"] = rec(("a", "T0"))
    with pytest.raises(C.Refused):
        C.check_type_table(table, "type")


def _hex(text: str) -> str:
    return text.encode().hex()


@pytest.mark.parametrize("name", sorted(set(BAD_TABLES) - {"not-a-table"}))
def test_r108_v2_value_is_type_even_when_the_value_does_not_use_the_record(name):
    # an unused bad record beside a plain Bool value: the table is refused whole, never read lazily
    table = {**BAD_TABLES[name]}
    case = {"type": "Bool", "types": table, "input_hex": _hex("true")}
    assert C.derive_value(case) == {"outcome": "reject", "error": "type"}


def test_r108_v2_value_two_crashes_are_outcomes():
    # a record that contains itself through an Option: was a RecursionError in derive_value
    table = {"R": rec(("a", "Option<R>"))}
    case = {"type": "R", "types": table, "input_hex": _hex('[{"none": true}]')}
    assert C.derive_value(case) == {"outcome": "reject", "error": "type"}
    # an undeclared field type in a record the value does not use: was an uncaught Refused from _encoder_spec
    table = {"U": rec(("a", "Nope"))}
    case = {"type": "Bool", "types": table, "input_hex": _hex("true")}
    assert C.derive_value(case) == {"outcome": "reject", "error": "type"}


def test_r108_v2_value_malformed_json_keeps_its_place_in_the_code_order():
    table = {"R": rec(("a", "R"))}
    case = {"type": "Bool", "types": table, "input_hex": _hex("tru")}
    assert C.derive_value(case) == {"outcome": "reject", "error": "malformed-json"}


def test_r108_v2_value_good_tables_still_accept():
    table = {"R": rec(("a", "Bool"), ("b", "Option<Bool>"))}
    case = {"type": "R", "types": table, "input_hex": _hex('[true, {"some": false}]')}
    assert C.derive_value(case)["outcome"] == "accept"


@pytest.mark.parametrize("name", sorted(set(BAD_TABLES) - {"not-a-table"}))
def test_r108_v2_chunk_is_corpus(name, small_kit):
    row = small_kit.row("ragged")
    good = small_kit.chunk_file("ragged")
    corpus = {**small_kit.corpus_part(), "types": BAD_TABLES[name]}
    case = {"corpus": corpus, "suite": row, "chunk": 1, "chunk_file_hex": good.hex()}
    assert C.derive_chunk(case, small_kit) == {"error": "corpus"}
    with pytest.raises(C.Refused) as exc:
        C.check_corpus_and_row(corpus, row, 1)
    assert exc.value.code == "corpus"
