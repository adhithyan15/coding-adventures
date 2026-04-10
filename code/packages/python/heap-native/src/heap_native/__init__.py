"""
Heap -- Native Extension (Rust-backed via python-bridge)
========================================================
"""

from heap_native.heap_native import (  # type: ignore[import]
    MaxHeap,
    MinHeap,
    heap_sort,
    heapify,
    nlargest,
    nsmallest,
)

__all__ = [
    "MinHeap",
    "MaxHeap",
    "heapify",
    "heap_sort",
    "nlargest",
    "nsmallest",
]
