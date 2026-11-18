#!/usr/bin/env python3
"""Write (or ``--check``) the synthetic ``ir.tuple`` document of slice S10t-1 and its committed dumps.

The document ``tuple_forms_v1.warrant`` exercises **every tuple form and every ``match`` pattern** of
VER04r Appendix A.1 and every ``ir.tuple`` fault template of section 8.2, so that the Rust generator
(slice S10t-2) has one document to reproduce byte for byte.  It is generated, not hand-written,
because the arity matrix (2 to 8 components) is repetitive; the committed text and this script cannot
drift (``--check``, a test, and a line of ``BUILD``).

Five files are written beside this script:

``tuple_forms_v1.warrant``
    the document (this script's text);
``tuple_forms_v1.profile``
    the ``corpus.profile`` the dumps use (``chunk_size 1024``, the fault lines, the two
    ``omit_example`` lines of its match-shaped examples);
``tuple_forms_v1.oracle-lowering.json``
    the lowering dump (``lowering_dump``, schema ``warrant-lowering-parity-v1``): the lowered IR, the
    computed ``requires`` and the cost of every model, op ``admit``, claim and example;
``tuple_forms_v1.oracle-evaluation.json``
    the evaluation dump (``parity_dumps.evaluation_view``): domains, counts, every chunk line and
    digest, the examples and ``cases.tfv``;
``tuple_forms_v1.oracle-faults.json``
    the fault dump (``parity_dumps.faults_view``): the JSON of every fault row, which VER04r 16.10
    R11 makes **normative** for ``ok-as-err``, ``drop-consumed``, ``swap-components`` and the
    tuple-payload ``missing-error``, and the controls under both fault-evaluation gates;
``tuple_forms_v1.oracle-refusals.json``
    every corpus-error document of Appendix A.1 and the oracle's refusal of it (code and message).

    python3 make_tuple_forms.py            # (re)write the five files
    python3 make_tuple_forms.py --check    # fail if any committed file differs

Only the standard library and the oracle package are used.  Names: ``lit<n>`` for the arity-``n``
tuple literal, ``op_*`` for subject ops, ``c_*`` for claims, ``e_*`` for examples.
"""

from __future__ import annotations

import pathlib
import re
import sys

from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.lowering_dump import dump
from coding_adventures_warrant_oracle.parity_dumps import dump_evaluation, dump_faults
from coding_adventures_warrant_oracle.parser import parse_source

HERE = pathlib.Path(__file__).resolve().parent
NAME = "tuple_forms_v1"

#: The ``corpus.profile`` of the dumps.  ``chunk_size 1024`` is the smallest chunk, so the larger
#: suites have several chunks and the slim stride (``m > 16``) never applies; the ``fault`` lines
#: cover every template of the slice; the two ``omit_example`` lines excuse the match-shaped examples
#: (``not-op-eq-pure``, VER04r 8.4), which lower and are not evaluated.
PROFILE = """\
budget 16777216
chunk_size 1024
slim_stride auto
fault op_k ok-as-err
fault op_k swap-components
fault op_pair drop-consumed
fault op_dec drop-consumed
fault op_dec missing-error
fault op_zero missing-error
fault op_lit2 swap-components
fault op_lit3 swap-components
fault op_proj_last8 invert
fault op_bytes flip-result-bit
fault op_bytes drop-last-result-byte
fault op_bits flip-result-bit
fault op_bits drop-last-result-byte
fault op_u8 flip-result-bit
omit_example warrant/tuple_forms/example/e_match_ok
omit_example warrant/tuple_forms/example/e_match_err
"""

OPS: list = []  # (name, [params]) for the contract blocks
B = "Bool"
RT = "Result<(Bool,Bool),TupleError>"
RB = "Result<Bool,TupleError>"

#: Pure models, each also checked by a generated subject op ``op_<name>`` with the same signature
#: and a reference claim ``c_<name>``: ``(name, parameters, result type, body)``.  The matrix covers
#: the tuple forms (literal at every arity, nesting, a typed literal context, ``project``, ``==``
#: and ``!=``) and every ``match`` pattern.
MODELS = [
    # tuple literals, arity 2 to 8, over Bool components (a tuple value is a JSON array)
    *[
        (
            f"lit{n}",
            ", ".join(f"{'abcdefgh'[i]}: Bool" for i in range(n)),
            "(" + ",".join([B] * n) + ")",
            "(" + ", ".join("abcdefgh"[:n]) + ")",
        )
        for n in range(2, 9)
    ],
    ("nested", "a: Bool, b: Bool, c: Bool", "((Bool,Bool),Bool)", "((a, b), c)"),
    ("deep", "a: Bool", "(((Bool,Nat),Bool),Bytes<0>)", '(((a, 1), not a), b"")'),
    ("lit_ctx", "", "(Nat,Nat)", "(1, 2)"),
    ("lit_arith", "a: U8", "(U8,Nat)", "(a + 1u8, 3)"),
    ("pair", "a: Bool, n: Nat", "(Bool,Nat)", "(a, n)"),
    # project: ``t.i``, a chain for nested tuples, the postfix form after a call and on a literal
    ("proj0", "t: (Bool,Nat)", B, "t.0"),
    ("proj1", "t: (Bool,Nat)", "Nat", "t.1"),
    ("proj_chain", "t: ((Bool,Nat),Bool)", "Nat", "t.0.1"),
    ("proj_last8", "t: (Bool,Bool,Bool,Bool,Bool,Bool,Bool,Bool)", B, "t.7"),
    ("proj_postfix", "a: Bool, n: Nat", "Nat", "pair(a, n).1"),
    ("proj_literal", "", "U8", "(true, 2u8).1"),
    ("swap", "t: (Bool,Bool)", "(Bool,Bool)", "(t.1, t.0)"),
    # structural equality, component by component
    ("tup_eq", "a: (Bool,Bool), b: (Bool,Bool)", B, "a == b"),
    ("tup_ne", "a: (Bool,Bool), b: (Bool,Bool)", B, "a != b"),
    ("tup_eq_lit", "a: (Bool,Nat)", B, "a == (true, 3)"),
    # match on a Result: ok, err, err_any, tuple, bind, wild
    ("k", "a: Bool, b: Bool", RT, "if a and b then Err(TupleError.Worse) else Ok((a, b))"),
    ("dec", "a: Bool, n: Nat", "Result<(Bool,Nat),TupleError>", "if n > 60 then Err(TupleError.Bad) else Ok((a, n))"),
    ("m_ok_wild", f"r: {RT}", B, "match r with Ok(_) -> true | Err(_) -> false"),
    ("m_ok_bind", f"r: {RB}", B, "match r with Ok(x) -> x | Err(_) -> false"),
    ("m_ok_tuple", f"r: {RT}", B, "match r with Ok((a, b)) -> a and b | Err(_) -> false"),
    ("m_ok_tuple_wild", f"r: {RT}", B, "match r with Ok((_, b)) -> b | Err(_) -> false"),
    (
        "m_ok_nested",
        "r: Result<((Bool,Bool),Bool),TupleError>",
        B,
        "match r with Ok(((a, _), c)) -> a and c | Err(_) -> false",
    ),
    (
        "m_err_cases",
        f"r: {RB}",
        "Nat",
        "match r with Ok(_) -> 0 | Err(TupleError.Bad) -> 1 | Err(TupleError.Worse) -> 2 | Err(TupleError.Worst) -> 3",
    ),
    ("m_err_any_last", f"r: {RB}", "Nat", "match r with Err(TupleError.Bad) -> 1 | Err(e) -> 2 | Ok(_) -> 0"),
    ("m_bind_whole", f"r: {RB}", B, "match r with Ok(_) -> true | whole -> whole == r"),
    ("m_wild_arm", f"r: {RB}", "Nat", "match r with Ok(_) -> 1 | _ -> 0"),
    ("m_returns_tuple", f"r: {RT}", "(Bool,Bool)", "match r with Ok((a, b)) -> (b, a) | Err(_) -> (false, false)"),
    (
        "m_returns_result",
        f"r: {RT}",
        RT,
        "match r with Ok((a, b)) -> Ok((b, a)) | Err(_) -> Err(TupleError.Bad)",
    ),
    (
        "m_nested_match",
        "r: Result<Result<Bool,TupleError>,TupleError>",
        "Nat",
        "match r with Ok(Ok(_)) -> 0 | Ok(Err(_)) -> 1 | Err(_) -> 2",
    ),
    (
        "m_inner_match",
        f"r: {RB}, s: {RB}",
        B,
        "match r with Ok(x) -> (match s with Ok(y) -> x and y | Err(_) -> false) | Err(_) -> false",
    ),
    (
        "m_on_call",
        "a: Bool, b: Bool",
        B,
        "match k(a, b) with Ok((x, _)) -> x | Err(TupleError.Worse) -> true | Err(_) -> false",
    ),
    ("m_arith_arm", "r: Result<U8,TupleError>", "U8", "match r with Ok(x) -> x + 1u8 | Err(_) -> 0u8"),
]


def _names(params: str) -> list[str]:
    return re.findall(r"(?:^|, )(\w+): ", params)


def main() -> str:
    out: list[str] = []
    w = out.append

    def op(line: str) -> None:
        """Declare a subject op and remember its parameters for the contract blocks."""
        w(line)
        name, rest = line.split("op ", 1)[1].split("(", 1)
        OPS.append((name.strip(), _names(rest.rsplit(")", 1)[0])))

    OPS.clear()
    w("warrant tuple_forms v1")
    w("  -- Synthetic ir.tuple document of slice S10t-1 (VER04r Appendix A.1): every tuple form,")
    w("  -- every match pattern and every ir.tuple fault template.  GENERATED by make_tuple_forms.py;")
    w("  -- do not edit by hand.")
    w("  error TupleError = Bad | Worse | Worst")
    w("")
    w("  -- pure models: tuple literals (arity 2 to 8, nesting, a typed literal context), project,")
    w("  -- structural equality, and match on a Result with every pattern --------------------------")
    for name, params, ret, body in MODELS:
        w(f"  model {name}({params}) -> {ret} = {body}")
    w("  model mixed(a: U8, n: Nat, d: Bytes<0>, f: Bool) -> (U8,Nat,Bytes<0>,Bool) = (a, n, d, f)")
    w("")
    w("  -- subject ops, claims and examples ---------------------------------------------------")
    for name, params, ret, _body in MODELS:
        op(f"  op op_{name}({params}) -> {ret}")
    op("  op op_mixed(a: U8, n: Nat | n < 2, d: Bytes<0>, f: Bool) -> (U8,Nat,Bytes<0>,Bool)")
    op("  op op_zero(d: Bytes<0>) -> Result<(Bool,Nat),TupleError>")
    op("  op op_flip(r: Result<(Bool,Bool),TupleError>) -> Result<(Bool,Bool),TupleError>")
    op("  op op_inc(a: U8) -> U8")
    op("  op op_small(a: U8 | a < 100u8) -> U8")
    op("  op op_bytes(d: Bytes<2>) -> Result<Bytes<2>,TupleError>")
    op("  op op_bits(d: Bytes<2>) -> Bytes<2>")
    op("  op op_u8(a: U8) -> U8")
    w("  assume platform:")
    w("    none")
    w("  assume environment:")
    w("    none")
    for name, params in OPS:
        if params:  # a labels block needs at least one row
            w(f"  labels {name}:")
            for prm in params:
                w(f"    {prm} secret")
    for name, params in OPS:
        for prm in params:
            w(f"  input {name}.{prm}:")
            w("    hostile")
            w("    ensures no_panic, terminates")
    for name, _ in OPS:
        w(f"  bound {name}:")
        w("    steps <= 64")
        w("    heap <= 0")
        w("    depth <= 8")
    w("")
    w("  -- claims: one reference claim per model (op_<m>(args) == m(args)) ------------------------")
    for name, params, _ret, _body in MODELS:
        args = ", ".join(_names(params))
        call = f"op_{name}({args}) == {name}({args})"
        w(f"  claim c_{name}:")
        w(f"    forall {params}. {call}" if params else f"    {call}")
    w("  claim c_mixed:")
    w("    forall a: U8, n: Nat | n < 2, d: Bytes<0>, f: Bool. op_mixed(a, n, d, f) == (a, n, d, f)")
    w("  claim c_pair_literal:")
    w("    forall a: Bool. op_pair(a, 1) == (a, 1)")
    w("  claim c_swap_involutive:")
    w("    forall t: (Bool,Bool). op_swap(op_swap(t)) == t")
    w("  claim c_u8_bool_ne:")
    w("    forall a: (Bool,Bool), b: (Bool,Bool). op_tup_eq(a, b) != tup_ne(a, b)")
    w("  claim c_u8_bool:")
    w("    forall t: (U8,Bool). op_inc(t.0) == t.0 + 1u8")
    w("")
    w("  -- claims: match on a Result.  Pure scrutinee, evaluated in the expected side -------------")
    w("  claim c_ok_and_inline:")
    w("    forall r: Result<(Bool,Bool),TupleError>. op_m_ok_tuple(r) == (match r with Ok((a, b)) -> a and b | Err(_) -> false)")
    w("  claim c_flip:")
    w("    forall r: Result<(Bool,Bool),TupleError>. op_flip(r) == m_returns_result(r)")
    w("  claim c_flip_inline:")
    w("    forall r: Result<(Bool,Bool),TupleError>. op_flip(r) == (match r with Ok((a, b)) -> Ok((b, a)) | Err(_) -> Err(TupleError.Bad))")
    w("  claim c_inc_in_arm:")
    w("    forall r: Result<U8,TupleError>. op_inc(match r with Ok(x) -> x | Err(_) -> 0u8) == m_arith_arm(r)")
    w("  claim c_admit_match:")
    w("    forall r: Result<(Bool,Bool),TupleError> | match r with Ok((a, b)) -> a or b | Err(_) -> true. op_m_ok_tuple(r) == m_ok_tuple(r)")
    w("")
    w("  -- claims: a subject call as the scrutinee, and a subject call inside an arm (pure scrutinee) -")
    w("  claim c_k_match:")
    w("    forall a: Bool, b: Bool. match op_k(a, b) with Ok((x, y)) -> x == a and y == b | Err(TupleError.Worse) -> a and b | Err(_) -> false")
    w("  claim c_zero:")
    w("    forall d: Bytes<0>. op_zero(d) == Err(TupleError.Bad)")
    w("  claim c_arm_calls:")
    w("    forall r: Result<U8,TupleError>. match r with Ok(x) -> op_inc(x) == x + 1u8 | Err(_) -> true")
    w("  claim c_arm_admit_call:")
    w("    forall r: Result<U8,TupleError> | match r with Ok(x) -> x < 100u8 | Err(_) -> true. match r with Ok(x) -> op_small(x) == x | Err(_) -> true")
    w("")
    w("  -- claims for the faults of the Bytes forms ------------------------------------------------")
    w("  claim c_bytes:")
    w("    forall d: Bytes<2>. op_bytes(d) == (if len(d) == 2 then Ok(d) else Err(TupleError.Bad))")
    w("  claim c_bits:")
    w("    forall d: Bytes<2>. op_bits(d) == d")
    w("  claim c_u8:")
    w("    forall a: U8. op_u8(a) == a")
    w("")
    w("  -- examples: tuple values in TFV (L<n>), eq-shaped; the two match-shaped ones are omitted ----")
    w("  example e_pair:")
    w("    op_pair(true, 3) == (true, 3)")
    w("  example e_lit2:")
    w("    op_lit2(true, false) == (true, false)")
    w("  example e_nested:")
    w("    op_nested(true, false, true) == ((true, false), true)")
    w("  example e_tuple_arg:")
    w("    op_swap((true, false)) == (false, true)")
    w("  example e_k_ok:")
    w("    op_k(true, false) == Ok((true, false))")
    w("  example e_k_err:")
    w("    op_k(true, true) == Err(TupleError.Worse)")
    w("  example e_dec:")
    w("    op_dec(true, 7) == Ok((true, 7))")
    w("  example e_result_arg:")
    w("    op_flip(Ok((true, false))) == Ok((false, true))")
    w("  example e_match_ok:")
    w("    match op_k(true, false) with Ok((a, b)) -> a and not b | Err(_) -> false")
    w("  example e_match_err:")
    w("    match op_k(true, true) with Ok(_) -> false | Err(TupleError.Worse) -> true | Err(_) -> false")
    w("")
    return "\n".join(out) + "\n"


# -- corpus errors: every refusal of VER04r Appendix A.1, as (name, document body, oracle code) ----
_BASE = (
    "warrant refusals v1\n"
    "  error E = A | B | C\n"
    "  op f(a: Bool) -> Result<(Bool,Bool),E>\n"
    "  op g(a: Bool) -> Result<Bool,E>\n"
)

#: ``(name, claim or model text, expected oracle code)``; each text is appended to ``_BASE``.
REFUSALS = [
    (
        "fall-through-missing-err",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true\n",
        "corpus",
    ),
    (
        "fall-through-missing-case",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(E.A) -> false | Err(E.B) -> false\n",
        "corpus",
    ),
    (
        "fall-through-missing-ok",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Err(_) -> false\n",
        "corpus",
    ),
    (
        "fall-through-partial-tuple",
        "  model m(r: Result<(Bool,Bool),E>) -> Bool = match r with Ok((a, b)) -> a | Err(_) -> false | Ok(_) -> true\n",
        "corpus",
    ),
    (
        "unreachable-after-wild",
        "  model m(r: Result<Bool,E>) -> Bool = match r with _ -> true | Ok(_) -> false\n",
        "corpus",
    ),
    (
        "unreachable-after-err-any",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(_) -> false | Err(E.A) -> true\n",
        "corpus",
    ),
    (
        "unreachable-duplicate-case",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(E.A) -> false | Err(E.A) -> true | Err(_) -> false\n",
        "corpus",
    ),
    (
        "unreachable-after-all-cases",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(E.A) -> false | Err(E.B) -> false | Err(E.C) -> false | Err(_) -> true\n",
        "corpus",
    ),
    (
        "unreachable-tuple-after-bind",
        "  model m(r: Result<(Bool,Bool),E>) -> Bool = match r with Ok(t) -> true | Ok((a, b)) -> a | Err(_) -> false\n",
        "corpus",
    ),
    (
        "duplicate-bind",
        "  model m(r: Result<(Bool,Bool),E>) -> Bool = match r with Ok((a, a)) -> a | Err(_) -> false\n",
        "corpus",
    ),
    (
        "duplicate-bind-nested",
        "  model m(r: Result<((Bool,Bool),Bool),E>) -> Bool = match r with Ok(((a, b), a)) -> a | Err(_) -> false\n",
        "corpus",
    ),
    (
        "bind-shadows-a-parameter",
        "  model m(a: Bool, r: Result<Bool,E>) -> Bool = match r with Ok(a) -> a | Err(_) -> false\n",
        "corpus",
    ),
    (
        "bind-shadows-a-binder",
        "  claim c:\n    forall a: Bool. g(a) == (match g(a) with Ok(a) -> Ok(a) | Err(_) -> Err(E.A))\n",
        "corpus",
    ),
    (
        "bind-shadows-result",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(result) -> result | Err(_) -> false\n",
        "corpus",
    ),
    (
        "bind-shadows-an-outer-bind",
        "  model m(r: Result<Result<Bool,E>,E>) -> Bool = match r with Ok(x) -> (match x with Ok(x) -> x | Err(_) -> false) | Err(_) -> false\n",
        "corpus",
    ),
    (
        "bind-shadows-the-scrutinee-name",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(r) -> r | Err(_) -> false\n",
        "corpus",
    ),
]

#: Documents the lowerer refuses for another reason (named so the generator can mirror them).
OTHER_REFUSALS = [
    (
        "arms-have-no-common-type",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(_) -> 1\n",
        "type",
    ),
    (
        "a-tuple-pattern-needs-a-tuple",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok((a, b)) -> a | Err(_) -> false\n",
        "type",
    ),
    (
        "a-tuple-pattern-of-the-wrong-arity",
        "  model m(r: Result<(Bool,Bool),E>) -> Bool = match r with Ok((a, b, c)) -> a | Err(_) -> false\n",
        "type",
    ),
    (
        "an-unknown-error-case",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(E.Z) -> false | Err(_) -> true\n",
        "type",
    ),
    (
        "a-projection-index-out-of-range",
        "  model m(t: (Bool,Bool)) -> Bool = t.2\n",
        "type",
    ),
    (
        "a-projection-of-a-non-tuple",
        "  model m(t: Bool) -> Bool = t.0\n",
        "level",
    ),
]


def refusal_documents() -> list[tuple[str, str, str]]:
    return [(n, _BASE + body, code) for n, body, code in [*REFUSALS, *OTHER_REFUSALS]]


def refusal_of(source: str) -> OracleError | None:
    """The oracle's refusal of a document: a raised error, or the first unit recorded ``unsupported``."""
    try:
        spec = lower_document(parse_source(source.encode()))
    except OracleError as exc:
        return exc
    for unit in [*spec.models.values(), *spec.ops.values(), *spec.suites, *spec.examples]:
        if unit.unsupported is not None:
            return unit.unsupported
    return None


def refusals() -> dict:
    """The refusal dump: every document with the oracle's code and message (never echoing input)."""
    rows = []
    for name, source, expected in refusal_documents():
        exc = refusal_of(source)
        assert exc is not None, f"{name} was not refused"
        assert exc.code == expected, (name, exc.code, exc.message)
        rows.append(
            {
                "name": name,
                "oracle_code": exc.code,
                "oracle_level": exc.level or "",
                "message": exc.message,
                "source": source,
                # VER04r A.1: the generator refuses a corpus error with the code `ir`.
                "generator_code": "ir" if exc.code == "corpus" else "",
            }
        )
    return {"schema": "warrant-refusals-parity-v1", "refusals": rows}


def build() -> dict[pathlib.Path, bytes]:
    doc = main().encode("utf-8")
    profile = PROFILE.encode("utf-8")
    return {
        HERE / f"{NAME}.warrant": doc,
        HERE / f"{NAME}.profile": profile,
        HERE / f"{NAME}.oracle-lowering.json": dump(doc),
        HERE / f"{NAME}.oracle-evaluation.json": dump_evaluation(doc, profile),
        HERE / f"{NAME}.oracle-faults.json": dump_faults(doc, profile),
        HERE / f"{NAME}.oracle-refusals.json": canonical(refusals()),
    }


def run(argv: list[str]) -> int:
    files = build()
    if "--check" in argv:
        bad = [p.name for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        for name in bad:
            print(f"DIFFERS: {name}", file=sys.stderr)
        if not bad:
            print(f"ok: {len(files)} files")
        return 1 if bad else 0
    for path, data in files.items():
        path.write_bytes(data)
        print(f"wrote {path.name} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1:]))
