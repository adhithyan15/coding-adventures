defmodule CodingAdventures.SkipList do
  @moduledoc """
  Ordered collection with rank and range queries.
  """

  @enforce_keys [:entries, :compare]
  defstruct [:entries, :compare]

  @type comparator :: (any(), any() -> integer())

  def new(opts \\ []) do
    compare = Keyword.get(opts, :compare, &default_compare/2)
    %__MODULE__{entries: [], compare: compare}
  end

  def from_list(entries, opts \\ []) do
    Enum.reduce(entries, new(opts), fn {key, value}, acc -> insert(acc, key, value) end)
  end

  def size(%__MODULE__{entries: entries}), do: length(entries)
  def is_empty(%__MODULE__{entries: entries}), do: entries == []
  def keys(list), do: list |> entries() |> Enum.map(&elem(&1, 0))
  def values(list), do: list |> entries() |> Enum.map(&elem(&1, 1))
  def entries(%__MODULE__{entries: entries}), do: entries
  def to_list(list), do: keys(list)

  def insert(%__MODULE__{} = list, key, value) do
    %{list | entries: insert_sorted(list.entries, {key, value}, list.compare)}
  end

  def delete(%__MODULE__{} = list, key) do
    %{list | entries: Enum.reject(list.entries, fn {existing_key, _} -> existing_key == key end)}
  end

  def search(%__MODULE__{entries: entries}, key) do
    case Enum.find(entries, fn {existing_key, _} -> existing_key == key end) do
      {_, value} -> value
      nil -> nil
    end
  end

  def contains?(list, key), do: search(list, key) != nil

  def rank(%__MODULE__{entries: entries, compare: compare}, key) do
    case Enum.find_index(entries, fn {existing_key, _} -> compare.(existing_key, key) == 0 end) do
      nil -> nil
      index -> index
    end
  end

  def by_rank(%__MODULE__{entries: entries}, rank) when rank >= 0 do
    case Enum.at(entries, rank) do
      {key, _value} -> key
      nil -> nil
    end
  end

  def min(%__MODULE__{entries: []}), do: nil
  def min(%__MODULE__{entries: [{key, _} | _]}), do: key

  def max(%__MODULE__{entries: []}), do: nil
  def max(%__MODULE__{entries: entries}), do: entries |> List.last() |> elem(0)

  def range(%__MODULE__{entries: entries, compare: compare}, lo, hi, inclusive \\ true) do
    Enum.filter(entries, fn {key, _} ->
      lower_ok =
        if inclusive do
          compare.(key, lo) >= 0
        else
          compare.(key, lo) > 0
        end

      upper_ok =
        if inclusive do
          compare.(key, hi) <= 0
        else
          compare.(key, hi) < 0
        end

      lower_ok and upper_ok
    end)
  end

  defp insert_sorted([], entry, _compare), do: [entry]

  defp insert_sorted([{key, _} = head | tail], {new_key, _} = entry, compare) do
    case compare.(new_key, key) do
      order when order < 0 -> [entry, head | tail]
      0 -> [entry | tail]
      _ -> [head | insert_sorted(tail, entry, compare)]
    end
  end

  defp default_compare(left, right) do
    cond do
      left == right -> 0
      left < right -> -1
      true -> 1
    end
  end
end
