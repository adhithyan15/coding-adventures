defmodule CodingAdventures.Heap do
  @moduledoc """
  Min-heap and max-heap priority queues.
  """

  def default_compare(left, right) do
    cond do
      left == right -> 0
      left < right -> -1
      true -> 1
    end
  end

  def sift_up(data, index, higher_priority) do
    if index > 0 do
      parent = div(index - 1, 2)

      if higher_priority.(Enum.at(data, index), Enum.at(data, parent)) do
        current = Enum.at(data, index)
        parent_value = Enum.at(data, parent)
        data = List.replace_at(data, index, parent_value)
        data = List.replace_at(data, parent, current)
        sift_up(data, parent, higher_priority)
      else
        data
      end
    else
      data
    end
  end

  def sift_down(data, index, higher_priority) do
    total = length(data)
    left = 2 * index + 1
    right = 2 * index + 2
    best = index

    best =
      if left < total and higher_priority.(Enum.at(data, left), Enum.at(data, best)) do
        left
      else
        best
      end

    best =
      if right < total and higher_priority.(Enum.at(data, right), Enum.at(data, best)) do
        right
      else
        best
      end

    if best == index do
      data
    else
      current = Enum.at(data, index)
      best_value = Enum.at(data, best)
      data = List.replace_at(data, index, best_value)
      data = List.replace_at(data, best, current)
      sift_down(data, best, higher_priority)
    end
  end

  def build_heap(data, higher_priority) do
    data = Enum.to_list(data)

    if length(data) < 2 do
      data
    else
      Enum.reduce((div(length(data) - 2, 2))..0, data, fn index, acc ->
        sift_down(acc, index, higher_priority)
      end)
    end
  end

  defmodule MinHeap do
    @moduledoc false
    defstruct data: [], compare: &CodingAdventures.Heap.default_compare/2

    def new(compare \\ &CodingAdventures.Heap.default_compare/2) when is_function(compare, 2) do
      %__MODULE__{compare: compare}
    end

    def from_iterable(items, compare \\ &CodingAdventures.Heap.default_compare/2)
        when is_function(compare, 2) do
      %__MODULE__{data: CodingAdventures.Heap.build_heap(items, fn left, right ->
        compare.(left, right) < 0
      end), compare: compare}
    end

    def push(%__MODULE__{data: data, compare: compare} = heap, value) do
      %{heap | data: CodingAdventures.Heap.sift_up(data ++ [value], length(data), fn left, right ->
        compare.(left, right) < 0
      end)}
    end

    def pop(%__MODULE__{data: []}), do: raise(ArgumentError, "pop from an empty heap")

    def pop(%__MODULE__{data: [root]} = heap), do: {root, %{heap | data: []}}

    def pop(%__MODULE__{data: data, compare: compare} = heap) do
      last = List.last(data)
      data = List.delete_at(data, length(data) - 1)
      data = List.replace_at(data, 0, last)
      data = CodingAdventures.Heap.sift_down(data, 0, fn left, right -> compare.(left, right) < 0 end)
      {Enum.at(heap.data, 0), %{heap | data: data}}
    end

    def peek(%__MODULE__{data: []}), do: raise(ArgumentError, "peek at an empty heap")
    def peek(%__MODULE__{data: [root]}), do: root
    def peek(%__MODULE__{data: data}), do: hd(data)
    def is_empty(%__MODULE__{data: data}), do: data == []
    def size(%__MODULE__{data: data}), do: length(data)
    def to_array(%__MODULE__{data: data}), do: Enum.to_list(data)
  end

  defmodule MaxHeap do
    @moduledoc false
    defstruct data: [], compare: &CodingAdventures.Heap.default_compare/2

    def new(compare \\ &CodingAdventures.Heap.default_compare/2) when is_function(compare, 2) do
      %__MODULE__{compare: compare}
    end

    def from_iterable(items, compare \\ &CodingAdventures.Heap.default_compare/2)
        when is_function(compare, 2) do
      %__MODULE__{data: CodingAdventures.Heap.build_heap(items, fn left, right ->
        compare.(left, right) > 0
      end), compare: compare}
    end

    def push(%__MODULE__{data: data, compare: compare} = heap, value) do
      %{heap | data: CodingAdventures.Heap.sift_up(data ++ [value], length(data), fn left, right ->
        compare.(left, right) > 0
      end)}
    end

    def pop(%__MODULE__{data: []}), do: raise(ArgumentError, "pop from an empty heap")
    def pop(%__MODULE__{data: [root]} = heap), do: {root, %{heap | data: []}}

    def pop(%__MODULE__{data: data, compare: compare} = heap) do
      last = List.last(data)
      data = List.delete_at(data, length(data) - 1)
      data = List.replace_at(data, 0, last)
      data = CodingAdventures.Heap.sift_down(data, 0, fn left, right -> compare.(left, right) > 0 end)
      {Enum.at(heap.data, 0), %{heap | data: data}}
    end

    def peek(%__MODULE__{data: []}), do: raise(ArgumentError, "peek at an empty heap")
    def peek(%__MODULE__{data: [root]}), do: root
    def peek(%__MODULE__{data: data}), do: hd(data)
    def is_empty(%__MODULE__{data: data}), do: data == []
    def size(%__MODULE__{data: data}), do: length(data)
    def to_array(%__MODULE__{data: data}), do: Enum.to_list(data)
  end

  def heapify(items), do: MinHeap.from_iterable(items).data

  def heap_sort(items) do
    heap = MinHeap.from_iterable(items)
    heap_sort(heap, [])
  end

  def nlargest(items, n) when n <= 0, do: []
  def nlargest(items, n) do
    items |> Enum.sort(:desc) |> Enum.take(n)
  end

  def nsmallest(items, n) when n <= 0, do: []
  def nsmallest(items, n) do
    items |> Enum.sort() |> Enum.take(n)
  end

  defp heap_sort(%MinHeap{} = heap, acc) do
    if MinHeap.is_empty(heap) do
      Enum.reverse(acc)
    else
      {value, next} = MinHeap.pop(heap)
      heap_sort(next, [value | acc])
    end
  end
end
