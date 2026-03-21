"""
================================================================
UTILITIES — DATA LOADING AND TRAINING HELPERS
================================================================

This sub-package provides tools for preparing and iterating over
training data:

- Dataset: abstract base class for datasets
- TensorDataset: dataset from pre-loaded tensors
- DataLoader: iterate over a dataset in batches

================================================================
"""

from .data import DataLoader, Dataset, TensorDataset

__all__ = [
    "Dataset",
    "TensorDataset",
    "DataLoader",
]
