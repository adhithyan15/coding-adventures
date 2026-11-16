"""Syntax tree for ``.warrant`` documents.

The tree is deliberately close to the surface syntax: one node per source
construct, no desugaring.  Desugaring (``a != b``, refinements into ``admit``,
``x.0``) happens in :mod:`.lower`, because VER04r section 10.3 demands that
lowering be syntax-directed and unoptimised, and the cleanest way to guarantee
that is to keep the tree faithful and do the (small) translation in one place.

Types
-----
A type is a :class:`Ty`: a head name plus arguments.  ``Bytes<4>`` is
``Ty("Bytes", (4,))``, ``Bytes<N>`` is ``Ty("Bytes", ("N",))``, a tuple type is
``Ty("()", (t1, t2, ...))`` and ``Result<Bytes,E>`` is
``Ty("Result", (Ty("Bytes"), Ty("E")))``.  :meth:`Ty.__str__` gives the
canonical spelling of VER04r section 2.1 (no spaces), which is also what a
corpus writes.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class Ty:
    """A Warrant type expression."""

    head: str
    args: tuple = ()

    def __str__(self) -> str:
        if self.head == "()":
            return "(" + ",".join(str(a) for a in self.args) + ")"
        if not self.args:
            return self.head
        return self.head + "<" + ",".join(str(a) for a in self.args) + ">"


# -- expressions -------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class BoolLit:
    value: bool


@dataclass(frozen=True, slots=True)
class IntLit:
    """A decimal or hex literal; ``suffix`` is e.g. ``"u8"`` or ``None``."""

    value: int
    suffix: str | None


@dataclass(frozen=True, slots=True)
class BytesLit:
    hex: str


@dataclass(frozen=True, slots=True)
class Name:
    """A bare or dotted name: ``a``, ``SelectError.CASE``, ``t.0``, ``r.field``."""

    parts: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class Call:
    """A reference applied to static arguments and/or arguments.

    ``args is None`` means no parenthesised argument list (``f<3>`` alone).
    """

    name: tuple[str, ...]
    statics: tuple  # ints and names, from ``<...>``
    args: tuple | None


@dataclass(frozen=True, slots=True)
class Unary:
    op: str  # "not" | "neg"
    operand: object


@dataclass(frozen=True, slots=True)
class Binary:
    op: str  # == != < <= > >= + - * and or =>
    left: object
    right: object


@dataclass(frozen=True, slots=True)
class If:
    cond: object
    then: object
    other: object


@dataclass(frozen=True, slots=True)
class Proj:
    """Postfix projection ``e.0`` / ``e.field`` after a non-name head."""

    base: object
    field: str


@dataclass(frozen=True, slots=True)
class TupleLit:
    items: tuple


@dataclass(frozen=True, slots=True)
class ListLit:
    items: tuple


@dataclass(frozen=True, slots=True)
class Pattern:
    """A match pattern: ``kind`` is wild, bind, bool, tuple or ctor."""

    kind: str
    name: tuple[str, ...] = ()
    items: tuple = ()
    value: bool = False


@dataclass(frozen=True, slots=True)
class Match:
    scrutinee: object
    arms: tuple  # ((Pattern, expr), ...)


@dataclass(frozen=True, slots=True)
class Binder:
    """A quantifier binder or operation parameter: ``name: type [| refinement]``."""

    name: str
    ty: Ty
    refinement: object | None = None
    inout: bool = False


@dataclass(frozen=True, slots=True)
class Quant:
    kind: str  # "forall" | "exists"
    binders: tuple
    body: object


# -- declarations ------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ErrorDecl:
    name: str
    cases: tuple[str, ...]
    line: int = 0


@dataclass(frozen=True, slots=True)
class AliasDecl:
    name: str
    ty: Ty
    line: int = 0


@dataclass(frozen=True, slots=True)
class ModelDecl:
    name: str
    params: tuple
    ret: Ty
    body: object
    line: int = 0


@dataclass(frozen=True, slots=True)
class RecordDecl:
    name: str
    fields: tuple  # ((name, Ty), ...)
    line: int = 0


@dataclass(frozen=True, slots=True)
class VariantDecl:
    name: str
    ctors: tuple  # ((name, ((field, Ty), ...)), ...)
    line: int = 0


@dataclass(frozen=True, slots=True)
class OpDecl:
    name: str
    statics: tuple[str, ...]
    params: tuple
    ret: Ty
    fresh: bool
    line: int = 0


@dataclass(frozen=True, slots=True)
class ExampleDecl:
    name: str
    body: object
    line: int = 0


@dataclass(frozen=True, slots=True)
class ClaimDecl:
    name: str
    body: object
    line: int = 0


@dataclass(frozen=True, slots=True)
class AssumeDecl:
    kind: str  # platform | environment
    rows: tuple  # () for ``none``; strings (platform) or (name, expr) rows


@dataclass(frozen=True, slots=True)
class LabelsDecl:
    op: str
    rows: tuple  # ((path tuple, "public"|"secret"), ...)


@dataclass(frozen=True, slots=True)
class InputDecl:
    op: str
    param: str
    rejects: tuple  # ((expr, (Error, Case)), ...)
    ensures: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class BoundDecl:
    op: str
    rows: tuple  # ((metric, expr), ...)


@dataclass(frozen=True, slots=True)
class EnsuresDecl:
    """WX-G ``ensures <op>:`` block of postcondition rows (VER04k section 4)."""

    op: str
    rows: tuple


@dataclass(frozen=True, slots=True)
class Document:
    name: str
    version: str  # "v0" or "v1"
    decls: tuple = field(default_factory=tuple)
