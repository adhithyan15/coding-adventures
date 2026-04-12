defmodule CodingAdventures.TreeSet do
  @moduledoc """
  Ordered set with range and algebra helpers.
  """

  alias CodingAdventures.SkipList

  @enforce_keys [:list]
  defstruct [:list]

  def new(opts \\ []) do
    %__MODULE__{list: SkipList.new(opts)}
  end

  def from_values(values, opts \\ []) do
    Enum.reduce(values, new(opts), &add(&2, &1))
  end

  def size(%__MODULE__{list: list}), do: SkipList.size(list)
  def is_empty(%__MODULE__{list: list}), do: SkipList.is_empty(list)
  def to_list(%__MODULE__{list: list}), do: SkipList.to_list(list)
  def values(set), do: to_list(set)
  def min(%__MODULE__{list: list}), do: SkipList.min(list)
  def max(%__MODULE__{list: list}), do: SkipList.max(list)
  def rank(%__MODULE__{list: list}, value), do: SkipList.rank(list, value)
  def by_rank(%__MODULE__{list: list}, rank), do: SkipList.by_rank(list, rank)
  def predecessor(%__MODULE__{} = set, value), do: predecessor_list(values(set), value)
  def successor(%__MODULE__{} = set, value), do: successor_list(values(set), value)

  def add(%__MODULE__{list: list} = set, value) do
    %{set | list: SkipList.insert(list, value, true)}
  end

  def delete(%__MODULE__{list: list} = set, value) do
    %{set | list: SkipList.delete(list, value)}
  end

  def has?(%__MODULE__{list: list}, value), do: SkipList.contains?(list, value)

  def union(left, right), do: from_values(values(left) ++ values(right))
  def intersection(left, right), do: from_values(Enum.filter(values(left), &has?(right, &1)))
  def difference(left, right), do: from_values(Enum.reject(values(left), &has?(right, &1)))
  def symmetric_difference(left, right), do: from_values(Enum.filter(values(left) ++ values(right), fn value -> has?(left, value) != has?(right, value) end))
  def subset?(left, right), do: Enum.all?(values(left), &has?(right, &1))
  def superset?(left, right), do: subset?(right, left)
  def disjoint?(left, right), do: Enum.all?(values(left), fn value -> not has?(right, value) end)

  def range(%__MODULE__{list: list}, lo, hi, inclusive \\ true) do
    list
    |> SkipList.range(lo, hi, inclusive)
    |> Enum.map(&elem(&1, 0))
  end

  defp predecessor_list(values, value) when is_list(values) do
    values
    |> Enum.filter(&(&1 < value))
    |> List.last()
  end

  defp successor_list(values, value) when is_list(values) do
    values
    |> Enum.find(&(&1 > value))
  end
end
