"""
================================================================
VARIABLE — MUTABLE TENSOR WITH NAME AND TRAINABILITY
================================================================

In TensorFlow, a Variable is the primary way to hold mutable state
that persists across calls to a computation. Think of it as a named
container for a Tensor whose value can change over time.

=== Variables vs Constants ===

TensorFlow distinguishes between:
- **tf.constant**: Immutable. Once created, the value never changes.
  Used for input data, hyperparameters, etc.
- **tf.Variable**: Mutable. Can be updated in-place via assign().
  Used for model weights and biases.

This distinction matters for training:
    weight = tf.Variable([1.0, 2.0, 3.0])   # mutable — optimizer updates this
    input_data = tf.constant([4.0, 5.0, 6.0])  # immutable — just data

=== The trainable Flag ===

A Variable can be trainable (default) or non-trainable:
- **trainable=True**: GradientTape watches it automatically, and the
  optimizer updates it during training. Used for weights and biases.
- **trainable=False**: Not tracked by GradientTape. Used for things
  like step counters, running statistics in BatchNorm, etc.

=== In-place Mutation ===

Unlike PyTorch (where you modify param.data directly), TensorFlow
uses explicit mutation methods:
    var.assign([10.0, 20.0, 30.0])     # replace value entirely
    var.assign_add([1.0, 1.0, 1.0])    # add in-place: [11, 21, 31]
    var.assign_sub([0.5, 0.5, 0.5])    # subtract in-place: [10.5, 20.5, 30.5]

These methods update the Variable's underlying data without creating
a new Python object, which is important when other code holds references
to the same Variable (like the optimizer's parameter list).

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor

# =========================================================================
# A counter for auto-generating unique Variable names.
# In real TensorFlow, names are used for debugging, serialization,
# and the TensorBoard visualization tool.
# =========================================================================
_variable_counter = 0


class Variable(Tensor):
    """A mutable, named Tensor tracked by GradientTape when trainable.

    This is TensorFlow's equivalent of a learnable parameter. Unlike
    a plain Tensor (which is immutable in TF semantics), a Variable
    can be updated in-place during training.

    Args:
        initial_value: A Tensor or list providing the starting value.
        trainable: Whether GradientTape should track gradients for this
                   Variable. Default: True (it's a learnable weight).
        name: Optional human-readable name. If None, auto-generated
              as "Variable:0", "Variable:1", etc.

    Example:
        w = Variable(Tensor.from_list([1.0, 2.0, 3.0]), name="weights")
        w.assign_add(Tensor.from_list([0.1, 0.1, 0.1]))
        print(w.data)  # [1.1, 2.1, 3.1]
    """

    def __init__(
        self,
        initial_value: Tensor | list | float,
        trainable: bool = True,
        name: str | None = None,
    ) -> None:
        # ─── Convert various input types to a Tensor ─────────────
        # TensorFlow's Variable accepts lists, scalars, and Tensors.
        if isinstance(initial_value, Tensor):
            data = list(initial_value.data)
            shape = initial_value.shape
        elif isinstance(initial_value, list):
            t = Tensor.from_list(initial_value)
            data = list(t.data)
            shape = t.shape
        else:
            # Scalar
            data = [float(initial_value)]
            shape = (1,)

        # Initialize the underlying Tensor with requires_grad matching trainable.
        # In TF, trainable Variables are watched by GradientTape automatically.
        super().__init__(data, shape, requires_grad=trainable)

        # ─── Store TF-specific attributes ────────────────────────
        # We use object.__setattr__ is not needed here since Tensor
        # uses __slots__, but we keep the pattern for clarity.
        self._trainable = trainable

        # Auto-generate a name if none provided
        global _variable_counter  # noqa: PLW0603
        if name is None:
            name = f"Variable:{_variable_counter}"
            _variable_counter += 1
        self._name = name

    # =====================================================================
    # Properties
    # =====================================================================

    @property
    def trainable(self) -> bool:
        """Whether this Variable is tracked by GradientTape.

        Trainable Variables have their gradients computed during
        tape.gradient(). Non-trainable Variables are ignored.
        """
        return self._trainable

    @property
    def name(self) -> str:
        """Human-readable name for debugging and serialization.

        In real TensorFlow, names form a hierarchy:
            "dense/kernel:0", "dense/bias:0", etc.
        We keep it simpler here.
        """
        return self._name

    # =====================================================================
    # In-place mutation methods
    # =====================================================================

    def assign(self, value: Tensor | list | float) -> None:
        """Replace this Variable's value entirely.

        This is TensorFlow's way of updating a Variable in-place.
        Unlike Python assignment (var = new_value), which creates a
        new object, assign() modifies the existing Variable object.

        This matters because the optimizer and GradientTape hold
        references to the original Variable object.

        Args:
            value: New value (Tensor, list, or scalar).

        Example:
            w = Variable([1.0, 2.0])
            w.assign([10.0, 20.0])
            print(w.data)  # [10.0, 20.0]
        """
        if isinstance(value, Tensor):
            self.data = list(value.data)
            self.shape = value.shape
        elif isinstance(value, list):
            t = Tensor.from_list(value)
            self.data = list(t.data)
            self.shape = t.shape
        else:
            self.data = [float(value)]
            self.shape = (1,)

    def assign_add(self, delta: Tensor | list | float) -> None:
        """Add delta to this Variable in-place: var += delta.

        Used by some optimizers and for accumulating values:
            step_counter.assign_add(1)   # increment step
            momentum.assign_add(grad)    # accumulate gradient

        Args:
            delta: Value to add (same shape as this Variable).
        """
        if isinstance(delta, Tensor):
            delta_data = delta.data
        elif isinstance(delta, list):
            t = Tensor.from_list(delta)
            delta_data = t.data
        else:
            delta_data = [float(delta)] * len(self.data)

        self.data = [a + b for a, b in zip(self.data, delta_data)]

    def assign_sub(self, delta: Tensor | list | float) -> None:
        """Subtract delta from this Variable in-place: var -= delta.

        Commonly used in gradient descent:
            weight.assign_sub(learning_rate * gradient)

        This is the core operation of training — updating weights
        in the direction that reduces the loss.

        Args:
            delta: Value to subtract (same shape as this Variable).
        """
        if isinstance(delta, Tensor):
            delta_data = delta.data
        elif isinstance(delta, list):
            t = Tensor.from_list(delta)
            delta_data = t.data
        else:
            delta_data = [float(delta)] * len(self.data)

        self.data = [a - b for a, b in zip(self.data, delta_data)]

    # =====================================================================
    # Display
    # =====================================================================

    def __repr__(self) -> str:
        trainable_str = ", trainable=True" if self._trainable else ", trainable=False"
        if self.numel <= 10:
            data_str = str(self.data)
        else:
            data_str = f"[{self.data[0]}, ..., {self.data[-1]}]"
        return (
            f"<tf.Variable '{self._name}' shape={self.shape} "
            f"data={data_str}{trainable_str}>"
        )
