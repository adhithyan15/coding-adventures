"""
heap — DT04: Min-Heap and Max-Heap
====================================

A priority queue backed by a flat array with O(log n) push/pop and O(1) peek.

Public API::

    from heap import MinHeap, MaxHeap
    from heap import heapify, heap_sort, nlargest, nsmallest
"""

from heap.bounded import BoundedHeap, CheckedPriorityQueue, HeapFullError
from heap.functions import heap_sort, heapify, nlargest, nsmallest
from heap.heap import MaxHeap, MinHeap

__version__ = "0.2.0"

__all__ = [
    "MinHeap",
    "MaxHeap",
    "BoundedHeap",
    "CheckedPriorityQueue",
    "HeapFullError",
    "heapify",
    "heap_sort",
    "nlargest",
    "nsmallest",
]
