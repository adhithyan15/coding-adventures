defmodule CodingAdventures.CtCompare do
  @moduledoc """
  Constant-time comparison helpers for byte strings and unsigned counters.
  """

  import Bitwise

  @u64_max 0xFFFF_FFFF_FFFF_FFFF

  # Catch-all clauses. Without them a wrong-type argument raises
  # FunctionClauseError, and on BEAM that error's stack trace carries the full
  # argument list, so the secret inputs would be printed into crash logs. These
  # clauses dispatch on the public *type* of the arguments only, and an error
  # raised inside a function body does not record its arguments.
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

  def ct_eq(_left, _right), do: raise(ArgumentError, "ct_eq requires two binaries")

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

  def ct_select_bytes(_left, _right, _choice),
    do: raise(ArgumentError, "ct_select_bytes requires two binaries and choice 0 or 1")

  def ct_eq_u64(left, right) when is_integer(left) and is_integer(right) do
    validate_u64!(left, "left")
    validate_u64!(right, "right")

    diff = band(bxor(left, right), @u64_max)
    folded = band(bsr(bor(diff, band(-diff, @u64_max)), 63), 1)
    folded == 0
  end

  def ct_eq_u64(_left, _right),
    do: raise(ArgumentError, "ct_eq_u64 requires two unsigned 64-bit integers")

  # -- Bit-returning equality (CT01, "How a lane receives the selection bit")
  #
  # ct_select_bytes takes its selection bit as an integer, because every way
  # Elixir has of turning true/false into 1/0 is an `if`, a `case` or a
  # pattern match. A caller holding only ct_eq's boolean would need exactly
  # that branch, so each equality also returns the bit itself, computed from
  # the difference accumulator with arithmetic only. `bsr` on a negative
  # integer is an arithmetic shift, so for an accumulator in 0..255:
  #
  #     acc = 0          bsr(0 - 1, 8) = -1,  band(-1, 1) = 1   (equal)
  #     acc = 1 .. 255   bsr(acc - 1, 8) = 0, band(0, 1) = 0    (different)
  #
  # Use it as ct_select_bytes(a, b, ct_eq_bit(x, y)).

  @doc "1 when the binaries are equal, else 0. A length mismatch is 0."
  def ct_eq_bit(left, right) when is_binary(left) and is_binary(right) do
    if byte_size(left) != byte_size(right) do
      0
    else
      left
      |> :binary.bin_to_list()
      |> Enum.zip(:binary.bin_to_list(right))
      |> Enum.reduce(0, fn {left_byte, right_byte}, acc ->
        bor(acc, bxor(left_byte, right_byte))
      end)
      |> Kernel.-(1)
      |> bsr(8)
      |> band(1)
    end
  end

  def ct_eq_bit(_left, _right), do: raise(ArgumentError, "ct_eq_bit requires two binaries")

  @doc "Fixed-size companion to `ct_eq_bit/2`."
  def ct_eq_fixed_bit(left, right), do: ct_eq_bit(left, right)

  @doc """
  1 when the two unsigned 64-bit values are equal, else 0. The difference lies
  in 0..2^64-1, so subtracting 1 is negative exactly when it was 0.
  """
  def ct_eq_u64_bit(left, right) when is_integer(left) and is_integer(right) do
    validate_u64!(left, "left")
    validate_u64!(right, "right")
    band(bsr(band(bxor(left, right), @u64_max) - 1, 64), 1)
  end

  def ct_eq_u64_bit(_left, _right),
    do: raise(ArgumentError, "ct_eq_u64_bit requires two unsigned 64-bit integers")

  defp validate_u64!(value, name) do
    if value < 0 or value > @u64_max do
      raise ArgumentError, "#{name} must be an unsigned 64-bit integer"
    end
  end
end
