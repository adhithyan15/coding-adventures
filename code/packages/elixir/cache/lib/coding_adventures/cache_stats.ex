defmodule CodingAdventures.CacheStats do
  @moduledoc """
  Statistics for one cache level.
  """

  defstruct reads: 0, writes: 0, hits: 0, misses: 0, evictions: 0, writebacks: 0

  @type t :: %__MODULE__{
          reads: non_neg_integer(),
          writes: non_neg_integer(),
          hits: non_neg_integer(),
          misses: non_neg_integer(),
          evictions: non_neg_integer(),
          writebacks: non_neg_integer()
        }

  @spec total_accesses(t()) :: non_neg_integer()
  def total_accesses(%__MODULE__{} = stats), do: stats.reads + stats.writes

  @spec hit_rate(t()) :: float()
  def hit_rate(%__MODULE__{} = stats) do
    accesses = total_accesses(stats)
    if accesses == 0, do: 0.0, else: stats.hits / accesses
  end

  @spec miss_rate(t()) :: float()
  def miss_rate(%__MODULE__{} = stats) do
    accesses = total_accesses(stats)
    if accesses == 0, do: 0.0, else: stats.misses / accesses
  end

  @spec record_read(t(), keyword()) :: t()
  def record_read(%__MODULE__{} = stats, hit: true), do: %{stats | reads: stats.reads + 1, hits: stats.hits + 1}
  def record_read(%__MODULE__{} = stats, hit: false), do: %{stats | reads: stats.reads + 1, misses: stats.misses + 1}

  @spec record_write(t(), keyword()) :: t()
  def record_write(%__MODULE__{} = stats, hit: true), do: %{stats | writes: stats.writes + 1, hits: stats.hits + 1}
  def record_write(%__MODULE__{} = stats, hit: false), do: %{stats | writes: stats.writes + 1, misses: stats.misses + 1}

  @spec record_eviction(t(), keyword()) :: t()
  def record_eviction(%__MODULE__{} = stats, dirty: true) do
    %{stats | evictions: stats.evictions + 1, writebacks: stats.writebacks + 1}
  end

  def record_eviction(%__MODULE__{} = stats, dirty: false) do
    %{stats | evictions: stats.evictions + 1}
  end

  @spec reset(t()) :: t()
  def reset(%__MODULE__{} = _stats), do: %__MODULE__{}
end
