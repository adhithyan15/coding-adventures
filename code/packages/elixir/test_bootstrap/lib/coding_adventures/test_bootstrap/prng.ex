defmodule CodingAdventures.TestBootstrap.Prng do
  @moduledoc """
  Deterministic randomness and hashing for test runs (TF01 5.1, VER05 2).

  BEAM integers never overflow, which is the opposite of what these algorithms
  were designed for: SplitMix64 and FNV-1a are defined on 64-bit words that
  WRAP.  So every add, multiply and shift below is followed by an explicit
  `band(_, @mask64)`.  Forgetting one mask gives a perfectly plausible 70-bit
  number and a silently different random stream.

  The generator is an immutable value: `next_u64/1` returns `{word, rng}`.
  """
  import Bitwise

  @mask64 0xFFFF_FFFF_FFFF_FFFF
  @golden_gamma 0x9E37_79B9_7F4A_7C15
  @mix_mul1 0xBF58_476D_1CE4_E5B9
  @mix_mul2 0x94D0_49BB_1331_11EB
  @fnv_offset 0xCBF2_9CE4_8422_2325
  @fnv_prime 0x0000_0100_0000_01B3
  @two64 1 <<< 64

  defstruct [:state]

  @type t :: %__MODULE__{state: non_neg_integer()}

  defimpl Inspect do
    # The state IS the stream: never printed.
    def inspect(_rng, _opts), do: "#Prng<>"
  end

  def mask64, do: @mask64

  @doc "A generator seeded with any integer, reduced modulo 2^64."
  @spec new(integer()) :: t()
  def new(seed) when is_integer(seed), do: %__MODULE__{state: band(seed, @mask64)}

  def new(other),
    do: raise(ArgumentError, "seed must be an integer, got #{kind(other)}")

  @doc """
  SplitMix64: state += golden gamma; then a three-step bit mixer.

      z1  = (s  ^ (s  >> 30)) * 0xbf58476d1ce4e5b9   (mod 2^64)
      z2  = (z1 ^ (z1 >> 27)) * 0x94d049bb133111eb   (mod 2^64)
      out = z2 ^ (z2 >> 31)

  Seed 0's first output is 0xe220a8397b1dcdaf.
  """
  @spec next_u64(t()) :: {non_neg_integer(), t()}
  def next_u64(%__MODULE__{state: state}) do
    s = band(state + @golden_gamma, @mask64)
    z = band(bxor(s, s >>> 30) * @mix_mul1, @mask64)
    z = band(bxor(z, z >>> 27) * @mix_mul2, @mask64)
    {band(bxor(z, z >>> 31), @mask64), %__MODULE__{state: s}}
  end

  @doc """
  Uniform integer in `[0, n)` for `1 <= n <= 2**64`, by rejection sampling.

  The threshold is `t = rem(2**64 - n, n)`.  Raws below `t` are redrawn so the
  remaining `2**64 - t` values split into whole residue classes.  (`rem(-n, n)`
  would be 0 for every n: nothing would ever be rejected.)

  Worked example, n = 3: 2**64 = 3 * 6148914691236517205 + 1, so t = 1 and the
  single raw 0 is thrown away.

  `bounded(rng, 1)` still consumes one word (the VER05 2.2 loop run once): the
  stream position must be identical in every lane.
  """
  @spec bounded(t(), pos_integer()) :: {non_neg_integer(), t()}
  def bounded(rng, n) do
    check_int!(n, "bounded(n)")
    if n < 1, do: raise(ArgumentError, "bounded(n) requires n >= 1, got #{n}")
    if n > @two64, do: raise(ArgumentError, "bounded(n) requires n <= 2**64, got #{n}")

    if n == @two64 do
      next_u64(rng)
    else
      reject_loop(rng, n, rem(@two64 - n, n))
    end
  end

  defp reject_loop(rng, n, threshold) do
    {x, rng} = next_u64(rng)
    if x >= threshold, do: {rem(x, n), rng}, else: reject_loop(rng, n, threshold)
  end

  @doc """
  Uniform integer in `[0, span)` for any `span >= 1` (VER05 7.2).

  `span <= 2**64` is `bounded/2`; wider spans draw `k = max(2, ceil(bitlen(span-1)/64))`
  words (first draw most significant) with `t = rem(2**(64k) - span, span)`.
  """
  def bounded_big(rng, span) do
    check_int!(span, "span")
    if span < 1, do: raise(ArgumentError, "span must be >= 1, got #{span}")

    if span <= @two64 do
      bounded(rng, span)
    else
      words = max(2, div(bit_length(span - 1) + 63, 64))
      threshold = rem((1 <<< (64 * words)) - span, span)
      big_loop(rng, span, words, threshold)
    end
  end

  defp big_loop(rng, span, words, threshold) do
    {raw, rng} =
      Enum.reduce(1..words, {0, rng}, fn _, {acc, r} ->
        {w, r} = next_u64(r)
        {(acc <<< 64) ||| w, r}
      end)

    if raw >= threshold, do: {rem(raw, span), rng}, else: big_loop(rng, span, words, threshold)
  end

  @doc "Number of bits needed for a non-negative integer (0 for 0)."
  def bit_length(0), do: 0

  def bit_length(x) when is_integer(x) and x > 0 do
    <<lead, _::binary>> = bin = :binary.encode_unsigned(x)
    8 * (byte_size(bin) - 1) + byte_bits(lead, 0)
  end

  defp byte_bits(0, acc), do: acc
  defp byte_bits(b, acc), do: byte_bits(b >>> 1, acc + 1)

  @doc "64-bit FNV-1a of a binary (hashed as its raw bytes)."
  @spec fnv1a64(binary()) :: non_neg_integer()
  def fnv1a64(data) when is_binary(data) do
    for <<b <- data>>, reduce: @fnv_offset do
      hash -> band(bxor(hash, b) * @fnv_prime, @mask64)
    end
  end

  def fnv1a64(other), do: raise(ArgumentError, "fnv1a64 expects a binary, got #{kind(other)}")

  @doc "`sub_seed(run_seed, id) = SplitMix64(run_seed ^ fnv1a64(id)).next_u64` (VER05 2.3)."
  def sub_seed(run_seed, property_id) when is_integer(run_seed) and is_binary(property_id) do
    {word, _} = next_u64(new(bxor(band(run_seed, @mask64), fnv1a64(property_id))))
    word
  end

  def sub_seed(_, _), do: raise(ArgumentError, "sub_seed needs an integer seed and a binary id")

  defp check_int!(value, _name) when is_integer(value), do: :ok

  defp check_int!(value, name),
    do: raise(ArgumentError, "#{name} must be an integer, got #{kind(value)}")

  defp kind(v) when is_float(v), do: "a float"
  defp kind(v) when is_atom(v), do: "an atom"
  defp kind(v) when is_binary(v), do: "a binary"
  defp kind(_), do: "another term"
end
