"""Warrant values: reading them from corpus JSON and writing them as stream bytes.

A value crosses two boundaries and each has its own form (VER04r sections 2.3
and 2.4).  This module builds, for one Warrant type, one **codec**: an object that
knows all three spellings of the values of that type --

* ``decode(raw)``   corpus JSON  ->  the Python value the evaluator computes with;
* ``encode(value)`` the Python value  ->  its canonical stream bytes (section 2.4);
* ``to_json(value)`` the Python value  ->  corpus JSON again (for reporting).

**Corpus JSON (section 2.3).**  The corpus never tags a value with its own type:
the reader holds the *declared* type and reads to it, range-checking.  Every
integer is a decimal *string* (a JavaScript reader cannot lose bits), bytes are
lowercase hex, and the composites are

=================  =========================================================
type               JSON
=================  =========================================================
``Bool``           ``true`` / ``false``
integers, ``Nat``  ``"18446744073709551615"``
``Bytes``          ``"0a0b"``
tuple, record      an array in component / declared field order
``Option<T>``      ``{"none":true}`` or ``{"some":v}``
``Result<T,E>``    ``{"ok":v}`` or ``{"err":"CASE"}``
variant            ``{"ctor":"Name","fields":[...]}``
=================  =========================================================

``List<T>`` has a JSON form too, but lists belong to the reserved level
``ir.list`` ([D63]): this runner refuses the type, as the level it lacks, and
never decodes, encodes or reports a list.

**Python values inside the runner.**  The form is *type-directed*: two values
are told apart by the type they are read at, never by their shape.

=================  =========================================================
``Bool``           ``bool``
integers, ``Nat``  ``int``
``Bytes``          ``bytes``
tuple, record      ``tuple``
``Option<T>``      ``("none",)`` or ``("some", v)``
``Result<T,E>``    ``("ok", v)`` or ``("err", "CASE")``
variant            ``(ctor_name, (field, ...))``
=================  =========================================================

**The stream (section 2.4).**  Digests are over bytes every lane can produce
identically.  A leading tag byte says what follows; an integer is always eight
bytes (the *type name* in the stream's context says how wide it really is), and
every length or count is four bytes, little-endian::

    Bool            00 | 01
    U8..U64, Nat    10 + 8 bytes, unsigned
    I8..I64         11 + 8 bytes, two's complement, sign-extended to 64 bits
    Bytes           20 + length + the bytes
    Option          40 | 41 + inner
    Ok / Err        50 + inner | 51 + length + UTF-8 case name
    tuple, record   64 + count + components
    variant         65 + name length + UTF-8 name + field count + fields

Worked example: the record ``Point(x: U8, y: U8)`` holding ``Point(1, 2)`` is
``64 02000000  10 0100000000000000  10 0200000000000000``; ``-1`` as an ``I8`` is
``11 ffffffffffffffff``.  A composite carries its tag and count because it must
be self-delimiting: without them two different values could serialise to the
same bytes.
"""

from __future__ import annotations

from collections.abc import Callable

from ..errors import CorpusError
from ..jsonio import parse_json
from .errors import Refusal
from .semantics import STANDARD, Semantics
from .wtypes import FIXED, MAX_FIXED_WIDTH, Ty, Value, fixed_range, parse_type

_HEX_DIGITS = frozenset("0123456789abcdef")
# A Bytes value is at most MAX_FIXED_WIDTH bytes, so at most twice that many digits;
# a longer string is refused before it is looked at digit by digit.
_MAX_HEX_DIGITS = 2 * MAX_FIXED_WIDTH


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
# The codecs
# ---------------------------------------------------------------------------
class Codec:
    """What is known about one type: how to read, write and report its values."""

    def decode(self, raw: object) -> Value:
        raise NotImplementedError

    def encode(self, value: Value) -> bytes:
        raise NotImplementedError

    def to_json(self, value: Value) -> object:
        raise NotImplementedError


class _Truth(Codec):
    def decode(self, raw: object) -> bool:
        if raw is True or raw is False:
            return raw
        raise Refusal("value", "only the JSON words true and false are booleans")

    def encode(self, value: Value) -> bytes:
        return b"\x01" if value else b"\x00"

    def to_json(self, value: Value) -> object:
        return bool(value)


class _Whole(Codec):
    """An integer type: a fixed width (signed or not), ``Nat`` or ``Int``."""

    def __init__(self, ty: Ty, sem: Semantics) -> None:
        self.ty, self.sem = ty, sem
        self.signed = ty.head in FIXED and ty.signed
        if ty.head == "Nat":
            self.low, self.high = 0, (1 << 64) - 1
        elif ty.head in FIXED:
            self.low, self.high = fixed_range(ty)
        else:  # Int: an IR literal only; it has no stream form
            self.low = self.high = None  # type: ignore[assignment]

    def decode(self, raw: object) -> int:
        return self.sem.decode_int(raw, self.ty)

    def encode(self, value: Value) -> bytes:
        if self.low is None:
            raise Refusal("type", "Int has no stream form")
        if type(value) is not int or not self.low <= value <= self.high:
            raise Refusal("value", "the integer does not fit its type")
        if self.signed:
            return b"\x11" + self.sem.encode_signed(value)
        return b"\x10" + value.to_bytes(8, "little")

    def to_json(self, value: Value) -> object:
        return str(value)


class _Blob(Codec):
    """``Bytes`` (``length`` is ``None``) or ``Bytes<N>`` (``length`` is ``N``)."""

    def __init__(self, length: object, sem: Semantics) -> None:
        self.length = length if type(length) is int else None
        self.sem = sem

    def decode(self, raw: object) -> bytes:
        if not isinstance(raw, str) or len(raw) > _MAX_HEX_DIGITS:
            raise Refusal(
                "value", "bytes must be a hex string no longer than the byte limit"
            )
        if len(raw) % 2 or not _HEX_DIGITS.issuperset(raw):
            raise Refusal("value", "bytes must be an even run of lowercase hex digits")
        if self.length is not None and len(raw) != 2 * self.length:
            raise Refusal(
                "value", "this fixed-width bytes value has the wrong number of bytes"
            )
        return bytes.fromhex(raw)

    def encode(self, value: Value) -> bytes:
        return b"\x20" + self.sem.u32(len(value)) + value

    def to_json(self, value: Value) -> object:
        return value.hex()


class _Maybe(Codec):
    def __init__(self, inner: Codec, sem: Semantics) -> None:
        self.inner, self.sem = inner, sem

    def decode(self, raw: object) -> tuple:
        if type(raw) is dict and len(raw) == 1:
            if "none" in raw:
                if raw["none"] is True:  # the literal true; 1 is not it (F11)
                    return ("none",)
            elif "some" in raw:
                return ("some", self.inner.decode(raw["some"]))
        raise Refusal(
            "value", "an optional value is a single-key object, none:true or some:value"
        )

    def encode(self, value: Value) -> bytes:
        if value[0] == "none":
            return b"\x40"
        return b"\x41" + self.inner.encode(value[1])

    def to_json(self, value: Value) -> object:
        if value[0] == "none":
            return {"none": True}
        return {"some": self.inner.to_json(value[1])}


class _Outcome(Codec):
    """``Result<T,E>``: ``ok`` carries a ``T``, ``err`` names a case of ``E``."""

    def __init__(self, ok: Codec, cases: tuple, sem: Semantics) -> None:
        self.ok, self.cases, self.sem = ok, cases, sem

    def decode(self, raw: object) -> tuple:
        if type(raw) is dict and len(raw) == 1:
            if "ok" in raw:
                return ("ok", self.ok.decode(raw["ok"]))
            if "err" in raw and raw["err"] in self.cases and type(raw["err"]) is str:
                return ("err", raw["err"])
        raise Refusal(
            "value", "a result is a single-key object, ok:value or err:a declared case"
        )

    def encode(self, value: Value) -> bytes:
        if value[0] == "ok":
            return b"\x50" + self.ok.encode(value[1])
        name = value[1].encode("utf-8")
        return b"\x51" + self.sem.u32(len(name)) + name

    def to_json(self, value: Value) -> object:
        if value[0] == "ok":
            return {"ok": self.ok.to_json(value[1])}
        return {"err": value[1]}


class _Row(Codec):
    """A tuple or a record: a fixed number of components, each with its own codec."""

    def __init__(self, parts: list[Codec], sem: Semantics) -> None:
        self.parts, self.sem = parts, sem

    def decode(self, raw: object) -> tuple:
        if type(raw) is not list or len(raw) != len(self.parts):
            raise Refusal("value", f"an array of exactly {len(self.parts)} items")
        return tuple(
            part.decode(item) for part, item in zip(self.parts, raw, strict=True)
        )

    def encode(self, value: Value) -> bytes:
        head = b"\x64" + self.sem.u32(len(self.parts))
        return head + b"".join(
            part.encode(item) for part, item in zip(self.parts, value, strict=True)
        )

    def to_json(self, value: Value) -> object:
        return [
            part.to_json(item) for part, item in zip(self.parts, value, strict=True)
        ]


class _Either(Codec):
    """A variant: a constructor name and that constructor's fields."""

    def __init__(self, shapes: dict[str, list[Codec]], sem: Semantics) -> None:
        self.shapes, self.sem = shapes, sem

    def decode(self, raw: object) -> tuple:
        if type(raw) is not dict or set(raw) != {"ctor", "fields"}:
            raise Refusal(
                "value", "a variant value has the keys ctor and fields and no others"
            )
        name, fields = raw["ctor"], raw["fields"]
        if type(name) is not str or name not in self.shapes:
            raise Refusal("value", "the variant has no constructor of that name")
        shape = self.shapes[name]
        if type(fields) is not list or len(fields) != len(shape):
            raise Refusal(
                "value", "the constructor was given the wrong count of fields"
            )
        return (
            name,
            tuple(part.decode(item) for part, item in zip(shape, fields, strict=True)),
        )

    def encode(self, value: Value) -> bytes:
        name, fields = value
        raw_name = name.encode("utf-8")
        shape = self.shapes[name]
        return (
            b"\x65"
            + self.sem.u32(len(raw_name))
            + raw_name
            + self.sem.u32(len(fields))
            + b"".join(
                part.encode(item) for part, item in zip(shape, fields, strict=True)
            )
        )

    def to_json(self, value: Value) -> object:
        name, fields = value
        shape = self.shapes[name]
        return {
            "ctor": name,
            "fields": [
                part.to_json(item) for part, item in zip(shape, fields, strict=True)
            ],
        }


# ---------------------------------------------------------------------------
# Building a codec from a type
# ---------------------------------------------------------------------------
def codec_for(ty: Ty, types: dict | None = None, sem: Semantics = STANDARD) -> Codec:
    """The codec of ``ty`` under the section 2.2 table ``types``.

    A ``List`` is refused as the level ``ir.list`` ([D63]).  A record or variant
    that contains itself, however far down, is refused as a ``type`` error: the
    table is data from outside, and section 2.2 rule 1 forbids recursion.
    """
    try:
        return _Builder(types or {}, sem).codec(ty)
    except RecursionError as error:  # a chain of nested records, thousands long
        raise Refusal("type", "declarations nested too deeply") from error


class _Builder:
    """Builds the codec of one type, remembering the declarations being expanded."""

    def __init__(self, types: dict, sem: Semantics) -> None:
        self.types, self.sem = types, sem
        self.expanding: list[str] = []

    def codec(self, ty: Ty) -> Codec:
        head = ty.head
        if head == "Bool":
            return _Truth()
        if head == "Nat" or head == "Int" or head in FIXED:
            return _Whole(ty, self.sem)
        if head == "Bytes":
            return _Blob(ty.args[0] if ty.args else None, self.sem)
        if head == "List":
            raise Refusal("level-missing", "List<T> belongs to the level ir.list")
        if head == "Option":
            return _Maybe(self.codec(ty.args[0]), self.sem)
        if head == "Result":
            return _Outcome(
                self.codec(ty.args[0]), self.cases_of(ty.args[1].head), self.sem
            )
        if head == "()":
            return _Row([self.codec(part) for part in ty.args], self.sem)
        return self.declared(head)

    def cases_of(self, error: str) -> tuple:
        """The declared cases of the error type ``error``.

        A descriptor is read *without* a types table (R44), and then no error
        type is declared: its values can be written but no ``err`` can be read.
        """
        row = self.types.get(error)
        if isinstance(row, dict) and row.get("kind") == "error":
            return tuple(row["cases"])
        return ()

    def declared(self, name: str) -> Codec:
        row = self.types.get(name)
        kind = row.get("kind") if isinstance(row, dict) else None
        if kind not in ("record", "variant"):
            raise Refusal("type", f"{name} is not a declared record or variant")
        if name in self.expanding:
            raise Refusal("type", f"{name} contains itself")
        self.expanding.append(name)
        try:
            if kind == "record":
                parts = [self.member(text) for _field, text in row["fields"]]
                return _Row(parts, self.sem)
            return _Either(
                {
                    ctor: [self.member(text) for text in texts]
                    for ctor, texts in row["ctors"]
                },
                self.sem,
            )
        finally:
            self.expanding.pop()

    def member(self, text: object) -> Codec:
        return self.codec(parse_type(text, self.types))


# ---------------------------------------------------------------------------
# The public entry points
# ---------------------------------------------------------------------------
def decode_value(
    raw: object, ty: Ty, types: dict | None, sem: Semantics = STANDARD
) -> object:
    """Read the corpus JSON ``raw`` as a value of type ``ty``, or refuse ``value``."""
    return codec_for(ty, types, sem).decode(raw)


def value_to_json(value: object, ty: Ty, types: dict | None = None) -> object:
    """The untagged corpus JSON of ``value`` at type ``ty`` (``Int`` included)."""
    return codec_for(ty, types).to_json(value)


Encoder = Callable[[Value], bytes]


def make_encoder(
    ty: Ty, types: dict | None = None, sem: Semantics = STANDARD
) -> Encoder:
    """A function from a value of type ``ty`` to its section 2.4 bytes.

    Built **once** per type and called per value: an enumerated chunk encodes up
    to 65,536 tuples, and re-reading the type for each one would dominate the run.
    """
    return codec_for(ty, types, sem).encode


def encode_value(
    value: object, ty: Ty, types: dict | None = None, sem: Semantics = STANDARD
) -> bytes:
    """The section 2.4 bytes of one value: no header, no context prefix."""
    return make_encoder(ty, types, sem)(value)
