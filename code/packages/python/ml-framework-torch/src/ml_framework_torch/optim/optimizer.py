"""
================================================================
OPTIMIZER BASE CLASS — THE INTERFACE FOR ALL OPTIMIZERS
================================================================

All optimizers share a common interface:
    optimizer.zero_grad()  — clear gradients
    optimizer.step()       — update parameters using gradients

The Optimizer base class stores a reference to the model's parameters
and provides zero_grad(). Subclasses implement step() with their
specific update rule.

=== How Parameter Updates Work ===

Our Tensors store data as a flat list[float]. To update a parameter
"in place" (without breaking the autograd graph), we directly
replace the data list:

    # Instead of: param = param - lr * grad  (creates new tensor)
    # We do: param.data = [w - lr*g for w, g in zip(param.data, grad.data)]

This modifies the parameter's values while keeping the same Python
object, so all references to it remain valid.

================================================================
"""

from __future__ import annotations

from collections.abc import Iterator

from ml_framework_core import Parameter


class Optimizer:
    """Base class for all optimizers.

    Args:
        params: Iterator of Parameters to optimize (from model.parameters())
        lr: Learning rate — how big of a step to take. Default: 0.01

    The learning rate is the most important hyperparameter in training.
    Too high → loss oscillates or diverges.
    Too low → training is painfully slow.
    """

    def __init__(
        self,
        params: Iterator[Parameter] | list[Parameter],
        lr: float = 0.01,
    ) -> None:
        # Materialize the iterator into a list so we can iterate multiple times
        self.params: list[Parameter] = list(params)
        self.lr = lr

    def zero_grad(self) -> None:
        """Reset all parameter gradients to None.

        Must be called before each backward() pass, otherwise gradients
        accumulate across iterations:

            Step 1: grad = ∂L₁/∂w
            Step 2: grad = ∂L₁/∂w + ∂L₂/∂w  ← wrong! (if not zeroed)
        """
        for p in self.params:
            p.grad = None

    def step(self) -> None:
        """Update parameters using their gradients.

        Subclasses must override this with their specific update rule.
        """
        raise NotImplementedError(f"{self.__class__.__name__} must implement step()")
