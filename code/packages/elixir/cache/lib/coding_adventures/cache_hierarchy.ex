defmodule CodingAdventures.HierarchyAccess do
  @moduledoc """
  Record of one full hierarchy access.
  """

  @enforce_keys [:address, :served_by, :total_cycles, :hit_at_level]
  defstruct [:address, :served_by, :total_cycles, :hit_at_level, level_accesses: []]

  @type t :: %__MODULE__{
          address: non_neg_integer(),
          served_by: String.t(),
          total_cycles: non_neg_integer(),
          hit_at_level: non_neg_integer(),
          level_accesses: [CodingAdventures.CacheAccess.t()]
        }
end

defmodule CodingAdventures.CacheHierarchy do
  @moduledoc """
  Inclusive multi-level cache hierarchy.
  """

  alias CodingAdventures.Cache
  alias CodingAdventures.HierarchyAccess

  defstruct l1i: nil, l1d: nil, l2: nil, l3: nil, main_memory_latency: 100

  @type t :: %__MODULE__{
          l1i: Cache.t() | nil,
          l1d: Cache.t() | nil,
          l2: Cache.t() | nil,
          l3: Cache.t() | nil,
          main_memory_latency: non_neg_integer()
        }

  @spec new(keyword()) :: t()
  def new(opts \\ []) do
    %__MODULE__{
      l1i: Keyword.get(opts, :l1i),
      l1d: Keyword.get(opts, :l1d),
      l2: Keyword.get(opts, :l2),
      l3: Keyword.get(opts, :l3),
      main_memory_latency: Keyword.get(opts, :main_memory_latency, 100)
    }
  end

  @spec read(t(), non_neg_integer(), boolean(), non_neg_integer()) :: {t(), HierarchyAccess.t()}
  def read(%__MODULE__{} = hierarchy, address, is_instruction \\ false, cycle \\ 0)
      when is_integer(address) and address >= 0 and is_boolean(is_instruction) and is_integer(cycle) and cycle >= 0 do
    levels = level_specs(hierarchy, is_instruction)

    if levels == [] do
      {hierarchy,
       %HierarchyAccess{
         address: address,
         served_by: "memory",
         total_cycles: hierarchy.main_memory_latency,
         hit_at_level: 0,
         level_accesses: []
       }}
    else
      do_read(hierarchy, levels, address, cycle)
    end
  end

  @spec write(t(), non_neg_integer(), [non_neg_integer()] | nil, non_neg_integer()) ::
          {t(), HierarchyAccess.t()}
  def write(%__MODULE__{} = hierarchy, address, data \\ nil, cycle \\ 0)
      when is_integer(address) and address >= 0 and is_integer(cycle) and cycle >= 0 do
    levels = level_specs(hierarchy, false)

    if levels == [] do
      {hierarchy,
       %HierarchyAccess{
         address: address,
         served_by: "memory",
         total_cycles: hierarchy.main_memory_latency,
         hit_at_level: 0,
         level_accesses: []
       }}
    else
      [{first_name, first_key, first_cache} | rest] = levels
      {updated_first, first_access} = Cache.write(first_cache, address, data, cycle)
      hierarchy = put_level(hierarchy, first_key, updated_first)

      if first_access.hit do
        {hierarchy,
         %HierarchyAccess{
           address: address,
           served_by: first_name,
           total_cycles: first_cache.config.access_latency,
           hit_at_level: 0,
           level_accesses: [first_access]
         }}
      else
        {hierarchy, accesses, total_cycles, served_by, hit_level} =
          Enum.reduce_while(Enum.with_index(rest, 1), {hierarchy, [first_access], first_cache.config.access_latency, "memory", length(levels)}, fn {{name, key, cache}, index}, {acc_h, acc_accesses, acc_cycles, _served, _hit} ->
            {updated_cache, access} = Cache.read(cache, address, 1, cycle)
            next_h = put_level(acc_h, key, updated_cache)
            next_cycles = acc_cycles + cache.config.access_latency

            if access.hit do
              {:halt, {next_h, acc_accesses ++ [access], next_cycles, name, index}}
            else
              {:cont, {next_h, acc_accesses ++ [access], next_cycles, "memory", length(levels)}}
            end
          end)

        total_cycles =
          if served_by == "memory",
            do: total_cycles + hierarchy.main_memory_latency,
            else: total_cycles

        {hierarchy,
         %HierarchyAccess{
           address: address,
           served_by: served_by,
           total_cycles: total_cycles,
           hit_at_level: hit_level,
           level_accesses: accesses
         }}
      end
    end
  end

  @spec invalidate_all(t()) :: t()
  def invalidate_all(%__MODULE__{} = hierarchy) do
    hierarchy
    |> maybe_invalidate(:l1i)
    |> maybe_invalidate(:l1d)
    |> maybe_invalidate(:l2)
    |> maybe_invalidate(:l3)
  end

  @spec reset_stats(t()) :: t()
  def reset_stats(%__MODULE__{} = hierarchy) do
    hierarchy
    |> maybe_reset_stats(:l1i)
    |> maybe_reset_stats(:l1d)
    |> maybe_reset_stats(:l2)
    |> maybe_reset_stats(:l3)
  end

  defp do_read(hierarchy, levels, address, cycle) do
    {hierarchy, accesses, total_cycles, served_by, hit_level} =
      Enum.reduce_while(Enum.with_index(levels), {hierarchy, [], 0, "memory", length(levels)}, fn {{name, key, cache}, index}, {acc_h, acc_accesses, acc_cycles, _served, _hit} ->
        {updated_cache, access} = Cache.read(cache, address, 1, cycle)
        next_h = put_level(acc_h, key, updated_cache)
        next_cycles = acc_cycles + cache.config.access_latency

        if access.hit do
          {:halt, {next_h, acc_accesses ++ [access], next_cycles, name, index}}
        else
          {:cont, {next_h, acc_accesses ++ [access], next_cycles, "memory", length(levels)}}
        end
      end)

    total_cycles =
      if served_by == "memory",
        do: total_cycles + hierarchy.main_memory_latency,
        else: total_cycles

    fill_data = List.duplicate(0, get_line_size(levels))

    hierarchy =
      if hit_level > 0 do
        fill_higher_levels(hierarchy, Enum.take(levels, hit_level), address, fill_data, cycle)
      else
        hierarchy
      end

    {hierarchy,
     %HierarchyAccess{
       address: address,
       served_by: served_by,
       total_cycles: total_cycles,
       hit_at_level: hit_level,
       level_accesses: accesses
     }}
  end

  defp fill_higher_levels(hierarchy, levels, address, fill_data, cycle) do
    Enum.reduce(levels, hierarchy, fn {_name, key, cache}, acc ->
      current_cache = Map.fetch!(acc, key)
      {updated_cache, _evicted} = Cache.fill_line(current_cache || cache, address, fill_data, cycle)
      put_level(acc, key, updated_cache)
    end)
  end

  defp level_specs(%__MODULE__{} = hierarchy, true) do
    []
    |> maybe_append_level("L1I", :l1i, hierarchy.l1i)
    |> maybe_append_level("L2", :l2, hierarchy.l2)
    |> maybe_append_level("L3", :l3, hierarchy.l3)
  end

  defp level_specs(%__MODULE__{} = hierarchy, false) do
    []
    |> maybe_append_level("L1D", :l1d, hierarchy.l1d)
    |> maybe_append_level("L2", :l2, hierarchy.l2)
    |> maybe_append_level("L3", :l3, hierarchy.l3)
  end

  defp maybe_append_level(levels, _name, _key, nil), do: levels
  defp maybe_append_level(levels, name, key, cache), do: levels ++ [{name, key, cache}]

  defp get_line_size([{_name, _key, cache} | _]), do: cache.config.line_size
  defp get_line_size([]), do: 64

  defp put_level(%__MODULE__{} = hierarchy, key, value), do: Map.put(hierarchy, key, value)

  defp maybe_invalidate(%__MODULE__{} = hierarchy, key) do
    case Map.get(hierarchy, key) do
      nil -> hierarchy
      cache -> Map.put(hierarchy, key, Cache.invalidate(cache))
    end
  end

  defp maybe_reset_stats(%__MODULE__{} = hierarchy, key) do
    case Map.get(hierarchy, key) do
      nil -> hierarchy
      cache -> Map.put(hierarchy, key, %{cache | stats: CodingAdventures.CacheStats.reset(cache.stats)})
    end
  end
end
