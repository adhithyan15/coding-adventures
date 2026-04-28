defmodule CodingAdventures.CtCompareTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CtCompare
  import Bitwise

  test "ct_eq matches binary equality" do
    assert CtCompare.ct_eq("abcdef", "abcdef")
    assert CtCompare.ct_eq("", "")
    refute CtCompare.ct_eq("abcdef", "abcdeg")
    refute CtCompare.ct_eq("abcdef", "bbcdef")
    refute CtCompare.ct_eq("abc", "abcd")
  end

  test "ct_eq detects every bit position" do
    base = :binary.copy(<<0x42>>, 32)

    for index <- 0..31, bit <- 0..7 do
      bytes = :binary.bin_to_list(base)
      flipped = List.update_at(bytes, index, &bxor(&1, bsl(1, bit))) |> :binary.list_to_bin()
      refute CtCompare.ct_eq(base, flipped)
    end
  end

  test "ct_eq_fixed delegates to ct_eq" do
    assert CtCompare.ct_eq_fixed(:binary.copy(<<0x11>>, 16), :binary.copy(<<0x11>>, 16))
    refute CtCompare.ct_eq_fixed(:binary.copy(<<0x11>>, 16), :binary.copy(<<0x11>>, 15) <> <<0x10>>)
  end

  test "ct_select_bytes preserves byte values" do
    left = :binary.list_to_bin(Enum.to_list(0..255))
    right = :binary.list_to_bin(Enum.reverse(0..255))

    assert CtCompare.ct_select_bytes(left, right, true) == left
    assert CtCompare.ct_select_bytes(left, right, false) == right
    assert CtCompare.ct_select_bytes("", "", true) == ""

    assert_raise ArgumentError, ~r/equal-length/, fn ->
      CtCompare.ct_select_bytes(<<1>>, <<1, 2>>, true)
    end
  end

  test "ct_eq_u64 handles edges and rejects out of range" do
    assert CtCompare.ct_eq_u64(0, 0)
    assert CtCompare.ct_eq_u64(0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF)
    refute CtCompare.ct_eq_u64(0, bsl(1, 63))

    base = 0x1234_5678_9ABC_DEF0

    for bit <- 0..63 do
      refute CtCompare.ct_eq_u64(base, bxor(base, bsl(1, bit)))
    end

    assert_raise ArgumentError, fn -> CtCompare.ct_eq_u64(-1, 0) end
    assert_raise ArgumentError, fn -> CtCompare.ct_eq_u64(0, bsl(1, 64)) end
  end
end
