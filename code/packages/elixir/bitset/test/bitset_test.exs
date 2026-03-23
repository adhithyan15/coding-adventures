defmodule CodingAdventures.BitsetTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Bitset

  # ===========================================================================
  # Constructor tests
  # ===========================================================================

  describe "new/1" do
    test "creates a bitset with the given size" do
      bs = Bitset.new(100)
      assert Bitset.size(bs) == 100
    end

    test "capacity is rounded up to the next multiple of 64" do
      bs = Bitset.new(100)
      # 100 bits needs ceil(100/64) = 2 words = 128 bits capacity
      assert Bitset.capacity(bs) == 128
    end

    test "new(0) creates an empty bitset" do
      bs = Bitset.new(0)
      assert Bitset.size(bs) == 0
      assert Bitset.capacity(bs) == 0
    end

    test "new(64) creates a 1-word bitset" do
      bs = Bitset.new(64)
      assert Bitset.size(bs) == 64
      assert Bitset.capacity(bs) == 64
    end

    test "new(65) creates a 2-word bitset" do
      bs = Bitset.new(65)
      assert Bitset.size(bs) == 65
      assert Bitset.capacity(bs) == 128
    end

    test "all bits start as zero" do
      bs = Bitset.new(100)
      assert Bitset.popcount(bs) == 0
      assert Bitset.none?(bs) == true
    end
  end

  describe "from_integer/1" do
    test "from_integer(0) creates an empty bitset" do
      bs = Bitset.from_integer(0)
      assert Bitset.size(bs) == 0
      assert Bitset.to_integer(bs) == 0
    end

    test "from_integer(5) creates a 3-bit bitset (binary 101)" do
      bs = Bitset.from_integer(5)
      assert Bitset.size(bs) == 3
      assert Bitset.test?(bs, 0) == true
      assert Bitset.test?(bs, 1) == false
      assert Bitset.test?(bs, 2) == true
    end

    test "from_integer(1) creates a 1-bit bitset" do
      bs = Bitset.from_integer(1)
      assert Bitset.size(bs) == 1
      assert Bitset.test?(bs, 0) == true
    end

    test "from_integer(255) creates an 8-bit bitset" do
      bs = Bitset.from_integer(255)
      assert Bitset.size(bs) == 8
      assert Bitset.popcount(bs) == 8
    end

    test "round-trip: from_integer then to_integer" do
      for val <- [0, 1, 5, 42, 255, 1023, 65535] do
        assert Bitset.to_integer(Bitset.from_integer(val)) == val
      end
    end

    test "large integer spanning multiple words" do
      # 2^64 + 1 requires two words
      large = Bitwise.bsl(1, 64) + 1
      bs = Bitset.from_integer(large)
      assert Bitset.size(bs) == 65
      assert Bitset.test?(bs, 0) == true
      assert Bitset.test?(bs, 64) == true
      assert Bitset.test?(bs, 1) == false
      assert Bitset.to_integer(bs) == large
    end
  end

  describe "from_binary_str/1" do
    test "parses '1010' correctly" do
      {:ok, bs} = Bitset.from_binary_str("1010")
      assert Bitset.size(bs) == 4
      assert Bitset.test?(bs, 0) == false
      assert Bitset.test?(bs, 1) == true
      assert Bitset.test?(bs, 2) == false
      assert Bitset.test?(bs, 3) == true
      assert Bitset.to_integer(bs) == 10
    end

    test "empty string produces empty bitset" do
      {:ok, bs} = Bitset.from_binary_str("")
      assert Bitset.size(bs) == 0
    end

    test "all ones" do
      {:ok, bs} = Bitset.from_binary_str("1111")
      assert Bitset.size(bs) == 4
      assert Bitset.popcount(bs) == 4
      assert Bitset.to_integer(bs) == 15
    end

    test "all zeros" do
      {:ok, bs} = Bitset.from_binary_str("0000")
      assert Bitset.size(bs) == 4
      assert Bitset.popcount(bs) == 0
    end

    test "returns error for invalid characters" do
      {:error, msg} = Bitset.from_binary_str("102")
      assert msg =~ "invalid binary string"
    end

    test "returns error for non-binary chars" do
      {:error, _} = Bitset.from_binary_str("hello")
    end

    test "single bit '1'" do
      {:ok, bs} = Bitset.from_binary_str("1")
      assert Bitset.size(bs) == 1
      assert Bitset.test?(bs, 0) == true
    end

    test "single bit '0'" do
      {:ok, bs} = Bitset.from_binary_str("0")
      assert Bitset.size(bs) == 1
      assert Bitset.test?(bs, 0) == false
    end

    test "round-trip with to_binary_str" do
      for str <- ["1", "0", "1010", "1111", "10000001"] do
        {:ok, bs} = Bitset.from_binary_str(str)
        assert Bitset.to_binary_str(bs) == str
      end
    end
  end

  describe "from_binary_str!/1" do
    test "works for valid input" do
      bs = Bitset.from_binary_str!("101")
      assert Bitset.to_integer(bs) == 5
    end

    test "raises BitsetError for invalid input" do
      assert_raise Bitset.BitsetError, fn ->
        Bitset.from_binary_str!("abc")
      end
    end
  end

  # ===========================================================================
  # Single-bit operation tests
  # ===========================================================================

  describe "set/2" do
    test "sets a bit within range" do
      bs = Bitset.new(10) |> Bitset.set(5)
      assert Bitset.test?(bs, 5) == true
    end

    test "setting an already-set bit is idempotent" do
      bs = Bitset.new(10) |> Bitset.set(5) |> Bitset.set(5)
      assert Bitset.test?(bs, 5) == true
      assert Bitset.popcount(bs) == 1
    end

    test "auto-grows when setting beyond len" do
      bs = Bitset.new(10) |> Bitset.set(100)
      assert Bitset.size(bs) == 101
      assert Bitset.test?(bs, 100) == true
    end

    test "auto-grows from empty bitset" do
      bs = Bitset.new(0) |> Bitset.set(5)
      assert Bitset.size(bs) == 6
      assert Bitset.test?(bs, 5) == true
    end

    test "set multiple bits" do
      bs =
        Bitset.new(100)
        |> Bitset.set(0)
        |> Bitset.set(42)
        |> Bitset.set(99)

      assert Bitset.popcount(bs) == 3
      assert Bitset.test?(bs, 0) == true
      assert Bitset.test?(bs, 42) == true
      assert Bitset.test?(bs, 99) == true
    end

    test "set bit at word boundary (bit 63)" do
      bs = Bitset.new(100) |> Bitset.set(63)
      assert Bitset.test?(bs, 63) == true
    end

    test "set bit at second word start (bit 64)" do
      bs = Bitset.new(100) |> Bitset.set(64)
      assert Bitset.test?(bs, 64) == true
    end
  end

  describe "clear/2" do
    test "clears a set bit" do
      bs = Bitset.new(10) |> Bitset.set(5) |> Bitset.clear(5)
      assert Bitset.test?(bs, 5) == false
    end

    test "clearing an already-clear bit is a no-op" do
      bs = Bitset.new(10) |> Bitset.clear(5)
      assert Bitset.test?(bs, 5) == false
    end

    test "clearing beyond len does not grow" do
      bs = Bitset.new(10)
      bs2 = Bitset.clear(bs, 999)
      assert Bitset.size(bs2) == 10
    end
  end

  describe "test?/2" do
    test "returns false for unset bits" do
      bs = Bitset.new(100)
      assert Bitset.test?(bs, 50) == false
    end

    test "returns true for set bits" do
      bs = Bitset.new(100) |> Bitset.set(50)
      assert Bitset.test?(bs, 50) == true
    end

    test "returns false for indices beyond len" do
      bs = Bitset.new(10)
      assert Bitset.test?(bs, 999) == false
    end
  end

  describe "toggle/2" do
    test "toggles 0 to 1" do
      bs = Bitset.new(10) |> Bitset.toggle(5)
      assert Bitset.test?(bs, 5) == true
    end

    test "toggles 1 to 0" do
      bs = Bitset.new(10) |> Bitset.set(5) |> Bitset.toggle(5)
      assert Bitset.test?(bs, 5) == false
    end

    test "double toggle is identity" do
      bs = Bitset.new(10) |> Bitset.toggle(5) |> Bitset.toggle(5)
      assert Bitset.test?(bs, 5) == false
    end

    test "auto-grows when toggling beyond len" do
      bs = Bitset.new(10) |> Bitset.toggle(100)
      assert Bitset.size(bs) == 101
      assert Bitset.test?(bs, 100) == true
    end
  end

  # ===========================================================================
  # Bulk bitwise operation tests
  # ===========================================================================

  describe "bitwise_and/2" do
    test "AND of identical bitsets is the same bitset" do
      a = Bitset.from_integer(0b1100)
      result = Bitset.bitwise_and(a, a)
      assert Bitset.to_integer(result) == 0b1100
    end

    test "AND produces intersection" do
      a = Bitset.from_integer(0b1100)
      b = Bitset.from_integer(0b1010)
      result = Bitset.bitwise_and(a, b)
      assert Bitset.to_integer(result) == 0b1000
    end

    test "AND with zero produces zero" do
      a = Bitset.from_integer(0b1111)
      b = Bitset.new(4)
      result = Bitset.bitwise_and(a, b)
      assert Bitset.popcount(result) == 0
    end

    test "AND with different sizes" do
      a = Bitset.from_integer(0b11001100)
      b = Bitset.from_integer(0b1010)
      result = Bitset.bitwise_and(a, b)
      assert Bitset.to_integer(result) == 0b1000
      assert Bitset.size(result) == 8
    end
  end

  describe "bitwise_or/2" do
    test "OR produces union" do
      a = Bitset.from_integer(0b1100)
      b = Bitset.from_integer(0b1010)
      result = Bitset.bitwise_or(a, b)
      assert Bitset.to_integer(result) == 0b1110
    end

    test "OR with zero is identity" do
      a = Bitset.from_integer(0b1111)
      b = Bitset.new(4)
      result = Bitset.bitwise_or(a, b)
      assert Bitset.to_integer(result) == 0b1111
    end

    test "OR with different sizes" do
      a = Bitset.from_integer(0b11001100)
      b = Bitset.from_integer(0b1010)
      result = Bitset.bitwise_or(a, b)
      assert Bitset.to_integer(result) == 0b11001110
    end
  end

  describe "bitwise_xor/2" do
    test "XOR produces symmetric difference" do
      a = Bitset.from_integer(0b1100)
      b = Bitset.from_integer(0b1010)
      result = Bitset.bitwise_xor(a, b)
      assert Bitset.to_integer(result) == 0b0110
    end

    test "XOR with itself produces zero" do
      a = Bitset.from_integer(0b1111)
      result = Bitset.bitwise_xor(a, a)
      assert Bitset.popcount(result) == 0
    end

    test "XOR with zero is identity" do
      a = Bitset.from_integer(0b1010)
      b = Bitset.new(4)
      result = Bitset.bitwise_xor(a, b)
      assert Bitset.to_integer(result) == 0b1010
    end
  end

  describe "flip_all/1" do
    test "flips all bits" do
      a = Bitset.from_integer(0b1010)
      result = Bitset.flip_all(a)
      assert Bitset.to_integer(result) == 0b0101
    end

    test "double flip is identity" do
      a = Bitset.from_integer(0b1010)
      result = Bitset.flip_all(Bitset.flip_all(a))
      assert Bitset.to_integer(result) == 0b1010
    end

    test "flip of all zeros is all ones (within len)" do
      bs = Bitset.new(4)
      result = Bitset.flip_all(bs)
      assert Bitset.to_integer(result) == 0b1111
      assert Bitset.popcount(result) == 4
    end

    test "flip of empty bitset is empty" do
      bs = Bitset.new(0)
      result = Bitset.flip_all(bs)
      assert Bitset.size(result) == 0
    end

    test "flip respects clean-trailing-bits invariant" do
      # A bitset with len=5 should have only 5 bits flipped,
      # not all 64 bits of the word.
      bs = Bitset.new(5)
      result = Bitset.flip_all(bs)
      assert Bitset.popcount(result) == 5
      assert Bitset.size(result) == 5
    end
  end

  describe "difference/2" do
    test "set difference (AND-NOT)" do
      a = Bitset.from_integer(0b1110)
      b = Bitset.from_integer(0b1010)
      result = Bitset.difference(a, b)
      assert Bitset.to_integer(result) == 0b0100
    end

    test "difference with self is zero" do
      a = Bitset.from_integer(0b1111)
      result = Bitset.difference(a, a)
      assert Bitset.popcount(result) == 0
    end

    test "difference with zero is identity" do
      a = Bitset.from_integer(0b1010)
      b = Bitset.new(4)
      result = Bitset.difference(a, b)
      assert Bitset.to_integer(result) == 0b1010
    end

    test "difference with different sizes" do
      a = Bitset.from_integer(0b11111111)
      b = Bitset.from_integer(0b1010)
      result = Bitset.difference(a, b)
      assert Bitset.to_integer(result) == 0b11110101
    end
  end

  # ===========================================================================
  # Counting and query operation tests
  # ===========================================================================

  describe "popcount/1" do
    test "empty bitset has popcount 0" do
      assert Bitset.popcount(Bitset.new(100)) == 0
    end

    test "counts set bits correctly" do
      bs = Bitset.from_integer(0b10110)
      assert Bitset.popcount(bs) == 3
    end

    test "all bits set" do
      {:ok, bs} = Bitset.from_binary_str("11111111")
      assert Bitset.popcount(bs) == 8
    end

    test "single bit" do
      bs = Bitset.from_integer(1)
      assert Bitset.popcount(bs) == 1
    end
  end

  describe "size/1" do
    test "returns the logical length" do
      assert Bitset.size(Bitset.new(100)) == 100
      assert Bitset.size(Bitset.new(0)) == 0
    end

    test "size grows after set beyond len" do
      bs = Bitset.new(10) |> Bitset.set(100)
      assert Bitset.size(bs) == 101
    end
  end

  describe "capacity/1" do
    test "capacity is a multiple of 64" do
      for size <- [0, 1, 63, 64, 65, 100, 128, 129, 200] do
        cap = Bitset.capacity(Bitset.new(size))
        assert rem(cap, 64) == 0, "capacity #{cap} for size #{size} not multiple of 64"
        assert cap >= size, "capacity #{cap} less than size #{size}"
      end
    end
  end

  describe "any?/1" do
    test "empty bitset: any? is false" do
      assert Bitset.any?(Bitset.new(100)) == false
    end

    test "bitset with one bit: any? is true" do
      bs = Bitset.new(100) |> Bitset.set(50)
      assert Bitset.any?(bs) == true
    end

    test "zero-length bitset: any? is false" do
      assert Bitset.any?(Bitset.new(0)) == false
    end
  end

  describe "all?/1" do
    test "empty (len=0) bitset: all? is true (vacuous truth)" do
      assert Bitset.all?(Bitset.new(0)) == true
    end

    test "all bits set: all? is true" do
      {:ok, bs} = Bitset.from_binary_str("1111")
      assert Bitset.all?(bs) == true
    end

    test "not all bits set: all? is false" do
      {:ok, bs} = Bitset.from_binary_str("1110")
      assert Bitset.all?(bs) == false
    end

    test "single zero bit: all? is false" do
      bs = Bitset.new(1)
      assert Bitset.all?(bs) == false
    end

    test "single one bit: all? is true" do
      bs = Bitset.new(1) |> Bitset.set(0)
      assert Bitset.all?(bs) == true
    end

    test "64-bit all ones" do
      # Create a 64-bit bitset with all bits set
      bs = Bitset.new(64)

      bs =
        Enum.reduce(0..63, bs, fn i, acc ->
          Bitset.set(acc, i)
        end)

      assert Bitset.all?(bs) == true
    end

    test "65-bit with first 64 set but not bit 64" do
      bs = Bitset.new(65)

      bs =
        Enum.reduce(0..63, bs, fn i, acc ->
          Bitset.set(acc, i)
        end)

      assert Bitset.all?(bs) == false
    end
  end

  describe "none?/1" do
    test "empty bitset: none? is true" do
      assert Bitset.none?(Bitset.new(100)) == true
    end

    test "bitset with bit set: none? is false" do
      bs = Bitset.new(100) |> Bitset.set(50)
      assert Bitset.none?(bs) == false
    end
  end

  # ===========================================================================
  # Iteration tests
  # ===========================================================================

  describe "set_bits/1" do
    test "empty bitset returns empty list" do
      assert Bitset.set_bits(Bitset.new(100)) == []
    end

    test "returns indices in ascending order" do
      bs = Bitset.from_integer(0b10100101)
      assert Bitset.set_bits(bs) == [0, 2, 5, 7]
    end

    test "single bit" do
      bs = Bitset.new(100) |> Bitset.set(42)
      assert Bitset.set_bits(bs) == [42]
    end

    test "bits across word boundaries" do
      bs =
        Bitset.new(200)
        |> Bitset.set(0)
        |> Bitset.set(63)
        |> Bitset.set(64)
        |> Bitset.set(127)
        |> Bitset.set(128)
        |> Bitset.set(199)

      assert Bitset.set_bits(bs) == [0, 63, 64, 127, 128, 199]
    end

    test "all bits set in small bitset" do
      {:ok, bs} = Bitset.from_binary_str("1111")
      assert Bitset.set_bits(bs) == [0, 1, 2, 3]
    end
  end

  # ===========================================================================
  # Conversion tests
  # ===========================================================================

  describe "to_integer/1" do
    test "empty bitset returns 0" do
      assert Bitset.to_integer(Bitset.new(0)) == 0
    end

    test "converts correctly" do
      assert Bitset.to_integer(Bitset.from_integer(42)) == 42
    end

    test "bitset with no bits set returns 0" do
      assert Bitset.to_integer(Bitset.new(100)) == 0
    end

    test "large multi-word value" do
      large = Bitwise.bsl(1, 64) + 1
      bs = Bitset.from_integer(large)
      assert Bitset.to_integer(bs) == large
    end
  end

  describe "to_binary_str/1" do
    test "empty bitset returns empty string" do
      assert Bitset.to_binary_str(Bitset.new(0)) == ""
    end

    test "from_integer(5) produces '101'" do
      bs = Bitset.from_integer(5)
      assert Bitset.to_binary_str(bs) == "101"
    end

    test "all-zero bitset of len 4 produces '0000'" do
      bs = Bitset.new(4)
      assert Bitset.to_binary_str(bs) == "0000"
    end

    test "preserves leading zeros" do
      {:ok, bs} = Bitset.from_binary_str("0001")
      assert Bitset.to_binary_str(bs) == "0001"
    end
  end

  # ===========================================================================
  # Equality tests
  # ===========================================================================

  describe "equal?/2" do
    test "identical bitsets are equal" do
      a = Bitset.from_integer(42)
      b = Bitset.from_integer(42)
      assert Bitset.equal?(a, b) == true
    end

    test "different values are not equal" do
      a = Bitset.from_integer(42)
      b = Bitset.from_integer(43)
      assert Bitset.equal?(a, b) == false
    end

    test "different lengths are not equal even if same bits" do
      a = Bitset.from_integer(5)
      # Create a bitset with same bits but different len
      {:ok, b} = Bitset.from_binary_str("0101")
      # a has len=3, b has len=4
      assert Bitset.equal?(a, b) == false
    end

    test "empty bitsets are equal" do
      assert Bitset.equal?(Bitset.new(0), Bitset.new(0)) == true
    end

    test "capacity doesn't affect equality" do
      # Both have same len and bits, but might have different capacity
      a = Bitset.new(10) |> Bitset.set(5)
      b = Bitset.new(10) |> Bitset.set(5)
      assert Bitset.equal?(a, b) == true
    end
  end

  # ===========================================================================
  # Protocol tests
  # ===========================================================================

  describe "String.Chars protocol" do
    test "to_string produces Bitset(binary)" do
      bs = Bitset.from_integer(5)
      assert to_string(bs) == "Bitset(101)"
    end

    test "empty bitset" do
      bs = Bitset.new(0)
      assert to_string(bs) == "Bitset()"
    end
  end

  describe "Inspect protocol" do
    test "inspect produces #Bitset<binary, len=N>" do
      bs = Bitset.from_integer(5)
      result = inspect(bs)
      assert result == "#Bitset<101, len=3>"
    end

    test "inspect empty bitset" do
      bs = Bitset.new(0)
      assert inspect(bs) == "#Bitset<, len=0>"
    end
  end

  # ===========================================================================
  # Edge case and integration tests
  # ===========================================================================

  describe "auto-growth" do
    test "growth doubles capacity" do
      bs = Bitset.new(100)
      assert Bitset.capacity(bs) == 128

      bs = Bitset.set(bs, 200)
      assert Bitset.capacity(bs) == 256
      assert Bitset.size(bs) == 201
    end

    test "growth from zero" do
      bs = Bitset.new(0)
      bs = Bitset.set(bs, 0)
      assert Bitset.size(bs) == 1
      assert Bitset.capacity(bs) == 64
    end

    test "large growth" do
      bs = Bitset.new(10)
      bs = Bitset.set(bs, 500)
      assert Bitset.size(bs) == 501
      assert Bitset.capacity(bs) >= 501
      assert Bitset.test?(bs, 500) == true
      assert Bitset.test?(bs, 499) == false
    end
  end

  describe "clean-trailing-bits invariant" do
    test "flip_all then popcount gives correct result for non-64-aligned len" do
      bs = Bitset.new(5)
      flipped = Bitset.flip_all(bs)
      assert Bitset.popcount(flipped) == 5
    end

    test "toggle then popcount on boundary" do
      bs = Bitset.new(65)
      bs = Bitset.toggle(bs, 0)
      assert Bitset.popcount(bs) == 1
    end

    test "operations preserve invariant across word boundaries" do
      a = Bitset.new(100)

      a =
        Enum.reduce(0..99, a, fn i, acc ->
          Bitset.set(acc, i)
        end)

      b = Bitset.flip_all(a)
      assert Bitset.popcount(b) == 0
    end
  end

  describe "combined operations" do
    test "set, clear, test round-trip" do
      bs = Bitset.new(100)
      bs = Bitset.set(bs, 42)
      assert Bitset.test?(bs, 42) == true
      bs = Bitset.clear(bs, 42)
      assert Bitset.test?(bs, 42) == false
    end

    test "set_bits matches popcount" do
      bs =
        Bitset.new(200)
        |> Bitset.set(3)
        |> Bitset.set(42)
        |> Bitset.set(100)
        |> Bitset.set(199)

      assert length(Bitset.set_bits(bs)) == Bitset.popcount(bs)
    end

    test "OR of two disjoint sets has popcount = sum of popcounts" do
      a = Bitset.from_integer(0b1100)
      b = Bitset.from_integer(0b0011)
      result = Bitset.bitwise_or(a, b)
      assert Bitset.popcount(result) == Bitset.popcount(a) + Bitset.popcount(b)
    end

    test "AND of disjoint sets is empty" do
      a = Bitset.from_integer(0b1100)
      b = Bitset.from_integer(0b0011)
      result = Bitset.bitwise_and(a, b)
      assert Bitset.popcount(result) == 0
    end

    test "De Morgan's law: NOT(A AND B) == NOT(A) OR NOT(B)" do
      a = Bitset.from_binary_str!("11001010")
      b = Bitset.from_binary_str!("10110101")

      lhs = Bitset.flip_all(Bitset.bitwise_and(a, b))
      rhs = Bitset.bitwise_or(Bitset.flip_all(a), Bitset.flip_all(b))

      assert Bitset.equal?(lhs, rhs) == true
    end

    test "De Morgan's law: NOT(A OR B) == NOT(A) AND NOT(B)" do
      a = Bitset.from_binary_str!("11001010")
      b = Bitset.from_binary_str!("10110101")

      lhs = Bitset.flip_all(Bitset.bitwise_or(a, b))
      rhs = Bitset.bitwise_and(Bitset.flip_all(a), Bitset.flip_all(b))

      assert Bitset.equal?(lhs, rhs) == true
    end
  end
end
