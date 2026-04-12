defmodule CodingAdventures.ArrayList do
  @moduledoc """
  Chunked array-backed ordered sequence.

  The list is stored as a sequence of fixed-size chunks. End operations only
  touch a single chunk, keeping the working set bounded by the configured
  chunk size instead of the full list length.
  """

  @default_chunk_size 32

  defstruct chunks: [], len: 0, chunk_size: @default_chunk_size

  @type t :: %__MODULE__{
          chunks: [[any()]],
          len: non_neg_integer(),
          chunk_size: pos_integer()
        }

  def new(opts \\ []) do
    chunk_size = normalize_chunk_size(Keyword.get(opts, :chunk_size, @default_chunk_size))
    %__MODULE__{chunk_size: chunk_size}
  end

  def from_list(items, opts \\ []) when is_list(items) do
    list = new(opts)

    items
    |> Enum.chunk_every(list.chunk_size)
    |> Enum.reduce(list, fn chunk, acc ->
      %{acc | chunks: acc.chunks ++ [chunk], len: acc.len + length(chunk)}
    end)
  end

  def to_list(%__MODULE__{chunks: chunks}), do: Enum.flat_map(chunks, & &1)
  def len(%__MODULE__{len: len}), do: len
  def is_empty(list), do: len(list) == 0
  def chunk_size(%__MODULE__{chunk_size: chunk_size}), do: chunk_size
  def chunk_count(%__MODULE__{chunks: chunks}), do: length(chunks)

  def push_left(%__MODULE__{chunks: []} = list, value) do
    %{list | chunks: [[value]], len: list.len + 1}
  end

  def push_left(%__MODULE__{chunks: [first | rest], chunk_size: chunk_size} = list, value) do
    if length(first) < chunk_size do
      %{list | chunks: [[value | first] | rest], len: list.len + 1}
    else
      %{list | chunks: [[value], first | rest], len: list.len + 1}
    end
  end

  def push_right(%__MODULE__{chunks: []} = list, value) do
    %{list | chunks: [[value]], len: list.len + 1}
  end

  def push_right(%__MODULE__{chunks: chunks, chunk_size: chunk_size} = list, value) do
    {last_chunk, rest} = List.pop_at(chunks, -1)

    if length(last_chunk) < chunk_size do
      %{list | chunks: rest ++ [last_chunk ++ [value]], len: list.len + 1}
    else
      %{list | chunks: chunks ++ [[value]], len: list.len + 1}
    end
  end

  def pop_left(%__MODULE__{chunks: []} = list), do: {list, nil}

  def pop_left(%__MODULE__{chunks: [first | rest]} = list) do
    case first do
      [head | tail] ->
        next_chunks = if tail == [], do: rest, else: [tail | rest]
        {%{list | chunks: next_chunks, len: list.len - 1}, head}

      [] ->
        {list, nil}
    end
  end

  def pop_right(%__MODULE__{chunks: []} = list), do: {list, nil}

  def pop_right(%__MODULE__{chunks: chunks} = list) do
    {last_chunk, rest} = List.pop_at(chunks, -1)

    case List.pop_at(last_chunk, -1) do
      {nil, _} ->
        {list, nil}

      {value, next_last} ->
        next_chunks = if next_last == [], do: rest, else: rest ++ [next_last]
        {%{list | chunks: next_chunks, len: list.len - 1}, value}
    end
  end

  def index(%__MODULE__{len: len} = list, idx) when is_integer(idx) do
    idx = normalize_index(idx, len)

    if idx < 0 or idx >= len do
      nil
    else
      fetch_by_index(list.chunks, idx, 0)
    end
  end

  def range(%__MODULE__{len: len} = list, start, stop) when is_integer(start) and is_integer(stop) do
    start = normalize_index(start, len)
    stop = normalize_index(stop, len)

    if start > stop or start >= len do
      []
    else
      list.chunks
      |> collect_range(start, stop, 0, [])
      |> Enum.reverse()
      |> List.flatten()
    end
  end

  defp normalize_chunk_size(size) when is_integer(size) and size > 0, do: size
  defp normalize_chunk_size(other), do: raise(ArgumentError, "invalid chunk size: #{inspect(other)}")

  defp normalize_index(index, len) when index < 0, do: len + index
  defp normalize_index(index, _len), do: index

  defp fetch_by_index([], _index, _offset), do: nil

  defp fetch_by_index([chunk | rest], index, offset) do
    chunk_len = length(chunk)

    if index < offset + chunk_len do
      Enum.at(chunk, index - offset)
    else
      fetch_by_index(rest, index, offset + chunk_len)
    end
  end

  defp collect_range([], _start, _stop, _offset, acc), do: acc

  defp collect_range([chunk | rest], start, stop, offset, acc) do
    chunk_len = length(chunk)
    chunk_start = offset
    chunk_stop = offset + chunk_len - 1

    cond do
      stop < chunk_start ->
        acc

      start > chunk_stop ->
        collect_range(rest, start, stop, offset + chunk_len, acc)

      true ->
        from = max(start - offset, 0)
        to = min(stop - offset, chunk_len - 1)
        slice = Enum.slice(chunk, from..to)
        collect_range(rest, start, stop, offset + chunk_len, [slice | acc])
    end
  end
end
