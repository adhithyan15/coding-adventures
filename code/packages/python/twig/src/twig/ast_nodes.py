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
class Module:
    """A ``(module name (export ...) (import ...))`` declaration.

    Attached to the :class:`Program` it heads.  Contains:

    * ``name`` — slash-separated module path (``stdlib/io``,
      ``user/compiler/lexer``).  Must match the file's location
      relative to a module-search-path root; module-resolution
      enforces that mismatch is a compile error (TW04 Phase 4b).

    * ``exports`` — ordered list of names this module makes
      visible to importers.  Names not in this list are
      file-private.  An empty list means "no public surface"
      (legal but unusual — useful for entry-point modules whose
      only role is the side-effect of running their top-level
      forms).

    * ``imports`` — ordered list of module paths whose exports
      this module brings into its namespace.  Each imported
      module's names are accessed as ``<module-path>/<name>``
      (e.g. ``host/write-byte``, ``stdlib/io/println``) — the
      prefix is mandatory; see TW04 spec on namespace hygiene.

    Phase 4a is parser-only: the AST records this declaration
    but no module-resolution, cross-module IR, or per-backend
    lowering happens yet.  The :class:`Program`'s ``forms`` list
    is unchanged from the no-module case.
    """

    name: str
    exports: list[str] = field(default_factory=list)
    imports: list[str] = field(default_factory=list)
    line: int | None = None
    column: int | None = None


@dataclass
class Program:
    """A whole compilation unit.

    ``forms`` is the ordered sequence of top-level defines and
    expressions.  Top-level expressions accumulate into the
    synthesised ``main`` function during compilation.

    ``module`` is the optional :class:`Module` declaration
    (TW04 Phase 4a).  When ``None``, the program belongs to an
    implicit "default module" — back-compat for every single-file
    Twig program written before TW04 landed.
    """

    forms: list[Form]
    module: Module | None = None


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
