defmodule CodingAdventures.FenwickTree do
  @moduledoc """
  Immutable Binary Indexed Tree for prefix sums and point updates.
  """

  import Bitwise

  defstruct length: 0, bit: {0.0}

  @type t :: %__MODULE__{length: non_neg_integer(), bit: tuple()}

  def new(length) when is_integer(length) and length >= 0 do
    %__MODULE__{length: length, bit: List.to_tuple(List.duplicate(0.0, length + 1))}
  end

  def new(length),
    do: raise(ArgumentError, "size must be a non-negative integer, got #{inspect(length)}")

  def from_list(values) when is_list(values) do
    length = length(values)

    bit =
      Enum.reduce(1..length//1, List.to_tuple([0.0 | values]), fn index, bit ->
        parent = index + lowbit(index)

        if parent <= length do
          put_elem(bit, parent, elem(bit, parent) + elem(bit, index))
        else
          bit
        end
      end)

    %__MODULE__{length: length, bit: bit}
  end

  def update(%__MODULE__{} = tree, index, delta) do
    check_index!(tree, index)

    bit =
      Stream.iterate(index, &(&1 + lowbit(&1)))
      |> Enum.take_while(&(&1 <= tree.length))
      |> Enum.reduce(tree.bit, fn current, bit ->
        put_elem(bit, current, elem(bit, current) + delta)
      end)

    %{tree | bit: bit}
  end

  def prefix_sum(%__MODULE__{} = tree, index) do
    unless is_integer(index) and index >= 0 and index <= tree.length do
      raise ArgumentError, "prefix_sum index #{inspect(index)} out of range [0, #{tree.length}]"
    end

    Stream.iterate(index, &(&1 - lowbit(&1)))
    |> Enum.take_while(&(&1 > 0))
    |> Enum.reduce(0.0, fn current, total -> total + elem(tree.bit, current) end)
  end

  def range_sum(%__MODULE__{} = tree, left, right) do
    if left > right do
      raise ArgumentError, "left (#{left}) must be <= right (#{right})"
    end

    check_index!(tree, left)
    check_index!(tree, right)

    if left == 1 do
      prefix_sum(tree, right)
    else
      prefix_sum(tree, right) - prefix_sum(tree, left - 1)
    end
  end

  def point_query(%__MODULE__{} = tree, index) do
    check_index!(tree, index)
    range_sum(tree, index, index)
  end

  def find_kth(%__MODULE__{length: 0}, _target) do
    raise ArgumentError, "find_kth called on empty tree"
  end

  def find_kth(%__MODULE__{}, target) when target <= 0 do
    raise ArgumentError, "k must be positive, got #{inspect(target)}"
  end

  def find_kth(%__MODULE__{} = tree, target) do
    total = prefix_sum(tree, tree.length)

    if target > total do
      raise ArgumentError, "k exceeds total sum of the tree"
    end

    {index, _target} =
      Stream.iterate(highest_power_of_two_at_most(tree.length), &div(&1, 2))
      |> Enum.take_while(&(&1 > 0))
      |> Enum.reduce({0, target}, fn step, {index, remaining} ->
        next = index + step

        if next <= tree.length and elem(tree.bit, next) < remaining do
          {next, remaining - elem(tree.bit, next)}
        else
          {index, remaining}
        end
      end)

    index + 1
  end

  def empty?(%__MODULE__{length: 0}), do: true
  def empty?(%__MODULE__{}), do: false

  def bit_array(%__MODULE__{} = tree) do
    tree.bit
    |> Tuple.to_list()
    |> tl()
  end

  defp check_index!(%__MODULE__{} = tree, index) do
    unless is_integer(index) and index >= 1 and index <= tree.length do
      raise ArgumentError, "Index #{inspect(index)} out of range [1, #{tree.length}]"
    end
  end

  defp lowbit(index), do: index &&& -index

  defp highest_power_of_two_at_most(0), do: 0

  defp highest_power_of_two_at_most(number) do
    Stream.iterate(1, &(&1 * 2))
    |> Enum.take_while(&(&1 <= number))
    |> List.last()
  end
end
