"""Tests for the autograd engine: backward, gradient computation, no_grad."""

import pytest

from ml_framework_core.autograd import Function, is_grad_enabled, no_grad
from ml_framework_core.tensor import Tensor

# =========================================================================
# Simple gradients
# =========================================================================


class TestSimpleGradients:
    """Test basic gradient computations through the backward algorithm."""

    def test_y_equals_2x(self):
        """y = 2*x => dy/dx = 2."""
        x = Tensor.from_list([3.0], requires_grad=True)
        y = x * 2.0
        z = y.sum()
        z.backward()
        assert x.grad is not None
        assert abs(x.grad.data[0] - 2.0) < 1e-10

    def test_y_equals_x_plus_1(self):
        """y = x + 1 => dy/dx = 1."""
        x = Tensor.from_list([5.0], requires_grad=True)
        y = x + 1.0
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 1.0) < 1e-10

    def test_y_equals_neg_x(self):
        """y = -x => dy/dx = -1."""
        x = Tensor.from_list([3.0], requires_grad=True)
        y = -x
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - (-1.0)) < 1e-10

    def test_y_equals_x_div_2(self):
        """y = x / 2 => dy/dx = 0.5."""
        x = Tensor.from_list([4.0], requires_grad=True)
        y = x / 2.0
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 0.5) < 1e-10

    def test_vector_gradient(self):
        """y = sum(2*x) for x = [1, 2, 3] => dy/dx = [2, 2, 2]."""
        x = Tensor.from_list([1.0, 2.0, 3.0], requires_grad=True)
        y = x * 2.0
        z = y.sum()
        z.backward()
        for g in x.grad.data:
            assert abs(g - 2.0) < 1e-10


# =========================================================================
# Chain rule
# =========================================================================


class TestChainRule:
    """Test chain rule: gradients through multiple operations."""

    def test_x_plus_1_squared(self):
        """y = (x+1)^2, dy/dx = 2*(x+1). At x=2, dy/dx = 6."""
        x = Tensor.from_list([2.0], requires_grad=True)
        y = (x + 1.0) ** 2.0
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 6.0) < 1e-10

    def test_three_ops_chain(self):
        """y = (2*x + 3).sum(), dy/dx = 2."""
        x = Tensor.from_list([1.0, 2.0], requires_grad=True)
        y = x * 2.0
        z = y + 3.0
        loss = z.sum()
        loss.backward()
        for g in x.grad.data:
            assert abs(g - 2.0) < 1e-10

    def test_exp_chain(self):
        """y = exp(2*x), dy/dx = 2*exp(2*x). At x=0, dy/dx = 2."""
        x = Tensor.from_list([0.0], requires_grad=True)
        y = (x * 2.0).exp()
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 2.0) < 1e-10

    def test_log_chain(self):
        """y = log(x), dy/dx = 1/x. At x=2, dy/dx = 0.5."""
        x = Tensor.from_list([2.0], requires_grad=True)
        y = x.log()
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 0.5) < 1e-10


# =========================================================================
# Multiple paths (same tensor used twice)
# =========================================================================


class TestMultiplePaths:
    """Test gradient accumulation when a tensor is used multiple times."""

    def test_x_times_x(self):
        """y = x*x, dy/dx = 2x. At x=3, dy/dx = 6."""
        x = Tensor.from_list([3.0], requires_grad=True)
        y = x * x
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 6.0) < 1e-10

    def test_x_plus_x(self):
        """y = x + x, dy/dx = 2."""
        x = Tensor.from_list([5.0], requires_grad=True)
        y = x + x
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 2.0) < 1e-10

    def test_x_times_x_vector(self):
        """y = sum(x*x) for x = [1, 2, 3] => dy/dx = [2, 4, 6]."""
        x = Tensor.from_list([1.0, 2.0, 3.0], requires_grad=True)
        y = x * x
        z = y.sum()
        z.backward()
        expected = [2.0, 4.0, 6.0]
        for g, e in zip(x.grad.data, expected, strict=True):
            assert abs(g - e) < 1e-10

    def test_x_used_three_times(self):
        """y = x + x + x, dy/dx = 3."""
        x = Tensor.from_list([2.0], requires_grad=True)
        y = x + x + x
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 3.0) < 1e-10


# =========================================================================
# MatMul gradient
# =========================================================================


class TestMatMulGradient:
    """Test gradient computation for matrix multiplication."""

    def test_matmul_grad_a(self):
        """c = a @ b, dL/da = grad @ b.T."""
        a = Tensor.from_list(
            [[1.0, 2.0], [3.0, 4.0]], requires_grad=True
        )
        b = Tensor.from_list(
            [[5.0, 6.0], [7.0, 8.0]], requires_grad=True
        )
        c = a @ b
        loss = c.sum()
        loss.backward()
        # dL/dC = ones(2,2), dL/dA = ones(2,2) @ B.T
        # ones(2,2) @ [[5,7],[6,8]] = [[11,15],[11,15]]
        assert a.grad is not None
        expected_a = [5.0 + 6.0, 7.0 + 8.0, 5.0 + 6.0, 7.0 + 8.0]
        for g, e in zip(a.grad.data, expected_a, strict=True):
            assert abs(g - e) < 1e-10

    def test_matmul_grad_b(self):
        """c = a @ b, dL/db = a.T @ grad."""
        a = Tensor.from_list(
            [[1.0, 2.0], [3.0, 4.0]], requires_grad=True
        )
        b = Tensor.from_list(
            [[5.0, 6.0], [7.0, 8.0]], requires_grad=True
        )
        c = a @ b
        loss = c.sum()
        loss.backward()
        # dL/dB = A.T @ ones(2,2)
        # A.T = [[1,3],[2,4]], A.T @ ones = [[4,4],[6,6]]
        assert b.grad is not None
        expected_b = [4.0, 4.0, 6.0, 6.0]
        for g, e in zip(b.grad.data, expected_b, strict=True):
            assert abs(g - e) < 1e-10

    def test_matmul_non_square(self):
        """Test gradient for non-square matmul: (2,3) @ (3,2)."""
        a = Tensor.from_list(
            [[1, 2, 3], [4, 5, 6]], requires_grad=True
        )
        b = Tensor.from_list(
            [[1, 0], [0, 1], [1, 1]], requires_grad=True
        )
        c = a @ b
        assert c.shape == (2, 2)
        loss = c.sum()
        loss.backward()
        assert a.grad is not None
        assert a.grad.shape == (2, 3)
        assert b.grad is not None
        assert b.grad.shape == (3, 2)


# =========================================================================
# no_grad context manager
# =========================================================================


class TestNoGrad:
    def test_no_grad_disables_tracking(self):
        with no_grad():
            assert is_grad_enabled() is False
        assert is_grad_enabled() is True

    def test_no_grad_operations_still_work(self):
        x = Tensor.from_list([1.0, 2.0], requires_grad=True)
        with no_grad():
            y = x + 1.0
        # Operations still compute, but no_grad state is tracked
        assert y.data == [2.0, 3.0]

    def test_is_grad_enabled_default(self):
        assert is_grad_enabled() is True

    def test_no_grad_restores_after_exception(self):
        try:
            with no_grad():
                raise ValueError("test")
        except ValueError:
            pass
        assert is_grad_enabled() is True


# =========================================================================
# Error conditions
# =========================================================================


class TestBackwardErrors:
    def test_backward_non_scalar_no_grad_arg(self):
        """backward() on non-scalar without gradient should raise."""
        x = Tensor.from_list([1.0, 2.0, 3.0], requires_grad=True)
        y = x * 2.0
        with pytest.raises(RuntimeError, match="non-scalar"):
            y.backward()

    def test_backward_non_grad_tensor(self):
        """backward() on tensor without requires_grad should raise."""
        x = Tensor.from_list([1.0, 2.0])
        with pytest.raises(
            RuntimeError, match="doesn't require grad"
        ):
            x.backward()

    def test_backward_with_explicit_gradient(self):
        """backward() with explicit gradient on non-scalar works."""
        x = Tensor.from_list([1.0, 2.0, 3.0], requires_grad=True)
        y = x * 2.0
        grad = Tensor.ones(3)
        y.backward(grad)
        for g in x.grad.data:
            assert abs(g - 2.0) < 1e-10


# =========================================================================
# Gradient accumulation
# =========================================================================


class TestGradientAccumulation:
    def test_backward_twice_accumulates(self):
        """Calling backward twice should accumulate gradients."""
        x = Tensor.from_list([1.0], requires_grad=True)
        y = (x * 3.0).sum()
        y.backward()
        assert abs(x.grad.data[0] - 3.0) < 1e-10

        # Create a new computation and backward again
        y2 = (x * 5.0).sum()
        y2.backward()
        # Gradient should accumulate: 3 + 5 = 8
        assert abs(x.grad.data[0] - 8.0) < 1e-10

    def test_zero_grad_pattern(self):
        """Show how to zero gradients manually."""
        x = Tensor.from_list([1.0], requires_grad=True)
        y = (x * 3.0).sum()
        y.backward()
        assert abs(x.grad.data[0] - 3.0) < 1e-10

        # Zero the gradient manually
        x.grad = None
        y2 = (x * 5.0).sum()
        y2.backward()
        assert abs(x.grad.data[0] - 5.0) < 1e-10


# =========================================================================
# Function base class
# =========================================================================


class TestFunctionBase:
    def test_function_repr(self):
        f = Function()
        assert "<Function>" in repr(f)

    def test_function_forward_not_implemented(self):
        f = Function()
        with pytest.raises(NotImplementedError):
            f.forward()

    def test_function_backward_not_implemented(self):
        f = Function()
        with pytest.raises(NotImplementedError):
            f.backward(Tensor([1.0], (1,)))

    def test_save_for_backward(self):
        f = Function()
        t = Tensor([1.0], (1,))
        f.save_for_backward(t)
        assert len(f.saved_tensors) == 1
        assert f.saved_tensors[0] is t

    def test_apply_sets_grad_fn(self):
        """When any input requires_grad, result has grad_fn."""
        x = Tensor.from_list([1.0], requires_grad=True)
        y = x + 1.0
        assert y._grad_fn is not None

    def test_apply_no_grad_fn_when_no_grad(self):
        """When no input requires_grad, result has no grad_fn."""
        x = Tensor.from_list([1.0])
        y = x + 1.0
        assert y._grad_fn is None
