"""
================================================================
NORMALIZATION LAYERS — KEEPING ACTIVATIONS WELL-BEHAVED
================================================================

As data flows through deep networks, activation distributions can
shift dramatically (a phenomenon called "internal covariate shift").
Normalization layers fix this by re-centering and re-scaling
activations to have mean ≈ 0 and variance ≈ 1.

=== BatchNorm ===

Normalizes across the BATCH dimension:
    For each feature: mean and variance computed over all samples

    Input shape: (batch, features)
    Statistics: computed over dim 0 (batch)

    normalized = (x - mean) / sqrt(var + eps)
    output = gamma * normalized + beta

Where gamma (scale) and beta (shift) are learnable parameters.
This lets the network undo the normalization if it wants to.

During training: uses batch statistics
During evaluation: uses running averages collected during training

=== LayerNorm ===

Normalizes across the FEATURE dimension (within each sample):
    For each sample: mean and variance computed over all features

    Input shape: (batch, features)
    Statistics: computed over dim 1 (features)

LayerNorm is preferred in transformers because it doesn't depend
on batch size and works the same in training and evaluation.

================================================================
"""

from __future__ import annotations

import math

from ml_framework_core import Parameter, Tensor

from .module import Module


class BatchNorm1d(Module):
    """Batch Normalization over a 1-D input (batch of feature vectors).

    For each feature, computes:
        y = gamma * (x - mean) / sqrt(var + eps) + beta

    Args:
        num_features: Number of features (size of dim 1)
        eps: Small constant for numerical stability. Default: 1e-5
        momentum: Factor for running statistics update. Default: 0.1

    The running mean and variance are updated during training:
        running_mean = (1 - momentum) * running_mean + momentum * batch_mean
        running_var  = (1 - momentum) * running_var  + momentum * batch_var

    Example:
        bn = BatchNorm1d(64)
        x = Tensor.randn(32, 64)  # batch of 32, 64 features each
        y = bn(x)                 # normalized output, same shape
    """

    def __init__(
        self,
        num_features: int,
        eps: float = 1e-5,
        momentum: float = 0.1,
    ) -> None:
        super().__init__()
        object.__setattr__(self, "num_features", num_features)
        object.__setattr__(self, "eps", eps)
        object.__setattr__(self, "momentum", momentum)

        # Learnable parameters: scale (gamma) and shift (beta)
        # Initialized to gamma=1, beta=0 so the layer starts as identity
        self.weight = Parameter(Tensor.ones(num_features))  # gamma
        self.bias = Parameter(Tensor.zeros(num_features))  # beta

        # Running statistics (not learnable, not Parameters)
        # These track the global mean/var across all training batches
        object.__setattr__(self, "running_mean", Tensor.zeros(num_features))
        object.__setattr__(self, "running_var", Tensor.ones(num_features))

    def forward(self, x: Tensor) -> Tensor:
        """Normalize input using batch or running statistics.

        Shape: (batch_size, num_features) → (batch_size, num_features)

        During training:
            1. Compute mean and var across the batch (dim 0)
            2. Normalize: (x - mean) / sqrt(var + eps)
            3. Scale and shift: gamma * normalized + beta
            4. Update running statistics

        During eval:
            1. Use running_mean and running_var instead of batch stats
            2. Normalize, scale, and shift as above
        """
        if x.ndim != 2:
            raise ValueError(
                f"BatchNorm1d expects 2-D input (batch, features), got {x.ndim}-D"
            )

        batch_size, features = x.shape
        if features != self.num_features:
            raise ValueError(f"Expected {self.num_features} features, got {features}")

        if self.training:
            # ─── Compute batch statistics ───────────────────────
            # Mean: average each feature across the batch
            mean = [0.0] * features
            for i in range(batch_size):
                for j in range(features):
                    mean[j] += x.data[i * features + j]
            mean = [m / batch_size for m in mean]

            # Variance: average squared deviation from mean
            var = [0.0] * features
            for i in range(batch_size):
                for j in range(features):
                    diff = x.data[i * features + j] - mean[j]
                    var[j] += diff * diff
            var = [v / batch_size for v in var]

            # ─── Update running statistics ──────────────────────
            mom = self.momentum
            new_rm = [
                (1 - mom) * r + mom * m for r, m in zip(self.running_mean.data, mean)
            ]
            new_rv = [
                (1 - mom) * r + mom * v for r, v in zip(self.running_var.data, var)
            ]
            object.__setattr__(
                self,
                "running_mean",
                Tensor(new_rm, (features,)),
            )
            object.__setattr__(
                self,
                "running_var",
                Tensor(new_rv, (features,)),
            )
        else:
            # ─── Use running statistics ─────────────────────────
            mean = list(self.running_mean.data)
            var = list(self.running_var.data)

        # ─── Normalize, scale, shift ────────────────────────────
        result = [0.0] * (batch_size * features)
        for i in range(batch_size):
            for j in range(features):
                idx = i * features + j
                normalized = (x.data[idx] - mean[j]) / math.sqrt(var[j] + self.eps)
                result[idx] = self.weight.data[j] * normalized + self.bias.data[j]

        return Tensor(result, x.shape, device=x.device)

    def __repr__(self) -> str:
        return (
            f"BatchNorm1d({self.num_features}, "
            f"eps={self.eps}, momentum={self.momentum})"
        )


class LayerNorm(Module):
    """Layer Normalization over the last dimension.

    For each sample independently, computes:
        y = gamma * (x - mean) / sqrt(var + eps) + beta

    Where mean and var are computed over the feature dimension.

    Args:
        normalized_shape: Size of the feature dimension to normalize
        eps: Small constant for numerical stability. Default: 1e-5

    Unlike BatchNorm:
    - Statistics are per-sample (not per-batch)
    - No running statistics needed
    - Same behavior in training and evaluation
    - Preferred in transformers and NLP models

    Example:
        ln = LayerNorm(512)
        x = Tensor.randn(8, 512)  # 8 samples, 512 features
        y = ln(x)                 # normalized per-sample
    """

    def __init__(
        self,
        normalized_shape: int,
        eps: float = 1e-5,
    ) -> None:
        super().__init__()
        object.__setattr__(self, "normalized_shape", normalized_shape)
        object.__setattr__(self, "eps", eps)

        # Learnable affine parameters
        self.weight = Parameter(Tensor.ones(normalized_shape))  # gamma
        self.bias = Parameter(Tensor.zeros(normalized_shape))  # beta

    def forward(self, x: Tensor) -> Tensor:
        """Normalize each sample across its feature dimension.

        For 2-D input (batch, features):
            For each row i:
                mean_i = mean(x[i, :])
                var_i = var(x[i, :])
                y[i, j] = gamma[j] * (x[i,j] - mean_i) / sqrt(var_i + eps) + beta[j]
        """
        if x.ndim != 2:
            raise ValueError(f"LayerNorm expects 2-D input, got {x.ndim}-D")

        batch_size, features = x.shape
        if features != self.normalized_shape:
            raise ValueError(
                f"Expected {self.normalized_shape} features, got {features}"
            )

        result = [0.0] * (batch_size * features)

        for i in range(batch_size):
            row_start = i * features
            row = x.data[row_start : row_start + features]

            # Compute mean of this sample's features
            mean = sum(row) / features

            # Compute variance of this sample's features
            var = sum((v - mean) ** 2 for v in row) / features

            # Normalize, scale, shift
            inv_std = 1.0 / math.sqrt(var + self.eps)
            for j in range(features):
                normalized = (row[j] - mean) * inv_std
                result[row_start + j] = (
                    self.weight.data[j] * normalized + self.bias.data[j]
                )

        return Tensor(result, x.shape, device=x.device)

    def __repr__(self) -> str:
        return f"LayerNorm({self.normalized_shape}, eps={self.eps})"
