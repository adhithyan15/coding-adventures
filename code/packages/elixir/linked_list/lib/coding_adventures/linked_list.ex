defmodule CodingAdventures.LinkedList do
  @moduledoc """
  Immutable linked-list style wrapper for ordered sequences.
  """

  defstruct items: []

  @type t :: %__MODULE__{items: [any()]}

  def new(), do: %__MODULE__{}

  def from_list(items) when is_list(items), do: %__MODULE__{items: items}

  def to_list(%__MODULE__{items: items}), do: items
  def len(%__MODULE__{items: items}), do: length(items)
  def is_empty(list), do: len(list) == 0

  def push_left(%__MODULE__{items: items} = list, value), do: %{list | items: [value | items]}

  def push_right(%__MODULE__{items: items} = list, value), do: %{list | items: items ++ [value]}

  def pop_left(%__MODULE__{items: []} = list), do: {list, nil}

  def pop_left(%__MODULE__{items: [head | tail]} = list), do: {%{list | items: tail}, head}

  def pop_right(%__MODULE__{items: []} = list), do: {list, nil}

  def pop_right(%__MODULE__{items: items} = list) do
    {value, next_items} = List.pop_at(items, -1)
    {%{list | items: next_items}, value}
  end

  def index(%__MODULE__{items: items}, idx) when is_integer(idx), do: Enum.at(items, idx)

  def range(%__MODULE__{items: items}, start, stop) when is_integer(start) and is_integer(stop) do
    length = length(items)
    start = normalize_index(start, length)
    stop = normalize_index(stop, length)

    if start > stop or start >= length do
      []
    else
      items |> Enum.slice(start..min(stop, length - 1))
    end
  end

  defp normalize_index(index, length) when index < 0, do: length + index
  defp normalize_index(index, _length), do: index
end
