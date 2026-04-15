defmodule CodingAdventures.RadixTree do
  @moduledoc """
  Lightweight radix-tree-like prefix index for string keys.

  This implementation is intentionally simple and immutable. It preserves the
  public semantics needed by the datastore engine: insert, delete, membership,
  prefix lookup, and sorted key enumeration.
  """

  defstruct entries: %{}

  @type t :: %__MODULE__{entries: %{String.t() => any()}}

  def new(), do: %__MODULE__{}

  def from_list(entries) when is_list(entries) do
    Enum.reduce(entries, new(), fn
      {key, value}, acc -> put(acc, key, value)
      other, _ -> raise ArgumentError, "expected {key, value} tuple, got #{inspect(other)}"
    end)
  end

  def put(%__MODULE__{entries: entries} = tree, key, value) when is_binary(key) do
    %{tree | entries: Map.put(entries, key, value)}
  end

  def delete(%__MODULE__{entries: entries} = tree, key) when is_binary(key) do
    %{tree | entries: Map.delete(entries, key)}
  end

  def get(%__MODULE__{entries: entries}, key) when is_binary(key), do: Map.get(entries, key)
  def has?(tree, key), do: get(tree, key) != nil
  def contains?(tree, key), do: has?(tree, key)
  def size(%__MODULE__{entries: entries}), do: map_size(entries)
  def is_empty(tree), do: size(tree) == 0

  def keys(%__MODULE__{entries: entries}) do
    entries |> Map.keys() |> Enum.sort()
  end

  def values(%__MODULE__{entries: entries}) do
    entries |> Enum.sort_by(fn {key, _} -> key end) |> Enum.map(&elem(&1, 1))
  end

  def to_map(%__MODULE__{entries: entries}), do: entries

  def starts_with?(%__MODULE__{entries: entries}, prefix) when is_binary(prefix) do
    Enum.any?(Map.keys(entries), &String.starts_with?(&1, prefix))
  end

  def words_with_prefix(%__MODULE__{entries: entries}, prefix) when is_binary(prefix) do
    entries
    |> Map.keys()
    |> Enum.filter(&String.starts_with?(&1, prefix))
    |> Enum.sort()
  end

  def longest_prefix_match(%__MODULE__{entries: entries}, key) when is_binary(key) do
    entries
    |> Map.keys()
    |> Enum.filter(&String.starts_with?(key, &1))
    |> Enum.max_by(&String.length/1, fn -> nil end)
  end
end
