defmodule CodingAdventures.QrCodeTest do
  use ExUnit.Case

  import Bitwise

  alias CodingAdventures.QrCode
  alias CodingAdventures.QrCode.Tables
  alias CodingAdventures.QrCode.Encoder
  alias CodingAdventures.QrCode.RS
  alias CodingAdventures.Barcode2D.ModuleGrid

  # ============================================================================
  # version/0
  # ============================================================================

  describe "version/0" do
    test "returns the package version string" do
      assert QrCode.version() == "0.1.0"
    end
  end

  # ============================================================================
  # encode/2 — basic output shape
  # ============================================================================

  describe "encode/2 — output shape" do
    test "returns {:ok, ModuleGrid} for a simple string" do
      assert {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      assert %ModuleGrid{} = grid
    end

    test "grid module_shape is :square" do
      {:ok, grid} = QrCode.encode("A", :m)
      assert grid.module_shape == :square
    end

    test "grid rows == cols (always square)" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert grid.rows == grid.cols
    end

    test "grid rows is a positive integer" do
      {:ok, grid} = QrCode.encode("X", :m)
      assert is_integer(grid.rows)
      assert grid.rows > 0
    end

    test "modules list has rows rows" do
      {:ok, grid} = QrCode.encode("HELLO", :m)
      assert length(grid.modules) == grid.rows
    end

    test "each row in modules has cols entries" do
      {:ok, grid} = QrCode.encode("HELLO", :m)
      Enum.each(grid.modules, fn row ->
        assert length(row) == grid.cols
      end)
    end

    test "all module values are booleans" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      Enum.each(grid.modules, fn row ->
        Enum.each(row, fn m -> assert is_boolean(m) end)
      end)
    end
  end

  # ============================================================================
  # encode/2 — version-size relationship
  # ============================================================================

  describe "encode/2 — version and symbol size" do
    test "version 1 symbol is 21x21" do
      # 'A' in :h fits in version 1
      {:ok, grid} = QrCode.encode("A", :h)
      assert grid.rows == 21
      assert grid.cols == 21
    end

    test "single digit fits in version 1" do
      {:ok, grid} = QrCode.encode("0", :m)
      assert grid.rows == 21
    end

    test "empty string encodes to version 1" do
      {:ok, grid} = QrCode.encode("", :m)
      assert grid.rows == 21
    end

    test "symbol size formula: 4*version+17" do
      # For version 1: 4*1+17 = 21
      {:ok, grid} = QrCode.encode("A", :m)
      sz = grid.rows
      # sz = 4*v+17 so v = (sz-17)/4
      ver = div(sz - 17, 4)
      assert sz == 4 * ver + 17
    end

    test "HELLO WORLD encodes at version 1 in :m" do
      # HELLO WORLD is 11 chars, alphanumeric, fits in v1 at M
      {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      assert grid.rows == 21
    end

    test "longer input uses higher version" do
      {:ok, g1} = QrCode.encode("A", :m)
      {:ok, g2} = QrCode.encode(String.duplicate("A", 100), :m)
      assert g2.rows > g1.rows
    end
  end

  # ============================================================================
  # encode/2 — ECC levels
  # ============================================================================

  describe "encode/2 — ECC levels" do
    test "default ECC level is :m" do
      {:ok, g_default} = QrCode.encode("HELLO WORLD")
      {:ok, g_m} = QrCode.encode("HELLO WORLD", :m)
      # Same size means same version was selected
      assert g_default.rows == g_m.rows
    end

    test ":l produces a result" do
      assert {:ok, _} = QrCode.encode("TEST", :l)
    end

    test ":m produces a result" do
      assert {:ok, _} = QrCode.encode("TEST", :m)
    end

    test ":q produces a result" do
      assert {:ok, _} = QrCode.encode("TEST", :q)
    end

    test ":h produces a result" do
      assert {:ok, _} = QrCode.encode("TEST", :h)
    end

    test ":l uses same or smaller version than :m for same input" do
      # Higher ECC → more redundancy → less data capacity → higher version.
      # Lower ECC → fewer redundancy bytes → can fit same data in smaller version.
      {:ok, g_l} = QrCode.encode(String.duplicate("X", 50), :l)
      {:ok, g_h} = QrCode.encode(String.duplicate("X", 50), :h)
      assert g_l.rows <= g_h.rows
    end

    test ":h uses same or larger version than :l for same input" do
      {:ok, g_l} = QrCode.encode("HELLO WORLD", :l)
      {:ok, g_h} = QrCode.encode("HELLO WORLD", :h)
      assert g_h.rows >= g_l.rows
    end
  end

  # ============================================================================
  # encode/2 — error cases
  # ============================================================================

  describe "encode/2 — error cases" do
    test "returns error for input exceeding max length" do
      # 7090 bytes exceeds the 7089-byte guard
      huge = String.duplicate("A", 7090)
      assert {:error, :input_too_long} = QrCode.encode(huge, :m)
    end

    test "returns {:error, :input_too_long} not a raise" do
      huge = String.duplicate("A", 7090)
      result = QrCode.encode(huge, :m)
      assert match?({:error, :input_too_long}, result)
    end
  end

  # ============================================================================
  # encode/2 — structural correctness
  # ============================================================================

  describe "encode/2 — structural correctness" do
    test "top-left finder top-left corner is dark" do
      # The top-left finder pattern starts at (0,0). Its border is dark.
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert Enum.at(Enum.at(grid.modules, 0), 0) == true
    end

    test "top-left finder top-right corner is dark" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert Enum.at(Enum.at(grid.modules, 0), 6) == true
    end

    test "top-left finder bottom-left corner is dark" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert Enum.at(Enum.at(grid.modules, 6), 0) == true
    end

    test "top-left finder center (3,3) is dark" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert Enum.at(Enum.at(grid.modules, 3), 3) == true
    end

    test "top-left finder inner white ring (1,1) is light" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert Enum.at(Enum.at(grid.modules, 1), 1) == false
    end

    test "separator row 7 col 0 is light" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      assert Enum.at(Enum.at(grid.modules, 7), 0) == false
    end

    test "dark module at (4*v+9, 8) is always dark" do
      # For version 1, dark module is at row 4*1+9=13, col 8.
      {:ok, grid} = QrCode.encode("A", :m)
      sz = grid.rows
      ver = div(sz - 17, 4)
      dark_row = 4 * ver + 9
      assert Enum.at(Enum.at(grid.modules, dark_row), 8) == true
    end

    test "grid has some dark modules and some light modules" do
      {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      all_vals = List.flatten(grid.modules)
      assert Enum.any?(all_vals, & &1)
      assert Enum.any?(all_vals, &(not &1))
    end

    test "encoding produces a scannable-looking grid (not all dark)" do
      {:ok, grid} = QrCode.encode("https://example.com", :m)
      dark_count = grid.modules |> List.flatten() |> Enum.count(& &1)
      total = grid.rows * grid.cols
      # A valid QR code is roughly 40-60% dark
      ratio = dark_count / total
      assert ratio > 0.2 and ratio < 0.8
    end
  end

  # ============================================================================
  # encode/2 — different encoding modes
  # ============================================================================

  describe "encode/2 — encoding modes" do
    test "numeric mode: all digits" do
      assert {:ok, grid} = QrCode.encode("0123456789", :m)
      assert grid.rows == 21
    end

    test "alphanumeric mode: uppercase letters and digits" do
      assert {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      assert grid.rows == 21
    end

    test "byte mode: lowercase letters" do
      assert {:ok, grid} = QrCode.encode("hello world", :m)
      assert %ModuleGrid{} = grid
    end

    test "byte mode: UTF-8 string" do
      assert {:ok, _} = QrCode.encode("こんにちは", :m)
    end

    test "byte mode: mixed case string" do
      assert {:ok, grid} = QrCode.encode("Hello, World!", :m)
      assert %ModuleGrid{} = grid
    end

    test "numeric mode uses fewer or equal modules than byte mode" do
      {:ok, g_num} = QrCode.encode("0123456789", :m)
      {:ok, g_byte} = QrCode.encode("hello12345", :m)
      # Numeric fits in smaller or equal version
      assert g_num.rows <= g_byte.rows
    end

    test "URL encodes successfully" do
      assert {:ok, grid} = QrCode.encode("https://example.com", :m)
      assert grid.rows > 0
    end

    test "single character encodes" do
      assert {:ok, _} = QrCode.encode("A", :m)
    end

    test "space character (byte mode) encodes" do
      # Lowercase with space — byte mode
      assert {:ok, _} = QrCode.encode("hello world", :m)
    end
  end

  # ============================================================================
  # Tables module
  # ============================================================================

  describe "Tables — ecc_index/1" do
    test ":L maps to 0" do
      assert Tables.ecc_index(:L) == 0
    end

    test ":M maps to 1" do
      assert Tables.ecc_index(:M) == 1
    end

    test ":Q maps to 2" do
      assert Tables.ecc_index(:Q) == 2
    end

    test ":H maps to 3" do
      assert Tables.ecc_index(:H) == 3
    end
  end

  describe "Tables — ecc_indicator/1" do
    test ":L indicator is 1 (0b01)" do
      assert Tables.ecc_indicator(:L) == 0b01
    end

    test ":M indicator is 0 (0b00)" do
      assert Tables.ecc_indicator(:M) == 0b00
    end

    test ":Q indicator is 3 (0b11)" do
      assert Tables.ecc_indicator(:Q) == 0b11
    end

    test ":H indicator is 2 (0b10)" do
      assert Tables.ecc_indicator(:H) == 0b10
    end
  end

  describe "Tables — ecc_codewords_per_block/2" do
    test "version 1 :L gives 7" do
      assert Tables.ecc_codewords_per_block(:L, 1) == 7
    end

    test "version 1 :M gives 10" do
      assert Tables.ecc_codewords_per_block(:M, 1) == 10
    end

    test "version 1 :Q gives 13" do
      assert Tables.ecc_codewords_per_block(:Q, 1) == 13
    end

    test "version 1 :H gives 17" do
      assert Tables.ecc_codewords_per_block(:H, 1) == 17
    end

    test "version 5 :M gives 24" do
      assert Tables.ecc_codewords_per_block(:M, 5) == 24
    end
  end

  describe "Tables — num_blocks/2" do
    test "version 1 :M has 1 block" do
      assert Tables.num_blocks(:M, 1) == 1
    end

    test "version 5 :M has 2 blocks" do
      assert Tables.num_blocks(:M, 5) == 2
    end

    test "version 10 :H has 8 blocks" do
      assert Tables.num_blocks(:H, 10) == 8
    end
  end

  describe "Tables — alignment_positions/1" do
    test "version 1 has no alignment positions" do
      assert Tables.alignment_positions(1) == []
    end

    test "version 2 has positions [6, 18]" do
      assert Tables.alignment_positions(2) == [6, 18]
    end

    test "version 7 has positions [6, 22, 38]" do
      assert Tables.alignment_positions(7) == [6, 22, 38]
    end

    test "version 40 has 7 alignment positions" do
      assert length(Tables.alignment_positions(40)) == 7
    end
  end

  describe "Tables — remainder_bits/1" do
    test "version 1 has 0 remainder bits" do
      assert Tables.remainder_bits(1) == 0
    end

    test "version 2 has 7 remainder bits" do
      assert Tables.remainder_bits(2) == 7
    end

    test "version 14 has 3 remainder bits" do
      assert Tables.remainder_bits(14) == 3
    end

    test "version 21 has 4 remainder bits" do
      assert Tables.remainder_bits(21) == 4
    end
  end

  describe "Tables — num_raw_data_modules/1" do
    test "version 1 has 208 raw data modules" do
      assert Tables.num_raw_data_modules(1) == 208
    end

    test "version 2 has more raw data modules than version 1" do
      assert Tables.num_raw_data_modules(2) > Tables.num_raw_data_modules(1)
    end

    test "raw data modules increase with version" do
      Enum.reduce(1..40, 0, fn v, prev ->
        curr = Tables.num_raw_data_modules(v)
        assert curr > prev
        curr
      end)
    end
  end

  describe "Tables — num_data_codewords/2" do
    test "version 1 :L has 19 data codewords" do
      assert Tables.num_data_codewords(1, :L) == 19
    end

    test "version 1 :M has 16 data codewords" do
      assert Tables.num_data_codewords(1, :M) == 16
    end

    test "version 1 :Q has 13 data codewords" do
      assert Tables.num_data_codewords(1, :Q) == 13
    end

    test "version 1 :H has 9 data codewords" do
      assert Tables.num_data_codewords(1, :H) == 9
    end

    test "data codewords increase with version (same ECC)" do
      Enum.reduce(1..40, 0, fn v, prev ->
        curr = Tables.num_data_codewords(v, :M)
        assert curr > prev
        curr
      end)
    end
  end

  describe "Tables — generator/1" do
    test "generator(7) has length 8" do
      assert length(Tables.generator(7)) == 8
    end

    test "generator(10) has length 11" do
      assert length(Tables.generator(10)) == 11
    end

    test "generator(7) leading coefficient is 1 (monic)" do
      gen = Tables.generator(7)
      assert hd(gen) == 1
    end

    test "all generator coefficients are bytes (0..255)" do
      gen = Tables.generator(7)
      Enum.each(gen, fn c -> assert c >= 0 and c <= 255 end)
    end
  end

  # ============================================================================
  # Encoder module
  # ============================================================================

  describe "Encoder — select_mode/1" do
    test "all-digits input selects :numeric" do
      assert Encoder.select_mode("01234") == :numeric
    end

    test "empty string selects :numeric" do
      assert Encoder.select_mode("") == :numeric
    end

    test "uppercase letters and digits select :alphanumeric" do
      assert Encoder.select_mode("HELLO WORLD") == :alphanumeric
    end

    test "lowercase letters select :byte" do
      assert Encoder.select_mode("hello") == :byte
    end

    test "mixed case selects :byte" do
      assert Encoder.select_mode("Hello") == :byte
    end

    test "special chars outside alphanumeric set select :byte" do
      assert Encoder.select_mode("hello!") == :byte
    end

    test "URL selects :byte" do
      assert Encoder.select_mode("https://example.com") == :byte
    end
  end

  describe "Encoder — numeric?/1" do
    test "all digits returns true" do
      assert Encoder.numeric?("0123456789")
    end

    test "empty string returns true" do
      assert Encoder.numeric?("")
    end

    test "letters return false" do
      refute Encoder.numeric?("A1")
    end

    test "mixed returns false" do
      refute Encoder.numeric?("12 34")
    end
  end

  describe "Encoder — alphanumeric?/1" do
    test "digits and uppercase letters return true" do
      assert Encoder.alphanumeric?("HELLO WORLD")
    end

    test "lowercase returns false" do
      refute Encoder.alphanumeric?("hello")
    end

    test "special chars in set return true" do
      assert Encoder.alphanumeric?("$%*+-./:")
    end

    test "chars outside set return false" do
      refute Encoder.alphanumeric?("@")
    end
  end

  describe "Encoder — char_count_bits/2" do
    test "numeric mode, version 1 → 10 bits" do
      assert Encoder.char_count_bits(:numeric, 1) == 10
    end

    test "numeric mode, version 9 → 10 bits" do
      assert Encoder.char_count_bits(:numeric, 9) == 10
    end

    test "numeric mode, version 10 → 12 bits" do
      assert Encoder.char_count_bits(:numeric, 10) == 12
    end

    test "numeric mode, version 27 → 14 bits" do
      assert Encoder.char_count_bits(:numeric, 27) == 14
    end

    test "alphanumeric mode, version 1 → 9 bits" do
      assert Encoder.char_count_bits(:alphanumeric, 1) == 9
    end

    test "alphanumeric mode, version 10 → 11 bits" do
      assert Encoder.char_count_bits(:alphanumeric, 10) == 11
    end

    test "alphanumeric mode, version 27 → 13 bits" do
      assert Encoder.char_count_bits(:alphanumeric, 27) == 13
    end

    test "byte mode, version 1 → 8 bits" do
      assert Encoder.char_count_bits(:byte, 1) == 8
    end

    test "byte mode, version 10 → 16 bits" do
      assert Encoder.char_count_bits(:byte, 10) == 16
    end
  end

  describe "Encoder — select_version/2" do
    test "short string fits in version 1" do
      assert {:ok, 1} = Encoder.select_version("A", :m)
    end

    test "HELLO WORLD fits in version 1 at :m" do
      assert {:ok, 1} = Encoder.select_version("HELLO WORLD", :m)
    end

    test "longer string needs higher version" do
      {:ok, v1} = Encoder.select_version("A", :m)
      {:ok, v2} = Encoder.select_version(String.duplicate("A", 100), :m)
      assert v2 > v1
    end

    test "returns error for input exceeding v40 capacity" do
      # Create a string that's definitely too long for any ECC level
      huge = String.duplicate("A", 3000)
      assert {:error, :input_too_long} = Encoder.select_version(huge, :h)
    end

    test "version is in range 1..40" do
      {:ok, v} = Encoder.select_version("HELLO", :m)
      assert v >= 1 and v <= 40
    end
  end

  describe "Encoder — build_data_codewords/3" do
    test "returns a list of bytes" do
      cw = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      assert is_list(cw)
      Enum.each(cw, fn b -> assert is_integer(b) and b >= 0 and b <= 255 end)
    end

    test "length matches num_data_codewords" do
      cw = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      expected = Tables.num_data_codewords(1, :m)
      assert length(cw) == expected
    end

    test "HELLO WORLD v1 M starts with correct mode indicator" do
      # Alphanumeric mode indicator is 0b0010, so first byte should have
      # 0b0010 in the top 4 bits followed by the char count.
      # HELLO WORLD = 11 chars. char_count_bits(alphanumeric, 1) = 9 bits.
      # First byte: 0010_0000 = 0x20... wait, the first 4 bits are the mode
      # (0b0010) and the next 9 bits are the char count (11 = 0b000001011).
      # Together: 0b0010_0000_0101_1...
      # First byte: 0b00100000 = 0x20
      cw = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      # Mode indicator for alphanumeric = 0b0010 = 2
      # This goes in bits 7..4 of the first byte
      first_byte = hd(cw)
      # Top 4 bits should be 0010 (alphanumeric mode)
      assert (first_byte >>> 4) == 0b0010
    end

    test "numeric '0' v1 M has correct mode indicator" do
      cw = Encoder.build_data_codewords("0", 1, :m)
      first_byte = hd(cw)
      # Mode indicator for numeric = 0b0001
      assert (first_byte >>> 4) == 0b0001
    end
  end

  describe "Encoder — encode_numeric/2" do
    test "encodes empty string to empty bits" do
      assert Encoder.encode_numeric("") == []
    end

    test "encodes 3-digit group to 10 bits" do
      bits = Encoder.encode_numeric("012")
      assert length(bits) == 10
    end

    test "012 encodes to 10-bit value 12 (binary: 0000001100)" do
      bits = Encoder.encode_numeric("012")
      # 012 = 12 decimal
      # 12 in 10 bits: 0000001100
      assert bits == [0, 0, 0, 0, 0, 0, 1, 1, 0, 0]
    end

    test "encodes 2-digit group to 7 bits" do
      bits = Encoder.encode_numeric("01")
      assert length(bits) == 7
    end

    test "encodes single digit to 4 bits" do
      bits = Encoder.encode_numeric("5")
      assert length(bits) == 4
      # 5 in 4 bits: 0101
      assert bits == [0, 1, 0, 1]
    end

    test "encodes 6-digit number to 20 bits (two 3-digit groups)" do
      bits = Encoder.encode_numeric("012345")
      assert length(bits) == 20
    end
  end

  describe "Encoder — encode_alphanumeric/2" do
    test "encodes empty string to empty bits" do
      assert Encoder.encode_alphanumeric("") == []
    end

    test "encodes pair of chars to 11 bits" do
      bits = Encoder.encode_alphanumeric("AC")
      assert length(bits) == 11
    end

    test "encodes single char to 6 bits" do
      bits = Encoder.encode_alphanumeric("A")
      assert length(bits) == 6
    end

    test "A has index 10" do
      bits = Encoder.encode_alphanumeric("A")
      # 10 in 6 bits: 001010
      assert bits == [0, 0, 1, 0, 1, 0]
    end

    test "encodes 4-char string to 22 bits (two 11-bit pairs)" do
      bits = Encoder.encode_alphanumeric("ABCD")
      assert length(bits) == 22
    end
  end

  describe "Encoder — encode_byte/2" do
    test "encodes empty string to empty bits" do
      assert Encoder.encode_byte("") == []
    end

    test "encodes one ASCII byte to 8 bits" do
      bits = Encoder.encode_byte("A")
      assert length(bits) == 8
    end

    test "'A' encodes to 0x41 = 0b01000001" do
      bits = Encoder.encode_byte("A")
      assert bits == [0, 1, 0, 0, 0, 0, 0, 1]
    end

    test "multi-byte UTF-8 char encodes each byte separately" do
      # 'é' is 2 bytes in UTF-8: 0xC3 0xA9
      bits = Encoder.encode_byte("é")
      assert length(bits) == 16
    end
  end

  # ============================================================================
  # RS module
  # ============================================================================

  describe "RS — encode_block/2" do
    test "returns ECC bytes equal to n_ecc (generator length - 1)" do
      gen = Tables.generator(7)
      ecc = RS.encode_block([32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236], gen)
      # generator(7) has length 8, so n_ecc = 7
      assert length(ecc) == 7
    end

    test "all ECC bytes are in range 0..255" do
      gen = Tables.generator(10)
      data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 236, 17, 236]
      ecc = RS.encode_block(data, gen)
      Enum.each(ecc, fn b -> assert b >= 0 and b <= 255 end)
    end

    test "empty data produces zero ECC bytes" do
      gen = Tables.generator(7)
      ecc = RS.encode_block([], gen)
      assert Enum.all?(ecc, &(&1 == 0))
    end
  end

  describe "RS — compute_blocks/3" do
    test "returns one block for version 1 :m" do
      # Version 1 :M has 1 block.
      data = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      blocks = RS.compute_blocks(data, 1, :m)
      assert length(blocks) == 1
    end

    test "each block is a {data, ecc} tuple" do
      data = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      [{d, e}] = RS.compute_blocks(data, 1, :m)
      assert is_list(d)
      assert is_list(e)
    end

    test "data bytes in block sum to total data codewords" do
      data = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      blocks = RS.compute_blocks(data, 1, :m)
      total = Enum.reduce(blocks, 0, fn {d, _e}, acc -> acc + length(d) end)
      assert total == Tables.num_data_codewords(1, :m)
    end

    test "ecc bytes per block match ECC_CODEWORDS_PER_BLOCK table" do
      data = Encoder.build_data_codewords("HELLO WORLD", 1, :m)
      blocks = RS.compute_blocks(data, 1, :m)
      expected_ecc_len = Tables.ecc_codewords_per_block(:m, 1)
      Enum.each(blocks, fn {_d, e} -> assert length(e) == expected_ecc_len end)
    end

    test "version 5 :m has 2 blocks" do
      data = Encoder.build_data_codewords(String.duplicate("A", 30), 5, :m)
      blocks = RS.compute_blocks(data, 5, :m)
      assert length(blocks) == Tables.num_blocks(:m, 5)
    end
  end

  describe "RS — interleave_blocks/1" do
    test "single block: interleaved == data ++ ecc" do
      data = [1, 2, 3]
      ecc = [10, 20]
      result = RS.interleave_blocks([{data, ecc}])
      assert result == [1, 2, 3, 10, 20]
    end

    test "two equal-length blocks are correctly interleaved" do
      # Data: block0=[1,2], block1=[3,4]  ECC: block0=[5,6], block1=[7,8]
      # Interleaved data: 1, 3, 2, 4
      # Interleaved ECC: 5, 7, 6, 8
      result = RS.interleave_blocks([{[1, 2], [5, 6]}, {[3, 4], [7, 8]}])
      assert result == [1, 3, 2, 4, 5, 7, 6, 8]
    end

    test "total length = sum of all data + ecc bytes" do
      blocks = [
        {[1, 2, 3], [10, 11, 12, 13]},
        {[4, 5, 6], [14, 15, 16, 17]}
      ]
      result = RS.interleave_blocks(blocks)
      expected_len = 3 + 3 + 4 + 4
      assert length(result) == expected_len
    end

    test "unequal data blocks are handled correctly" do
      # One 2-byte block and one 3-byte block.
      # Data interleave: b0[0], b1[0], b0[1], b1[1], b1[2]
      result = RS.interleave_blocks([{[1, 2], [10]}, {[3, 4, 5], [20]}])
      data_part = Enum.take(result, 5)
      assert data_part == [1, 3, 2, 4, 5]
    end
  end

  # ============================================================================
  # encode/2 — reproducibility
  # ============================================================================

  describe "encode/2 — reproducibility" do
    test "same input produces identical grids" do
      {:ok, g1} = QrCode.encode("HELLO WORLD", :m)
      {:ok, g2} = QrCode.encode("HELLO WORLD", :m)
      assert g1.modules == g2.modules
    end

    test "different inputs produce different grids" do
      {:ok, g1} = QrCode.encode("HELLO WORLD", :m)
      {:ok, g2} = QrCode.encode("GOODBYE WORLD", :m)
      # They may be the same size (version) but different bit patterns
      assert g1.modules != g2.modules
    end
  end

  # ============================================================================
  # Integration: known QR code properties
  # ============================================================================

  describe "Integration — known QR code properties" do
    test "version 1 has exactly 21x21 = 441 modules total" do
      {:ok, grid} = QrCode.encode("A", :h)
      assert length(List.flatten(grid.modules)) == 441
    end

    test "version 2 has exactly 25x25 = 625 modules total" do
      # Need a string that forces version 2
      # Version 1 M holds up to 25 alphanumeric chars; version 1 H holds up to 10
      {:ok, grid} = QrCode.encode(String.duplicate("A", 15), :h)
      assert grid.rows == 25
      assert length(List.flatten(grid.modules)) == 625
    end

    test "timing strips on row 6 alternate starting dark" do
      # In a version 2+ symbol, row 6 (timing strip) alternates dark/light.
      # Columns 8..size-9 should alternate, starting with dark at even index.
      {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      sz = grid.rows
      row6 = Enum.at(grid.modules, 6)

      # Check timing strip region (cols 8..sz-9).
      # Note: the mask is applied to non-reserved modules only. Timing strip
      # is reserved and not masked, so it should keep its original alternating pattern.
      Enum.each(8..(sz - 9), fn c ->
        expected_dark = rem(c, 2) == 0
        actual = Enum.at(row6, c)
        assert actual == expected_dark, "Timing strip at col #{c}: expected #{expected_dark}, got #{actual}"
      end)
    end

    test "top-right finder top-right corner is dark" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      sz = grid.rows
      # Top-right finder occupies cols sz-7..sz-1, rows 0..6.
      assert Enum.at(Enum.at(grid.modules, 0), sz - 1) == true
    end

    test "bottom-left finder bottom-left corner is dark" do
      {:ok, grid} = QrCode.encode("TEST", :m)
      sz = grid.rows
      # Bottom-left finder occupies rows sz-7..sz-1, cols 0..6.
      assert Enum.at(Enum.at(grid.modules, sz - 1), 0) == true
    end

    test "top-left finder inner light ring at (1,1) is light" do
      {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      assert Enum.at(Enum.at(grid.modules, 1), 1) == false
    end

    test "top-left finder inner light ring at (1,5) is light" do
      {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      assert Enum.at(Enum.at(grid.modules, 1), 5) == false
    end

    test "top-left finder inner light ring at (5,1) is light" do
      {:ok, grid} = QrCode.encode("HELLO WORLD", :m)
      assert Enum.at(Enum.at(grid.modules, 5), 1) == false
    end
  end
end
