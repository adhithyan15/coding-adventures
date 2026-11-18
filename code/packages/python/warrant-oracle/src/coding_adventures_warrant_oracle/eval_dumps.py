"""The committed *value* and *domain* tables of slice S10c-1 (VER04r sections 5.1, 13, 16.10).

Slice S10c-1 evaluates the 35 ``ir.fixed`` built-ins and resolves the domains of [D21], [D25] and
[D26].  The Rust generator's S10c-2 must reproduce all of it **byte for byte**, so the oracle commits
two tables beside the evaluation and fault dumps of :mod:`.parity_dumps`
(``tests/fixtures/fixed-eval-v1/``):

``warrant-fixed-values-parity-v1`` (:func:`values_view`)
    one line per evaluation of a built-in on edge values, at **every** width and signedness: every
    built-in of Appendix A.1, every width, the edge values of VER04j 2.8 and the count rows
    ``{0, 1, w-1, w, w+1, 2w, 255, 65535, 65536}`` of 3.8.  A row is a string::

        <tag> <T> <operand>.. = <result>          bit_and U8 255 15 = 15
        <tag> <S>-><D> <operand> = <result>       wrap U8->I8 255 = -1   (also try, widen)
        <tag> <T,T> <operand>.. = <result>        div Int,Nat -7 2 = some -4   (Int/Nat operands)

    an integer is decimal, a ``Bool`` is ``true``/``false``, ``Bytes`` is ``x`` and lowercase hex
    (``x`` alone is the empty string), an ``Option`` is ``none`` or ``some <v>``.  The rows are what the
    oracle's evaluator computes; the tests hold that evaluator against an independent bit-list
    reference, so the table is a statement of Appendix A.1 and not only of this implementation.
``warrant-domain-parity-v1`` (:func:`domain_view`)
    the resolved leaf domains: the descriptor, label and size of every leaf this slice settles and the
    **full value list** of the ``fx_edge`` leaves (``U16`` 281, ``U32`` 313, ``I16`` 527, ``I32`` 559,
    ``I64`` 623, [D21], [D25]) and of ``nat_shift`` (73, [D26]), values as decimal strings; and the
    descriptors a consumer must **reject** (8-bit ``fx_edge``, an ``fx_edge`` or ``fx_full`` of the wrong
    width, a ``nat_shift`` of the wrong size) with the oracle's message.

Both are canonical JSON (:func:`.corpus_v2.canonical`) and carry no generator name or version.
"""

from __future__ import annotations

from . import ast as A
from .corpus_v2 import canonical
from .domains import (
    descriptor_size,
    leaf_bytes_n,
    leaf_fx_edge,
    leaf_fx_full,
    leaf_nat_count,
    leaf_nat_shift,
    leaf_u64_edge,
)
from .errors import OracleError
from .ir_eval import Program, wrap_to
from .lower import FIXED

VALUES_SCHEMA = "warrant-fixed-values-parity-v1"
DOMAIN_SCHEMA = "warrant-domain-parity-v1"
TYPES = ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"]
NAT, INT, BYTES = A.Ty("Nat"), A.Ty("Int"), A.Ty("Bytes")
FORMAT = (
    "tag type operands = result; integers decimal, Bool true/false, Bytes x<hex>, Option none|some <v>; "
    "a conversion's type is S->D; an Int/Nat operand pair is written Int,Nat"
)


def span(name: str) -> tuple[int, int]:
    bits, signed = FIXED[name]
    return (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)


def sample(name: str) -> list[int]:
    """The edge values of a type: its extremes and their neighbours, 0, 1, the middle and a bit pattern."""
    bits, signed = FIXED[name]
    lo, hi = span(name)
    pattern = wrap_to(int.from_bytes(b"\xa5" * (bits // 8), "big"), bits, signed)
    values = {lo, hi, hi - 1, 0, 1, hi >> 1, pattern}
    values |= {lo + 1, -1} if signed else {2}
    return sorted(values)


def counts(width: int) -> list[int]:
    """The shift-count rows of VER04j 3.8: ``0 1 w-1 w w+1 2w 255 65535 65536``."""
    return sorted({0, 1, width - 1, width, width + 1, 2 * width, 255, 65535, 65536})


def show(v: object) -> str:
    """One value in the row grammar of the module docstring."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, bytes):
        return "x" + v.hex()
    if isinstance(v, tuple) and v[0] == "none":
        return "none"
    if isinstance(v, tuple) and v[0] == "some":
        return "some " + show(v[1])
    raise OracleError("generation", "a value outside the row grammar")  # pragma: no cover


class _Rows:
    """Evaluates built-ins on environments and records one row per evaluation."""

    def __init__(self) -> None:
        self.prog = Program({}, {})
        self.out: list[str] = []

    def row(self, ir: list, tenv: dict, label: str, *operands: object, env: dict | None = None) -> None:
        compiled = self.prog.compile_node(ir, tenv)
        values = env if env is not None else dict(zip(tenv, operands))
        shown = " ".join(show(values[name]) for name in tenv)
        self.out.append(f"{label} {shown} = {show(compiled.fn(values))}")


def value_rows() -> list[str]:
    """Every row of the values table, in a fixed order (type, then built-in, then operands)."""
    rows = _Rows()
    V = lambda n: ["var", n]  # noqa: E731
    for name in TYPES:
        bits, signed = FIXED[name]
        t = A.Ty(name)
        lo, hi = span(name)
        values = sample(name)
        rows.out.append(f"max_value {name} = {hi}")
        rows.out.append(f"min_value {name} = {lo}")
        binary = ["bit_and", "bit_or", "bit_xor", "checked_add", "checked_sub", "checked_mul"]
        binary += ["div_trunc", "rem_trunc"] if signed else ["div", "rem"]
        for tag in binary:
            for a in values:
                for b in values:
                    rows.row([tag, V("a"), V("b")], {"a": t, "b": t}, f"{tag} {name}", a, b)
        unary = ["bit_not", "popcount", "clz", "ctz", "be_bytes", "le_bytes", "to_int"]
        unary += [] if signed else ["to_nat"]
        for tag in unary:
            for a in values:
                rows.row([tag, V("a")], {"a": t}, f"{tag} {name}", a)
        shifts = ["shl", "shr", "rotl", "rotr", "bit"] + (["sar"] if signed else [])
        for tag in shifts:
            for a in values:
                for n in counts(bits):
                    rows.row([tag, V("a"), V("n")], {"a": t, "n": NAT}, f"{tag} {name}", a, n)
        for target in TYPES:
            tlo, thi = span(target)
            inside = tlo <= lo and hi <= thi
            for tag in ("wrap", "try") + (("widen",) if inside else ()):
                for a in values:
                    rows.row([tag, target, V("a")], {"a": t}, f"{tag} {name}->{target}", a)
        octets = [bytes(x & 0xFF for x in range(k, k + bits // 8)) for k in (0, 0x80, 0xF8)]
        octets += [(a & ((1 << bits) - 1)).to_bytes(bits // 8, "big") for a in values]
        width_bytes = A.Ty("Bytes", (bits // 8,))
        for tag in ("from_be", "from_le"):
            for b in octets:
                rows.row([tag, name, V("b")], {"b": width_bytes}, f"{tag} {name}", b)
        # unwrap_or: the default is used exactly when the producer is none
        for a, b in [(hi, 1), (hi, 0), (lo, 1), (0, 0)]:
            for d in (0, hi):
                rows.row(
                    ["unwrap_or", ["checked_add", V("a"), V("b")], V("d")],
                    {"a": t, "b": t, "d": t},
                    f"unwrap_or_checked_add {name}",
                    a,
                    b,
                    d,
                )
    ints = [-(1 << 70), -7, -1, 0, 1, 7, 1 << 70]
    nats = [0, 1, 7, (1 << 64) - 1]
    for tag in ("div", "rem", "imin", "imax"):
        for ta, va in (("Nat", nats), ("Int", ints)):
            for tb, vb in (("Nat", nats), ("Int", ints)):
                for a in va:
                    for b in vb:
                        rows.row(
                            [tag, V("a"), V("b")],
                            {"a": A.Ty(ta), "b": A.Ty(tb)},
                            f"{tag} {ta},{tb}",
                            a,
                            b,
                        )
    for data in (b"", b"\x00", b"\x01\x02\x03"):
        for index in (-1, 0, 1, 2, 3, 1 << 70):
            for kind, ty in (("Int", INT), ("Nat", NAT)):
                if kind == "Nat" and index < 0:
                    continue
                rows.row(["at", V("b"), V("i")], {"b": BYTES, "i": ty}, f"at Bytes,{kind}", data, index)
    return rows.out


def values_view() -> dict:
    return {
        "schema": VALUES_SCHEMA,
        "format": FORMAT,
        "samples": {name: [str(v) for v in sample(name)] for name in TYPES},
        "counts": {str(FIXED[name][0]): counts(FIXED[name][0]) for name in ("U8", "U16", "U32", "U64")},
        "rows": value_rows(),
    }


def dump_values() -> bytes:
    return canonical(values_view())


# -- domains --------------------------------------------------------------------------------------


def _leaf(desc, full: bool) -> dict:
    row = {"descriptor": desc.to_json(), "label": desc.label, "size": desc.size()}
    values = [v.hex() if isinstance(v, bytes) else str(v) for v in map(desc.nth, range(desc.size()))]
    if full:
        row["values"] = values
    else:
        row["head"], row["tail"] = values[:4], values[-4:]
    return row


#: Descriptors a consumer must reject (VER04r 5.1: [D25] no 8-bit ``fx_edge``; the wrong width of
#: ``fx_full``/``fx_edge``; a wrong size, which includes the withdrawn 377-value signed set).
REJECTED = [
    {"d": "fx_edge", "t": "U8", "size": 256},
    {"d": "fx_edge", "t": "I8", "size": 256},
    {"d": "fx_edge", "t": "U64", "size": 377},
    {"d": "fx_edge", "t": "I16", "size": 377},
    {"d": "fx_edge", "t": "I64", "size": 377},
    {"d": "fx_edge", "t": "U32", "size": 314},
    {"d": "fx_full", "t": "U8", "size": 256},
    {"d": "fx_full", "t": "U32", "size": 4294967296},
    {"d": "nat_shift", "size": 72},
    {"d": "nat_shift", "size": 73, "k": 64},
    {"d": "nat_count", "k": 64, "size": 73},
]
ACCEPTED = [
    {"d": "fx_edge", "t": "U16", "size": 281},
    {"d": "fx_edge", "t": "I16", "size": 527},
    {"d": "fx_edge", "t": "I32", "size": 559},
    {"d": "fx_edge", "t": "I64", "size": 623},
    {"d": "nat_shift", "size": 73},
    {"d": "tuple", "of": [{"d": "nat_shift", "size": 73}, {"d": "bool", "size": 2}], "size": 146},
]


def _verdict(descriptor: dict) -> dict:
    try:
        size = descriptor_size(descriptor)
    except OracleError as exc:
        return {"descriptor": descriptor, "oracle_code": exc.code, "message": exc.message}
    return {"descriptor": descriptor, "accepted": True, "size": size}


def domain_view() -> dict:
    leaves = {
        "fx_edge:U16": _leaf(leaf_fx_edge(A.Ty("U16")), True),
        "fx_edge:U32": _leaf(leaf_fx_edge(A.Ty("U32")), True),
        "fx_edge:I16": _leaf(leaf_fx_edge(A.Ty("I16")), True),
        "fx_edge:I32": _leaf(leaf_fx_edge(A.Ty("I32")), True),
        "fx_edge:I64": _leaf(leaf_fx_edge(A.Ty("I64")), True),
        "nat_shift": _leaf(leaf_nat_shift(), True),
        "nat_0_64": _leaf(leaf_nat_count(64), False),
        "u64_edge": _leaf(leaf_u64_edge(), True),
        "fx_full:I8": _leaf(leaf_fx_full(A.Ty("I8")), False),
        "fx_full:U16": _leaf(leaf_fx_full(A.Ty("U16")), False),
        "fx_full:I16": _leaf(leaf_fx_full(A.Ty("I16")), False),
        "bytes_n:2": _leaf(leaf_bytes_n(2), True),
    }
    return {
        "schema": DOMAIN_SCHEMA,
        "leaves": leaves,
        "consumer": [_verdict(d) for d in [*ACCEPTED, *REJECTED]],
    }


def dump_domains() -> bytes:
    return canonical(domain_view())
