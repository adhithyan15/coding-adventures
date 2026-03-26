"""
================================================================
NEURAL NETWORK MODULES (nn)
================================================================

This sub-package mirrors PyTorch's torch.nn namespace. It provides:

1. Module — base class for all layers (handles parameter registration)
2. Linear — fully-connected layer (y = x @ W.T + b)
3. Activation layers — ReLU, GELU, Sigmoid, Tanh, Softmax, LogSoftmax
4. Sequential — chain layers into a pipeline
5. Loss functions — MSELoss, CrossEntropyLoss, BCELoss, etc.
6. Normalization — BatchNorm1d, LayerNorm
7. Dropout — regularization by randomly zeroing elements
8. Embedding — lookup table for discrete tokens (like words)
9. Flatten — reshape multi-dim input to 2-D
10. functional — stateless versions of all operations (F.relu, etc.)

Usage:
    import ml_framework_torch as torch

    model = torch.nn.Sequential(
        torch.nn.Linear(784, 128),
        torch.nn.ReLU(),
        torch.nn.Linear(128, 10),
    )

    output = model(input_tensor)  # calls forward() on each layer
================================================================
"""

from .activation import GELU, ReLU, Sigmoid, Softmax, Tanh, LogSoftmax
from .dropout import Dropout
from .embedding import Embedding
from .flatten import Flatten
from .linear import Linear
from .loss import (
    BCELoss,
    BCEWithLogitsLoss,
    CrossEntropyLoss,
    L1Loss,
    MSELoss,
    NLLLoss,
)
from .module import Module
from .normalization import BatchNorm1d, LayerNorm
from .sequential import Sequential

__all__ = [
    # Base
    "Module",
    "Sequential",
    # Layers
    "Linear",
    "Flatten",
    "Embedding",
    "Dropout",
    # Activations
    "ReLU",
    "GELU",
    "Sigmoid",
    "Tanh",
    "Softmax",
    "LogSoftmax",
    # Normalization
    "BatchNorm1d",
    "LayerNorm",
    # Loss
    "MSELoss",
    "CrossEntropyLoss",
    "BCELoss",
    "BCEWithLogitsLoss",
    "L1Loss",
    "NLLLoss",
]
