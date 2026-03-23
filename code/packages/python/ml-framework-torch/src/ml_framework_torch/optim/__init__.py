"""
================================================================
OPTIMIZERS — PARAMETER UPDATE ALGORITHMS
================================================================

Optimizers take the gradients computed by backward() and use them
to update model parameters. This is the "learning" in machine learning.

The simplest optimizer is SGD (Stochastic Gradient Descent):
    new_weight = old_weight - learning_rate * gradient

More advanced optimizers (Adam, RMSprop) adapt the learning rate
per-parameter based on gradient history.

Usage:
    optimizer = optim.SGD(model.parameters(), lr=0.01)
    # or
    optimizer = optim.Adam(model.parameters(), lr=0.001)

    # Training loop:
    for batch in data:
        optimizer.zero_grad()       # clear old gradients
        loss = model(batch)         # forward pass
        loss.backward()             # compute gradients
        optimizer.step()            # update parameters

================================================================
"""

from .adam import Adam, AdamW
from .optimizer import Optimizer
from .rmsprop import RMSprop
from .sgd import SGD

__all__ = [
    "Optimizer",
    "SGD",
    "Adam",
    "AdamW",
    "RMSprop",
]
