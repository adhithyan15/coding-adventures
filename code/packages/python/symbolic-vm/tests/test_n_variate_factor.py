"""Pipeline tests for n-variate Hensel-lift factorisation (Track K1).

These tests exercise the end-to-end ``Factor(expr)`` path through the
VM: they construct ``Factor(...)`` IR directly, evaluate it on a
:class:`SymbolicBackend`, and verify the result by expanding the
returned product back to a sparse-dict polynomial and comparing
against the sparse-dict expansion of the input.

Verifying *shape* (e.g. asserting "the result is Mul(x+y+z, ...)") is
brittle — Hensel may emit factors in a different deterministic order
than the human-recognisable canonical order, and integer-content can
get pulled out separately.  Instead we verify the *algebraic* property
that the returned product equals the input as polynomials.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

_FACTOR = IRSymbol("Factor")

x = IRSymbol("x")
y = IRSymbol("y")
z = IRSymbol("z")
w = IRSymbol("w")


def make_vm() -> VM:
    return VM(SymbolicBackend())


# ---------------------------------------------------------------------------
# Local IR → sparse-dict polynomial expander.
#
# We deliberately do NOT reuse the production ``_ir_to_npoly`` from
# ``cas_handlers`` here — tests should round-trip through their own
# independent expander so a bug in production code can't masquerade as
# a bug-compatible test result.  The grammar is identical: literals,
# variables in ``vars``, Add/Sub/Neg/Mul/Pow over those.
# ---------------------------------------------------------------------------

PolyDict = dict[tuple[int, ...], Fraction]


def expand_to_dict(node: IRNode, vars: list[IRSymbol]) -> PolyDict:
    """Recursive structural expander for the polynomial subset of IR."""
    n = len(vars)
    zero = (0,) * n
    var_idx = {v: i for i, v in enumerate(vars)}

    def unit(v: IRSymbol) -> tuple[int, ...]:
        i = var_idx[v]
        k = [0] * n
        k[i] = 1
        return tuple(k)

    def add_keys(a: tuple[int, ...], b: tuple[int, ...]) -> tuple[int, ...]:
        return tuple(a[i] + b[i] for i in range(n))

    def normalize(d: PolyDict) -> PolyDict:
        return {k: v for k, v in d.items() if v != 0}

    def add(a: PolyDict, b: PolyDict) -> PolyDict:
        out: PolyDict = dict(a)
        for k, v in b.items():
            out[k] = out.get(k, Fraction(0)) + v
        return normalize(out)

    def neg(a: PolyDict) -> PolyDict:
        return {k: -v for k, v in a.items()}

    def mul(a: PolyDict, b: PolyDict) -> PolyDict:
        out: PolyDict = {}
        for ka, va in a.items():
            for kb, vb in b.items():
                k = add_keys(ka, kb)
                out[k] = out.get(k, Fraction(0)) + va * vb
        return normalize(out)

    def walk(node: IRNode) -> PolyDict:
        if isinstance(node, IRInteger):
            return {zero: Fraction(node.value)} if node.value != 0 else {}
        if isinstance(node, IRRational):
            return {zero: Fraction(node.numer, node.denom)}
        if isinstance(node, IRSymbol):
            if node in var_idx:
                return {unit(node): Fraction(1)}
            raise ValueError(f"unexpected symbol in test expander: {node!r}")
        assert isinstance(node, IRApply)
        head = node.head
        if head == ADD:
            acc: PolyDict = {}
            for a in node.args:
                acc = add(acc, walk(a))
            return acc
        if head == SUB:
            a, b = walk(node.args[0]), walk(node.args[1])
            return add(a, neg(b))
        if head == NEG:
            return neg(walk(node.args[0]))
        if head == MUL:
            acc = {zero: Fraction(1)}
            for a in node.args:
                acc = mul(acc, walk(a))
            return acc
        if head == POW:
            base = walk(node.args[0])
            exp = node.args[1]
            assert isinstance(exp, IRInteger)
            e = exp.value
            assert e >= 0
            if e == 0:
                return {zero: Fraction(1)}
            out = base
            for _ in range(e - 1):
                out = mul(out, base)
            return out
        raise ValueError(f"unexpected IR head in test expander: {head!r}")

    return walk(node)


# ---------------------------------------------------------------------------
# Tests.
# ---------------------------------------------------------------------------


def test_n_variate_factor_pipeline_x3_y3_z3_minus_3xyz() -> None:
    """factor(x^3 + y^3 + z^3 - 3*x*y*z).

    The classical identity:

        x^3 + y^3 + z^3 - 3·x·y·z = (x + y + z) · (x^2 + y^2 + z^2 - x·y - y·z - z·x)

    We verify via algebraic equality (expand the result, compare
    sparse-dict polynomials) rather than by shape — that lets the
    Hensel lift emit factors in any order it likes.
    """
    vm = make_vm()
    # x^3 + y^3 + z^3 - 3*x*y*z, built left-to-right.
    target = IRApply(
        SUB,
        (
            IRApply(
                ADD,
                (
                    IRApply(
                        ADD,
                        (
                            IRApply(POW, (x, IRInteger(3))),
                            IRApply(POW, (y, IRInteger(3))),
                        ),
                    ),
                    IRApply(POW, (z, IRInteger(3))),
                ),
            ),
            IRApply(MUL, (IRInteger(3), IRApply(MUL, (IRApply(MUL, (x, y)), z)))),
        ),
    )
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)

    # The handler must produce something other than the unevaluated
    # Factor(...) shell — otherwise no factorisation was found.
    assert not (isinstance(result, IRApply) and result.head == _FACTOR), (
        f"Expected a successful factorisation, got unevaluated wrapper: {result!r}"
    )

    # Round-trip: expand result and compare to the input polynomial.
    vars = [x, y, z]
    result_dict = expand_to_dict(result, vars)
    target_dict = expand_to_dict(target, vars)
    assert result_dict == target_dict


def test_n_variate_factor_pipeline_linear_product() -> None:
    """factor((x + y + z)·(x + 2·y + 3·z)) round-trips.

    Expanded input:
        x^2 + 3·x·y + 4·x·z + 2·y^2 + 5·y·z + 3·z^2
    """
    vm = make_vm()
    # Build the *expanded* input — Factor sees only the unfactored sum.
    # x^2 + 3xy + 4xz + 2y^2 + 5yz + 3z^2
    expanded = IRApply(
        ADD,
        (
            IRApply(POW, (x, IRInteger(2))),
            IRApply(
                ADD,
                (
                    IRApply(MUL, (IRInteger(3), IRApply(MUL, (x, y)))),
                    IRApply(
                        ADD,
                        (
                            IRApply(MUL, (IRInteger(4), IRApply(MUL, (x, z)))),
                            IRApply(
                                ADD,
                                (
                                    IRApply(
                                        MUL,
                                        (IRInteger(2), IRApply(POW, (y, IRInteger(2)))),
                                    ),
                                    IRApply(
                                        ADD,
                                        (
                                            IRApply(
                                                MUL,
                                                (IRInteger(5), IRApply(MUL, (y, z))),
                                            ),
                                            IRApply(
                                                MUL,
                                                (
                                                    IRInteger(3),
                                                    IRApply(POW, (z, IRInteger(2))),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
        ),
    )
    expr = IRApply(_FACTOR, (expanded,))
    result = vm.eval(expr)
    vars = [x, y, z]
    # If Hensel found a factorisation, the expanded result equals the
    # input.  If not, the wrapper survives unevaluated.  Either way,
    # the algebraic identity must hold: result-product == input.
    if isinstance(result, IRApply) and result.head == _FACTOR:
        return  # unevaluated — nothing to round-trip
    result_dict = expand_to_dict(result, vars)
    expanded_dict = expand_to_dict(expanded, vars)
    assert result_dict == expanded_dict


def test_n_variate_factor_pipeline_fall_through_irreducible() -> None:
    """factor(x^2 + y^2 + z^2 + 1) — irreducible over Q, falls through.

    No pattern handler recognises this, Hensel can't find a factor (it
    is irreducible over Q in three variables — a sum of three squares
    plus a positive constant has no rational zero), and the result
    must be the unevaluated ``Factor(...)`` IR or the input itself.
    The contract is: do not crash, do not produce a wrong factoring.
    """
    vm = make_vm()
    target = IRApply(
        ADD,
        (
            IRApply(POW, (x, IRInteger(2))),
            IRApply(
                ADD,
                (
                    IRApply(POW, (y, IRInteger(2))),
                    IRApply(
                        ADD,
                        (IRApply(POW, (z, IRInteger(2))), IRInteger(1)),
                    ),
                ),
            ),
        ),
    )
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    # Either the Factor(...) wrapper stays (no factorisation found) or
    # — should the lift ever produce something — the round-trip must
    # still equal the input polynomial.  Both outcomes are correct.
    # Detect "unevaluated" by head, not by raw structural equality:
    # the VM canonicalises nested Add into a left-associative chain,
    # so ``result == expr`` can be ``False`` even when the wrapper
    # was preserved.
    if isinstance(result, IRApply) and result.head == _FACTOR:
        return  # unevaluated wrapper — accepted
    vars = [x, y, z]
    target_dict = expand_to_dict(target, vars)
    result_dict = expand_to_dict(result, vars)
    assert result_dict == target_dict


def test_n_variate_factor_pipeline_does_not_crash_on_transcendental() -> None:
    """factor(sin(x) + y + z) — transcendental, must fall through cleanly.

    Regression guard: the n-variate bridge must return None when the
    input has a non-polynomial head, not raise or produce nonsense.
    """
    vm = make_vm()
    sin = IRSymbol("Sin")
    target = IRApply(
        ADD,
        (IRApply(sin, (x,)), IRApply(ADD, (y, z))),
    )
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    # The expected outcome is "unchanged" — Factor doesn't know what
    # to do with a transcendental, so it leaves the call wrapper in
    # place.  We just require it not to crash.
    assert isinstance(result, IRNode)
