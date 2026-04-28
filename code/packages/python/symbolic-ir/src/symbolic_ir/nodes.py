"""The six IR node types and the set of standard head symbols.

Design notes
------------

Every node is a ``@dataclass(frozen=True, slots=True)``. Frozen makes them
immutable (so they're hashable and safe to share); slots avoids the
overhead of per-instance ``__dict__`` (important for trees with thousands
of nodes).

``IRRational`` always normalizes on construction: ``IRRational(2, 4)``
becomes ``IRRational(1, 2)``. Negative rationals keep the sign in the
numerator: ``IRRational(1, -2)`` becomes ``IRRational(-1, 2)``. Division
by zero raises ``ValueError``.

``IRApply`` stores its arguments as a ``tuple`` (not a ``list``) so the
node stays hashable. The head is an arbitrary ``IRNode``, but in practice
it is always an ``IRSymbol`` — we don't enforce this at the type level
because higher-order heads (e.g. a function returned from another call)
are conceivable in future dialects.

The standard head symbols at the bottom of this module are singletons.
Every place in the system that wants to refer to ``Add`` uses the shared
``ADD`` constant, which keeps equality checks cheap and avoids
proliferation of equivalent symbol objects.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import gcd


class IRNode:
    """Abstract base for every node in the symbolic IR.

    This class exists purely for ``isinstance()`` checks. All real node
    types are the six frozen dataclasses defined below.
    """

    __slots__ = ()


@dataclass(frozen=True, slots=True)
class IRSymbol(IRNode):
    """A named atom — a variable, constant, or operation head.

    Examples: ``IRSymbol("x")``, ``IRSymbol("Pi")``, ``IRSymbol("Add")``.
    The name is case-sensitive (like MACSYMA and Mathematica).
    """

    name: str

    def __str__(self) -> str:
        return self.name


@dataclass(frozen=True, slots=True)
class IRInteger(IRNode):
    """An arbitrary-precision integer literal.

    Python's ``int`` is already arbitrary-precision, so no bigint class is
    needed. Negative values are allowed directly; we do not wrap them in
    ``IRApply(Neg, ...)`` at the IR level — that's a surface-syntax
    concern.
    """

    value: int

    def __str__(self) -> str:
        return str(self.value)


@dataclass(frozen=True, slots=True)
class IRRational(IRNode):
    """An exact fraction numerator/denominator, always in reduced form.

    Two invariants hold after construction:

    1. ``denom > 0`` — the sign lives in the numerator.
    2. ``gcd(abs(numer), denom) == 1`` — the fraction is reduced.

    We do NOT auto-collapse rationals with denominator 1 to ``IRInteger``
    here, because that would change the constructor's return type in a
    surprising way. Callers that want that collapse should use the
    :func:`rational` factory below.
    """

    numer: int
    denom: int

    def __post_init__(self) -> None:
        # Can't mutate frozen fields directly — use object.__setattr__.
        if self.denom == 0:
            raise ValueError("IRRational denominator cannot be zero")
        numer, denom = self.numer, self.denom
        if denom < 0:
            numer, denom = -numer, -denom
        g = gcd(abs(numer), denom)
        if g > 1:
            numer //= g
            denom //= g
        object.__setattr__(self, "numer", numer)
        object.__setattr__(self, "denom", denom)

    def __str__(self) -> str:
        return f"{self.numer}/{self.denom}"


@dataclass(frozen=True, slots=True)
class IRFloat(IRNode):
    """A double-precision floating-point literal.

    Floats in a CAS are always suspicious: they destroy the exactness
    that makes symbolic computation valuable. We include ``IRFloat`` for
    completeness (MACSYMA has ``1.5`` literals) but the default
    simplification path avoids introducing them from integer/rational
    arithmetic.
    """

    value: float

    def __str__(self) -> str:
        return repr(self.value)


@dataclass(frozen=True, slots=True)
class IRString(IRNode):
    """A string literal. Rare in CAS use but present in MACSYMA output
    (e.g. ``print("x=", x)``) and in some rewrite rule conditions."""

    value: str

    def __str__(self) -> str:
        return f'"{self.value}"'


@dataclass(frozen=True, slots=True)
class IRApply(IRNode):
    """A compound expression: ``head`` applied to a tuple of ``args``.

    The single compound form in the IR. Everything from ``x + y`` to
    ``diff(f(x), x)`` to ``matrix([a, b], [c, d])`` is an ``IRApply``.
    The uniform shape is what makes tree-walking code simple.

    The args tuple is stored as-is; we do not sort or canonicalize for
    commutative operators like ``Add`` at the IR level. That's the VM's
    job — canonicalization depends on what the backend considers
    "equivalent" and should not be hardcoded here.
    """

    head: IRNode
    args: tuple[IRNode, ...]

    def __str__(self) -> str:
        return f"{self.head}({', '.join(str(a) for a in self.args)})"


# ---------------------------------------------------------------------------
# Standard head symbols — the vocabulary every backend understands.
# ---------------------------------------------------------------------------
#
# These are plain ``IRSymbol`` singletons. Using them instead of
# ``IRSymbol("Add")`` everywhere keeps equality checks cheap (identity
# comparison works) and provides a single place to discover the standard
# vocabulary.
#
# Frontends that need custom operations (a Mathematica-specific
# ``HoldForm``, for example) simply introduce new ``IRSymbol`` values;
# the VM treats them the same as the standard ones, falling back to
# "leave unevaluated" in symbolic mode.

# Arithmetic
ADD = IRSymbol("Add")
SUB = IRSymbol("Sub")
MUL = IRSymbol("Mul")
DIV = IRSymbol("Div")
POW = IRSymbol("Pow")
NEG = IRSymbol("Neg")
INV = IRSymbol("Inv")

# Elementary functions
EXP = IRSymbol("Exp")
LOG = IRSymbol("Log")
SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")
TAN = IRSymbol("Tan")
SQRT = IRSymbol("Sqrt")
ATAN = IRSymbol("Atan")
ASIN = IRSymbol("Asin")
ACOS = IRSymbol("Acos")

# Hyperbolic functions
SINH = IRSymbol("Sinh")
COSH = IRSymbol("Cosh")
TANH = IRSymbol("Tanh")
ASINH = IRSymbol("Asinh")
ACOSH = IRSymbol("Acosh")
ATANH = IRSymbol("Atanh")

# Calculus
D = IRSymbol("D")
INTEGRATE = IRSymbol("Integrate")

# Relations
EQUAL = IRSymbol("Equal")
NOT_EQUAL = IRSymbol("NotEqual")
LESS = IRSymbol("Less")
GREATER = IRSymbol("Greater")
LESS_EQUAL = IRSymbol("LessEqual")
GREATER_EQUAL = IRSymbol("GreaterEqual")

# Logic
AND = IRSymbol("And")
OR = IRSymbol("Or")
NOT = IRSymbol("Not")
IF = IRSymbol("If")

# Containers
LIST = IRSymbol("List")

# Binding
ASSIGN = IRSymbol("Assign")  # x : expr  — evaluate rhs, bind
DEFINE = IRSymbol("Define")  # f(x) := expr  — delayed, for functions
RULE = IRSymbol("Rule")  # pattern -> replacement (rewrite rules)

# Control flow (Phase G — MACSYMA grammar extensions)
#
# These five heads implement the structured-programming forms that let
# MACSYMA programs do something beyond single-expression evaluation.
#
#   While(condition, body)
#       Evaluate ``body`` repeatedly as long as ``condition`` is truthy.
#       Returns the last value of ``body`` (or ``False`` if the loop
#       never executes).
#
#   ForRange(var, start, step, end, body)
#       Equivalent to ``for var: start step step thru end do body``.
#       Binds ``var`` to ``start``, ``start+step``, … up to ``end``
#       (inclusive), evaluating ``body`` on each iteration.  Returns
#       the last body value.
#
#   ForEach(var, list, body)
#       Equivalent to ``for var in list do body``.
#       Binds ``var`` to each element of ``list`` in turn.
#       Returns the last body value.
#
#   Block(locals_list, stmt1, stmt2, …, stmtN)
#       Creates a local scope, evaluates statements in order, returns
#       the value of the last statement.  ``locals_list`` is an
#       ``IRApply(List, ...)`` whose elements are either
#       ``IRSymbol`` (declare, initialize to False) or
#       ``IRApply(Assign, sym, rhs)`` (declare, initialize to rhs).
#       Local bindings are restored on exit (even via Return).
#
#   Return(value)
#       Immediately exits the enclosing Block/While/ForRange/ForEach
#       with ``value``.  Implemented via a Python exception so it
#       unwinds cleanly through arbitrary nesting.
WHILE = IRSymbol("While")
FOR_RANGE = IRSymbol("ForRange")
FOR_EACH = IRSymbol("ForEach")
BLOCK = IRSymbol("Block")
RETURN = IRSymbol("Return")
