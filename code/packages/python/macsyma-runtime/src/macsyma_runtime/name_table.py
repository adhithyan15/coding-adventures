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
