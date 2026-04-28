"""Typed AST nodes for Twig.

Why a typed AST on top of the generic ``ASTNode``?
==================================================
The grammar-driven parser emits a generic :class:`lang_parser.ASTNode`
tree where each node carries a ``rule_name`` and a list of children
that mix ASTNodes with ``Token``s.  Walking that tree directly in the
compiler means a sea of ``isinstance`` checks and ``rule_name``
dispatches with no static structure.

Twig has eight semantic forms.  Lifting the generic AST into typed
dataclasses (:class:`If`, :class:`Lambda`, :class:`Apply`, …) gives
the compiler and the free-variable analyser a small, exhaustive
set of cases to handle, with each variant carrying exactly the
fields it needs.  This is the same pattern that ``compiler-ir`` and
``interpreter-ir`` use for their own IR shapes.

Conversion happens in :mod:`twig.ast_extract`.  Source positions are
preserved on every node for future LSP / debugger integration.
"""

from __future__ import annotations

from dataclasses import dataclass, field

# ---------------------------------------------------------------------------
# Atoms
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class IntLit:
    value: int
    line: int | None = None
    column: int | None = None


@dataclass(frozen=True)
class BoolLit:
    value: bool
    line: int | None = None
    column: int | None = None


@dataclass(frozen=True)
class NilLit:
    line: int | None = None
    column: int | None = None


@dataclass(frozen=True)
class SymLit:
    """A quoted symbol: ``'foo`` or ``(quote foo)``."""

    name: str
    line: int | None = None
    column: int | None = None


@dataclass(frozen=True)
class VarRef:
    """A bare name reference: ``x``, ``length``, ``+``, ``cons``."""

    name: str
    line: int | None = None
    column: int | None = None


# ---------------------------------------------------------------------------
# Compound expressions
# ---------------------------------------------------------------------------


@dataclass
class If:
    cond: Expr
    then_branch: Expr
    else_branch: Expr
    line: int | None = None
    column: int | None = None


@dataclass
class Let:
    """Mutually-independent (Scheme ``let``, not ``let*``) bindings.

    ``body`` is one or more expressions; the value of the last one is
    the value of the ``let``.
    """

    bindings: list[tuple[str, Expr]]
    body: list[Expr] = field(default_factory=list)
    line: int | None = None
    column: int | None = None


@dataclass
class Begin:
    exprs: list[Expr]
    line: int | None = None
    column: int | None = None


@dataclass
class Lambda:
    """Anonymous function.

    Free variables are resolved at compile time: every name that
    appears in ``body``, isn't a parameter, and isn't a top-level
    ``define`` becomes a *captured* variable, computed by
    :mod:`twig.free_vars`.
    """

    params: list[str]
    body: list[Expr]
    line: int | None = None
    column: int | None = None


@dataclass
class Apply:
    """Function application: ``(fn arg0 arg1 ...)``.

    The compiler decides at compile time whether this is a direct
    call (``fn`` is a top-level name) or an indirect closure call
    (``fn`` is anything else).
    """

    fn: Expr
    args: list[Expr]
    line: int | None = None
    column: int | None = None


# ---------------------------------------------------------------------------
# Top-level forms
# ---------------------------------------------------------------------------


@dataclass
class Define:
    """``(define name expr)`` or its function-sugar variant."""

    name: str
    expr: Expr
    line: int | None = None
    column: int | None = None


@dataclass
class Program:
    """A whole compilation unit.

    ``forms`` is the ordered sequence of top-level defines and
    expressions.  Top-level expressions accumulate into the
    synthesised ``main`` function during compilation.
    """

    forms: list[Form]


# ---------------------------------------------------------------------------
# Type aliases
# ---------------------------------------------------------------------------


# A type-checker-friendly union of every expression kind.
Expr = (
    IntLit
    | BoolLit
    | NilLit
    | SymLit
    | VarRef
    | If
    | Let
    | Begin
    | Lambda
    | Apply
)

Form = Expr | Define
