"""Tests for Parameter: a Tensor that always requires gradient."""

from ml_framework_core.parameter import Parameter
from ml_framework_core.tensor import Tensor


class TestParameterCreation:
    def test_from_tensor(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        p = Parameter(t)
        assert p.data == [1.0, 2.0, 3.0]
        assert p.shape == (3,)

    def test_from_2d_tensor(self):
        t = Tensor.from_list([[1, 2], [3, 4]])
        p = Parameter(t)
        assert p.shape == (2, 2)
        assert p.data == [1.0, 2.0, 3.0, 4.0]

    def test_default_parameter(self):
        p = Parameter()
        assert p.shape == (1,)
        assert p.data == [0.0]

    def test_from_zeros(self):
        t = Tensor.zeros(3, 4)
        p = Parameter(t)
        assert p.shape == (3, 4)
        assert all(x == 0.0 for x in p.data)

    def test_from_randn(self):
        t = Tensor.randn(5, 5)
        p = Parameter(t)
        assert p.shape == (5, 5)
        assert len(p.data) == 25


class TestParameterRequiresGrad:
    def test_requires_grad_always_true(self):
        t = Tensor.from_list([1.0, 2.0])
        p = Parameter(t)
        assert p.requires_grad is True

    def test_requires_grad_default(self):
        p = Parameter()
        assert p.requires_grad is True

    def test_requires_grad_explicit_true(self):
        t = Tensor.from_list([1.0])
        p = Parameter(t, requires_grad=True)
        assert p.requires_grad is True

    def test_requires_grad_explicit_false(self):
        """Force requires_grad=False (e.g., frozen layers)."""
        t = Tensor.from_list([1.0])
        p = Parameter(t, requires_grad=False)
        assert p.requires_grad is False


class TestParameterRepr:
    def test_repr_small(self):
        t = Tensor.from_list([1.0, 2.0])
        p = Parameter(t)
        r = repr(p)
        assert "Parameter(" in r
        assert "shape=(2,)" in r

    def test_repr_large(self):
        t = Tensor.zeros(10)
        p = Parameter(t)
        r = repr(p)
        assert "..." in r

    def test_repr_contains_data(self):
        t = Tensor.from_list([3.14])
        p = Parameter(t)
        r = repr(p)
        assert "3.14" in r


class TestParameterGradient:
    def test_gradient_flows(self):
        """Parameters accumulate gradients through computation."""
        p = Parameter(Tensor.from_list([2.0, 3.0]))
        y = (p * 4.0).sum()
        y.backward()
        assert p.grad is not None
        assert p.grad.data == [4.0, 4.0]

    def test_is_leaf(self):
        p = Parameter(Tensor.from_list([1.0]))
        assert p.is_leaf is True

    def test_grad_fn_none(self):
        p = Parameter(Tensor.from_list([1.0]))
        assert p.grad_fn is None


class TestParameterIsSubclass:
    def test_isinstance_tensor(self):
        p = Parameter(Tensor.from_list([1.0]))
        assert isinstance(p, Tensor)

    def test_parameter_in_arithmetic(self):
        p = Parameter(Tensor.from_list([2.0, 3.0]))
        t = Tensor.from_list([1.0, 1.0])
        result = p + t
        assert result.data == [3.0, 4.0]

    def test_parameter_matmul(self):
        w = Parameter(
            Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        )
        x = Tensor.from_list([[2.0, 3.0]])
        result = x @ w
        assert result.data == [2.0, 3.0]

    def test_parameter_device(self):
        t = Tensor.from_list([1.0], device="cpu")
        p = Parameter(t)
        assert p.device == "cpu"
