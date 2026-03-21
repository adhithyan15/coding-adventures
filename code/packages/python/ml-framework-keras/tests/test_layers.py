"""Tests for the layers module."""

import pytest
from ml_framework_core import Tensor

from ml_framework_keras.layers import (
    BatchNormalization,
    Dense,
    Dropout,
    Embedding,
    Flatten,
    Input,
    Layer,
    LayerNorm,
    LayerNormalization,
    ReLU,
    Softmax,
)


class TestLayerBase:
    def test_default_name(self):
        layer = Layer()
        assert layer._name == "layer"

    def test_custom_name(self):
        layer = Layer(name="my_layer")
        assert layer._name == "my_layer"

    def test_not_built_initially(self):
        layer = Layer()
        assert not layer._built

    def test_empty_weights(self):
        layer = Layer()
        assert layer.trainable_weights == []
        assert layer.non_trainable_weights == []

    def test_count_params_empty(self):
        layer = Layer()
        assert layer.count_params() == 0

    def test_get_config(self):
        layer = Layer(name="test")
        config = layer.get_config()
        assert config["name"] == "test"

    def test_repr(self):
        layer = Layer(name="my_layer")
        assert "my_layer" in repr(layer)

    def test_call_raises_not_implemented(self):
        layer = Layer()
        with pytest.raises(NotImplementedError):
            layer(Tensor.from_list([1.0]))

    def test_add_weight_glorot(self):
        layer = Layer()
        param = layer.add_weight("w", (4, 3), initializer="glorot_uniform")
        assert param.shape == (4, 3)
        assert param.requires_grad
        assert len(layer.trainable_weights) == 1

    def test_add_weight_zeros(self):
        layer = Layer()
        param = layer.add_weight("b", (5,), initializer="zeros")
        assert all(v == 0.0 for v in param.data)

    def test_add_weight_ones(self):
        layer = Layer()
        param = layer.add_weight("g", (3,), initializer="ones")
        assert all(v == 1.0 for v in param.data)

    def test_add_weight_non_trainable(self):
        layer = Layer()
        layer.add_weight("w", (3,), trainable=False)
        assert len(layer.trainable_weights) == 0
        assert len(layer.non_trainable_weights) == 1

    def test_add_weight_unknown_initializer(self):
        layer = Layer()
        param = layer.add_weight("w", (3,), initializer="random_normal")
        assert param.shape == (3,)


class TestDense:
    def test_basic_forward(self):
        dense = Dense(4)
        x = Tensor.from_list([[1.0, 2.0, 3.0]], shape=(1, 3))
        y = dense(x)
        assert y.shape == (1, 4)

    def test_lazy_build(self):
        dense = Dense(4)
        assert dense.kernel is None
        x = Tensor.from_list([[1.0, 2.0]], shape=(1, 2))
        dense(x)
        assert dense.kernel is not None
        assert dense.kernel.shape == (2, 4)

    def test_with_activation(self):
        dense = Dense(4, activation="relu")
        x = Tensor.from_list([[1.0, -1.0]], shape=(1, 2))
        y = dense(x)
        assert y.shape == (1, 4)
        # ReLU: no negative values in output
        for val in y.data:
            assert val >= 0.0

    def test_no_bias(self):
        dense = Dense(3, use_bias=False)
        x = Tensor.from_list([[1.0, 2.0]], shape=(1, 2))
        dense(x)
        assert dense.bias is None
        # Only kernel weight, no bias
        assert len(dense.trainable_weights) == 1

    def test_with_bias(self):
        dense = Dense(3, use_bias=True)
        x = Tensor.from_list([[1.0, 2.0]], shape=(1, 2))
        dense(x)
        assert dense.bias is not None
        assert len(dense.trainable_weights) == 2

    def test_batch_processing(self):
        dense = Dense(2)
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]], shape=(2, 2))
        y = dense(x)
        assert y.shape == (2, 2)

    def test_get_config(self):
        dense = Dense(128, activation="relu", use_bias=False)
        config = dense.get_config()
        assert config["units"] == 128
        assert config["activation"] == "relu"
        assert config["use_bias"] is False

    def test_get_config_no_activation(self):
        dense = Dense(64)
        config = dense.get_config()
        assert config["activation"] is None

    def test_repr(self):
        dense = Dense(128, activation="relu")
        assert "128" in repr(dense)
        assert "relu" in repr(dense)

    def test_count_params(self):
        dense = Dense(4, use_bias=True)
        x = Tensor.from_list([[1.0, 2.0, 3.0]], shape=(1, 3))
        dense(x)
        # kernel: 3*4=12, bias: 4 → total 16
        assert dense.count_params() == 16

    def test_build_requires_shape(self):
        dense = Dense(4)
        with pytest.raises(ValueError, match="requires an input shape"):
            dense.build(None)


class TestDropout:
    def test_training_mode_changes_values(self):
        dropout = Dropout(0.99)  # Very high rate for testing
        x = Tensor.from_list([1.0] * 100, shape=(10, 10))
        y = dropout(x, training=True)
        # With 99% dropout, most values should be 0
        zeros = sum(1 for v in y.data if v == 0.0)
        assert zeros > 50  # probabilistic, but very likely

    def test_inference_mode_passthrough(self):
        dropout = Dropout(0.5)
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = dropout(x, training=False)
        assert y.data == [1.0, 2.0, 3.0]

    def test_zero_rate_passthrough(self):
        dropout = Dropout(0.0)
        x = Tensor.from_list([1.0, 2.0])
        y = dropout(x, training=True)
        assert y.data == [1.0, 2.0]

    def test_invalid_rate(self):
        with pytest.raises(ValueError, match="Dropout rate"):
            Dropout(1.0)
        with pytest.raises(ValueError, match="Dropout rate"):
            Dropout(-0.1)

    def test_get_config(self):
        dropout = Dropout(0.3)
        config = dropout.get_config()
        assert config["rate"] == 0.3

    def test_repr(self):
        dropout = Dropout(0.5)
        assert "0.5" in repr(dropout)

    def test_no_weights(self):
        dropout = Dropout(0.5)
        assert dropout.trainable_weights == []


class TestBatchNormalization:
    def test_basic_forward_training(self):
        bn = BatchNormalization()
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]], shape=(3, 2))
        y = bn(x, training=True)
        assert y.shape == (3, 2)

    def test_normalizes_to_zero_mean(self):
        bn = BatchNormalization()
        x = Tensor.from_list([[1.0, 10.0], [3.0, 20.0], [5.0, 30.0]], shape=(3, 2))
        y = bn(x, training=True)
        # Each feature should have approximately zero mean
        for j in range(2):
            col_mean = sum(y.data[i * 2 + j] for i in range(3)) / 3
            assert abs(col_mean) < 1e-5

    def test_inference_mode(self):
        bn = BatchNormalization()
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]], shape=(2, 2))
        # First call in training to set running stats
        bn(x, training=True)
        # Then inference
        y = bn(x, training=False)
        assert y.shape == (2, 2)

    def test_has_trainable_weights(self):
        bn = BatchNormalization()
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]], shape=(2, 2))
        bn(x, training=True)
        # gamma and beta
        assert len(bn.trainable_weights) == 2

    def test_get_config(self):
        bn = BatchNormalization(epsilon=1e-5, momentum=0.9)
        config = bn.get_config()
        assert config["epsilon"] == 1e-5
        assert config["momentum"] == 0.9

    def test_repr(self):
        bn = BatchNormalization()
        assert "BatchNormalization" in repr(bn)


class TestLayerNormalization:
    def test_basic_forward(self):
        ln = LayerNormalization()
        x = Tensor.from_list([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]], shape=(2, 3))
        y = ln(x)
        assert y.shape == (2, 3)

    def test_normalizes_each_sample(self):
        ln = LayerNormalization()
        x = Tensor.from_list([[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]], shape=(2, 3))
        y = ln(x)
        # Each row should have approximately zero mean
        for i in range(2):
            row = y.data[i * 3 : (i + 1) * 3]
            row_mean = sum(row) / 3
            assert abs(row_mean) < 1e-5

    def test_has_weights(self):
        ln = LayerNormalization()
        x = Tensor.from_list([[1.0, 2.0]], shape=(1, 2))
        ln(x)
        assert len(ln.trainable_weights) == 2

    def test_alias(self):
        assert LayerNorm is LayerNormalization

    def test_get_config(self):
        ln = LayerNormalization(epsilon=1e-4)
        config = ln.get_config()
        assert config["epsilon"] == 1e-4


class TestFlatten:
    def test_basic(self):
        flatten = Flatten()
        x = Tensor.from_list([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], shape=(1, 2, 3))
        y = flatten(x)
        assert y.shape == (1, 6)
        assert y.data == [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]

    def test_batch_preserved(self):
        flatten = Flatten()
        x = Tensor.zeros(3, 2, 4)
        y = flatten(x)
        assert y.shape == (3, 8)

    def test_get_config(self):
        flatten = Flatten()
        config = flatten.get_config()
        assert "name" in config

    def test_repr(self):
        assert "Flatten" in repr(Flatten())


class TestEmbedding:
    def test_basic_lookup(self):
        embed = Embedding(10, 4)
        indices = Tensor.from_list([0.0, 1.0, 2.0], shape=(3,))
        y = embed(indices)
        assert y.shape == (3, 4)

    def test_2d_input(self):
        embed = Embedding(10, 3)
        indices = Tensor.from_list([[0.0, 1.0], [2.0, 3.0]], shape=(2, 2))
        y = embed(indices)
        assert y.shape == (2, 2, 3)

    def test_out_of_range_raises(self):
        embed = Embedding(5, 3)
        indices = Tensor.from_list([10.0], shape=(1,))
        embed.build((1,))
        with pytest.raises(IndexError):
            embed(indices)

    def test_has_weights(self):
        embed = Embedding(100, 32)
        indices = Tensor.from_list([0.0], shape=(1,))
        embed(indices)
        assert len(embed.trainable_weights) == 1
        assert embed.trainable_weights[0].shape == (100, 32)

    def test_get_config(self):
        embed = Embedding(100, 64)
        config = embed.get_config()
        assert config["input_dim"] == 100
        assert config["output_dim"] == 64

    def test_repr(self):
        embed = Embedding(100, 64)
        assert "100" in repr(embed)
        assert "64" in repr(embed)


class TestInput:
    def test_creation(self):
        inp = Input(shape=(784,))
        assert inp.shape == (784,)

    def test_with_name(self):
        inp = Input(shape=(784,), name="my_input")
        assert inp._name == "my_input"

    def test_repr(self):
        inp = Input(shape=(784,))
        assert "784" in repr(inp)


class TestReLULayer:
    def test_forward(self):
        layer = ReLU()
        x = Tensor.from_list([-1.0, 0.0, 1.0, 2.0])
        y = layer(x)
        assert y.data == [0.0, 0.0, 1.0, 2.0]

    def test_repr(self):
        assert "ReLU" in repr(ReLU())


class TestSoftmaxLayer:
    def test_forward(self):
        layer = Softmax()
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = layer(x)
        assert abs(sum(y.data) - 1.0) < 1e-6

    def test_custom_axis(self):
        layer = Softmax(axis=-1)
        config = layer.get_config()
        assert config["axis"] == -1

    def test_repr(self):
        assert "Softmax" in repr(Softmax())
