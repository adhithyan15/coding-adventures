"""
================================================================
SEQUENTIAL — A CONTAINER THAT CHAINS LAYERS IN ORDER
================================================================

Sequential is the simplest way to build a neural network. It takes
a list of layers and applies them one after another:

    model = Sequential(
        Linear(784, 256),    # input → hidden
        ReLU(),              # activation
        Linear(256, 10),     # hidden → output
    )

    output = model(input)
    # Equivalent to:
    # h = Linear(784, 256)(input)
    # h = ReLU()(h)
    # output = Linear(256, 10)(h)

=== Why Sequential Exists ===

Without Sequential, you'd have to write a custom Module:

    class MyModel(Module):
        def __init__(self):
            super().__init__()
            self.fc1 = Linear(784, 256)
            self.relu = ReLU()
            self.fc2 = Linear(256, 10)

        def forward(self, x):
            x = self.fc1(x)
            x = self.relu(x)
            x = self.fc2(x)
            return x

Sequential automates this pattern. For more complex architectures
(skip connections, branches), you still need custom Modules.

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor

from .module import Module


class Sequential(Module):
    """A sequential container that applies layers in order.

    Layers are stored as numbered children (0, 1, 2, ...) and
    applied left-to-right in forward().

    Args:
        *layers: Variable number of Module instances to chain together.

    Example:
        model = Sequential(
            Linear(10, 20),
            ReLU(),
            Linear(20, 5),
        )
        output = model(torch.randn(3, 10))  # (3, 5)
    """

    def __init__(self, *layers: Module) -> None:
        super().__init__()
        # Register each layer as a numbered child module.
        # Using str(i) as the key matches PyTorch convention:
        # model._modules = {"0": Linear, "1": ReLU, "2": Linear}
        for i, layer in enumerate(layers):
            # This triggers __setattr__, which registers in _modules
            setattr(self, str(i), layer)

    def forward(self, x: Tensor) -> Tensor:
        """Apply each layer in sequence.

        This is the "pipe" operation:
            x → layer0 → layer1 → ... → layerN → output
        """
        for module in self._modules.values():
            x = module(x)
        return x

    def __repr__(self) -> str:
        lines = ["Sequential("]
        for name, module in self._modules.items():
            lines.append(f"  ({name}): {module!r}")
        lines.append(")")
        return "\n".join(lines)

    def __len__(self) -> int:
        """Return the number of layers."""
        return len(self._modules)

    def __getitem__(self, idx: int) -> Module:
        """Access a layer by index."""
        return self._modules[str(idx)]
