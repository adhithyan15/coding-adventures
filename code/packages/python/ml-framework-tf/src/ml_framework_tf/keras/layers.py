"""
================================================================
TF.KERAS.LAYERS — NEURAL NETWORK BUILDING BLOCKS
================================================================

Keras layers are the building blocks of neural networks. Each layer:
1. Has learnable parameters (weights and biases)
2. Implements a forward computation (the `call` method)
3. Tracks its own parameters for the optimizer

=== Keras vs PyTorch Naming ===

| Keras (TF)                  | PyTorch                     |
|-----------------------------|-----------------------------|
| Dense(units=64)             | Linear(out_features=64)     |
| layer.call(x)              | layer.forward(x)            |
| layer(x) calls call()      | layer(x) calls forward()    |
| layer.trainable_weights    | list(layer.parameters())    |
| activation='relu'          | separate ReLU() module      |

=== Layer Lifecycle ===

In real Keras, layers are "lazy" — they don't create weights until
the first call (when they know the input shape). Our simplified
implementation requires explicit input shapes for simplicity.

=== The Layer Base Class ===

All Keras layers inherit from Layer, which provides:
- Parameter tracking via trainable_weights
- Training/eval mode toggle
- The __call__ → call() delegation pattern

================================================================
"""

from __future__ import annotations

import math
import random as _random

from ml_framework_core import Parameter, Tensor

from . import activations


# =========================================================================
# Base Layer
# =========================================================================


class Layer:
    """Base class for all Keras layers.

    Subclasses implement call() for the forward computation.
    __call__ delegates to call(), matching Keras behavior.

    The trainable_weights property returns all learnable Parameters.
    """

    def __init__(self, **kwargs: object) -> None:
        self._trainable_weights: list[Parameter] = []
        self._layers: list[Layer] = []
        self.training = True
        self._name = kwargs.get("name", self.__class__.__name__)

    def call(self, x: Tensor) -> Tensor:
        """Forward computation. Subclasses must override."""
        raise NotImplementedError(f"{self.__class__.__name__} must implement call()")

    def __call__(self, *args: Tensor) -> Tensor:
        """Make layers callable: layer(x) calls layer.call(x)."""
        return self.call(*args)

    @property
    def trainable_weights(self) -> list[Parameter]:
        """All learnable parameters in this layer and sub-layers."""
        params = list(self._trainable_weights)
        for layer in self._layers:
            params.extend(layer.trainable_weights)
        return params

    @property
    def name(self) -> str:
        """Human-readable name for this layer."""
        return self._name


# =========================================================================
# Dense (Fully Connected) Layer
# =========================================================================


class Dense(Layer):
    """Fully connected layer: y = activation(x @ W + b).

    This is the most fundamental layer. Each output neuron is
    connected to every input neuron (hence "dense" or "fully connected").

    In TensorFlow/Keras, you specify the number of output units
    and optionally an activation function:
        Dense(64, activation='relu')

    This is equivalent to PyTorch's:
        nn.Sequential(nn.Linear(in_features, 64), nn.ReLU())

    But more compact because Keras bundles the activation with the layer.

    Args:
        units: Number of output neurons (out_features in PyTorch).
        activation: Activation function name ('relu', 'sigmoid', etc.)
                    or callable. None means no activation (linear).
        use_bias: Whether to include a bias term. Default: True.
        input_dim: Number of input features. Required on first layer
                   (later layers infer it from the previous layer).

    Example:
        layer = Dense(128, activation='relu', input_dim=784)
        output = layer(input_tensor)  # shape: (batch, 128)
    """

    def __init__(
        self,
        units: int,
        activation: str | callable | None = None,
        use_bias: bool = True,
        input_dim: int | None = None,
        **kwargs: object,
    ) -> None:
        super().__init__(**kwargs)
        self.units = units
        self.activation_fn = activations.get(activation)
        self.use_bias = use_bias
        self.input_dim = input_dim

        # ─── Build weights if input_dim is known ────────────────
        self._built = False
        if input_dim is not None:
            self._build(input_dim)

    def _build(self, input_dim: int) -> None:
        """Create weight and bias Parameters.

        Uses Xavier/Glorot initialization:
            stddev = 1 / sqrt(fan_in)

        This keeps activation variance stable across layers.
        """
        stddev = 1.0 / math.sqrt(input_dim)
        self.kernel = Parameter(Tensor.randn(input_dim, self.units) * stddev)
        self._trainable_weights.append(self.kernel)

        if self.use_bias:
            self.bias = Parameter(Tensor.zeros(self.units))
            self._trainable_weights.append(self.bias)
        else:
            self.bias = None

        self.input_dim = input_dim
        self._built = True

    def call(self, x: Tensor) -> Tensor:
        """Compute: activation(x @ kernel + bias).

        Steps:
        1. If not built yet, build with the input shape
        2. Matrix multiply: x @ kernel → (batch, units)
        3. Add bias (broadcast across batch)
        4. Apply activation function
        """
        # ─── Lazy build on first call ────────────────────────────
        if not self._built:
            self._build(x.shape[-1])

        # ─── Linear transformation ───────────────────────────────
        # x: (batch, input_dim), kernel: (input_dim, units)
        output = x @ self.kernel  # (batch, units)

        # ─── Add bias (with broadcasting) ────────────────────────
        if self.bias is not None:
            batch_size = x.shape[0]
            ones_col = Tensor.ones(batch_size, 1)
            bias_row = self.bias.reshape(1, self.units)
            bias_broadcast = ones_col @ bias_row
            output = output + bias_broadcast

        # ─── Apply activation ────────────────────────────────────
        output = self.activation_fn(output)

        return output

    def __repr__(self) -> str:
        act_name = getattr(self.activation_fn, "__name__", "None")
        return f"Dense(units={self.units}, activation='{act_name}')"


# =========================================================================
# Flatten
# =========================================================================


class Flatten(Layer):
    """Flatten all dimensions except the batch dimension.

    Converts a multi-dimensional tensor (batch, d1, d2, ...) into
    a 2-D tensor (batch, d1*d2*...).

    This is typically used between convolutional layers and dense layers:
        model = Sequential([
            Conv2D(32, 3),    # output: (batch, 26, 26, 32)
            Flatten(),         # output: (batch, 21632)
            Dense(128),        # output: (batch, 128)
        ])

    Example:
        x = Tensor.randn(4, 3, 8)  # batch=4, 3 channels, 8 wide
        flat = Flatten()(x)          # shape: (4, 24)
    """

    def call(self, x: Tensor) -> Tensor:
        if x.ndim <= 2:
            return x
        batch_size = x.shape[0]
        flat_size = 1
        for s in x.shape[1:]:
            flat_size *= s
        return x.reshape(batch_size, flat_size)

    def __repr__(self) -> str:
        return "Flatten()"


# =========================================================================
# Dropout
# =========================================================================


class Dropout(Layer):
    """Randomly zero elements during training for regularization.

    During training, each element is set to zero with probability `rate`,
    and surviving elements are scaled by 1/(1-rate) to maintain the
    expected value (inverted dropout).

    During inference (training=False), dropout is disabled.

    Args:
        rate: Fraction of elements to drop. Default: 0.5.

    Example:
        dropout = Dropout(0.3)
        dropout.training = True
        y = dropout(x)  # ~30% zeros, rest scaled by 1/0.7
    """

    def __init__(self, rate: float = 0.5, **kwargs: object) -> None:
        super().__init__(**kwargs)
        if not 0.0 <= rate < 1.0:
            raise ValueError(f"Dropout rate must be in [0, 1), got {rate}")
        self.rate = rate

    def call(self, x: Tensor) -> Tensor:
        if not self.training or self.rate == 0.0:
            return x

        scale = 1.0 / (1.0 - self.rate)
        data = []
        for val in x.data:
            if _random.random() < self.rate:
                data.append(0.0)
            else:
                data.append(val * scale)
        return Tensor(data, x.shape, device=x.device)

    def __repr__(self) -> str:
        return f"Dropout(rate={self.rate})"


# =========================================================================
# BatchNormalization
# =========================================================================


class BatchNormalization(Layer):
    """Batch normalization layer.

    Normalizes activations across the batch dimension, then applies
    a learnable scale (gamma) and shift (beta):
        y = gamma * (x - mean) / sqrt(var + eps) + beta

    During training: uses batch statistics.
    During inference: uses running averages.

    Args:
        axis: Feature axis to normalize. Default: -1 (last dim).
        epsilon: Small constant for numerical stability. Default: 1e-5.
        momentum: Factor for running statistics. Default: 0.1.

    Example:
        bn = BatchNormalization()
        x = Tensor.randn(32, 64)
        y = bn(x)  # normalized, same shape
    """

    def __init__(
        self,
        axis: int = -1,
        epsilon: float = 1e-5,
        momentum: float = 0.1,
        **kwargs: object,
    ) -> None:
        super().__init__(**kwargs)
        self.axis = axis
        self.epsilon = epsilon
        self.momentum = momentum
        self._num_features: int | None = None
        self.gamma: Parameter | None = None
        self.beta: Parameter | None = None
        self.running_mean: Tensor | None = None
        self.running_var: Tensor | None = None

    def _build(self, num_features: int) -> None:
        self._num_features = num_features
        self.gamma = Parameter(Tensor.ones(num_features))
        self.beta = Parameter(Tensor.zeros(num_features))
        self._trainable_weights.extend([self.gamma, self.beta])
        self.running_mean = Tensor.zeros(num_features)
        self.running_var = Tensor.ones(num_features)

    def call(self, x: Tensor) -> Tensor:
        if x.ndim != 2:
            raise ValueError(f"BatchNormalization expects 2-D input, got {x.ndim}-D")

        batch_size, features = x.shape

        if self.gamma is None:
            self._build(features)

        if self.training:
            mean = [0.0] * features
            for i in range(batch_size):
                for j in range(features):
                    mean[j] += x.data[i * features + j]
            mean = [m / batch_size for m in mean]

            var = [0.0] * features
            for i in range(batch_size):
                for j in range(features):
                    diff = x.data[i * features + j] - mean[j]
                    var[j] += diff * diff
            var = [v / batch_size for v in var]

            # Update running stats
            mom = self.momentum
            self.running_mean = Tensor(
                [(1 - mom) * r + mom * m for r, m in zip(self.running_mean.data, mean)],
                (features,),
            )
            self.running_var = Tensor(
                [(1 - mom) * r + mom * v for r, v in zip(self.running_var.data, var)],
                (features,),
            )
        else:
            mean = list(self.running_mean.data)
            var = list(self.running_var.data)

        result = [0.0] * (batch_size * features)
        for i in range(batch_size):
            for j in range(features):
                idx = i * features + j
                normalized = (x.data[idx] - mean[j]) / math.sqrt(var[j] + self.epsilon)
                result[idx] = self.gamma.data[j] * normalized + self.beta.data[j]

        return Tensor(result, x.shape, device=x.device)

    def __repr__(self) -> str:
        return f"BatchNormalization(epsilon={self.epsilon})"


# =========================================================================
# LayerNormalization
# =========================================================================


class LayerNormalization(Layer):
    """Layer normalization — normalizes across features for each sample.

    Unlike BatchNorm, LayerNorm normalizes per-sample (not per-batch),
    so it behaves the same during training and inference. Preferred
    in transformers and sequence models.

    Args:
        axis: Axis to normalize. Default: -1.
        epsilon: Numerical stability constant. Default: 1e-5.
    """

    def __init__(
        self,
        axis: int = -1,
        epsilon: float = 1e-5,
        **kwargs: object,
    ) -> None:
        super().__init__(**kwargs)
        self.axis = axis
        self.epsilon = epsilon
        self._num_features: int | None = None
        self.gamma: Parameter | None = None
        self.beta: Parameter | None = None

    def _build(self, num_features: int) -> None:
        self._num_features = num_features
        self.gamma = Parameter(Tensor.ones(num_features))
        self.beta = Parameter(Tensor.zeros(num_features))
        self._trainable_weights.extend([self.gamma, self.beta])

    def call(self, x: Tensor) -> Tensor:
        if x.ndim != 2:
            raise ValueError(f"LayerNormalization expects 2-D input, got {x.ndim}-D")

        batch_size, features = x.shape
        if self.gamma is None:
            self._build(features)

        result = [0.0] * (batch_size * features)
        for i in range(batch_size):
            row_start = i * features
            row = x.data[row_start : row_start + features]
            mean = sum(row) / features
            var = sum((v - mean) ** 2 for v in row) / features
            inv_std = 1.0 / math.sqrt(var + self.epsilon)

            for j in range(features):
                normalized = (row[j] - mean) * inv_std
                result[row_start + j] = (
                    self.gamma.data[j] * normalized + self.beta.data[j]
                )

        return Tensor(result, x.shape, device=x.device)

    def __repr__(self) -> str:
        return f"LayerNormalization(epsilon={self.epsilon})"


# =========================================================================
# Embedding
# =========================================================================


class Embedding(Layer):
    """Maps integer indices to dense vectors.

    A lookup table: token ID → embedding vector.
    The embeddings are learnable parameters updated during training.

    Args:
        input_dim: Vocabulary size (number of unique tokens).
        output_dim: Dimension of each embedding vector.

    Example:
        embed = Embedding(1000, 64)
        indices = Tensor.from_list([5.0, 10.0, 15.0])
        vectors = embed(indices)  # shape: (3, 64)
    """

    def __init__(
        self,
        input_dim: int,
        output_dim: int,
        **kwargs: object,
    ) -> None:
        super().__init__(**kwargs)
        self.input_dim = input_dim
        self.output_dim = output_dim
        self.embeddings = Parameter(Tensor.randn(input_dim, output_dim))
        self._trainable_weights.append(self.embeddings)

    def call(self, indices: Tensor) -> Tensor:
        if indices.ndim != 1:
            raise ValueError(f"Embedding expects 1-D indices, got {indices.ndim}-D")

        num_indices = indices.shape[0]
        result = [0.0] * (num_indices * self.output_dim)

        for k in range(num_indices):
            idx = int(indices.data[k])
            if idx < 0 or idx >= self.input_dim:
                raise IndexError(f"Index {idx} out of range [0, {self.input_dim})")
            row_start = idx * self.output_dim
            for j in range(self.output_dim):
                result[k * self.output_dim + j] = self.embeddings.data[row_start + j]

        return Tensor(result, (num_indices, self.output_dim), device=indices.device)

    def __repr__(self) -> str:
        return f"Embedding(input_dim={self.input_dim}, output_dim={self.output_dim})"


# =========================================================================
# Activation Layers (wrappers for use in Sequential)
# =========================================================================


class ReLU(Layer):
    """ReLU activation as a layer (for use in Sequential)."""

    def call(self, x: Tensor) -> Tensor:
        return activations.relu(x)

    def __repr__(self) -> str:
        return "ReLU()"


class Softmax(Layer):
    """Softmax activation as a layer.

    Args:
        axis: Dimension along which to compute softmax. Default: -1.
    """

    def __init__(self, axis: int = -1, **kwargs: object) -> None:
        super().__init__(**kwargs)
        self.axis = axis

    def call(self, x: Tensor) -> Tensor:
        return activations.softmax(x, axis=self.axis)

    def __repr__(self) -> str:
        return f"Softmax(axis={self.axis})"


# =========================================================================
# Input (placeholder for functional API)
# =========================================================================


class Input:
    """Specifies the shape of input data for the functional API.

    This doesn't perform any computation — it's a metadata marker
    that tells the Model what shape to expect.

    Args:
        shape: Shape of one sample (excluding batch dimension).

    Example:
        inputs = Input(shape=(784,))
        x = Dense(128, activation='relu')(inputs)
    """

    def __init__(self, shape: tuple[int, ...], **kwargs: object) -> None:
        self.shape = shape
        self._name = kwargs.get("name", "input")

    def __repr__(self) -> str:
        return f"Input(shape={self.shape})"
