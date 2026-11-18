"""``cases.tfv`` for a v2 corpus: TFV v1 records for the examples (VER04r section 2.5).

TFV v1 (TF01 section 2) is a line format any language parses in a few dozen lines::

    tfv 1
    <id> R<n> Kop S<hex of the op name> K<param> <value> ... [Kwidth I<n>] Kexpect <value>

``R<n>`` is a record of ``n`` fields, ``K<name>`` a field name, ``S<hex>`` a text (hex of
its UTF-8), ``X<hex>`` bytes, ``I<decimal>`` an integer, ``T``/``F`` a boolean and
``M<n>`` a map of ``n`` ``key value`` pairs.  ``Result`` stays the ``M`` form of VER04i
section 2.7: ``M1 S6f6b <ok value>`` or ``M1 S657272 S<error case>`` (section 2.5 of
VER04r keeps it that way).  ``cases.tfv`` holds **examples only** [D11g]: everything
large is enumerated, not materialised.  For ct-compare the bytes equal the v1 file.

This module is the oracle's own writer; it shares nothing with the generator.
"""

from __future__ import annotations

from . import ast as A
from .errors import OracleError
from .lower import FIXED


def _text(value: str) -> str:
    return "S" + value.encode("utf-8").hex()


def value(v: object, ty: A.Ty) -> str:
    """One TFV value of Warrant type ``ty`` (``ir.core`` and ``ir.tuple`` types).

    A tuple is ``L<n> v1 .. vn`` (section 2.5; VER04j 4.9), components in order and nested the same
    way; a ``Result`` stays the ``M`` form around whatever its payload is.
    """
    head = ty.head
    if head == "Bool":
        return "T" if v else "F"
    if head in FIXED or head == "Nat":
        return f"I{v}"
    if head == "Bytes":
        return "X" + v.hex()
    if head == "()":
        return f"L{len(v)} " + " ".join(value(x, t) for x, t in zip(v, ty.args))
    if head == "Result":
        if v[0] == "ok":
            return "M1 " + _text("ok") + " " + value(v[1], ty.args[0])
        return "M1 " + _text("err") + " " + _text(v[1])
    raise OracleError("level", "a TFV value outside ir.core needs a later level", level="ir.data")


def record(example_id: str, op: str, params: list, args: list, width, expected, ret: A.Ty) -> str:
    """The TFV line of one example: ``op``, each argument by parameter name, ``width``, ``expect``."""
    fields = [("op", _text(op))]
    fields += [(name, value(a, ty)) for (name, ty), a in zip(params, args)]
    if width is not None:
        fields.append(("width", f"I{width}"))
    fields.append(("expect", value(expected, ret)))
    body = " ".join(f"K{k} {v}" for k, v in fields)
    return f"{example_id} R{len(fields)} {body}"


def document(lines: list[str]) -> bytes:
    """``tfv 1`` then one record per line, LF endings, one final LF."""
    return ("tfv 1\n" + "".join(line + "\n" for line in lines)).encode("utf-8")
