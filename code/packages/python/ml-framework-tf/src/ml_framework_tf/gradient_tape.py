"""
================================================================
GRADIENT TAPE — TENSORFLOW'S EXPLICIT GRADIENT TRACKING
================================================================

This is THE defining feature that separates TensorFlow's API from
PyTorch's. While PyTorch tracks gradients implicitly (any tensor
with requires_grad=True is automatically recorded), TensorFlow
requires you to explicitly open a "tape" that records operations.

=== The Tape Metaphor ===

Think of GradientTape like a cassette tape recorder:
1. You press RECORD (enter the `with` block)
2. All operations on watched tensors are recorded
3. You press STOP (exit the `with` block)
4. You REWIND and compute gradients (call tape.gradient())

    x = tf.Variable([1.0, 2.0, 3.0])

    with tf.GradientTape() as tape:     # press RECORD
        y = x * x                        # recorded: Mul(x, x)
        loss = tf.reduce_sum(y)          # recorded: Sum(y)

    grads = tape.gradient(loss, [x])     # REWIND: compute dx
    # grads = [2.0, 4.0, 6.0]  (derivative of x^2 is 2x)

=== Why Explicit Taping? ===

TensorFlow chose this design because:
1. **Memory control**: Only operations inside the tape are recorded.
   No computation graph builds up during inference.
2. **Higher-order gradients**: You can nest tapes to compute
   gradients of gradients (Hessians, etc.).
3. **Selective watching**: You choose which tensors to differentiate
   with respect to, rather than tracking everything.

=== One-shot vs Persistent ===

By default, a tape is consumed after one gradient() call:
    grads = tape.gradient(loss, [x])    # works
    grads2 = tape.gradient(loss, [y])   # ERROR: tape consumed!

With persistent=True, you can call gradient() multiple times:
    with tf.GradientTape(persistent=True) as tape:
        ...
    grads_x = tape.gradient(loss, [x])  # works
    grads_y = tape.gradient(loss, [y])  # also works!

Persistent tapes use more memory (they keep the graph around),
so only use them when you need multiple gradient() calls.

=== Implementation Detail ===

Under the hood, our GradientTape works by:
1. On __enter__: Ensure all watched Variables have requires_grad=True
2. Inside the tape: Operations on those tensors build the autograd
   computation graph (this happens automatically in ml-framework-core)
3. On gradient(): Call target.backward() and collect .grad from sources

This is a thin wrapper around the core autograd engine — the actual
gradient computation is delegated to Tensor.backward().

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor

from .variable import Variable


class GradientTape:
    """Records operations for automatic differentiation.

    This is TensorFlow's primary mechanism for computing gradients.
    It records operations on watched tensors inside a context manager,
    then computes gradients via the tape.gradient() method.

    Args:
        persistent: If True, the tape can be used for multiple
                    gradient() calls. Default: False (one-shot).

    Example:
        x = tf.Variable([1.0, 2.0, 3.0])

        with tf.GradientTape() as tape:
            y = x ** 2
            loss = tf.reduce_sum(y)

        grads = tape.gradient(loss, [x])
        # grads[0].data == [2.0, 4.0, 6.0]
    """

    def __init__(self, persistent: bool = False) -> None:
        # ─── Tape state ──────────────────────────────────────────
        self._persistent = persistent
        self._used = False

        # List of tensors being watched. Variables with trainable=True
        # are watched automatically; constants must be watched explicitly.
        self._watched: list[Tensor] = []

    def __enter__(self) -> GradientTape:
        """Start recording.

        When the tape is entered, it ensures all watched Variables
        have requires_grad=True so the autograd engine will build
        a computation graph for them.

        Returns self so you can use the `as tape` syntax:
            with tf.GradientTape() as tape:
                ...
        """
        return self

    def __exit__(self, *args: object) -> None:
        """Stop recording.

        In our implementation, there's nothing special to do here
        because the autograd graph is built incrementally during
        forward operations. Real TensorFlow would stop recording
        to its internal trace here.
        """
        pass

    def watch(self, tensor: Tensor) -> None:
        """Explicitly watch a tensor for gradient computation.

        By default, GradientTape only watches tf.Variable objects
        with trainable=True. If you want gradients with respect to
        a tf.constant or non-trainable Variable, you must call watch().

        This is useful for computing gradients of the loss with
        respect to the INPUT (not just the weights):

            x = tf.constant([1.0, 2.0, 3.0])
            with tf.GradientTape() as tape:
                tape.watch(x)          # explicitly watch the constant
                y = x ** 2
                loss = tf.reduce_sum(y)
            grads = tape.gradient(loss, [x])  # now this works

        Args:
            tensor: The tensor to watch for gradient computation.
        """
        tensor.requires_grad = True
        self._watched.append(tensor)

    def gradient(
        self,
        target: Tensor,
        sources: list[Tensor | Variable],
    ) -> list[Tensor | None]:
        """Compute gradients of target with respect to sources.

        This is the payoff — after recording operations, you ask
        "how does the target change when I change each source?"

        The answer is computed via reverse-mode automatic differentiation
        (backpropagation), which walks the computation graph backward
        from target to each source.

        Args:
            target: The tensor to differentiate (usually a scalar loss).
            sources: List of tensors to compute gradients for
                     (usually model Variables).

        Returns:
            List of gradient tensors, one per source. If a source
            has no gradient path to the target, its entry is None.

        Raises:
            RuntimeError: If the tape was already consumed (non-persistent).

        Example:
            grads = tape.gradient(loss, [w1, w2, b])
            # grads[0] = dloss/dw1
            # grads[1] = dloss/dw2
            # grads[2] = dloss/db
        """
        # ─── Check if tape is consumed ───────────────────────────
        if self._used and not self._persistent:
            raise RuntimeError(
                "GradientTape.gradient() can only be called once on a "
                "non-persistent tape. Set persistent=True if you need "
                "to call gradient() multiple times."
            )
        self._used = True

        # ─── Ensure sources have requires_grad=True ──────────────
        # Variables with trainable=True already have this set, but
        # explicitly watched tensors might not if watch() was called
        # after some operations.
        for source in sources:
            source.requires_grad = True

        # ─── Clear any existing gradients ────────────────────────
        # This prevents accumulation from previous backward() calls.
        for source in sources:
            source.grad = None

        # ─── Run backward pass ───────────────────────────────────
        # This delegates to ml-framework-core's autograd engine,
        # which walks the computation graph in reverse topological
        # order, computing gradients via the chain rule.
        target.backward()

        # ─── Collect gradients from each source ──────────────────
        # After backward(), each source tensor's .grad field holds
        # its gradient (or None if there's no path from target).
        return [s.grad for s in sources]
