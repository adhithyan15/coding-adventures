"""Newton's method numeric root finder — the core algorithm.

Newton's method (also called the Newton-Raphson method) is one of the
oldest and most effective algorithms for finding the root of a smooth
function f near an initial guess x0. The iteration is:

    x_{n+1} = x_n - f(x_n) / f'(x_n)

Geometrically, this draws the tangent line to f at x_n and uses the
zero of that tangent line as the next guess. When f is smooth and the
initial guess is close to a root, the method converges *quadratically*
— meaning the number of correct digits roughly doubles with each step.

This module keeps the algorithm pure: it operates on symbolic IR trees
but receives ``eval_fn`` and ``diff_fn`` as parameters, avoiding a hard
import cycle with ``symbolic_vm``. The handler in ``handlers.py`` wires
those in from the live VM.

How substitution works
----------------------
Given symbolic f(x) = x^2 - 2, to evaluate at x_n = 1.5 we:

1. Use ``cas_substitution.subst(IRFloat(1.5), x_sym, f_ir)`` to get
   the substituted tree ``(1.5)^2 - 2`` (still unevaluated IR).
2. Pass the result through ``eval_fn`` which collapses the arithmetic:
   ``(1.5)^2 - 2`` → ``0.25``.

This approach means we don't need a separate numerical evaluator; the
symbolic VM's own arithmetic pipeline handles it.
"""

from __future__ import annotations

from collections.abc import Callable

from cas_substitution import subst
from symbolic_ir import IRFloat, IRInteger, IRNode, IRRational, IRSymbol


class MNewtonError(Exception):
    """Raised when Newton's method cannot proceed.

    The most common cause is a zero derivative at the current iterate.
    Division by zero in the Newton step would produce an undefined next
    guess — we raise here rather than silently returning NaN or inf.

    The handler in ``handlers.py`` catches this and returns the original
    unevaluated expression, which is the MACSYMA convention for failed
    numeric evaluation.
    """


def _ir_to_float(node: IRNode) -> float | None:
    """Extract a Python float from a numeric IR literal.

    Returns ``None`` for anything that is not a numeric literal so the
    caller can fall back to returning the unevaluated expression rather
    than crashing.

    The three numeric IR types are:
    - ``IRInteger(n)``        — exact integer; convert via ``float(n)``.
    - ``IRFloat(v)``          — already a float; use directly.
    - ``IRRational(p, q)``    — exact fraction; divide numerator by denom.

    Symbols, applies, and strings return ``None``.
    """
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        # IRRational stores numer and denom as ints, both after gcd
        # reduction and sign normalization.
        return node.numer / node.denom
    return None


def mnewton_solve(
    f_ir: IRNode,
    x_sym: IRSymbol,
    x0_ir: IRNode,
    eval_fn: Callable[[IRNode], IRNode],  # vm.eval
    diff_fn: Callable[[IRNode, IRSymbol], IRNode],  # derivative._diff
    tol: float = 1e-10,
    max_iter: int = 50,
) -> IRNode:
    """Find a root of f near x0 using Newton's method.

    Algorithm overview
    ------------------
    1. Differentiate f symbolically *once* before the loop. This is
       the key efficiency: f' is computed once as a symbolic tree,
       then numerically evaluated on each iteration by substituting
       the current x_n. Avoids redundant symbolic work inside the loop.

    2. Convert x0 to a Python float. If x0 is not a numeric literal
       (e.g. it is a symbol like ``a``), return the original expression
       unevaluated — there is no sensible starting point.

    3. Iterate Newton's formula up to ``max_iter`` times:

       a. Substitute the current float x_n into f and f' as ``IRFloat``.
       b. Evaluate the substituted tree through the VM so arithmetic
          collapses to a single numeric literal.
       c. If f(x_n) is already within ``tol`` of zero, convergence!
          Return ``IRFloat(x_n)``.
       d. If f'(x_n) is smaller than 1e-300 in absolute value, the
          tangent is flat — raise ``MNewtonError`` so the handler can
          return unevaluated.
       e. Compute x_{n+1} = x_n - f(x_n) / f'(x_n) and continue.

    4. If we exhaust ``max_iter`` iterations without convergence, return
       the best approximation found: ``IRFloat(x_n)``. Newton's method
       with a reasonable x0 typically needs fewer than 10 iterations to
       reach double-precision accuracy; hitting max_iter indicates either
       a bad starting guess or a pathological function.

    Parameters
    ----------
    f_ir:
        The function expression in ``x_sym``.
    x_sym:
        The independent variable symbol.
    x0_ir:
        The initial guess as an IR numeric node.
    eval_fn:
        ``vm.eval`` — collapses arithmetic in a substituted tree.
    diff_fn:
        ``derivative._diff`` — computes df/dx symbolically.
    tol:
        Convergence tolerance on |f(x_n)|. Default 1e-10.
    max_iter:
        Maximum number of Newton steps before returning best guess.

    Returns
    -------
    IRNode
        ``IRFloat(root)`` on convergence or after max_iter steps.
        The original expression ``f_ir`` if x0 is not numeric.
    """
    # ---- Step 1: differentiate f symbolically, once -----------------------
    # This produces an IR tree representing f'(x) that we will evaluate
    # numerically on every iteration. The symbolic derivative is computed
    # by the same machinery that handles D(f, x) in the VM, but called
    # directly here to avoid the overhead of wrapping in an IRApply and
    # dispatching back through the VM.
    f_prime_ir = eval_fn(diff_fn(f_ir, x_sym))

    # ---- Step 2: convert x0 to float --------------------------------------
    # We need a numerical starting point. If the caller passes a symbol
    # or an unevaluated expression, we cannot iterate — return as-is.
    x_n = _ir_to_float(x0_ir)
    if x_n is None:
        # x0 is symbolic — return f_ir unevaluated. This matches what
        # every CAS does when given a non-numeric initial guess.
        return f_ir

    # ---- Step 3: iterate Newton's method ----------------------------------
    for _iteration in range(max_iter):
        # Substitute the current float x_n into both f and f'.
        # subst(value, var, expr) replaces every occurrence of var with value.
        x_n_ir = IRFloat(x_n)

        # Evaluate f(x_n): substitute then collapse through the VM.
        f_xn_ir = eval_fn(subst(x_n_ir, x_sym, f_ir))
        f_xn = _ir_to_float(f_xn_ir)

        # If eval_fn cannot reduce the substituted expression to a
        # numeric literal (e.g. f contains a symbol other than x_sym),
        # return the original expression unevaluated.
        if f_xn is None:
            return f_ir

        # Convergence check: |f(x_n)| < tol means we found a root.
        if abs(f_xn) < tol:
            return IRFloat(x_n)

        # Evaluate f'(x_n): same pattern — substitute then collapse.
        f_prime_xn_ir = eval_fn(subst(x_n_ir, x_sym, f_prime_ir))
        f_prime_xn = _ir_to_float(f_prime_xn_ir)

        if f_prime_xn is None:
            # Derivative did not reduce to a number — leave unevaluated.
            return f_ir

        # Guard against near-zero derivative (flat tangent).
        # A derivative smaller than 1e-300 in absolute value would cause
        # catastrophic cancellation or overflow in the Newton step.
        if abs(f_prime_xn) < 1e-300:
            raise MNewtonError(
                f"Newton's method: derivative is zero at x = {x_n!r}. "
                "Cannot continue iteration."
            )

        # Newton step: move to the x-intercept of the tangent at x_n.
        x_n = x_n - f_xn / f_prime_xn

    # ---- Step 4: return best approximation after max_iter -----------------
    # We exhausted the iteration budget. Return what we have — this is
    # the standard behaviour in numeric CAS (Maxima, Maple, Mathematica
    # all return the last iterate rather than raising an error).
    return IRFloat(x_n)
