"""Mixed partial-fraction integration — Phase 2f.

After Hermite reduction (Phase 2c) the log part is ``C(x)/E(x)`` with
``E`` squarefree. When Rothstein–Trager (Phase 2d) and the direct arctan
formula (Phase 2e) both return ``None``, this module handles the next
most common class: denominators of the form

    E(x)  =  L(x) · Q(x)

where

- ``L(x)`` is a product of distinct linear factors over Q
  (i.e. every rational root of ``E`` contributes one linear factor), and
- ``Q(x) = ax² + bx + c`` is a single irreducible quadratic over Q.

The algorithm uses the **Bézout identity** to split the numerator:

    u′ · L + v′ · Q = 1        (extended GCD, scaled to monic Bézout)

    C/(L·Q) = (C·v′ mod L)/L  +  (C·u′ mod Q)/Q
               ↑                  ↑
               Phase 2d (RT)      Phase 2e (arctan)

Both pieces are always proper fractions and their denominators are
coprime, so Phase 2d and Phase 2e are guaranteed to succeed on them.

See ``code/specs/mixed-integral.md`` for the full derivation.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    divmod_poly,
    extended_gcd,
    multiply,
    normalize,
    rational_roots,
)
from symbolic_ir import IRNode, IRSymbol

from symbolic_vm.arctan_integral import arctan_integral
from symbolic_vm.polynomial_bridge import rt_pairs_to_ir
from symbolic_vm.rothstein_trager import rothstein_trager


def mixed_integral(
    num: Polynomial,
    den: Polynomial,
    x_sym: IRSymbol,
) -> IRNode | None:
    """Return the IR for ``∫ num/den dx`` when ``den`` splits as L·Q.

    Pre-conditions (caller's responsibility, not re-checked here):
    - RT has already returned ``None`` on ``(num, den)``.
    - The arctan single-quadratic path has already returned ``None``.
    - ``den`` is squarefree with rational coefficients.
    - ``deg num < deg den``.

    Returns ``None`` when the denominator does not fit the L·Q shape
    (no rational roots, or the irreducible remainder has degree ≠ 2).
    Never returns ``None`` when the pre-conditions are satisfied and the
    denominator fits — if it did, that would be a bug (RT or arctan would
    have caught the degenerate cases first).
    """
    den_n = normalize(den)
    num_n = tuple(Fraction(c) for c in normalize(num))

    # Step 1: find all rational roots of den — these become the linear factors.
    roots = rational_roots(den_n)
    if not roots:
        return None  # No linear factors; purely irreducible — not our job.

    # Step 2: build L = ∏(x − rᵢ) with Fraction coefficients.
    L: Polynomial = (Fraction(1),)
    for r in roots:
        L = multiply(L, (-r, Fraction(1)))  # x − r

    # Step 3: Q = den / L  (exact division — no remainder).
    Q_quot, Q_rem = divmod_poly(
        tuple(Fraction(c) for c in den_n), L
    )
    Q = normalize(Q_quot)
    if normalize(Q_rem):
        return None  # Remainder non-zero — shouldn't happen on valid input.

    # Phase 2f handles only a *single* irreducible quadratic remainder.
    if len(Q) - 1 != 2:
        return None  # deg Q ≠ 2; could be deg-0 (handled by RT) or deg≥4.
    if rational_roots(Q):
        return None  # Q has rational roots — the caller should have caught this.

    # Step 4: Bézout split — (g, u, v) with u·L + v·Q = g.
    g_raw, u_raw, v_raw = extended_gcd(L, Q)
    g_n = normalize(g_raw)

    # g should be a non-zero constant since gcd(L, Q) = 1.
    if not g_n:
        return None
    g_const = Fraction(g_n[0])  # g = (constant,) — extract as Fraction

    # Scale Bézout coefficients so u′·L + v′·Q = 1.
    u_prime = tuple(Fraction(c) / g_const for c in normalize(u_raw))
    v_prime = tuple(Fraction(c) / g_const for c in normalize(v_raw))

    # Step 5: compute the numerator for each piece.
    # C_L = (num · v′) mod L   — goes over the linear denominator L
    # C_Q = (num · u′) mod Q   — goes over the quadratic denominator Q
    C_L_full = multiply(num_n, v_prime)
    _, C_L = divmod_poly(C_L_full, L)

    C_Q_full = multiply(num_n, u_prime)
    _, C_Q = divmod_poly(C_Q_full, Q)

    # Step 6: integrate the L-part via Rothstein–Trager.
    # L is a product of distinct linear factors over Q, so RT is guaranteed
    # to succeed (all log coefficients are rational residues).
    rt_pairs = rothstein_trager(C_L, L)
    if rt_pairs is None:
        return None  # Should not happen — signals a caller-invariant violation.

    # Step 7: integrate the Q-part via the arctan formula (Phase 2e).
    at_ir = arctan_integral(C_Q, Q, x_sym)

    # Step 8: assemble.
    from symbolic_ir import ADD, IRApply
    log_ir = rt_pairs_to_ir(rt_pairs, x_sym)
    return IRApply(ADD, (log_ir, at_ir))


__all__ = ["mixed_integral"]
