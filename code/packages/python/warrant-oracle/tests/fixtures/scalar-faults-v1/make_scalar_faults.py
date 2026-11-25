#!/usr/bin/env python3
"""Write (or ``--check``) the synthetic scalar-faults document of slice S11f-1 and its dumps.

VER04r 8.2 (16.14) adds to the fault registry ``saturate-add``, ``saturate-sub``, ``shift-mask`` and a
``missing-error`` on a scalar payload (``Bool``, ``Nat`` or a fixed-width type) and on a ``Bytes`` payload with no
``Bytes`` parameter.  Section 8.2 fixes their JSON, so this directory is the oracle's committed proof that the
registry reproduces it; the generator's slice S11f-2 must reproduce every file below byte for byte.

It is a directory of its own (the generator's parity tests reproduce the dumps of the older directories byte for
byte, so a new fault row there would turn them red until the mirror landed).

Four files are written beside this script (plus ``README.md`` and ``.gitattributes``, which are not generated):

``scalar_faults_v1.warrant``
    the document (this script's text): for each of the eight fixed-width types a bare saturating op ``sat_<t>``
    and a ``Result`` one ``satr_<t>`` (for four of them, with a third, ignored parameter), shift ops of every signedness and
    width combination, and the scalar ``missing-error`` shapes (a parameter of the exact type, a zero);
``scalar_faults_v1.profile``
    the ``corpus.profile`` the dump uses (one ``fault`` line per op and fault);
``scalar_faults_v1.oracle-faults.json``
    the fault dump (``parity_dumps.faults_view``): every fault row (the JSON of section 8.2) and the controls under
    the explicit closed gate and the open gate;
``scalar_faults_v1.oracle-refusals.json``
    every signature each fault refuses, with the oracle's code and message.

    python3 make_scalar_faults.py            # (re)write the files (--write is the same)
    python3 make_scalar_faults.py --check    # fail if any committed file differs
"""

from __future__ import annotations

import pathlib
import sys

from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import fault_row
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parity_dumps import dump_faults
from coding_adventures_warrant_oracle.parser import parse_source

HERE = pathlib.Path(__file__).resolve().parent
NAME = "scalar_faults_v1"

TYPES = ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"]
#: The ``Result`` form is the same expression wrapped in ``["ok", _]`` whatever ``T`` is, so it is dumped for the
#: narrowest and the widest of each signedness only (the dump's run time grows with every op).
RESULT_TYPES = ["U8", "U64", "I8", "I64"]

#: ``(op name, parameters, result type, model body, faults)``.  Every op has a pure model ``m_<op>`` over the same
#: parameters, so a reference claim ``c_<op>`` (``op(args) == m_op(args)``) is a stand-in the witness search can
#: evaluate.  The saturating ops return their first operand (the model is the identity on it), the shift ops
#: return the shifted operand itself (so the rewrite of the last parameter is visible), and the ``missing-error``
#: ops answer with the value the fault would put in the ``Ok``.
OPS: list[tuple[str, str, str, str, list[str]]] = []
for _t in TYPES:
    _l = _t.lower()
    OPS.append((f"sat_{_l}", f"a: {_t}, b: {_t}", _t, "a", ["saturate-add", "saturate-sub"]))
    if _t in RESULT_TYPES:
        OPS.append((f"satr_{_l}", f"a: {_t}, b: {_t}, c: Bool", f"Result<{_t},E>", "Ok(a)", ["saturate-add", "saturate-sub"]))
OPS += [
    # the width is the FIRST parameter's, the rewritten parameter is the LAST, of its own type and signedness
    ("sh_u8", "a: U8, n: U8", "U8", "n", ["shift-mask"]),
    ("sh_u32_by_u8", "a: U32, n: U8", "U8", "n", ["shift-mask"]),
    ("sh_i16_by_u16", "a: I16, n: U16", "U16", "n", ["shift-mask"]),
    ("sh_u8_by_i8", "a: U8, n: I8", "I8", "n", ["shift-mask"]),
    ("sh_i64_by_i32", "a: I64, n: I32", "I32", "n", ["shift-mask"]),
    ("sh_u64_by_u64", "a: U64, n: U64", "U64", "n", ["shift-mask"]),
    ("sh_only", "a: U16", "U16", "a", ["shift-mask"]),
    # scalar missing-error: a parameter of exactly the payload type, or the type's zero
    ("me_u8", "a: U8, b: U32", "Result<U8,E>", "if b == 0u32 then Err(E.Bad) else Ok(a)", ["missing-error"]),
    ("me_u16_zero", "a: U8", "Result<U16,E>", "if a == 0u8 then Err(E.Bad) else Ok(1u16)", ["missing-error"]),
    ("me_i32", "a: I32, b: U32", "Result<I32,E>", "if b == 0u32 then Err(E.Bad) else Ok(a)", ["missing-error"]),
    ("me_bool", "a: U8, f: Bool", "Result<Bool,E>", "if a == 0u8 then Err(E.Bad) else Ok(f)", ["missing-error"]),
    ("me_bool_zero", "a: U8", "Result<Bool,E>", "if a == 0u8 then Err(E.Bad) else Ok(true)", ["missing-error"]),
    ("me_nat", "a: Bool, n: Nat | n < 8", "Result<Nat,E>", "if a then Err(E.Bad) else Ok(n)", ["missing-error"]),
    ("me_nat_zero", "a: Bool", "Result<Nat,E>", "if a then Err(E.Bad) else Ok(1)", ["missing-error"]),
    ("me_bytes4_zero", "a: U8", "Result<Bytes<4>,E>", "if a == 0u8 then Err(E.Bad) else Ok(b\"01010101\")", ["missing-error"]),
    ("me_bytes_zero", "a: U8", "Result<Bytes,E>", "if a == 0u8 then Err(E.Bad) else Ok(b\"01\")", ["missing-error"]),
    # R9: a tuple parameter is a type that needs ir.tuple, so the row's `requires` is ir.fixed + ir.tuple
    ("sat_u8_tuple_param", "a: U8, b: U8, p: (Bool,Bool)", "U8", "a", ["saturate-add"]),
]

#: A second, restricted claim on an op, as ``off-by-one-consumed-v1`` does: ``me_u8`` also claims its ``Err`` region
#: (``b == 0``, where the model answers ``Err(E.Bad)``), a claim whose stand-in expects ``Err`` and so *witnesses* the
#: fault too; the control then has two required claims, not one.
EXTRA_CLAIMS = {
    "me_u8": [("err", "a: U8, b: U32 | b == 0u32", "me_u8(a, b) == Err(E.Bad)")],
}


def _param_names(params: str) -> list[str]:
    """The parameter names of ``a: Bool, n: Nat | n < 8`` (a ``|`` guard follows its binder)."""
    names = []
    for part in params.split(", "):
        head = part.split(":")[0].strip()
        if ":" in part and head.isidentifier():
            names.append(head)
    return names


def _plain(params: str) -> str:
    return params.replace(" | n < 8", "")


PROFILE = (
    "budget 16777216\nchunk_size 16384\nslim_stride auto\n"
    + "".join(f"fault {name} {fault}\n" for name, _p, _r, _b, faults in OPS for fault in faults)
)


def document() -> str:
    out = [
        "warrant scalar_faults v1",
        "  -- Synthetic document of slice S11f-1 (VER04r 8.2): the shapes `saturate-add`, `saturate-sub`, `shift-mask`",
        "  -- and the scalar `missing-error` apply to.  GENERATED by make_scalar_faults.py; do not edit by hand.",
        "  error E = Bad | Worse",
        "",
    ]
    for name, params, ret, body, _f in OPS:
        out.append(f"  model m_{name}({_plain(params)}) -> {ret} = {body}")
    out.append("")
    for name, params, ret, _b, _f in OPS:
        out.append(f"  op {name}({params}) -> {ret}")
    out += ["  assume platform:", "    none", "  assume environment:", "    none"]
    names = [(name, _param_names(params)) for name, params, *_ in OPS]
    for name, prms in names:
        out.append(f"  labels {name}:")
        out += [f"    {p} secret" for p in prms]
    for name, prms in names:
        for p in prms:
            out += [f"  input {name}.{p}:", "    hostile", "    ensures no_panic, terminates"]
    for name, _ in names:
        out += [f"  bound {name}:", "    steps <= 64", "    heap <= 0", "    depth <= 8"]
    out.append("")
    for name, params, *_ in OPS:
        args = ", ".join(_param_names(params))
        out += [f"  claim c_{name}:", f"    forall {params}. {name}({args}) == m_{name}({args})"]
        for suffix, eparams, expr in EXTRA_CLAIMS.get(name, []):
            out += [f"  claim c_{name}_{suffix}:", f"    forall {eparams}. {expr}"]
    out.append("")
    return "\n".join(out) + "\n"


#: Signatures a fault refuses: ``(name, op declaration)``; each is a ``generation`` error whose message starts
#: ``fault:`` (the generator's code ``fault``, nothing skipped).  The same list is tried against both saturating
#: faults; ``shift-mask`` and ``missing-error`` have lists of their own.
SATURATE_REFUSALS = [
    ("a-nat-first-parameter", "op f(a: Nat, b: U8) -> U8"),
    ("a-nat-second-parameter", "op f(a: U8, b: Nat) -> U8"),
    ("two-nat-parameters-and-a-nat-result", "op f(a: Nat, b: Nat) -> Nat"),
    ("two-different-widths", "op f(a: U8, b: U16) -> U8"),
    ("two-different-signs", "op f(a: U8, b: I8) -> U8"),
    ("a-single-parameter", "op f(a: U8) -> U8"),
    ("no-parameter", "op f() -> U8"),
    ("two-bool-parameters", "op f(a: Bool, b: Bool) -> Bool"),
    ("two-bytes-parameters", "op f(a: Bytes, b: Bytes) -> Bytes"),
    ("a-bool-result", "op f(a: U8, b: U8) -> Bool"),
    ("a-nat-result", "op f(a: U8, b: U8) -> Nat"),
    ("a-result-of-another-width", "op f(a: U8, b: U8) -> U16"),
    ("an-option-result", "op f(a: U8, b: U8) -> Option<U8>"),
    ("a-bytes-result", "op f(a: U8, b: U8) -> Bytes"),
    ("a-tuple-result", "op f(a: U8, b: U8) -> (U8,U8)"),
    ("a-result-of-a-bool", "op f(a: U8, b: U8) -> Result<Bool,E>"),
    ("a-result-of-another-width-payload", "op f(a: U8, b: U8) -> Result<U16,E>"),
    ("a-result-of-a-tuple", "op f(a: U8, b: U8) -> Result<(U8,U8),E>"),
    ("a-result-of-bytes", "op f(a: U8, b: U8) -> Result<Bytes,E>"),
]

SHIFT_REFUSALS = [
    ("a-nat-first-parameter", "op f(a: Nat, n: U8) -> U8"),
    ("a-nat-last-parameter", "op f(a: U8, n: Nat) -> U8"),
    ("a-single-nat-parameter", "op f(n: Nat) -> U8"),
    ("a-bool-last-parameter", "op f(a: U8, n: Bool) -> U8"),
    ("a-bool-first-parameter", "op f(a: Bool, n: U8) -> U8"),
    ("a-bytes-last-parameter", "op f(a: U8, n: Bytes) -> U8"),
    ("a-bytes-first-parameter", "op f(a: Bytes, n: U8) -> U8"),
    ("no-parameter", "op f() -> U8"),
]

MISSING_ERROR_REFUSALS = [
    ("a-scalar-result", "op f(a: U8) -> U8"),
    ("a-bool-result", "op f(a: U8) -> Bool"),
    ("a-bytes-result", "op f(a: U8) -> Bytes"),
    ("a-variant-payload", "variant K = P | Q\n  op f(a: U8) -> Result<K,E>"),
    ("an-option-result", "op f(a: U8) -> Option<U8>"),
    ("a-named-width-bytes-payload-with-no-bytes-parameter", "op f<N: Nat>(a: U8) -> Result<Bytes<N>,E>"),
]

CASES = [
    ("saturate-add", SATURATE_REFUSALS),
    ("saturate-sub", SATURATE_REFUSALS),
    ("shift-mask", SHIFT_REFUSALS),
    ("missing-error", MISSING_ERROR_REFUSALS),
]


def refusals() -> dict:
    """Every refused signature with the oracle's code and message (never echoing input).

    Schema ``warrant-faults-refusals-v2``: ``{"schema", "faults": [names], "refusals": [row, ..]}`` where a row is
    ``{"fault", "name", "oracle_code", "message", "source"}``.  (``-v1``, in ``off-by-one-consumed-v1``, has one
    top-level ``fault`` string and no per-row ``fault``.)"""
    rows = []
    for fault, cases in CASES:
        for name, decl in cases:
            source = f"warrant refusals v1\n  error E = A | B\n  {decl}\n"
            spec = lower_document(parse_source(source.encode()))
            try:
                fault_row(
                    spec.ops["f"], fault, spec.errors, frozenset([*spec.records, *spec.variants]), dict(spec.records)
                )
            except OracleError as exc:
                rows.append(
                    {"fault": fault, "name": name, "oracle_code": exc.code, "message": exc.message, "source": source}
                )
            else:
                raise AssertionError(f"{fault}: {name} was not refused")
    return {
        "schema": "warrant-faults-refusals-v2",
        "faults": [f for f, _ in CASES],
        "refusals": rows,
    }


def build() -> dict[pathlib.Path, bytes]:
    doc = document().encode("utf-8")
    profile = PROFILE.encode("utf-8")
    return {
        HERE / f"{NAME}.warrant": doc,
        HERE / f"{NAME}.profile": profile,
        HERE / f"{NAME}.oracle-faults.json": dump_faults(doc, profile, f"{NAME}.warrant", "fixed"),
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
