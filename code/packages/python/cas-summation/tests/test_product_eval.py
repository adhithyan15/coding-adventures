"""Unit tests for product_eval.py — symbolic product evaluation."""


from symbolic_ir import GAMMA_FUNC, IRApply, IRInteger, IRRational, IRSymbol

from cas_summation.product_eval import evaluate_product_expr

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _eval_ir(node) -> float:
    """Naively evaluate an IR arithmetic tree to a Python float."""
    from symbolic_ir import ADD, DIV, MUL, NEG, POW, SUB, IRApply, IRFloat, IRInteger

    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRApply):
        if node.head == ADD:
            return sum(_eval_ir(a) for a in node.args)
        if node.head == SUB:
            return _eval_ir(node.args[0]) - _eval_ir(node.args[1])
        if node.head == MUL:
            result = 1.0
            for a in node.args:
                result *= _eval_ir(a)
            return result
        if node.head == DIV:
            return _eval_ir(node.args[0]) / _eval_ir(node.args[1])
        if node.head == NEG:
            return -_eval_ir(node.args[0])
        if node.head == POW:
            return _eval_ir(node.args[0]) ** _eval_ir(node.args[1])
    raise ValueError(f"Cannot eval: {node}")


_k = IRSymbol("k")
_n = IRSymbol("n")


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestProductEval:
    def test_factorial_product_k(self):
        """product(k, k, 1, n) → GammaFunc(n+1) (= n!)."""
        result = evaluate_product_expr(_k, _k, IRInteger(1), _n, vm=None)
        assert result is not None
        # Result should be GAMMA_FUNC(n + 1)
        assert isinstance(result, IRApply) and result.head == GAMMA_FUNC
        from symbolic_ir import ADD
        arg = result.args[0]
        assert isinstance(arg, IRApply) and arg.head == ADD

    def test_factorial_concrete(self):
        """product(k, k, 1, 4) → GammaFunc(5) shape."""
        result = evaluate_product_expr(_k, _k, IRInteger(1), IRInteger(4), vm=None)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == GAMMA_FUNC

    def test_constant_factor(self):
        """product(3, k, 1, n) → 3^(n - 1 + 1) = 3^n shape."""
        result = evaluate_product_expr(IRInteger(3), _k, IRInteger(1), _n, vm=None)
        assert result is not None
        from symbolic_ir import POW
        assert isinstance(result, IRApply) and result.head == POW

    def test_scaled_factorial(self):
        """product(2*k, k, 1, n) → 2^n * GammaFunc(n+1) shape."""
        from symbolic_ir import MUL
        f = IRApply(MUL, (IRInteger(2), _k))
        result = evaluate_product_expr(f, _k, IRInteger(1), _n, vm=None)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == MUL

    def test_non_lo1_returns_none_for_factorial(self):
        """product(k, k, 0, n) — lo=0 doesn't match the factorial pattern."""
        result = evaluate_product_expr(_k, _k, IRInteger(0), _n, vm=None)
        # lo=0 is not the factorial case
        # might match "constant in k"? No, k depends on k.
        # We expect None here since the special case requires lo=1.
        # Actually, let's check: the constant check fails (k depends on k),
        # and the factorial check requires lo=1. So it falls through.
        assert result is None

    def test_unrecognised_returns_none(self):
        """product(k^2, k, 1, n) — not a recognised pattern."""
        from symbolic_ir import POW
        f = IRApply(POW, (_k, IRInteger(2)))
        result = evaluate_product_expr(f, _k, IRInteger(1), _n, vm=None)
        assert result is None
