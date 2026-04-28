"""VM handlers for multivariate polynomial operations.

This module provides three handlers that integrate with the symbolic VM:

``Groebner(List(polys), List(vars))``
    Compute the Gröbner basis of a polynomial ideal.
    Returns ``List(g1, g2, …)`` where each gᵢ is an IR polynomial.

``PolyReduce(f, List(polys), List(vars))``
    Reduce the polynomial ``f`` by the list of polynomials.
    Returns the remainder IR node.

``IdealSolve(List(polys), List(vars))``
    Solve the polynomial system (find all common roots).
    Returns ``List(List(Rule(x, val1), Rule(y, val2), …), …)``
    — one inner list per solution.

All handlers follow the *graceful fall-through* contract: if they receive
unexpected input (wrong arity, non-polynomial expression, unsolvable system)
they return the ``expr`` unchanged so the VM renders it unevaluated.

IR ↔ MPoly conversion
---------------------
The conversion walks the IR tree recursively.  Supported IR shapes:

- ``IRInteger(n)``           → constant MPoly
- ``IRRational(p, q)``       → constant MPoly with Fraction(p, q)
- ``IRSymbol(name)``         → single-variable monomial  x_{var_list.index(name)}
- ``IRApply(ADD, args)``     → sum of sub-polys
- ``IRApply(MUL, args)``     → product of sub-polys
- ``IRApply(POW, (b, exp))`` → polynomial power (exp must be IRInteger ≥ 0)
- ``IRApply(NEG, (arg,))``   → negation
- ``IRApply(SUB, (a, b))``   → a - b

Anything else raises ``ConversionError``.

MPoly → IR conversion
---------------------
Iterates the coefficient dict in descending grlex order to build a
canonical IR tree:  ``Add(Mul(coeff, x^a, y^b, …), …)``.
"""

from __future__ import annotations

from fractions import Fraction
from typing import TYPE_CHECKING

from symbolic_ir import (
    ADD,
    LIST,
    MUL,
    NEG,
    POW,
    RULE,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_multivariate.groebner import GrobnerError, buchberger
from cas_multivariate.polynomial import MPoly
from cas_multivariate.reduce import reduce_poly
from cas_multivariate.solve import ideal_solve

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


# ---------------------------------------------------------------------------
# IR head singletons for this package
# ---------------------------------------------------------------------------

GROEBNER: IRSymbol = IRSymbol("Groebner")
POLY_REDUCE: IRSymbol = IRSymbol("PolyReduce")
IDEAL_SOLVE: IRSymbol = IRSymbol("IdealSolve")


# ---------------------------------------------------------------------------
# Conversion error
# ---------------------------------------------------------------------------


class ConversionError(Exception):
    """Raised when an IR node cannot be converted to an MPoly.

    The caller should catch this and return the expression unevaluated.
    """


# ---------------------------------------------------------------------------
# IR → MPoly
# ---------------------------------------------------------------------------


def _ir_to_mpoly(node: IRNode, var_list: list[str]) -> MPoly:
    """Recursively convert an IR expression to an MPoly.

    Parameters
    ----------
    node:
        The IR node to convert.  Supported shapes are documented in the
        module docstring above.
    var_list:
        Ordered list of variable names (e.g. ``["x", "y"]``).  The index
        of a variable in this list determines which exponent position it
        occupies in the monomial tuple.

    Returns
    -------
    The corresponding :class:`~cas_multivariate.polynomial.MPoly`.

    Raises
    ------
    ConversionError
        If the node contains a shape not supported (e.g. ``Sin(x)``).
    """
    nvars = len(var_list)

    if isinstance(node, IRInteger):
        return MPoly.constant(Fraction(node.value), nvars)

    if isinstance(node, IRRational):
        return MPoly.constant(Fraction(node.numer, node.denom), nvars)

    if isinstance(node, IRSymbol):
        if node.name in var_list:
            idx = var_list.index(node.name)
            exp: tuple[int, ...] = tuple(1 if i == idx else 0 for i in range(nvars))
            return MPoly({exp: Fraction(1)}, nvars)
        # Could be a constant like %pi or %e — not a polynomial.
        raise ConversionError(
            f"Unrecognised symbol in polynomial context: {node.name!r}"
        )

    if isinstance(node, IRApply):
        head = node.head

        if head == ADD:
            # n-ary sum
            if not node.args:
                return MPoly.zero(nvars)
            result = _ir_to_mpoly(node.args[0], var_list)
            for arg in node.args[1:]:
                result = result + _ir_to_mpoly(arg, var_list)
            return result

        if head == SUB:
            # Binary difference
            if len(node.args) != 2:
                raise ConversionError(f"Sub expects 2 args, got {len(node.args)}")
            return (
                _ir_to_mpoly(node.args[0], var_list)
                - _ir_to_mpoly(node.args[1], var_list)
            )

        if head == MUL:
            # n-ary product
            if not node.args:
                return MPoly.constant(Fraction(1), nvars)
            result = _ir_to_mpoly(node.args[0], var_list)
            for arg in node.args[1:]:
                result = result * _ir_to_mpoly(arg, var_list)
            return result

        if head == NEG:
            # Unary negation
            if len(node.args) != 1:
                raise ConversionError(f"Neg expects 1 arg, got {len(node.args)}")
            return -_ir_to_mpoly(node.args[0], var_list)

        if head == POW:
            # Pow(base, exp) — exp must be a non-negative IRInteger
            if len(node.args) != 2:
                raise ConversionError(f"Pow expects 2 args, got {len(node.args)}")
            base_node, exp_node = node.args
            if not isinstance(exp_node, IRInteger):
                raise ConversionError(
                    f"Pow exponent must be an integer, got {type(exp_node).__name__}"
                )
            exp_val = exp_node.value
            if exp_val < 0:
                raise ConversionError(
                    f"Negative exponent {exp_val} not allowed in polynomial"
                )
            if exp_val == 0:
                return MPoly.constant(Fraction(1), nvars)
            base_poly = _ir_to_mpoly(base_node, var_list)
            result = MPoly.constant(Fraction(1), nvars)
            for _ in range(exp_val):
                result = result * base_poly
            return result

    raise ConversionError(
        f"Cannot convert IR node to polynomial: {type(node).__name__}({node})"
    )


# ---------------------------------------------------------------------------
# MPoly → IR
# ---------------------------------------------------------------------------


def _frac_to_ir(f: Fraction) -> IRNode:
    """Convert a Fraction to an IRInteger or IRRational node.

    Example::

        _frac_to_ir(Fraction(3))      # → IRInteger(3)
        _frac_to_ir(Fraction(1, 2))   # → IRRational(1, 2)
        _frac_to_ir(Fraction(-1, 2))  # → IRRational(-1, 2)
    """
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def _mpoly_to_ir(p: MPoly, var_symbols: list[IRSymbol]) -> IRNode:
    """Convert an MPoly to an IR expression tree.

    The output is a sum of terms, each term a product of coefficient and
    variable powers.  We use grlex order (descending) for canonical output.

    Example::

        # 3*x^2*y + (1/2)*y + 1  in Q[x, y]
        # Produces: Add(Add(Mul(3, Pow(x, 2), y), Mul(1/2, y)), 1)
    """
    if p.is_zero():
        return IRInteger(0)

    terms: list[IRNode] = []
    for monomial in p.monomials_descending("grlex"):
        coeff = p.coeffs[monomial]
        # Build the variable-power part for this monomial.
        var_parts: list[IRNode] = []
        for i, exp in enumerate(monomial):
            if exp == 0:
                continue
            v = var_symbols[i]
            if exp == 1:
                var_parts.append(v)
            else:
                var_parts.append(IRApply(POW, (v, IRInteger(exp))))

        if not var_parts:
            # Pure constant term.
            terms.append(_frac_to_ir(coeff))
        else:
            # Coefficient * variable powers.
            if coeff == 1:
                # Coefficient 1: omit the scalar.
                if len(var_parts) == 1:
                    terms.append(var_parts[0])
                else:
                    term: IRNode = var_parts[0]
                    for vp in var_parts[1:]:
                        term = IRApply(MUL, (term, vp))
                    terms.append(term)
            elif coeff == -1:
                # Coefficient -1: use Neg.
                if len(var_parts) == 1:
                    terms.append(IRApply(NEG, (var_parts[0],)))
                else:
                    inner: IRNode = var_parts[0]
                    for vp in var_parts[1:]:
                        inner = IRApply(MUL, (inner, vp))
                    terms.append(IRApply(NEG, (inner,)))
            else:
                # General coefficient.
                parts: list[IRNode] = [_frac_to_ir(coeff)] + var_parts
                if len(parts) == 1:
                    terms.append(parts[0])
                else:
                    acc: IRNode = parts[0]
                    for part in parts[1:]:
                        acc = IRApply(MUL, (acc, part))
                    terms.append(acc)

    if not terms:
        return IRInteger(0)
    if len(terms) == 1:
        return terms[0]
    acc_ir: IRNode = terms[0]
    for t in terms[1:]:
        acc_ir = IRApply(ADD, (acc_ir, t))
    return acc_ir


# ---------------------------------------------------------------------------
# Helper: extract variable list from IR List node
# ---------------------------------------------------------------------------


def _extract_var_list(node: IRNode) -> list[str] | None:
    """Extract a list of variable names from an ``IRApply(List, ...)`` node.

    Returns ``None`` if the node is not a ``List`` of ``IRSymbol``s.

    Example::

        _extract_var_list(IRApply(LIST, (IRSymbol("x"), IRSymbol("y"))))
        # → ["x", "y"]
    """
    if not isinstance(node, IRApply):
        return None
    if node.head != LIST:
        return None
    names: list[str] = []
    for arg in node.args:
        if not isinstance(arg, IRSymbol):
            return None
        names.append(arg.name)
    return names


def _extract_poly_list(node: IRNode, var_list: list[str]) -> list[MPoly] | None:
    """Extract a list of MPoly from an ``IRApply(List, ...)`` of poly IR nodes.

    Returns ``None`` if any element cannot be converted.

    Example::

        # List(Add(x, y, -1), Sub(x, y)) with vars ["x","y"]
        # → [MPoly for x+y-1, MPoly for x-y]
    """
    if not isinstance(node, IRApply):
        return None
    if node.head != LIST:
        return None
    polys: list[MPoly] = []
    for arg in node.args:
        try:
            polys.append(_ir_to_mpoly(arg, var_list))
        except ConversionError:
            return None
    return polys


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def groebner_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Groebner(List(polys), List(vars))`` — compute the Gröbner basis.

    Evaluates the polynomial list and variable list, then runs Buchberger's
    algorithm.  Returns a ``List(…)`` of IR polynomial expressions.

    Falls through to unevaluated if:
    - Wrong number of arguments (not 2).
    - Second argument is not a List of Symbols.
    - First argument is not a List of convertible polynomials.
    - The Gröbner computation exceeds safety limits.

    Example IR call::

        Groebner(
            List(Add(Mul(x,x), y, -1), Add(x, Mul(y,y), -1)),
            List(x, y)
        )
        → List(...)   -- reduced Gröbner basis
    """
    if len(expr.args) != 2:
        return expr

    poly_list_node, var_list_node = expr.args

    # Extract variable names.
    var_list = _extract_var_list(var_list_node)
    if var_list is None or len(var_list) == 0:
        return expr

    # Extract polynomials.
    polys = _extract_poly_list(poly_list_node, var_list)
    if polys is None:
        return expr

    # Run Buchberger.
    try:
        basis = buchberger(polys, order="grlex")
    except GrobnerError:
        return expr

    # Convert each basis element back to IR.
    var_symbols = [IRSymbol(name) for name in var_list]
    basis_irs: list[IRNode] = [_mpoly_to_ir(g, var_symbols) for g in basis]
    return IRApply(LIST, tuple(basis_irs))


def poly_reduce_handler(vm: VM, expr: IRApply) -> IRNode:
    """``PolyReduce(f, List(polys), List(vars))`` — reduce f by the divisors.

    Returns the remainder of ``f`` after reduction by the polynomial list.

    Falls through to unevaluated if:
    - Wrong number of arguments (not 3).
    - Second or third argument is not a List.
    - Any polynomial cannot be converted.
    - Safety limits exceeded.

    Example IR call::

        PolyReduce(
            Mul(x, x),                        # f = x^2
            List(Add(x, -1)),                 # G = [x - 1]
            List(x)
        )
        → 1   (since x^2 = (x-1)(x+1) + 1... wait, with [x-1]: x^2 mod (x-1) = 1)
    """
    if len(expr.args) != 3:
        return expr

    f_node, poly_list_node, var_list_node = expr.args

    var_list = _extract_var_list(var_list_node)
    if var_list is None or len(var_list) == 0:
        return expr

    try:
        f_poly = _ir_to_mpoly(f_node, var_list)
    except ConversionError:
        return expr

    polys = _extract_poly_list(poly_list_node, var_list)
    if polys is None:
        return expr

    try:
        remainder = reduce_poly(f_poly, polys, order="grlex")
    except GrobnerError:
        return expr

    var_symbols = [IRSymbol(name) for name in var_list]
    return _mpoly_to_ir(remainder, var_symbols)


def ideal_solve_handler(vm: VM, expr: IRApply) -> IRNode:
    """``IdealSolve(List(polys), List(vars))`` — solve the polynomial system.

    Returns a list of solutions:
    ``List(List(Rule(x, val1), Rule(y, val2), …), …)``

    Falls through to unevaluated if:
    - Wrong number of arguments (not 2).
    - System cannot be solved (complex roots, non-triangular, etc.).

    Example IR call::

        IdealSolve(
            List(Add(x, y, -1), Sub(x, y)),
            List(x, y)
        )
        → List(List(Rule(x, 1/2), Rule(y, 1/2)))
    """
    if len(expr.args) != 2:
        return expr

    poly_list_node, var_list_node = expr.args

    var_list = _extract_var_list(var_list_node)
    if var_list is None or len(var_list) == 0:
        return expr

    polys = _extract_poly_list(poly_list_node, var_list)
    if polys is None:
        return expr

    solutions = ideal_solve(polys, order="lex")
    if solutions is None:
        return expr

    # Build IR: List(List(Rule(x, val), …), …)
    var_symbols = [IRSymbol(name) for name in var_list]
    solution_irs: list[IRNode] = []
    for sol in solutions:
        if len(sol) != len(var_list):
            continue  # Malformed — skip.
        rules: list[IRNode] = [
            IRApply(RULE, (var_symbols[i], _frac_to_ir(sol[i])))
            for i in range(len(var_list))
        ]
        solution_irs.append(IRApply(LIST, tuple(rules)))

    if not solution_irs:
        return expr

    return IRApply(LIST, tuple(solution_irs))


# ---------------------------------------------------------------------------
# Public builder
# ---------------------------------------------------------------------------


def build_multivariate_handler_table() -> dict[str, object]:
    """Return the handler table for multivariate polynomial operations.

    The keys are ``"Groebner"``, ``"PolyReduce"``, ``"IdealSolve"``.

    This is merged into ``build_cas_handler_table()`` in ``symbolic-vm``
    so that these operations are automatically available in every MACSYMA
    session.

    Returns
    -------
    Dict mapping head name → handler callable.
    """
    return {
        GROEBNER.name: groebner_handler,
        POLY_REDUCE.name: poly_reduce_handler,
        IDEAL_SOLVE.name: ideal_solve_handler,
    }
