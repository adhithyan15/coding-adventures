"""Symbolic IR — the universal expression tree for computer algebra systems.

Every CAS frontend (MACSYMA, Mathematica, Maple, REDUCE, SymPy syntax)
compiles to this single IR. Every CAS backend (strict evaluation,
symbolic rewriting, LaTeX rendering, transpilation) consumes this IR.

The IR has exactly six node types:

- ``IRSymbol(name)``        — an identifier: ``x``, ``%pi``, ``Add``.
- ``IRInteger(value)``      — an arbitrary-precision integer.
- ``IRRational(n, d)``      — an exact fraction, always reduced.
- ``IRFloat(value)``        — a double-precision float.
- ``IRString(value)``       — a string literal.
- ``IRApply(head, args)``   — an operation: head(args...).

All nodes are immutable and hashable. The compound form is always
``IRApply`` — there are no `BinaryOp` or `FunctionCall` sub-types. The
head of an ``IRApply`` is always an ``IRSymbol`` (by convention), which
keeps the structure uniformly Lisp-like.

Example::

    from symbolic_ir import IRSymbol, IRInteger, IRApply, ADD, POW

    # Represent x^2 + 1
    x = IRSymbol("x")
    expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
"""

from symbolic_ir.nodes import (
    ACOS,
    ACOSH,
    ADD,
    ALG_FACTOR,
    AND,
    COTH,
    CSCH,
    GROEBNER,
    IDEAL_SOLVE,
    POLY_REDUCE,
    ASIN,
    ASINH,
    ASSIGN,
    ATAN,
    ATANH,
    BLOCK,
    SECH,
    C1,
    C2,
    C_CONST,
    COS,
    COSH,
    DEFINE,
    DIRAC_DELTA,
    DIV,
    EQUAL,
    EXP,
    FOR_EACH,
    FOR_RANGE,
    FOURIER,
    GREATER,
    GREATER_EQUAL,
    IF,
    IFOURIER,
    ILT,
    INTEGRATE,
    INV,
    LAPLACE,
    LESS,
    LESS_EQUAL,
    LIST,
    LOG,
    MNEWTON,
    MUL,
    NEG,
    NOT,
    NOT_EQUAL,
    ODE2,
    OR,
    POW,
    RETURN,
    RULE,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    UNIT_STEP,
    WHILE,
    D,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

__all__ = [
    "IRNode",
    "IRSymbol",
    "IRInteger",
    "IRRational",
    "IRFloat",
    "IRString",
    "IRApply",
    # Standard head symbols
    "ACOS",
    "ALG_FACTOR",
    "ACOSH",
    "ASIN",
    "ASINH",
    "ATAN",
    "ATANH",
    "ADD",
    "SUB",
    "MUL",
    "DIV",
    "POW",
    "NEG",
    "INV",
    "EXP",
    "LOG",
    "SIN",
    "SINH",
    "COS",
    "COSH",
    "TAN",
    "TANH",
    # Reciprocal hyperbolic functions (Phase 15)
    "COTH",
    "SECH",
    "CSCH",
    "SQRT",
    "D",
    "INTEGRATE",
    "EQUAL",
    "NOT_EQUAL",
    "LESS",
    "GREATER",
    "LESS_EQUAL",
    "GREATER_EQUAL",
    "AND",
    "OR",
    "NOT",
    "IF",
    "LIST",
    "ASSIGN",
    "DEFINE",
    "RULE",
    # Control flow (Phase G)
    "WHILE",
    "FOR_RANGE",
    "FOR_EACH",
    "BLOCK",
    "RETURN",
    # Numeric root-finding
    "MNEWTON",
    # Laplace / Fourier transforms
    "DIRAC_DELTA",
    "UNIT_STEP",
    "LAPLACE",
    "ILT",
    "FOURIER",
    "IFOURIER",
    # ODE solving (cas-ode)
    "ODE2",
    "C_CONST",
    "C1",
    "C2",
    # Multivariate polynomial operations (cas-multivariate)
    "GROEBNER",
    "POLY_REDUCE",
    "IDEAL_SOLVE",
]
