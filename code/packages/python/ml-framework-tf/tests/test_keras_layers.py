"""Tests for tf.keras.layers — neural network building blocks."""

import pytest
import ml_framework_tf as tf
from ml_framework_core import Tensor
from ml_framework_tf.keras.layers import (
    BatchNormalization,
    Dense,
    Dropout,
    Embedding,
    Flatten,
    Input,
    Layer,
    LayerNormalization,
    ReLU,
    Softmax,
)


class TestDense:
    def test_output_shape(self):
        layer = Dense(64, input_dim=10)
        x = Tensor.randn(4, 10)
        y = layer(x)
        assert y.shape == (4, 64)

    def test_lazy_build(self):
        layer = Dense(32)
        assert not layer._built
        x = Tensor.randn(2, 8)
        y = layer(x)
        assert layer._built
        assert y.shape == (2, 32)

    def test_with_activation(self):
        layer = Dense(8, activation="relu", input_dim=4)
        x = Tensor.randn(2, 4)
        y = layer(x)
        assert y.shape == (2, 8)
        # ReLU means no negative values
        for val in y.data:
            assert val >= 0.0

    def test_no_bias(self):
        layer = Dense(4, use_bias=False, input_dim=3)
        assert layer.bias is None
        assert len(layer.trainable_weights) == 1  # only kernel

    def test_trainable_weights(self):
        layer = Dense(4, input_dim=3)
        weights = layer.trainable_weights
        assert len(weights) == 2  # kernel + bias

    def test_repr(self):
        layer = Dense(64, activation="relu")
        assert "Dense" in repr(layer)


class TestFlatten:
    def test_3d_to_2d(self):
        x = Tensor.randn(4, 3, 8)
        flat = Flatten()(x)
        assert flat.shape == (4, 24)

    def test_2d_passthrough(self):
        x = Tensor.randn(4, 10)
        flat = Flatten()(x)
        assert flat.shape == (4, 10)

    def test_repr(self):
        assert "Flatten" in repr(Flatten())


class TestDropout:
    def test_training_mode(self):
        dropout = Dropout(rate=0.5)
        dropout.training = True
        x = Tensor.ones(100)
        y = dropout(x)
        # Some values should be zero, others scaled
        num_zeros = sum(1 for v in y.data if v == 0.0)
        assert num_zeros > 10  # statistical check
        assert num_zeros < 90

    def test_eval_mode(self):
        dropout = Dropout(rate=0.5)
        dropout.training = False
        x = Tensor.ones(10)
        y = dropout(x)
        assert y.data == [1.0] * 10

    def test_rate_zero(self):
        dropout = Dropout(rate=0.0)
        dropout.training = True
        x = Tensor.ones(10)
        y = dropout(x)
        assert y.data == [1.0] * 10

    def test_invalid_rate(self):
        with pytest.raises(ValueError):
            Dropout(rate=1.0)

    def test_repr(self):
        assert "0.3" in repr(Dropout(rate=0.3))


class TestBatchNormalization:
    def test_output_shape(self):
        bn = BatchNormalization()
        x = Tensor.randn(4, 8)
        y = bn(x)
        assert y.shape == (4, 8)

    def test_training_normalizes(self):
        bn = BatchNormalization()
        bn.training = True
        # Create data with known mean/var
        data = [10.0] * 8 + [20.0] * 8  # two samples
        x = Tensor(data, (2, 8))
        y = bn(x)
        # After normalization, values should be close to -1 and +1
        assert abs(y.data[0] - y.data[8]) > 0.1

    def test_eval_uses_running_stats(self):
        bn = BatchNormalization()
        bn.training = True
        x = Tensor.randn(4, 4)
        bn(x)  # update running stats
        bn.training = False
        y = bn(x)
        assert y.shape == x.shape

    def test_trainable_weights(self):
        bn = BatchNormalization()
        x = Tensor.randn(4, 4)
        bn(x)  # triggers build
        assert len(bn.trainable_weights) == 2  # gamma + beta

    def test_invalid_ndim(self):
        bn = BatchNormalization()
        x = Tensor.from_list([1.0, 2.0, 3.0])
        with pytest.raises(ValueError, match="2-D"):
            bn(x)


class TestLayerNormalization:
    def test_output_shape(self):
        ln = LayerNormalization()
        x = Tensor.randn(4, 8)
        y = ln(x)
        assert y.shape == (4, 8)

    def test_per_sample_normalization(self):
        ln = LayerNormalization()
        x = Tensor.from_list([[10.0, 20.0, 30.0, 40.0]])
        y = ln(x)
        # Mean of normalized output should be ~0
        mean = sum(y.data) / len(y.data)
        assert abs(mean) < 0.1

    def test_trainable_weights(self):
        ln = LayerNormalization()
        x = Tensor.randn(2, 4)
        ln(x)
        assert len(ln.trainable_weights) == 2


class TestEmbedding:
    def test_output_shape(self):
        embed = Embedding(100, 16)
        indices = Tensor.from_list([0.0, 5.0, 10.0])
        y = embed(indices)
        assert y.shape == (3, 16)

    def test_lookup_values(self):
        embed = Embedding(10, 4)
        indices = Tensor.from_list([0.0])
        y = embed(indices)
        # Should match the first row of the weight matrix
        assert y.data == embed.embeddings.data[:4]

    def test_out_of_range(self):
        embed = Embedding(10, 4)
        indices = Tensor.from_list([10.0])
        with pytest.raises(IndexError):
            embed(indices)

    def test_trainable_weights(self):
        embed = Embedding(10, 4)
        assert len(embed.trainable_weights) == 1


class TestActivationLayers:
    def test_relu_layer(self):
        layer = ReLU()
        x = tf.constant([-1.0, 0.0, 1.0])
        y = layer(x)
        assert y.data == [0.0, 0.0, 1.0]

    def test_softmax_layer(self):
        layer = Softmax()
        x = tf.constant([1.0, 2.0, 3.0])
        y = layer(x)
        assert abs(sum(y.data) - 1.0) < 1e-6


class TestInput:
    def test_shape(self):
        inp = Input(shape=(784,))
        assert inp.shape == (784,)

    def test_repr(self):
        assert "784" in repr(Input(shape=(784,)))


class TestLayerBase:
    def test_abstract_call(self):
        layer = Layer()
        with pytest.raises(NotImplementedError):
            layer(Tensor.ones(2))

    def test_name(self):
        layer = Layer(name="my_layer")
        assert layer.name == "my_layer"
