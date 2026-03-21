"""Tests for built-in autograd Functions: forward and backward.

For each function we verify:
1. Forward output matches expected values
2. Backward gradient matches hand-computed values
3. Numerical gradient check: (f(x+h) - f(x-h)) / 2h ~= analytical
"""

import math

from ml_framework_core.tensor import Tensor

# =========================================================================
# Numerical gradient checking helper
# =========================================================================


def numerical_gradient(
    f, x: Tensor, h: float = 1e-5
) -> list[float]:
    """Compute numerical gradient of scalar-valued f w.r.t. x.

    Uses central difference: (f(x+h) - f(x-h)) / (2h)
    """
    grads = []
    for i in range(len(x.data)):
        # f(x + h)
        x_plus = Tensor(list(x.data), x.shape, device=x.device)
        x_plus.data[i] += h
        y_plus = f(x_plus).data[0]

        # f(x - h)
        x_minus = Tensor(list(x.data), x.shape, device=x.device)
        x_minus.data[i] -= h
        y_minus = f(x_minus).data[0]

        grads.append((y_plus - y_minus) / (2 * h))
    return grads


def check_gradient(f, x: Tensor, atol: float = 1e-4):
    """Compare analytical and numerical gradients.

    The function f must return a scalar tensor (numel == 1).
    """
    x_grad = Tensor(
        list(x.data), x.shape, requires_grad=True, device=x.device
    )
    y = f(x_grad)
    if not y.requires_grad:
        return
    y.backward()
    analytical = x_grad.grad.data

    numerical = numerical_gradient(f, x)
    for a, n in zip(analytical, numerical, strict=True):
        assert abs(a - n) < atol, (
            f"Analytical {a} != Numerical {n}"
        )


# =========================================================================
# AddFunction
# =========================================================================


class TestAddFunction:
    def test_forward(self):
        a = Tensor.from_list([1.0, 2.0, 3.0])
        b = Tensor.from_list([4.0, 5.0, 6.0])
        c = a + b
        assert c.data == [5.0, 7.0, 9.0]

    def test_backward_both_grad(self):
        a = Tensor.from_list([1.0, 2.0], requires_grad=True)
        b = Tensor.from_list([3.0, 4.0], requires_grad=True)
        c = (a + b).sum()
        c.backward()
        assert a.grad.data == [1.0, 1.0]
        assert b.grad.data == [1.0, 1.0]

    def test_backward_one_grad(self):
        a = Tensor.from_list([1.0, 2.0], requires_grad=True)
        b = Tensor.from_list([3.0, 4.0])
        c = (a + b).sum()
        c.backward()
        assert a.grad.data == [1.0, 1.0]

    def test_numerical_gradient(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        b = Tensor.from_list([4.0, 5.0, 6.0])
        check_gradient(lambda t: (t + b).sum(), x)


# =========================================================================
# SubFunction
# =========================================================================


class TestSubFunction:
    def test_forward(self):
        a = Tensor.from_list([5.0, 7.0])
        b = Tensor.from_list([1.0, 3.0])
        c = a - b
        assert c.data == [4.0, 4.0]

    def test_backward(self):
        a = Tensor.from_list([5.0, 7.0], requires_grad=True)
        b = Tensor.from_list([1.0, 3.0], requires_grad=True)
        c = (a - b).sum()
        c.backward()
        assert a.grad.data == [1.0, 1.0]
        assert b.grad.data == [-1.0, -1.0]

    def test_numerical_gradient(self):
        x = Tensor.from_list([5.0, 7.0])
        b = Tensor.from_list([1.0, 3.0])
        check_gradient(lambda t: (t - b).sum(), x)


# =========================================================================
# MulFunction
# =========================================================================


class TestMulFunction:
    def test_forward(self):
        a = Tensor.from_list([2.0, 3.0])
        b = Tensor.from_list([4.0, 5.0])
        c = a * b
        assert c.data == [8.0, 15.0]

    def test_backward(self):
        a = Tensor.from_list([2.0, 3.0], requires_grad=True)
        b = Tensor.from_list([4.0, 5.0], requires_grad=True)
        c = (a * b).sum()
        c.backward()
        # dL/da = b, dL/db = a
        assert a.grad.data == [4.0, 5.0]
        assert b.grad.data == [2.0, 3.0]

    def test_backward_scalar(self):
        x = Tensor.from_list([3.0], requires_grad=True)
        y = (x * 5.0).sum()
        y.backward()
        assert abs(x.grad.data[0] - 5.0) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([2.0, 3.0])
        b = Tensor.from_list([4.0, 5.0])
        check_gradient(lambda t: (t * b).sum(), x)


# =========================================================================
# DivFunction
# =========================================================================


class TestDivFunction:
    def test_forward(self):
        a = Tensor.from_list([6.0, 8.0])
        b = Tensor.from_list([2.0, 4.0])
        c = a / b
        assert c.data == [3.0, 2.0]

    def test_backward_numerator(self):
        a = Tensor.from_list([6.0], requires_grad=True)
        b = Tensor.from_list([3.0])
        c = (a / b).sum()
        c.backward()
        # dL/da = 1/b = 1/3
        assert abs(a.grad.data[0] - 1.0 / 3.0) < 1e-10

    def test_backward_denominator(self):
        a = Tensor.from_list([6.0], requires_grad=True)
        b = Tensor.from_list([3.0], requires_grad=True)
        c = (a / b).sum()
        c.backward()
        # dL/db = -a/b^2 = -6/9 = -2/3
        assert abs(b.grad.data[0] - (-6.0 / 9.0)) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([6.0, 8.0])
        d = Tensor.from_list([2.0, 4.0])
        check_gradient(lambda t: (t / d).sum(), x)

    def test_numerical_gradient_denominator(self):
        x = Tensor.from_list([2.0, 4.0])
        n = Tensor.from_list([6.0, 8.0])
        check_gradient(lambda t: (n / t).sum(), x)


# =========================================================================
# NegFunction
# =========================================================================


class TestNegFunction:
    def test_forward(self):
        a = Tensor.from_list([1.0, -2.0, 3.0])
        c = -a
        assert c.data == [-1.0, 2.0, -3.0]

    def test_backward(self):
        x = Tensor.from_list([1.0, -2.0], requires_grad=True)
        y = (-x).sum()
        y.backward()
        assert x.grad.data == [-1.0, -1.0]

    def test_numerical_gradient(self):
        x = Tensor.from_list([1.0, -2.0, 3.0])
        check_gradient(lambda t: (-t).sum(), x)


# =========================================================================
# PowFunction
# =========================================================================


class TestPowFunction:
    def test_forward_square(self):
        a = Tensor.from_list([2.0, 3.0])
        c = a**2.0
        assert c.data == [4.0, 9.0]

    def test_forward_sqrt(self):
        a = Tensor.from_list([4.0, 9.0])
        c = a**0.5
        assert abs(c.data[0] - 2.0) < 1e-10
        assert abs(c.data[1] - 3.0) < 1e-10

    def test_forward_cube(self):
        a = Tensor.from_list([2.0])
        c = a**3.0
        assert abs(c.data[0] - 8.0) < 1e-10

    def test_backward_square(self):
        x = Tensor.from_list([3.0], requires_grad=True)
        y = (x**2.0).sum()
        y.backward()
        # d(x^2)/dx = 2x = 6
        assert abs(x.grad.data[0] - 6.0) < 1e-10

    def test_backward_sqrt(self):
        x = Tensor.from_list([4.0], requires_grad=True)
        y = (x**0.5).sum()
        y.backward()
        # d(x^0.5)/dx = 0.5 * x^(-0.5) = 0.5/2 = 0.25
        assert abs(x.grad.data[0] - 0.25) < 1e-10

    def test_numerical_gradient_square(self):
        x = Tensor.from_list([2.0, 3.0])
        check_gradient(lambda t: (t**2.0).sum(), x)

    def test_numerical_gradient_cube(self):
        x = Tensor.from_list([1.5, 2.5])
        check_gradient(lambda t: (t**3.0).sum(), x)


# =========================================================================
# MatMulFunction
# =========================================================================

import pytest  # noqa: E402


class TestMatMulFunction:
    def test_forward_2x2(self):
        a = Tensor.from_list([[1, 0], [0, 1]])
        b = Tensor.from_list([[5, 6], [7, 8]])
        c = a @ b
        assert c.data == [5.0, 6.0, 7.0, 8.0]

    def test_forward_non_square(self):
        a = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        b = Tensor.from_list([[1, 0], [0, 1], [1, 0]])
        c = a @ b
        assert c.shape == (2, 2)
        assert c.data == [4.0, 2.0, 10.0, 5.0]

    def test_forward_shape_mismatch_raises(self):
        a = Tensor.from_list([[1, 2]])
        b = Tensor.from_list([[1, 2]])
        with pytest.raises(ValueError, match="shape mismatch"):
            a @ b

    def test_forward_non_2d_raises(self):
        a = Tensor.from_list([1.0, 2.0])
        b = Tensor.from_list([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match="2-D"):
            a @ b

    def test_backward_a(self):
        a = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        b = Tensor.from_list([[3.0], [4.0]])
        c = a @ b  # (1,2) @ (2,1) = (1,1)
        c.sum().backward()
        # dL/dA = grad @ B.T = [[1]] @ [[3, 4]] = [[3, 4]]
        assert abs(a.grad.data[0] - 3.0) < 1e-10
        assert abs(a.grad.data[1] - 4.0) < 1e-10

    def test_backward_b(self):
        a = Tensor.from_list([[1.0, 2.0]])
        b = Tensor.from_list([[3.0], [4.0]], requires_grad=True)
        c = a @ b
        c.sum().backward()
        # dL/dB = A.T @ grad = [[1],[2]] @ [[1]] = [[1],[2]]
        assert abs(b.grad.data[0] - 1.0) < 1e-10
        assert abs(b.grad.data[1] - 2.0) < 1e-10

    def test_numerical_gradient_a(self):
        b = Tensor.from_list([[3.0], [4.0]])
        x = Tensor.from_list([[1.0, 2.0]])
        check_gradient(lambda t: (t @ b).sum(), x)


# =========================================================================
# SumFunction
# =========================================================================


class TestSumFunction:
    def test_forward_all(self):
        t = Tensor.from_list([1.0, 2.0, 3.0, 4.0])
        s = t.sum()
        assert s.data == [10.0]
        assert s.shape == (1,)

    def test_forward_dim0(self):
        t = Tensor.from_list([[1, 2], [3, 4]])
        s = t.sum(dim=0)
        assert s.data == [4.0, 6.0]
        assert s.shape == (2,)

    def test_forward_dim1(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        s = t.sum(dim=1)
        assert s.data == [6.0, 15.0]
        assert s.shape == (2,)

    def test_forward_keepdim(self):
        t = Tensor.from_list([[1, 2], [3, 4]])
        s = t.sum(dim=0, keepdim=True)
        assert s.shape == (1, 2)
        assert s.data == [4.0, 6.0]

    def test_forward_negative_dim(self):
        t = Tensor.from_list([[1, 2], [3, 4]])
        s = t.sum(dim=-1)
        assert s.data == [3.0, 7.0]

    def test_backward_all(self):
        x = Tensor.from_list([1.0, 2.0, 3.0], requires_grad=True)
        s = x.sum()
        s.backward()
        assert x.grad.data == [1.0, 1.0, 1.0]

    def test_backward_dim(self):
        x = Tensor.from_list(
            [[1.0, 2.0], [3.0, 4.0]], requires_grad=True
        )
        s = x.sum(dim=0)
        s.backward(Tensor.from_list([1.0, 1.0]))
        assert x.grad.data == [1.0, 1.0, 1.0, 1.0]

    def test_numerical_gradient(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        check_gradient(lambda t: t.sum(), x)

    def test_numerical_gradient_dim(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        check_gradient(lambda t: t.sum(dim=0).sum(), x)


# =========================================================================
# MeanFunction
# =========================================================================


class TestMeanFunction:
    def test_forward_all(self):
        t = Tensor.from_list([2.0, 4.0, 6.0])
        m = t.mean()
        assert abs(m.data[0] - 4.0) < 1e-10

    def test_forward_dim(self):
        t = Tensor.from_list([[1, 3], [5, 7]])
        m = t.mean(dim=0)
        assert abs(m.data[0] - 3.0) < 1e-10
        assert abs(m.data[1] - 5.0) < 1e-10

    def test_forward_dim1(self):
        t = Tensor.from_list([[2, 4], [6, 8]])
        m = t.mean(dim=1)
        assert abs(m.data[0] - 3.0) < 1e-10
        assert abs(m.data[1] - 7.0) < 1e-10

    def test_backward_all(self):
        x = Tensor.from_list([2.0, 4.0, 6.0], requires_grad=True)
        m = x.mean()
        m.backward()
        for g in x.grad.data:
            assert abs(g - 1.0 / 3.0) < 1e-10

    def test_backward_dim(self):
        x = Tensor.from_list(
            [[1.0, 2.0], [3.0, 4.0]], requires_grad=True
        )
        m = x.mean(dim=0)
        m.backward(Tensor.from_list([1.0, 1.0]))
        for g in x.grad.data:
            assert abs(g - 0.5) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([2.0, 4.0, 6.0])
        check_gradient(lambda t: t.mean(), x)

    def test_numerical_gradient_dim(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        check_gradient(lambda t: t.mean(dim=0).sum(), x)


# =========================================================================
# ExpFunction
# =========================================================================


class TestExpFunction:
    def test_forward(self):
        t = Tensor.from_list([0.0, 1.0, 2.0])
        r = t.exp()
        assert abs(r.data[0] - 1.0) < 1e-10
        assert abs(r.data[1] - math.e) < 1e-6
        assert abs(r.data[2] - math.exp(2.0)) < 1e-6

    def test_backward(self):
        x = Tensor.from_list([1.0], requires_grad=True)
        y = x.exp().sum()
        y.backward()
        assert abs(x.grad.data[0] - math.e) < 1e-6

    def test_backward_zero(self):
        x = Tensor.from_list([0.0], requires_grad=True)
        y = x.exp().sum()
        y.backward()
        assert abs(x.grad.data[0] - 1.0) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([0.5, 1.0, -0.5])
        check_gradient(lambda t: t.exp().sum(), x)


# =========================================================================
# LogFunction
# =========================================================================


class TestLogFunction:
    def test_forward(self):
        t = Tensor.from_list([1.0, math.e, math.exp(2.0)])
        r = t.log()
        assert abs(r.data[0] - 0.0) < 1e-10
        assert abs(r.data[1] - 1.0) < 1e-10
        assert abs(r.data[2] - 2.0) < 1e-6

    def test_forward_zero(self):
        t = Tensor.from_list([0.0])
        r = t.log()
        assert r.data[0] == float("-inf")

    def test_backward(self):
        x = Tensor.from_list([2.0], requires_grad=True)
        y = x.log().sum()
        y.backward()
        assert abs(x.grad.data[0] - 0.5) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([0.5, 1.0, 2.0, 5.0])
        check_gradient(lambda t: t.log().sum(), x)


# =========================================================================
# AbsFunction
# =========================================================================


class TestAbsFunction:
    def test_forward(self):
        t = Tensor.from_list([-3.0, 0.0, 4.0])
        r = t.abs()
        assert r.data == [3.0, 0.0, 4.0]

    def test_backward_positive(self):
        x = Tensor.from_list([2.0], requires_grad=True)
        y = x.abs().sum()
        y.backward()
        assert abs(x.grad.data[0] - 1.0) < 1e-10

    def test_backward_negative(self):
        x = Tensor.from_list([-3.0], requires_grad=True)
        y = x.abs().sum()
        y.backward()
        assert abs(x.grad.data[0] - (-1.0)) < 1e-10

    def test_backward_zero(self):
        x = Tensor.from_list([0.0], requires_grad=True)
        y = x.abs().sum()
        y.backward()
        assert abs(x.grad.data[0] - 0.0) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([-3.0, 2.0, 5.0])
        check_gradient(lambda t: t.abs().sum(), x)


# =========================================================================
# ClampFunction
# =========================================================================


class TestClampFunction:
    def test_forward_both_bounds(self):
        t = Tensor.from_list([-5.0, 0.0, 10.0])
        r = t.clamp(min_val=-1.0, max_val=5.0)
        assert r.data == [-1.0, 0.0, 5.0]

    def test_forward_min_only(self):
        t = Tensor.from_list([-5.0, 3.0])
        r = t.clamp(min_val=0.0)
        assert r.data == [0.0, 3.0]

    def test_forward_max_only(self):
        t = Tensor.from_list([3.0, 10.0])
        r = t.clamp(max_val=5.0)
        assert r.data == [3.0, 5.0]

    def test_backward_in_range(self):
        x = Tensor.from_list([2.0], requires_grad=True)
        y = x.clamp(min_val=0.0, max_val=5.0).sum()
        y.backward()
        assert abs(x.grad.data[0] - 1.0) < 1e-10

    def test_backward_clamped_min(self):
        x = Tensor.from_list([-2.0], requires_grad=True)
        y = x.clamp(min_val=0.0, max_val=5.0).sum()
        y.backward()
        assert abs(x.grad.data[0] - 0.0) < 1e-10

    def test_backward_clamped_max(self):
        x = Tensor.from_list([10.0], requires_grad=True)
        y = x.clamp(min_val=0.0, max_val=5.0).sum()
        y.backward()
        assert abs(x.grad.data[0] - 0.0) < 1e-10

    def test_numerical_gradient(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        check_gradient(
            lambda t: t.clamp(min_val=0.0, max_val=5.0).sum(), x
        )


# =========================================================================
# ReLUFunction
# =========================================================================


class TestReLUFunction:
    def test_forward_positive(self):
        from ml_framework_core.functions import ReLUFunction

        t = Tensor.from_list([1.0, 2.0, 3.0])
        r = ReLUFunction.apply(t)
        assert r.data == [1.0, 2.0, 3.0]

    def test_forward_negative(self):
        from ml_framework_core.functions import ReLUFunction

        t = Tensor.from_list([-1.0, -2.0, -3.0])
        r = ReLUFunction.apply(t)
        assert r.data == [0.0, 0.0, 0.0]

    def test_forward_mixed(self):
        from ml_framework_core.functions import ReLUFunction

        t = Tensor.from_list([-2.0, 0.0, 3.0])
        r = ReLUFunction.apply(t)
        assert r.data == [0.0, 0.0, 3.0]

    def test_backward_positive(self):
        from ml_framework_core.functions import ReLUFunction

        x = Tensor.from_list([2.0, 3.0], requires_grad=True)
        y = ReLUFunction.apply(x).sum()
        y.backward()
        assert x.grad.data == [1.0, 1.0]

    def test_backward_negative(self):
        from ml_framework_core.functions import ReLUFunction

        x = Tensor.from_list([-2.0, -3.0], requires_grad=True)
        y = ReLUFunction.apply(x).sum()
        y.backward()
        assert x.grad.data == [0.0, 0.0]

    def test_backward_mixed(self):
        from ml_framework_core.functions import ReLUFunction

        x = Tensor.from_list(
            [-1.0, 2.0, -3.0, 4.0], requires_grad=True
        )
        y = ReLUFunction.apply(x).sum()
        y.backward()
        assert x.grad.data == [0.0, 1.0, 0.0, 1.0]

    def test_numerical_gradient(self):
        from ml_framework_core.functions import ReLUFunction

        x = Tensor.from_list([1.0, 2.0, -1.0, -2.0])
        check_gradient(lambda t: ReLUFunction.apply(t).sum(), x)


# =========================================================================
# SigmoidFunction
# =========================================================================


class TestSigmoidFunction:
    def test_forward_zero(self):
        from ml_framework_core.functions import SigmoidFunction

        t = Tensor.from_list([0.0])
        r = SigmoidFunction.apply(t)
        assert abs(r.data[0] - 0.5) < 1e-10

    def test_forward_large_positive(self):
        from ml_framework_core.functions import SigmoidFunction

        t = Tensor.from_list([10.0])
        r = SigmoidFunction.apply(t)
        assert r.data[0] > 0.999

    def test_forward_large_negative(self):
        from ml_framework_core.functions import SigmoidFunction

        t = Tensor.from_list([-10.0])
        r = SigmoidFunction.apply(t)
        assert r.data[0] < 0.001

    def test_backward(self):
        from ml_framework_core.functions import SigmoidFunction

        x = Tensor.from_list([0.0], requires_grad=True)
        y = SigmoidFunction.apply(x).sum()
        y.backward()
        # sigmoid'(0) = 0.5 * 0.5 = 0.25
        assert abs(x.grad.data[0] - 0.25) < 1e-10

    def test_numerical_gradient(self):
        from ml_framework_core.functions import SigmoidFunction

        x = Tensor.from_list([-1.0, 0.0, 1.0, 2.0])
        check_gradient(
            lambda t: SigmoidFunction.apply(t).sum(), x
        )


# =========================================================================
# TanhFunction
# =========================================================================


class TestTanhFunction:
    def test_forward_zero(self):
        from ml_framework_core.functions import TanhFunction

        t = Tensor.from_list([0.0])
        r = TanhFunction.apply(t)
        assert abs(r.data[0]) < 1e-10

    def test_forward_values(self):
        from ml_framework_core.functions import TanhFunction

        t = Tensor.from_list([1.0])
        r = TanhFunction.apply(t)
        assert abs(r.data[0] - math.tanh(1.0)) < 1e-10

    def test_backward(self):
        from ml_framework_core.functions import TanhFunction

        x = Tensor.from_list([0.0], requires_grad=True)
        y = TanhFunction.apply(x).sum()
        y.backward()
        assert abs(x.grad.data[0] - 1.0) < 1e-10

    def test_backward_nonzero(self):
        from ml_framework_core.functions import TanhFunction

        x = Tensor.from_list([1.0], requires_grad=True)
        y = TanhFunction.apply(x).sum()
        y.backward()
        expected = 1.0 - math.tanh(1.0) ** 2
        assert abs(x.grad.data[0] - expected) < 1e-10

    def test_numerical_gradient(self):
        from ml_framework_core.functions import TanhFunction

        x = Tensor.from_list([-1.0, 0.0, 0.5, 2.0])
        check_gradient(
            lambda t: TanhFunction.apply(t).sum(), x
        )


# =========================================================================
# GELUFunction
# =========================================================================


class TestGELUFunction:
    def test_forward_zero(self):
        from ml_framework_core.functions import GELUFunction

        t = Tensor.from_list([0.0])
        r = GELUFunction.apply(t)
        assert abs(r.data[0]) < 1e-10

    def test_forward_positive(self):
        from ml_framework_core.functions import GELUFunction

        t = Tensor.from_list([3.0])
        r = GELUFunction.apply(t)
        assert r.data[0] > 2.9

    def test_forward_negative(self):
        from ml_framework_core.functions import GELUFunction

        t = Tensor.from_list([-3.0])
        r = GELUFunction.apply(t)
        assert abs(r.data[0]) < 0.1

    def test_backward(self):
        from ml_framework_core.functions import GELUFunction

        x = Tensor.from_list([0.0], requires_grad=True)
        y = GELUFunction.apply(x).sum()
        y.backward()
        assert abs(x.grad.data[0] - 0.5) < 1e-6

    def test_numerical_gradient(self):
        from ml_framework_core.functions import GELUFunction

        x = Tensor.from_list([-1.0, 0.0, 0.5, 2.0])
        check_gradient(
            lambda t: GELUFunction.apply(t).sum(), x
        )


# =========================================================================
# SoftmaxFunction
# =========================================================================


class TestSoftmaxFunction:
    def test_forward_sums_to_one(self):
        from ml_framework_core.functions import SoftmaxFunction

        t = Tensor.from_list([1.0, 2.0, 3.0])
        r = SoftmaxFunction.apply(t, 0)
        total = sum(r.data)
        assert abs(total - 1.0) < 1e-10

    def test_forward_all_equal(self):
        from ml_framework_core.functions import SoftmaxFunction

        t = Tensor.from_list([1.0, 1.0, 1.0])
        r = SoftmaxFunction.apply(t, 0)
        for v in r.data:
            assert abs(v - 1.0 / 3.0) < 1e-10

    def test_forward_2d_dim1(self):
        from ml_framework_core.functions import SoftmaxFunction

        t = Tensor.from_list([[1, 2], [3, 4]])
        r = SoftmaxFunction.apply(t, 1)
        assert abs(r.data[0] + r.data[1] - 1.0) < 1e-10
        assert abs(r.data[2] + r.data[3] - 1.0) < 1e-10

    def test_forward_2d_dim0(self):
        from ml_framework_core.functions import SoftmaxFunction

        t = Tensor.from_list([[1, 2], [3, 4]])
        r = SoftmaxFunction.apply(t, 0)
        assert abs(r.data[0] + r.data[2] - 1.0) < 1e-10
        assert abs(r.data[1] + r.data[3] - 1.0) < 1e-10

    def test_forward_numerical_stability(self):
        from ml_framework_core.functions import SoftmaxFunction

        t = Tensor.from_list([1000.0, 1001.0, 1002.0])
        r = SoftmaxFunction.apply(t, 0)
        assert abs(sum(r.data) - 1.0) < 1e-10

    def test_backward_1d(self):
        from ml_framework_core.functions import SoftmaxFunction

        x = Tensor.from_list([1.0, 2.0, 3.0], requires_grad=True)
        y = SoftmaxFunction.apply(x, 0)
        loss = y.sum()
        loss.backward()
        for g in x.grad.data:
            assert abs(g) < 1e-6

    def test_numerical_gradient(self):
        from ml_framework_core.functions import SoftmaxFunction

        x = Tensor.from_list([1.0, 2.0, 3.0])

        def f(t):
            s = SoftmaxFunction.apply(t, 0)
            w = Tensor.from_list([1.0, 0.0, 0.0])
            return (s * w).sum()

        check_gradient(f, x)

    def test_numerical_gradient_2d(self):
        from ml_framework_core.functions import SoftmaxFunction

        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])

        def f(t):
            s = SoftmaxFunction.apply(t, 1)
            w = Tensor.from_list([[1.0, 0.0], [1.0, 0.0]])
            return (s * w).sum()

        check_gradient(f, x, atol=1e-3)


# =========================================================================
# ReshapeFunction
# =========================================================================


class TestReshapeFunction:
    def test_forward(self):
        t = Tensor.from_list([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
        r = t.reshape(2, 3)
        assert r.shape == (2, 3)
        assert r.data == t.data

    def test_forward_flatten(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        r = t.reshape(6)
        assert r.shape == (6,)

    def test_backward(self):
        x = Tensor.from_list(
            [1.0, 2.0, 3.0, 4.0], requires_grad=True
        )
        y = x.reshape(2, 2).sum()
        y.backward()
        assert x.grad.data == [1.0, 1.0, 1.0, 1.0]
        assert x.grad.shape == (4,)


# =========================================================================
# TransposeFunction
# =========================================================================


class TestTransposeFunction:
    def test_forward_2d(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        r = t.transpose(0, 1)
        assert r.shape == (3, 2)
        expected = [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
        assert r.data == expected

    def test_forward_3d(self):
        t = Tensor.from_list(
            [[[1], [2], [3]], [[4], [5], [6]]]
        )
        assert t.shape == (2, 3, 1)
        r = t.transpose(0, 2)
        assert r.shape == (1, 3, 2)

    def test_backward_2d(self):
        x = Tensor.from_list(
            [[1.0, 2.0], [3.0, 4.0]], requires_grad=True
        )
        y = x.transpose(0, 1).sum()
        y.backward()
        assert x.grad.data == [1.0, 1.0, 1.0, 1.0]

    def test_roundtrip(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        r = t.transpose(0, 1).transpose(0, 1)
        assert r.shape == t.shape
        assert r.data == t.data


# =========================================================================
# Edge cases and combined operations
# =========================================================================


class TestFunctionEdgeCases:
    def test_add_with_zero(self):
        a = Tensor.from_list([1.0, 2.0])
        b = Tensor.zeros(2)
        c = a + b
        assert c.data == [1.0, 2.0]

    def test_mul_with_one(self):
        a = Tensor.from_list([3.0, 4.0])
        b = Tensor.ones(2)
        c = a * b
        assert c.data == [3.0, 4.0]

    def test_mul_with_zero(self):
        a = Tensor.from_list([3.0, 4.0])
        b = Tensor.zeros(2)
        c = a * b
        assert c.data == [0.0, 0.0]

    def test_pow_zero(self):
        a = Tensor.from_list([2.0, 3.0])
        c = a**0.0
        assert c.data == [1.0, 1.0]

    def test_pow_one(self):
        a = Tensor.from_list([2.0, 3.0])
        c = a**1.0
        assert c.data == [2.0, 3.0]

    def test_identity_matmul(self):
        eye = Tensor.eye(3)
        mat = Tensor.from_list(
            [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
        )
        c = eye @ mat
        assert c.data == mat.data

    def test_chained_arithmetic(self):
        x = Tensor.from_list([2.0], requires_grad=True)
        y = ((x * 3.0) + 1.0) ** 2.0
        z = y.sum()
        z.backward()
        # dy/dx = 2*(3x+1)*3 = 6*(3*2+1) = 42
        assert abs(x.grad.data[0] - 42.0) < 1e-6
