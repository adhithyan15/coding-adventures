"""
================================================================
AUTOGRAD ENGINE — AUTOMATIC DIFFERENTIATION VIA COMPUTATION GRAPHS
================================================================

This is the engine that makes "loss.backward()" work. It implements
reverse-mode automatic differentiation (backpropagation).

=== How It Works ===

Every time you do an operation on tensors (add, multiply, matmul, etc.),
the autograd engine secretly records what happened:

    x = Tensor([1, 2, 3], requires_grad=True)
    y = x * 2        # Records: MulFunction(x, 2) → y
    z = y.sum()       # Records: SumFunction(y) → z

This creates a DAG (Directed Acyclic Graph):

    x  ──→  [Mul by 2]  ──→  y  ──→  [Sum]  ──→  z

When you call z.backward(), the engine:

1. Starts at z with gradient = 1.0 (∂z/∂z = 1)
2. Calls SumFunction.backward(grad=1.0) → sends grad to y
3. Calls MulFunction.backward(grad from y) → sends grad to x
4. Stores final gradient in x.grad

=== The Function Base Class ===

Each differentiable operation subclasses Function and implements:
- forward(*inputs) → compute the output
- backward(grad_output) → compute gradient w.r.t. each input

The chain rule connects them: if z = f(y) and y = g(x), then
    ∂z/∂x = ∂z/∂y · ∂y/∂x

Each Function provides the local ∂output/∂input, and the engine
chains them together automatically.
================================================================
"""

from __future__ import annotations

from typing import Any

from .tensor import Tensor


class Function:
    """
    ================================================================
    BASE CLASS FOR ALL DIFFERENTIABLE OPERATIONS
    ================================================================

    Every autograd operation (add, mul, matmul, relu, etc.) is a
    Function subclass. It has two halves:

    1. forward(*inputs) → output: compute the result
    2. backward(grad_output) → grad_inputs: compute local gradients

    The apply() classmethod handles the bookkeeping:
    - Calls forward() to compute the result
    - If any input requires_grad, attaches this Function as grad_fn
    - Saves whatever backward() needs (input tensors, shapes, etc.)

    Subclasses store saved tensors in self.saved_tensors and any
    other metadata they need for backward.
    ================================================================
    """

    def __init__(self) -> None:
        self.saved_tensors: list[Tensor] = []
        self.saved_metadata: dict[str, Any] = {}
        self._input_requires_grad: list[bool] = []

    def save_for_backward(self, *tensors: Tensor) -> None:
        """Save tensors needed for the backward pass."""
        self.saved_tensors = list(tensors)

    @classmethod
    def apply(cls, *args: Any) -> Tensor:
        """
        Run the forward pass and wire up the computation graph.

        This is the entry point for all autograd operations. It:
        1. Creates a Function instance
        2. Calls forward() to compute the result
        3. If gradient tracking is needed, sets result._grad_fn
        """
        fn = cls()

        # Separate tensor inputs from non-tensor args
        tensor_inputs = [a for a in args if isinstance(a, Tensor)]
        needs_grad = any(t.requires_grad for t in tensor_inputs)

        # Track which inputs need gradients
        fn._input_requires_grad = [
            isinstance(a, Tensor) and a.requires_grad for a in args
        ]

        # Run forward
        result = fn.forward(*args)

        # Wire up computation graph
        if needs_grad:
            result.requires_grad = True
            result._grad_fn = fn

        return result

    def forward(self, *args: Any) -> Tensor:
        """Compute the forward pass. Subclasses must override."""
        raise NotImplementedError

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        """Compute gradients. Subclasses must override.

        Returns one gradient per forward() argument (or None if that
        argument doesn't need a gradient).
        """
        raise NotImplementedError

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__}>"


# =========================================================================
# The backward() algorithm — topological sort + reverse walk
# =========================================================================


def backward(tensor: Tensor, gradient: Tensor | None = None) -> None:
    """
    ================================================================
    REVERSE-MODE AUTOMATIC DIFFERENTIATION
    ================================================================

    This function implements backpropagation. Given a tensor (usually
    a scalar loss), it computes gradients for all leaf tensors that
    contributed to it.

    Algorithm:
    1. Start with the output tensor and its gradient (default: 1.0)
    2. Topological-sort all nodes in the computation graph
    3. Walk in reverse order, calling each node's backward()
    4. Accumulate gradients at each node

    This is O(V + E) where V is the number of operations and E is
    the number of tensor edges in the graph.
    ================================================================
    """
    if not tensor.requires_grad:
        raise RuntimeError(
            "backward() called on a tensor that doesn't require grad"
        )

    # Default gradient is all-ones (∂loss/∂loss = 1)
    if gradient is None:
        if tensor.numel != 1:
            raise RuntimeError(
                "backward() requires a gradient argument for non-scalar tensors"
            )
        gradient = Tensor.ones(*tensor.shape, device=tensor.device)

    # ─── Step 1: Topological sort ───────────────────────────────────
    # We need to process nodes in reverse topological order so that
    # when we compute a node's backward, all downstream gradients
    # have already been accumulated.

    topo_order: list[Tensor] = []
    visited: set[int] = set()

    def _build_topo(t: Tensor) -> None:
        tid = id(t)
        if tid in visited:
            return
        visited.add(tid)
        if t._grad_fn is not None:
            for saved in t._grad_fn.saved_tensors:
                _build_topo(saved)
        topo_order.append(t)

    _build_topo(tensor)

    # ─── Step 2: Reverse walk ───────────────────────────────────────
    # Each tensor accumulates its gradient from all paths that lead to it.

    grad_map: dict[int, Tensor] = {id(tensor): gradient}

    for node in reversed(topo_order):
        node_grad = grad_map.get(id(node))
        if node_grad is None:
            continue

        # If this is a leaf node, store the gradient
        if node._grad_fn is None:
            if node.requires_grad:
                if node.grad is None:
                    node.grad = Tensor(
                        list(node_grad.data),
                        node_grad.shape,
                        device=node.device,
                    )
                else:
                    # Accumulate (for shared parameters)
                    pairs = zip(
                        node.grad.data, node_grad.data, strict=False
                    )
                    node.grad = Tensor(
                        [a + b for a, b in pairs],
                        node.grad.shape,
                        device=node.device,
                    )
            continue

        # Call backward on the Function
        input_grads = node._grad_fn.backward(node_grad)

        # Distribute gradients to saved tensors
        saved = node._grad_fn.saved_tensors
        for saved_tensor, input_grad in zip(saved, input_grads, strict=False):
            if input_grad is None:
                continue
            tid = id(saved_tensor)
            if tid in grad_map:
                # Accumulate gradients from multiple paths
                existing = grad_map[tid]
                pairs = zip(
                    existing.data, input_grad.data, strict=False
                )
                grad_map[tid] = Tensor(
                    [a + b for a, b in pairs],
                    existing.shape,
                    device=existing.device,
                )
            else:
                grad_map[tid] = input_grad


# =========================================================================
# no_grad context manager — disable gradient tracking
# =========================================================================


class _NoGradContext:
    """Context manager that disables gradient tracking.

    Usage:
        with no_grad():
            output = model(input)  # No computation graph built
    """

    _enabled = False

    def __enter__(self) -> None:
        _NoGradContext._enabled = True

    def __exit__(self, *args: object) -> None:
        _NoGradContext._enabled = False


def no_grad() -> _NoGradContext:
    """Returns a context manager that disables gradient tracking."""
    return _NoGradContext()


def is_grad_enabled() -> bool:
    """Check if gradient tracking is currently enabled."""
    return not _NoGradContext._enabled
