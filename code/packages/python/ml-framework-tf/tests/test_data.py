"""Tests for tf.data.Dataset — data pipeline utilities."""

import pytest
from ml_framework_core import Tensor
from ml_framework_tf.data import Dataset


class TestFromTensorSlices:
    def test_single_tensor(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
        ds = Dataset.from_tensor_slices(x)
        assert len(ds) == 3

    def test_single_tensor_iteration(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        ds = Dataset.from_tensor_slices(x)
        elements = list(ds)
        assert len(elements) == 2
        assert elements[0].data == [1.0, 2.0]
        assert elements[1].data == [3.0, 4.0]

    def test_tuple_of_tensors(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        y = Tensor.from_list([0.0, 1.0])
        ds = Dataset.from_tensor_slices((x, y))
        elements = list(ds)
        assert len(elements) == 2
        xi, yi = elements[0]
        assert xi.data == [1.0, 2.0]
        assert yi.data == [0.0]

    def test_mismatched_first_dim(self):
        x = Tensor.from_list([[1.0], [2.0]])
        y = Tensor.from_list([1.0, 2.0, 3.0])
        with pytest.raises(ValueError, match="same first dimension"):
            Dataset.from_tensor_slices((x, y))

    def test_1d_tensor(self):
        x = Tensor.from_list([10.0, 20.0, 30.0])
        ds = Dataset.from_tensor_slices(x)
        elements = list(ds)
        assert len(elements) == 3
        assert elements[0].data == [10.0]

    def test_invalid_type(self):
        with pytest.raises(TypeError):
            Dataset.from_tensor_slices("not a tensor")


class TestBatch:
    def test_even_batching(self):
        x = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]])
        ds = Dataset.from_tensor_slices(x).batch(2)
        batches = list(ds)
        assert len(batches) == 2
        assert batches[0].shape == (2, 1)
        assert batches[1].shape == (2, 1)

    def test_uneven_batching(self):
        x = Tensor.from_list([[1.0], [2.0], [3.0]])
        ds = Dataset.from_tensor_slices(x).batch(2)
        batches = list(ds)
        assert len(batches) == 2
        assert batches[0].shape == (2, 1)
        assert batches[1].shape == (1, 1)

    def test_tuple_batching(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        y = Tensor.from_list([0.0, 1.0])
        ds = Dataset.from_tensor_slices((x, y)).batch(2)
        batches = list(ds)
        assert len(batches) == 1
        xb, yb = batches[0]
        assert xb.shape == (2, 2)
        assert yb.shape == (2, 1)


class TestShuffle:
    def test_shuffle_preserves_length(self):
        x = Tensor.from_list([1.0, 2.0, 3.0, 4.0, 5.0])
        ds = Dataset.from_tensor_slices(x).shuffle(100)
        assert len(ds) == 5

    def test_shuffle_changes_order(self):
        """Shuffle should (usually) change order with enough elements."""
        x = Tensor.from_list([float(i) for i in range(100)])
        ds = Dataset.from_tensor_slices(x).shuffle(100)
        original = list(range(100))
        shuffled = [elem.data[0] for elem in ds]
        # It's astronomically unlikely that 100 elements stay in order
        assert shuffled != original


class TestStackEmpty:
    def test_stack_empty_raises(self):
        from ml_framework_tf.data import _stack_tensors

        with pytest.raises(ValueError, match="Cannot stack"):
            _stack_tensors([])


class TestChaining:
    def test_shuffle_then_batch(self):
        x = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]])
        ds = Dataset.from_tensor_slices(x).shuffle(10).batch(2)
        batches = list(ds)
        assert len(batches) == 2
        for b in batches:
            assert b.shape == (2, 1)

    def test_full_pipeline(self):
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.0, 0.0]])
        y = Tensor.from_list([1.0, 0.0, 1.0, 0.0])
        ds = Dataset.from_tensor_slices((x, y)).shuffle(4).batch(2)
        batches = list(ds)
        assert len(batches) == 2
        for xb, yb in batches:
            assert xb.shape[0] == 2
            assert yb.shape[0] == 2
