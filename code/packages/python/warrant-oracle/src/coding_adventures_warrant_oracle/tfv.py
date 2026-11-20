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
from .lower import FIXED, Spec


def _text(value: str) -> str:
    return "S" + value.encode("utf-8").hex()


def value(v: object, ty: A.Ty, spec: Spec | None = None) -> str:
    """One TFV value of Warrant type ``ty``.

    A tuple or a record is ``L<n> v1 .. vn`` (section 2.5; VER04j 4.9), components or fields in order and
    nested the same way; a ``Result`` stays the ``M`` form around whatever its payload is.  Slice S11c-1 adds
    the two forms VER04j 4.9 gives and section 2.5 adopts: an ``Option`` is ``L0`` (none) or ``L1 x`` (some),
    and a variant is ``L2 S<hex ctor name> L<k> f1 .. fk``, by constructor *name* (so reordering the
    declarations cannot change a vector).  Which one a ``L1`` is, an ``Option`` or a one-field record, is
    decided by the declared type, never by the text.  ``spec`` supplies the declared records and variants.
    """
    head = ty.head
    if head == "Bool":
        return "T" if v else "F"
    if head in FIXED or head == "Nat":
        return f"I{v}"
    if head == "Bytes":
        return "X" + v.hex()
    if head == "()":
        return f"L{len(v)} " + " ".join(value(x, t, spec) for x, t in zip(v, ty.args))
    if head == "Result":
        if v[0] == "ok":
            return "M1 " + _text("ok") + " " + value(v[1], ty.args[0], spec)
        return "M1 " + _text("err") + " " + _text(v[1])
    if head == "Option":
        return "L0" if v[0] == "none" else "L1 " + value(v[1], ty.args[0], spec)
    if spec is not None and head in spec.records:
        types = [t for _, t in spec.records[head]]
        return f"L{len(v)} " + " ".join(value(x, t, spec) for x, t in zip(v, types))
    if spec is not None and head in spec.variants:
        fields = dict(spec.variants[head])[v[0]]
        body = "".join(" " + value(x, t, spec) for x, (_, t) in zip(v[1], fields))
        return f"L2 {_text(v[0])} L{len(fields)}{body}"
    raise OracleError("level", "a TFV value of this type needs a later level", level="ir.list")


def record(
    example_id: str, op: str, params: list, args: list, width, expected, ret: A.Ty, spec: Spec | None = None
) -> str:
    """The TFV line of one example: ``op``, each argument by parameter name, ``width``, ``expect``."""
    fields = [("op", _text(op))]
    fields += [(name, value(a, ty, spec)) for (name, ty), a in zip(params, args)]
    if width is not None:
        fields.append(("width", f"I{width}"))
    fields.append(("expect", value(expected, ret, spec)))
    body = " ".join(f"K{k} {v}" for k, v in fields)
    return f"{example_id} R{len(fields)} {body}"


def document(lines: list[str]) -> bytes:
    """``tfv 1`` then one record per line, LF endings, one final LF."""
    return ("tfv 1\n" + "".join(line + "\n" for line in lines)).encode("utf-8")
