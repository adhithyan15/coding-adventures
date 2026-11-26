defmodule CodingAdventures.TestBootstrap.Fuzz.Bitmap do
  @moduledoc """
  A 65,536-bit execution map (VER06 section 4.1); bit `e` set means edge `e` was
  executed.  It is a VALUE (the campaign threads it): a `MapSet` of reduced edge
  ids, with `raw/1` rendering the 8,192-byte map on demand.  Ids are reduced with
  floored `Integer.mod/2` (collisions are the known price of a fixed-size map;
  AFL pays the same one), so `-1` is edge 65,535.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.Fuzz.Limits

  defstruct set: MapSet.new()

  defimpl Inspect do
    # Which edges were hit is the campaign's business, not a log line's.
    def inspect(bm, _opts), do: "#Fuzz.Bitmap<edges=#{MapSet.size(bm.set)}>"
  end

  def new, do: %__MODULE__{}

  def reduce(edge), do: Integer.mod(Limits.check_int!(edge, "edge"), Limits.coverage_map_size())

  def record(%__MODULE__{set: s} = bm, edge), do: %{bm | set: MapSet.put(s, reduce(edge))}

  def has?(%__MODULE__{set: s}, edge), do: MapSet.member?(s, reduce(edge))

  @doc "ORs a trace into this (cumulative) map; returns `{has_new, count_new, bitmap}`."
  def admit(%__MODULE__{set: s} = bm, trace) do
    novel = trace |> Enum.map(&reduce/1) |> MapSet.new() |> MapSet.difference(s)
    {MapSet.size(novel) > 0, MapSet.size(novel), %{bm | set: MapSet.union(s, novel)}}
  end

  def covered_edges(%__MODULE__{set: s}), do: s |> MapSet.to_list() |> Enum.sort()

  def count(%__MODULE__{set: s}), do: MapSet.size(s)

  def merge(%__MODULE__{set: a} = bm, %__MODULE__{set: b}), do: %{bm | set: MapSet.union(a, b)}

  def reset(%__MODULE__{} = bm), do: %{bm | set: MapSet.new()}

  @doc "The 8,192-byte map: bit `e mod 8` of byte `e div 8`."
  def raw(%__MODULE__{set: s}) do
    bytes =
      Enum.reduce(s, %{}, fn e, acc ->
        Map.update(acc, div(e, 8), 1 <<< rem(e, 8), &(&1 ||| 1 <<< rem(e, 8)))
      end)

    for i <- 0..(Limits.coverage_bytes() - 1), into: <<>>, do: <<Map.get(bytes, i, 0)>>
  end

  @doc """
  Pure novelty check: does `trace` hold an edge absent from `existing`?  Returns
  `{admitted, new_edges}` counting UNIQUE novel edges over the full, UNREDUCED ids.

      corpus_admit([1, 2, 3], [2, 4])  #=> {true, 1}
      corpus_admit([1, 2, 3], [1, 2])  #=> {false, 0}
  """
  def corpus_admit(existing, trace) when is_list(existing) and is_list(trace) do
    Enum.each(existing ++ trace, &Limits.check_int!(&1, "edge"))
    novel = MapSet.difference(MapSet.new(trace), MapSet.new(existing))
    {MapSet.size(novel) > 0, MapSet.size(novel)}
  end

  @doc """
  Mutation slots for a seed: `max(1, round(base * (1 + new/total) / sqrt(len)))`.
  The edge bonus is 1.0 while nothing is covered.  Rounding is half AWAY FROM ZERO
  (`floor(x + 0.5)`) so every lane agrees on exact halves; `Float.round/1` is not
  used.

  base 10, len 4, 3 new of 6 total: `10 * 1.5 / 2 = 7.5` rounds to 8.
  """
  def calculate_energy(base_energy, seed_len, new_edges, total_edges) do
    base = max(1, base_energy) * 1.0
    bonus = if total_edges > 0, do: 1.0 + new_edges / total_edges, else: 1.0
    slots = floor(base * bonus / :math.sqrt(max(1, seed_len)) + 0.5)
    max(1, slots)
  end
end
