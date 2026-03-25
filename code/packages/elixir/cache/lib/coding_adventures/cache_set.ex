defmodule CodingAdventures.CacheConfig do
  @moduledoc """
  Cache-level configuration.
  """

  @enforce_keys [:name, :total_size]
  defstruct name: nil,
            total_size: nil,
            line_size: 64,
            associativity: 4,
            access_latency: 1,
            write_policy: "write-back"

  @type t :: %__MODULE__{
          name: String.t(),
          total_size: pos_integer(),
          line_size: pos_integer(),
          associativity: pos_integer(),
          access_latency: non_neg_integer(),
          write_policy: String.t()
        }

  @spec new(keyword()) :: {:ok, t()} | {:error, String.t()}
  def new(attrs) when is_list(attrs) do
    config = struct(__MODULE__, attrs)

    with :ok <- validate(config) do
      {:ok, config}
    end
  end

  @spec new!(keyword()) :: t()
  def new!(attrs) do
    case new(attrs) do
      {:ok, config} -> config
      {:error, message} -> raise ArgumentError, message
    end
  end

  @spec num_lines(t()) :: pos_integer()
  def num_lines(%__MODULE__{} = config), do: div(config.total_size, config.line_size)

  @spec num_sets(t()) :: pos_integer()
  def num_sets(%__MODULE__{} = config), do: div(num_lines(config), config.associativity)

  defp validate(%__MODULE__{name: name}) when not is_binary(name) or byte_size(name) == 0 do
    {:error, "name must be a non-empty string"}
  end

  defp validate(%__MODULE__{total_size: total_size}) when not is_integer(total_size) or total_size <= 0 do
    {:error, "total_size must be positive, got #{inspect(total_size)}"}
  end

  defp validate(%__MODULE__{line_size: line_size})
       when not is_integer(line_size) or line_size <= 0 do
    {:error, "line_size must be a positive power of 2, got #{inspect(line_size)}"}
  end

  defp validate(%__MODULE__{line_size: line_size}) when not (Bitwise.band(line_size, line_size - 1) == 0) do
    {:error, "line_size must be a positive power of 2, got #{inspect(line_size)}"}
  end

  defp validate(%__MODULE__{associativity: associativity})
       when not is_integer(associativity) or associativity <= 0 do
    {:error, "associativity must be positive, got #{inspect(associativity)}"}
  end

  defp validate(%__MODULE__{access_latency: access_latency})
       when not is_integer(access_latency) or access_latency < 0 do
    {:error, "access_latency must be non-negative, got #{inspect(access_latency)}"}
  end

  defp validate(%__MODULE__{write_policy: write_policy})
       when write_policy not in ["write-back", "write-through"] do
    {:error, "write_policy must be 'write-back' or 'write-through', got #{inspect(write_policy)}"}
  end

  defp validate(%__MODULE__{} = config) do
    product = config.line_size * config.associativity

    if rem(config.total_size, product) == 0 do
      :ok
    else
      {:error,
       "total_size (#{config.total_size}) must be divisible by line_size * associativity (#{product})"}
    end
  end
end

defmodule CodingAdventures.CacheSet do
  @moduledoc """
  One set in a set-associative cache.
  """

  alias CodingAdventures.CacheLine

  @enforce_keys [:lines]
  defstruct [:lines]

  @type t :: %__MODULE__{lines: [CacheLine.t()]}

  @spec new(pos_integer(), pos_integer()) :: t()
  def new(associativity, line_size)
      when is_integer(associativity) and associativity > 0 and is_integer(line_size) and line_size > 0 do
    %__MODULE__{lines: Enum.map(1..associativity, fn _ -> CacheLine.new(line_size) end)}
  end

  @spec lookup(t(), non_neg_integer()) :: {:hit, non_neg_integer()} | :miss
  def lookup(%__MODULE__{} = cache_set, tag) when is_integer(tag) and tag >= 0 do
    case Enum.find_index(cache_set.lines, &(&1.valid and &1.tag == tag)) do
      nil -> :miss
      index -> {:hit, index}
    end
  end

  @spec access(t(), non_neg_integer(), non_neg_integer()) ::
          {{:hit, CacheLine.t()} | {:miss, CacheLine.t()}, t()}
  def access(%__MODULE__{} = cache_set, tag, cycle)
      when is_integer(tag) and tag >= 0 and is_integer(cycle) and cycle >= 0 do
    case lookup(cache_set, tag) do
      {:hit, index} ->
        line = Enum.at(cache_set.lines, index)
        touched = CacheLine.touch(line, cycle)
        updated = %{cache_set | lines: List.replace_at(cache_set.lines, index, touched)}
        {{:hit, touched}, updated}

      :miss ->
        victim = Enum.at(cache_set.lines, find_lru_index(cache_set))
        {{:miss, victim}, cache_set}
    end
  end

  @spec allocate(t(), non_neg_integer(), [non_neg_integer()], non_neg_integer()) ::
          {t(), CacheLine.t() | nil, boolean()}
  def allocate(%__MODULE__{} = cache_set, tag, data, cycle)
      when is_integer(tag) and tag >= 0 and is_list(data) and is_integer(cycle) and cycle >= 0 do
    index = find_lru_index(cache_set)
    victim = Enum.at(cache_set.lines, index)
    eviction? = victim.valid

    evicted =
      if victim.valid and victim.dirty do
        %CacheLine{
          valid: true,
          dirty: true,
          tag: victim.tag,
          data: Enum.to_list(victim.data),
          last_access: victim.last_access
        }
      end

    replacement = CacheLine.fill(victim, tag, data, cycle)
    updated = %{cache_set | lines: List.replace_at(cache_set.lines, index, replacement)}
    {updated, evicted, eviction?}
  end

  @spec find_lru_index(t()) :: non_neg_integer()
  def find_lru_index(%__MODULE__{} = cache_set) do
    case Enum.find_index(cache_set.lines, &(not &1.valid)) do
      nil ->
        cache_set.lines
        |> Enum.with_index()
        |> Enum.min_by(fn {line, _index} -> line.last_access end)
        |> elem(1)

      index ->
        index
    end
  end
end
