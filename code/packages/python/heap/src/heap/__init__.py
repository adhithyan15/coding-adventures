"""
heap — DT04: Min-Heap and Max-Heap
====================================

A priority queue backed by a flat array with O(log n) push/pop and O(1) peek.

Public API::

    from heap import MinHeap, MaxHeap
    from heap import heapify, heap_sort, nlargest, nsmallest
"""

from heap.heap import MaxHeap, MinHeap
from heap.functions import heapify, heap_sort, nlargest, nsmallest

__version__ = "0.1.0"

__all__ = [
    "MinHeap",
    "MaxHeap",
    "heapify",
    "heap_sort",
    "nlargest",
    "nsmallest",
]
