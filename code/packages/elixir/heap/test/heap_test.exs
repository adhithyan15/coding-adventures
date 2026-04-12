defmodule CodingAdventures.HeapTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Heap
  alias CodingAdventures.Heap.{MaxHeap, MinHeap}

  test "min heap push and pop" do
    heap = MinHeap.new()
    heap = MinHeap.push(heap, 3)
    heap = MinHeap.push(heap, 1)
    heap = MinHeap.push(heap, 2)

    assert MinHeap.peek(heap) == 1
    {value, heap} = MinHeap.pop(heap)
    assert value == 1
    assert MinHeap.pop(heap) |> elem(0) == 2
  end

  test "max heap from iterable" do
    heap = MaxHeap.from_iterable([1, 5, 3, 2])
    assert MaxHeap.peek(heap) == 5
    assert Heap.heapify([3, 1, 2]) |> Enum.sort() == [1, 2, 3]
  end

  test "nlargest and nsmallest work" do
    assert Heap.nlargest([3, 1, 4, 1, 5], 2) == [5, 4]
    assert Heap.nsmallest([3, 1, 4, 1, 5], 2) == [1, 1]
  end

  test "empty heaps raise and report empty" do
    heap = MinHeap.new()
    assert MinHeap.is_empty(heap)
    assert MinHeap.size(heap) == 0
    assert MinHeap.to_array(heap) == []
    assert_raise ArgumentError, fn -> MinHeap.peek(heap) end
    assert_raise ArgumentError, fn -> MinHeap.pop(heap) end
  end

  test "from_iterable builds both heap kinds" do
    min = MinHeap.from_iterable([5, 1, 3, 2])
    max = MaxHeap.from_iterable([5, 1, 3, 2])
    assert MinHeap.peek(min) == 1
    assert MaxHeap.peek(max) == 5
    assert Heap.heap_sort([5, 1, 3, 2]) == [1, 2, 3, 5]
  end

  test "max heap push pop and empty errors are covered" do
    heap = MaxHeap.new()
    heap = MaxHeap.push(heap, 3)
    heap = MaxHeap.push(heap, 5)
    heap = MaxHeap.push(heap, 1)

    assert MaxHeap.peek(heap) == 5
    {value, heap} = MaxHeap.pop(heap)
    assert value == 5
    assert MaxHeap.size(heap) == 2
    assert MaxHeap.to_array(heap) |> Enum.sort() == [1, 3]
    assert MaxHeap.is_empty(MaxHeap.new())
    assert_raise ArgumentError, fn -> MaxHeap.peek(MaxHeap.new()) end
    assert_raise ArgumentError, fn -> MaxHeap.pop(MaxHeap.new()) end
  end
end
