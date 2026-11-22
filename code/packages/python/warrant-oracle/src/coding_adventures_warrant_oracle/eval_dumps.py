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

The data tables of slice S11c-1 (``tests/fixtures/data-eval-v1/``)
--------------------------------------------------------------------
Slice S11c-1 evaluates ``ir.data`` and writes its values, so two more tables join these, for the Rust
generator's S11c-2 to reproduce byte for byte:

``warrant-data-values-parity-v1`` (:func:`data_values_view`)
    one row per value of a record, a variant, an ``Option`` or a ``Result`` of one: its declared type, its
    **JSON** (section 2.3), its **stream** bytes in hex (section 2.4: ``40``/``41`` for an ``Option``, ``64`` for a
    record, ``65`` for a variant) and its **TFV** (section 2.5: ``L0``/``L1 x``, ``L<n> ..``, ``L2 S<ctor> L<k> ..``),
    each read back and compared (the JSON decodes to the value, with the kit's reader), and the JSON forms a consumer
    must **reject** with the reader's message;
``warrant-data-domain-parity-v1`` (:func:`data_domain_view`)
    the descriptors of data binder types (``tuple`` for a record, ``variant``, ``option``, ``result``), their sizes
    and values, the budget **demotions** with the path text of section 5.3 [D54] (``0.0``, ``0.Done.1.some``,
    ``0.some``, ``0.ok.0``) and the descriptors a consumer must accept or reject.
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
DATA_VALUES_SCHEMA = "warrant-data-values-parity-v1"
DATA_DOMAIN_SCHEMA = "warrant-data-domain-parity-v1"
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


# -- the data tables of slice S11c-1 ----------------------------------------------------------------------------


def _doc_spec(spec_bytes: bytes):
    """The lowered document and the section 2.2 ``types`` object the kit's reader works from."""
    from .corpus_v2 import _types
    from .lower import lower_document
    from .parser import parse_source

    spec = lower_document(parse_source(spec_bytes))
    return spec, _types(spec)


#: Values written in the data table: ``(type, value)``, the oracle's own Python values (a record is a tuple of its
#: fields, a variant ``(ctor, fields)``, an ``Option`` ``("none",)`` or ``("some", v)``).  The two documents of
#: the fixture declare every type named here.
DATA_VALUES = [
    ("Option<U8>", ("none",)),
    ("Option<U8>", ("some", 7)),
    ("Option<Bool>", ("some", False)),
    ("Option<Option<Bool>>", ("some", ("none",))),
    ("Option<Option<Bool>>", ("some", ("some", True))),
    ("Option<(Bool,Nat)>", ("some", (True, 4))),
    ("Option<Bytes>", ("some", b"\x01\xff")),
    ("Point", (1, 2)),
    ("Point", (255, 0)),
    ("Wrapped", ((1, 2), True)),
    ("Slot", (("some", 9), (True, 2))),
    ("Slot", (("none",), (False, 0))),
    ("Wide", (65535, 3)),
    ("Sum", (b"\x01", 3, True)),
    ("Holder", (b"\x00\xff", (b"\xff", True))),
    ("Shape", ("Circle", (3,))),
    ("Shape", ("Rect", (3, 4))),
    ("Shape", ("Dot", ())),
    ("Dir", ("Both", (True, False))),
    ("Msg", ("Short", ())),
    ("Msg", ("Done", (5, ("none",)))),
    ("Msg", ("Done", (5, ("some", 65535)))),
    ("Msg", ("Opt", (("some", 1),))),
    ("Result<Sum,DataError>", ("ok", (b"", 0, False))),
    ("Result<Sum,DataError>", ("err", "Bad")),
    ("Result<Option<U8>,DataError>", ("ok", ("none",))),
    ("Result<Option<U8>,DataError>", ("ok", ("some", 7))),
    ("Result<Dir,DataError>", ("ok", ("Down", (True,)))),
]

#: JSON a consumer must refuse (section 2.3): ``(type, json)``.
DATA_REJECTED = [
    ("Option<U8>", {"none": False}),
    ("Option<U8>", {}),
    ("Option<U8>", {"none": True, "some": "1"}),
    ("Option<U8>", {"some": 1}),
    ("Option<U8>", {"some": "256"}),
    ("Option<U8>", None),
    ("Option<U8>", "none"),
    ("Point", ["1"]),
    ("Point", ["1", "2", "3"]),
    ("Point", {"x": "1", "y": "2"}),
    ("Point", ["1", "256"]),
    ("Wrapped", [["1", "2"], 1]),
    ("Shape", {"ctor": "Circle"}),
    ("Shape", {"ctor": "Circle", "fields": ["3"], "extra": 1}),
    ("Shape", {"ctor": "Square", "fields": ["3"]}),
    ("Shape", {"ctor": "Circle", "fields": []}),
    ("Shape", {"ctor": "Circle", "fields": ["3", "4"]}),
    ("Shape", {"ctor": "Circle", "fields": [3]}),
    ("Shape", {"ctor": "Dot", "fields": ["0"]}),
    ("Shape", ["Circle", ["3"]]),
    ("Msg", {"ctor": "Done", "fields": ["5", {"some": "65536"}]}),
    ("Result<Sum,DataError>", {"ok": ["", "0"]}),
    ("Result<Sum,DataError>", {"err": "Missing"}),
]


def data_values_view(spec_bytes: bytes) -> dict:
    """The value table of the data kinds (see the module docstring): JSON, stream and TFV of each value."""
    from . import tfv
    from .conformance import Refused, decode_value, parse_type
    from .corpus_v2 import value_json
    from .stream import make_encoder

    spec, types = _doc_spec(spec_bytes)
    rows = []
    for text, value in DATA_VALUES:
        ty = parse_type(text, types)
        as_json = value_json(value, ty, spec)
        # The kit's reader (an independent implementation of section 2.3) reads the JSON back to the value.
        assert decode_value(as_json, ty, types) == value, text
        rows.append(
            {
                "type": text,
                "json": as_json,
                "stream": make_encoder(spec, ty)(value).hex(),
                "tfv": tfv.value(value, ty, spec),
            }
        )
    rejected = []
    for text, raw in DATA_REJECTED:
        try:
            decode_value(raw, parse_type(text, types), types)
        except Refused as exc:
            rejected.append({"type": text, "json": raw, "oracle_code": exc.code, "message": exc.why})
        else:  # pragma: no cover - a row of the table that is accepted is a bug of the table
            raise AssertionError(f"{text} {raw!r} was accepted")
    return {
        "schema": DATA_VALUES_SCHEMA,
        "format": (
            "type, json (section 2.3), stream (section 2.4, hex), tfv (section 2.5); a record is an array, an Option "
            "{none:true} or {some:v}, a variant {ctor, fields}; stream 40/41 Option, 64 record, 65 variant"
        ),
        "values": rows,
        "rejected": rejected,
    }


def _values_of(desc, spec, ty: A.Ty, indexes: list[int]) -> list:
    from .corpus_v2 import value_json

    return [value_json(desc.nth(i), ty, spec) for i in indexes]


#: The binder types whose descriptors the domain table writes with their values: small ones in full, large ones
#: by their first and last four values.
DATA_DOMAINS = [
    "Option<Bool>",
    "Option<Option<Bool>>",
    "Option<(Bool,Nat)>",
    "Duo",
    "Tri",
    "Dir",
    "Mode",
    "Shape",
    "Point",
    "Slot",
    "Result<Option<Bool>,DataError>",
    "Result<Dir,DataError>",
]

#: Budget demotions: each row is a list of binder types, demoted by the profile's budget (section 5.3, [D54]).
DATA_DEMOTIONS = [
    ["Wide"],
    ["Msg"],
    ["Option<U16>", "Nat"],
    ["Nat", "U16"],
    ["Result<Wide,DataError>"],
    ["Dir", "Slot"],
]

#: Descriptors a consumer must accept and reject (section 5.1): data nodes only.
DATA_ACCEPTED = [
    {"d": "option", "of": {"d": "bool", "size": 2}, "size": 3},
    {"d": "tuple", "of": [{"d": "u8_all", "size": 256}, {"d": "u8_all", "size": 256}], "size": 65536},
    {
        "d": "variant",
        "ctors": [["Up", []], ["Down", [{"d": "bool", "size": 2}]], ["Both", [{"d": "bool", "size": 2}, {"d": "bool", "size": 2}]]],
        "size": 7,
    },
    {
        "d": "variant",
        "ctors": [["Short", []], ["Done", [{"d": "nat_0_64", "size": 65}, {"d": "option", "of": {"d": "fx_edge", "t": "U16", "size": 281}, "size": 282}]]],
        "size": 18331,
    },
    {"d": "result", "ok": {"d": "option", "of": {"d": "bool", "size": 2}, "size": 3}, "errs": ["Bad", "Worse"], "size": 5},
]
DATA_REJECTED_DESCRIPTORS = [
    {"d": "option", "of": {"d": "bool", "size": 2}, "size": 2},
    {"d": "option", "size": 3},
    {"d": "option", "of": {"d": "bool", "size": 2}, "size": 3, "extra": 1},
    {"d": "tuple", "of": [], "size": 1},
    {"d": "tuple", "of": [{"d": "bool", "size": 2}], "size": 3},
    {"d": "variant", "ctors": [], "size": 0},
    {"d": "variant", "ctors": [["Up", []], ["Down", [{"d": "bool", "size": 2}]]], "size": 2},
    {"d": "variant", "ctors": [["Up", []], ["Down"]], "size": 2},
    {"d": "variant", "ctors": [["Up", []], ["Down", [{"d": "fx_edge", "t": "U8", "size": 256}]]], "size": 257},
    {"d": "variant", "of": [], "size": 1},
    {"d": "result", "ok": {"d": "bool", "size": 2}, "errs": ["Bad"], "size": 2},
]


def data_domain_view(spec_bytes: bytes, profile_raw: bytes) -> dict:
    """The domain table of the data kinds (see the module docstring)."""
    from .conformance import parse_type
    from .domains import demote, descriptor_for
    from .profile import parse_profile

    spec, types = _doc_spec(spec_bytes)
    budget = parse_profile(profile_raw).budget
    leaves = {}
    for text in DATA_DOMAINS:
        ty = parse_type(text, types)
        desc = descriptor_for(spec, ty)
        size = desc.size()
        row = {"descriptor": desc.to_json(), "label": desc.label, "size": size}
        if size <= 16:
            row["values"] = _values_of(desc, spec, ty, list(range(size)))
        else:
            row["head"] = _values_of(desc, spec, ty, list(range(4)))
            row["tail"] = _values_of(desc, spec, ty, list(range(size - 4, size)))
        leaves[text] = row
    demotions = []
    for binders in DATA_DEMOTIONS:
        descs = [descriptor_for(spec, parse_type(t, types)) for t in binders]
        before = [d.size() for d in descs]
        paths = demote(descs, budget)
        demotions.append(
            {
                "binders": binders,
                "budget": budget,
                "sizes_before": before,
                "demoted": paths,
                "descriptors": [d.to_json() for d in descs],
                "sizes": [d.size() for d in descs],
            }
        )
    return {
        "schema": DATA_DOMAIN_SCHEMA,
        "domains": leaves,
        "demotions": demotions,
        "consumer": [_verdict(d) for d in [*DATA_ACCEPTED, *DATA_REJECTED_DESCRIPTORS]],
    }
