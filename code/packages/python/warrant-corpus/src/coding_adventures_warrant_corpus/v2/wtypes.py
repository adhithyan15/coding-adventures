"""Warrant types, as VER04r section 2.1 writes them down.

One grammar names a type everywhere: in ``ops``, in binders, in a stream's
context and in a manifest::

    type = "Bool" | "U8" | "U16" | "U32" | "U64" | "I8" | "I16" | "I32" | "I64"
         | "Nat" | "Bytes" | "Bytes<" N ">" | "(" type {"," type} ")"
         | "List<" type ">" | "Option<" type ">" | "Result<" type "," E ">"
         | Name

:func:`parse_type` turns that text into a :class:`Ty`; ``str(ty)`` is the
inverse, so ``str(parse_type(text))`` is the canonical spelling.

``Int`` is the mathematical integer of VER04 section 3.  It exists **inside the
IR only**: the type of ``Nat - Nat`` and of a ``lit`` node that says
``"type": "Int"``.  A value, a binder or a signature may not have it, which is
what the ``allow_int`` switch below is for.

Two of the shapes here are only half-known while an IR tree is being typed
bottom-up.  A bare ``ok(e)`` has no error type yet and a bare ``err("CASE")`` no
payload type yet (Appendix A, R41); the placeholder ``?`` stands for the missing
half and :func:`unify` fills it in from whatever the node meets.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any

from .errors import Refusal

# name -> (bit width, signed)
FIXED: dict[str, tuple[int, bool]] = {
    "U8": (8, False),
    "U16": (16, False),
    "U32": (32, False),
    "U64": (64, False),
    "I8": (8, True),
    "I16": (16, True),
    "I32": (32, True),
    "I64": (64, True),
}
PRIMITIVE = frozenset({"Bool", "Nat", *FIXED})

MAX_FIXED_WIDTH = 4096  # VER04h: bounds every Bytes<N>
MAX_TYPE_DEPTH = 16  # enclosing constructors; a bare name is depth 0

_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_WIDTH = re.compile(r"0|[1-9][0-9]*")

UNKNOWN = "?"

# A Warrant value in its Python form (see :mod:`.values`).  Deliberately loose: the
# shape of a value is decided by its *Warrant* type (a ``bool``, an ``int``, a
# ``bytes``, a tuple ...), which Python's type system cannot express.
Value = Any


@dataclass(frozen=True)
class Ty:
    """A parsed type: a ``head`` and the ``args`` the head takes.

    ===============  ==========================================================
    head             args
    ===============  ==========================================================
    ``Bool`` ``U8``  none (also ``Nat`` and ``Int``)
    ``Bytes``        none, or ``(N,)`` for ``Bytes<N>`` (``N`` an ``int``, or the
                     ``str`` name of a static width parameter)
    ``()``           the component types of a tuple
    ``List`` ``Option``  the element type
    ``Result``       ``(ok_type, Ty(error_name))``
    ``Name``         none: a record, variant or error type from ``types``
    ``?``            none: the unknown half of a bare ``ok`` or ``err``
    ===============  ==========================================================
    """

    head: str
    args: tuple = ()

    def __str__(self) -> str:
        if self.head == "Bytes" and self.args:
            return f"Bytes<{self.args[0]}>"
        if self.head == "()":
            return "(" + ",".join(str(a) for a in self.args) + ")"
        if self.head in ("List", "Option"):
            return f"{self.head}<{self.args[0]}>"
        if self.head == "Result":
            return f"Result<{self.args[0]},{self.args[1]}>"
        return self.head

    @property
    def is_fixed(self) -> bool:
        return self.head in FIXED

    @property
    def is_number(self) -> bool:
        """Fixed-width, ``Nat`` or ``Int``: the types arithmetic is defined on."""
        return self.head in FIXED or self.head in ("Nat", "Int")

    @property
    def is_exact(self) -> bool:
        """``Nat`` or ``Int``: the unbounded-but-windowed types of section 3.2."""
        return self.head in ("Nat", "Int")

    @property
    def width(self) -> int:
        """Bit width of a fixed-width type."""
        return FIXED[self.head][0]

    @property
    def signed(self) -> bool:
        return FIXED[self.head][1]


BOOL = Ty("Bool")
NAT = Ty("Nat")
INT = Ty("Int")
BYTES = Ty("Bytes")
UNKNOWN_TY = Ty(UNKNOWN)


def fixed_range(ty: Ty) -> tuple[int, int]:
    """The inclusive value range of a fixed-width type (two's complement if signed)."""
    bits, signed = FIXED[ty.head]
    if signed:
        return -(1 << (bits - 1)), (1 << (bits - 1)) - 1
    return 0, (1 << bits) - 1


def parse_type(
    text: object,
    types: dict | None,
    *,
    allow_int: bool = False,
    widths: frozenset[str] = frozenset(),
) -> Ty:
    """Parse the canonical type text ``text``; refuse with the kit code ``type``.

    ``types`` is the section 2.2 table: a bare name must be a ``record`` or
    ``variant`` in it, and the error name of a ``Result`` an ``error`` in it.
    ``None`` is the *open* mode of a descriptor, which carries no table: there
    only the error name of a ``Result`` is free text.  ``widths`` are the static
    width parameters (``Bytes<M>``) of an op signature.

    Depth (section 2.2 rule 1, R40): a bare name is depth 0 and each ``(``,
    ``List<``, ``Option<`` or ``Result<`` around it adds one, so sixteen nested
    wrappers are accepted and seventeen refused.
    """
    if not isinstance(text, str):
        raise Refusal("type", "a type is text")
    pos = 0

    def name() -> str:
        nonlocal pos
        hit = _NAME.match(text, pos)
        if not hit:
            raise Refusal("type", "a type name was expected")
        pos = hit.end()
        return hit.group(0)

    def expect(char: str) -> None:
        nonlocal pos
        if text[pos : pos + 1] != char:
            raise Refusal("type", f"{char!r} was expected")
        pos += 1

    def one(level: int) -> Ty:
        nonlocal pos
        if level > MAX_TYPE_DEPTH:
            raise Refusal("type", "a type nested more than 16 deep")
        if text[pos : pos + 1] == "(":
            pos += 1
            parts = [one(level + 1)]
            while text[pos : pos + 1] == ",":
                pos += 1
                parts.append(one(level + 1))
            expect(")")
            if not 2 <= len(parts) <= 8:
                raise Refusal("type", "a tuple has 2 to 8 components")
            return Ty("()", tuple(parts))
        head = name()
        if head == "Bytes" and text[pos : pos + 1] == "<":
            return bytes_width(level)
        if head in ("List", "Option"):
            expect("<")
            inner = one(level + 1)
            expect(">")
            return Ty(head, (inner,))
        if head == "Result":
            expect("<")
            ok = one(level + 1)
            expect(",")
            error = name()
            expect(">")
            if types is not None and types.get(error, {}).get("kind") != "error":
                raise Refusal("type", "the error type of a Result must be declared")
            return Ty("Result", (ok, Ty(error)))
        if head == "Int":
            if not allow_int:
                raise Refusal("type", "Int exists inside the IR only")
            return INT
        if head in PRIMITIVE or head == "Bytes":
            return Ty(head)
        if types is not None and types.get(head, {}).get("kind") in (
            "record",
            "variant",
        ):
            return Ty(head)
        raise Refusal("type", "not a type of section 2.1, or not in types")

    def bytes_width(_level: int) -> Ty:
        nonlocal pos
        pos += 1  # the "<"
        hit = _WIDTH.match(text, pos)
        if hit:
            pos = hit.end()
            number = int(hit.group(0))
            if number > MAX_FIXED_WIDTH:
                raise Refusal("type", "Bytes<N> with N over 4096")
            width: int | str = number
        else:
            parameter = name()
            if parameter not in widths:
                raise Refusal("type", "a Bytes width is a decimal literal")
            width = parameter
        expect(">")
        return Ty("Bytes", (width,))

    ty = one(0)
    if pos != len(text):
        raise Refusal("type", "text after the end of a type")
    return ty


# ---------------------------------------------------------------------------
# Relations used by the IR checker (VER04r section 16.16 R51, restricted to the
# shapes `ir.core` can produce)
# ---------------------------------------------------------------------------
def flows(source: Ty, target: Ty) -> bool:
    """May a value of type ``source`` be passed where ``target`` is wanted?

    The checker's *assignability*: a type flows to itself; ``Nat`` to ``Int``;
    ``Bytes<N>`` to ``Bytes``; ``Result<S,E>`` to ``Result<T,E>`` when ``S``
    flows to ``T`` (the same ``E``).  The unknown ``?`` flows both ways: it is a
    hole the node it meets will fill.  Nothing else lifts: ``U8`` does not flow
    to ``Nat`` and ``Bool`` is not ``U8``.
    """
    if UNKNOWN in (source.head, target.head) or source == target:
        return True
    if source == NAT and target == INT:
        return True
    if source.head == "Bytes" and target == BYTES:
        return True
    if source.head == "Result" and target.head == "Result":
        return flows(source.args[0], target.args[0]) and flows(
            source.args[1], target.args[1]
        )
    if source.head == target.head == "Option":
        return flows(source.args[0], target.args[0])
    return False


def unify(a: Ty, b: Ty) -> Ty | None:
    """The common type of two operands or two branches, or ``None``.

    Equal types; ``?`` and anything; ``Nat`` and ``Int`` to ``Int``; two
    ``Bytes`` types of different width to plain ``Bytes``; two ``Result`` types
    component-wise.  ``None`` means "no common type", which the checker reports
    as ``ill-typed``.
    """
    if a.head == UNKNOWN:
        return b
    if b.head == UNKNOWN or a == b:
        return a
    if {a, b} == {NAT, INT}:
        return INT
    if a.head == b.head == "Bytes":
        return BYTES
    if a.head == b.head == "Result":
        ok, err = unify(a.args[0], b.args[0]), unify(a.args[1], b.args[1])
        return Ty("Result", (ok, err)) if ok and err else None
    if a.head == b.head == "Option":
        inner = unify(a.args[0], b.args[0])
        return Ty("Option", (inner,)) if inner else None
    return None


def is_undetermined(ty: Ty) -> bool:
    """Does ``ty`` still hold a ``?`` (the unknown half of a bare ``ok``/``err``)?"""
    return ty.head == UNKNOWN or any(
        isinstance(arg, Ty) and is_undetermined(arg) for arg in ty.args
    )
