defmodule CodingAdventures.HashMap do
  @moduledoc """
  Immutable hash map with chaining or open addressing.
  """

  import Bitwise

  @empty :empty
  @tombstone :tombstone

  @enforce_keys [:strategy, :hash_fn, :capacity, :size, :buckets, :slots]
  defstruct [:strategy, :hash_fn, :capacity, :size, :buckets, :slots]

  @type strategy :: :chaining | :open_addressing
  @type hash_fn :: :phash2 | :djb2 | :fnv1a | :sha256
  @type t :: %__MODULE__{
          strategy: strategy(),
          hash_fn: hash_fn(),
          capacity: pos_integer(),
          size: non_neg_integer(),
          buckets: [[{any(), any()}]],
          slots: [empty_slot()]
        }

  @type empty_slot :: :empty | :tombstone | {:occupied, any(), any()}

  def new(opts \\ []) do
    strategy = normalize_strategy(Keyword.get(opts, :strategy, :chaining))
    hash_fn = normalize_hash_fn(Keyword.get(opts, :hash_fn, :phash2))
    capacity = max(1, Keyword.get(opts, :capacity, 16))

    %__MODULE__{
      strategy: strategy,
      hash_fn: hash_fn,
      capacity: capacity,
      size: 0,
      buckets: if(strategy == :chaining, do: List.duplicate([], capacity), else: []),
      slots: if(strategy == :open_addressing, do: List.duplicate(@empty, capacity), else: [])
    }
  end

  def from_list(entries, opts \\ []) when is_list(entries) do
    Enum.reduce(entries, new(opts), fn
      {key, value}, acc -> put(acc, key, value)
      entry, _acc -> raise ArgumentError, "expected {key, value} tuple, got #{inspect(entry)}"
    end)
  end

  def size(%__MODULE__{size: size}), do: size
  def capacity(%__MODULE__{capacity: capacity}), do: capacity
  def strategy(%__MODULE__{strategy: strategy}), do: strategy
  def hash_fn(%__MODULE__{hash_fn: hash_fn}), do: hash_fn

  def load_factor(%__MODULE__{size: size, capacity: capacity}), do: size / capacity

  def has_key?(map, key), do: get(map, key) != nil

  def get(%__MODULE__{strategy: :chaining, buckets: buckets, capacity: _capacity} = map, key) do
    index = bucket_index(map, key)

    buckets
    |> Enum.at(index, [])
    |> Enum.find(fn {existing_key, _value} -> existing_key == key end)
    |> case do
      {^key, value} -> value
      _ -> nil
    end
  end

  def get(%__MODULE__{strategy: :open_addressing} = map, key) do
    {_, result} = open_address_lookup(map, key)
    result
  end

  def put(%__MODULE__{strategy: :chaining} = map, key, value) do
    index = bucket_index(map, key)
    bucket = Enum.at(map.buckets, index, [])

    {next_bucket, replaced?} = replace_bucket_entry(bucket, key, value)

    next_map =
      %{map | buckets: List.replace_at(map.buckets, index, next_bucket)}
      |> maybe_bump_size(not replaced?)

    maybe_resize(next_map)
  end

  def put(%__MODULE__{strategy: :open_addressing} = map, key, value) do
    map = if map.size >= map.capacity, do: resize(map, map.capacity * 2), else: map

    case open_address_insert_slot(map, key) do
      {:found, index} ->
        %{map | slots: List.replace_at(map.slots, index, {:occupied, key, value})}

      {:insert, index} ->
        next_map =
          %{map | slots: List.replace_at(map.slots, index, {:occupied, key, value})}
          |> maybe_bump_size(true)

        maybe_resize(next_map)

      :full ->
        put(resize(map, map.capacity * 2), key, value)
    end
  end

  def delete(%__MODULE__{strategy: :chaining} = map, key) do
    index = bucket_index(map, key)
    bucket = Enum.at(map.buckets, index, [])
    {next_bucket, removed?} = delete_bucket_entry(bucket, key)

    if removed? do
      %{map | buckets: List.replace_at(map.buckets, index, next_bucket), size: map.size - 1}
    else
      map
    end
  end

  def delete(%__MODULE__{strategy: :open_addressing} = map, key) do
    case open_address_lookup(map, key) do
      {index, _value} when not is_nil(index) ->
        %{map | slots: List.replace_at(map.slots, index, @tombstone), size: map.size - 1}

      _ ->
        map
    end
  end

  def entries(%__MODULE__{strategy: :chaining, buckets: buckets}) do
    buckets
    |> Enum.flat_map(& &1)
  end

  def entries(%__MODULE__{strategy: :open_addressing, slots: slots}) do
    Enum.flat_map(slots, fn
      {:occupied, key, value} -> [{key, value}]
      _ -> []
    end)
  end

  def keys(map), do: map |> entries() |> Enum.map(&elem(&1, 0))
  def values(map), do: map |> entries() |> Enum.map(&elem(&1, 1))

  defp normalize_strategy(strategy) when strategy in [:chaining, "chaining"], do: :chaining
  defp normalize_strategy(strategy) when strategy in [:open_addressing, :open, "open_addressing", "open-addressing", "open"], do: :open_addressing
  defp normalize_strategy(other), do: raise(ArgumentError, "unknown hash map strategy: #{inspect(other)}")

  defp normalize_hash_fn(hash_fn) when hash_fn in [:phash2, "phash2"], do: :phash2
  defp normalize_hash_fn(hash_fn) when hash_fn in [:djb2, "djb2"], do: :djb2
  defp normalize_hash_fn(hash_fn) when hash_fn in [:fnv1a, "fnv1a"], do: :fnv1a
  defp normalize_hash_fn(hash_fn) when hash_fn in [:sha256, "sha256"], do: :sha256
  defp normalize_hash_fn(other), do: raise(ArgumentError, "unknown hash function: #{inspect(other)}")

  defp maybe_bump_size(map, true), do: %{map | size: map.size + 1}
  defp maybe_bump_size(map, false), do: map

  defp maybe_resize(%__MODULE__{strategy: :chaining} = map) do
    if load_factor(map) > 1.0, do: resize(map, map.capacity * 2), else: map
  end

  defp maybe_resize(%__MODULE__{strategy: :open_addressing} = map) do
    if load_factor(map) > 0.75, do: resize(map, map.capacity * 2), else: map
  end

  defp resize(%__MODULE__{strategy: strategy, hash_fn: hash_fn} = map, new_capacity) do
    entries = entries(map)

    Enum.reduce(entries, %__MODULE__{
      strategy: strategy,
      hash_fn: hash_fn,
      capacity: max(1, new_capacity),
      size: 0,
      buckets: if(strategy == :chaining, do: List.duplicate([], max(1, new_capacity)), else: []),
      slots: if(strategy == :open_addressing, do: List.duplicate(@empty, max(1, new_capacity)), else: [])
    }, fn {key, value}, acc ->
      put_without_resize(acc, key, value)
    end)
  end

  defp put_without_resize(%__MODULE__{strategy: :chaining} = map, key, value) do
    index = bucket_index(map, key)
    bucket = Enum.at(map.buckets, index, [])

    {next_bucket, replaced?} = replace_bucket_entry(bucket, key, value)

    %{map | buckets: List.replace_at(map.buckets, index, next_bucket)}
    |> maybe_bump_size(not replaced?)
  end

  defp put_without_resize(%__MODULE__{strategy: :open_addressing} = map, key, value) do
    case open_address_insert_slot(map, key) do
      {:found, index} ->
        %{map | slots: List.replace_at(map.slots, index, {:occupied, key, value})}

      {:insert, index} ->
        %{map | slots: List.replace_at(map.slots, index, {:occupied, key, value}), size: map.size + 1}

      :full ->
        raise "hash map resize failed: table was unexpectedly full"
    end
  end

  defp replace_bucket_entry(bucket, key, value) do
    {reversed, replaced?} =
      Enum.reduce(bucket, {[], false}, fn
        {existing_key, _old_value}, {_acc, true} = acc when existing_key == key ->
          acc

        {existing_key, _old_value}, {acc, false} when existing_key == key ->
          {[{key, value} | acc], true}

        entry, {acc, replaced?} ->
          {[entry | acc], replaced?}
      end)

    next_bucket =
      if replaced? do
        Enum.reverse(reversed)
      else
        Enum.reverse(reversed) ++ [{key, value}]
      end

    {next_bucket, replaced?}
  end

  defp delete_bucket_entry(bucket, key) do
    {reversed, removed?} =
      Enum.reduce(bucket, {[], false}, fn
        {existing_key, _old_value}, {acc, false} when existing_key == key ->
          {acc, true}

        entry, {acc, removed?} ->
          {[entry | acc], removed?}
      end)

    {Enum.reverse(reversed), removed?}
  end

  defp open_address_lookup(%__MODULE__{slots: slots} = map, key) do
    start = bucket_index(map, key)
    open_address_lookup(slots, start, key, map.capacity, 0)
  end

  defp open_address_lookup(_slots, _start, _key, capacity, probe) when probe >= capacity, do: {nil, nil}

  defp open_address_lookup(slots, start, key, capacity, probe) do
    index = rem(start + probe, capacity)

    case Enum.at(slots, index) do
      @empty -> {nil, nil}
      @tombstone -> open_address_lookup(slots, start, key, capacity, probe + 1)
      {:occupied, ^key, value} -> {index, value}
      {:occupied, _other_key, _value} -> open_address_lookup(slots, start, key, capacity, probe + 1)
    end
  end

  defp open_address_insert_slot(%__MODULE__{slots: slots} = map, key) do
    start = bucket_index(map, key)
    open_address_insert_slot(slots, start, key, map.capacity, 0, nil)
  end

  defp open_address_insert_slot(_slots, _start, _key, capacity, probe, first_tombstone)
       when probe >= capacity do
    case first_tombstone do
      nil -> :full
      index -> {:insert, index}
    end
  end

  defp open_address_insert_slot(slots, start, key, capacity, probe, first_tombstone) do
    index = rem(start + probe, capacity)

    case Enum.at(slots, index) do
      @empty ->
        case first_tombstone do
          nil -> {:insert, index}
          tombstone_index -> {:insert, tombstone_index}
        end

      @tombstone ->
        next_first = first_tombstone || index
        open_address_insert_slot(slots, start, key, capacity, probe + 1, next_first)

      {:occupied, ^key, _value} ->
        {:found, index}

      {:occupied, _other_key, _value} ->
        open_address_insert_slot(slots, start, key, capacity, probe + 1, first_tombstone)
    end
  end

  defp bucket_index(%__MODULE__{} = map, key) do
    case map.hash_fn do
      :phash2 -> :erlang.phash2(serialize_key(key), map.capacity)
      :djb2 -> rem(djb2_hash(serialize_key(key)), map.capacity)
      :fnv1a -> rem(fnv1a_32(serialize_key(key)), map.capacity)
      :sha256 -> rem(sha256_hash(serialize_key(key)), map.capacity)
    end
  end

  defp serialize_key(key), do: inspect(key, limit: :infinity, printable_limit: :infinity)

  defp djb2_hash(bytes) do
    bytes
    |> :binary.bin_to_list()
    |> Enum.reduce(5381, fn byte, hash -> rem(((hash <<< 5) + hash) + byte, 1 <<< 32) end)
  end

  defp fnv1a_32(bytes) do
    bytes
    |> :binary.bin_to_list()
    |> Enum.reduce(0x811C9DC5, fn byte, hash ->
      hash = Bitwise.bxor(hash, byte)
      rem(hash * 0x01000193, 1 <<< 32)
    end)
  end

  defp sha256_hash(bytes) do
    <<hash::unsigned-64, _::binary>> = :crypto.hash(:sha256, bytes)
    hash
  end
end
