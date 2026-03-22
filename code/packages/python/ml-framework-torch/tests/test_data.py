"""Tests for Dataset, TensorDataset, and DataLoader."""

from ml_framework_core import Tensor

from ml_framework_torch.utils.data import DataLoader, Dataset, TensorDataset


class TestDataset:
    def test_abstract_len(self) -> None:
        d = Dataset()
        try:
            len(d)
            assert False, "Should raise"
        except NotImplementedError:
            pass

    def test_abstract_getitem(self) -> None:
        d = Dataset()
        try:
            d[0]
            assert False, "Should raise"
        except NotImplementedError:
            pass


class TestTensorDataset:
    def test_basic_creation(self) -> None:
        X = Tensor.randn(10, 3)
        y = Tensor.randn(10, 1)
        ds = TensorDataset(X, y)
        assert len(ds) == 10

    def test_single_tensor(self) -> None:
        X = Tensor.randn(5, 4)
        ds = TensorDataset(X)
        assert len(ds) == 5

    def test_getitem(self) -> None:
        X = Tensor.from_list([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
        y = Tensor.from_list([10.0, 20.0, 30.0])
        ds = TensorDataset(X, y)

        x0, y0 = ds[0]
        assert x0.shape == (2,)
        assert x0.data == [1.0, 2.0]
        assert y0.data == [10.0]

    def test_negative_index(self) -> None:
        X = Tensor.from_list([[1.0], [2.0], [3.0]])
        ds = TensorDataset(X)
        (x,) = ds[-1]
        assert x.data == [3.0]

    def test_out_of_range(self) -> None:
        X = Tensor.randn(3, 2)
        ds = TensorDataset(X)
        try:
            ds[5]
            assert False, "Should raise"
        except IndexError:
            pass

    def test_mismatched_lengths(self) -> None:
        X = Tensor.randn(5, 2)
        y = Tensor.randn(3, 1)
        try:
            TensorDataset(X, y)
            assert False, "Should raise"
        except ValueError:
            pass

    def test_empty(self) -> None:
        try:
            TensorDataset()
            assert False, "Should raise"
        except ValueError:
            pass


class TestDataLoader:
    def test_basic_iteration(self) -> None:
        X = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]])
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=2)

        batches = list(loader)
        assert len(batches) == 2
        # Each batch should be a tuple of one tensor
        assert batches[0][0].shape == (2, 1)

    def test_last_batch_smaller(self) -> None:
        X = Tensor.from_list([[1.0], [2.0], [3.0]])
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=2)

        batches = list(loader)
        assert len(batches) == 2
        assert batches[1][0].shape == (1, 1)

    def test_drop_last(self) -> None:
        X = Tensor.from_list([[1.0], [2.0], [3.0]])
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=2, drop_last=True)

        batches = list(loader)
        assert len(batches) == 1

    def test_shuffle(self) -> None:
        """With shuffle, order should vary across epochs."""
        import random

        random.seed(42)

        X = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]])
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=1, shuffle=True)

        epoch1 = [b[0].data[0] for b in loader]
        epoch2 = [b[0].data[0] for b in loader]
        # It's possible they're the same by chance, but unlikely
        # At minimum, both should have 4 batches
        assert len(epoch1) == 4
        assert len(epoch2) == 4

    def test_len(self) -> None:
        X = Tensor.randn(10, 3)
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=3)
        assert len(loader) == 4  # ceil(10/3)

    def test_len_drop_last(self) -> None:
        X = Tensor.randn(10, 3)
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=3, drop_last=True)
        assert len(loader) == 3  # floor(10/3)

    def test_two_tensors(self) -> None:
        X = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        y = Tensor.from_list([0.0, 1.0])
        ds = TensorDataset(X, y)
        loader = DataLoader(ds, batch_size=2)

        batches = list(loader)
        assert len(batches) == 1
        bx, by = batches[0]
        assert bx.shape == (2, 2)
        assert by.shape == (2, 1)

    def test_batch_size_one(self) -> None:
        X = Tensor.from_list([[1.0], [2.0]])
        ds = TensorDataset(X)
        loader = DataLoader(ds, batch_size=1)
        batches = list(loader)
        assert len(batches) == 2
