defmodule CodingAdventures.BloomFilter do
  @moduledoc """
  Immutable Bloom filter for probabilistic membership checks.
  """

  import Bitwise

  @default_expected_items 1_000
  @default_false_positive_rate 0.01
  @mask32 0xFFFF_FFFF

  defstruct bit_count: 0,
            hash_count: 0,
            expected_items: 0,
            bits: <<>>,
            bits_set: 0,
            items_added: 0

  @type t :: %__MODULE__{
          bit_count: pos_integer(),
          hash_count: pos_integer(),
          expected_items: non_neg_integer(),
          bits: binary(),
          bits_set: non_neg_integer(),
          items_added: non_neg_integer()
        }

  def new(
        expected_items \\ @default_expected_items,
        false_positive_rate \\ @default_false_positive_rate
      ) do
    validate_expected_items!(expected_items)
    validate_false_positive_rate!(false_positive_rate)

    bit_count = optimal_m(expected_items, false_positive_rate)
    hash_count = optimal_k(bit_count, expected_items)
    from_parts(bit_count, hash_count, expected_items)
  end

  def from_params(bit_count, hash_count) do
    validate_bit_count!(bit_count)
    validate_hash_count!(hash_count)
    from_parts(bit_count, hash_count, 0)
  end

  def add(%__MODULE__{} = filter, element) do
    {bits, bits_set} =
      element
      |> hash_indices(filter)
      |> Enum.reduce({filter.bits, filter.bits_set}, fn idx, {bits, bits_set} ->
        set_bit(bits, bits_set, idx)
      end)

    %{filter | bits: bits, bits_set: bits_set, items_added: filter.items_added + 1}
  end

  def contains?(%__MODULE__{} = filter, element) do
    element
    |> hash_indices(filter)
    |> Enum.all?(&bit_set?(filter.bits, &1))
  end

  def fill_ratio(%__MODULE__{bit_count: 0}), do: 0.0

  def fill_ratio(%__MODULE__{} = filter) do
    filter.bits_set / filter.bit_count
  end

  def estimated_false_positive_rate(%__MODULE__{bits_set: 0}), do: 0.0

  def estimated_false_positive_rate(%__MODULE__{} = filter) do
    :math.pow(fill_ratio(filter), filter.hash_count)
  end

  def over_capacity?(%__MODULE__{} = filter) do
    filter.expected_items > 0 and filter.items_added > filter.expected_items
  end

  def size_bytes(%__MODULE__{} = filter), do: byte_size(filter.bits)

  def optimal_m(expected_items, false_positive_rate) do
    Float.ceil(-expected_items * :math.log(false_positive_rate) / :math.pow(:math.log(2), 2))
    |> trunc()
  end

  def optimal_k(bit_count, expected_items) do
    max(1, round(bit_count / expected_items * :math.log(2)))
  end

  def capacity_for_memory(memory_bytes, false_positive_rate) do
    Float.floor(-memory_bytes * 8 * :math.pow(:math.log(2), 2) / :math.log(false_positive_rate))
    |> trunc()
  end

  defp from_parts(bit_count, hash_count, expected_items) do
    %__MODULE__{
      bit_count: bit_count,
      hash_count: hash_count,
      expected_items: expected_items,
      bits: :binary.copy(<<0>>, div(bit_count + 7, 8))
    }
  end

  defp validate_expected_items!(expected_items)
       when is_integer(expected_items) and expected_items > 0, do: :ok

  defp validate_expected_items!(expected_items) do
    raise ArgumentError,
          "expected_items must be a positive integer, got #{inspect(expected_items)}"
  end

  defp validate_false_positive_rate!(rate) when is_number(rate) and rate > 0 and rate < 1, do: :ok

  defp validate_false_positive_rate!(rate) do
    raise ArgumentError,
          "false_positive_rate must be in the open interval (0, 1), got #{inspect(rate)}"
  end

  defp validate_bit_count!(bit_count) when is_integer(bit_count) and bit_count > 0, do: :ok

  defp validate_bit_count!(bit_count) do
    raise ArgumentError, "bit_count must be a positive integer, got #{inspect(bit_count)}"
  end

  defp validate_hash_count!(hash_count) when is_integer(hash_count) and hash_count > 0, do: :ok

  defp validate_hash_count!(hash_count) do
    raise ArgumentError, "hash_count must be a positive integer, got #{inspect(hash_count)}"
  end

  defp hash_indices(element, %__MODULE__{} = filter) do
    raw = element |> to_string() |> :unicode.characters_to_binary()
    h1 = raw |> fnv1a32() |> fmix32()
    h2 = raw |> djb2() |> fmix32() ||| 1

    Enum.map(0..(filter.hash_count - 1), fn i ->
      rem(h1 + i * h2, filter.bit_count)
    end)
  end

  defp set_bit(bits, bits_set, idx) do
    byte_idx = div(idx, 8)
    bit_mask = 1 <<< rem(idx, 8)
    <<prefix::binary-size(byte_idx), byte, suffix::binary>> = bits

    if (byte &&& bit_mask) == 0 do
      {prefix <> <<byte ||| bit_mask>> <> suffix, bits_set + 1}
    else
      {bits, bits_set}
    end
  end

  defp bit_set?(bits, idx) do
    byte_idx = div(idx, 8)
    bit_mask = 1 <<< rem(idx, 8)
    <<_prefix::binary-size(byte_idx), byte, _suffix::binary>> = bits
    (byte &&& bit_mask) != 0
  end

  defp fnv1a32(bytes) do
    bytes
    |> :binary.bin_to_list()
    |> Enum.reduce(0x811C9DC5, fn byte, hash ->
      bxor(hash, byte) * 0x01000193 &&& @mask32
    end)
  end

  defp djb2(bytes) do
    bytes
    |> :binary.bin_to_list()
    |> Enum.reduce(5_381, fn byte, hash ->
      hash * 33 + byte &&& @mask32
    end)
  end

  defp fmix32(hash) do
    hash = bxor(hash, hash >>> 16)
    hash = hash * 0x85EBCA6B &&& @mask32
    hash = bxor(hash, hash >>> 13)
    hash = hash * 0xC2B2AE35 &&& @mask32
    bxor(hash, hash >>> 16) &&& @mask32
  end
end
