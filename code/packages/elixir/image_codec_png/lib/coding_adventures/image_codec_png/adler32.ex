defmodule CodingAdventures.ImageCodecPng.Adler32 do
  @moduledoc """
  RFC 1950 Adler-32 checksum implementation with NMAX chunked modular reduction.
  """

  import Bitwise
  alias CodingAdventures.ImageCodecPng.Constants

  @doc """
  Computes the 32-bit unsigned Adler-32 checksum of binary data.
  """
  def compute(data) when is_binary(data) do
    do_compute(data, Constants.adler_initial_a(), Constants.adler_initial_b())
  end

  defp do_compute(<<>>, a, b) do
    (bsl(b, Constants.adler_b_shift()) ||| a) &&& Constants.uint32_mask()
  end

  defp do_compute(data, a, b) do
    # RFC 1950 invariant: NMAX = 5552 is the largest number of terms for which
    # b cannot overflow a 32-bit unsigned accumulator (b_max = 5552 * 255 * 5553 / 2 ~= 3.93e9 < 2^32).
    chunk_size = min(byte_size(data), Constants.adler_chunk_size())
    <<chunk::binary-size(chunk_size), rest::binary>> = data
    {new_a, new_b} = sum_chunk(chunk, a, b)
    do_compute(rest, rem(new_a, Constants.adler_base()), rem(new_b, Constants.adler_base()))
  end

  defp sum_chunk(<<>>, a, b), do: {a, b}

  defp sum_chunk(<<byte, rest::binary>>, a, b) do
    next_a = a + byte
    sum_chunk(rest, next_a, b + next_a)
  end
end
