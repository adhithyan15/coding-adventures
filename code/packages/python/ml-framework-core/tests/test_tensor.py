"""Tests for the Tensor class: creation, properties, indexing, display."""

import math

import pytest

from ml_framework_core.tensor import (
    Tensor,
    _compute_strides,
    _flat_index,
    _numel,
)

# =========================================================================
# Helper utilities
# =========================================================================


class TestHelperFunctions:
    """Tests for the private helper functions in tensor.py."""

    def test_numel_scalar(self):
        assert _numel(()) == 1

    def test_numel_1d(self):
        assert _numel((5,)) == 5

    def test_numel_2d(self):
        assert _numel((3, 4)) == 12

    def test_numel_3d(self):
        assert _numel((2, 3, 4)) == 24

    def test_flat_index_1d(self):
        assert _flat_index((2,), (5,)) == 2

    def test_flat_index_2d(self):
        assert _flat_index((1, 2), (3, 4)) == 6

    def test_flat_index_3d(self):
        assert _flat_index((1, 2, 3), (2, 3, 4)) == 23

    def test_compute_strides_empty(self):
        assert _compute_strides(()) == ()

    def test_compute_strides_1d(self):
        assert _compute_strides((5,)) == (1,)

    def test_compute_strides_2d(self):
        assert _compute_strides((3, 4)) == (4, 1)

    def test_compute_strides_3d(self):
        assert _compute_strides((2, 3, 4)) == (12, 4, 1)


# =========================================================================
# Factory methods
# =========================================================================


class TestTensorZeros:
    def test_zeros_1d(self):
        t = Tensor.zeros(5)
        assert t.shape == (5,)
        assert t.data == [0.0] * 5

    def test_zeros_2d(self):
        t = Tensor.zeros(2, 3)
        assert t.shape == (2, 3)
        assert t.data == [0.0] * 6

    def test_zeros_requires_grad(self):
        t = Tensor.zeros(3, requires_grad=True)
        assert t.requires_grad is True

    def test_zeros_device(self):
        t = Tensor.zeros(3, device="cuda")
        assert t.device == "cuda"


class TestTensorOnes:
    def test_ones_1d(self):
        t = Tensor.ones(4)
        assert t.data == [1.0] * 4

    def test_ones_2d(self):
        t = Tensor.ones(2, 3)
        assert t.shape == (2, 3)
        assert all(x == 1.0 for x in t.data)

    def test_ones_requires_grad(self):
        t = Tensor.ones(3, requires_grad=True)
        assert t.requires_grad is True


class TestTensorFull:
    def test_full_value(self):
        t = Tensor.full((3, 2), 7.0)
        assert t.shape == (3, 2)
        assert all(x == 7.0 for x in t.data)

    def test_full_negative(self):
        t = Tensor.full((2,), -3.5)
        assert t.data == [-3.5, -3.5]

    def test_full_requires_grad(self):
        t = Tensor.full((2, 2), 1.0, requires_grad=True)
        assert t.requires_grad is True


class TestTensorRandn:
    def test_randn_shape(self):
        t = Tensor.randn(3, 4)
        assert t.shape == (3, 4)
        assert len(t.data) == 12

    def test_randn_different_values(self):
        t = Tensor.randn(100)
        # Very unlikely all values are the same
        assert len(set(t.data)) > 1

    def test_randn_roughly_normal(self):
        t = Tensor.randn(1000)
        mean = sum(t.data) / len(t.data)
        # Should be roughly 0 (within 0.5 for 1000 samples)
        assert abs(mean) < 0.5


class TestTensorEye:
    def test_eye_2(self):
        t = Tensor.eye(2)
        assert t.shape == (2, 2)
        assert t.data == [1.0, 0.0, 0.0, 1.0]

    def test_eye_3(self):
        t = Tensor.eye(3)
        assert t.shape == (3, 3)
        assert t.data == [1, 0, 0, 0, 1, 0, 0, 0, 1]

    def test_eye_1(self):
        t = Tensor.eye(1)
        assert t.data == [1.0]

    def test_eye_requires_grad(self):
        t = Tensor.eye(2, requires_grad=True)
        assert t.requires_grad is True


class TestTensorArange:
    def test_arange_basic(self):
        t = Tensor.arange(0, 5)
        assert t.shape == (5,)
        assert t.data == [0.0, 1.0, 2.0, 3.0, 4.0]

    def test_arange_with_step(self):
        t = Tensor.arange(0, 10, 2.0)
        assert t.data == [0.0, 2.0, 4.0, 6.0, 8.0]

    def test_arange_float(self):
        t = Tensor.arange(0.5, 2.5, 0.5)
        assert len(t.data) == 4
        assert abs(t.data[0] - 0.5) < 1e-10

    def test_arange_empty(self):
        t = Tensor.arange(5, 0)
        assert t.shape == (0,)
        assert t.data == []


class TestTensorFromList:
    def test_from_list_1d(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        assert t.shape == (3,)
        assert t.data == [1.0, 2.0, 3.0]

    def test_from_list_2d(self):
        t = Tensor.from_list([[1, 2], [3, 4]])
        assert t.shape == (2, 2)
        assert t.data == [1.0, 2.0, 3.0, 4.0]

    def test_from_list_3d(self):
        t = Tensor.from_list([[[1, 2], [3, 4]], [[5, 6], [7, 8]]])
        assert t.shape == (2, 2, 2)
        assert t.data == [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]

    def test_from_list_with_shape(self):
        t = Tensor.from_list([1, 2, 3, 4], shape=(2, 2))
        assert t.shape == (2, 2)

    def test_from_list_requires_grad(self):
        t = Tensor.from_list([1.0], requires_grad=True)
        assert t.requires_grad is True

    def test_from_list_scalar(self):
        t = Tensor.from_list(5.0)
        assert t.data == [5.0]
        assert t.shape == (1,)

    def test_from_list_int_values(self):
        t = Tensor.from_list([1, 2, 3])
        assert all(isinstance(x, float) for x in t.data)

    def test_from_list_inconsistent_shape_raises(self):
        with pytest.raises(ValueError, match="Inconsistent"):
            Tensor.from_list([[1, 2], [3]])


# =========================================================================
# Constructor validation
# =========================================================================


class TestTensorConstructor:
    def test_shape_mismatch_raises(self):
        with pytest.raises(ValueError, match="doesn't match shape"):
            Tensor([1.0, 2.0], (3,))

    def test_default_grad_none(self):
        t = Tensor([1.0], (1,))
        assert t.grad is None

    def test_default_grad_fn_none(self):
        t = Tensor([1.0], (1,))
        assert t._grad_fn is None

    def test_default_device_cpu(self):
        t = Tensor([1.0], (1,))
        assert t.device == "cpu"


# =========================================================================
# Properties
# =========================================================================


class TestTensorProperties:
    def test_ndim_0d(self):
        # A "scalar" tensor has shape (1,) in this framework
        t = Tensor([1.0], (1,))
        assert t.ndim == 1

    def test_ndim_1d(self):
        t = Tensor.zeros(5)
        assert t.ndim == 1

    def test_ndim_2d(self):
        t = Tensor.zeros(3, 4)
        assert t.ndim == 2

    def test_ndim_3d(self):
        t = Tensor.zeros(2, 3, 4)
        assert t.ndim == 3

    def test_numel_property(self):
        t = Tensor.zeros(3, 4)
        assert t.numel == 12

    def test_is_leaf_true(self):
        t = Tensor.ones(3, requires_grad=True)
        assert t.is_leaf is True

    def test_is_leaf_false_after_op(self):
        x = Tensor.from_list([1.0, 2.0], requires_grad=True)
        y = x + 1
        assert y.is_leaf is False

    def test_grad_fn_none_for_leaf(self):
        t = Tensor.ones(3)
        assert t.grad_fn is None

    def test_grad_fn_exists_after_op(self):
        x = Tensor.from_list([1.0], requires_grad=True)
        y = x + 1
        assert y.grad_fn is not None

    def test_device_property(self):
        t = Tensor([1.0], (1,), device="metal")
        assert t.device == "metal"


# =========================================================================
# Indexing (__getitem__)
# =========================================================================


class TestTensorIndexing:
    def test_getitem_1d(self):
        t = Tensor.from_list([10.0, 20.0, 30.0])
        result = t[0]
        assert result.data == [10.0]

    def test_getitem_1d_last(self):
        t = Tensor.from_list([10.0, 20.0, 30.0])
        result = t[2]
        assert result.data == [30.0]

    def test_getitem_1d_negative(self):
        t = Tensor.from_list([10.0, 20.0, 30.0])
        result = t[-1]
        assert result.data == [30.0]

    def test_getitem_2d(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        row = t[0]
        assert row.shape == (3,)
        assert row.data == [1.0, 2.0, 3.0]

    def test_getitem_2d_second_row(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        row = t[1]
        assert row.data == [4.0, 5.0, 6.0]

    def test_getitem_3d(self):
        t = Tensor.from_list(
            [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
        )
        page = t[1]
        assert page.shape == (2, 2)
        assert page.data == [5.0, 6.0, 7.0, 8.0]

    def test_getitem_out_of_range(self):
        t = Tensor.from_list([1.0, 2.0])
        with pytest.raises(IndexError):
            t[5]

    def test_getitem_non_int_raises(self):
        t = Tensor.from_list([1.0, 2.0])
        with pytest.raises(TypeError):
            t[0.5]

    def test_getitem_negative_out_of_range(self):
        t = Tensor.from_list([1.0, 2.0])
        with pytest.raises(IndexError):
            t[-3]


# =========================================================================
# repr
# =========================================================================


class TestTensorRepr:
    def test_repr_small(self):
        t = Tensor.from_list([1.0, 2.0])
        r = repr(t)
        assert "Tensor(" in r
        assert "shape=(2,)" in r

    def test_repr_requires_grad(self):
        t = Tensor.from_list([1.0], requires_grad=True)
        assert "requires_grad=True" in repr(t)

    def test_repr_grad_fn(self):
        x = Tensor.from_list([1.0], requires_grad=True)
        y = x + 1
        r = repr(y)
        assert "grad_fn=" in r

    def test_repr_large_tensor(self):
        t = Tensor.zeros(20)
        r = repr(t)
        assert "..." in r

    def test_repr_no_grad_fn_for_leaf(self):
        t = Tensor.from_list([1.0])
        r = repr(t)
        assert "grad_fn=" not in r


# =========================================================================
# item()
# =========================================================================


class TestTensorItem:
    def test_item_scalar(self):
        t = Tensor([3.14], (1,))
        assert t.item() == 3.14

    def test_item_non_scalar_raises(self):
        t = Tensor.zeros(3)
        with pytest.raises(ValueError, match="single-element"):
            t.item()


# =========================================================================
# to() -- device transfer
# =========================================================================


class TestTensorTo:
    def test_to_same_device(self):
        t = Tensor.from_list([1.0, 2.0])
        t2 = t.to("cpu")
        assert t2 is t  # Same object returned

    def test_to_different_device(self):
        t = Tensor.from_list([1.0, 2.0])
        t2 = t.to("cuda")
        assert t2.device == "cuda"
        assert t2.data == [1.0, 2.0]

    def test_to_preserves_shape(self):
        t = Tensor.zeros(2, 3)
        t2 = t.to("metal")
        assert t2.shape == (2, 3)

    def test_to_preserves_requires_grad(self):
        t = Tensor.ones(3, requires_grad=True)
        t2 = t.to("cuda")
        assert t2.requires_grad is True


# =========================================================================
# detach()
# =========================================================================


class TestTensorDetach:
    def test_detach_basic(self):
        t = Tensor.from_list([1.0, 2.0], requires_grad=True)
        d = t.detach()
        assert d.requires_grad is False

    def test_detach_data_copied(self):
        t = Tensor.from_list([1.0, 2.0])
        d = t.detach()
        d.data[0] = 99.0
        assert t.data[0] == 1.0  # Original unchanged

    def test_detach_preserves_shape(self):
        t = Tensor.zeros(2, 3, requires_grad=True)
        d = t.detach()
        assert d.shape == (2, 3)


# =========================================================================
# Comparison ops
# =========================================================================


class TestTensorComparisons:
    def test_eq_scalar(self):
        t = Tensor.from_list([1.0, 2.0, 1.0])
        result = t.eq(1.0)
        assert result.data == [1.0, 0.0, 1.0]

    def test_eq_tensor(self):
        a = Tensor.from_list([1.0, 2.0, 3.0])
        b = Tensor.from_list([1.0, 0.0, 3.0])
        result = a.eq(b)
        assert result.data == [1.0, 0.0, 1.0]

    def test_gt_scalar(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        result = t.gt(2.0)
        assert result.data == [0.0, 0.0, 1.0]

    def test_gt_tensor(self):
        a = Tensor.from_list([3.0, 1.0, 2.0])
        b = Tensor.from_list([1.0, 2.0, 2.0])
        result = a.gt(b)
        assert result.data == [1.0, 0.0, 0.0]

    def test_lt_scalar(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        result = t.lt(2.0)
        assert result.data == [1.0, 0.0, 0.0]

    def test_lt_tensor(self):
        a = Tensor.from_list([1.0, 3.0, 2.0])
        b = Tensor.from_list([2.0, 2.0, 2.0])
        result = a.lt(b)
        assert result.data == [1.0, 0.0, 0.0]

    def test_comparison_no_grad(self):
        t = Tensor.from_list([1.0], requires_grad=True)
        result = t.eq(1.0)
        assert result.requires_grad is False


# =========================================================================
# __len__
# =========================================================================


class TestTensorLen:
    def test_len_1d(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        assert len(t) == 3

    def test_len_2d(self):
        t = Tensor.zeros(4, 5)
        assert len(t) == 4


# =========================================================================
# Shape operations (basic)
# =========================================================================


class TestTensorShapeOps:
    def test_flatten(self):
        t = Tensor.zeros(2, 3, 4)
        f = t.flatten()
        assert f.shape == (24,)

    def test_flatten_partial(self):
        t = Tensor.zeros(2, 3, 4)
        f = t.flatten(1, 2)
        assert f.shape == (2, 12)

    def test_unsqueeze_dim0(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        u = t.unsqueeze(0)
        assert u.shape == (1, 3)

    def test_unsqueeze_dim1(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        u = t.unsqueeze(1)
        assert u.shape == (3, 1)

    def test_unsqueeze_negative(self):
        t = Tensor.from_list([1.0, 2.0])
        u = t.unsqueeze(-1)
        assert u.shape == (2, 1)

    def test_squeeze_all(self):
        t = Tensor([1.0], (1, 1, 1))
        s = t.squeeze()
        assert s.shape == (1,)

    def test_squeeze_dim(self):
        t = Tensor([1.0, 2.0], (1, 2))
        s = t.squeeze(0)
        assert s.shape == (2,)

    def test_squeeze_non_one_dim(self):
        t = Tensor.zeros(2, 3)
        s = t.squeeze(0)
        assert s.shape == (2, 3)

    def test_t_2d(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        tr = t.t()
        assert tr.shape == (3, 2)
        assert tr.data[0] == 1.0
        assert tr.data[1] == 4.0

    def test_t_non_2d_raises(self):
        t = Tensor.zeros(2, 3, 4)
        with pytest.raises(ValueError, match="2-D"):
            t.t()

    def test_reshape(self):
        t = Tensor.from_list([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
        r = t.reshape(2, 3)
        assert r.shape == (2, 3)
        assert r.data == [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]

    def test_transpose_2d(self):
        t = Tensor.from_list([[1, 2], [3, 4], [5, 6]])
        tr = t.transpose(0, 1)
        assert tr.shape == (2, 3)
        expected = [1.0, 3.0, 5.0, 2.0, 4.0, 6.0]
        assert tr.data == expected


# =========================================================================
# Arithmetic operator wiring (basic correctness)
# =========================================================================


class TestTensorArithmetic:
    def test_add_tensors(self):
        a = Tensor.from_list([1.0, 2.0])
        b = Tensor.from_list([3.0, 4.0])
        c = a + b
        assert c.data == [4.0, 6.0]

    def test_add_scalar(self):
        a = Tensor.from_list([1.0, 2.0])
        c = a + 10.0
        assert c.data == [11.0, 12.0]

    def test_radd_scalar(self):
        a = Tensor.from_list([1.0, 2.0])
        c = 10.0 + a
        assert c.data == [11.0, 12.0]

    def test_sub_tensors(self):
        a = Tensor.from_list([5.0, 3.0])
        b = Tensor.from_list([1.0, 2.0])
        c = a - b
        assert c.data == [4.0, 1.0]

    def test_sub_scalar(self):
        a = Tensor.from_list([5.0, 3.0])
        c = a - 1.0
        assert c.data == [4.0, 2.0]

    def test_rsub_scalar(self):
        a = Tensor.from_list([1.0, 2.0])
        c = 10.0 - a
        assert c.data == [9.0, 8.0]

    def test_mul_tensors(self):
        a = Tensor.from_list([2.0, 3.0])
        b = Tensor.from_list([4.0, 5.0])
        c = a * b
        assert c.data == [8.0, 15.0]

    def test_mul_scalar(self):
        a = Tensor.from_list([2.0, 3.0])
        c = a * 3.0
        assert c.data == [6.0, 9.0]

    def test_rmul_scalar(self):
        a = Tensor.from_list([2.0, 3.0])
        c = 3.0 * a
        assert c.data == [6.0, 9.0]

    def test_div_tensors(self):
        a = Tensor.from_list([6.0, 8.0])
        b = Tensor.from_list([2.0, 4.0])
        c = a / b
        assert c.data == [3.0, 2.0]

    def test_div_scalar(self):
        a = Tensor.from_list([6.0, 9.0])
        c = a / 3.0
        assert c.data == [2.0, 3.0]

    def test_neg(self):
        a = Tensor.from_list([1.0, -2.0, 3.0])
        c = -a
        assert c.data == [-1.0, 2.0, -3.0]

    def test_pow(self):
        a = Tensor.from_list([2.0, 3.0])
        c = a**2.0
        assert c.data == [4.0, 9.0]

    def test_matmul(self):
        a = Tensor.from_list([[1, 2], [3, 4]])
        b = Tensor.from_list([[5, 6], [7, 8]])
        c = a @ b
        assert c.shape == (2, 2)
        assert c.data == [19.0, 22.0, 43.0, 50.0]

    def test_sqrt(self):
        a = Tensor.from_list([4.0, 9.0, 16.0])
        c = a.sqrt()
        assert abs(c.data[0] - 2.0) < 1e-10
        assert abs(c.data[1] - 3.0) < 1e-10
        assert abs(c.data[2] - 4.0) < 1e-10


# =========================================================================
# Reduction operations (basic)
# =========================================================================


class TestTensorReductions:
    def test_sum_all(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        s = t.sum()
        assert s.data == [6.0]

    def test_sum_dim(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        s = t.sum(dim=0)
        assert s.data == [5.0, 7.0, 9.0]

    def test_sum_dim1(self):
        t = Tensor.from_list([[1, 2, 3], [4, 5, 6]])
        s = t.sum(dim=1)
        assert s.data == [6.0, 15.0]

    def test_mean_all(self):
        t = Tensor.from_list([2.0, 4.0, 6.0])
        m = t.mean()
        assert abs(m.data[0] - 4.0) < 1e-10

    def test_mean_dim(self):
        t = Tensor.from_list([[1, 3], [5, 7]])
        m = t.mean(dim=0)
        assert abs(m.data[0] - 3.0) < 1e-10
        assert abs(m.data[1] - 5.0) < 1e-10


# =========================================================================
# Element-wise math (basic)
# =========================================================================


class TestTensorMath:
    def test_exp(self):
        t = Tensor.from_list([0.0, 1.0])
        r = t.exp()
        assert abs(r.data[0] - 1.0) < 1e-10
        assert abs(r.data[1] - math.e) < 1e-10

    def test_log(self):
        t = Tensor.from_list([1.0, math.e])
        r = t.log()
        assert abs(r.data[0] - 0.0) < 1e-10
        assert abs(r.data[1] - 1.0) < 1e-10

    def test_abs(self):
        t = Tensor.from_list([-1.0, 0.0, 2.0])
        r = t.abs()
        assert r.data == [1.0, 0.0, 2.0]

    def test_clamp_both(self):
        t = Tensor.from_list([-5.0, 0.0, 10.0])
        r = t.clamp(min_val=-1.0, max_val=5.0)
        assert r.data == [-1.0, 0.0, 5.0]

    def test_clamp_min_only(self):
        t = Tensor.from_list([-5.0, 0.0, 10.0])
        r = t.clamp(min_val=0.0)
        assert r.data == [0.0, 0.0, 10.0]

    def test_clamp_max_only(self):
        t = Tensor.from_list([-5.0, 0.0, 10.0])
        r = t.clamp(max_val=5.0)
        assert r.data == [-5.0, 0.0, 5.0]
