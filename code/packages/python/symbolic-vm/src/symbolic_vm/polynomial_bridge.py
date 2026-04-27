"""IR ↔ Polynomial bridge — Phase 2b of the symbolic integrator.

Translates between :mod:`symbolic_ir` trees and :mod:`polynomial`
tuples. This is the one-way coupling point between the two packages —
neither ``symbolic-ir`` nor ``polynomial`` depends on the other; the
bridge lives here in ``symbolic-vm`` because the VM is the piece that
already imports both.

Two functions:

- :func:`to_rational` recognises rational functions of a named variable
  and returns the ``(numerator, denominator)`` pair as ``Polynomial``
  tuples with ``Fraction`` coefficients. Returns ``None`` otherwise —
  giving the caller a clean rational-or-not gate.
- :func:`from_polynomial` emits the canonical IR tree for a polynomial
  at the named variable.

See ``code/specs/polynomial-bridge.md`` for the full scope and the
decision table.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import Polynomial, add, multiply, normalize, one, subtract
from symbolic_ir import (
    ADD,
    DIV,
    LOG,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# A rational function decomposition: (numerator, denominator). Both are
# non-zero; denominator is normalised but not cleared of common factors.
Rational = tuple[Polynomial, Polynomial]


# ---------------------------------------------------------------------------
# to_rational
# ---------------------------------------------------------------------------


def to_rational(f: IRNode, x: IRSymbol) -> Rational | None:
    """Try to represent ``f`` as a rational function of ``x``.

    Returns ``(numerator, denominator)`` with ``Fraction`` coefficients
    if successful, or ``None`` if ``f`` contains anything outside
    Q(x) — transcendental functions, symbolic exponents, floats, or
    free symbols.

    No common-factor cancellation. ``(x² − 1) / (x − 1)`` comes back
    verbatim; the caller runs ``polynomial.gcd`` if they want
    reduction. Hermite reduction does exactly that as its first step,
    so the bridge deliberately stays structural.
    """
    return _walk(f, x)


def _walk(node: IRNode, x: IRSymbol) -> Rational | None:
    # Literal integers and rationals → constant numerator, 1 denom.
    if isinstance(node, IRInteger):
        return ((Fraction(node.value),), _ONE_POLY)
    if isinstance(node, IRRational):
        return ((Fraction(node.numer, node.denom),), _ONE_POLY)

    # Floats are out — they destroy exact arithmetic. The CAS at large
    # is Q-only; if we ever want to admit R[x] it gets a separate code
    # path.
    if isinstance(node, IRFloat):
        return None

    # The named variable.
    if isinstance(node, IRSymbol):
        if node == x:
            return ((Fraction(0), Fraction(1)), _ONE_POLY)
        # Every other bare symbol is a free variable. We could treat it
        # as a formal constant, but that would silently admit integrands
        # the integrator won't actually integrate over Q. Be strict —
        # the caller can preprocess if they want to handle free symbols.
        return None

    # Everything structured flows through here.
    if not isinstance(node, IRApply):
        return None

    head = node.head

    if head == ADD:
        return _reduce_binop(node.args, x, _add_rational)
    if head == SUB:
        # Sub is binary in the IR; we don't bother with n-ary fallback.
        if len(node.args) != 2:
            return None
        a = _walk(node.args[0], x)
        b = _walk(node.args[1], x)
        if a is None or b is None:
            return None
        return _sub_rational(a, b)
    if head == NEG:
        if len(node.args) != 1:
            return None
        a = _walk(node.args[0], x)
        if a is None:
            return None
        num, den = a
        return (_neg_poly(num), den)
    if head == MUL:
        return _reduce_binop(node.args, x, _mul_rational)
    if head == DIV:
        if len(node.args) != 2:
            return None
        a = _walk(node.args[0], x)
        b = _walk(node.args[1], x)
        if a is None or b is None:
            return None
        return _div_rational(a, b)
    if head == POW:
        if len(node.args) != 2:
            return None
        return _pow_rational(node.args[0], node.args[1], x)

    # Any other head — Sin, Log, Exp, Sqrt, user-defined symbols — is
    # outside Q(x). The integrator's Phase 3 will handle transcendental
    # towers; the rational-function pipeline refuses them here.
    return None


# ---------------------------------------------------------------------------
# Rational arithmetic on (num, den) pairs
# ---------------------------------------------------------------------------
#
# These are exactly the field-of-fractions operations — the same moves
# we'd make with integer fractions, lifted to Q[x]. No cancellation;
# each operation picks the obvious common denominator.


def _add_rational(a: Rational, b: Rational) -> Rational:
    num = add(multiply(a[0], b[1]), multiply(b[0], a[1]))
    den = multiply(a[1], b[1])
    return (num, den)


def _sub_rational(a: Rational, b: Rational) -> Rational:
    num = subtract(multiply(a[0], b[1]), multiply(b[0], a[1]))
    den = multiply(a[1], b[1])
    return (num, den)


def _mul_rational(a: Rational, b: Rational) -> Rational:
    return (multiply(a[0], b[0]), multiply(a[1], b[1]))


def _div_rational(a: Rational, b: Rational) -> Rational | None:
    # b = b_num / b_den.  a / b = (a_num · b_den) / (a_den · b_num).
    # If b_num is identically zero, the whole expression is undefined.
    # ``multiply`` already normalises, so ``new_den == ()`` iff the
    # product is the zero polynomial.
    new_den = multiply(a[1], b[0])
    if not new_den:
        return None
    return (multiply(a[0], b[1]), new_den)


def _pow_rational(base: IRNode, exponent: IRNode, x: IRSymbol) -> Rational | None:
    if not isinstance(exponent, IRInteger):
        return None
    base_r = _walk(base, x)
    if base_r is None:
        return None
    n = exponent.value
    num, den = base_r
    if n == 0:
        # Anything^0 = 1, even if "anything" depends on x. We return 1/1
        # rather than the clever 0^0 distinction — if base simplifies
        # to 0 later, the integrator will notice.
        return (_ONE_POLY, _ONE_POLY)
    if n < 0:
        if not normalize(num):
            # 0^(negative) is undefined. ``num`` may still carry
            # explicit zero coefficients (e.g. ``(Fraction(0),)``) that
            # haven't been stripped yet, so normalise before checking.
            return None
        num, den = den, num
        n = -n
    return (_pow_poly(num, n), _pow_poly(den, n))


def _pow_poly(p: Polynomial, n: int) -> Polynomial:
    # Repeated multiply. ``n`` is small in practice (human integrands),
    # so fast-exponentiation isn't worth the complexity.
    result = one()
    for _ in range(n):
        result = multiply(result, p)
    return result


def _neg_poly(p: Polynomial) -> Polynomial:
    return tuple(-c for c in p)


def _reduce_binop(
    args: tuple[IRNode, ...],
    x: IRSymbol,
    op,
) -> Rational | None:
    """Fold ``op`` across ``args``. Handles the n-ary Add / Mul shapes
    the compiler is free to emit. ``op`` is either :func:`_add_rational`
    or :func:`_mul_rational`, neither of which can fail on non-None
    inputs — so we only check for None from the sub-walks.
    """
    if not args:
        return None
    acc = _walk(args[0], x)
    if acc is None:
        return None
    for arg in args[1:]:
        other = _walk(arg, x)
        if other is None:
            return None
        acc = op(acc, other)
    return acc


_ONE_POLY: Polynomial = (Fraction(1),)


# ---------------------------------------------------------------------------
# from_polynomial
# ---------------------------------------------------------------------------


def from_polynomial(p: Polynomial, x: IRSymbol) -> IRNode:
    """Build the canonical IR tree for ``p(x)``.

    Output shape matches what the existing differentiator and Phase 1
    integrator emit, so no post-processing is needed:

    - zero polynomial          → ``IRInteger(0)``
    - constant ``(c,)``        → ``_coef(c)``
    - non-constant polynomial  → ``Add(term_0, term_1, …)`` where each
      ``term_i`` is either a coefficient, a bare ``x``, or
      ``Mul(coef, Pow(x, IRInteger(i)))`` as appropriate.
    """
    if not p:
        return IRInteger(0)
    if len(p) == 1:
        return _coef(p[0])

    terms: list[IRNode] = []
    for i, c in enumerate(p):
        if c == 0:
            continue
        terms.append(_term(c, i, x))

    if not terms:
        return IRInteger(0)
    if len(terms) == 1:
        return terms[0]
    # Fold terms into a left-associative binary ``Add`` chain. The VM's
    # Add handler — and every other arithmetic handler — is strictly
    # binary; emitting an n-ary apply would trip the arity check on the
    # first ``vm.eval``. Left-associative matches the shape produced by
    # the MACSYMA compiler, so downstream code sees a uniform tree.
    acc = terms[0]
    for term in terms[1:]:
        acc = IRApply(ADD, (acc, term))
    return acc


def _term(c, i: int, x: IRSymbol) -> IRNode:
    """Build ``c · x^i`` in the canonical shape."""
    if i == 0:
        return _coef(c)
    # Build x^i — bare x for i == 1, otherwise Pow(x, i).
    power: IRNode = x if i == 1 else IRApply(POW, (x, IRInteger(i)))
    # Drop the coefficient when it's 1; emit a NEG wrapper for -1.
    if c == 1:
        return power
    if c == -1:
        return IRApply(NEG, (power,))
    return IRApply(MUL, (_coef(c), power))


def _coef(c) -> IRNode:
    """Lift a Fraction / int coefficient into its canonical IR literal."""
    if isinstance(c, Fraction):
        if c.denominator == 1:
            return IRInteger(c.numerator)
        return IRRational(c.numerator, c.denominator)
    # int and other whole-number types.
    return IRInteger(int(c))


# ---------------------------------------------------------------------------
# Linear-argument IR builder (shared by exp_integral.py and log_integral.py)
# ---------------------------------------------------------------------------


def linear_to_ir(a: Fraction, b: Fraction, x: IRSymbol) -> IRNode:
    """Build IR for ``a·x + b``.

    Produces the simplest canonical form:
    - ``a == 0``         → constant ``b``
    - ``a == 1, b == 0`` → bare ``x``
    - ``b == 0``         → ``a·x``  (or ``Neg(x)`` for ``a == −1``)
    - general            → ``Add(a·x, b)``
    """
    if a == 0:
        return _coef(Fraction(b))

    # Build the a·x term.
    if a == 1:
        ax: IRNode = x
    elif a == -1:
        ax = IRApply(NEG, (x,))
    else:
        ax = IRApply(MUL, (_coef(Fraction(a)), x))

    if b == 0:
        return ax

    return IRApply(ADD, (ax, _coef(Fraction(b))))


# ---------------------------------------------------------------------------
# Log-sum IR builder (shared by integrate.py and mixed_integral.py)
# ---------------------------------------------------------------------------


def rt_pairs_to_ir(pairs, x: IRSymbol) -> IRNode:
    """Assemble ``Σ c_i · log(v_i)`` as IR from Rothstein–Trager pairs.

    Each pair is ``(c: Fraction, v: Polynomial)`` with ``v`` monic and
    non-constant. The emitted IR is a left-associative binary ``Add``
    chain of log terms; the chain collapses to a single node when there
    is only one pair. A coefficient of ``1`` renders as bare ``Log(v)``
    (no redundant ``Mul(1, ·)``); ``−1`` renders as ``Neg(Log(v))``;
    integer coefficients render as ``Mul(IRInteger, Log(v))``.
    """
    terms: list[IRNode] = []
    for c, v in pairs:
        log_ir = IRApply(LOG, (from_polynomial(v, x),))
        if c == 1:
            terms.append(log_ir)
        elif c == -1:
            terms.append(IRApply(NEG, (log_ir,)))
        else:
            if c.denominator == 1:
                coef: IRNode = IRInteger(c.numerator)
            else:
                coef = IRRational(c.numerator, c.denominator)
            terms.append(IRApply(MUL, (coef, log_ir)))
    if len(terms) == 1:
        return terms[0]
    acc = terms[0]
    for t in terms[1:]:
        acc = IRApply(ADD, (acc, t))
    return acc
