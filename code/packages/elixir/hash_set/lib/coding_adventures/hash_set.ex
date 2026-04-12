defmodule CodingAdventures.HashSet do
  @moduledoc """
  Immutable hash set backed by `CodingAdventures.HashMap`.
  """

  alias CodingAdventures.HashMap

  @enforce_keys [:map]
  defstruct [:map]

  def new(opts \\ []) do
    %__MODULE__{map: HashMap.new(opts)}
  end

  def from_values(values, opts \\ []) do
    Enum.reduce(values, new(opts), &add(&2, &1))
  end

  def size(%__MODULE__{map: map}), do: HashMap.size(map)
  def values(%__MODULE__{map: map}), do: HashMap.keys(map)
  def to_list(set), do: values(set)
  def has?(%__MODULE__{map: map}, value), do: HashMap.has_key?(map, value)

  def add(%__MODULE__{map: map} = set, value) do
    %{set | map: HashMap.put(map, value, true)}
  end

  def delete(%__MODULE__{map: map} = set, value) do
    %{set | map: HashMap.delete(map, value)}
  end

  def union(left, right) do
    from_values(Enum.uniq(values(left) ++ values(right)))
  end

  def intersection(left, right) do
    right_values = MapSet.new(values(right))
    values(left) |> Enum.filter(&MapSet.member?(right_values, &1)) |> from_values()
  end

  def difference(left, right) do
    right_values = MapSet.new(values(right))
    values(left) |> Enum.reject(&MapSet.member?(right_values, &1)) |> from_values()
  end

  def symmetric_difference(left, right) do
    union(left, right)
    |> values()
    |> Enum.reject(fn value -> has?(left, value) and has?(right, value) end)
    |> from_values()
  end

  def subset?(left, right) do
    Enum.all?(values(left), &has?(right, &1))
  end

  def superset?(left, right), do: subset?(right, left)
  def disjoint?(left, right), do: Enum.all?(values(left), fn value -> not has?(right, value) end)
end
