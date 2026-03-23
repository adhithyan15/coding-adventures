"""
================================================================
LAYERS — THE BUILDING BLOCKS OF NEURAL NETWORKS
================================================================

Layers are the atoms of a Keras model. Each layer:
1. Has learnable weights (Parameters) that the optimizer updates
2. Transforms input tensors to output tensors (the forward pass)
3. Builds its weights lazily on first call (so you don't need to
   specify input dimensions — Keras figures them out automatically)

=== The Layer Lifecycle ===

    layer = Dense(128, activation="relu")   # 1. Create (no weights yet!)
    y = layer(x)                            # 2. First call triggers build()
    y = layer(x2)                           # 3. Subsequent calls skip build

This "lazy building" is one of Keras's best design decisions. In
PyTorch, you must specify both input AND output dimensions:

    nn.Linear(784, 128)   # PyTorch: must know input size

In Keras, you only specify the output:

    Dense(128)            # Keras: input size inferred from first input

=== The Layer Base Class ===

All layers inherit from Layer, which provides:
- Lazy weight creation via build()
- Weight tracking (trainable_weights, non_trainable_weights)
- Configuration serialization (get_config)
- Parameter counting (count_params)

================================================================
"""

from __future__ import annotations

import math
import random
from typing import Any

from ml_framework_core import Parameter, Tensor

from .activations import get_activation


# =========================================================================
# Base Layer
# =========================================================================


class Layer:
    """Base class for all Keras layers.

    Every layer must implement:
    - call(inputs, training=None): the forward computation
    - build(input_shape): create weights (called once, on first input)

    The __call__ method handles the lazy-build pattern automatically.
    """

    def __init__(self, **kwargs: Any) -> None:
        # ─── Internal state ──────────────────────────────────────
        self._trainable_weights: list[Parameter] = []
        self._non_trainable_weights: list[Parameter] = []
        self._built = False
        self._name = kwargs.get("name", self.__class__.__name__.lower())

    # ─── Weight management ───────────────────────────────────────

    def add_weight(
        self,
        name: str,
        shape: tuple[int, ...],
        initializer: str = "glorot_uniform",
        trainable: bool = True,
    ) -> Parameter:
        """Create and register a weight (Parameter) for this layer.

        The initializer determines the initial values:
        - "glorot_uniform" (default): Xavier uniform initialization
          Samples from U(-limit, limit) where limit = sqrt(6 / (fan_in + fan_out))
          This keeps activation variance stable across layers.
        - "zeros": All zeros (commonly used for biases)

        Args:
            name: Human-readable name for the weight.
            shape: Shape of the parameter tensor.
            initializer: Initialization strategy.
            trainable: If True, this weight will be updated by the optimizer.

        Returns:
            A new Parameter tensor, registered with this layer.
        """
        if initializer == "zeros":
            data = [0.0] * _numel(shape)
        elif initializer == "glorot_uniform":
            # Xavier/Glorot uniform: limit = sqrt(6 / (fan_in + fan_out))
            # For a weight matrix (fan_in, fan_out), this keeps activations
            # at a reasonable scale regardless of layer width.
            fan_in = shape[0] if len(shape) >= 1 else 1
            fan_out = shape[1] if len(shape) >= 2 else shape[0]
            limit = math.sqrt(6.0 / (fan_in + fan_out))
            data = [random.uniform(-limit, limit) for _ in range(_numel(shape))]
        elif initializer == "ones":
            data = [1.0] * _numel(shape)
        else:
            # Default: small random normal values
            data = [random.gauss(0, 0.01) for _ in range(_numel(shape))]

        tensor = Tensor(data, shape, requires_grad=trainable)
        param = Parameter(tensor)

        if trainable:
            self._trainable_weights.append(param)
        else:
            self._non_trainable_weights.append(param)

        return param

    @property
    def trainable_weights(self) -> list[Parameter]:
        """List of all trainable weights in this layer."""
        return list(self._trainable_weights)

    @property
    def non_trainable_weights(self) -> list[Parameter]:
        """List of all non-trainable weights in this layer."""
        return list(self._non_trainable_weights)

    def count_params(self) -> int:
        """Total number of trainable parameters."""
        return sum(p.numel for p in self._trainable_weights)

    # ─── Build / Call pattern ────────────────────────────────────

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        """Create weights based on the input shape.

        Override this in subclasses to create layer-specific weights.
        Called automatically on the first __call__.

        Args:
            input_shape: Shape of the input tensor (without batch dim
                         in some cases, depending on the layer).
        """
        self._built = True

    def call(self, inputs: Any, training: bool | None = None) -> Any:
        """Forward computation. Subclasses must override this.

        Args:
            inputs: Input tensor(s).
            training: Whether the model is in training mode. Some layers
                      (like Dropout, BatchNorm) behave differently during
                      training vs inference.

        Returns:
            Output tensor(s).
        """
        raise NotImplementedError

    def __call__(self, *args: Any, **kwargs: Any) -> Any:
        """Invoke the layer, auto-building on first call.

        This is the key Keras pattern: layers build themselves
        lazily when they see their first input. This means you
        never need to specify input dimensions explicitly.

        The flow:
        1. If not built, call build(input_shape) to create weights
        2. Call self.call(inputs) for the actual computation
        """
        if not self._built:
            # Infer input shape from the first argument
            first_arg = args[0] if args else None
            if hasattr(first_arg, "shape"):
                self.build(first_arg.shape)
            else:
                self.build(None)
        return self.call(*args, **kwargs)

    # ─── Serialization ───────────────────────────────────────────

    def get_config(self) -> dict[str, Any]:
        """Return a dict describing this layer's configuration.

        This enables model serialization/deserialization — you can
        recreate a layer from its config dict.
        """
        return {"name": self._name}

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(name='{self._name}')"


# =========================================================================
# Dense — Fully Connected Layer
# =========================================================================


class Dense(Layer):
    """Fully connected (dense) layer: y = activation(x @ W + b).

    This is the most fundamental layer in deep learning. Every input
    is connected to every output through a learnable weight.

    For an input of shape (batch_size, in_features):
        W has shape (in_features, units)
        b has shape (units,)
        output has shape (batch_size, units)

    Note: Unlike PyTorch's Linear (which stores W as (out, in) and
    transposes during forward), Keras stores W as (in, out) — no
    transpose needed. This matches the standard math notation:
        y = x @ W + b

    Args:
        units: Number of output features (neurons).
        activation: Activation function (string, callable, or None).
        use_bias: Whether to add a bias term. Default: True.

    Example:
        layer = Dense(128, activation="relu")
        x = Tensor.randn(32, 784)   # batch of 32, 784 features each
        y = layer(x)                 # shape: (32, 128)
    """

    def __init__(
        self,
        units: int,
        activation: str | None = None,
        use_bias: bool = True,
        **kwargs: Any,
    ) -> None:
        super().__init__(**kwargs)
        self.units = units
        self.activation = get_activation(activation)
        self.use_bias = use_bias

        # Weights are created lazily in build() — we don't know
        # the input size until the first call.
        self.kernel: Parameter | None = None
        self.bias: Parameter | None = None

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        """Create kernel and bias weights.

        The kernel shape is (in_features, units) — this way the
        forward pass is just x @ kernel (no transpose needed).

        We use Glorot uniform initialization for the kernel
        and zeros for the bias. This is the Keras default.
        """
        if input_shape is None:
            raise ValueError("Dense layer requires an input shape to build.")
        in_features = input_shape[-1]

        # Kernel: (in_features, units) with Glorot init
        self.kernel = self.add_weight(
            "kernel", (in_features, self.units), initializer="glorot_uniform"
        )

        # Bias: (units,) initialized to zeros
        if self.use_bias:
            self.bias = self.add_weight("bias", (self.units,), initializer="zeros")

        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        """Forward pass: y = activation(x @ W + b).

        Steps:
        1. Matrix multiply: x @ kernel → (batch, units)
        2. Add bias (if enabled): broadcast and add
        3. Apply activation (if specified)
        """
        assert self.kernel is not None, "Layer not built"
        output = inputs @ self.kernel

        if self.use_bias and self.bias is not None:
            # Broadcast bias across batch dimension.
            # bias has shape (units,), we need (batch_size, units).
            # Use the ones-column trick for autograd compatibility:
            #   ones(batch, 1) @ bias.reshape(1, units) → (batch, units)
            batch_size = inputs.shape[0]
            ones_col = Tensor.ones(batch_size, 1)
            bias_row = self.bias.reshape(1, self.units)
            bias_broadcast = ones_col @ bias_row
            output = output + bias_broadcast

        if self.activation is not None:
            output = self.activation(output)

        return output

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config.update(
            {
                "units": self.units,
                "activation": (self.activation.__name__ if self.activation else None),
                "use_bias": self.use_bias,
            }
        )
        return config

    def __repr__(self) -> str:
        act = self.activation.__name__ if self.activation else "None"
        return f"Dense(units={self.units}, activation={act})"


# =========================================================================
# Dropout — Regularization by Randomly Zeroing Activations
# =========================================================================


class Dropout(Layer):
    """Randomly zeros elements during training to prevent overfitting.

    During training, each element has probability `rate` of being set
    to zero. The remaining elements are scaled up by 1/(1-rate) to
    keep the expected value the same (inverted dropout).

    Why? Without dropout, neurons co-adapt — they rely on specific
    other neurons being present. Dropout forces each neuron to be
    useful on its own, leading to more robust features.

    During inference (training=False), dropout does nothing — all
    values pass through unchanged.

    The 1/(1-rate) scaling is called "inverted dropout":
        Training:   randomly zero `rate` fraction, scale rest by 1/(1-rate)
        Inference:  pass through unchanged

    This means we don't need to scale at inference time, which is
    simpler and faster.

    Args:
        rate: Fraction of elements to zero. Between 0 and 1. Default: 0.5.

    Example:
        dropout = Dropout(0.3)           # zero 30% of elements
        y = dropout(x, training=True)    # some values zeroed and scaled
        y = dropout(x, training=False)   # all values pass through
    """

    def __init__(self, rate: float = 0.5, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        if not 0.0 <= rate < 1.0:
            raise ValueError(f"Dropout rate must be in [0, 1), got {rate}")
        self.rate = rate

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        # Dropout has no weights — nothing to build
        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        """Apply dropout during training, pass through during inference."""
        if not training or self.rate == 0.0:
            return inputs

        # Generate random mask: 1 with probability (1-rate), 0 with probability rate
        scale = 1.0 / (1.0 - self.rate)
        mask_data = [
            scale if random.random() >= self.rate else 0.0
            for _ in range(len(inputs.data))
        ]
        mask = Tensor(mask_data, inputs.shape)
        return inputs * mask

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["rate"] = self.rate
        return config

    def __repr__(self) -> str:
        return f"Dropout(rate={self.rate})"


# =========================================================================
# BatchNormalization — Normalize Activations Across the Batch
# =========================================================================


class BatchNormalization(Layer):
    """Batch normalization: normalize activations to zero mean, unit variance.

    For each feature (column), across all samples in the batch:
        x_norm = (x - mean(x)) / sqrt(var(x) + epsilon)
        y = gamma * x_norm + beta

    Where gamma (scale) and beta (shift) are learnable parameters
    that let the network undo the normalization if that's useful.

    === Why Batch Normalization Works ===

    1. Reduces internal covariate shift: earlier layers change their
       weight distributions during training, which shifts the input
       distribution for later layers. BatchNorm stabilizes this.

    2. Allows higher learning rates: normalized activations don't
       explode or vanish, so you can use larger step sizes.

    3. Acts as a regularizer: the noise from batch statistics
       (each batch has slightly different mean/variance) adds
       regularization similar to dropout.

    === Training vs Inference ===

    Training: use batch mean/variance (computed from current mini-batch)
    Inference: use running mean/variance (exponential moving average
    accumulated during training)

    Args:
        epsilon: Small constant for numerical stability. Default: 1e-3.
        momentum: Factor for running mean/variance update. Default: 0.99.
            running = momentum * running + (1 - momentum) * batch_stat

    Example:
        bn = BatchNormalization()
        y = bn(x, training=True)    # normalize using batch stats
        y = bn(x, training=False)   # normalize using running stats
    """

    def __init__(
        self,
        epsilon: float = 1e-3,
        momentum: float = 0.99,
        **kwargs: Any,
    ) -> None:
        super().__init__(**kwargs)
        self.epsilon = epsilon
        self.momentum = momentum

        # These are set in build()
        self.gamma: Parameter | None = None
        self.beta: Parameter | None = None
        self._running_mean: list[float] | None = None
        self._running_var: list[float] | None = None

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        """Create gamma, beta, and running statistics."""
        if input_shape is None:
            raise ValueError("BatchNormalization requires an input shape.")
        num_features = input_shape[-1]

        # Learnable scale (gamma) — initialized to 1
        self.gamma = self.add_weight("gamma", (num_features,), initializer="ones")
        # Learnable shift (beta) — initialized to 0
        self.beta = self.add_weight("beta", (num_features,), initializer="zeros")

        # Running statistics for inference (not trainable)
        self._running_mean = [0.0] * num_features
        self._running_var = [1.0] * num_features

        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        """Normalize the input.

        Training mode: compute mean/var from the current batch
        Inference mode: use accumulated running mean/var
        """
        assert self.gamma is not None and self.beta is not None
        assert self._running_mean is not None and self._running_var is not None

        batch_size = inputs.shape[0]
        num_features = inputs.shape[-1]

        if training:
            # Compute batch mean and variance for each feature
            mean = [0.0] * num_features
            for i in range(batch_size):
                for j in range(num_features):
                    mean[j] += inputs.data[i * num_features + j]
            mean = [m / batch_size for m in mean]

            var = [0.0] * num_features
            for i in range(batch_size):
                for j in range(num_features):
                    diff = inputs.data[i * num_features + j] - mean[j]
                    var[j] += diff * diff
            var = [v / batch_size for v in var]

            # Update running statistics (exponential moving average)
            for j in range(num_features):
                self._running_mean[j] = (
                    self.momentum * self._running_mean[j]
                    + (1 - self.momentum) * mean[j]
                )
                self._running_var[j] = (
                    self.momentum * self._running_var[j] + (1 - self.momentum) * var[j]
                )
        else:
            mean = self._running_mean
            var = self._running_var

        # Normalize: (x - mean) / sqrt(var + eps)
        # Then scale and shift: gamma * x_norm + beta
        result_data = []
        for i in range(batch_size):
            for j in range(num_features):
                x_val = inputs.data[i * num_features + j]
                x_norm = (x_val - mean[j]) / math.sqrt(var[j] + self.epsilon)
                result_data.append(self.gamma.data[j] * x_norm + self.beta.data[j])

        return Tensor(result_data, inputs.shape, requires_grad=inputs.requires_grad)

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config.update(
            {
                "epsilon": self.epsilon,
                "momentum": self.momentum,
            }
        )
        return config

    def __repr__(self) -> str:
        return f"BatchNormalization(epsilon={self.epsilon}, momentum={self.momentum})"


# =========================================================================
# LayerNormalization — Normalize Across Features (Not Batch)
# =========================================================================


class LayerNormalization(Layer):
    """Layer normalization: normalize across features for each sample.

    Unlike BatchNorm (which normalizes across the batch for each feature),
    LayerNorm normalizes across features for each sample independently.

    For each sample:
        x_norm = (x - mean(x)) / sqrt(var(x) + epsilon)
        y = gamma * x_norm + beta

    === BatchNorm vs LayerNorm ===

    BatchNorm: normalize each FEATURE across the BATCH
        → mean/var shape: (num_features,)
        → depends on batch size (problematic for small batches)

    LayerNorm: normalize each SAMPLE across its FEATURES
        → mean/var: one scalar per sample
        → independent of batch size (works with any batch size)

    LayerNorm is preferred in transformers because:
    1. Sequence lengths vary → batch stats are unreliable
    2. It's independent of batch size → works with batch_size=1

    Args:
        epsilon: Small constant for numerical stability. Default: 1e-6.
    """

    def __init__(self, epsilon: float = 1e-6, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.epsilon = epsilon
        self.gamma: Parameter | None = None
        self.beta: Parameter | None = None

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        if input_shape is None:
            raise ValueError("LayerNormalization requires an input shape.")
        num_features = input_shape[-1]

        self.gamma = self.add_weight("gamma", (num_features,), initializer="ones")
        self.beta = self.add_weight("beta", (num_features,), initializer="zeros")
        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        """Normalize each sample across its features."""
        assert self.gamma is not None and self.beta is not None

        batch_size = inputs.shape[0]
        num_features = inputs.shape[-1]
        result_data = []

        for i in range(batch_size):
            start = i * num_features
            end = start + num_features
            sample = inputs.data[start:end]

            mean = sum(sample) / num_features
            var = sum((x - mean) ** 2 for x in sample) / num_features

            for j in range(num_features):
                x_norm = (sample[j] - mean) / math.sqrt(var + self.epsilon)
                result_data.append(self.gamma.data[j] * x_norm + self.beta.data[j])

        return Tensor(result_data, inputs.shape, requires_grad=inputs.requires_grad)

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["epsilon"] = self.epsilon
        return config

    def __repr__(self) -> str:
        return f"LayerNormalization(epsilon={self.epsilon})"


# Alias to match Keras naming conventions
LayerNorm = LayerNormalization


# =========================================================================
# Flatten — Collapse All Dimensions Into One
# =========================================================================


class Flatten(Layer):
    """Flatten the input to 2D: (batch_size, features).

    Converts any multi-dimensional input into a flat vector per sample.
    This is typically used between convolutional layers and dense layers.

    Example:
        # Input shape: (32, 8, 8, 64) — batch of 32, 8x8 feature maps, 64 channels
        # Output shape: (32, 4096) — 8*8*64 = 4096 features per sample

    Note: The batch dimension (first) is always preserved.
    """

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        batch_size = inputs.shape[0]
        flat_size = 1
        for dim in inputs.shape[1:]:
            flat_size *= dim
        return inputs.reshape(batch_size, flat_size)

    def get_config(self) -> dict[str, Any]:
        return super().get_config()

    def __repr__(self) -> str:
        return "Flatten()"


# =========================================================================
# Embedding — Map Integer Indices to Dense Vectors
# =========================================================================


class Embedding(Layer):
    """Map integer indices to dense vector representations.

    An embedding table is a matrix of shape (input_dim, output_dim).
    Each row is the learned vector for one token/word/category.

    Given an integer index i, the embedding layer returns row i of
    the embedding matrix. This is mathematically equivalent to
    one-hot encoding followed by a Dense layer, but much more
    memory-efficient.

    === Why Embeddings Matter ===

    Words (or any categorical data) can't be fed directly to neural
    networks — they need to be converted to numbers. One-hot encoding
    creates huge sparse vectors (vocabulary_size dimensions). Embeddings
    compress this to a dense vector of any size you choose.

    The embedding vectors are LEARNED during training. After training,
    similar words end up with similar vectors (e.g., "king" and "queen"
    are close in embedding space).

    Args:
        input_dim: Size of the vocabulary (number of unique tokens).
        output_dim: Dimensionality of the embedding vectors.

    Example:
        embed = Embedding(10000, 128)   # 10K words → 128-dim vectors
        # Input: integer tensor of shape (batch, seq_len)
        # Output: float tensor of shape (batch, seq_len, 128)
    """

    def __init__(self, input_dim: int, output_dim: int, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.input_dim = input_dim
        self.output_dim = output_dim
        self.embeddings: Parameter | None = None

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        self.embeddings = self.add_weight(
            "embeddings",
            (self.input_dim, self.output_dim),
            initializer="glorot_uniform",
        )
        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        """Look up embeddings for integer indices.

        For each integer in the input, fetch the corresponding row
        from the embedding matrix.
        """
        assert self.embeddings is not None
        result_data: list[float] = []

        for idx_float in inputs.data:
            idx = int(idx_float)
            if idx < 0 or idx >= self.input_dim:
                raise IndexError(
                    f"Embedding index {idx} out of range [0, {self.input_dim})"
                )
            # Fetch row `idx` from the embedding matrix
            start = idx * self.output_dim
            end = start + self.output_dim
            result_data.extend(self.embeddings.data[start:end])

        # Output shape: input_shape + (output_dim,)
        output_shape = inputs.shape + (self.output_dim,)
        return Tensor(result_data, output_shape)

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config.update(
            {
                "input_dim": self.input_dim,
                "output_dim": self.output_dim,
            }
        )
        return config

    def __repr__(self) -> str:
        return f"Embedding(input_dim={self.input_dim}, output_dim={self.output_dim})"


# =========================================================================
# Input — Symbolic Placeholder for Functional API
# =========================================================================


class Input:
    """Symbolic placeholder that marks the beginning of a model graph.

    Input is NOT a real layer — it's a symbolic node that represents
    the shape of the model's input. It's used with the Functional API:

        inputs = Input(shape=(784,))
        x = Dense(128, activation="relu")(inputs)
        outputs = Dense(10, activation="softmax")(x)
        model = Model(inputs=inputs, outputs=outputs)

    The Input carries metadata (shape) and tracks which layers are
    applied to it, forming a computation graph that Model can replay.

    Args:
        shape: Shape of a single input sample (excluding batch dimension).
        name: Optional name for this input.
    """

    def __init__(self, shape: tuple[int, ...], name: str | None = None) -> None:
        self.shape = shape
        self._name = name or "input"
        # Track the chain of layers applied after this Input
        self._layers: list[Layer] = []
        # A symbolic "output" that layers can be applied to
        self._output_shape = shape

    def __repr__(self) -> str:
        return f"Input(shape={self.shape})"


class _SymbolicTensor:
    """A symbolic tensor that records layers in a computation graph.

    When you call a layer on an Input or _SymbolicTensor, it doesn't
    actually compute anything — it just records the layer and returns
    a new _SymbolicTensor. The Model class later replays this chain
    with real data.
    """

    def __init__(
        self,
        shape: tuple[int, ...],
        source: Input | _SymbolicTensor,
        layer: Layer,
    ) -> None:
        self.shape = shape
        self._source = source
        self._layer = layer


# =========================================================================
# Activation layers (standalone layer versions)
# =========================================================================


class ReLU(Layer):
    """ReLU activation as a standalone layer.

    Sometimes you want to apply an activation as a separate layer
    rather than as part of Dense. This is useful in architectures
    like ResNets where the activation comes after batch normalization.

    Example:
        model = Sequential([
            Dense(128),
            BatchNormalization(),
            ReLU(),              # activation as a separate step
            Dense(10),
        ])
    """

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        from .activations import relu

        return relu(inputs)

    def __repr__(self) -> str:
        return "ReLU()"


class Softmax(Layer):
    """Softmax activation as a standalone layer.

    Args:
        axis: Axis along which to compute softmax. Default: -1.
    """

    def __init__(self, axis: int = -1, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.axis = axis

    def build(self, input_shape: tuple[int, ...] | None) -> None:
        self._built = True

    def call(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        from .activations import softmax

        return softmax(inputs, axis=self.axis)

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["axis"] = self.axis
        return config

    def __repr__(self) -> str:
        return f"Softmax(axis={self.axis})"


# =========================================================================
# Helper
# =========================================================================


def _numel(shape: tuple[int, ...]) -> int:
    """Total number of elements for a given shape."""
    result = 1
    for s in shape:
        result *= s
    return result
