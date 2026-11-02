defmodule CodingAdventures.CtCompare do
  @moduledoc """
  Constant-time comparison helpers for byte strings and unsigned counters.
  """

  import Bitwise

  @u64_max 0xFFFF_FFFF_FFFF_FFFF

  def ct_eq(left, right) when is_binary(left) and is_binary(right) do
    if byte_size(left) != byte_size(right) do
      false
    else
      left
      |> :binary.bin_to_list()
      |> Enum.zip(:binary.bin_to_list(right))
      |> Enum.reduce(0, fn {left_byte, right_byte}, acc ->
        bor(acc, bxor(left_byte, right_byte))
      end)
      |> Kernel.==(0)
    end
  end

  def ct_eq_fixed(left, right), do: ct_eq(left, right)

  @doc """
  Select `left` when `choice` is 1 and `right` when it is 0, without branching
  on `choice`.

  Why an integer and not a boolean: Elixir has no branch-free way to turn a
  boolean into a number. Every conversion is an `if` or a pattern match on the
  secret bit, which CT01 forbids. The bit is therefore an integer that must
  be exactly 0 or 1, as Go's `crypto/subtle` does:

      choice = 0  ->  band(-0, 0xFF) = 0x00  ->  result is right
      choice = 1  ->  band(-1, 0xFF) = 0xFF  ->  result is left

  The guard checks only the *type*. The range check clears bit 0 and tests for
  zero, a single computation whose result is the same for both valid values,
  so a valid call takes the same path whichever bit is secret. A length
  mismatch or a bit other than 0 or 1 raises `ArgumentError` before either
  input is read.
  """
  def ct_select_bytes(left, right, choice)
      when is_binary(left) and is_binary(right) and is_integer(choice) do
    if byte_size(left) != byte_size(right) do
      raise ArgumentError, "ct_select_bytes requires equal-length binaries"
    end

    if band(choice, bnot(1)) != 0 do
      raise ArgumentError, "ct_select_bytes requires choice to be 0 or 1"
    end

    mask = band(-choice, 0xFF)

    left
    |> :binary.bin_to_list()
    |> Enum.zip(:binary.bin_to_list(right))
    |> Enum.map(fn {left_byte, right_byte} ->
      bxor(right_byte, band(bxor(left_byte, right_byte), mask))
    end)
    |> :binary.list_to_bin()
  end

  def ct_eq_u64(left, right) when is_integer(left) and is_integer(right) do
    validate_u64!(left, "left")
    validate_u64!(right, "right")

    diff = band(bxor(left, right), @u64_max)
    folded = band(bsr(bor(diff, band(-diff, @u64_max)), 63), 1)
    folded == 0
  end

  defp validate_u64!(value, name) do
    if value < 0 or value > @u64_max do
      raise ArgumentError, "#{name} must be an unsigned 64-bit integer"
    end
  end
end
