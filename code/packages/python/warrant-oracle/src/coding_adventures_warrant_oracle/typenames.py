"""Canonical type names read back into types (VER04r section 2.1).

The IR names a type in exactly one place that is not a source type: the static type argument of
``["none", "T"]`` (Appendix A.2), beside the ``ir.fixed`` conversions' fixed-width names.  A corpus writes
a type as its canonical text (``Option<(U8,Bool)>``, ``Bytes<4>``, ``Result<Nat,E>``, a record name), and a
consumer that types IR bottom-up has to read that text back.  :func:`parse_type_name` does it with a small
recursive-descent reader over the grammar of section 2.1::

    type = "Bool" | "U8" .. "U64" | "I8" .. "I64" | "Nat" | "Int" | "Bytes" | "Bytes<" N ">"
         | "(" type { "," type } ")"           -- tuple, 2..=8 components
         | "List<" type ">" | "Option<" type ">" | "Result<" type "," ErrName ">"
         | Name                                 -- a record, variant or error type from ``types``

``Int`` is allowed here, as in a ``lit`` (section 2.1: it may be *written* in a ``type`` field of a literal
and, by Appendix A.2, of a ``none``).  The reader checks the **shape** only; whether a name is declared, a
width is in range or a list is allowed is the caller's (:class:`.ir_eval.Program`).  Nothing here echoes
the input: a malformed name is one :class:`ValueError` with a fixed message.
"""

from __future__ import annotations

import re

from . import ast as A

#: The deepest type expression a corpus may write (section 2.2 rule 1).
MAX_TYPE_DEPTH = 16
_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_WIDTH = re.compile(r"(0|[1-9][0-9]{0,3})")


def parse_type_name(text: object) -> A.Ty:
    """The type whose canonical spelling is ``text``; :class:`ValueError` if it is not one."""
    if not isinstance(text, str) or not text.isascii() or len(text) > 512:
        raise ValueError("not a canonical type name")
    ty, end = _type(text, 0, 1)
    if end != len(text):
        raise ValueError("not a canonical type name")
    return ty


def _type(text: str, pos: int, depth: int) -> tuple[A.Ty, int]:
    if depth > MAX_TYPE_DEPTH:
        raise ValueError("a type nests too deeply")
    if text.startswith("(", pos):
        items: list[A.Ty] = []
        pos += 1
        while True:
            item, pos = _type(text, pos, depth + 1)
            items.append(item)
            if text.startswith(",", pos):
                pos += 1
                continue
            break
        if not text.startswith(")", pos) or not 2 <= len(items) <= 8:
            raise ValueError("not a canonical type name")
        return A.Ty("()", tuple(items)), pos + 1
    match = _NAME.match(text, pos)
    if match is None:
        raise ValueError("not a canonical type name")
    head, pos = match.group(), match.end()
    if not text.startswith("<", pos):
        return A.Ty(head), pos
    pos += 1
    if head == "Bytes":
        width = _WIDTH.match(text, pos)
        if width is None or not text.startswith(">", width.end()):
            raise ValueError("not a canonical type name")
        return A.Ty("Bytes", (int(width.group()),)), width.end() + 1
    args: list[A.Ty] = []
    while True:
        arg, pos = _type(text, pos, depth + 1)
        args.append(arg)
        if text.startswith(",", pos):
            pos += 1
            continue
        break
    if not text.startswith(">", pos):
        raise ValueError("not a canonical type name")
    if head in ("List", "Option") and len(args) == 1:
        return A.Ty(head, tuple(args)), pos + 1
    if head == "Result" and len(args) == 2 and not args[1].args:
        return A.Ty("Result", tuple(args)), pos + 1
    raise ValueError("not a canonical type name")
