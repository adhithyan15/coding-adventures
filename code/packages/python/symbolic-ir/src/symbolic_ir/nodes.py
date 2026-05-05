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

# Hyperbolic functions — forward and inverse
SINH = IRSymbol("Sinh")
COSH = IRSymbol("Cosh")
TANH = IRSymbol("Tanh")
ASINH = IRSymbol("Asinh")
ACOSH = IRSymbol("Acosh")
ATANH = IRSymbol("Atanh")

# Reciprocal hyperbolic functions (Phase 15)
# coth = cosh/sinh, sech = 1/cosh, csch = 1/sinh.
# These are NOT defined as Mul(Inv(...), ...) at the IR level — they are
# first-class heads so handlers can evaluate them numerically and the
# differentiator can emit exact symbolic derivatives.
COTH = IRSymbol("Coth")
SECH = IRSymbol("Sech")
CSCH = IRSymbol("Csch")

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

# Numeric root-finding
MNEWTON = IRSymbol("MNewton")

# Laplace / Fourier transforms (cas-laplace, cas-fourier)
#
# DiracDelta and UnitStep are canonical here; any future transform package
# that needs them imports from cas_laplace.heads (which re-exports them).
# They are also registered here so that symbolic-ir consumers can import
# them without depending on cas-laplace.
DIRAC_DELTA = IRSymbol("DiracDelta")  # δ(t) — Dirac delta distribution
UNIT_STEP = IRSymbol("UnitStep")      # u(t) — Heaviside unit step function
LAPLACE = IRSymbol("Laplace")         # L{f(t)} — Laplace transform head
ILT = IRSymbol("ILT")                 # L⁻¹{F(s)} — inverse Laplace transform
FOURIER = IRSymbol("Fourier")         # F{f(t)} — Fourier transform head
IFOURIER = IRSymbol("IFourier")       # F⁻¹{F(ω)} — inverse Fourier transform

# ODE solving (cas-ode)
#
# ODE2 is the head for MACSYMA's ode2(eqn, y, x) operation.
# C_CONST, C1, C2 are the integration constants for first- and second-order
# ODEs respectively, matching MACSYMA's %c, %c1, %c2 naming convention.
ODE2 = IRSymbol("ODE2")        # ode2(eqn, y, x) — ODE solver head
C_CONST = IRSymbol("%c")       # integration constant for first-order ODEs
C1 = IRSymbol("%c1")           # first  integration constant for 2nd-order ODEs
C2 = IRSymbol("%c2")           # second integration constant for 2nd-order ODEs

# Algebraic factoring (cas-algebraic)
#
# AlgFactor is the head for MACSYMA's algfactor(poly, sqrt(d)) operation.
# It factors a univariate polynomial over the algebraic number field Q[√d].
#
# Example: AlgFactor(x^4+1, Sqrt(2)) → (x^2+√2x+1)(x^2−√2x+1)
ALG_FACTOR = IRSymbol("AlgFactor")  # algfactor(poly, sqrt(d)) head

# Multivariate polynomial operations (cas-multivariate)
#
# These three heads implement Gröbner-basis computation, polynomial
# reduction, and ideal solving for multivariate polynomial systems.
#
#   Groebner(List(polys), List(vars))
#       Compute the reduced Gröbner basis of the ideal generated by
#       ``polys`` using Buchberger's algorithm over Q.
#       Returns ``List(g1, g2, …)`` of IR polynomials.
#
#   PolyReduce(f, List(polys), List(vars))
#       Reduce ``f`` by the list of polynomials using multivariate
#       polynomial division.  Returns the remainder IR node.
#
#   IdealSolve(List(polys), List(vars))
#       Solve the polynomial system (all polys = 0) via lex Gröbner basis
#       and back-substitution.  Returns
#       ``List(List(Rule(x, v1), Rule(y, v2), …), …)`` —
#       one inner list per solution.
GROEBNER = IRSymbol("Groebner")      # groebner(polys, vars) head
POLY_REDUCE = IRSymbol("PolyReduce")  # poly_reduce(f, polys, vars) head
IDEAL_SOLVE = IRSymbol("IdealSolve")  # ideal_solve(polys, vars) head

# Simplification operations (Phase 21)
#
# These nine heads implement the MACSYMA assumption framework and the
# radical/log/exponentialize simplification family.
#
# Assumption framework:
#   Assume(relation)         — record a fact, e.g. Assume(Greater(x, 0))
#   Assume(sym, property)    — record a property, e.g. Assume(n, integer)
#   Forget(relation)         — remove a specific fact
#   Forget()                 — remove ALL recorded assumptions
#   Is(relation)             — query: returns "true" / "false" / "unknown"
#   Sign(x)                  — sign function: 1, -1, 0, or unevaluated
#
# Radical / log / exponential simplification:
#   Radcan(expr)             — canonical form for radical expressions
#   LogContract(expr)        — combine log sums into a single log
#   LogExpand(expr)          — expand a log over products / powers
#   Exponentialize(expr)     — convert trig/hyp functions to exp form
#   DeMoivre(expr)           — convert exp(a+bi) → exp(a)·(cos b + i·sin b)
ASSUME = IRSymbol("Assume")
FORGET = IRSymbol("Forget")
IS = IRSymbol("Is")
SIGN = IRSymbol("Sign")
RADCAN = IRSymbol("Radcan")
LOGCONTRACT = IRSymbol("LogContract")
LOGEXPAND = IRSymbol("LogExpand")
EXPONENTIALIZE = IRSymbol("Exponentialize")
DEMOIVRE = IRSymbol("DeMoivre")

# Pattern-matching operations (Phase 22)
#
# MACSYMA's user-defined rewrite-rule system built on top of the
# structural pattern matcher in ``cas-pattern-matching``.
#
# Pattern variable declaration:
#   MatchDeclare(sym)            — declare sym as a wildcard variable
#   MatchDeclare(sym, predicate) — declare sym with a type predicate
#                                  (integerp, symbolp, floatp, etc.)
#
# Named-rule management:
#   Defrule(name, lhs, rhs)     — compile + store a named rewrite rule
#   Apply1(name, expr)          — apply named rule once at root
#   Apply2(name, expr)          — apply named rule recursively (fixed-point)
#
# Automatic simplifier integration:
#   TellSimp(lhs, rhs)          — add a rule to the VM's auto-simplifier;
#                                  fires on every eval of matching exprs
MATCHDECLARE = IRSymbol("MatchDeclare")
DEFRULE = IRSymbol("Defrule")
APPLY1 = IRSymbol("Apply1")
APPLY2 = IRSymbol("Apply2")
TELLSIMP = IRSymbol("TellSimp")

# ── Phase 23 — Special functions ──────────────────────────────────────────
#
# Error functions:
#   Erf(x)  = (2/√π) ∫₀^x exp(-t²) dt   — the standard error function
#   Erfc(x) = 1 - Erf(x)                 — complementary error function
#   Erfi(x) = (2/√π) ∫₀^x exp(t²) dt    — imaginary error function
#             (= -i · erf(i·x))
ERF = IRSymbol("Erf")
ERFC = IRSymbol("Erfc")
ERFI = IRSymbol("Erfi")

# Trigonometric integrals:
#   Si(x)  = ∫₀^x sin(t)/t dt            — sine integral
#   Ci(x)  = γ + log(x) + ∫₀^x (cos(t)-1)/t dt  — cosine integral (γ = Euler–Mascheroni)
#   Shi(x) = ∫₀^x sinh(t)/t dt           — hyperbolic sine integral
#   Chi(x) = γ + log(x) + ∫₀^x (cosh(t)-1)/t dt  — hyperbolic cosine integral
SI = IRSymbol("Si")
CI = IRSymbol("Ci")
SHI = IRSymbol("Shi")
CHI = IRSymbol("Chi")

# Dilogarithm (Spence's function):
#   Li₂(z) = -∫₀^z log(1-t)/t dt = Σ_{k=1}^∞ z^k/k²  for |z| ≤ 1
#   Li₂(1) = π²/6  (Basel problem)
LI2 = IRSymbol("Li2")

# Gamma and Beta functions:
#   GammaFunc(n) = (n-1)!  for positive integers
#   GammaFunc(1/2) = √π
#   BetaFunc(a,b) = GammaFunc(a)·GammaFunc(b) / GammaFunc(a+b)
#
# Note: we use GammaFunc / BetaFunc rather than Gamma / Beta to avoid
# shadowing Python's built-in names in handler code.
GAMMA_FUNC = IRSymbol("GammaFunc")
BETA_FUNC = IRSymbol("BetaFunc")

# Fresnel integrals:
#   FresnelS(x) = ∫₀^x sin(π·t²/2) dt
#   FresnelC(x) = ∫₀^x cos(π·t²/2) dt
#   Both → 1/2 as x → ∞.
FRESNEL_S = IRSymbol("FresnelS")
FRESNEL_C = IRSymbol("FresnelC")

# ── Phase 25 — Symbolic summation and product ─────────────────────────────
#
# These two heads represent unevaluated (or partially evaluated) symbolic
# sums and products over a discrete index variable.  Both take exactly four
# arguments:
#
#   Sum(f, k, a, b)     ≡  Σ_{k=a}^{b} f(k)
#   Product(f, k, a, b) ≡  Π_{k=a}^{b} f(k)
#
# where:
#   f  — the summand / factor expression (may contain k)
#   k  — the index variable (must be an IRSymbol)
#   a  — the lower bound (inclusive; may be symbolic)
#   b  — the upper bound (inclusive; may be symbolic or %inf)
#
# The VM handler attempts a closed-form evaluation in this order:
#
#   1. Constant summand (f does not contain k):
#        Σ c = c · (b − a + 1)
#
#   2. Geometric series (f = coeff · base^k, base constant in k):
#        Σ_{k=a}^{b} r^k = r^a · (r^{b−a+1} − 1) / (r − 1)     [finite]
#        Σ_{k=a}^{∞}  r^k = r^a / (1 − r)                        [infinite]
#
#   3. Power of the index (f = c · k^m, m ∈ {0,…,5}):
#        Uses Faulhaber's polynomial formula F(n,m) = Σ_{k=1}^n k^m.
#        General bounds: c · [F(b,m) − F(a−1,m)].
#
#   4. Classic infinite series (b = %inf):
#        Σ_{k=1}^∞ 1/k²  = π²/6   (Basel problem, Euler 1734)
#        Σ_{k=1}^∞ 1/k⁴  = π⁴/90
#        Σ_{k=0}^∞ (−1)^k/(2k+1) = π/4  (Leibniz)
#        Σ_{k=0}^∞ 1/k!  = %e
#        Σ_{k=0}^∞ x^k/k! = exp(x)
#
#   5. Unevaluated: returns Sum/Product unchanged.
#
# Product-specific closed forms:
#   Π_{k=1}^{n} k   = GammaFunc(n+1)      (factorial)
#   Π_{k=a}^{b} c   = c^(b−a+1)          (constant factor)
SUM = IRSymbol("Sum")
PRODUCT = IRSymbol("Product")
