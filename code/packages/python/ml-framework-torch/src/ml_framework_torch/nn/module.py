"""
================================================================
MODULE — THE BASE CLASS FOR ALL NEURAL NETWORK LAYERS
================================================================

Every neural network layer in PyTorch inherits from Module. It's the
fundamental building block that provides:

1. **Parameter registration** — When you assign a Parameter as an
   attribute, Module automatically tracks it. This is how
   model.parameters() knows about all weights and biases.

2. **Submodule registration** — When you assign another Module as
   an attribute, it becomes a child. model.parameters() recurses
   into children, finding all learnable weights in the entire tree.

3. **Training/eval mode** — model.train() vs model.eval() controls
   behavior of Dropout and BatchNorm.

4. **Device management** — model.to("cuda") moves all parameters.

5. **State dict** — model.state_dict() serializes all parameters
   for saving/loading checkpoints.

=== How __setattr__ Magic Works ===

The key trick is overriding __setattr__. In Python, every time you do:
    self.weight = Parameter(...)

Python calls self.__setattr__("weight", parameter). Our override
intercepts this and registers the Parameter in self._parameters.
Similarly for Module children → self._modules.

This is why you can just write:
    class MyLayer(Module):
        def __init__(self):
            super().__init__()
            self.weight = Parameter(Tensor.randn(10, 5))  # auto-registered!

=== IMPORTANT IMPLEMENTATION DETAIL ===

We must initialize _modules, _parameters, and training using
object.__setattr__() in __init__ BEFORE any regular assignments.
Otherwise, our custom __setattr__ would try to access _parameters
before it exists, causing infinite recursion.

================================================================
"""

from __future__ import annotations

from collections.abc import Iterator

from ml_framework_core import Parameter, Tensor


class Module:
    """Base class for all neural network layers.

    Subclasses must implement forward(*args) → Tensor.
    __call__ delegates to forward, mirroring PyTorch's behavior.

    Example:
        class MyLinear(Module):
            def __init__(self, in_features, out_features):
                super().__init__()
                self.weight = Parameter(Tensor.randn(out_features, in_features))
                self.bias = Parameter(Tensor.zeros(out_features))

            def forward(self, x):
                return x @ self.weight.t() + self.bias

        layer = MyLinear(784, 128)
        output = layer(input_tensor)  # calls forward()
        list(layer.parameters())      # [weight, bias]
    """

    def __init__(self) -> None:
        # ─── Bootstrap: use object.__setattr__ to avoid recursion ───
        # Our __setattr__ checks self._parameters and self._modules,
        # so they must exist before any regular attribute assignment.
        # This is the same pattern PyTorch uses internally.
        object.__setattr__(self, "_parameters", {})
        object.__setattr__(self, "_modules", {})
        object.__setattr__(self, "training", True)

    # =================================================================
    # Forward pass — subclasses override this
    # =================================================================

    def forward(self, *args: Tensor) -> Tensor:
        """Compute the layer's output. Subclasses must implement this.

        This is where the actual math happens: matrix multiplies,
        activations, normalization, etc.
        """
        raise NotImplementedError(f"{self.__class__.__name__} must implement forward()")

    def __call__(self, *args: Tensor) -> Tensor:
        """Make the module callable: model(x) calls model.forward(x).

        In real PyTorch, __call__ also handles hooks (pre-forward,
        post-forward) for debugging and monitoring. We keep it simple.
        """
        return self.forward(*args)

    # =================================================================
    # Attribute registration magic
    # =================================================================

    def __setattr__(self, name: str, value: object) -> None:
        """Auto-register Parameters and child Modules.

        This is the magic that makes parameter tracking "just work".
        When you write self.weight = Parameter(...), this method:
        1. Detects that value is a Parameter
        2. Stores it in self._parameters["weight"]
        3. Also stores it as a regular attribute (so self.weight works)

        Similarly for Module children → self._modules.

        For None values (like optional bias), we skip registration
        but still set the attribute normally.
        """
        # ─── Register Parameters ───────────────────────────────────
        if isinstance(value, Parameter):
            self._parameters[name] = value
        # ─── Register child Modules (but not Parameters, which are
        #      Tensor subclasses and wouldn't be Module instances) ──
        elif isinstance(value, Module):
            self._modules[name] = value

        # Always set the attribute normally so self.name works
        object.__setattr__(self, name, value)

    # =================================================================
    # Parameter iteration
    # =================================================================

    def parameters(self) -> Iterator[Parameter]:
        """Yield all learnable parameters in this module and its children.

        This is what the optimizer uses to know which tensors to update:
            optimizer = SGD(model.parameters(), lr=0.01)

        It works recursively: a Sequential containing two Linear layers
        will yield all four parameters (2 weights + 2 biases).
        """
        # First, yield our own parameters
        yield from self._parameters.values()
        # Then, recurse into child modules
        for module in self._modules.values():
            yield from module.parameters()

    def named_parameters(self, prefix: str = "") -> Iterator[tuple[str, Parameter]]:
        """Yield (name, parameter) pairs with dotted path names.

        Example:
            model = Sequential(Linear(2, 3), Linear(3, 1))
            list(model.named_parameters())
            # [("0.weight", ...), ("0.bias", ...), ("1.weight", ...), ("1.bias", ...)]

        The prefix argument is used internally for recursion:
            - Top level: prefix="" → "weight"
            - One level deep: prefix="0." → "0.weight"
        """
        for name, param in self._parameters.items():
            yield f"{prefix}{name}", param
        for mod_name, module in self._modules.items():
            yield from module.named_parameters(prefix=f"{prefix}{mod_name}.")

    def named_modules(self, prefix: str = "") -> Iterator[tuple[str, Module]]:
        """Yield (name, module) pairs for this module and all descendants.

        Useful for inspecting model architecture or applying transformations
        to specific layer types.
        """
        yield prefix, self
        for name, module in self._modules.items():
            full_name = f"{prefix}.{name}" if prefix else name
            yield from module.named_modules(prefix=full_name)

    # =================================================================
    # Training / evaluation mode
    # =================================================================

    def train(self, mode: bool = True) -> Module:
        """Set training mode (affects Dropout, BatchNorm, etc.).

        In training mode:
        - Dropout randomly zeros elements
        - BatchNorm uses batch statistics

        In eval mode:
        - Dropout passes through unchanged
        - BatchNorm uses running statistics
        """
        self.training = mode
        for module in self._modules.values():
            module.train(mode)
        return self

    def eval(self) -> Module:
        """Switch to evaluation mode. Shortcut for self.train(False)."""
        return self.train(False)

    # =================================================================
    # Device management
    # =================================================================

    def to(self, device: str) -> Module:
        """Move all parameters to a different device.

        Example: model.to("cuda") moves weights to GPU.

        This creates new Parameter objects with data on the target device.
        Since our Tensor.to() copies data, the old parameters remain
        on the original device (matching PyTorch's in-place semantics
        would require mutable tensors).
        """
        for name, param in self._parameters.items():
            new_param = Parameter(param.to(device))
            self._parameters[name] = new_param
            object.__setattr__(self, name, new_param)
        for module in self._modules.values():
            module.to(device)
        return self

    # =================================================================
    # Gradient management
    # =================================================================

    def zero_grad(self) -> None:
        """Reset all parameter gradients to None.

        Called before each training step to prevent gradient accumulation
        from previous iterations:

            optimizer.zero_grad()   # or model.zero_grad()
            loss = model(x)
            loss.backward()         # fresh gradients
            optimizer.step()
        """
        for p in self.parameters():
            p.grad = None

    # =================================================================
    # State dict (serialization)
    # =================================================================

    def state_dict(self) -> dict[str, Tensor]:
        """Export all parameters as a flat dictionary.

        Returns a dict mapping dotted parameter names to Tensors:
            {"weight": Tensor(...), "bias": Tensor(...)}

        For nested models:
            {"0.weight": ..., "0.bias": ..., "1.weight": ..., "1.bias": ...}

        This is used for saving model checkpoints.
        """
        state: dict[str, Tensor] = {}
        for name, param in self.named_parameters():
            state[name] = param
        return state

    def load_state_dict(self, state: dict[str, Tensor]) -> None:
        """Load parameters from a state dictionary.

        This restores weights from a saved checkpoint. It walks the
        state dict and copies data into matching parameters.
        """
        param_dict = dict(self.named_parameters())
        for name, value in state.items():
            if name in param_dict:
                param = param_dict[name]
                param.data = list(value.data)
                param.shape = value.shape

    # =================================================================
    # Display
    # =================================================================

    def __repr__(self) -> str:
        """Pretty-print the module tree, similar to PyTorch."""
        lines = [f"{self.__class__.__name__}("]
        for name, module in self._modules.items():
            mod_str = repr(module).replace("\n", "\n  ")
            lines.append(f"  ({name}): {mod_str}")
        if self._modules:
            lines.append(")")
        else:
            # Single-line for leaf modules
            return f"{self.__class__.__name__}()"
        return "\n".join(lines)
