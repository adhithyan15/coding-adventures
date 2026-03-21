"""
================================================================
TF.DATA — THE TENSORFLOW DATA PIPELINE
================================================================

tf.data.Dataset is TensorFlow's abstraction for efficient data
loading and preprocessing. It provides a lazy, chainable pipeline
for feeding data to models.

=== The tf.data Philosophy ===

In TensorFlow, data loading follows a pipeline pattern:
    raw_data → Dataset → .shuffle() → .batch() → .prefetch() → model

Each transformation returns a new Dataset (functional/immutable style),
so you chain them together:

    dataset = tf.data.Dataset.from_tensor_slices((x_train, y_train))
    dataset = dataset.shuffle(1000)
    dataset = dataset.batch(32)

    for x_batch, y_batch in dataset:
        ...

=== from_tensor_slices ===

The most common way to create a Dataset. It takes one or more
tensors and slices them along the first dimension (each row
becomes one sample):

    x = [[1, 2], [3, 4], [5, 6]]   # 3 samples, 2 features each
    y = [0, 1, 1]                    # 3 labels

    dataset = tf.data.Dataset.from_tensor_slices((x, y))
    # Yields: ([1, 2], 0), ([3, 4], 1), ([5, 6], 1)

=== Our Simplified Implementation ===

Real tf.data.Dataset supports lazy evaluation, parallel loading,
caching, interleaving, and more. Our implementation provides the
essential functionality:
- from_tensor_slices: create a dataset from tensors
- batch: group samples into batches
- shuffle: randomize order
- Iteration via __iter__

This is enough to write training loops that look like real TF code.

================================================================
"""

from __future__ import annotations

import random as _random

from ml_framework_core import Tensor


class Dataset:
    """A sequence of elements (samples) that can be iterated over.

    This is TensorFlow's core data abstraction. You typically create
    a Dataset via the from_tensor_slices() factory method, then
    chain transformations like .batch() and .shuffle().

    Example:
        x = tf.constant([[1, 2], [3, 4], [5, 6]])
        y = tf.constant([0, 1, 1])

        ds = tf.data.Dataset.from_tensor_slices((x, y))
        ds = ds.shuffle(100).batch(2)

        for x_batch, y_batch in ds:
            print(x_batch.shape)  # (2, 2)
    """

    def __init__(self, elements: list) -> None:
        """Create a Dataset from a list of elements.

        Each element is a tuple of Tensors (one per input tensor).
        For a single-tensor dataset, each element is a Tensor.

        Users should use from_tensor_slices() instead of calling
        this constructor directly.
        """
        self._elements = elements

    @staticmethod
    def from_tensor_slices(
        tensors: Tensor | tuple[Tensor, ...] | list[Tensor],
    ) -> Dataset:
        """Create a Dataset by slicing tensors along the first dimension.

        This is the primary factory method. It takes one tensor (or a
        tuple of tensors) and slices along dim 0. Each slice becomes
        one element of the dataset.

        For a single tensor of shape (N, D):
            Creates N elements, each of shape (D,)

        For a tuple (x, y) where x is (N, D) and y is (N,):
            Creates N elements, each a tuple (x_i, y_i)

        Args:
            tensors: A single Tensor, or a tuple/list of Tensors.
                     All tensors must have the same first dimension.

        Returns:
            A Dataset that iterates over slices.

        Example:
            x = Tensor.from_list([[1, 2], [3, 4], [5, 6]])
            ds = Dataset.from_tensor_slices(x)
            # 3 elements: Tensor([1, 2]), Tensor([3, 4]), Tensor([5, 6])

            ds = Dataset.from_tensor_slices((x, y))
            # 3 elements: (Tensor([1, 2]), Tensor([0])), ...
        """
        # ─── Normalize input to a tuple of tensors ───────────────
        if isinstance(tensors, Tensor):
            tensor_list = (tensors,)
            single = True
        elif isinstance(tensors, (tuple, list)):
            tensor_list = tuple(tensors)
            single = False
        else:
            raise TypeError(f"Expected Tensor or tuple of Tensors, got {type(tensors)}")

        # ─── Validate all tensors have same first dimension ──────
        n = tensor_list[0].shape[0]
        for i, t in enumerate(tensor_list):
            if t.shape[0] != n:
                raise ValueError(
                    f"All tensors must have the same first dimension. "
                    f"Tensor 0 has {n}, tensor {i} has {t.shape[0]}"
                )

        # ─── Slice each tensor along dim 0 ──────────────────────
        elements = []
        for idx in range(n):
            if single:
                # Single tensor: element is just the slice
                elements.append(_slice_tensor(tensor_list[0], idx))
            else:
                # Multiple tensors: element is a tuple of slices
                slices = tuple(_slice_tensor(t, idx) for t in tensor_list)
                elements.append(slices)

        return Dataset(elements)

    def batch(self, batch_size: int) -> Dataset:
        """Group consecutive elements into batches.

        Each batch combines batch_size elements into a single
        element by stacking along a new first dimension.

        For elements of shape (D,):
            Batches have shape (batch_size, D)

        The last batch may be smaller than batch_size if the
        dataset size is not evenly divisible.

        Args:
            batch_size: Number of elements per batch.

        Returns:
            A new Dataset where each element is a batch.

        Example:
            ds = Dataset.from_tensor_slices(x)  # 10 elements
            ds = ds.batch(3)
            # 4 batches: 3, 3, 3, 1 elements
        """
        batched_elements = []

        for start in range(0, len(self._elements), batch_size):
            end = min(start + batch_size, len(self._elements))
            chunk = self._elements[start:end]

            if isinstance(chunk[0], tuple):
                # Multiple tensors per element — batch each position
                num_tensors = len(chunk[0])
                batched = tuple(
                    _stack_tensors([elem[j] for elem in chunk])
                    for j in range(num_tensors)
                )
                batched_elements.append(batched)
            else:
                # Single tensor per element
                batched_elements.append(_stack_tensors(chunk))

        return Dataset(batched_elements)

    def shuffle(self, buffer_size: int) -> Dataset:
        """Randomly shuffle the elements.

        In real TensorFlow, shuffle uses a buffer: it fills a buffer
        of buffer_size elements and randomly draws from it. This is
        memory-efficient for large datasets.

        Our simplified implementation shuffles all elements in memory
        (the buffer_size parameter is accepted for API compatibility
        but we shuffle the entire dataset).

        Args:
            buffer_size: Size of the shuffle buffer (accepted for
                         API compatibility, full shuffle performed).

        Returns:
            A new Dataset with elements in random order.
        """
        shuffled = list(self._elements)
        _random.shuffle(shuffled)
        return Dataset(shuffled)

    def __iter__(self):
        """Iterate over elements in the dataset.

        Each element is either a single Tensor or a tuple of Tensors,
        depending on how the dataset was created.
        """
        return iter(self._elements)

    def __len__(self) -> int:
        """Return the number of elements in the dataset."""
        return len(self._elements)


# =========================================================================
# Helper functions
# =========================================================================


def _slice_tensor(t: Tensor, idx: int) -> Tensor:
    """Extract a single slice along the first dimension.

    For a tensor of shape (N, D1, D2, ...):
        Returns a tensor of shape (D1, D2, ...)

    For a tensor of shape (N,):
        Returns a tensor of shape (1,) containing one value.
    """
    if t.ndim == 1:
        return Tensor([t.data[idx]], (1,), device=t.device)

    inner_shape = t.shape[1:]
    inner_size = 1
    for s in inner_shape:
        inner_size *= s

    start = idx * inner_size
    end = start + inner_size
    return Tensor(t.data[start:end], inner_shape, device=t.device)


def _stack_tensors(tensors: list[Tensor]) -> Tensor:
    """Stack tensors along a new first dimension.

    Given tensors of shape (D,), returns shape (N, D) where N = len(tensors).
    This is the batch-building operation.
    """
    if not tensors:
        raise ValueError("Cannot stack empty list of tensors")

    inner_shape = tensors[0].shape
    all_data: list[float] = []
    for t in tensors:
        all_data.extend(t.data)

    new_shape = (len(tensors), *inner_shape)
    return Tensor(all_data, new_shape, device=tensors[0].device)
