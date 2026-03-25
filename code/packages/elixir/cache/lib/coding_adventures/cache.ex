defmodule CodingAdventures.CacheAccess do
  @moduledoc """
  Record of a single cache access.
  """

  @enforce_keys [:address, :hit, :tag, :set_index, :offset, :cycles]
  defstruct [:address, :hit, :tag, :set_index, :offset, :cycles, :evicted]

  @type t :: %__MODULE__{
          address: non_neg_integer(),
          hit: boolean(),
          tag: non_neg_integer(),
          set_index: non_neg_integer(),
          offset: non_neg_integer(),
          cycles: non_neg_integer(),
          evicted: CodingAdventures.CacheLine.t() | nil
        }
end

defmodule CodingAdventures.Cache do
  @moduledoc """
  A single configurable cache level.
  """

  import Bitwise

  alias CodingAdventures.CacheAccess
  alias CodingAdventures.CacheConfig
  alias CodingAdventures.CacheSet
  alias CodingAdventures.CacheStats

  @enforce_keys [:config, :sets]
  defstruct [:config, :sets, stats: %CacheStats{}, offset_bits: 0, set_bits: 0, set_mask: 0]

  @type t :: %__MODULE__{
          config: CacheConfig.t(),
          sets: [CacheSet.t()],
          stats: CacheStats.t(),
          offset_bits: non_neg_integer(),
          set_bits: non_neg_integer(),
          set_mask: non_neg_integer()
        }

  @spec new(CacheConfig.t()) :: t()
  def new(%CacheConfig{} = config) do
    num_sets = CacheConfig.num_sets(config)

    %__MODULE__{
      config: config,
      sets: Enum.map(1..num_sets, fn _ -> CacheSet.new(config.associativity, config.line_size) end),
      stats: %CacheStats{},
      offset_bits: int_log2(config.line_size),
      set_bits: if(num_sets > 1, do: int_log2(num_sets), else: 0),
      set_mask: num_sets - 1
    }
  end

  @spec decompose_address(t(), non_neg_integer()) ::
          {non_neg_integer(), non_neg_integer(), non_neg_integer()}
  def decompose_address(%__MODULE__{} = cache, address)
      when is_integer(address) and address >= 0 do
    offset = band(address, (1 <<< cache.offset_bits) - 1)
    set_index = band(address >>> cache.offset_bits, cache.set_mask)
    tag = address >>> (cache.offset_bits + cache.set_bits)
    {tag, set_index, offset}
  end

  @spec read(t(), non_neg_integer(), non_neg_integer(), non_neg_integer()) :: {t(), CacheAccess.t()}
  def read(%__MODULE__{} = cache, address, _size \\ 1, cycle \\ 0)
      when is_integer(address) and address >= 0 and is_integer(cycle) and cycle >= 0 do
    {tag, set_index, offset} = decompose_address(cache, address)
    cache_set = Enum.at(cache.sets, set_index)
    {{result, line}, updated_set} = normalize_access(CacheSet.access(cache_set, tag, cycle))

    case result do
      :hit ->
        updated =
          cache
          |> put_set(set_index, updated_set)
          |> update_stats(&CacheStats.record_read(&1, hit: true))

        {updated,
         %CacheAccess{
           address: address,
           hit: true,
           tag: tag,
           set_index: set_index,
           offset: offset,
           cycles: cache.config.access_latency
         }}

      :miss ->
        fill_data = List.duplicate(0, cache.config.line_size)
        {allocated_set, evicted, eviction?} = CacheSet.allocate(updated_set, tag, fill_data, cycle)

        updated =
          cache
          |> put_set(set_index, allocated_set)
          |> update_stats(&CacheStats.record_read(&1, hit: false))
          |> maybe_record_eviction(eviction?, evicted)

        _ = line

        {updated,
         %CacheAccess{
           address: address,
           hit: false,
           tag: tag,
           set_index: set_index,
           offset: offset,
           cycles: cache.config.access_latency,
           evicted: evicted
         }}
      end
  end

  @spec write(t(), non_neg_integer(), [non_neg_integer()] | nil, non_neg_integer()) ::
          {t(), CacheAccess.t()}
  def write(%__MODULE__{} = cache, address, data \\ nil, cycle \\ 0)
      when is_integer(address) and address >= 0 and is_integer(cycle) and cycle >= 0 do
    {tag, set_index, offset} = decompose_address(cache, address)
    cache_set = Enum.at(cache.sets, set_index)
    {{result, _line}, updated_set} = normalize_access(CacheSet.access(cache_set, tag, cycle))

    case result do
      :hit ->
        {:hit, way_index} = CacheSet.lookup(updated_set, tag)
        line = Enum.at(updated_set.lines, way_index)
        new_data = write_bytes(line.data, offset, data || [])
        dirty? = cache.config.write_policy == "write-back"
        updated_line = %{line | data: new_data, dirty: dirty?, last_access: cycle}
        final_set = %{updated_set | lines: List.replace_at(updated_set.lines, way_index, updated_line)}

        updated =
          cache
          |> put_set(set_index, final_set)
          |> update_stats(&CacheStats.record_write(&1, hit: true))

        {updated,
         %CacheAccess{
           address: address,
           hit: true,
           tag: tag,
           set_index: set_index,
           offset: offset,
           cycles: cache.config.access_latency
         }}

      :miss ->
        fill_data = write_bytes(List.duplicate(0, cache.config.line_size), offset, data || [])
        {allocated_set, evicted, eviction?} = CacheSet.allocate(updated_set, tag, fill_data, cycle)
        {:hit, way_index} = CacheSet.lookup(allocated_set, tag)
        line = Enum.at(allocated_set.lines, way_index)
        dirty? = cache.config.write_policy == "write-back"
        updated_line = %{line | dirty: dirty?}
        final_set = %{allocated_set | lines: List.replace_at(allocated_set.lines, way_index, updated_line)}

        updated =
          cache
          |> put_set(set_index, final_set)
          |> update_stats(&CacheStats.record_write(&1, hit: false))
          |> maybe_record_eviction(eviction?, evicted)

        {updated,
         %CacheAccess{
           address: address,
           hit: false,
           tag: tag,
           set_index: set_index,
           offset: offset,
           cycles: cache.config.access_latency,
           evicted: evicted
         }}
    end
  end

  @spec invalidate(t()) :: t()
  def invalidate(%__MODULE__{} = cache) do
    sets =
      Enum.map(cache.sets, fn cache_set ->
        %{cache_set | lines: Enum.map(cache_set.lines, &CodingAdventures.CacheLine.invalidate/1)}
      end)

    %{cache | sets: sets}
  end

  @spec fill_line(t(), non_neg_integer(), [non_neg_integer()], non_neg_integer()) ::
          {t(), CodingAdventures.CacheLine.t() | nil}
  def fill_line(%__MODULE__{} = cache, address, data, cycle \\ 0)
      when is_integer(address) and address >= 0 and is_list(data) and is_integer(cycle) and cycle >= 0 do
    {tag, set_index, _offset} = decompose_address(cache, address)
    cache_set = Enum.at(cache.sets, set_index)
    {updated_set, evicted, _eviction?} = CacheSet.allocate(cache_set, tag, data, cycle)
    {put_set(cache, set_index, updated_set), evicted}
  end

  defp int_log2(value), do: trunc(:math.log2(value))

  defp normalize_access({{:hit, line}, updated_set}), do: {{:hit, line}, updated_set}
  defp normalize_access({{:miss, line}, updated_set}), do: {{:miss, line}, updated_set}

  defp put_set(%__MODULE__{} = cache, index, cache_set) do
    %{cache | sets: List.replace_at(cache.sets, index, cache_set)}
  end

  defp update_stats(%__MODULE__{} = cache, fun), do: %{cache | stats: fun.(cache.stats)}

  defp maybe_record_eviction(%__MODULE__{} = cache, false, _evicted), do: cache

  defp maybe_record_eviction(%__MODULE__{} = cache, true, evicted) do
    dirty? = not is_nil(evicted)
    update_stats(cache, &CacheStats.record_eviction(&1, dirty: dirty?))
  end

  defp write_bytes(data, _offset, []), do: data

  defp write_bytes(data, offset, bytes) do
    Enum.with_index(bytes)
    |> Enum.reduce(data, fn {byte, index}, acc ->
      target = offset + index

      if target < length(acc) do
        List.replace_at(acc, target, byte)
      else
        acc
      end
    end)
  end
end
