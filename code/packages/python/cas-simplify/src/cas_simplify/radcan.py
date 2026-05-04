"""Radical canonicalization — ``radcan(expr)``.

MACSYMA's ``radcan`` simplifies expressions involving square roots,
nth roots, and rational exponents into a canonical form where:

- All Sqrt products are merged into a single Sqrt of the product.
- Perfect-square factors are extracted from under radicals.
- Common rational exponents in a Mul are collected.
- Exp(Log(x)) and Log(Exp(x)) pairs are cancelled.

Algorithm
---------
A single bottom-up tree walk applies five rules at each ``IRApply`` node.
No fixpoint loop is needed because the rules are non-recursive: a node
produced by one rule never triggers another rule at the same level.

Rules
-----
1. **Sqrt merge** — ``√a · √b = √(ab)``

   Inside a ``Mul``, collect all ``Sqrt`` arguments into a single
   merged radicand and produce one ``Sqrt``.

2. **Perfect-square extraction** — ``√(x² · b) = x · √b``

   When a Sqrt radicand is a Mul containing ``Pow(x, 2)`` for a
   positive ``x`` (from context), pull ``x`` outside.
   For a positive integer radicand that is a perfect square, return
   the integer square root directly.

3. **Sqrt(x²) → x** — ``√(x²) = x`` when x > 0

   Handled as a special case of rule 2 with no remaining factor.

4. **Common rational exponent collection** — ``a^(p/q) · b^(p/q) = (ab)^(p/q)``

   In a Mul, group non-integer rational exponents by value; if two or
   more bases share the same exponent, merge them.
   (Does not apply to exponent 1/2 — that is handled by Sqrt merge.)

5. **Exp-Log cancellation** — ``Exp(Log(x)) = x``, ``Log(Exp(x)) = x``

   Cancels the pair when the heads are exact inverses.

These rules interact safely: Sqrt(a)*Sqrt(b) is handled by rule 1 before
rule 4 ever sees 1/2-exponents; Pow(x,2) inside Sqrt is handled by rule 2.

Example usage::

    from cas_simplify.radcan import radcan
    from cas_simplify.assumptions import AssumptionContext
    from symbolic_ir import *

    ctx = AssumptionContext()
    ctx.assume_relation(IRApply(GREATER, (IRSymbol("x"), IRInteger(0))))

    # √(x² · y) → x · √y
    expr = IRApply(SQRT, (IRApply(MUL, (
        IRApply(POW, (IRSymbol("x"), IRInteger(2))),
        IRSymbol("y"),
    )),))
    radcan(expr, ctx)
    # → Mul(x, Sqrt(y))
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    EXP,
    LOG,
    MUL,
    POW,
    SQRT,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_simplify.assumptions import AssumptionContext

# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def radcan(
    expr: IRNode,
    ctx: AssumptionContext | None = None,
) -> IRNode:
    """Radical canonicalization — simplify radical expressions.

    Applies the five rules described in the module docstring in a single
    bottom-up pass.  The optional ``ctx`` parameter supplies sign facts
    needed for rule 2 (``√(x²·b) = x·√b`` requires ``x > 0``).

    Parameters
    ----------
    expr:
        The IR expression to simplify.
    ctx:
        Optional :class:`~cas_simplify.assumptions.AssumptionContext`
        carrying per-symbol sign facts.

    Returns
    -------
    IRNode
        The simplified expression; equal to ``expr`` if no rule fired.
    """
    if not isinstance(expr, IRApply):
        return expr
    # Recurse into children first so rules apply bottom-up.
    new_args = tuple(radcan(a, ctx) for a in expr.args)
    node: IRApply = (
        IRApply(expr.head, new_args)
        if new_args != expr.args
        else expr
    )
    return _apply_rules(node, ctx)


# ---------------------------------------------------------------------------
# Rule dispatch
# ---------------------------------------------------------------------------


def _apply_rules(expr: IRApply, ctx: AssumptionContext | None) -> IRNode:
    """Apply radcan rules to a fully-recursed node."""
    head = expr.head
    if head == MUL:
        return _rule_mul(expr, ctx)
    if head == SQRT:
        return _rule_sqrt(expr, ctx)
    if head == POW:
        return _rule_pow(expr)
    if head == EXP:
        return _rule_exp(expr)
    if head == LOG:
        return _rule_log(expr)
    return expr


# ---------------------------------------------------------------------------
# Rule 1 + 4 — Mul: merge Sqrt products and collect rational exponents
# ---------------------------------------------------------------------------


def _rule_mul(expr: IRApply, ctx: AssumptionContext | None) -> IRNode:
    """Merge √a·√b → √(ab) and aᵖ/ᵍ·bᵖ/ᵍ → (ab)ᵖ/ᵍ in a product."""
    args = list(expr.args)

    # --- Phase A: collect and merge all Sqrt(x) factors ---
    sqrt_radicands: list[IRNode] = []
    non_sqrt: list[IRNode] = []

    for a in args:
        if isinstance(a, IRApply) and a.head == SQRT and len(a.args) == 1:
            sqrt_radicands.append(a.args[0])
        else:
            non_sqrt.append(a)

    if len(sqrt_radicands) >= 2:
        # Merge into a single Sqrt(product) and re-apply radcan.
        radicand = sqrt_radicands[0]
        for r in sqrt_radicands[1:]:
            radicand = IRApply(MUL, (radicand, r))
        merged_sqrt = radcan(IRApply(SQRT, (radicand,)), ctx)
        args = non_sqrt + [merged_sqrt]

    # --- Phase B: collect identical non-half rational exponents ---
    # Group: exponent (as Fraction) → list[base]
    rational_groups: dict[Fraction, list[IRNode]] = {}
    remaining: list[IRNode] = []

    for a in args:
        exp_frac = _rational_exponent(a)
        # Only collect proper fractions (not 1/2, not integers, not 1).
        if (
            exp_frac is not None
            and exp_frac.denominator > 1
            and exp_frac != Fraction(1, 2)
        ):
            base = _base_of(a)
            if base is not None:
                rational_groups.setdefault(exp_frac, []).append(base)
                continue
        remaining.append(a)

    merged: list[IRNode] = []
    for exp_frac, bases in rational_groups.items():
        if len(bases) == 1:
            # Single base — push back as-is.
            remaining.append(IRApply(POW, (bases[0], _frac_to_ir(exp_frac))))
        else:
            # Merge bases into (b0*b1*...*bn)^(p/q).
            product = bases[0]
            for b in bases[1:]:
                product = IRApply(MUL, (product, b))
            merged.append(IRApply(POW, (product, _frac_to_ir(exp_frac))))

    all_args = remaining + merged

    if not all_args:
        return IRInteger(1)
    if len(all_args) == 1:
        return all_args[0]
    return IRApply(MUL, tuple(all_args))


# ---------------------------------------------------------------------------
# Rules 2 + 3 — Sqrt: extract perfect-square factors
# ---------------------------------------------------------------------------


def _rule_sqrt(expr: IRApply, ctx: AssumptionContext | None) -> IRNode:
    """Extract perfect-square factors from under a Sqrt."""
    if len(expr.args) != 1:
        return expr

    arg = expr.args[0]

    # Sqrt(integer perfect square) → integer square root.
    if isinstance(arg, IRInteger) and arg.value >= 0:
        root = _integer_sqrt(arg.value)
        if root is not None:
            return IRInteger(root)

    # Sqrt(x²) → |x|, or → x when x > 0.
    if _is_square_power(arg):
        base = _base_of(arg)
        if base is not None:
            return _abs_or_pos(base, ctx)

    # Sqrt(Mul(...)) — extract Pow(x,2) factors when x > 0.
    if isinstance(arg, IRApply) and arg.head == MUL:
        outer: list[IRNode] = []
        inner: list[IRNode] = []
        for factor in arg.args:
            result = _try_extract_from_sqrt(factor, ctx)
            if result is not None:
                outer.append(result)
            else:
                inner.append(factor)

        if outer:
            outer_prod = _mul_or_one(outer)
            inner_prod = _mul_or_one(inner)
            if inner_prod == IRInteger(1):
                return outer_prod
            inner_sqrt = IRApply(SQRT, (inner_prod,))
            return IRApply(MUL, (outer_prod, inner_sqrt))

    return expr


def _try_extract_from_sqrt(
    factor: IRNode, ctx: AssumptionContext | None
) -> IRNode | None:
    """Return what to pull outside Sqrt(factor * ...), or None if not extractable.

    A factor is extractable when:
    - It is ``Pow(x, 2)`` and either x is a positive integer literal or
      x is a symbol known positive from ctx.
    - It is a positive integer that is a perfect square.
    """
    # Pow(symbol, 2) with x > 0 from ctx.
    if _is_square_power(factor):
        base = _base_of(factor)
        if base is None:
            return None
        if isinstance(base, IRInteger) and base.value > 0:
            return base
        if (
            isinstance(base, IRSymbol)
            and ctx is not None
            and ctx.is_positive(base.name) is True
        ):
            return base
        return None

    # Positive integer perfect square (e.g. 4 → 2).
    if isinstance(factor, IRInteger) and factor.value > 0:
        root = _integer_sqrt(factor.value)
        if root is not None:
            return IRInteger(root)

    return None


def _is_square_power(node: IRNode) -> bool:
    """True if node is Pow(something, 2)."""
    return (
        isinstance(node, IRApply)
        and node.head == POW
        and len(node.args) == 2
        and node.args[1] == IRInteger(2)
    )


def _abs_or_pos(base: IRNode, ctx: AssumptionContext | None) -> IRNode:
    """Return base when known positive, or Sqrt(base²) otherwise (leave unevaluated).

    Since we don't have an Abs head, we just return base when positivity is
    confirmed and leave the original Sqrt intact otherwise — the function's
    caller handles that case.
    """
    if isinstance(base, IRInteger) and base.value > 0:
        return base
    if (
        isinstance(base, IRSymbol)
        and ctx is not None
        and ctx.is_positive(base.name) is True
    ):
        return base
    # Unknown sign — return the Sqrt unevaluated (caller returns expr as-is).
    return IRApply(SQRT, (IRApply(POW, (base, IRInteger(2))),))


# ---------------------------------------------------------------------------
# Rule 4 helper — Pow(Sqrt(x), 2) → x
# ---------------------------------------------------------------------------


def _rule_pow(expr: IRApply) -> IRNode:
    """Pow(Sqrt(x), 2) → x."""
    if len(expr.args) != 2:
        return expr
    base, exp_node = expr.args
    if (
        isinstance(base, IRApply)
        and base.head == SQRT
        and len(base.args) == 1
        and exp_node == IRInteger(2)
    ):
        return base.args[0]
    return expr


# ---------------------------------------------------------------------------
# Rule 5 — Exp-Log cancellation
# ---------------------------------------------------------------------------


def _rule_exp(expr: IRApply) -> IRNode:
    """Exp(Log(x)) → x."""
    if len(expr.args) == 1:
        arg = expr.args[0]
        if isinstance(arg, IRApply) and arg.head == LOG and len(arg.args) == 1:
            return arg.args[0]
    return expr


def _rule_log(expr: IRApply) -> IRNode:
    """Log(Exp(x)) → x."""
    if len(expr.args) == 1:
        arg = expr.args[0]
        if isinstance(arg, IRApply) and arg.head == EXP and len(arg.args) == 1:
            return arg.args[0]
    return expr


# ---------------------------------------------------------------------------
# Utility helpers
# ---------------------------------------------------------------------------


def _rational_exponent(node: IRNode) -> Fraction | None:
    """Return the exponent of Pow(base, exp) as Fraction, or None."""
    if not isinstance(node, IRApply):
        return None
    if node.head != POW or len(node.args) != 2:
        return None
    exp = node.args[1]
    if isinstance(exp, IRInteger):
        return Fraction(exp.value)
    if isinstance(exp, IRRational):
        return Fraction(exp.numer, exp.denom)
    return None


def _base_of(node: IRNode) -> IRNode | None:
    """Return the base of Pow(base, exp), or None."""
    if isinstance(node, IRApply) and node.head == POW and len(node.args) == 2:
        return node.args[0]
    return None


def _integer_sqrt(n: int) -> int | None:
    """Return the integer square root of n if n is a perfect square, else None."""
    if n < 0:
        return None
    if n == 0:
        return 0
    root = int(n**0.5)
    # Correct for floating-point rounding around perfect squares.
    while root * root > n:
        root -= 1
    while (root + 1) * (root + 1) <= n:
        root += 1
    return root if root * root == n else None


def _frac_to_ir(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _mul_or_one(nodes: list[IRNode]) -> IRNode:
    """Build a Mul from a list, or return IRInteger(1) for an empty list."""
    if not nodes:
        return IRInteger(1)
    if len(nodes) == 1:
        return nodes[0]
    return IRApply(MUL, tuple(nodes))
