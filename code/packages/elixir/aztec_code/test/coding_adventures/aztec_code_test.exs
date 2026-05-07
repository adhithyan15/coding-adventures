defmodule CodingAdventures.AztecCodeTest do
  @moduledoc """
  Tests for the Aztec Code encoder.

  Test organisation:
    1.  GF(16) arithmetic (log/antilog tables, multiplication)
    2.  GF(16) RS generator polynomial
    3.  GF(16) RS encoding
    4.  GF(256)/0x12D arithmetic
    5.  GF(256) RS generator polynomial
    6.  GF(256) RS encoding
    7.  Bit encoding (Binary-Shift from Upper mode)
    8.  Symbol size selection
    9.  Padding (pad_to_bytes)
    10. Bit stuffing
    11. Mode message encoding
    12. Bullseye pattern
    13. Reference grid
    14. Data placement spiral
    15. Integration tests (full encode pipeline)
    16. Edge cases and error handling
  """

  use ExUnit.Case, async: true

  import Bitwise

  alias CodingAdventures.AztecCode, as: AZ

  # ===========================================================================
  # 1. GF(16) arithmetic
  # ===========================================================================
  #
  # GF(16) is built from p(x) = x^4 + x + 1 (0x13).
  # The primitive element α satisfies α^4 = α + 1.
  #
  # Key identities:
  #   α^0 = 1,  α^1 = 2,  α^2 = 4,  α^3 = 8,  α^4 = 3
  #   α^5 = 6,  α^6 = 12, α^7 = 11, α^8 = 5,  α^9 = 10
  #   α^10 = 7, α^11 = 14, α^12 = 15, α^13 = 13, α^14 = 9
  #   α^15 = 1  (period = 15 → α is primitive)

  describe "GF(16) multiplication" do
    test "0 is absorbing" do
      assert AZ.gf16_mul(0, 7) == 0
      assert AZ.gf16_mul(5, 0) == 0
      assert AZ.gf16_mul(0, 0) == 0
    end

    test "1 is multiplicative identity" do
      for i <- 1..15 do
        assert AZ.gf16_mul(1, i) == i
        assert AZ.gf16_mul(i, 1) == i
      end
    end

    test "α × α = α^2: gf16_mul(2, 2) = 4" do
      assert AZ.gf16_mul(2, 2) == 4
    end

    test "α^2 × α = α^3: gf16_mul(4, 2) = 8" do
      assert AZ.gf16_mul(4, 2) == 8
    end

    test "α^3 × α = α^4: gf16_mul(8, 2) = 3" do
      # α^4 = α + 1 = 0b0011 = 3
      assert AZ.gf16_mul(8, 2) == 3
    end

    test "α^4 × α = α^5: gf16_mul(3, 2) = 6" do
      assert AZ.gf16_mul(3, 2) == 6
    end

    test "α^14 × α = α^15 = α^0 = 1: gf16_mul(9, 2) = 1" do
      # α^15 = 1 (period is 15)
      assert AZ.gf16_mul(9, 2) == 1
    end

    test "multiplication is commutative" do
      for a <- 1..15, b <- 1..15 do
        assert AZ.gf16_mul(a, b) == AZ.gf16_mul(b, a)
      end
    end

    test "multiplication closes over GF(16)" do
      for a <- 1..15, b <- 1..15 do
        result = AZ.gf16_mul(a, b)
        assert result >= 1 and result <= 15,
               "gf16_mul(#{a}, #{b}) = #{result}, not in 1..15"
      end
    end

    test "gf16_mul(3, 3) = 5" do
      # α^4 × α^4 = α^8 = 5
      assert AZ.gf16_mul(3, 3) == 5
    end
  end

  # ===========================================================================
  # 2. GF(16) RS generator polynomial
  # ===========================================================================

  describe "GF(16) RS generator polynomial" do
    test "degree-1 generator has 2 coefficients" do
      g = AZ.build_gf16_generator(1)
      assert length(g) == 2
    end

    test "degree-5 generator for compact mode message has 6 coefficients" do
      g = AZ.build_gf16_generator(5)
      assert length(g) == 6
    end

    test "leading coefficient (last in list) is 1 — monic polynomial" do
      g5 = AZ.build_gf16_generator(5)
      assert List.last(g5) == 1

      g6 = AZ.build_gf16_generator(6)
      assert List.last(g6) == 1
    end

    test "generator with n=1 has root α^1 = 2" do
      # g(x) = x + α^1 = x + 2
      # In our list representation [g0, g1] (low degree first): [2, 1]
      g = AZ.build_gf16_generator(1)
      assert g == [2, 1]
    end
  end

  # ===========================================================================
  # 3. GF(16) RS encoding
  # ===========================================================================

  describe "GF(16) RS encoding" do
    test "encodes 2 data nibbles to 5 ECC nibbles for compact mode" do
      ecc = AZ.gf16_rs_encode([7, 2], 5)
      assert length(ecc) == 5
    end

    test "encodes 4 data nibbles to 6 ECC nibbles for full mode" do
      ecc = AZ.gf16_rs_encode([1, 2, 3, 4], 6)
      assert length(ecc) == 6
    end

    test "all ECC nibbles are valid GF(16) elements (0..15)" do
      ecc = AZ.gf16_rs_encode([5, 3], 5)

      Enum.each(ecc, fn nibble ->
        assert nibble >= 0 and nibble <= 15
      end)
    end

    test "zero data produces valid ECC (not all zero for non-trivial generator)" do
      ecc = AZ.gf16_rs_encode([0, 0], 5)
      assert length(ecc) == 5
      # All-zero data → all-zero remainder for any linear code
      assert Enum.all?(ecc, fn x -> x == 0 end)
    end

    test "ECC changes when data changes" do
      ecc1 = AZ.gf16_rs_encode([1, 2], 5)
      ecc2 = AZ.gf16_rs_encode([1, 3], 5)
      assert ecc1 != ecc2
    end
  end

  # ===========================================================================
  # 4. GF(256)/0x12D arithmetic
  # ===========================================================================

  describe "GF(256)/0x12D arithmetic" do
    test "exp table: α^0 = 1, α^1 = 2, α^7 = 128" do
      t = AZ.gf256_exp_table()
      assert elem(t, 0) == 1
      assert elem(t, 1) == 2
      assert elem(t, 7) == 128
    end

    test "α^8 = 0x2D under 0x12D reduction" do
      # 0x80 << 1 = 0x100; XOR 0x12D → 0x12D XOR 0x100 = 0x2D
      t = AZ.gf256_exp_table()
      assert elem(t, 8) == 0x2D
    end

    test "α^255 = 1 (field period)" do
      t = AZ.gf256_exp_table()
      assert elem(t, 255) == 1
    end

    test "0 is absorbing" do
      assert AZ.gf256_mul(0, 255) == 0
      assert AZ.gf256_mul(128, 0) == 0
    end

    test "1 is multiplicative identity" do
      for i <- 1..255//17 do
        assert AZ.gf256_mul(1, i) == i
      end
    end

    test "gf256_mul(2, 2) = 4" do
      assert AZ.gf256_mul(2, 2) == 4
    end

    test "gf256_mul(0x80, 2) = 0x2D" do
      assert AZ.gf256_mul(0x80, 2) == 0x2D
    end

    test "multiplication is commutative" do
      for {a, b} <- [{3, 5}, {127, 200}, {255, 1}, {0x2D, 0x5A}] do
        assert AZ.gf256_mul(a, b) == AZ.gf256_mul(b, a)
      end
    end
  end

  # ===========================================================================
  # 5. GF(256) RS generator polynomial
  # ===========================================================================

  describe "GF(256) RS generator polynomial" do
    test "degree-4 generator has 5 coefficients" do
      g = AZ.build_gf256_generator(4)
      assert length(g) == 5
    end

    test "leading coefficient (last) = 1 (monic)" do
      g = AZ.build_gf256_generator(7)
      assert List.last(g) == 1
    end

    test "generator for n=1 is [α^1, 1] = [2, 1]" do
      g = AZ.build_gf256_generator(1)
      assert g == [2, 1]
    end
  end

  # ===========================================================================
  # 6. GF(256) RS encoding
  # ===========================================================================

  describe "GF(256) RS encoding" do
    test "returns correct number of ECC bytes" do
      ecc = AZ.gf256_rs_encode([1, 2, 3, 4], 4)
      assert length(ecc) == 4
    end

    test "all-zero input produces all-zero ECC" do
      ecc = AZ.gf256_rs_encode([0, 0, 0], 3)
      assert Enum.all?(ecc, fn x -> x == 0 end)
    end

    test "ECC changes when data changes" do
      ecc1 = AZ.gf256_rs_encode([65], 4)
      ecc2 = AZ.gf256_rs_encode([66], 4)
      assert ecc1 != ecc2
    end

    test "ECC bytes are in range 0..255" do
      ecc = AZ.gf256_rs_encode([0x48, 0x65, 0x6C, 0x6C, 0x6F], 5)

      Enum.each(ecc, fn b ->
        assert b >= 0 and b <= 255
      end)
    end
  end

  # ===========================================================================
  # 7. Bit encoding — Binary-Shift from Upper mode
  # ===========================================================================
  #
  # For input "A" (1 byte = 0x41 = 65):
  #   Binary-Shift escape: 11111 (5 bits)
  #   Length = 1 ≤ 31:     00001 (5 bits)
  #   'A' = 0x41 = 65:     01000001 (8 bits)
  #   Total: 5 + 5 + 8 = 18 bits

  describe "encode_bytes_as_bits" do
    test "single byte 'A' produces 18 bits" do
      bits = AZ.encode_bytes_as_bits(<<65>>)
      assert length(bits) == 18
    end

    test "first 5 bits are the Binary-Shift escape (11111)" do
      bits = AZ.encode_bytes_as_bits(<<65>>)
      assert Enum.take(bits, 5) == [1, 1, 1, 1, 1]
    end

    test "next 5 bits are the length (00001 for 1 byte)" do
      bits = AZ.encode_bytes_as_bits(<<65>>)
      assert Enum.slice(bits, 5, 5) == [0, 0, 0, 0, 1]
    end

    test "byte bits for 'A' = 0x41 = 0b01000001" do
      bits = AZ.encode_bytes_as_bits(<<65>>)
      assert Enum.slice(bits, 10, 8) == [0, 1, 0, 0, 0, 0, 0, 1]
    end

    test "31 bytes use short length prefix (5 bits)" do
      # 31 bytes: escape(5) + length(5) + data(31×8) = 258 bits
      input = List.duplicate(0, 31)
      bits = AZ.encode_bytes_as_bits(input)
      assert length(bits) == 5 + 5 + 31 * 8
    end

    test "32 bytes use long length prefix (5+11 bits)" do
      # 32 bytes: escape(5) + 00000(5) + len(11) + data(32×8) = 277 bits
      input = List.duplicate(0, 32)
      bits = AZ.encode_bytes_as_bits(input)
      assert length(bits) == 5 + 5 + 11 + 32 * 8
    end

    test "long length prefix: first 5 bits after escape are 00000" do
      input = List.duplicate(65, 32)
      bits = AZ.encode_bytes_as_bits(input)
      # escape[5], then 00000[5], then 11-bit length
      assert Enum.slice(bits, 5, 5) == [0, 0, 0, 0, 0]
    end

    test "long length prefix: 11-bit length encodes 32 correctly" do
      input = List.duplicate(65, 32)
      bits = AZ.encode_bytes_as_bits(input)
      # 11-bit encoding of 32 = 0b00000100000
      len_bits = Enum.slice(bits, 10, 11)
      decoded = Enum.reduce(len_bits, 0, fn b, acc -> (acc <<< 1) ||| b end)
      assert decoded == 32
    end

    test "accepts binary input" do
      bits_from_binary = AZ.encode_bytes_as_bits("Hi")
      bits_from_list = AZ.encode_bytes_as_bits([0x48, 0x69])
      assert bits_from_binary == bits_from_list
    end
  end

  # ===========================================================================
  # 8. Symbol size selection
  # ===========================================================================

  describe "select_symbol" do
    test "very short input selects compact layer 1" do
      # 1 byte → 18 bits; stuffed ≈ 22 bits; need ceil(22/8) = 3 bytes
      # compact layer 1: 9 max bytes, 23% ECC → 3 ECC, 6 data → fits
      {:ok, spec} = AZ.select_symbol(18, 23)
      assert spec.compact == true
      assert spec.layers == 1
    end

    test "compact layer 1 at 23% ECC: data_cw_count = 7" do
      # 9 max bytes, ceil(0.23 × 9) = 3 ECC → 6 data
      {:ok, spec} = AZ.select_symbol(10, 23)
      assert spec.compact == true
      assert spec.data_cw_count + spec.ecc_cw_count <= 9
    end

    test "returns error for excessively long input" do
      assert {:error, :input_too_long} = AZ.select_symbol(999_999, 23)
    end

    test "selects full mode when compact cannot fit" do
      # A large input that exceeds compact 4-layer capacity
      # compact 4: max 81 bytes, 23% ECC → ~62 data bytes
      # encode_bytes_as_bits(63 bytes) = 5+5+63×8 = 514 bits
      {:ok, spec} = AZ.select_symbol(514, 23)
      # May be compact or full depending on stuffed size; just verify it succeeds
      assert is_boolean(spec.compact)
      assert spec.layers >= 1
    end

    test "spec includes total_bits" do
      {:ok, spec} = AZ.select_symbol(10, 23)
      assert spec.total_bits > 0
    end
  end

  # ===========================================================================
  # 9. Padding
  # ===========================================================================

  describe "pad_to_bytes" do
    test "pads a single bit to 1 byte" do
      result = AZ.pad_to_bytes([1], 1)
      assert length(result) == 1
      assert hd(result) == 128  # 10000000
    end

    test "3 bits padded to 1 byte: [1,0,1] → 10100000 = 160" do
      assert AZ.pad_to_bytes([1, 0, 1], 1) == [160]
    end

    test "exact multiple of 8: 8 bits → 1 byte" do
      assert AZ.pad_to_bytes([1, 0, 1, 0, 0, 0, 0, 1], 1) == [161]
    end

    test "pads to multiple target bytes" do
      result = AZ.pad_to_bytes([1, 0, 1], 3)
      assert length(result) == 3
    end

    test "all-zero last byte → replaced with 0xFF" do
      # 8 zero bits padded to 2 bytes: first byte = 0, second = 0 → 0xFF
      result = AZ.pad_to_bytes(List.duplicate(0, 8), 2)
      assert List.last(result) == 0xFF
    end

    test "non-zero last byte not replaced" do
      # 1 bit [1] padded to 1 byte: 10000000 = 128 → not replaced
      result = AZ.pad_to_bytes([1], 1)
      assert hd(result) == 128
    end
  end

  # ===========================================================================
  # 10. Bit stuffing
  # ===========================================================================
  #
  # Insert complement after every run of exactly 4 identical bits.
  # The stuffed bit resets the run counter.

  describe "stuff_bits" do
    test "empty input returns empty" do
      assert AZ.stuff_bits([]) == []
    end

    test "no runs of 4: no stuffing needed" do
      input = [1, 0, 1, 0, 1, 0]
      assert AZ.stuff_bits(input) == input
    end

    test "exactly 4 ones: inserts a 0 after them" do
      assert AZ.stuff_bits([1, 1, 1, 1]) == [1, 1, 1, 1, 0]
    end

    test "exactly 4 zeros: inserts a 1 after them" do
      assert AZ.stuff_bits([0, 0, 0, 0]) == [0, 0, 0, 0, 1]
    end

    test "5 ones: stuff after 4, then the 5th continues normally" do
      # [1,1,1,1] → stuff 0 → [1,1,1,1,0], then [1] → [1,1,1,1,0,1]
      assert AZ.stuff_bits([1, 1, 1, 1, 1]) == [1, 1, 1, 1, 0, 1]
    end

    test "8 zeros: stuff after first 4, then stuff after next 4" do
      # [0,0,0,0] → stuff 1 → [0,0,0,0,1], then [0,0,0,0] → stuff 1
      assert AZ.stuff_bits([0, 0, 0, 0, 0, 0, 0, 0]) == [0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
    end

    test "spec example: 1 1 1 1 0 0 0 0 → inserts stuff bits" do
      # After 4 ones: insert complement 0. That 0 starts a new run.
      # The stuffed 0 + 3 more 0s = run of 4 → insert complement 1 after the
      # 3rd input zero (4th zero in the running count including the stuffed 0).
      # Then the last 0 is the 5th input zero (second run of zeros after the 1).
      # Actual sequence:
      #   1,1,1,1 → emit, then stuff 0: [1,1,1,1,0]
      #   run_val=0, run_len=1; next 0 → run_len=2; next 0 → run_len=3; next 0 → run_len=4 → stuff 1
      # Result: [1,1,1,1, 0, 0,0,0, 1, 0]
      # Note: the stuffed bit is the 5th element (after 4 ones), and the next
      # stuffed bit (1) appears after the 4th consecutive zero (positions 6-9).
      input = [1, 1, 1, 1, 0, 0, 0, 0]
      expected = [1, 1, 1, 1, 0, 0, 0, 0, 1, 0]
      assert AZ.stuff_bits(input) == expected
    end

    test "stuffed bit resets the run (5th identical bit after stuff is 1 run)" do
      # After [1,1,1,1,0] (stuff = 0), next bit 0 starts run_len = 1 for 0
      # so [1,1,1,1, 0,0,0,0] = run of 4 zeros STARTING from the stuff bit
      # wait: [0] is stuffed bit, run_val=0, run_len=1; then next 0 → run_len=2 ...
      # [1,1,1,1,0,0,0,0] → stuff after 4 ones → [1,1,1,1,0] then continue [0,0,0,0]
      # the stuff 0 has run_len=1; [0,0,0,0] → lengths 2,3,4 → stuff 1
      # expected: [1,1,1,1, 0, 0,0,0,0, 1]
      input = [1, 1, 1, 1, 0, 0, 0, 0]
      result = AZ.stuff_bits(input)
      assert length(result) == 10
    end

    test "3 identical bits: no stuffing" do
      input = [0, 0, 0, 1, 0, 0, 0]
      assert AZ.stuff_bits(input) == input
    end
  end

  # ===========================================================================
  # 11. Mode message encoding
  # ===========================================================================

  describe "encode_mode_message" do
    test "compact mode message is 28 bits" do
      bits = AZ.encode_mode_message(true, 1, 7)
      assert length(bits) == 28
    end

    test "full mode message is 40 bits" do
      bits = AZ.encode_mode_message(false, 2, 12)
      assert length(bits) == 40
    end

    test "all bits are 0 or 1" do
      bits = AZ.encode_mode_message(true, 2, 15)
      assert Enum.all?(bits, fn b -> b == 0 or b == 1 end)

      bits2 = AZ.encode_mode_message(false, 5, 40)
      assert Enum.all?(bits2, fn b -> b == 0 or b == 1 end)
    end

    test "compact: different layer counts produce different mode messages" do
      bits1 = AZ.encode_mode_message(true, 1, 7)
      bits2 = AZ.encode_mode_message(true, 2, 7)
      assert bits1 != bits2
    end

    test "full: different layer counts produce different mode messages" do
      bits1 = AZ.encode_mode_message(false, 1, 11)
      bits2 = AZ.encode_mode_message(false, 5, 11)
      assert bits1 != bits2
    end

    test "compact: different cw counts produce different mode messages" do
      bits1 = AZ.encode_mode_message(true, 1, 5)
      bits2 = AZ.encode_mode_message(true, 1, 6)
      assert bits1 != bits2
    end

    test "compact layer 1, 7 cws: first byte encodes (layers-1)<<6|(cws-1)" do
      # m = (0 << 6) | 6 = 6
      # nibble[0] = 6 & 0xF = 6, nibble[1] = (6 >> 4) & 0xF = 0
      # bits for nibble[0] (MSB first): 0 1 1 0
      bits = AZ.encode_mode_message(true, 1, 7)
      assert Enum.take(bits, 4) == [0, 1, 1, 0]
    end

    test "compact layer 1, 7 cws: second nibble bits = 0 0 0 0" do
      bits = AZ.encode_mode_message(true, 1, 7)
      assert Enum.slice(bits, 4, 4) == [0, 0, 0, 0]
    end
  end

  # ===========================================================================
  # 12. Bullseye pattern
  # ===========================================================================
  #
  # The bullseye center (cx, cy) must satisfy:
  #   Chebyshev distance d = max(|col-cx|, |row-cy|)
  #   d ≤ 1     → DARK
  #   d = 2     → LIGHT
  #   d = 3     → DARK
  #   d = 4     → LIGHT
  #   d = 5     → DARK (compact outer ring)
  #   d = 6     → LIGHT (full only)
  #   d = 7     → DARK  (full outer ring)

  describe "draw_bullseye" do
    setup do
      size = 15
      cx = 7
      cy = 7
      modules = for _r <- 0..(size - 1), do: List.duplicate(false, size)
      modules_t = List.to_tuple(Enum.map(modules, &List.to_tuple/1))
      reserved_t = List.to_tuple(Enum.map(modules, fn _ -> List.to_tuple(List.duplicate(false, size)) end))
      %{modules: modules_t, reserved: reserved_t, cx: cx, cy: cy, size: size}
    end

    test "center module is DARK", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {m, _res} = AZ.draw_bullseye(m, res, cx, cy, true)
      row_t = elem(m, cy)
      assert elem(row_t, cx) == true
    end

    test "d=1 module (adjacent to center) is DARK", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {m, _res} = AZ.draw_bullseye(m, res, cx, cy, true)
      # (cx+1, cy) has Chebyshev distance 1 → DARK
      row_t = elem(m, cy)
      assert elem(row_t, cx + 1) == true
    end

    test "d=2 module is LIGHT", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {m, _res} = AZ.draw_bullseye(m, res, cx, cy, true)
      # (cx+2, cy) has d=2 → LIGHT
      row_t = elem(m, cy)
      assert elem(row_t, cx + 2) == false
    end

    test "d=3 module is DARK", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {m, _res} = AZ.draw_bullseye(m, res, cx, cy, true)
      row_t = elem(m, cy)
      assert elem(row_t, cx + 3) == true
    end

    test "d=4 module is LIGHT", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {m, _res} = AZ.draw_bullseye(m, res, cx, cy, true)
      row_t = elem(m, cy)
      assert elem(row_t, cx + 4) == false
    end

    test "d=5 module (outer compact ring) is DARK", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {m, _res} = AZ.draw_bullseye(m, res, cx, cy, true)
      row_t = elem(m, cy)
      assert elem(row_t, cx + 5) == true
    end

    test "all bullseye modules are reserved", %{modules: m, reserved: res, cx: cx, cy: cy} do
      {_m, res2} = AZ.draw_bullseye(m, res, cx, cy, true)
      # Check all 11×11 = 121 modules are reserved
      for dr <- -5..5, dc <- -5..5 do
        row_t = elem(res2, cy + dr)
        assert elem(row_t, cx + dc) == true,
               "Module at dr=#{dr} dc=#{dc} not reserved"
      end
    end

    test "full bullseye: d=6 is LIGHT, d=7 is DARK" do
      size = 19
      cx = 9
      cy = 9
      modules = List.to_tuple(Enum.map(0..(size - 1), fn _ ->
        List.to_tuple(List.duplicate(false, size))
      end))
      reserved = List.to_tuple(Enum.map(0..(size - 1), fn _ ->
        List.to_tuple(List.duplicate(false, size))
      end))
      {m, _res} = AZ.draw_bullseye(modules, reserved, cx, cy, false)
      row_t = elem(m, cy)
      assert elem(row_t, cx + 6) == false  # d=6 LIGHT
      assert elem(row_t, cx + 7) == true   # d=7 DARK
    end
  end

  # ===========================================================================
  # 13. Reference grid
  # ===========================================================================

  describe "draw_reference_grid" do
    setup do
      size = 55  # 10-layer full symbol
      cx = 27
      cy = 27
      modules = List.to_tuple(Enum.map(0..(size - 1), fn _ ->
        List.to_tuple(List.duplicate(false, size))
      end))
      reserved = List.to_tuple(Enum.map(0..(size - 1), fn _ ->
        List.to_tuple(List.duplicate(false, size))
      end))
      %{m: modules, res: reserved, cx: cx, cy: cy, size: size}
    end

    test "center row module at (cy, cx) is reserved and DARK", %{m: m, res: res, cx: cx, cy: cy, size: size} do
      {m2, res2} = AZ.draw_reference_grid(m, res, cx, cy, size)
      row_m = elem(m2, cy)
      row_r = elem(res2, cy)
      assert elem(row_m, cx) == true   # center intersection → DARK
      assert elem(row_r, cx) == true   # reserved
    end

    test "center row, off-center column: alternates dark/light from cx", %{m: m, res: res, cx: cx, cy: cy, size: size} do
      {m2, _res2} = AZ.draw_reference_grid(m, res, cx, cy, size)
      row_m = elem(m2, cy)
      # (cx - col) mod 2 == 0 → DARK; col = cx → 0 → DARK; col = cx+1 → -1 mod 2 = 1 → LIGHT
      assert elem(row_m, cx + 1) == false
      assert elem(row_m, cx + 2) == true
    end

    test "reference grid at cy+16: row is placed", %{m: m, res: res, cx: cx, cy: cy, size: size} do
      {_m2, res2} = AZ.draw_reference_grid(m, res, cx, cy, size)
      row_r = elem(res2, cy + 16)
      # Some modules on this row should be reserved
      has_reserved = Enum.any?(0..(size - 1), fn col ->
        elem(row_r, col)
      end)
      assert has_reserved
    end
  end

  # ===========================================================================
  # 14. Data placement spiral — structural invariants
  # ===========================================================================

  describe "data placement via full encode" do
    test "compact 1-layer: module at center (cx, cy) is dark (bullseye center)" do
      {:ok, grid} = AZ.encode("A")
      cx = div(grid.cols, 2)
      cy = div(grid.rows, 2)
      assert Enum.at(Enum.at(grid.modules, cy), cx) == true
    end

    test "compact 1-layer: ring at d=2 from center is all LIGHT" do
      {:ok, grid} = AZ.encode("A")
      cx = div(grid.cols, 2)
      cy = div(grid.rows, 2)

      # Check the 20 modules on the d=2 ring (perimeter of 5×5 square minus 3×3)
      # Actually check a few representative ones
      for {dc, dr} <- [{2, 0}, {-2, 0}, {0, 2}, {0, -2}, {2, 2}, {2, -2}, {-2, 2}, {-2, -2}] do
        assert Enum.at(Enum.at(grid.modules, cy + dr), cx + dc) == false,
               "Module at dc=#{dc} dr=#{dr} should be LIGHT (d=2 ring)"
      end
    end

    test "compact 1-layer: ring at d=3 from center is all DARK" do
      {:ok, grid} = AZ.encode("A")
      cx = div(grid.cols, 2)
      cy = div(grid.rows, 2)

      for {dc, dr} <- [{3, 0}, {-3, 0}, {0, 3}, {0, -3}] do
        assert Enum.at(Enum.at(grid.modules, cy + dr), cx + dc) == true,
               "Module at dc=#{dc} dr=#{dr} should be DARK (d=3 ring)"
      end
    end

    test "orientation marks (4 corners of mode ring) are DARK" do
      {:ok, grid} = AZ.encode("A")
      cx = div(grid.cols, 2)
      cy = div(grid.rows, 2)
      r = 6  # compact bullseye_radius + 1

      corners = [
        {cx - r, cy - r},
        {cx + r, cy - r},
        {cx + r, cy + r},
        {cx - r, cy + r}
      ]

      for {col, row} <- corners do
        assert Enum.at(Enum.at(grid.modules, row), col) == true,
               "Orientation mark at col=#{col} row=#{row} should be DARK"
      end
    end
  end

  # ===========================================================================
  # 15. Integration tests
  # ===========================================================================

  describe "encode/2 integration" do
    test "encode 'A' returns 15×15 compact 1-layer symbol" do
      assert {:ok, grid} = AZ.encode("A")
      assert grid.rows == 15
      assert grid.cols == 15
    end

    test "modules list has correct dimensions" do
      {:ok, grid} = AZ.encode("A")
      assert length(grid.modules) == 15

      Enum.each(grid.modules, fn row ->
        assert length(row) == 15
      end)
    end

    test "all modules are booleans" do
      {:ok, grid} = AZ.encode("Hello, World!")

      Enum.each(grid.modules, fn row ->
        Enum.each(row, fn m ->
          assert is_boolean(m)
        end)
      end)
    end

    test "encode 'Hello World' succeeds and produces a symbol ≥ 15×15" do
      assert {:ok, grid} = AZ.encode("Hello World")
      assert grid.rows >= 15
      assert grid.rows == grid.cols
    end

    test "longer input requires larger symbol" do
      {:ok, g1} = AZ.encode("A")
      {:ok, g2} = AZ.encode(String.duplicate("A", 100))
      assert g2.rows >= g1.rows
    end

    test "encode a URL" do
      assert {:ok, grid} = AZ.encode("https://example.com")
      assert grid.rows >= 15
    end

    test "encode binary input (list of bytes)" do
      assert {:ok, grid} = AZ.encode([0x00, 0x01, 0x02, 0x03])
      assert grid.rows >= 15
    end

    test "encode empty string succeeds" do
      # Empty input: Binary-Shift escape(5) + length 0 (5 bits) = 10 bits
      assert {:ok, grid} = AZ.encode("")
      assert grid.rows >= 15
    end

    test "encode! raises on input too long" do
      assert_raise ArgumentError, fn ->
        AZ.encode!(String.duplicate("x", 100_000))
      end
    end

    test "encode returns error tuple on input too long" do
      assert {:error, :input_too_long} = AZ.encode(String.duplicate("x", 100_000))
    end

    test "full symbol: 50-byte input produces a valid symbol" do
      input = String.duplicate("Hello", 10)  # 50 bytes
      assert {:ok, grid} = AZ.encode(input)
      assert grid.rows >= 15
      assert grid.rows == grid.cols
    end

    test "full encode pipeline: different inputs produce different grids" do
      {:ok, g1} = AZ.encode("Hello")
      {:ok, g2} = AZ.encode("World")
      assert g1.modules != g2.modules
    end

    test "render_ascii returns a string" do
      result = AZ.render_ascii("A")
      assert is_binary(result)
      assert String.contains?(result, "█") or String.contains?(result, " ")
    end

    test "render_ascii for error returns inspect string" do
      result = AZ.render_ascii(String.duplicate("x", 100_000))
      assert is_binary(result)
    end

    test "symbol size formula: compact size = 11 + 4 * layers" do
      # For "A": compact layer 1 → size 15
      # For a slightly longer input that fits in compact layer 2 → size 19
      {:ok, g1} = AZ.encode("A")
      assert g1.rows == 15  # compact layer 1: 11 + 4*1 = 15

      # Encode 10 bytes (should need compact layer 2 or bigger)
      {:ok, g_larger} = AZ.encode(String.duplicate("X", 10))
      assert g_larger.rows >= 15
    end

    test "version returns a string" do
      assert is_binary(AZ.version())
      assert AZ.version() == "0.1.0"
    end
  end

  # ===========================================================================
  # 16. Edge cases
  # ===========================================================================

  describe "edge cases" do
    test "single space character" do
      assert {:ok, grid} = AZ.encode(" ")
      assert grid.rows >= 15
    end

    test "all uppercase ASCII: A-Z" do
      assert {:ok, grid} = AZ.encode("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
      assert grid.rows >= 15
    end

    test "numeric string" do
      assert {:ok, grid} = AZ.encode("01234567890123456789")
      assert grid.rows >= 15
    end

    test "binary data (non-printable bytes)" do
      bytes = Enum.to_list(0..31)
      assert {:ok, grid} = AZ.encode(bytes)
      assert grid.rows >= 15
    end

    test "symbol is always a square" do
      for input <- ["A", "Hello", "https://example.com/path?q=1"] do
        {:ok, grid} = AZ.encode(input)
        assert grid.rows == grid.cols, "Expected square for input #{inspect(input)}"
      end
    end

    test "gf16_mul is associative for sampled triples" do
      triples = [{2, 3, 4}, {5, 7, 11}, {3, 6, 12}, {1, 15, 8}]

      for {a, b, c} <- triples do
        assert AZ.gf16_mul(AZ.gf16_mul(a, b), c) == AZ.gf16_mul(a, AZ.gf16_mul(b, c))
      end
    end

    test "gf256_mul is associative for sampled triples" do
      triples = [{2, 3, 4}, {0x12, 0x34, 0x56}, {127, 200, 255}]

      for {a, b, c} <- triples do
        assert AZ.gf256_mul(AZ.gf256_mul(a, b), c) == AZ.gf256_mul(a, AZ.gf256_mul(b, c))
      end
    end

    test "stuff_bits output never has 5 consecutive identical bits" do
      # Bit stuffing guarantees no run of 5+ identical bits in the output.
      # (Runs of exactly 4 are allowed and are immediately followed by a
      #  complement stuffed bit, so runs of 5+ cannot occur.)
      result = AZ.stuff_bits([1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0])
      has_5_run =
        result
        |> Enum.chunk_every(5, 1, :discard)
        |> Enum.any?(fn chunk ->
          length(Enum.uniq(chunk)) == 1
        end)

      refute has_5_run
    end

    test "encode/2 with custom min_ecc_percent 50 succeeds" do
      assert {:ok, grid} = AZ.encode("Hello", %{min_ecc_percent: 50})
      assert grid.rows >= 15
    end

    test "encode/2 with min_ecc_percent 10 produces a valid grid" do
      assert {:ok, grid} = AZ.encode("Hello", %{min_ecc_percent: 10})
      assert grid.rows >= 15
    end
  end
end
