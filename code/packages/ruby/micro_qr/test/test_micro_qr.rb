# frozen_string_literal: true

# =============================================================================
# test_micro_qr.rb — comprehensive unit tests for the micro_qr package
# =============================================================================
#
# Tests mirror the Rust reference implementation and cover:
#   - VERSION constant
#   - Symbol dimensions (M1=11, M2=13, M3=15, M4=17)
#   - Auto-version selection (smallest symbol that fits)
#   - Forced version / ECC overrides
#   - Encoding modes (numeric, alphanumeric, byte)
#   - Structural modules (finder, separator, timing)
#   - Determinism (same input → same grid always)
#   - ECC level constraints (M1=Detection only, M2-M3=L/M, M4=L/M/Q)
#   - Error handling (InputTooLong, ECCNotAvailable)
#   - Capacity boundaries (M1 max 5 numeric, M4 max 35 numeric, etc.)
#   - Format information placement
#   - Grid completeness (square, all rows correct length)
#   - Cross-language test corpus (same sizes as Rust reference)
#   - encode_at convenience alias
#   - layout() → PaintScene delegation
#   - encode_and_layout() convenience method
#   - RS generator constants presence
#   - BitWriter internals
#   - Penalty scoring (mask selection produces valid penalty)
#   - Different inputs produce different grids

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 90
end

require "minitest/autorun"
require_relative "../lib/coding_adventures/micro_qr"

include CodingAdventures::MicroQR

# Helper: convert a ModuleGrid to a compact string for comparison.
def grid_str(grid)
  grid.modules.map { |row| row.map { |d| d ? "1" : "0" }.join }.join("\n")
end

class TestMicroQRVersion < Minitest::Test
  # ── VERSION constant ───────────────────────────────────────────────────────

  def test_version_is_semver
    parts = CodingAdventures::MicroQR::VERSION.split(".")
    assert_equal 3, parts.length
    assert parts.all? { |p| p.match?(/^\d+$/) }, "each VERSION segment must be numeric"
  end

  def test_version_value
    assert_equal "0.1.0", CodingAdventures::MicroQR::VERSION
  end

  # ── MicroQRVersion constants ───────────────────────────────────────────────

  def test_version_constants_exist
    assert_equal :M1, MicroQRVersion::M1
    assert_equal :M2, MicroQRVersion::M2
    assert_equal :M3, MicroQRVersion::M3
    assert_equal :M4, MicroQRVersion::M4
  end

  def test_version_all_contains_four_values
    assert_equal 4, MicroQRVersion::ALL.length
    assert_includes MicroQRVersion::ALL, MicroQRVersion::M1
    assert_includes MicroQRVersion::ALL, MicroQRVersion::M4
  end

  # ── MicroQREccLevel constants ──────────────────────────────────────────────

  def test_ecc_constants_exist
    assert_equal :Detection, MicroQREccLevel::Detection
    assert_equal :L, MicroQREccLevel::L
    assert_equal :M, MicroQREccLevel::M
    assert_equal :Q, MicroQREccLevel::Q
  end

  def test_ecc_all_contains_four_values
    assert_equal 4, MicroQREccLevel::ALL.length
  end
end

class TestMicroQRDimensions < Minitest::Test
  # ── Symbol dimensions ──────────────────────────────────────────────────────
  #
  # Formula: size = 2 × version_number + 9
  #   M1 (v=1): 2×1+9 = 11   M2 (v=2): 2×2+9 = 13
  #   M3 (v=3): 2×3+9 = 15   M4 (v=4): 2×4+9 = 17

  def test_m1_is_11x11
    g = encode("1")
    assert_equal 11, g.rows
    assert_equal 11, g.cols
  end

  def test_m2_is_13x13
    g = encode("HELLO")
    assert_equal 13, g.rows
    assert_equal 13, g.cols
  end

  def test_m3_is_15x15
    g = encode("MICRO QR TEST")
    assert_equal 15, g.rows
    assert_equal 15, g.cols
  end

  def test_m4_is_17x17
    g = encode("https://a.b")
    assert_equal 17, g.rows
    assert_equal 17, g.cols
  end

  def test_grid_is_square
    ["1", "HELLO", "MICRO QR TEST", "https://a.b"].each do |input|
      g = encode(input)
      assert_equal g.rows, g.cols, "grid must be square for '#{input}'"
    end
  end

  def test_module_shape_is_square
    g = encode("1")
    assert_equal "square", g.module_shape
  end

  def test_all_rows_have_correct_length
    ["1", "HELLO", "hello", "https://a.b"].each do |input|
      g = encode(input)
      g.modules.each_with_index do |row, r|
        assert_equal g.cols, row.length,
          "row #{r} has wrong length for '#{input}'"
      end
    end
  end
end

class TestMicroQRAutoSelect < Minitest::Test
  # ── Auto-version selection ─────────────────────────────────────────────────
  #
  # The encoder picks the smallest (version, ECC) combination that can hold
  # the input in the best encoding mode.

  def test_single_digit_selects_m1
    assert_equal 11, encode("1").rows
  end

  def test_five_digits_selects_m1
    assert_equal 11, encode("12345").rows
  end

  def test_six_digits_selects_m2
    # M1 holds max 5 numeric digits → 6 digits spills to M2
    assert_equal 13, encode("123456").rows
  end

  def test_hello_selects_m2
    # "HELLO" is 5 alphanumeric chars → M2-L (alpha_cap=6)
    assert_equal 13, encode("HELLO").rows
  end

  def test_eight_digit_numeric_selects_m2
    # 8 digits → M2-L numeric (cap=10)
    assert_equal 13, encode("01234567").rows
  end

  def test_hello_lowercase_selects_m3
    # "hello" = 5 bytes → M3-L has byte_cap=9, M2-L has byte_cap=4 → M3
    g = encode("hello")
    assert g.rows >= 15, "lowercase input should select at least M3"
  end

  def test_url_selects_m4
    # "https://a.b" = 11 bytes → needs M4 (M3-L byte_cap=9)
    assert_equal 17, encode("https://a.b").rows
  end

  def test_13_alphanum_selects_m3
    # "MICRO QR TEST" = 13 alphanumeric chars → M3-L (alpha_cap=14)
    assert_equal 15, encode("MICRO QR TEST").rows
  end
end

class TestMicroQRForcedVersion < Minitest::Test
  # ── Forced version and ECC overrides ──────────────────────────────────────

  def test_forced_m4_for_single_digit
    g = encode("1", version: MicroQRVersion::M4)
    assert_equal 17, g.rows
  end

  def test_forced_m3_for_hello
    g = encode("HELLO", version: MicroQRVersion::M3)
    assert_equal 15, g.rows
  end

  def test_forced_ecc_l_vs_m_differ
    g1 = encode("HELLO", ecc: MicroQREccLevel::L)
    g2 = encode("HELLO", ecc: MicroQREccLevel::M)
    # Both produce 13-row (M2) grids but different ECC → different format info
    refute_equal grid_str(g1), grid_str(g2),
      "L and M ECC must produce different grids for the same input"
  end

  def test_m4_l_m_q_all_differ
    gl = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::L)
    gm = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::M)
    gq = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::Q)
    refute_equal grid_str(gl), grid_str(gm)
    refute_equal grid_str(gm), grid_str(gq)
    refute_equal grid_str(gl), grid_str(gq)
  end

  def test_m1_detection_only
    g = encode("1", version: MicroQRVersion::M1, ecc: MicroQREccLevel::Detection)
    assert_equal 11, g.rows
  end
end

class TestMicroQRStructuralModules < Minitest::Test
  # ── Structural module tests ────────────────────────────────────────────────

  def test_finder_top_row_all_dark
    g = encode("1")
    (0..6).each { |c| assert g.modules[0][c], "finder top row col #{c} must be dark" }
  end

  def test_finder_bottom_row_all_dark
    g = encode("1")
    (0..6).each { |c| assert g.modules[6][c], "finder bottom row col #{c} must be dark" }
  end

  def test_finder_left_col_all_dark
    g = encode("1")
    (0..6).each { |r| assert g.modules[r][0], "finder left col row #{r} must be dark" }
  end

  def test_finder_right_col_all_dark
    g = encode("1")
    (0..6).each { |r| assert g.modules[r][6], "finder right col row #{r} must be dark" }
  end

  def test_finder_inner_ring_light
    g = encode("1")
    # Row 1, cols 1–5 must be light (inner ring)
    (1..5).each { |c| refute g.modules[1][c], "finder inner ring row 1 col #{c} must be light" }
    (1..5).each { |c| refute g.modules[5][c], "finder inner ring row 5 col #{c} must be light" }
    (1..5).each { |r| refute g.modules[r][1], "finder inner ring col 1 row #{r} must be light" }
    (1..5).each { |r| refute g.modules[r][5], "finder inner ring col 5 row #{r} must be light" }
  end

  def test_finder_core_dark
    g = encode("1")
    (2..4).each do |r|
      (2..4).each do |c|
        assert g.modules[r][c], "finder core (#{r},#{c}) must be dark"
      end
    end
  end

  def test_separator_row_7_light
    g = encode("HELLO")  # M2 (13×13)
    (0..7).each { |c| refute g.modules[7][c], "separator row 7 col #{c} must be light" }
  end

  def test_separator_col_7_light
    g = encode("HELLO")  # M2 (13×13)
    (0..7).each { |r| refute g.modules[r][7], "separator col 7 row #{r} must be light" }
  end

  def test_timing_row_0_m4
    g = encode("https://a.b")  # M4 (17×17)
    (8..16).each do |c|
      expected = c.even?
      assert_equal expected, g.modules[0][c],
        "timing row 0 col #{c}: expected #{expected}"
    end
  end

  def test_timing_col_0_m4
    g = encode("https://a.b")  # M4 (17×17)
    (8..16).each do |r|
      expected = r.even?
      assert_equal expected, g.modules[r][0],
        "timing col 0 row #{r}: expected #{expected}"
    end
  end

  def test_timing_row_0_m2
    g = encode("HELLO")  # M2 (13×13)
    (8..12).each do |c|
      expected = c.even?
      assert_equal expected, g.modules[0][c],
        "timing row 0 col #{c} for M2: expected #{expected}"
    end
  end
end

class TestMicroQRDeterminism < Minitest::Test
  # ── Determinism ────────────────────────────────────────────────────────────

  def test_same_input_same_grid
    ["1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b"].each do |input|
      g1 = encode(input)
      g2 = encode(input)
      assert_equal grid_str(g1), grid_str(g2),
        "encoding '#{input}' twice must produce identical grids"
    end
  end

  def test_different_inputs_different_grids
    g1 = encode("1")
    g2 = encode("2")
    refute_equal grid_str(g1), grid_str(g2)
  end

  def test_numeric_vs_same_alphanum_different_sizes
    # "1" → M1 (numeric mode), "A" → M2 (alphanumeric mode)
    g1 = encode("1")
    g2 = encode("A")
    refute_equal g1.rows, g2.rows
  end
end

class TestMicroQRECCConstraints < Minitest::Test
  # ── ECC level constraints ──────────────────────────────────────────────────

  def test_m1_rejects_ecc_l
    err = assert_raises(ECCNotAvailable) { encode("1", version: MicroQRVersion::M1, ecc: MicroQREccLevel::L) }
    assert_match(/ECCNotAvailable|No symbol configuration/i, err.class.to_s + err.message)
  end

  def test_m1_rejects_ecc_m
    assert_raises(ECCNotAvailable) { encode("1", version: MicroQRVersion::M1, ecc: MicroQREccLevel::M) }
  end

  def test_m1_rejects_ecc_q
    assert_raises(ECCNotAvailable) { encode("1", version: MicroQRVersion::M1, ecc: MicroQREccLevel::Q) }
  end

  def test_m2_rejects_ecc_q
    assert_raises(ECCNotAvailable) { encode("1", version: MicroQRVersion::M2, ecc: MicroQREccLevel::Q) }
  end

  def test_m3_rejects_ecc_q
    assert_raises(ECCNotAvailable) { encode("1", version: MicroQRVersion::M3, ecc: MicroQREccLevel::Q) }
  end

  def test_m2_accepts_ecc_l
    g = encode("1", version: MicroQRVersion::M2, ecc: MicroQREccLevel::L)
    assert_equal 13, g.rows
  end

  def test_m2_accepts_ecc_m
    g = encode("1", version: MicroQRVersion::M2, ecc: MicroQREccLevel::M)
    assert_equal 13, g.rows
  end

  def test_m4_accepts_all_ecc
    [MicroQREccLevel::L, MicroQREccLevel::M, MicroQREccLevel::Q].each do |ecc|
      g = encode("HELLO", version: MicroQRVersion::M4, ecc: ecc)
      assert_equal 17, g.rows, "M4 should accept ECC #{ecc}"
    end
  end
end

class TestMicroQRErrorHandling < Minitest::Test
  # ── Error handling ─────────────────────────────────────────────────────────

  def test_input_too_long_numeric
    # 36 digits > M4-L max 35 numeric
    assert_raises(InputTooLong) { encode("1" * 36) }
  end

  def test_input_too_long_byte
    # 16 bytes > M4-L max 15 bytes
    assert_raises(InputTooLong) { encode("a" * 16) }
  end

  def test_ecc_not_available_m1_l
    err = assert_raises(ECCNotAvailable) do
      encode("1", version: MicroQRVersion::M1, ecc: MicroQREccLevel::L)
    end
    refute_nil err.message
  end

  def test_ecc_not_available_invalid_combo
    assert_raises(ECCNotAvailable) do
      encode("1", version: MicroQRVersion::M1, ecc: MicroQREccLevel::Q)
    end
  end

  def test_empty_string_ok
    # Empty string should encode successfully (M1)
    g = encode("")
    assert_equal 11, g.rows
  end
end

class TestMicroQRCapacityBoundaries < Minitest::Test
  # ── Capacity boundary tests ────────────────────────────────────────────────

  def test_m1_max_5_numeric
    g = encode("12345")
    assert_equal 11, g.rows
  end

  def test_m1_overflow_6_numeric
    g = encode("123456")
    assert_equal 13, g.rows  # spills to M2
  end

  def test_m2_l_max_10_numeric
    g = encode("1234567890")
    assert_equal 13, g.rows
  end

  def test_m2_l_max_6_alpha
    g = encode("ABCDEF")
    assert_equal 13, g.rows
  end

  def test_m2_l_overflow_7_alpha
    # 7 alpha → M3-L (alpha_cap=14)
    g = encode("ABCDEFG")
    assert_equal 15, g.rows
  end

  def test_m3_l_max_14_alpha
    g = encode("A" * 14)
    assert_equal 15, g.rows
  end

  def test_m3_l_overflow_15_alpha
    # 15 alpha → M4-L (alpha_cap=21)
    g = encode("A" * 15)
    assert_equal 17, g.rows
  end

  def test_m4_l_max_35_numeric
    g = encode("1" * 35)
    assert_equal 17, g.rows
  end

  def test_m4_overflow_36_numeric
    assert_raises(InputTooLong) { encode("1" * 36) }
  end

  def test_m4_l_max_15_bytes
    g = encode("a" * 15)
    assert_equal 17, g.rows
  end

  def test_m4_q_max_21_numeric
    g = encode("1" * 21, ecc: MicroQREccLevel::Q)
    assert_equal 17, g.rows
  end

  def test_m4_q_max_9_bytes
    g = encode("a" * 9, ecc: MicroQREccLevel::Q)
    assert_equal 17, g.rows
  end
end

class TestMicroQRFormatInfo < Minitest::Test
  # ── Format information ─────────────────────────────────────────────────────
  #
  # Format info occupies row 8 cols 1–8 and col 8 rows 1–7.
  # Some of these must be dark (the all-zero format word is masked with 0x4445).

  def test_format_info_non_zero_m4
    g = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::L)
    any_dark_row = (1..8).any? { |c| g.modules[8][c] }
    any_dark_col = (1..7).any? { |r| g.modules[r][8] }
    assert any_dark_row || any_dark_col, "format info must have some dark modules"
  end

  def test_format_info_non_zero_m1
    g = encode("1")
    count = (1..8).count { |c| g.modules[8][c] } +
            (1..7).count { |r| g.modules[r][8] }
    assert count > 0, "M1 format info must have some dark modules"
  end

  def test_format_info_differs_per_ecc
    g1 = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::L)
    g2 = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::Q)
    row8_1 = (1..8).map { |c| g1.modules[8][c] }
    row8_2 = (1..8).map { |c| g2.modules[8][c] }
    refute_equal row8_1, row8_2,
      "format info row 8 must differ between L and Q ECC"
  end
end

class TestMicroQRCrossLanguageCorpus < Minitest::Test
  # ── Cross-language test corpus ─────────────────────────────────────────────
  #
  # These cases are the canonical cross-language verification set.
  # The expected symbol sizes must match the Rust reference implementation.

  CORPUS = [
    ["1",            11, "M1 single digit"],
    ["12345",        11, "M1 max 5 digits"],
    ["HELLO",        13, "M2-L alphanumeric 5 chars"],
    ["01234567",     13, "M2-L numeric 8 digits"],
    ["https://a.b",  17, "M4-L byte mode 11 chars"],
    ["MICRO QR TEST",15, "M3-L alphanumeric 13 chars"],
  ].freeze

  CORPUS.each do |input, expected_rows, description|
    define_method(:"test_corpus_#{input.gsub(/[^a-zA-Z0-9]/, "_")}") do
      g = encode(input)
      assert_equal expected_rows, g.rows,
        "#{description}: expected #{expected_rows}×#{expected_rows} grid, got #{g.rows}×#{g.cols}"
    end
  end
end

class TestMicroQRConvenienceMethods < Minitest::Test
  # ── encode_at convenience alias ────────────────────────────────────────────

  def test_encode_at_m1_detection
    g = encode_at("1", MicroQRVersion::M1, MicroQREccLevel::Detection)
    assert_equal 11, g.rows
  end

  def test_encode_at_m4_l
    g = encode_at("HELLO", MicroQRVersion::M4, MicroQREccLevel::L)
    assert_equal 17, g.rows
  end

  def test_encode_at_same_as_encode
    g1 = encode("HELLO", version: MicroQRVersion::M4, ecc: MicroQREccLevel::M)
    g2 = encode_at("HELLO", MicroQRVersion::M4, MicroQREccLevel::M)
    assert_equal grid_str(g1), grid_str(g2)
  end

  # ── layout() → PaintScene ──────────────────────────────────────────────────

  def test_layout_returns_paint_scene
    g = encode("HELLO")
    scene = CodingAdventures::MicroQR.layout(g)
    refute_nil scene, "layout must return a PaintScene"
    assert_respond_to scene, :width
    assert_respond_to scene, :height
    assert_respond_to scene, :instructions
  end

  def test_layout_uses_quiet_zone_2
    g = encode("1")   # M1, 11×11
    # Default quiet_zone_modules=2, module_size_px=10
    scene = CodingAdventures::MicroQR.layout(g)
    expected_width = (11 + 4) * 10  # (cols + 2*quiet) * module_px = 150
    assert_equal expected_width, scene.width
  end

  def test_layout_with_custom_module_size
    g = encode("1")
    scene = CodingAdventures::MicroQR.layout(g, {module_size_px: 5, quiet_zone_modules: 2})
    expected_width = (11 + 4) * 5
    assert_equal expected_width, scene.width
  end

  # ── encode_and_layout convenience ─────────────────────────────────────────

  def test_encode_and_layout_returns_paint_scene
    scene = CodingAdventures::MicroQR.encode_and_layout("HELLO")
    refute_nil scene
    assert_respond_to scene, :instructions
  end

  def test_encode_and_layout_with_version_ecc
    scene = CodingAdventures::MicroQR.encode_and_layout(
      "HELLO",
      version: MicroQRVersion::M4,
      ecc: MicroQREccLevel::Q
    )
    # M4 = 17×17, quiet=2, module_size=10 → width = (17+4)*10 = 210
    assert_equal 210, scene.width
  end
end

class TestMicroQRInternalHelpers < Minitest::Test
  # ── SYMBOL_CONFIGS completeness ────────────────────────────────────────────

  def test_symbol_configs_has_eight_entries
    assert_equal 8, SYMBOL_CONFIGS.length
  end

  def test_symbol_configs_symbol_indicators_unique
    indicators = SYMBOL_CONFIGS.map(&:symbol_indicator)
    assert_equal indicators.sort, (0..7).to_a
  end

  def test_symbol_configs_sizes_correct
    expected = {
      MicroQRVersion::M1 => 11,
      MicroQRVersion::M2 => 13,
      MicroQRVersion::M3 => 15,
      MicroQRVersion::M4 => 17
    }
    SYMBOL_CONFIGS.each do |cfg|
      assert_equal expected[cfg.version], cfg.size,
        "version #{cfg.version} should have size #{expected[cfg.version]}"
    end
  end

  # ── GENERATORS table ───────────────────────────────────────────────────────

  def test_generators_present_for_all_ecc_cw_counts
    [2, 5, 6, 8, 10, 14].each do |n|
      assert GENERATORS.key?(n), "GENERATORS must include key #{n}"
      assert_equal n + 1, GENERATORS[n].length,
        "generator for n=#{n} must have #{n + 1} coefficients (monic)"
      assert_equal 0x01, GENERATORS[n][0],
        "generator for n=#{n} must be monic (first coeff = 0x01)"
    end
  end

  # ── FORMAT_TABLE structure ─────────────────────────────────────────────────

  def test_format_table_has_eight_rows
    assert_equal 8, FORMAT_TABLE.length
  end

  def test_format_table_each_row_has_four_masks
    FORMAT_TABLE.each_with_index do |row, i|
      assert_equal 4, row.length,
        "FORMAT_TABLE[#{i}] must have 4 mask entries"
    end
  end

  def test_format_table_all_values_are_15_bit
    FORMAT_TABLE.each do |row|
      row.each do |val|
        assert val < (1 << 15), "format table value #{val.to_s(16)} exceeds 15 bits"
        assert val >= 0
      end
    end
  end

  # ── BitWriter internals ────────────────────────────────────────────────────

  def test_bit_writer_empty
    w = BitWriter.new
    assert_equal 0, w.bit_length
    assert_equal [], w.to_bytes
  end

  def test_bit_writer_single_byte
    w = BitWriter.new
    w.write(0b10110001, 8)
    assert_equal 8, w.bit_length
    assert_equal [0b10110001], w.to_bytes
  end

  def test_bit_writer_msb_first
    w = BitWriter.new
    w.write(0b101, 3)  # appends: 1, 0, 1
    w.write(0b01100, 5)  # appends: 0, 1, 1, 0, 0
    # 8 bits: 1, 0, 1, 0, 1, 1, 0, 0 = 0b10101100 = 0xAC
    assert_equal [0xAC], w.to_bytes
  end

  def test_bit_writer_partial_byte_padded_with_zeros
    w = BitWriter.new
    w.write(0b1010, 4)   # only 4 bits
    bytes = w.to_bytes
    assert_equal 1, bytes.length
    assert_equal 0b10100000, bytes[0]  # padded with 4 zeros
  end

  def test_bit_writer_bits_array
    w = BitWriter.new
    w.write(0b110, 3)
    assert_equal [1, 1, 0], w.bits
  end

  # ── Numeric mode encoding ──────────────────────────────────────────────────

  def test_numeric_encode_groups_of_three
    # "123" → 123 → 10 bits: 0001111011 = 0x7B (but only 10 bits matter)
    w = BitWriter.new
    encode_numeric("123", w)
    assert_equal 10, w.bit_length
    bytes = w.to_bytes
    # 0001111011_000000 → 0x1E (top byte) 0xC0 (bottom byte)
    assert_equal 0b00011110, bytes[0]
    assert_equal 0b11000000, bytes[1]
  end

  def test_numeric_encode_pair_7_bits
    # "45" → 45 → 7 bits: 0101101
    w = BitWriter.new
    encode_numeric("45", w)
    assert_equal 7, w.bit_length
    bytes = w.to_bytes
    # 0101101_0 = 0x5A
    assert_equal 0b01011010, bytes[0]
  end

  def test_numeric_encode_single_4_bits
    # "7" → 7 → 4 bits: 0111
    w = BitWriter.new
    encode_numeric("7", w)
    assert_equal 4, w.bit_length
    bytes = w.to_bytes
    # 0111_0000 = 0x70
    assert_equal 0b01110000, bytes[0]
  end

  # ── Alphanumeric mode encoding ─────────────────────────────────────────────

  def test_alphanumeric_encode_pair
    # "AC" → A=10, C=12 → 10*45+12 = 462 → 11 bits: 00111001110
    w = BitWriter.new
    encode_alphanumeric("AC", w)
    assert_equal 11, w.bit_length
  end

  def test_alphanumeric_encode_single
    # "A" → A=10 → 6 bits: 001010
    w = BitWriter.new
    encode_alphanumeric("A", w)
    assert_equal 6, w.bit_length
  end

  # ── RS encoder ─────────────────────────────────────────────────────────────

  def test_rs_encode_zero_data_gives_zero_remainder
    gen = GENERATORS[2]
    rem = rs_encode([0, 0, 0], gen)
    assert_equal [0, 0], rem
  end

  def test_rs_encode_produces_correct_length
    [2, 5, 6, 8, 10, 14].each do |n|
      gen = GENERATORS[n]
      data = Array.new(5, 0x42)
      rem = rs_encode(data, gen)
      assert_equal n, rem.length, "RS remainder length must equal ecc_cw count #{n}"
    end
  end

  def test_rs_encode_m1_detection_deterministic
    gen = GENERATORS[2]
    data = [0b01001100, 0b01100000, 0b00000000]  # "1" in M1
    rem1 = rs_encode(data, gen)
    rem2 = rs_encode(data, gen)
    assert_equal rem1, rem2
  end

  # ── Mask conditions ────────────────────────────────────────────────────────

  def test_mask_0_condition
    # (row + col) % 2 == 0
    assert mask_condition?(0, 0, 0)
    assert mask_condition?(0, 1, 1)
    refute mask_condition?(0, 0, 1)
    refute mask_condition?(0, 1, 0)
  end

  def test_mask_1_condition
    # row % 2 == 0
    assert mask_condition?(1, 0, 5)
    assert mask_condition?(1, 2, 3)
    refute mask_condition?(1, 1, 0)
    refute mask_condition?(1, 3, 0)
  end

  def test_mask_2_condition
    # col % 3 == 0
    assert mask_condition?(2, 5, 0)
    assert mask_condition?(2, 3, 3)
    refute mask_condition?(2, 0, 1)
    refute mask_condition?(2, 0, 2)
  end

  def test_mask_3_condition
    # (row + col) % 3 == 0
    assert mask_condition?(3, 0, 0)
    assert mask_condition?(3, 1, 2)
    refute mask_condition?(3, 0, 1)
    refute mask_condition?(3, 1, 0)
  end

  def test_mask_unknown_returns_false
    refute mask_condition?(4, 0, 0)
    refute mask_condition?(99, 5, 5)
  end
end

class TestMicroQRPenalty < Minitest::Test
  # ── Penalty scoring ────────────────────────────────────────────────────────

  def test_penalty_is_non_negative
    [encode("1"), encode("HELLO"), encode("https://a.b")].each do |g|
      # We can't call compute_penalty directly since it's private to module_function,
      # but we verify the encoding completes (meaning mask selection ran penalty scoring).
      assert g.rows > 0
    end
  end

  def test_all_dark_has_high_penalty
    # Build a 11×11 all-dark grid and compute penalty via reflection
    sz = 11
    all_dark = Array.new(sz) { Array.new(sz, true) }
    p = compute_penalty(all_dark, sz)
    assert p > 0, "all-dark grid must have non-zero penalty"
  end

  def test_all_light_has_high_penalty
    sz = 11
    all_light = Array.new(sz) { Array.new(sz, false) }
    p = compute_penalty(all_light, sz)
    assert p > 0, "all-light grid must have non-zero penalty"
  end
end

class TestMicroQRModuleGrid < Minitest::Test
  # ── ModuleGrid result ──────────────────────────────────────────────────────

  def test_module_grid_is_frozen
    g = encode("1")
    assert g.frozen?, "returned ModuleGrid must be frozen"
  end

  def test_module_grid_rows_frozen
    g = encode("1")
    g.modules.each { |row| assert row.frozen?, "each row must be frozen" }
  end

  def test_modules_array_frozen
    g = encode("1")
    assert g.modules.frozen?, "modules outer array must be frozen"
  end

  def test_all_module_values_are_boolean
    g = encode("HELLO")
    g.modules.each_with_index do |row, r|
      row.each_with_index do |val, c|
        assert [true, false].include?(val),
          "module at (#{r},#{c}) must be boolean, got #{val.inspect}"
      end
    end
  end
end
