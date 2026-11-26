"""Chunks (VER04r sections 5.2 to 5.5) on hand-built suites.

The kit pins the chunk file and the enumeration of the oracle's own suites.
These tests build small suites with the lane's own library and check the
properties that must hold for *any* suite: a chunk enumerated alone (unranked)
equals the same chunk reached by walking from ordinal 0, ``first_admitted`` is the
running sum of admitted cases, boundaries sit on product ordinals, and every
refusal path of the validator fires.
"""

from __future__ import annotations

import copy
import hashlib
import math

import pytest

from coding_adventures_warrant_corpus.v2 import chunks as C
from coding_adventures_warrant_corpus.v2 import ir as ir_module
from coding_adventures_warrant_corpus.v2.domains import build
from coding_adventures_warrant_corpus.v2.errors import KitError, Refusal
from coding_adventures_warrant_corpus.v2.ir import Program
from coding_adventures_warrant_corpus.v2.semantics import STANDARD
from coding_adventures_warrant_corpus.v2.values import encode_value
from coding_adventures_warrant_corpus.v2.wtypes import Ty, parse_type

ZERO = "0" * 64


def lit_u8(n):
    return ["lit", {"type": "U8", "value": str(n)}]


def lit_nat(n):
    return ["lit", {"type": "Nat", "value": str(n)}]


MODELS = {
    "m": {"params": [["b", "U8"]], "body": ["add", ["var", "b"], lit_u8(1)]},
}
OPS = {"f": {"params": [["b", "U8"]], "ret": "U8", "requires": ["ir.core"]}}
SUBJECT = ["subject_op", "f", [["var", "b"]]]
REFERENCE = ["eq", SUBJECT, ["model_call", "m", [["var", "b"]]]]


def bare(
    binders,
    *,
    admit=("lit", True),
    body=None,
    subject=None,
    expected=None,
    chunk_size=1024,
    header="warrant-stream-v1",
    result_type="Bool",
    types=None,
    ops=None,
    models=None,
    order=None,
):
    """A corpus part and a suite row whose digests are still zero.

    ``binders`` is ``[(name, type_text, descriptor), ...]``.
    """
    subject = SUBJECT if subject is None else subject
    if body is None and expected is not None:
        body = ["eq", subject, expected]  # section 4.3: reference means subject == pure
    corpus = {
        "chunk_size": chunk_size,
        "max_ir_cost": 2000,
        "types": types or {},
        "ops": OPS if ops is None else ops,
        "models": MODELS if models is None else models,
        "model_order": ["m"] if order is None else order,
    }
    product = math.prod(build(d).size for _n, _t, d in binders)
    chunks = max(1, -(-product // chunk_size))
    program = Program(
        types=corpus["types"],
        ops=corpus["ops"],
        models=corpus["models"],
        model_order=corpus["model_order"],
        levels=ir_module.IMPLEMENTED_LEVELS,
    )
    scope = {n: parse_type(t, corpus["types"]) for n, t, _d in binders}
    cost = (
        program.compile(list(admit), scope, "admit").cost
        + program.compile(body or REFERENCE, scope).cost
        + (program.compile(expected, scope, "expected").cost if expected else 0)
    )
    row = {
        "id": "warrant/test/exhaustive/claim",
        "kind": "exhaustive",
        "claim": "claim",
        "ops": ["f"],
        "requires": ["ir.core"],
        "binders": [[n, {"type": t, "domain": d}] for n, t, d in binders],
        "demoted": [],
        "domain_label": "full",
        "admit": list(admit),
        "body": body or REFERENCE,
        "oracle": (
            {"kind": "predicate"}
            if expected is None
            else {"kind": "reference", "subject": subject, "expected": expected}
        ),
        "result_type": result_type,
        "stream": header,
        "product": product,
        "cases": 0,
        "discarded": 0,
        "expected_calls": 0,
        "wraps": 0,
        "ir_cost": cost,
        "chunks": chunks,
        "chunks_sha256": ZERO,
        "inputs_sha256": ZERO,
        "slim_chunks": sorted({0, chunks - 1}),
        "slim_inputs_sha256": ZERO,
    }
    if expected is not None:
        row |= {"expected_sha256": ZERO, "slim_expected_sha256": ZERO}
    if header.endswith("v1"):
        row["v1_inputs_sha256"] = ZERO
        if expected is not None:
            row["v1_expected_sha256"] = ZERO
    return corpus, row


def sealed(binders, **options):
    """A corpus part, a suite row and its chunk file, sealed with the lane's own digests.

    The row's digests are computed by the library, so this checks *structure*
    (sequential equals alone, the running sums, the refusals), not agreement with
    the oracle: the kit does that.
    """
    corpus, row = bare(binders, **options)
    suite = C.build_suite(corpus, row)
    chunks, product = row["chunks"], row["product"]
    results = [C.run_chunk(suite, c) for c in range(chunks)]
    lines, first = [], 0
    for c, result in enumerate(results):
        parts = [c, result.cases, first, result.calls, result.inputs.hex()]
        if result.expected is not None:
            parts.append(result.expected.hex())
        lines.append(" ".join(str(p) for p in parts))
        first += result.cases
    data = ("\n".join(lines) + "\n").encode()
    row["chunks_sha256"] = hashlib.sha256(data).hexdigest()
    row["inputs_sha256"] = C.chunk_digest_framing([r.inputs for r in results])
    row["cases"], row["discarded"] = first, product - first
    row["expected_calls"] = sum(r.calls for r in results)
    if options.get("expected") is not None:
        row["expected_sha256"] = C.chunk_digest_framing([r.expected for r in results])
    return corpus, row, data


A_AND_B = [
    ("a", "Nat", {"d": "nat_count", "k": 69, "size": 70}),
    ("b", "U8", {"d": "u8_all", "size": 256}),
]
# a < 30 keeps 30 of 70 values of `a`; the guard `b < 128` makes the call count depend
# on the case, so the pure trace has to evaluate it.
GUARDED = ["if", ["lt", ["var", "b"], lit_u8(128)], SUBJECT, lit_u8(0)]
REFINED = {
    "admit": ["lt", ["var", "a"], lit_nat(30)],
    "subject": GUARDED,
    "expected": ["model_call", "m", [["var", "b"]]],
    "result_type": "U8",
}


@pytest.fixture(scope="module")
def refined():
    return sealed(A_AND_B, **REFINED)


# -- alone equals sequential ----------------------------------------------------------
def test_the_refinement_discards_tuples_and_the_chunks_account_for_them(refined):
    _corpus, row, data = refined
    assert row["chunks"] == 18 and row["product"] == 17920
    assert row["cases"] == 30 * 256 and row["discarded"] == 40 * 256
    assert row["expected_calls"] == 30 * 128
    assert len(data.splitlines()) == 18


def test_every_chunk_alone_equals_the_sequential_walk(refined):
    corpus, row, data = refined
    suite = C.build_suite(corpus, row)
    walked = C.run_sequential(suite, suite.m - 1)
    assert len(walked) == 18
    for chunk in range(suite.m):
        assert C.run_chunk(suite, chunk) == walked[chunk], chunk
    assert sum(r.cases for r in walked) == 7680


def test_first_admitted_is_the_running_sum_and_boundaries_are_product_ordinals(refined):
    corpus, row, data = refined
    suite = C.build_suite(corpus, row)
    lines = C.check_chunk_file(data, suite)
    running = 0
    for line in lines:
        assert line.first_admitted == running
        running += line.cases
    # chunk 0 covers product ordinals 0..1023 = `a` in 0..3: all admitted; the
    # chunks past ordinal 30*256 = 7680 admit nothing
    assert lines[0].cases == 1024 and lines[7].cases == 512 and lines[8].cases == 0
    assert lines[-1].cases == 0


def test_validate_and_run_reports_first_admitted_from_the_file(refined):
    corpus, row, data = refined
    out = C.validate_and_run(corpus, row, data, 5)
    assert out["first_admitted"] == 5 * 1024 and out["cases"] == 1024
    assert set(out) == {
        "cases",
        "first_admitted",
        "calls",
        "inputs_sha256",
        "expected_sha256",
    }


def test_sequential_disagreements_names_the_chunk_that_differs(refined):
    corpus, row, data = refined
    suite = C.build_suite(corpus, row)
    lines = C.check_chunk_file(data, suite)
    assert C.sequential_disagreements(suite, lines, 3) == []
    walked = C.run_sequential(suite, 3)
    tampered = list(walked)
    tampered[1] = C.ChunkResult(
        walked[1].cases, walked[1].calls, b"\x00" * 32, walked[1].expected
    )
    problems = C.sequential_disagreements(suite, lines, 3, walked=tampered)
    assert problems == ["chunk 1: the sequential walk disagrees with the chunk file"]
    swapped = list(walked)
    swapped[3] = swapped[2]
    problems = C.sequential_disagreements(suite, lines, 3, walked=swapped)
    assert any("enumerated alone it differs" in p for p in problems)


def test_the_context_is_the_header_the_suite_id_and_the_types_length_prefixed(refined):
    corpus, row, _data = refined

    def field(text):
        return len(text.encode()).to_bytes(4, "little") + text.encode()

    expected = (
        b"warrant-stream-v1\x00"
        + field("warrant/test/exhaustive/claim")
        + (2).to_bytes(4, "little")  # two binders
        + field("Nat")
        + field("U8")
        + field("U8")  # the result type
    )
    assert C.build_suite(corpus, row).context == expected


def test_a_chunk_streams_prefix_ends_with_the_chunk_index(refined):
    corpus, row, _data = refined
    suite = C.build_suite(corpus, row)
    prefix = STANDARD.chunk_prefix(suite.context, 7)
    assert prefix.endswith(b"\x07\x00\x00\x00") and prefix.startswith(suite.context)
    # an independent digest of chunk 0's inputs: the prefix, then the 1024 admitted tuples
    digest = hashlib.sha256(STANDARD.chunk_prefix(suite.context, 0))
    for ordinal in range(1024):
        a, b = divmod(ordinal, 256)
        digest.update(encode_value(a, Ty("Nat")) + encode_value(b, Ty("U8")))
    assert C.run_chunk(suite, 0).inputs == digest.digest()


# -- a binder too big to tabulate ------------------------------------------------------------
def test_a_tuple_binder_with_a_v2_stream_is_enumerated_by_unranking(monkeypatch):
    """2^24 tuples: no table, ``nth`` per value; chunk 1 alone equals the walk."""
    triple = {"d": "tuple", "of": [{"d": "u8_all", "size": 256}] * 3, "size": 1 << 24}
    # Pretend, for the test, that the runner implements ir.tuple.
    both = frozenset({"ir.core", "ir.tuple"})
    monkeypatch.setattr(ir_module, "IMPLEMENTED_LEVELS", both)
    corpus, row = bare(
        [("t", "(U8,U8,U8)", triple)],
        header="warrant-stream-v2",
        body=["lit", True],
        chunk_size=1 << 16,
        ops={},
        models={},
        order=[],
    )
    # The runner that does *not* implement it refuses the suite as a level (D64).
    with pytest.raises(Refusal) as caught:
        C.build_suite(
            corpus,
            row | {"requires": ["ir.core", "ir.tuple"]},
            levels=frozenset({"ir.core"}),
        )
    assert caught.value.code == "level-missing"
    # a row that claims ir.core only, yet reaches a tuple, is wrong: a corpus error
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row, levels=frozenset({"ir.core"}))
    assert caught.value.code == "corpus"
    suite = C.build_suite(corpus, row, levels=both)
    assert C._Enumerator(suite, STANDARD).columns[0].values is None
    walked = C.run_sequential(suite, 1)
    assert C.run_chunk(suite, 1) == walked[1] and walked[1].cases == 65536
    # chunk 1 starts at product ordinal 65,536, which unranks to (1, 0, 0)
    digest = hashlib.sha256(STANDARD.chunk_prefix(suite.context, 1))
    tuple_ty = Ty("()", (Ty("U8"),) * 3)
    for ordinal in range(1 << 16, 1 << 17):
        digest.update(
            encode_value(
                (ordinal // 65536, ordinal // 256 % 256, ordinal % 256), tuple_ty
            )
        )
    assert walked[1].inputs == digest.digest()


def test_a_domain_of_long_byte_strings_is_not_tabulated_but_enumerates_correctly():
    """Bytes<4096> has 12,295 values of 4 KiB: over the table's byte budget."""
    node = {"d": "bytes_n", "n": 4096, "size": 12292}
    corpus, row, data = sealed(
        [("x", "Bytes<4096>", node)], body=["lit", True], ops={}, models={}, order=[]
    )
    suite = C.build_suite(corpus, row)
    enumerator = C._Enumerator(suite, STANDARD)
    assert enumerator.columns[0].values is None
    assert C.run_chunk(suite, 2) == C.run_sequential(suite, 2)[2]
    digest = hashlib.sha256(STANDARD.chunk_prefix(suite.context, 2))
    desc = build(node)
    for i in range(2048, 3072):
        digest.update(encode_value(desc.nth(i), Ty("Bytes", (4096,))))
    assert C.run_chunk(suite, 2).inputs == digest.digest()


# -- refusals of the corpus part and the row (code `corpus`) -------------------------------------------
def tampered(refined, edit):
    corpus, row, data = copy.deepcopy(refined[0]), copy.deepcopy(refined[1]), refined[2]
    edit(corpus, row)
    return corpus, row, data


TRIPLE = {"d": "tuple", "of": [{"d": "u8_all", "size": 256}] * 3, "size": 1 << 24}


def drop_reference_keys(corpus, row):
    """Make the row look like a predicate suite, with an oracle kind nobody knows."""
    for key in ("expected_sha256", "slim_expected_sha256", "v1_expected_sha256"):
        row.pop(key)
    row["oracle"] = {"kind": "mystery"}


CORPUS_TAMPERS = {
    "chunk_size is a bool": lambda c, r: c.update(chunk_size=True),
    "chunk_size is text": lambda c, r: c.update(chunk_size="1024"),
    "max_ir_cost is a bool": lambda c, r: c.update(max_ir_cost=True),
    "a binder is not a pair": lambda c, r: r.update(binders=[1]),
    "a binder lacks its type": lambda c, r: r["binders"][0][1].pop("type"),
    "a descriptor not for the type": lambda c, r: r["binders"][0][1].update(type="U8"),
    "an unknown stream header": lambda c, r: r.update(stream="warrant-stream-v1-x"),
    "an unknown oracle kind": drop_reference_keys,
    "an ill-typed body": lambda c, r: r.update(
        body=["add", ["lit", True], ["lit", True]]
    ),
    "a body that is not Bool": lambda c, r: r.update(body=lit_u8(1)),
    "an admit that is not Bool": lambda c, r: r.update(admit=lit_u8(1)),
    "a subject call in admit": lambda c, r: r.update(admit=["eq", SUBJECT, lit_u8(1)]),
    "an unknown tag in the expected side": lambda c, r: r["oracle"].update(
        expected=["wibble"]
    ),
    "a model that calls a later one": lambda c, r: c["models"]["m"].update(
        body=["model_call", "m", [["var", "b"]]]
    ),
    "an unknown result type": lambda c, r: r.update(result_type="Nope"),
    "an ir_cost that is not recomputed": lambda c, r: r.update(
        ir_cost=r["ir_cost"] + 1
    ),
    "an ir_cost that is a bool": lambda c, r: r.update(ir_cost=True),
    "no binders but a product of 2": lambda c, r: r.update(binders=[]),
    "nine binders": lambda c, r: r.update(binders=r["binders"][:1] * 9),
    "a product past 2^28": lambda c, r: r.update(
        binders=[["t", {"type": "(U8,U8,U8)", "domain": TRIPLE}]] * 2, product=1 << 48
    ),
    "a product that is not the product": lambda c, r: r.update(
        product=r["product"] + 1
    ),
    "a chunk count that is wrong": lambda c, r: r.update(chunks=r["chunks"] + 1),
    "a row that is not a dict": lambda c, r: r.clear() or r.update(x=1),
}


@pytest.mark.parametrize("why", sorted(CORPUS_TAMPERS))
def test_a_refused_corpus_part_or_row_is_the_corpus_code(refined, why):
    corpus, row, _data = tampered(refined, CORPUS_TAMPERS[why])
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row)
    assert caught.value.code == "corpus", why


def test_a_corpus_that_is_not_an_object_or_has_other_keys_is_refused(refined):
    corpus, row, _data = refined
    for bad in ("corpus", {"chunk_size": 1024}, corpus | {"extra": 1}):
        with pytest.raises(Refusal) as caught:
            C.build_suite(bad, row)
        assert caught.value.code == "corpus"
    for size in (1000, 512, 1 << 17, 0):
        with pytest.raises(Refusal):
            C.build_suite(corpus | {"chunk_size": size}, row)


def test_the_row_key_set_depends_on_the_oracle_kind_and_the_header():
    base = C.expected_row_keys(
        {"oracle": {"kind": "predicate"}, "stream": "warrant-stream-v2"}
    )
    assert "expected_sha256" not in base and "v1_inputs_sha256" not in base
    reference = C.expected_row_keys(
        {"oracle": {"kind": "reference"}, "stream": "warrant-stream-v1"}
    )
    assert {
        "expected_sha256",
        "slim_expected_sha256",
        "v1_inputs_sha256",
        "v1_expected_sha256",
    } <= reference
    assert C.is_reference({"oracle": {"kind": "reference"}}) and not C.is_reference({})


def test_a_chunk_past_the_last_is_the_corpus_code(refined):
    corpus, row, data = refined
    with pytest.raises(Refusal) as caught:
        C.validate_and_run(corpus, row, data, 18)
    assert caught.value.code == "corpus"


# -- slim chunks and schema errors -------------------------------------------------------------------------
@pytest.mark.parametrize(
    "slim",
    [[1, 17], [0, 16], [0, 17, 17], [0, 5, 3, 17], [], [0, 18], "x", None, list(range(18))[:0] + [0] * 2 + [17]],
)  # fmt: skip
def test_an_invalid_slim_set_is_the_slim_chunks_code(slim):
    with pytest.raises(Refusal) as caught:
        C.check_slim_chunks({"slim_chunks": slim}, 18)
    assert caught.value.code == "slim-chunks"


def test_a_slim_set_of_exactly_32_entries_is_valid_and_33_is_not():
    C.check_slim_chunks({"slim_chunks": [*range(31), 255]}, 256)
    with pytest.raises(Refusal):
        C.check_slim_chunks({"slim_chunks": [*range(32), 255]}, 256)


@pytest.mark.parametrize("bad", [-1, True, "3", 1.5, None])
def test_a_slim_entry_or_index_that_breaks_the_schema_is_a_kit_error(bad):
    with pytest.raises(KitError):
        C.check_slim_chunks({"slim_chunks": [0, bad, 17]}, 18)
    with pytest.raises(KitError):
        C.schema_index(bad, "a chunk")
    assert C.schema_index(4, "a chunk") == 4


# -- the chunk file (code `chunk-file`) ----------------------------------------------------------------------
def refuse_file(refined, data=None, **row_edits):
    corpus, row, file = refined
    suite = C.build_suite(corpus, row | row_edits)
    with pytest.raises(Refusal) as caught:
        C.check_chunk_file(file if data is None else data, suite)
    assert caught.value.code == "chunk-file"


def test_a_chunk_file_is_checked_against_every_column_of_its_row(refined):
    data = refined[2]
    sealed_row = refined[1]
    refuse_file(refined, data[:-1] + b"X")  # not the digest of the file
    refuse_file(refined, cases=sealed_row["cases"] + 1)
    refuse_file(refined, discarded=sealed_row["discarded"] + 1)
    refuse_file(refined, inputs_sha256=ZERO)
    refuse_file(refined, expected_sha256=ZERO)
    refuse_file(refined, chunks_sha256=ZERO)


def test_every_line_of_the_chunk_file_obeys_the_grammar(refined):
    corpus, row, data = refined
    lines = data.decode().split("\n")[:-1]

    def refuse(new_lines):
        text = "\n".join(new_lines) + "\n"
        refuse_file(
            refined,
            text.encode(),
            chunks_sha256=hashlib.sha256(text.encode()).hexdigest(),
        )

    refuse([lines[0].replace(" 0 ", " 00 ", 1), *lines[1:]])  # a leading zero
    refuse([lines[0].rsplit(" ", 1)[0], *lines[1:]])  # the expected column missing
    refuse([lines[0] + " " + ZERO, *lines[1:]])  # a column too many
    refuse([lines[1], lines[0], *lines[2:]])  # out of sequence
    refuse(lines[:-1])  # a line short
    refuse(
        [lines[0].replace("0 1024 0", "0 1024 5"), *lines[1:]]
    )  # first_admitted wrong
    refuse([lines[0].upper(), *lines[1:]])  # upper-case hex
    refuse(["é" + lines[0], *lines[1:]])  # not ASCII


def test_a_ground_claim_has_no_binders_and_one_case_with_an_empty_tuple():
    """R75: zero binders is a product of 1: one chunk, one admitted case, empty input."""
    corpus, row, data = sealed([], body=["lit", True], ops={}, models={}, order=[])
    suite = C.build_suite(corpus, row)
    assert (row["product"], row["chunks"], row["cases"]) == (1, 1, 1)
    got = C.run_chunk(suite, 0)
    assert got.cases == 1
    assert (
        got.inputs == hashlib.sha256(STANDARD.chunk_prefix(suite.context, 0)).digest()
    )
    assert C.run_sequential(suite, 0) == [got]
    assert len(C.check_chunk_file(data, suite)) == 1
