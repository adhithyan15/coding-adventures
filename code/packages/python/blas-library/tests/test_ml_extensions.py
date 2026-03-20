"""Tests for ML extensions — activations, softmax, normalization, attention."""

from __future__ import annotations

import math

import pytest
from conftest import approx_equal, approx_list

from blas_library import CpuBlas, Matrix, Vector


@pytest.fixture
def blas() -> CpuBlas:
    return CpuBlas()


# =========================================================================
# Activation functions
# =========================================================================


class TestReLU:
    def test_positive_values(self, blas: CpuBlas) -> None:
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.relu(x)
        assert approx_list(result.data, [1.0, 2.0, 3.0, 4.0])

    def test_negative_values(self, blas: CpuBlas) -> None:
        x = Matrix(data=[-1.0, -2.0, -3.0, -4.0], rows=2, cols=2)
        result = blas.relu(x)
        assert approx_list(result.data, [0.0, 0.0, 0.0, 0.0])

    def test_mixed_values(self, blas: CpuBlas) -> None:
        x = Matrix(data=[-1.0, 2.0, -3.0, 4.0], rows=2, cols=2)
        result = blas.relu(x)
        assert approx_list(result.data, [0.0, 2.0, 0.0, 4.0])

    def test_zeros(self, blas: CpuBlas) -> None:
        x = Matrix(data=[0.0, 0.0], rows=1, cols=2)
        result = blas.relu(x)
        assert approx_list(result.data, [0.0, 0.0])

    def test_preserves_shape(self, blas: CpuBlas) -> None:
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        result = blas.relu(x)
        assert result.rows == 2
        assert result.cols == 3


class TestGELU:
    def test_zero(self, blas: CpuBlas) -> None:
        x = Matrix(data=[0.0], rows=1, cols=1)
        result = blas.gelu(x)
        assert approx_equal(result.data[0], 0.0, tol=1e-4)

    def test_positive(self, blas: CpuBlas) -> None:
        """GELU(x) is approximately x for large positive x."""
        x = Matrix(data=[3.0], rows=1, cols=1)
        result = blas.gelu(x)
        assert result.data[0] > 2.9  # Close to x=3

    def test_negative(self, blas: CpuBlas) -> None:
        """GELU(x) is approximately 0 for large negative x."""
        x = Matrix(data=[-3.0], rows=1, cols=1)
        result = blas.gelu(x)
        assert abs(result.data[0]) < 0.01

    def test_shape_preserved(self, blas: CpuBlas) -> None:
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.gelu(x)
        assert result.rows == 2 and result.cols == 2


class TestSigmoid:
    def test_zero(self, blas: CpuBlas) -> None:
        x = Matrix(data=[0.0], rows=1, cols=1)
        result = blas.sigmoid(x)
        assert approx_equal(result.data[0], 0.5)

    def test_large_positive(self, blas: CpuBlas) -> None:
        x = Matrix(data=[100.0], rows=1, cols=1)
        result = blas.sigmoid(x)
        assert approx_equal(result.data[0], 1.0, tol=1e-4)

    def test_large_negative(self, blas: CpuBlas) -> None:
        x = Matrix(data=[-100.0], rows=1, cols=1)
        result = blas.sigmoid(x)
        assert approx_equal(result.data[0], 0.0, tol=1e-4)

    def test_range(self, blas: CpuBlas) -> None:
        """All sigmoid outputs should be in (0, 1)."""
        x = Matrix(data=[-5.0, -1.0, 0.0, 1.0, 5.0], rows=1, cols=5)
        result = blas.sigmoid(x)
        for v in result.data:
            assert 0.0 < v < 1.0


class TestTanh:
    def test_zero(self, blas: CpuBlas) -> None:
        x = Matrix(data=[0.0], rows=1, cols=1)
        result = blas.tanh_activation(x)
        assert approx_equal(result.data[0], 0.0)

    def test_range(self, blas: CpuBlas) -> None:
        """All tanh outputs should be in (-1, 1)."""
        x = Matrix(data=[-5.0, -1.0, 0.0, 1.0, 5.0], rows=1, cols=5)
        result = blas.tanh_activation(x)
        for v in result.data:
            assert -1.0 < v < 1.0

    def test_known_value(self, blas: CpuBlas) -> None:
        x = Matrix(data=[1.0], rows=1, cols=1)
        result = blas.tanh_activation(x)
        assert approx_equal(result.data[0], math.tanh(1.0))


# =========================================================================
# Softmax
# =========================================================================


class TestSoftmax:
    def test_basic(self, blas: CpuBlas) -> None:
        """Softmax of [1, 2, 3] row."""
        x = Matrix(data=[1.0, 2.0, 3.0], rows=1, cols=3)
        result = blas.softmax(x, axis=-1)
        # Should sum to 1.0
        total = sum(result.data)
        assert approx_equal(total, 1.0)
        # Each value should be positive
        for v in result.data:
            assert v > 0.0
        # Larger input -> larger output
        assert result.data[2] > result.data[1] > result.data[0]

    def test_uniform(self, blas: CpuBlas) -> None:
        """Softmax of equal values should be uniform."""
        x = Matrix(data=[1.0, 1.0, 1.0], rows=1, cols=3)
        result = blas.softmax(x, axis=-1)
        for v in result.data:
            assert approx_equal(v, 1.0 / 3.0)

    def test_numerical_stability(self, blas: CpuBlas) -> None:
        """Softmax should not overflow for large values."""
        x = Matrix(data=[1000.0, 1001.0, 1002.0], rows=1, cols=3)
        result = blas.softmax(x, axis=-1)
        total = sum(result.data)
        assert approx_equal(total, 1.0)
        # No NaN or Inf
        for v in result.data:
            assert math.isfinite(v)

    def test_multi_row(self, blas: CpuBlas) -> None:
        """Each row should independently sum to 1.0."""
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        result = blas.softmax(x, axis=-1)
        row1_sum = sum(result.data[0:3])
        row2_sum = sum(result.data[3:6])
        assert approx_equal(row1_sum, 1.0)
        assert approx_equal(row2_sum, 1.0)

    def test_axis_0(self, blas: CpuBlas) -> None:
        """Softmax along axis 0 (columns)."""
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.softmax(x, axis=0)
        # Each column should sum to 1.0
        col0_sum = result.data[0] + result.data[2]
        col1_sum = result.data[1] + result.data[3]
        assert approx_equal(col0_sum, 1.0)
        assert approx_equal(col1_sum, 1.0)


# =========================================================================
# Normalization
# =========================================================================


class TestLayerNorm:
    def test_basic(self, blas: CpuBlas) -> None:
        """Layer norm with gamma=1, beta=0 should produce zero mean, unit var."""
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        gamma = Vector(data=[1.0, 1.0, 1.0], size=3)
        beta = Vector(data=[0.0, 0.0, 0.0], size=3)
        result = blas.layer_norm(x, gamma, beta)

        # First row: [1, 2, 3] -> mean=2, var=2/3
        # Normalized: [-1.22, 0.0, 1.22] approximately
        assert approx_equal(result.data[1], 0.0, tol=1e-4)

    def test_with_scaling(self, blas: CpuBlas) -> None:
        """gamma scales the output, beta shifts it."""
        x = Matrix(data=[0.0, 0.0, 0.0], rows=1, cols=3)
        gamma = Vector(data=[2.0, 2.0, 2.0], size=3)
        beta = Vector(data=[1.0, 1.0, 1.0], size=3)
        result = blas.layer_norm(x, gamma, beta)
        # All same input -> all normalized to 0 -> gamma*0 + beta = 1.0
        # But var=0, so we get 0/sqrt(eps) which is 0, then gamma*0 + beta = 1
        for v in result.data:
            assert approx_equal(v, 1.0, tol=1e-3)

    def test_gamma_beta_mismatch(self, blas: CpuBlas) -> None:
        x = Matrix(data=[1.0, 2.0], rows=1, cols=2)
        gamma = Vector(data=[1.0, 1.0, 1.0], size=3)
        beta = Vector(data=[0.0, 0.0], size=2)
        with pytest.raises(ValueError, match="gamma"):
            blas.layer_norm(x, gamma, beta)


class TestBatchNorm:
    def test_inference_mode(self, blas: CpuBlas) -> None:
        """Batch norm in inference mode uses running stats."""
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        gamma = Vector(data=[1.0, 1.0], size=2)
        beta = Vector(data=[0.0, 0.0], size=2)
        running_mean = Vector(data=[2.0, 3.0], size=2)
        running_var = Vector(data=[1.0, 1.0], size=2)
        result = blas.batch_norm(
            x, gamma, beta, running_mean, running_var, training=False
        )
        # Column 0: (1-2)/1 = -1, (3-2)/1 = 1
        # Column 1: (2-3)/1 = -1, (4-3)/1 = 1
        assert approx_equal(result.data[0], -1.0, tol=1e-4)
        assert approx_equal(result.data[1], -1.0, tol=1e-4)
        assert approx_equal(result.data[2], 1.0, tol=1e-4)
        assert approx_equal(result.data[3], 1.0, tol=1e-4)

    def test_training_mode(self, blas: CpuBlas) -> None:
        """Batch norm in training mode computes batch statistics."""
        x = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        gamma = Vector(data=[1.0, 1.0], size=2)
        beta = Vector(data=[0.0, 0.0], size=2)
        running_mean = Vector(data=[0.0, 0.0], size=2)
        running_var = Vector(data=[1.0, 1.0], size=2)
        result = blas.batch_norm(
            x, gamma, beta, running_mean, running_var, training=True
        )
        # Column 0: mean=2, var=1, normalized: [-1, 1]
        # Column 1: mean=3, var=1, normalized: [-1, 1]
        assert approx_equal(result.data[0], -1.0, tol=1e-4)
        assert approx_equal(result.data[2], 1.0, tol=1e-4)

    def test_gamma_beta_mismatch(self, blas: CpuBlas) -> None:
        x = Matrix(data=[1.0, 2.0], rows=1, cols=2)
        gamma = Vector(data=[1.0], size=1)
        beta = Vector(data=[0.0, 0.0], size=2)
        rm = Vector(data=[0.0, 0.0], size=2)
        rv = Vector(data=[1.0, 1.0], size=2)
        with pytest.raises(ValueError, match="gamma"):
            blas.batch_norm(x, gamma, beta, rm, rv)


# =========================================================================
# Convolution
# =========================================================================


class TestConv2d:
    def test_identity_filter(self, blas: CpuBlas) -> None:
        """A 1x1 identity filter should extract the element."""
        inp = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        weight = Matrix(data=[1.0], rows=1, cols=1)
        result = blas.conv2d(inp, weight)
        assert result.rows == 2 and result.cols == 2
        assert approx_list(result.data, [1.0, 2.0, 3.0, 4.0])

    def test_3x3_on_3x3(self, blas: CpuBlas) -> None:
        """3x3 filter on 3x3 input produces 1x1 output."""
        inp = Matrix(data=[1.0] * 9, rows=3, cols=3)
        weight = Matrix(data=[1.0] * 9, rows=3, cols=3)
        result = blas.conv2d(inp, weight)
        assert result.rows == 1 and result.cols == 1
        assert approx_equal(result.data[0], 9.0)

    def test_with_padding(self, blas: CpuBlas) -> None:
        """Padding should preserve spatial dimensions."""
        inp = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        weight = Matrix(
            data=[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], rows=3, cols=3
        )
        result = blas.conv2d(inp, weight, padding=1)
        # With 3x3 filter, padding=1 on 2x2 input -> 2x2 output
        assert result.rows == 2 and result.cols == 2

    def test_with_bias(self, blas: CpuBlas) -> None:
        inp = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        weight = Matrix(data=[1.0], rows=1, cols=1)
        bias = Vector(data=[10.0], size=1)
        result = blas.conv2d(inp, weight, bias=bias)
        assert approx_list(result.data, [11.0, 12.0, 13.0, 14.0])

    def test_stride_2(self, blas: CpuBlas) -> None:
        """Stride 2 on a 4x4 input with 1x1 filter produces 2x2 output."""
        inp = Matrix(data=list(range(1, 17)), rows=4, cols=4)
        weight = Matrix(data=[1.0], rows=1, cols=1)
        result = blas.conv2d(inp, weight, stride=2)
        assert result.rows == 2 and result.cols == 2
        assert approx_list(result.data, [1.0, 3.0, 9.0, 11.0])


# =========================================================================
# Attention
# =========================================================================


class TestAttention:
    def test_basic(self, blas: CpuBlas) -> None:
        """Simple 2x2 attention."""
        q = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        k = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        v = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.attention(q, k, v)
        assert result.rows == 2 and result.cols == 2

    def test_identity_attention(self, blas: CpuBlas) -> None:
        """When Q=K=identity and scale is large, attention should focus."""
        q = Matrix(data=[10.0, 0.0, 0.0, 10.0], rows=2, cols=2)
        k = Matrix(data=[10.0, 0.0, 0.0, 10.0], rows=2, cols=2)
        v = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        result = blas.attention(q, k, v, scale=1.0)
        # With large scale=1 and Q*K^T = [[100,0],[0,100]],
        # softmax focuses almost entirely on the diagonal
        assert result.rows == 2 and result.cols == 2

    def test_custom_scale(self, blas: CpuBlas) -> None:
        q = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        k = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        v = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.attention(q, k, v, scale=2.0)
        assert result.rows == 2 and result.cols == 2

    def test_with_mask(self, blas: CpuBlas) -> None:
        """Causal mask should zero out future positions."""
        q = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        k = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        v = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        # Causal mask: upper triangle = -inf
        mask = Matrix(data=[0.0, -1e9, 0.0, 0.0], rows=2, cols=2)
        result = blas.attention(q, k, v, mask=mask)
        assert result.rows == 2 and result.cols == 2

    def test_output_shape(self, blas: CpuBlas) -> None:
        """Output shape = (seq_len x d_v)."""
        q = Matrix(data=[0.0] * 12, rows=3, cols=4)
        k = Matrix(data=[0.0] * 12, rows=3, cols=4)
        v = Matrix(data=[0.0] * 6, rows=3, cols=2)
        result = blas.attention(q, k, v)
        assert result.rows == 3
        assert result.cols == 2
