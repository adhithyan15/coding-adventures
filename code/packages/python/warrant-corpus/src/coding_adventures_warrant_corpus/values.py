"""Warrant types and values, as this lane represents them.

A Warrant value crosses three boundaries, and each has its own encoding:

=========================  ===================================================
boundary                   encoding
=========================  ===================================================
corpus JSON                tagged objects: ``{"type": "Bytes", "value": "00"}``
``cases.tfv``              TFV tokens (see :mod:`.tfv`)
stream digest              ``warrant-stream-v1`` bytes (below)
=========================  ===================================================

Inside the runner every value is a plain Python object:

=================  ============================================
Warrant type       Python value
=================  ============================================
``Bool``           ``bool``
``U8`` ``U64``     ``int``
``Nat``            ``int``
``Bytes``          ``bytes``
``Result<T,E>``    ``("ok", value)`` or ``("err", "VARIANT")``
=================  ============================================

Why exact types matter
----------------------
Python's ``==`` is generous in exactly the wrong places::

    True == 1                      # a Bool op returning 1 would "match" true
    bytearray(b"a") == b"a"        # a mutable buffer "matches" an immutable one

A comparison that generous would let a library return the wrong *type* and
still pass.  So results are first checked with :func:`check_value`, which uses
``type(x) is T``, and only then compared with ``==``.

The stream encoding (VER04i section 2.6)
----------------------------------------
Digests are over bytes that every lane can produce identically::

    Bool   00 | 01
    U8/U64/Nat   10 + 8 bytes little-endian (unsigned)
    Bytes  20 + u32-LE length + the bytes
    Ok(v)  50 + encoding of v
    Err(E.V)  51 + u32-LE length + UTF-8 variant name
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from .errors import CorpusError, need

U64_MAX = (1 << 64) - 1
NAT_MAX = U64_MAX
MAX_WIDTH = 4096  # VER04h MAX_FIXED_WIDTH bounds every Bytes<N>

_DECIMAL = re.compile(r"0|[1-9][0-9]*")
_HEX = re.compile(r"(?:[0-9a-f]{2})*")
_BYTES_TYPE = re.compile(r"Bytes(?:<([0-9]+|[A-Z][A-Za-z0-9_]*)>)?")


@dataclass(frozen=True)
class Type:
    """A parsed Warrant type.

    ``kind`` is one of ``Bool U8 U64 Nat Bytes Result``.  ``width`` is set for
    ``Bytes<N>``: an ``int`` for a literal width, or the ``str`` name of a
    width parameter (``"N"``) in an operation signature.  ``ok`` and ``error``
    are set for ``Result``.
    """

    kind: str
    text: str
    width: int | str | None = None
    ok: Type | None = None
    error: str | None = None


def _split_top_level_comma(text: str) -> tuple[str, str]:
    depth = 0
    for index, char in enumerate(text):
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
        elif char == "," and depth == 0:
            return text[:index], text[index + 1 :]
    raise CorpusError("unsupported-type", "malformed Result type")


def parse_type(text: str) -> Type:
    """Parse a type name; anything outside the closed profile is an error."""
    if text in ("Bool", "U8", "U64", "Nat"):
        return Type(text, text)
    match = _BYTES_TYPE.fullmatch(text)
    if match:
        raw = match.group(1)
        width: int | str | None = None
        if raw is not None:
            width = int(raw) if raw.isdigit() else raw
            need(
                not isinstance(width, int) or width <= MAX_WIDTH,
                "unsupported-type",
                "Bytes width exceeds the fixed-width bound",
            )
        return Type("Bytes", text, width)
    if text.startswith("Result<") and text.endswith(">"):
        ok_text, error = _split_top_level_comma(text[len("Result<") : -1])
        need(
            re.fullmatch(r"[A-Z][A-Za-z0-9_]*", error) is not None,
            "unsupported-type",
            "Result error type must be a declared error name",
        )
        return Type("Result", text, ok=parse_type(ok_text), error=error)
    raise CorpusError(
        "unsupported-type", "type is outside the warrant-corpus-v1 profile"
    )


# ---------------------------------------------------------------------------
# Corpus JSON  ->  Python value
# ---------------------------------------------------------------------------
def _canonical_int(text: object, upper: int, what: str) -> int:
    need(
        isinstance(text, str) and _DECIMAL.fullmatch(text) is not None,
        "value",
        f"{what} must be canonical decimal text",
    )
    assert isinstance(text, str)  # for the type checker; need() proved it
    number = int(text)
    need(number <= upper, "value", f"{what} is out of range")
    return number


def decode_json(raw: object, ty: Type, errors: dict[str, list[str]]) -> object:
    """Decode a corpus JSON value of declared type ``ty``.

    ``errors`` maps each declared error type to its variant names, so a
    ``Result`` can only carry a variant the Warrant module declared.
    Integers and byte strings are tagged objects, never bare JSON numbers,
    because JSON numbers are floats in many parsers (and a ``U64`` must not
    lose its low bits).
    """
    if ty.kind == "Bool":
        need(type(raw) is bool, "value", "Bool value must be a JSON boolean")
        return raw
    if ty.kind == "Result":
        assert ty.ok is not None and ty.error is not None
        need(
            type(raw) is dict
            and set(raw) == {"type", "value"}
            and raw["type"] == ty.text,
            "value",
            "Result value must be tagged with its type",
        )
        body = raw["value"]
        need(type(body) is dict and len(body) == 1, "value", "invalid Result body")
        if "ok" in body:
            return ("ok", decode_json(body["ok"], ty.ok, errors))
        variant = body.get("err")
        need(
            type(variant) is str and variant in errors.get(ty.error, []),
            "value",
            "Result error is not a declared variant",
        )
        return ("err", variant)
    need(
        type(raw) is dict and set(raw) == {"type", "value"} and raw["type"] == ty.text,
        "value",
        f"{ty.text} value must be a tagged object",
    )
    text = raw["value"]
    if ty.kind == "Bytes":
        need(
            isinstance(text, str) and _HEX.fullmatch(text) is not None,
            "value",
            "Bytes value must be lowercase even-length hex",
        )
        assert isinstance(text, str)
        data = bytes.fromhex(text)
        if isinstance(ty.width, int):
            need(len(data) == ty.width, "value", "Bytes width mismatch")
        return data
    upper = {"U8": 255, "U64": U64_MAX, "Nat": NAT_MAX}[ty.kind]
    return _canonical_int(text, upper, ty.kind)


# ---------------------------------------------------------------------------
# Strict type check of a *result* (the thing a library returned)
# ---------------------------------------------------------------------------
def check_value(value: object, ty: Type, errors: dict[str, list[str]]) -> bool:
    """Is ``value`` exactly a ``ty``?  (``type(x) is T``, never ``isinstance``.)"""
    if ty.kind == "Bool":
        return type(value) is bool
    if ty.kind == "Bytes":
        return type(value) is bytes
    if ty.kind in ("U8", "U64", "Nat"):
        upper = {"U8": 255, "U64": U64_MAX, "Nat": NAT_MAX}[ty.kind]
        return type(value) is int and 0 <= value <= upper
    assert ty.ok is not None and ty.error is not None
    if type(value) is not tuple or len(value) != 2:
        return False
    if value[0] == "ok":
        return check_value(value[1], ty.ok, errors)
    return (
        value[0] == "err"
        and type(value[1]) is str
        and value[1] in errors.get(ty.error, [])
    )


# ---------------------------------------------------------------------------
# Stream encoding
# ---------------------------------------------------------------------------
def _u32(number: int) -> bytes:
    return number.to_bytes(4, "little")


def encode_stream(value: object, ty: Type) -> bytes:
    """The ``warrant-stream-v1`` bytes of ``value`` (see the module docstring)."""
    if ty.kind == "Bool":
        return b"\x01" if value else b"\x00"
    if ty.kind in ("U8", "U64", "Nat"):
        assert type(value) is int
        return b"\x10" + value.to_bytes(8, "little")
    if ty.kind == "Bytes":
        assert type(value) is bytes
        return b"\x20" + _u32(len(value)) + value
    assert ty.ok is not None and type(value) is tuple
    if value[0] == "ok":
        return b"\x50" + encode_stream(value[1], ty.ok)
    name = str(value[1]).encode("utf-8")
    return b"\x51" + _u32(len(name)) + name


def stream_prefix(suite_id: str, binder_types: list[str], result_type: str) -> bytes:
    """The context both digests start with (VER04i 2.6).

    ``"warrant-stream-v1\\0"``, the length-prefixed suite id, the binder count,
    each binder's canonical type name, then the result type name.  Because the
    context fixes every type, the per-case bytes need no delimiters.
    """

    def field(text: str) -> bytes:
        raw = text.encode("utf-8")
        return _u32(len(raw)) + raw

    return (
        b"warrant-stream-v1\x00"
        + field(suite_id)
        + _u32(len(binder_types))
        + b"".join(field(name) for name in binder_types)
        + field(result_type)
    )


@dataclass(frozen=True)
class OpSig:
    """An operation's signature as the corpus declares it."""

    name: str
    params: tuple[tuple[str, Type], ...]
    ret: Type
    width_params: tuple[str, ...] = ()
    fresh: bool = False


# ---------------------------------------------------------------------------
# TFV value  ->  Python value
# ---------------------------------------------------------------------------
def decode_tfv(raw: tuple, ty: Type, errors: dict[str, list[str]]) -> object:
    """Decode a parsed TFV value (see :mod:`.tfv`) of declared type ``ty``.

    The mapping is VER04i section 2.7: ``Bool`` is ``T``/``F``, integers are
    ``I<n>``, bytes are ``X<hex>``, and a ``Result`` is a one-entry map whose
    key is the text ``ok`` (value) or ``err`` (the *variant name* as text).
    """
    kind = raw[0]
    if ty.kind == "Bool":
        need(kind == "bool", "tfv-value", "expected a TFV boolean")
        return raw[1]
    if ty.kind == "Bytes":
        need(kind == "bytes", "tfv-value", "expected TFV bytes")
        if isinstance(ty.width, int):
            need(len(raw[1]) == ty.width, "tfv-value", "TFV Bytes width mismatch")
        return raw[1]
    if ty.kind in ("U8", "U64", "Nat"):
        upper = {"U8": 255, "U64": U64_MAX, "Nat": NAT_MAX}[ty.kind]
        need(
            kind == "int" and 0 <= raw[1] <= upper,
            "tfv-value",
            "TFV integer out of range",
        )
        return raw[1]
    assert ty.ok is not None and ty.error is not None
    need(
        kind == "map" and len(raw[1]) == 1, "tfv-value", "expected a one-entry TFV map"
    )
    ((key, body),) = raw[1]
    need(key[0] == "text", "tfv-value", "Result key must be text")
    if key[1] == "ok":
        return ("ok", decode_tfv(body, ty.ok, errors))
    need(
        key[1] == "err" and body[0] == "text" and body[1] in errors.get(ty.error, []),
        "tfv-value",
        "Result error is not a declared variant",
    )
    return ("err", body[1])
