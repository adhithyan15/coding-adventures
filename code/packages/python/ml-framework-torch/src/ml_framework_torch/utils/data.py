"""
================================================================
DATA UTILITIES — DATASET AND DATALOADER
================================================================

Training a neural network requires iterating over data in batches.
This module provides the standard PyTorch data loading abstractions:

1. **Dataset**: An abstract base that defines how to access samples
2. **TensorDataset**: A concrete dataset wrapping pre-loaded tensors
3. **DataLoader**: An iterator that yields batches from a dataset

=== Why Batches? ===

Processing one sample at a time (batch_size=1) is noisy — the
gradient from a single sample may point in a random direction.
Processing all samples at once (full batch) is expensive and uses
too much memory.

Mini-batch training (batch_size=32 to 256) is the sweet spot:
- Gradient estimates are reasonably accurate
- GPU/CPU can parallelize the computation
- Training is stable and efficient

=== The Data Pipeline ===

    Raw data → Dataset → DataLoader → Training loop

    dataset = TensorDataset(features, labels)
    loader = DataLoader(dataset, batch_size=32, shuffle=True)

    for batch_x, batch_y in loader:
        # batch_x: (32, num_features)
        # batch_y: (32,) or (32, num_outputs)
        loss = model(batch_x, batch_y)
        ...

================================================================
"""

from __future__ import annotations

import random

from ml_framework_core import Tensor


class Dataset:
    """Abstract base class for all datasets.

    Subclasses must implement:
    - __len__(): return the total number of samples
    - __getitem__(idx): return the sample at index idx

    Each sample is typically a tuple (input, target).
    """

    def __len__(self) -> int:
        """Return the total number of samples in the dataset."""
        raise NotImplementedError

    def __getitem__(self, idx: int) -> tuple:
        """Return the sample at the given index.

        Returns a tuple, typically (input_tensor, target_tensor).
        """
        raise NotImplementedError


class TensorDataset(Dataset):
    """A dataset wrapping multiple tensors.

    Each tensor's first dimension is the sample dimension, and all
    tensors must have the same first-dimension size.

    Example:
        X = Tensor.randn(100, 10)   # 100 samples, 10 features
        y = Tensor.randn(100, 1)    # 100 labels

        dataset = TensorDataset(X, y)
        len(dataset)                 # 100
        dataset[0]                   # (Tensor(shape=(10,)), Tensor(shape=(1,)))

    For single-tensor datasets:
        dataset = TensorDataset(X)
        dataset[0]                   # (Tensor(shape=(10,)),)
    """

    def __init__(self, *tensors: Tensor) -> None:
        if len(tensors) == 0:
            raise ValueError("TensorDataset requires at least one tensor")

        # All tensors must have the same number of samples (first dim)
        n = tensors[0].shape[0]
        for i, t in enumerate(tensors):
            if t.shape[0] != n:
                raise ValueError(
                    f"All tensors must have the same first dimension. "
                    f"Tensor 0 has {n}, tensor {i} has {t.shape[0]}"
                )

        self.tensors = tensors
        self._length = n

    def __len__(self) -> int:
        return self._length

    def __getitem__(self, idx: int) -> tuple[Tensor, ...]:
        """Slice each tensor at the given index.

        For a tensor of shape (N, D), returns a tensor of shape (D,).
        For a tensor of shape (N,), returns a tensor of shape (1,).
        """
        if idx < 0:
            idx = self._length + idx
        if idx < 0 or idx >= self._length:
            raise IndexError(f"Index {idx} out of range [0, {self._length})")

        result = []
        for t in self.tensors:
            # Extract one sample from the first dimension
            if t.ndim == 1:
                result.append(Tensor([t.data[idx]], (1,), device=t.device))
            else:
                # For n-D tensor: slice along first dim
                inner_shape = t.shape[1:]
                inner_size = 1
                for s in inner_shape:
                    inner_size *= s
                start = idx * inner_size
                end = start + inner_size
                result.append(
                    Tensor(
                        t.data[start:end],
                        inner_shape,
                        device=t.device,
                    )
                )
        return tuple(result)


class DataLoader:
    """Iterate over a dataset in batches.

    The DataLoader handles:
    1. Splitting the dataset into batches of batch_size samples
    2. Optionally shuffling the order each epoch
    3. Dropping the last incomplete batch (if drop_last=True)

    Usage:
        loader = DataLoader(dataset, batch_size=32, shuffle=True)

        for epoch in range(10):
            for batch in loader:
                x_batch, y_batch = batch
                # x_batch.shape = (32, ...)
                # y_batch.shape = (32, ...)

    Args:
        dataset: The Dataset to iterate over
        batch_size: Number of samples per batch (default: 1)
        shuffle: Whether to randomize sample order (default: False)
        drop_last: Drop the last batch if smaller than batch_size (default: False)
    """

    def __init__(
        self,
        dataset: Dataset,
        batch_size: int = 1,
        shuffle: bool = False,
        drop_last: bool = False,
    ) -> None:
        self.dataset = dataset
        self.batch_size = batch_size
        self.shuffle = shuffle
        self.drop_last = drop_last

    def __iter__(self):
        """Yield batches of data.

        Each batch is a tuple of Tensors with the batch dimension
        as the first dimension.

        For a TensorDataset with tensors of shape (N, D):
            Each batch yields tensors of shape (batch_size, D)
        """
        n = len(self.dataset)
        indices = list(range(n))

        if self.shuffle:
            random.shuffle(indices)

        # Process indices in chunks of batch_size
        for start in range(0, n, self.batch_size):
            end = min(start + self.batch_size, n)

            # Drop last incomplete batch if requested
            if self.drop_last and (end - start) < self.batch_size:
                break

            batch_indices = indices[start:end]

            # Collect individual samples
            samples = [self.dataset[i] for i in batch_indices]

            # Stack samples into batch tensors
            # samples is a list of tuples, e.g., [(x1, y1), (x2, y2), ...]
            # We want to produce (X_batch, Y_batch) where each is stacked
            num_tensors = len(samples[0])
            batch_tensors = []

            for tensor_idx in range(num_tensors):
                # Gather the tensor_idx-th element from each sample
                elements = [s[tensor_idx] for s in samples]
                batch_tensors.append(_stack_tensors(elements))

            yield tuple(batch_tensors)

    def __len__(self) -> int:
        """Return the number of batches per epoch."""
        n = len(self.dataset)
        if self.drop_last:
            return n // self.batch_size
        return (n + self.batch_size - 1) // self.batch_size


def _stack_tensors(tensors: list[Tensor]) -> Tensor:
    """Stack a list of tensors along a new first dimension.

    Given tensors of shape (D,), returns a tensor of shape (N, D)
    where N = len(tensors).

    This is like torch.stack(tensors, dim=0).
    """
    if not tensors:
        raise ValueError("Cannot stack empty list of tensors")

    inner_shape = tensors[0].shape
    batch_size = len(tensors)

    # Concatenate all data
    all_data: list[float] = []
    for t in tensors:
        if t.shape != inner_shape:
            raise ValueError(
                f"All tensors must have the same shape. "
                f"Expected {inner_shape}, got {t.shape}"
            )
        all_data.extend(t.data)

    new_shape = (batch_size, *inner_shape)
    return Tensor(all_data, new_shape, device=tensors[0].device)
