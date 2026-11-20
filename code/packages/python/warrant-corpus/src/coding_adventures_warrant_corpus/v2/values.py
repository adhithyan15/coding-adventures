"""Warrant values: the section 2.3 decoder and the section 2.4 stream encoder.

A value crosses two boundaries and each has its own form.

**Corpus JSON (section 2.3).**  The corpus never tags a value with its own
type: the reader holds the *declared* type and decodes to it, range-checking.
Every integer is a decimal *string* (so a JavaScript reader cannot lose bits),
bytes are lowercase hex, and the composites are:

=================  =========================================================
type               JSON
=================  =========================================================
``Bool``           ``true`` / ``false``
integers, ``Nat``  ``"18446744073709551615"``
``Bytes``          ``"0a0b"``
tuple, record      an array in component / declared field order
``List<T>``        an array, at most 4,096 elements
``Option<T>``      ``{"none":true}`` or ``{"some":v}``
``Result<T,E>``    ``{"ok":v}`` or ``{"err":"CASE"}``
variant            ``{"ctor":"Name","fields":[...]}``
=================  =========================================================

**Python values inside the runner.**

=================  =========================================================
``Bool``           ``bool``
integers, ``Nat``  ``int``
``Bytes``          ``bytes``
tuple, record      ``tuple``
``List<T>``        ``list``
``Option<T>``      ``("none",)`` or ``("some", v)``
``Result<T,E>``    ``("ok", v)`` or ``("err", "CASE")``
variant            ``(ctor_name, (field, ...))``
=================  =========================================================

The Python form is *type-directed*: two values are told apart by the type they
are read at, never by their shape, so ``("ok", 1)`` is unambiguous.

**The stream (section 2.4).**  Digests are over bytes every lane can produce
identically.  The tag byte says what follows::

    Bool          00 | 01
    U8..U64, Nat  10, then 8 bytes little-endian, unsigned
    I8..I64       11, then 8 bytes two's complement little-endian, sign-extended
    Bytes         20, u32-LE length, the bytes
    List<T>       30, u32-LE count, elements
    Option<T>     40 (none) | 41 then the inner value
    Ok(v)/Err(E.V)  50 then inner | 51, u32-LE length, UTF-8 case name
    tuple, record 64, u32-LE component count, components in order
    variant       65, u32-LE name length, UTF-8 name, u32-LE field count, fields

A worked example: the record ``Point(x: U8, y: U8)`` holding ``Point(1, 2)`` is
``64 02000000  10 0100000000000000  10 0200000000000000``; the value ``-1`` as an
``I8`` is ``11 ffffffffffffffff`` -- the *type name* in the stream's context says
it is eight bits, the stream does not.  Nested composites carry a count and a tag
because they must be self-delimiting: without them two different values could
serialise to the same bytes.
"""

from __future__ import annotations

import re
from collections.abc import Callable

from ..errors import CorpusError
from ..jsonio import parse_json
from .errors import Refusal
from .semantics import STANDARD, Semantics
from .wtypes import FIXED, MAX_FIXED_WIDTH, Ty, Value, fixed_range, parse_type

_HEX = re.compile(r"(?:[0-9a-f]{2})*")
MAX_LIST = 4096


def read_json(data: bytes) -> object:
    """Read ``data`` with the strict reader of VER04i 9.1, or refuse ``malformed-json``.

    UTF-8 only, no BOM, no duplicate keys (after unescaping), integers in
    ``+-(2^53 - 1)`` only, no unpaired surrogate escapes, at most 256 nested
    containers.  This is the lane's existing reader (:mod:`..jsonio`).
    """
    try:
        return parse_json(data)
    except CorpusError as error:
        raise Refusal("malformed-json", error.message) from error


# ---------------------------------------------------------------------------
# The types table
# ---------------------------------------------------------------------------
def _record_fields(types: dict, name: str) -> list[Ty]:
    return [parse_type(text, types) for _field, text in types[name]["fields"]]


def _variant_ctors(types: dict, name: str) -> dict[str, list[Ty]]:
    return {
        ctor: [parse_type(text, types) for text in fields]
        for ctor, fields in types[name]["ctors"]
    }


def _kind(ty: Ty, types: dict | None) -> str | None:
    """``record`` / ``variant`` for a named type, else ``None``."""
    if types is None or ty.head not in types:
        return None
    return types[ty.head].get("kind")


# ---------------------------------------------------------------------------
# section 2.3: JSON -> Python
# ---------------------------------------------------------------------------
def decode_value(
    raw: object, ty: Ty, types: dict | None, sem: Semantics = STANDARD
) -> object:
    """Decode the JSON ``raw`` as a value of type ``ty``, or refuse ``value``."""
    types = types or {}
    head = ty.head
    if head == "Bool":
        if type(raw) is not bool:
            raise Refusal("value", "a Bool is a JSON boolean")
        return raw
    if head == "Nat" or head in FIXED:
        return sem.decode_int(raw, ty)
    if head == "Bytes":
        return _decode_bytes(raw, ty)
    if head == "Result":
        return _decode_result(raw, ty, types, sem)
    if head == "Option":
        if raw == {"none": True} and type(raw) is dict:
            return ("none",)
        if type(raw) is dict and len(raw) == 1 and "some" in raw:
            return ("some", decode_value(raw["some"], ty.args[0], types, sem))
        raise Refusal("value", "an Option is {none: true} or {some: v}")
    if head == "List":
        if type(raw) is not list or len(raw) > MAX_LIST:
            raise Refusal("value", "a List is an array of at most 4096 elements")
        return [decode_value(item, ty.args[0], types, sem) for item in raw]
    if head == "()":
        return _decode_sequence(raw, list(ty.args), types, sem)
    kind = _kind(ty, types)
    if kind == "record":
        return _decode_sequence(raw, _record_fields(types, head), types, sem)
    if kind == "variant":
        return _decode_variant(raw, ty, types, sem)
    raise Refusal("value", "a value of this type cannot be decoded")


def _decode_bytes(raw: object, ty: Ty) -> bytes:
    if not isinstance(raw, str) or _HEX.fullmatch(raw) is None:
        raise Refusal("value", "Bytes is lowercase even-length hex")
    length = len(raw) // 2
    if length > MAX_FIXED_WIDTH:
        raise Refusal("value", "a Bytes value has at most 4096 bytes")
    if ty.args and isinstance(ty.args[0], int) and length != ty.args[0]:
        raise Refusal("value", "Bytes<N> has exactly N bytes")
    return bytes.fromhex(raw)


def _decode_result(raw: object, ty: Ty, types: dict, sem: Semantics) -> tuple:
    if type(raw) is not dict or len(raw) != 1:
        raise Refusal("value", "a Result has exactly one key")
    if "ok" in raw:
        return ("ok", decode_value(raw["ok"], ty.args[0], types, sem))
    case = raw.get("err")
    declared = types.get(ty.args[1].head, {}).get("cases")
    if not isinstance(case, str) or (declared is not None and case not in declared):
        raise Refusal("value", "an err case is a name the error type declares")
    return ("err", case)


def _decode_sequence(
    raw: object, parts: list[Ty], types: dict, sem: Semantics
) -> tuple:
    if type(raw) is not list or len(raw) != len(parts):
        raise Refusal("value", "a tuple or record is an array of its arity")
    return tuple(
        decode_value(item, part, types, sem)
        for item, part in zip(raw, parts, strict=True)
    )


def _decode_variant(raw: object, ty: Ty, types: dict, sem: Semantics) -> tuple:
    if type(raw) is not dict or set(raw) != {"ctor", "fields"}:
        raise Refusal("value", "a variant has exactly the keys ctor and fields")
    ctors = _variant_ctors(types, ty.head)
    ctor, fields = raw["ctor"], raw["fields"]
    if not isinstance(ctor, str) or ctor not in ctors or type(fields) is not list:
        raise Refusal("value", "an undeclared constructor")
    if len(fields) != len(ctors[ctor]):
        raise Refusal("value", "a constructor's field count")
    return (
        ctor,
        tuple(
            decode_value(item, part, types, sem)
            for item, part in zip(fields, ctors[ctor], strict=True)
        ),
    )


# ---------------------------------------------------------------------------
# Python -> JSON (the inverse; the harness writes outcomes with it)
# ---------------------------------------------------------------------------
def value_to_json(value: object, ty: Ty, types: dict | None = None) -> object:
    """The untagged corpus JSON of ``value`` at type ``ty`` (``Int`` included)."""
    types = types or {}
    head = ty.head
    if head == "Bool":
        return bool(value)
    if head in FIXED or head in ("Nat", "Int"):
        return str(value)
    if head == "Bytes":
        assert isinstance(value, bytes)
        return value.hex()
    assert isinstance(value, (tuple, list))
    if head == "Result":
        if value[0] == "ok":
            return {"ok": value_to_json(value[1], ty.args[0], types)}
        return {"err": value[1]}
    if head == "Option":
        if value[0] == "none":
            return {"none": True}
        return {"some": value_to_json(value[1], ty.args[0], types)}
    if head == "List":
        return [value_to_json(item, ty.args[0], types) for item in value]
    if head == "()":
        return [value_to_json(v, t, types) for v, t in zip(value, ty.args, strict=True)]
    if _kind(ty, types) == "record":
        parts = _record_fields(types, head)
        return [value_to_json(v, t, types) for v, t in zip(value, parts, strict=True)]
    ctor, fields = value
    parts = _variant_ctors(types, head)[ctor]
    return {
        "ctor": ctor,
        "fields": [
            value_to_json(v, t, types) for v, t in zip(fields, parts, strict=True)
        ],
    }


# ---------------------------------------------------------------------------
# section 2.4: Python -> stream bytes
# ---------------------------------------------------------------------------
Encoder = Callable[[Value], bytes]


def make_encoder(
    ty: Ty, types: dict | None = None, sem: Semantics = STANDARD
) -> Encoder:
    """A function from a value of type ``ty`` to its section 2.4 bytes.

    The encoder is built **once** per type and then called per value: an
    enumerated chunk encodes up to 65,536 tuples, and re-reading the type for
    each one would dominate the run.
    """
    types = types or {}
    head = ty.head
    u32 = sem.u32
    if head == "Bool":
        return lambda value: b"\x01" if value else b"\x00"
    if head == "Nat" or (head in FIXED and not ty.signed):
        limit = (1 << 64) - 1 if head == "Nat" else (1 << FIXED[head][0]) - 1

        def unsigned(value: Value) -> bytes:
            if type(value) is not int or not 0 <= value <= limit:
                raise Refusal("value", "a value outside its type cannot be encoded")
            return b"\x10" + value.to_bytes(8, "little")

        return unsigned
    if head in FIXED:
        low, high = fixed_range(ty)

        def signed(value: Value) -> bytes:
            if type(value) is not int or not low <= value <= high:
                raise Refusal("value", "a value outside its type cannot be encoded")
            return b"\x11" + sem.encode_signed(value)

        return signed
    if head == "Bytes":
        return lambda value: b"\x20" + u32(len(value)) + value
    if head == "Result":
        ok = make_encoder(ty.args[0], types, sem)

        def result(value: Value) -> bytes:
            if value[0] == "ok":
                return b"\x50" + ok(value[1])
            name = value[1].encode("utf-8")
            return b"\x51" + u32(len(name)) + name

        return result
    if head == "Option":
        inner = make_encoder(ty.args[0], types, sem)
        return lambda value: (
            b"\x40" if value[0] == "none" else b"\x41" + inner(value[1])
        )
    if head == "List":
        element = make_encoder(ty.args[0], types, sem)
        return lambda value: (
            b"\x30" + u32(len(value)) + b"".join(element(item) for item in value)
        )
    if head == "()" or _kind(ty, types) == "record":
        parts = list(ty.args) if head == "()" else _record_fields(types, head)
        encoders = [make_encoder(part, types, sem) for part in parts]
        return lambda value: (
            b"\x64"
            + u32(len(encoders))
            + b"".join(e(v) for e, v in zip(encoders, value, strict=True))
        )
    if _kind(ty, types) == "variant":
        table = {
            ctor: [make_encoder(part, types, sem) for part in parts]
            for ctor, parts in _variant_ctors(types, head).items()
        }

        def variant(value: Value) -> bytes:
            ctor, fields = value
            name = ctor.encode("utf-8")
            return (
                b"\x65"
                + u32(len(name))
                + name
                + u32(len(fields))
                + b"".join(e(v) for e, v in zip(table[ctor], fields, strict=True))
            )

        return variant
    raise Refusal("type", "a value of this type has no stream encoding")


def encode_value(
    value: object, ty: Ty, types: dict | None = None, sem: Semantics = STANDARD
) -> bytes:
    """The section 2.4 bytes of one value: no header, no context prefix."""
    return make_encoder(ty, types, sem)(value)
