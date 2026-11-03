defmodule CodingAdventures.CtCompareBitTest do
  # The bit-returning equality variants (CT01): they agree with the booleans
  # and are always exactly 0 or 1.
  use ExUnit.Case, async: true

  import Bitwise, only: [<<<: 2, bxor: 2]

  alias CodingAdventures.CtCompare, as: CT

  test "ct_eq_bit agrees with ct_eq on every byte pair" do
    for a <- 0..255, b <- 0..255 do
      want = if CT.ct_eq(<<a>>, <<b>>), do: 1, else: 0
      assert CT.ct_eq_bit(<<a>>, <<b>>) == want
      assert CT.ct_eq_fixed_bit(<<a>>, <<b>>) == want
    end
  end

  test "lengths and every bit position" do
    assert CT.ct_eq_bit("", "") == 1
    assert CT.ct_eq_bit("a", "ab") == 0
    assert_raise ArgumentError, fn -> CT.ct_eq_bit([1], "a") end
    base = :binary.copy(<<0>>, 64)

    for i <- 0..63, bit <- 0..7 do
      <<prefix::binary-size(i), _::8, rest::binary>> = base
      other = prefix <> <<1 <<< bit>> <> rest
      assert CT.ct_eq_bit(base, other) == 0
    end
  end

  test "ct_eq_u64_bit" do
    values = [0, 1, 2, 1 <<< 63, (1 <<< 63) - 1, (1 <<< 64) - 1, 0x0123456789ABCDEF]

    for x <- values do
      for y <- values, do: assert(CT.ct_eq_u64_bit(x, y) == if(x == y, do: 1, else: 0))
      for bit <- 0..63, do: assert(CT.ct_eq_u64_bit(x, bxor(x, 1 <<< bit)) == 0)
    end

    assert_raise ArgumentError, fn -> CT.ct_eq_u64_bit(-1, 0) end
    assert_raise ArgumentError, fn -> CT.ct_eq_u64_bit(0, 1 <<< 64) end
    assert_raise ArgumentError, fn -> CT.ct_eq_u64_bit("a", 0) end
  end

  test "the equality bit feeds selection" do
    assert CT.ct_select_bytes("left", "rite", CT.ct_eq_bit("k", "k")) == "left"
    assert CT.ct_select_bytes("left", "rite", CT.ct_eq_bit("k", "j")) == "rite"
    assert CT.ct_select_bytes("left", "rite", CT.ct_eq_u64_bit(7, 7)) == "left"
  end
end
