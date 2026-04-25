defmodule CodingAdventures.HashFunctions do
  @moduledoc """
  Non-cryptographic hash functions implemented from scratch.

  The module provides FNV-1a, DJB2, polynomial rolling hash, MurmurHash3, and
  deterministic analysis helpers for avalanche and bucket distribution.
  """

  import Bitwise

  @fnv32_offset_basis 0x811C9DC5
  @fnv32_prime 0x01000193
  @fnv64_offset_basis 0xCBF29CE484222325
  @fnv64_prime 0x00000100000001B3
  @polynomial_rolling_default_base 31
  @polynomial_rolling_default_modulus (1 <<< 61) - 1
  @mask32 0xFFFFFFFF
  @mask64 0xFFFFFFFFFFFFFFFF
  @murmur3_c1 0xCC9E2D51
  @murmur3_c2 0x1B873593

  def fnv1a32(data) when is_binary(data) do
    data
    |> :binary.bin_to_list()
    |> Enum.reduce(@fnv32_offset_basis, fn byte, hash ->
      hash
      |> bxor(byte)
      |> then(&band(&1 * @fnv32_prime, @mask32))
    end)
  end

  def fnv1a64(data) when is_binary(data) do
    data
    |> :binary.bin_to_list()
    |> Enum.reduce(@fnv64_offset_basis, fn byte, hash ->
      hash
      |> bxor(byte)
      |> then(&band(&1 * @fnv64_prime, @mask64))
    end)
  end

  def djb2(data) when is_binary(data) do
    data
    |> :binary.bin_to_list()
    |> Enum.reduce(5381, fn byte, hash ->
      band((hash <<< 5) + hash + byte, @mask64)
    end)
  end

  def polynomial_rolling(
        data,
        base \\ @polynomial_rolling_default_base,
        modulus \\ @polynomial_rolling_default_modulus
      )

  def polynomial_rolling(_data, _base, modulus) when modulus <= 0 do
    raise ArgumentError, "modulus must be positive"
  end

  def polynomial_rolling(data, base, modulus) when is_binary(data) do
    data
    |> :binary.bin_to_list()
    |> Enum.reduce(0, fn byte, hash ->
      rem(hash * base + byte, modulus)
    end)
  end

  def murmur3_32(data, seed \\ 0) when is_binary(data) do
    length = byte_size(data)
    block_count = div(length, 4)

    block_indexes = if block_count == 0, do: [], else: 0..(block_count - 1)//1

    hash =
      block_indexes
      |> Enum.reduce(band(seed, @mask32), fn block_index, hash ->
        offset = block_index * 4
        <<k0::little-32>> = binary_part(data, offset, 4)

        k =
          k0
          |> then(&band(&1 * @murmur3_c1, @mask32))
          |> rotl32(15)
          |> then(&band(&1 * @murmur3_c2, @mask32))

        hash
        |> bxor(k)
        |> rotl32(13)
        |> then(&band(&1 * 5 + 0xE6546B64, @mask32))
      end)

    tail_offset = block_count * 4
    tail = binary_part(data, tail_offset, length - tail_offset)

    k =
      tail
      |> :binary.bin_to_list()
      |> Enum.with_index()
      |> Enum.reduce(0, fn {byte, index}, acc -> bxor(acc, byte <<< (index * 8)) end)

    hash =
      if byte_size(tail) > 0 do
        mixed_tail =
          k
          |> then(&band(&1 * @murmur3_c1, @mask32))
          |> rotl32(15)
          |> then(&band(&1 * @murmur3_c2, @mask32))

        bxor(hash, mixed_tail)
      else
        hash
      end

    hash
    |> bxor(length)
    |> fmix32()
  end

  def avalanche_score(hash_fn, output_bits, sample_size \\ 100) do
    if output_bits <= 0 or output_bits > 64,
      do: raise(ArgumentError, "output_bits must be in 1..64")

    if sample_size <= 0, do: raise(ArgumentError, "sample_size must be positive")

    {total_bit_flips, total_trials} =
      0..(sample_size - 1)//1
      |> Enum.reduce({0, 0}, fn sample_index, {flip_acc, trial_acc} ->
        input = deterministic_bytes(sample_index)
        original = hash_fn.(input)

        0..(byte_size(input) * 8 - 1)//1
        |> Enum.reduce({flip_acc, trial_acc}, fn bit_position,
                                                 {inner_flip_acc, inner_trial_acc} ->
          byte_index = div(bit_position, 8)
          bit_mask = 1 <<< rem(bit_position, 8)
          <<prefix::binary-size(byte_index), byte, suffix::binary>> = input
          flipped = <<prefix::binary, bxor(byte, bit_mask), suffix::binary>>
          diff = bxor(original, hash_fn.(flipped))
          {inner_flip_acc + popcount(diff), inner_trial_acc + output_bits}
        end)
      end)

    total_bit_flips / total_trials
  end

  def distribution_test(hash_fn, inputs, num_buckets) do
    if num_buckets <= 0, do: raise(ArgumentError, "num_buckets must be positive")
    if inputs == [], do: raise(ArgumentError, "inputs must not be empty")

    counts =
      Enum.reduce(inputs, List.duplicate(0, num_buckets), fn input, counts ->
        bucket = rem(hash_fn.(input), num_buckets)
        List.update_at(counts, bucket, &(&1 + 1))
      end)

    expected = length(inputs) / num_buckets

    Enum.reduce(counts, 0.0, fn observed, sum ->
      delta = observed - expected
      sum + delta * delta / expected
    end)
  end

  defp rotl32(value, count) do
    band(value <<< count ||| value >>> (32 - count), @mask32)
  end

  defp fmix32(hash) do
    hash
    |> then(&bxor(&1, &1 >>> 16))
    |> then(&band(&1 * 0x85EBCA6B, @mask32))
    |> then(&bxor(&1, &1 >>> 13))
    |> then(&band(&1 * 0xC2B2AE35, @mask32))
    |> then(&bxor(&1, &1 >>> 16))
    |> band(@mask32)
  end

  defp popcount(value) do
    Stream.unfold(value, fn
      0 -> nil
      current -> {band(current, 1), current >>> 1}
    end)
    |> Enum.sum()
  end

  defp deterministic_bytes(sample_index) do
    {_state, bytes} =
      Enum.reduce(0..7, {bxor(0x9E3779B9, sample_index), []}, fn _index, {state, bytes} ->
        next_state = band(state * 1_664_525 + 1_013_904_223, @mask32)
        {next_state, [band(next_state, 0xFF) | bytes]}
      end)

    bytes
    |> Enum.reverse()
    |> :binary.list_to_bin()
  end
end
