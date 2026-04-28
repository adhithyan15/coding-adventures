"""Dialect protocol and `BaseDialect` defaults.

A dialect tells the walker how to spell things in a particular CAS
language. The walker is dialect-agnostic; the dialect supplies:

- numeric formats (integers, rationals, floats, strings),
- operator spellings (binary like `+`, unary like `-`),
- function-call shape (parens vs. brackets, function name aliases),
- list and call brackets,
- a precedence table — used by the walker to decide where parens go.

A dialect also gets a chance to apply *surface sugar* (e.g. turn
``Add(x, Neg(y))`` into ``x - y``) before the walker dispatches —
implementations can override :py:meth:`BaseDialect.try_sugar` for that.

Most dialects subclass :class:`BaseDialect` and override only the
spelling tables. See :mod:`cas_pretty_printer.macsyma` for a worked
example.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from symbolic_ir import IRApply, IRNode

# Default precedence levels — higher numbers bind tighter.
# These are the canonical values most CAS languages use; a dialect can
# override `precedence()` to change them.
PREC_OR = 10
PREC_AND = 20
PREC_NOT = 25
PREC_CMP = 30
PREC_ADD = 40
PREC_MUL = 50
PREC_NEG = 55
PREC_POW = 60
PREC_CALL = 70
PREC_ATOM = 100


@runtime_checkable
class Dialect(Protocol):
    """The minimal protocol every dialect must satisfy.

    Implementers should usually subclass :class:`BaseDialect` rather
    than implement this directly.
    """

    name: str

    # Numeric formatting -----------------------------------------------------
    def format_integer(self, value: int) -> str: ...
    def format_rational(self, numer: int, denom: int) -> str: ...
    def format_float(self, value: float) -> str: ...
    def format_string(self, value: str) -> str: ...
    def format_symbol(self, name: str) -> str: ...

    # Operator spellings — return None to fall back to function-call form.
    def binary_op(self, head_name: str) -> str | None: ...
    def unary_op(self, head_name: str) -> str | None: ...

    # Function-call spelling.
    def function_name(self, head_name: str) -> str: ...

    # Container delimiters — (open, close) pairs.
    def list_brackets(self) -> tuple[str, str]: ...
    def call_brackets(self) -> tuple[str, str]: ...

    # Precedence table.
    def precedence(self, head_name: str) -> int: ...
    def is_right_associative(self, head_name: str) -> bool: ...

    # Surface sugar hook — return a rewritten IRApply or None.
    def try_sugar(self, node: IRApply) -> IRNode | None: ...


# Precedence assignments by head name. Used by `BaseDialect.precedence`.
_DEFAULT_PRECEDENCE: dict[str, int] = {
    "Or": PREC_OR,
    "And": PREC_AND,
    "Not": PREC_NOT,
    "Equal": PREC_CMP,
    "NotEqual": PREC_CMP,
    "Less": PREC_CMP,
    "Greater": PREC_CMP,
    "LessEqual": PREC_CMP,
    "GreaterEqual": PREC_CMP,
    "Add": PREC_ADD,
    "Sub": PREC_ADD,
    "Mul": PREC_MUL,
    "Div": PREC_MUL,
    "Neg": PREC_NEG,
    "Pow": PREC_POW,
}

# Heads that are right-associative when they appear in chains.
_RIGHT_ASSOC: frozenset[str] = frozenset({"Pow"})

# Default operator spellings. Most language dialects share these.
_DEFAULT_BINARY: dict[str, str] = {
    "Add": " + ",
    "Sub": " - ",
    "Mul": "*",
    "Div": "/",
    "Pow": "^",
    "Or": " or ",
    "And": " and ",
    "Equal": " = ",
    "NotEqual": " # ",
    "Less": " < ",
    "Greater": " > ",
    "LessEqual": " <= ",
    "GreaterEqual": " >= ",
}

_DEFAULT_UNARY: dict[str, str] = {
    "Neg": "-",
    "Not": "not ",
}

# Function-name aliasing — IR head name → surface name. Most dialects
# use lowercase versions of the IR heads; Mathematica is one exception.
#
# Every CAS head that may appear in an un-evaluated or partially-evaluated
# IR tree should have an entry here so that the pretty-printer produces a
# round-trippable surface spelling.  Dialect subclasses can override
# individual entries (e.g. MACSYMA uses ``realpart`` instead of ``re``).
_DEFAULT_FUNCTION_NAMES: dict[str, str] = {
    # ---- elementary functions -----------------------------------------------
    "Sin": "sin",
    "Cos": "cos",
    "Tan": "tan",
    "Exp": "exp",
    "Log": "log",
    "Sqrt": "sqrt",
    "Cbrt": "cbrt",          # cube root — Cbrt(8) = 2
    "Abs": "abs",
    "Asin": "asin",
    "Acos": "acos",
    "Atan": "atan",
    "Sinh": "sinh",
    "Cosh": "cosh",
    "Tanh": "tanh",
    "Asinh": "asinh",
    "Acosh": "acosh",
    "Atanh": "atanh",
    # ---- calculus / algebra -------------------------------------------------
    "D": "diff",
    "Integrate": "integrate",
    "Simplify": "simplify",
    "Expand": "expand",
    "Factor": "factor",
    "Collect": "collect",
    "Together": "together",
    "RatSimplify": "ratsimp",
    "Apart": "apart",
    "Subst": "subst",
    "Solve": "solve",
    "NSolve": "nsolve",
    "Limit": "limit",
    "Taylor": "taylor",
    # ---- trig simplification ------------------------------------------------
    "TrigSimplify": "trigsimp",
    "TrigExpand": "trigexpand",
    "TrigReduce": "trigreduce",
    # ---- list operations ----------------------------------------------------
    "Length": "length",
    "First": "first",
    "Rest": "rest",
    "Last": "last",
    "Append": "append",
    "Reverse": "reverse",
    "Range": "range",
    "Map": "map",
    "Apply": "apply",
    "Select": "select",
    "Sort": "sort",
    "Part": "part",
    "Flatten": "flatten",
    "Join": "join",
    "MakeList": "makelist",
    # ---- matrix operations --------------------------------------------------
    "Matrix": "matrix",
    "Transpose": "transpose",
    "Determinant": "determinant",
    "Inverse": "inverse",
    # ---- numeric root-finding -----------------------------------------------
    "MNewton": "mnewton",
    # ---- arithmetic / numeric helpers ---------------------------------------
    "Gcd": "gcd",
    "Lcm": "lcm",
    "Mod": "mod",
    "Floor": "floor",
    "Ceiling": "ceiling",
    # ---- equation helpers ---------------------------------------------------
    "Lhs": "lhs",
    "Rhs": "rhs",
    "At": "at",
    # ---- complex number operations ------------------------------------------
    "Re": "re",
    "Im": "im",
    "Conjugate": "conjugate",
    "Arg": "arg",
    "RectForm": "rectform",
    "PolarForm": "polarform",
    # ---- number theory ------------------------------------------------------
    "IsPrime": "isprime",
    "NextPrime": "nextprime",
    "PrevPrime": "prevprime",
    "FactorInteger": "factorinteger",
    "Divisors": "divisors",
    "Totient": "totient",
    "MoebiusMu": "moebiusmu",
    "JacobiSymbol": "jacobi",
    "ChineseRemainder": "chineseremainder",
    "IntegerLength": "integerlength",
    # ---- Laplace transforms -------------------------------------------------
    "Laplace": "laplace",
    "ILT": "ilt",
    "DiracDelta": "delta",
    "UnitStep": "hstep",
    # ---- Fourier transforms -------------------------------------------------
    "Fourier": "fourier",
    "IFourier": "ifourier",
}


class BaseDialect:
    """Default implementations for every method on :class:`Dialect`.

    Subclasses typically override only ``name`` and the small spelling
    tables (``binary_ops``, ``unary_ops``, ``function_names``). The
    walker calls every method via this base, so individual subclasses
    can stay short.
    """

    name: str = "base"

    # Tables that subclasses may copy and modify.
    binary_ops: dict[str, str] = _DEFAULT_BINARY
    unary_ops: dict[str, str] = _DEFAULT_UNARY
    function_names: dict[str, str] = _DEFAULT_FUNCTION_NAMES
    precedence_table: dict[str, int] = _DEFAULT_PRECEDENCE
    right_assoc: frozenset[str] = _RIGHT_ASSOC

    # ---- numeric ----------------------------------------------------------

    def format_integer(self, value: int) -> str:
        return str(value)

    def format_rational(self, numer: int, denom: int) -> str:
        # Rationals like `-3/4` need parens around the numerator if
        # negative AND the result will be embedded in a larger expression.
        # The walker handles that via precedence; here we just render the
        # raw text.
        return f"{numer}/{denom}"

    def format_float(self, value: float) -> str:
        return repr(value)

    def format_string(self, value: str) -> str:
        return f'"{value}"'

    def format_symbol(self, name: str) -> str:
        return name

    # ---- operators --------------------------------------------------------

    def binary_op(self, head_name: str) -> str | None:
        return self.binary_ops.get(head_name)

    def unary_op(self, head_name: str) -> str | None:
        return self.unary_ops.get(head_name)

    def function_name(self, head_name: str) -> str:
        return self.function_names.get(head_name, head_name)

    # ---- containers -------------------------------------------------------

    def list_brackets(self) -> tuple[str, str]:
        return ("[", "]")

    def call_brackets(self) -> tuple[str, str]:
        return ("(", ")")

    # ---- precedence -------------------------------------------------------

    def precedence(self, head_name: str) -> int:
        # Unknown heads are function calls — bind as tightly as atoms.
        return self.precedence_table.get(head_name, PREC_CALL)

    def is_right_associative(self, head_name: str) -> bool:
        return head_name in self.right_assoc

    # ---- sugar ------------------------------------------------------------

    def try_sugar(self, node: IRApply) -> IRNode | None:
        """Hook for dialect-specific surface rewrites.

        Default: no sugar. Math dialects override this to convert
        ``Add(x, Neg(y))`` to a `Sub`, ``Mul(x, Inv(y))`` to a `Div`,
        and ``Mul(-1, x)`` to a `Neg`. See :class:`MathDialect`.
        """
        return None
