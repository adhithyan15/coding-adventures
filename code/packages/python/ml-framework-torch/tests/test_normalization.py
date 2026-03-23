"""Tests for normalization layers (BatchNorm1d, LayerNorm)."""

from ml_framework_core import Tensor

from ml_framework_torch.nn.normalization import BatchNorm1d, LayerNorm


class TestBatchNorm1d:
    def test_output_shape(self) -> None:
        bn = BatchNorm1d(4)
        x = Tensor.randn(3, 4)
        y = bn(x)
        assert y.shape == (3, 4)

    def test_normalized_mean_near_zero(self) -> None:
        """After batch norm, each feature should have near-zero mean."""
        bn = BatchNorm1d(4)
        x = Tensor.from_list(
            [
                [10.0, 20.0, 30.0, 40.0],
                [12.0, 22.0, 32.0, 42.0],
                [8.0, 18.0, 28.0, 38.0],
            ]
        )
        y = bn(x)
        # Mean of each feature across batch should be ≈ 0
        for j in range(4):
            col = [y.data[i * 4 + j] for i in range(3)]
            mean = sum(col) / 3
            assert abs(mean) < 1e-4

    def test_running_stats_updated(self) -> None:
        bn = BatchNorm1d(2)
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        bn(x)  # forward updates running stats
        # Running mean should no longer be all zeros
        assert bn.running_mean.data[0] != 0.0

    def test_eval_mode_uses_running_stats(self) -> None:
        bn = BatchNorm1d(2)
        # Train to update running stats
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        bn(x)

        # Switch to eval
        bn.eval()
        y1 = bn(x)
        y2 = bn(x)
        # Results should be identical in eval mode
        for a, b in zip(y1.data, y2.data):
            assert abs(a - b) < 1e-6

    def test_invalid_ndim(self) -> None:
        bn = BatchNorm1d(4)
        try:
            bn(Tensor.randn(4))  # 1-D, not 2-D
            assert False, "Should raise"
        except ValueError:
            pass

    def test_wrong_features(self) -> None:
        bn = BatchNorm1d(4)
        try:
            bn(Tensor.randn(3, 5))  # 5 features, expected 4
            assert False, "Should raise"
        except ValueError:
            pass

    def test_parameters(self) -> None:
        bn = BatchNorm1d(4)
        params = list(bn.parameters())
        assert len(params) == 2  # weight and bias

    def test_repr(self) -> None:
        r = repr(BatchNorm1d(64))
        assert "BatchNorm1d" in r
        assert "64" in r


class TestLayerNorm:
    def test_output_shape(self) -> None:
        ln = LayerNorm(4)
        x = Tensor.randn(3, 4)
        y = ln(x)
        assert y.shape == (3, 4)

    def test_normalized_per_sample(self) -> None:
        """Each sample should have near-zero mean after LayerNorm."""
        ln = LayerNorm(4)
        x = Tensor.from_list(
            [
                [10.0, 20.0, 30.0, 40.0],
                [1.0, 2.0, 3.0, 4.0],
            ]
        )
        y = ln(x)
        # Each row should have near-zero mean
        for i in range(2):
            row = [y.data[i * 4 + j] for j in range(4)]
            mean = sum(row) / 4
            assert abs(mean) < 1e-4

    def test_same_in_train_and_eval(self) -> None:
        """LayerNorm should behave identically in train and eval."""
        ln = LayerNorm(3)
        x = Tensor.from_list([[1.0, 2.0, 3.0]])

        ln.train()
        y_train = ln(x)

        ln.eval()
        y_eval = ln(x)

        for a, b in zip(y_train.data, y_eval.data):
            assert abs(a - b) < 1e-6

    def test_invalid_ndim(self) -> None:
        ln = LayerNorm(4)
        try:
            ln(Tensor.randn(4))
            assert False, "Should raise"
        except ValueError:
            pass

    def test_wrong_features(self) -> None:
        ln = LayerNorm(4)
        try:
            ln(Tensor.randn(3, 5))
            assert False, "Should raise"
        except ValueError:
            pass

    def test_parameters(self) -> None:
        ln = LayerNorm(4)
        params = list(ln.parameters())
        assert len(params) == 2  # weight and bias

    def test_repr(self) -> None:
        r = repr(LayerNorm(512))
        assert "LayerNorm" in r
        assert "512" in r
