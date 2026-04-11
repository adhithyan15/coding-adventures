from __future__ import annotations

import pytest

from heap_native import MaxHeap, MinHeap, heap_sort, heapify, nlargest, nsmallest


def test_min_heap_push_peek_pop_and_repr() -> None:
    heap = MinHeap()
    for value in [5, 3, 8, 1, 4]:
        heap.push(value)

    assert heap.peek() == 1
    assert heap.pop() == 1
    assert heap.to_array() == [3, 4, 8, 5]
    assert "MinHeap" in repr(heap)


def test_max_heap_from_iterable() -> None:
    heap = MaxHeap.from_iterable([5, 3, 8, 1, 4])
    assert heap.peek() == 8
    assert heap.pop() == 8
    assert len(heap) == 4


def test_empty_pop_and_peek_raise_index_error() -> None:
    with pytest.raises(IndexError):
        MinHeap().pop()
    with pytest.raises(IndexError):
        MaxHeap().peek()


def test_module_level_helpers() -> None:
    values = [3, 1, 4, 1, 5, 9, 2, 6]
    assert sorted(heapify(values)) == sorted(values)
    assert heap_sort(values) == sorted(values)
    assert nlargest(values, 3) == [9, 6, 5]
    assert nsmallest(values, 3) == [1, 1, 2]
