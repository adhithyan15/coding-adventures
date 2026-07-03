"""Generic tabular integration-by-parts fallback — Track E1.

When the pipeline's specific shape-aware handlers (Phase 1, Phase 3–28,
Phase 33–38, Phase 23 special-function fallbacks, Elliptic) have all
returned ``None`` for a ``Mul``-shaped integrand, this module makes a
last-ditch attempt to close the integral by **generic tabular IBP**.

The algorithm
=============

For an integrand of the form ``f = u(x) · w(x)``:

1. Identify a factorisation where ``u`` is **polynomial** in *x* (so its
   derivative chain ``u, u', u'', …`` terminates at zero in a finite
   number of steps ``N = deg(u) + 1``) and ``w`` is **integrable** (its
   antiderivative ``∫w dx`` has a closed form via the existing
   ``_integrate`` pipeline).
2. Build the two columns:

   ===========  ===============================
   k            D-column (differentiate)   I-column (integrate)
   ===========  ===============================
   0            ``u``                      ``w``
   1            ``u'``                     ``∫w dx``
   2            ``u''``                    ``∫∫w dx``
   …            …                          …
   N            ``0``  (terminates)        ``I^N(w)``
   ===========  ===============================

3. The result is the alternating sum

   ``∫ u·w dx  =  Σ_{k=0}^{N-1}  (-1)^k · u^(k)(x) · I^(k+1)(w)``

   The trailing remainder ``(-1)^N · ∫ u^(N) · I^N(w) dx`` vanishes
   because ``u^(N) = 0``.

Why "tabular"?
==============

This is the textbook table that lecturers draw for ``∫ x³ cos x dx``:

::

    D     |    I
    ------+-----------
    x³    |    cos x
    3x²   |    sin x
    6x    |   −cos x
    6     |   −sin x
    0     |    cos x

Multiplying down diagonals with alternating signs gives
``x³ sin x + 3x² cos x − 6x sin x − 6 cos x``.

When this fires (and when it doesn't)
=====================================

This is a **fallback** — it runs only after every other handler has
returned ``None``.  The point is to close integrals that the per-shape
helpers don't cover, *not* to replace them.  Common patterns it handles
that fall through earlier:

- Nested ``Mul`` trees where ``a · b · c`` is parsed as
  ``Mul(a, Mul(b, c))`` and the per-shape helpers only look at the
  outer two arguments.
- Three-or-more-factor products where two factors group into a
  polynomial and the rest forms an integrable kernel.

It deliberately does **not**:

- Recurse into itself.  Tabular has a natural termination on the D side
  (``u^(N) = 0``), but if the I-column hits an integrand whose
  antiderivative also needs tabular IBP we'd risk infinite descent.
  Bounded one-level recursion via the standard ``_integrate`` pipeline
  is enough for the realistic gap cases; we stop there.
- Split into more than two pieces.  The split is always
  ``factors  →  (polynomial-part, integrable-part)`` with a single
  ``w``.  Multi-way decompositions can be expressed as repeated
  applications of the same rule.

Termination
===========

The D-column terminates at the first ``0`` derivative — guaranteed for a
true polynomial.  We bound the loop at ``MAX_DEGREE + 2`` as a defensive
sanity check; any polynomial with degree above that is unlikely to
benefit from tabular IBP (the result blows up combinatorially).
"""

from __future__ import annotations

from collections.abc import Callable
from itertools import combinations

from polynomial import normalize
from symbolic_ir import (
    ADD,
    INTEGRATE,
    MUL,
    NEG,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import to_rational

# ---------------------------------------------------------------------------
# Tunables — keep tabular bounded so we never explore a combinatorial blow-up.
# ---------------------------------------------------------------------------

#: Maximum polynomial degree we'll differentiate.  Pure polynomials with
#: very high degree produce huge tabular outputs (each level grows the
#: assembled IR by O(deg)); the elementary-functions chain isn't the
#: right tool for those.  Eight is a comfortable upper bound — covers
#: every textbook IBP example without exploring quadratic-in-deg expansion.
_MAX_POLY_DEGREE = 8

#: Maximum number of ``Mul`` factors we consider after flattening.  With n
#: factors we try ``2^n − 2`` non-trivial subset splits — five factors is
#: 30 splits, plenty for realistic engineering integrands.
_MAX_FACTORS = 5

ZERO = IRInteger(0)
ONE = IRInteger(1)
NEG_ONE = IRInteger(-1)


# ---------------------------------------------------------------------------
# Mul flattening — keep the search across factor-orderings explicit
# ---------------------------------------------------------------------------


def _flatten_mul(node: IRNode) -> list[IRNode]:
    """Return the leaves of a (possibly nested-binary) ``Mul`` tree.

    ``Mul(a, Mul(b, Mul(c, d)))`` flattens to ``[a, b, c, d]``.  The VM
    emits binary ``Mul`` nodes throughout but users wrote (and expect) a
    flat associative product — without flattening, the IBP search would
    miss splits like ``u = a·c, w = b·d`` purely because the parse tree
    happened to group differently.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return [node]
    result: list[IRNode] = []
    for arg in node.args:
        result.extend(_flatten_mul(arg))
    return result


def _multiply_ir(factors: list[IRNode]) -> IRNode:
    """Rebuild a left-associative ``Mul`` chain from a list of factors.

    Empty list → ``1`` (the empty product).  Single factor returns
    itself so downstream rewriters see the simplest shape.
    """
    if not factors:
        return ONE
    if len(factors) == 1:
        return factors[0]
    acc = factors[0]
    for f in factors[1:]:
        acc = IRApply(MUL, (acc, f))
    return acc


# ---------------------------------------------------------------------------
# Polynomial-degree helpers — used to bound the differentiation chain
# ---------------------------------------------------------------------------


def _polynomial_degree(node: IRNode, x: IRSymbol) -> int | None:
    """Return the degree of *node* as a polynomial in *x*, or ``None``.

    Anything outside ``Q[x]`` (rationals, transcendentals, free symbols
    other than *x*) returns ``None``.  A non-zero constant returns ``0``;
    the zero polynomial returns ``-1`` so the caller can treat the
    "drop-the-factor" partition consistently.
    """
    r = to_rational(node, x)
    if r is None:
        return None
    num, den = r
    if len(normalize(den)) > 1:
        return None  # rational, not polynomial in x
    num_n = normalize(num)
    if not num_n:
        return -1  # zero
    # Strip trailing zeros — polynomial normalize already does this but
    # we double-check so the public contract (length-1 ⇒ constant) holds
    # for callers that mix-and-match coefficient providers.
    return len(num_n) - 1


# ---------------------------------------------------------------------------
# "Did the recursive integrate close?" — single source of truth
# ---------------------------------------------------------------------------


def _contains_integrate(node: IRNode) -> bool:
    """True if *node* contains any unevaluated ``Integrate(...)`` sub-tree.

    Used to reject I-column entries that the ``_integrate`` callback
    couldn't close to a true antiderivative.  Tabular IBP only emits a
    result when **every** ``∫w, ∫∫w, …, ∫^N w`` is fully closed —
    otherwise the assembled sum would still contain an unevaluated
    integral and the caller's "did we make progress?" check would fail.
    """
    if isinstance(node, IRApply):
        if node.head == INTEGRATE:
            return True
        return any(_contains_integrate(a) for a in node.args)
    return False


# ---------------------------------------------------------------------------
# The fallback itself
# ---------------------------------------------------------------------------


def try_ibp_tabular(
    f: IRNode,
    x: IRSymbol,
    integrate_fn: Callable[[IRNode], IRNode | None],
    diff_fn: Callable[[IRNode], IRNode],
    simplify_fn: Callable[[IRNode], IRNode] | None = None,
) -> IRNode | None:
    """Attempt generic tabular IBP on a ``Mul``-shaped integrand.

    Parameters
    ----------
    f
        The integrand.  Must be ``IRApply`` headed by ``MUL`` for IBP to
        fire; anything else short-circuits to ``None``.
    x
        Integration variable.
    integrate_fn
        Recursive integrator for the I-column.  Conventionally the
        caller passes the same ``_integrate`` Phase-1 pattern table that
        the outer handler runs first — this keeps the recursion bounded
        (no re-entry into tabular IBP) and ensures the I-column entries
        use the existing closed-form library.
    diff_fn
        Differentiator for the D-column.  Conventionally ``_diff``.
    simplify_fn
        Optional callable applied to each D/I column entry after raw
        differentiation / integration — usually ``vm.eval``.  The
        polynomial-degree-bound check and zero-detection both depend on
        the entries being in canonical form; without simplification a
        ``D[k]`` of ``0`` might show up as ``Add(-2, 2)`` and the loop
        wouldn't terminate.  If ``None``, raw outputs are used (the
        algorithm still works, just less aggressively).

    Returns
    -------
    IRNode | None
        Closed-form antiderivative as IR, or ``None`` when no viable
        ``(u, w)`` split was found.  ``None`` is the signal to the
        caller "this fallback can't help; leave the integral
        unevaluated."
    """
    # Only fires on Mul — everything else has dedicated handlers.
    if not isinstance(f, IRApply) or f.head != MUL:
        return None

    factors = _flatten_mul(f)
    if len(factors) < 2 or len(factors) > _MAX_FACTORS:
        return None

    # Enumerate non-trivial subset partitions: U has at least one factor
    # and at most n-1 factors (leaving w non-empty).  Prefer smaller U
    # first — tabular IBP is most efficient when ``u`` is low-degree.
    n = len(factors)
    indices = list(range(n))
    for u_size in range(1, n):
        for u_idx in combinations(indices, u_size):
            u_set = set(u_idx)
            u_factors = [factors[i] for i in indices if i in u_set]
            w_factors = [factors[i] for i in indices if i not in u_set]
            result = _try_split(
                u_factors,
                w_factors,
                x,
                integrate_fn,
                diff_fn,
                simplify_fn,
            )
            if result is not None:
                return result
    return None


def _try_split(
    u_factors: list[IRNode],
    w_factors: list[IRNode],
    x: IRSymbol,
    integrate_fn: Callable[[IRNode], IRNode | None],
    diff_fn: Callable[[IRNode], IRNode],
    simplify_fn: Callable[[IRNode], IRNode] | None,
) -> IRNode | None:
    """Try ``u = ∏ u_factors``, ``w = ∏ w_factors`` as the tabular split.

    Returns the assembled antiderivative, or ``None`` if any of the
    well-defined preconditions fail:

    1. ``u`` is a polynomial in *x* of bounded degree.
    2. Each iterated integral ``∫^k w dx`` for k = 1..N closes via
       ``integrate_fn`` (no residual unevaluated ``Integrate``).

    The ``u`` polynomial-check rejects splits where any ``u`` factor
    isn't polynomial — e.g. ``u_factors = [sin(x)]`` immediately fails
    because ``sin(x)`` isn't in Q[x].
    """
    u_ir = _multiply_ir(u_factors)
    if simplify_fn is not None:
        u_ir = simplify_fn(u_ir)

    deg = _polynomial_degree(u_ir, x)
    if deg is None:
        return None
    if deg < 0:
        # u is the zero polynomial — ∫ 0·w dx = 0.  Edge case, but
        # consistent with the algorithm.
        return ZERO
    if deg > _MAX_POLY_DEGREE:
        return None

    # The D-column: u, u', u'', ..., 0 (the +1 buys us the terminating
    # row at index = deg+1; in practice the loop exits earlier when the
    # derivative simplifies to literal zero).
    d_col: list[IRNode] = [u_ir]
    cur: IRNode = u_ir
    for _ in range(deg + 1):
        cur = diff_fn(cur)
        if simplify_fn is not None:
            cur = simplify_fn(cur)
        d_col.append(cur)
        if _is_zero(cur):
            break
    # After this loop the last entry is either literal zero or "we
    # exceeded deg+1 differentiations, polynomial assumption is wrong".
    if not _is_zero(d_col[-1]):
        return None
    N = len(d_col) - 1  # u^(N) = 0; the trailing sum vanishes

    # The I-column: w, ∫w, ∫∫w, ..., ∫^N w.  Each step uses the recursive
    # integrate_fn; if any step fails to close, abort the partition.
    w_ir = _multiply_ir(w_factors)
    if simplify_fn is not None:
        w_ir = simplify_fn(w_ir)
    # An integrable ``w`` that *happens* to itself simplify to a
    # polynomial in x would also be handled here, but the polynomial
    # path through the outer handler is faster.  We don't bail on it
    # explicitly — the algorithm still produces a correct result.
    i_col: list[IRNode] = [w_ir]
    cur = w_ir
    for _ in range(N):
        integrated = integrate_fn(cur)
        if integrated is None:
            return None
        if simplify_fn is not None:
            integrated = simplify_fn(integrated)
        if _contains_integrate(integrated):
            return None  # didn't close — abandon this split
        i_col.append(integrated)
        cur = integrated

    # Assemble: Σ_{k=0}^{N-1} (-1)^k · D[k] · I[k+1].
    pieces: list[IRNode] = []
    for k in range(N):
        term = IRApply(MUL, (d_col[k], i_col[k + 1]))
        if k % 2 == 1:
            term = IRApply(NEG, (term,))
        pieces.append(term)

    if not pieces:
        return ZERO
    result: IRNode = pieces[0]
    for term in pieces[1:]:
        result = IRApply(ADD, (result, term))
    return result


def _is_zero(node: IRNode) -> bool:
    """True iff *node* canonicalises to the integer literal ``0``.

    We only check the post-simplification form because the loop calls
    ``simplify_fn`` before this predicate.  A non-canonical zero like
    ``Sub(2, 2)`` would slip through, which is why ``simplify_fn`` is
    strongly recommended at the call site.
    """
    if isinstance(node, IRInteger) and node.value == 0:
        return True
    if (
        isinstance(node, IRApply)
        and node.head == NEG
        and len(node.args) == 1
    ):
        return _is_zero(node.args[0])
    return False


__all__ = ["try_ibp_tabular"]
# Re-export the small helpers for unit-testing visibility.  External
# callers should only use ``try_ibp_tabular`` — the rest are
# implementation details we expose for white-box tests.
__all__.extend(
    [
        "_flatten_mul",
        "_polynomial_degree",
        "_contains_integrate",
    ]
)
