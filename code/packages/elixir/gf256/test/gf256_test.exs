defmodule CodingAdventures.GF256Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.GF256, as: GF

  # ────────────────────────────────────────────────────────────────────────────
  # Table construction
  # ────────────────────────────────────────────────────────────────────────────

  describe "table construction" do
    test "ALOG[0] = 1 (g^0 = 1)" do
      assert Enum.at(GF.alog_table(), 0) == 1
    end

    test "ALOG[1] = 2 (g^1 = 2, the generator)" do
      assert Enum.at(GF.alog_table(), 1) == 2
    end

    test "ALOG[7] = 128 = 0x80 (2^7)" do
      assert Enum.at(GF.alog_table(), 7) == 0x80
    end

    test "ALOG[8] = 29 = 0x1D (first reduction: 256 XOR 0x11D)" do
      # 2^8 = 256, which overflows a byte. XOR with 0x11D = 285:
      # 256 XOR 285 = 0x100 XOR 0x11D = 0x01D = 29
      assert Enum.at(GF.alog_table(), 8) == 29
    end

    test "ALOG[255] = 1 (g^255 = g^0, the group has order 255)" do
      assert Enum.at(GF.alog_table(), 255) == 1
    end

    test "LOG[1] = 0 (log base g of 1 is 0)" do
      assert Enum.at(GF.log_table(), 1) == 0
    end

    test "LOG[2] = 1 (log base g of g = 1)" do
      assert Enum.at(GF.log_table(), 2) == 1
    end

    test "LOG[ALOG[i]] = i for all valid i (tables are inverses)" do
      alog = GF.alog_table()
      log = GF.log_table()

      # Verify for i in 0..254 (index 255 is the wrap-around sentinel)
      Enum.each(0..254, fn i ->
        val = Enum.at(alog, i)
        assert Enum.at(log, val) == i,
               "LOG[ALOG[#{i}]] = LOG[#{val}] should be #{i}"
      end)
    end

    test "all 255 non-zero elements appear exactly once in ALOG[0..254]" do
      alog = GF.alog_table() |> Enum.take(255)
      unique = MapSet.new(alog)
      assert MapSet.size(unique) == 255
      # Every value should be in 1..255
      assert MapSet.member?(unique, 0) == false
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # zero/0 and one/0
  # ────────────────────────────────────────────────────────────────────────────

  describe "zero and one" do
    test "zero() returns 0" do
      assert GF.zero() == 0
    end

    test "one() returns 1" do
      assert GF.one() == 1
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # add/2 and subtract/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "add" do
    test "add is XOR" do
      assert GF.add(0x53, 0xCA) == Bitwise.bxor(0x53, 0xCA)
    end

    test "add(x, x) = 0 for all x (every element is its own inverse)" do
      Enum.each(0..255, fn x ->
        assert GF.add(x, x) == 0,
               "add(#{x}, #{x}) should be 0"
      end)
    end

    test "add(x, 0) = x (additive identity)" do
      Enum.each(0..255, fn x ->
        assert GF.add(x, 0) == x
      end)
    end

    test "add is commutative" do
      assert GF.add(0x12, 0x34) == GF.add(0x34, 0x12)
    end

    test "add is associative" do
      assert GF.add(GF.add(0x01, 0x02), 0x03) == GF.add(0x01, GF.add(0x02, 0x03))
    end
  end

  describe "subtract" do
    test "subtract is the same as add (characteristic 2)" do
      assert GF.subtract(0x53, 0xCA) == GF.add(0x53, 0xCA)
    end

    test "subtract(x, x) = 0" do
      assert GF.subtract(0xFF, 0xFF) == 0
    end

    test "subtract(x, 0) = x" do
      assert GF.subtract(0xAB, 0) == 0xAB
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # multiply/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "multiply" do
    test "multiply(0x53, 0xCA) gives correct result for 0x11D polynomial" do
      # With 0x11D primitive polynomial, 0x53 × 0xCA = 0x8F.
      # (Verified by manual table construction.)
      assert GF.multiply(0x53, 0xCA) == 0x8F
    end

    test "multiply(a, 0) = 0 for any a" do
      Enum.each([1, 2, 0x53, 0xFF], fn a ->
        assert GF.multiply(a, 0) == 0
      end)
    end

    test "multiply(0, b) = 0 for any b" do
      Enum.each([1, 2, 0x53, 0xFF], fn b ->
        assert GF.multiply(0, b) == 0
      end)
    end

    test "multiply(x, 1) = x for all x (multiplicative identity)" do
      Enum.each(0..255, fn x ->
        assert GF.multiply(x, 1) == x,
               "multiply(#{x}, 1) should be #{x}"
      end)
    end

    test "multiply is commutative" do
      assert GF.multiply(0x12, 0x34) == GF.multiply(0x34, 0x12)
    end

    test "multiply is associative" do
      assert GF.multiply(GF.multiply(2, 3), 5) == GF.multiply(2, GF.multiply(3, 5))
    end

    test "multiply distributes over add" do
      # a*(b+c) = a*b + a*c
      a = 7
      b = 11
      c = 13
      assert GF.multiply(a, GF.add(b, c)) == GF.add(GF.multiply(a, b), GF.multiply(a, c))
    end

    test "multiply(2, 128) = 29 (first reduction step)" do
      # 2 * 128 = 256, overflow, XOR with 0x11D = 29
      assert GF.multiply(2, 128) == 29
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # divide/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "divide" do
    test "divide raises ArgumentError on zero divisor" do
      assert_raise ArgumentError, fn ->
        GF.divide(5, 0)
      end
    end

    test "divide(0, b) = 0 for any non-zero b" do
      assert GF.divide(0, 5) == 0
    end

    test "divide(a, 1) = a (dividing by 1 is identity)" do
      Enum.each(1..255, fn a ->
        assert GF.divide(a, 1) == a
      end)
    end

    test "divide(a, a) = 1 for all non-zero a" do
      Enum.each(1..255, fn a ->
        assert GF.divide(a, a) == 1,
               "divide(#{a}, #{a}) should be 1"
      end)
    end

    test "multiply(a, divide(b, a)) = b" do
      # Division is the inverse of multiplication
      a = 7
      b = 42
      assert GF.multiply(a, GF.divide(b, a)) == b
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # power/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "power" do
    test "power(base, 0) = 1 for any base including 0" do
      assert GF.power(0, 0) == 1
      assert GF.power(2, 0) == 1
      assert GF.power(0xFF, 0) == 1
    end

    test "power(0, n) = 0 for n > 0" do
      assert GF.power(0, 1) == 0
      assert GF.power(0, 10) == 0
    end

    test "power(2, 8) = 29 = 0x1D (first overflow reduction)" do
      assert GF.power(2, 8) == 29
    end

    test "power(2, 255) = 1 (the generator has multiplicative order 255)" do
      # g^255 = 1 is the defining property of the primitive polynomial:
      # the cyclic group of non-zero elements has order exactly 255.
      assert GF.power(2, 255) == 1
    end

    test "power(2, 1) = 2" do
      assert GF.power(2, 1) == 2
    end

    test "power(2, i) = ALOG[i] for all i in 0..254" do
      alog = GF.alog_table()

      Enum.each(0..254, fn i ->
        expected = Enum.at(alog, i)
        got = GF.power(2, i)
        assert got == expected,
               "power(2, #{i}) should be #{expected}, got #{got}"
      end)
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # inverse/1
  # ────────────────────────────────────────────────────────────────────────────

  describe "inverse" do
    test "inverse raises ArgumentError for 0" do
      assert_raise ArgumentError, fn ->
        GF.inverse(0)
      end
    end

    test "inverse(1) = 1 (1 is its own inverse)" do
      assert GF.inverse(1) == 1
    end

    test "inverse(0x53) = 0x8C for 0x11D polynomial" do
      # Well-known test vector for this primitive polynomial.
      assert GF.inverse(0x53) == 0x8C
    end

    test "inverse(0x8C) = 0x53 (inverses are mutual)" do
      assert GF.inverse(0x8C) == 0x53
    end

    test "inverse(inverse(x)) == x for several values" do
      Enum.each([2, 3, 7, 15, 0x53, 0xFF, 0xAB, 0x10, 0x01], fn x ->
        assert GF.inverse(GF.inverse(x)) == x,
               "inverse(inverse(#{x})) should be #{x}"
      end)
    end

    test "x * inverse(x) == 1 for x in 1..10" do
      Enum.each(1..10, fn x ->
        assert GF.multiply(x, GF.inverse(x)) == 1,
               "#{x} * inverse(#{x}) should be 1"
      end)
    end

    test "x * inverse(x) == 1 for all x in 1..255" do
      Enum.each(1..255, fn x ->
        assert GF.multiply(x, GF.inverse(x)) == 1,
               "#{x} * inverse(#{x}) should be 1"
      end)
    end

    test "multiply(0x53, 0x8C) = 1 (inverse pair)" do
      assert GF.multiply(0x53, 0x8C) == 1
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # GF256Field — parameterizable field factory
  # ────────────────────────────────────────────────────────────────────────────

  describe "new_field and field-aware overloads" do
    test "new_field returns a GF256Field struct with polynomial set" do
      field = GF.new_field(0x11B)
      assert %CodingAdventures.GF256Field{polynomial: 0x11B} = field
    end

    test "AES field: multiply(0x53, 0xCA) = 1 (inverses in 0x11B)" do
      aes = GF.new_field(0x11B)
      assert GF.multiply(aes, 0x53, 0xCA) == 1
    end

    test "AES field: multiply(0x57, 0x83) = 0xC1 (FIPS 197 Appendix B)" do
      aes = GF.new_field(0x11B)
      assert GF.multiply(aes, 0x57, 0x83) == 0xC1
    end

    test "AES field: inverse(0x53) = 0xCA" do
      aes = GF.new_field(0x11B)
      assert GF.inverse(aes, 0x53) == 0xCA
    end

    test "RS field (0x11D) matches module-level multiply for sample values" do
      rs = GF.new_field(0x11D)
      for a <- [0, 1, 0x53, 0xCA, 0xFF], b <- [0, 1, 0x8C, 0x7F, 0xFF] do
        assert GF.multiply(rs, a, b) == GF.multiply(a, b),
               "Field(0x11D).multiply(#{a}, #{b}) should match module-level"
      end
    end

    test "field multiply is commutative" do
      aes = GF.new_field(0x11B)
      for a <- [1, 2, 0x53, 0xFF], b <- [1, 2, 0x8C, 0x7F] do
        assert GF.multiply(aes, a, b) == GF.multiply(aes, b, a)
      end
    end

    test "field inverse times self is 1" do
      aes = GF.new_field(0x11B)
      for a <- 1..20 do
        assert GF.multiply(aes, a, GF.inverse(aes, a)) == 1
      end
    end

    test "field divide by zero raises ArgumentError" do
      aes = GF.new_field(0x11B)
      assert_raise ArgumentError, fn -> GF.divide(aes, 5, 0) end
    end

    test "field inverse of zero raises ArgumentError" do
      aes = GF.new_field(0x11B)
      assert_raise ArgumentError, fn -> GF.inverse(aes, 0) end
    end

    test "field add is XOR (polynomial-independent)" do
      aes = GF.new_field(0x11B)
      assert GF.add(aes, 0x53, 0xCA) == Bitwise.bxor(0x53, 0xCA)
    end
  end
end
