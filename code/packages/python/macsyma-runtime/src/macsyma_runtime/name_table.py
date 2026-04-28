"""Name-table extensions consumed by the MACSYMA compiler.

The base ``macsyma-compiler`` package ships a small ``_STANDARD_FUNCTIONS``
map covering the elementary operations (``diff``, ``integrate``, ``sin``,
``cos``, ...). The runtime wants to map a much larger set of MACSYMA
identifiers — ``factor``, ``expand``, ``simplify``, ``subst``, ``solve``,
``taylor``, ``limit``, ``length``, ``first``, etc. — to canonical IR
heads. This table is the central place where those mappings live.

The substrate handlers for each of these heads may not yet be
implemented; until they are, MACSYMA users will see e.g.
``Expand(x^2 + 2*x + 1)`` returned unevaluated, which is the same
fall-through behavior every CAS uses for unknown operations. Once the
substrate package lands, the routing is automatically picked up — no
compiler change required.
"""

from __future__ import annotations

from symbolic_ir import IRSymbol

# IR heads from substrate packages that may not exist yet — define
# them here as :class:`IRSymbol` singletons so the table can reference
# them. When the substrate package lands, it can re-export the same
# symbol (or a binding-equivalent one) without breaking compatibility.
SUBST = IRSymbol("Subst")
SIMPLIFY = IRSymbol("Simplify")
EXPAND = IRSymbol("Expand")
FACTOR = IRSymbol("Factor")
SOLVE = IRSymbol("Solve")
NSOLVE = IRSymbol("NSolve")
TAYLOR = IRSymbol("Taylor")
LIMIT = IRSymbol("Limit")

LENGTH = IRSymbol("Length")
FIRST = IRSymbol("First")
REST = IRSymbol("Rest")
LAST = IRSymbol("Last")
APPEND = IRSymbol("Append")
REVERSE = IRSymbol("Reverse")
RANGE = IRSymbol("Range")
MAP = IRSymbol("Map")
APPLY = IRSymbol("Apply")
SELECT = IRSymbol("Select")
SORT = IRSymbol("Sort")
PART = IRSymbol("Part")
FLATTEN = IRSymbol("Flatten")
JOIN = IRSymbol("Join")

MATRIX = IRSymbol("Matrix")
TRANSPOSE = IRSymbol("Transpose")
DETERMINANT = IRSymbol("Determinant")
INVERSE = IRSymbol("Inverse")

GCD = IRSymbol("Gcd")
LCM = IRSymbol("Lcm")
MOD = IRSymbol("Mod")
FLOOR = IRSymbol("Floor")
CEILING = IRSymbol("Ceiling")
ABS = IRSymbol("Abs")

# Equation-side selectors (C5)
LHS = IRSymbol("Lhs")
RHS = IRSymbol("Rhs")

# Generative list construction (C2)
MAKE_LIST = IRSymbol("MakeList")

# Point evaluation (C4)
AT = IRSymbol("At")

# Number theory (B3)
IS_PRIME = IRSymbol("IsPrime")
NEXT_PRIME = IRSymbol("NextPrime")
PREV_PRIME = IRSymbol("PrevPrime")
FACTOR_INTEGER = IRSymbol("FactorInteger")
DIVISORS = IRSymbol("Divisors")
TOTIENT = IRSymbol("Totient")
MOEBIUS_MU = IRSymbol("MoebiusMu")
JACOBI_SYMBOL = IRSymbol("JacobiSymbol")
CHINESE_REMAINDER = IRSymbol("ChineseRemainder")
INTEGER_LENGTH = IRSymbol("IntegerLength")

# Numeric root-finding (Newton's method)
MNEWTON = IRSymbol("MNewton")

# Trig transformation heads (B1)
TRIG_SIMPLIFY = IRSymbol("TrigSimplify")
TRIG_EXPAND = IRSymbol("TrigExpand")
TRIG_REDUCE = IRSymbol("TrigReduce")

# Rational function operations (A3)
COLLECT = IRSymbol("Collect")
TOGETHER = IRSymbol("Together")
RAT_SIMPLIFY = IRSymbol("RatSimplify")
APART = IRSymbol("Apart")

# Complex number IR heads (B2)
IMAGINARY_UNIT = IRSymbol("ImaginaryUnit")
RE = IRSymbol("Re")
IM = IRSymbol("Im")
CONJUGATE = IRSymbol("Conjugate")
ARG = IRSymbol("Arg")
RECT_FORM = IRSymbol("RectForm")
POLAR_FORM = IRSymbol("PolarForm")

# Re-export the runtime-owned heads so callers have one import.
from macsyma_runtime.heads import (  # noqa: E402
    ASSUME,
    BLOCK,
    EV,
    FORGET,
    IS,
    KILL,
)

# The map MACSYMA users see → canonical IR head.
#
# The MACSYMA compiler reads ``_STANDARD_FUNCTIONS`` (a similar table
# scoped to the lexicon it ships with). The runtime's
# :func:`extend_compiler_name_table` adds these on top.
MACSYMA_NAME_TABLE: dict[str, IRSymbol] = {
    # Algebraic operations
    "subst": SUBST,
    "simplify": SIMPLIFY,
    "expand": EXPAND,
    "factor": FACTOR,
    "solve": SOLVE,
    "nsolve": NSOLVE,
    "linsolve": SOLVE,  # MACSYMA's linsolve is linear-system solving
    "taylor": TAYLOR,
    "limit": LIMIT,
    # List operations
    "length": LENGTH,
    "first": FIRST,
    "rest": REST,
    "last": LAST,
    "append": APPEND,
    "reverse": REVERSE,
    "makelist": MAKE_LIST,
    "map": MAP,
    "apply": APPLY,
    "sublist": SELECT,
    "sort": SORT,
    "part": PART,
    "flatten": FLATTEN,
    "join": JOIN,
    # Matrix
    "matrix": MATRIX,
    "transpose": TRANSPOSE,
    "determinant": DETERMINANT,
    "invert": INVERSE,
    # Newton's method numeric root finder
    "mnewton": MNEWTON,
    # Number-theoretic
    "gcd": GCD,
    "lcm": LCM,
    "mod": MOD,
    "floor": FLOOR,
    "ceiling": CEILING,
    "abs": ABS,
    # Equation-side selectors (C5)
    "lhs": LHS,
    "rhs": RHS,
    # Point evaluation — At(expr, Equal(var, val)) (C4)
    "at": AT,
    # Number theory (B3)
    "primep": IS_PRIME,   # canonical MACSYMA name
    "is_prime": IS_PRIME,  # common alias used in interactive sessions
    "next_prime": NEXT_PRIME,
    "prev_prime": PREV_PRIME,
    "ifactor": FACTOR_INTEGER,
    "divisors": DIVISORS,
    "totient": TOTIENT,
    "moebius": MOEBIUS_MU,
    "jacobi": JACOBI_SYMBOL,
    "chinese": CHINESE_REMAINDER,
    "numdigits": INTEGER_LENGTH,
    # Trig transformation operations (B1)
    "trigsimp": TRIG_SIMPLIFY,
    "trigexpand": TRIG_EXPAND,
    "trigreduce": TRIG_REDUCE,
    # Rational function operations (A3)
    "collect": COLLECT,
    "together": TOGETHER,
    "ratsimp": RAT_SIMPLIFY,
    "partfrac": APART,
    # Complex number operations (B2)
    # %i is the imaginary unit constant; the compiler maps the token to
    # IMAGINARY_UNIT so the VM finds the pre-bound symbol.
    "%i": IMAGINARY_UNIT,
    "realpart": RE,
    "imagpart": IM,
    "conjugate": CONJUGATE,
    # cabs(z) = complex modulus; Abs dispatches to complex handler when z
    # contains ImaginaryUnit, so both names route to the same IR head.
    "cabs": ABS,
    "carg": ARG,
    "rectform": RECT_FORM,
    "polarform": POLAR_FORM,
    # Runtime-owned operations
    "kill": KILL,
    "ev": EV,
    "block": BLOCK,
    "assume": ASSUME,
    "forget": FORGET,
    "is": IS,
}


def extend_compiler_name_table(target: dict[str, IRSymbol]) -> None:
    """Merge :data:`MACSYMA_NAME_TABLE` into a compiler's name dict.

    Used at REPL startup so the compiler's ``_STANDARD_FUNCTIONS`` map
    knows about every name introduced here. Idempotent — calling twice
    with the same target is safe.
    """
    target.update(MACSYMA_NAME_TABLE)
