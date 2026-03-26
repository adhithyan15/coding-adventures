"""
================================================================
DROPOUT — REGULARIZATION BY RANDOM ZEROING
================================================================

Dropout is a simple but powerful regularization technique. During
training, it randomly sets some elements to zero with probability p:

    Input:  [1.0, 2.0, 3.0, 4.0, 5.0]
    Mask:   [1,   0,   1,   0,   1  ]  (random, p=0.4)
    Output: [1.0, 0.0, 3.0, 0.0, 5.0]

=== Why Dropout Works ===

By randomly "killing" neurons during training:
1. The network can't rely on any single neuron → more robust features
2. It's like training an ensemble of sub-networks simultaneously
3. At test time, all neurons are active → ensemble averaging

=== The Scaling Trick ===

If we drop 30% of neurons during training, the remaining 70% produce
smaller total activations. At test time (no dropout), activations
would be ~1.43x larger, causing a train/test mismatch.

Solution: scale surviving values by 1/(1-p) during training:
    Output: [1.43, 0.0, 4.29, 0.0, 7.14]  (divided by 0.7)

This way, expected values match between train and test.
This is called "inverted dropout."

================================================================
"""

from __future__ import annotations

import random

from ml_framework_core import Tensor

from .module import Module


class Dropout(Module):
    """Randomly zeros elements during training for regularization.

    Args:
        p: Probability of zeroing each element. Default: 0.5

    During evaluation (self.training = False), Dropout is a no-op:
    input passes through unchanged.

    Example:
        dropout = Dropout(p=0.3)
        x = Tensor.ones(2, 4)

        dropout.train()
        y = dropout(x)   # ~30% of elements are zero, rest scaled by 1/0.7

        dropout.eval()
        y = dropout(x)   # identity — same as input
    """

    def __init__(self, p: float = 0.5) -> None:
        super().__init__()
        if not 0.0 <= p < 1.0:
            raise ValueError(f"Dropout probability must be in [0, 1), got {p}")
        object.__setattr__(self, "p", p)

    def forward(self, x: Tensor) -> Tensor:
        """Apply dropout during training, identity during eval.

        During training:
        1. Generate a random mask (Bernoulli with probability 1-p)
        2. Zero out masked elements
        3. Scale survivors by 1/(1-p) to maintain expected value
        """
        # ─── Eval mode: no dropout ──────────────────────────────
        if not self.training:
            return x

        # ─── Training mode: apply inverted dropout ──────────────
        # Special case: p=0 means no dropout at all
        if self.p == 0.0:
            return x

        scale = 1.0 / (1.0 - self.p)
        data = []
        for val in x.data:
            if random.random() < self.p:
                data.append(0.0)  # dropped!
            else:
                data.append(val * scale)  # scaled up

        return Tensor(data, x.shape, device=x.device)

    def __repr__(self) -> str:
        return f"Dropout(p={self.p})"
