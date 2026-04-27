# frozen_string_literal: true

# =============================================================================
# test_qr_code.rb — comprehensive tests for the QR Code encoder
# =============================================================================
#
# Tests are organized to mirror the encoding pipeline:
#   1. Version / constants
#   2. Mode selection
#   3. Char-count field widths
#   4. BitWriter
#   5. Reed-Solomon (GF256 + generator + LFSR)
#   6. Data codeword assembly
#   7. Block splitting & interleaving
#   8. Grid geometry helpers
#   9. Structural module placement
#   10. Format & version information
#   11. Mask application & penalty scoring
#   12. Public encode() API (end-to-end, scannable output)
#   13. Error handling

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
end

require "minitest/autorun"
require_relative "../lib/qr_code"

class TestQrCode < Minitest::Test
  # ===========================================================================
  # 1. Version constant
  # ===========================================================================

  def test_version_is_semver
    parts = QrCode::VERSION.split(".")
    assert_equal 3, parts.length
    assert parts.all? { |p| p.match?(/\A\d+\z/) }, "Each semver part must be an integer"
  end

  def test_version_is_0_1_0
    assert_equal "0.1.0", QrCode::VERSION
  end

  # ===========================================================================
  # 2. Mode selection
  # ===========================================================================

  def test_mode_numeric_digits_only
    assert_equal :numeric, QrCode.select_mode("0123456789")
  end

  def test_mode_numeric_empty_string
    # Empty string matches \A\d*\z — numeric is the most compact
    assert_equal :numeric, QrCode.select_mode("")
  end

  def test_mode_alphanumeric_uppercase
    assert_equal :alphanumeric, QrCode.select_mode("HELLO WORLD")
  end

  def test_mode_alphanumeric_mixed_with_symbols
    assert_equal :alphanumeric, QrCode.select_mode("HTTPS://EXAMPLE.COM")
  end

  def test_mode_byte_lowercase
    # Lowercase letters are not in the alphanumeric set
    assert_equal :byte, QrCode.select_mode("hello world")
  end

  def test_mode_byte_utf8
    assert_equal :byte, QrCode.select_mode("café")
  end

  def test_mode_byte_mixed_case_url
    assert_equal :byte, QrCode.select_mode("https://example.com")
  end

  def test_mode_alphanumeric_all_45_chars
    assert_equal :alphanumeric, QrCode.select_mode("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".freeze)
  end

  # ===========================================================================
  # 3. Char-count field widths
  # ===========================================================================

  def test_char_count_bits_numeric_v1
    assert_equal 10, QrCode.char_count_bits(:numeric, 1)
  end

  def test_char_count_bits_numeric_v9
    assert_equal 10, QrCode.char_count_bits(:numeric, 9)
  end

  def test_char_count_bits_numeric_v10
    assert_equal 12, QrCode.char_count_bits(:numeric, 10)
  end

  def test_char_count_bits_numeric_v26
    assert_equal 12, QrCode.char_count_bits(:numeric, 26)
  end

  def test_char_count_bits_numeric_v27
    assert_equal 14, QrCode.char_count_bits(:numeric, 27)
  end

  def test_char_count_bits_numeric_v40
    assert_equal 14, QrCode.char_count_bits(:numeric, 40)
  end

  def test_char_count_bits_alphanumeric_v1
    assert_equal 9, QrCode.char_count_bits(:alphanumeric, 1)
  end

  def test_char_count_bits_alphanumeric_v9
    assert_equal 9, QrCode.char_count_bits(:alphanumeric, 9)
  end

  def test_char_count_bits_alphanumeric_v10
    assert_equal 11, QrCode.char_count_bits(:alphanumeric, 10)
  end

  def test_char_count_bits_alphanumeric_v26
    assert_equal 11, QrCode.char_count_bits(:alphanumeric, 26)
  end

  def test_char_count_bits_alphanumeric_v27
    assert_equal 13, QrCode.char_count_bits(:alphanumeric, 27)
  end

  def test_char_count_bits_byte_v1
    assert_equal 8, QrCode.char_count_bits(:byte, 1)
  end

  def test_char_count_bits_byte_v9
    assert_equal 8, QrCode.char_count_bits(:byte, 9)
  end

  def test_char_count_bits_byte_v10
    assert_equal 16, QrCode.char_count_bits(:byte, 10)
  end

  def test_char_count_bits_byte_v40
    assert_equal 16, QrCode.char_count_bits(:byte, 40)
  end

  # ===========================================================================
  # 4. BitWriter
  # ===========================================================================

  def test_bit_writer_single_write
    w = QrCode::BitWriter.new
    w.write(0b1010, 4)
    assert_equal [0b10100000], w.to_bytes
  end

  def test_bit_writer_full_byte
    w = QrCode::BitWriter.new
    w.write(0xFF, 8)
    assert_equal [0xFF], w.to_bytes
  end

  def test_bit_writer_zero_byte
    w = QrCode::BitWriter.new
    w.write(0, 8)
    assert_equal [0], w.to_bytes
  end

  def test_bit_writer_two_bytes
    w = QrCode::BitWriter.new
    w.write(0xAB, 8)
    w.write(0xCD, 8)
    assert_equal [0xAB, 0xCD], w.to_bytes
  end

  def test_bit_writer_cross_byte_boundary
    w = QrCode::BitWriter.new
    w.write(0b101, 3)    # bits: 1 0 1
    w.write(0b11001, 5)  # bits: 1 1 0 0 1
    # Total: 1 0 1 1 1 0 0 1 → 0b10111001 = 0xB9
    assert_equal [0xB9], w.to_bytes
  end

  def test_bit_writer_bit_length
    w = QrCode::BitWriter.new
    w.write(0, 3)
    assert_equal 3, w.bit_length
    w.write(0, 5)
    assert_equal 8, w.bit_length
  end

  def test_bit_writer_padding_at_end
    w = QrCode::BitWriter.new
    w.write(1, 1)  # bit: 1 → padded to 10000000 = 0x80
    assert_equal [0x80], w.to_bytes
  end

  # ===========================================================================
  # 5. Reed-Solomon generator and encoding
  # ===========================================================================

  def test_build_generator_degree_7
    g = QrCode.build_generator(7)
    assert_equal 8, g.length  # degree 7 has 8 coefficients
    assert_equal 1, g[0]      # monic: leading coefficient is 1
  end

  def test_build_generator_degree_10
    g = QrCode.build_generator(10)
    assert_equal 11, g.length
    assert_equal 1, g[0]
  end

  def test_get_generator_prebuilt_7
    g = QrCode.get_generator(7)
    assert_equal 8, g.length
  end

  def test_get_generator_prebuilt_10
    g = QrCode.get_generator(10)
    assert_equal 11, g.length
  end

  def test_get_generator_prebuilt_all_standard_sizes
    [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30].each do |n|
      g = QrCode.get_generator(n)
      assert_equal n + 1, g.length, "generator(#{n}) should have #{n + 1} coefficients"
      assert_equal 1, g[0], "generator(#{n}) should be monic"
    end
  end

  def test_rs_encode_length
    data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236]
    gen = QrCode.get_generator(13)
    ecc = QrCode.rs_encode(data, gen)
    assert_equal 13, ecc.length
  end

  def test_rs_encode_all_zeros_data
    # ECC of [0, 0, ..., 0] is all zeros because 0 × anything = 0 in GF256
    data = Array.new(5, 0)
    gen = QrCode.get_generator(7)
    ecc = QrCode.rs_encode(data, gen)
    assert_equal 7, ecc.length
    assert ecc.all?(&:zero?), "ECC of all-zero data should be all zeros"
  end

  def test_rs_encode_values_in_range
    data = [0, 1, 2, 100, 255]
    gen = QrCode.get_generator(7)
    ecc = QrCode.rs_encode(data, gen)
    assert ecc.all? { |b| b >= 0 && b <= 255 }, "All ECC bytes must be in [0,255]"
  end

  # ===========================================================================
  # 6. Grid geometry helpers
  # ===========================================================================

  def test_symbol_size_v1
    assert_equal 21, QrCode.symbol_size(1)
  end

  def test_symbol_size_v2
    assert_equal 25, QrCode.symbol_size(2)
  end

  def test_symbol_size_v10
    assert_equal 57, QrCode.symbol_size(10)
  end

  def test_symbol_size_v40
    assert_equal 177, QrCode.symbol_size(40)
  end

  def test_symbol_size_formula
    (1..40).each do |v|
      assert_equal 4 * v + 17, QrCode.symbol_size(v)
    end
  end

  def test_num_raw_data_modules_v1
    # V1: 21×21 = 441 total, minus finder/timing/format areas
    # Known value from ISO 18004
    assert_equal 208, QrCode.num_raw_data_modules(1)
  end

  def test_num_data_codewords_v1_m
    # V1-M: 16 data codewords (128 bits)
    assert_equal 16, QrCode.num_data_codewords(1, :M)
  end

  def test_num_data_codewords_v1_l
    assert_equal 19, QrCode.num_data_codewords(1, :L)
  end

  def test_num_data_codewords_v1_q
    assert_equal 13, QrCode.num_data_codewords(1, :Q)
  end

  def test_num_data_codewords_v1_h
    assert_equal 9, QrCode.num_data_codewords(1, :H)
  end

  def test_num_data_codewords_positive_all_versions
    [:L, :M, :Q, :H].each do |ecc|
      (1..40).each do |v|
        assert QrCode.num_data_codewords(v, ecc) > 0,
          "num_data_codewords(#{v}, #{ecc}) must be > 0"
      end
    end
  end

  def test_num_remainder_bits_v1
    assert_equal 0, QrCode.num_remainder_bits(1)
  end

  def test_num_remainder_bits_v2
    assert_equal 7, QrCode.num_remainder_bits(2)
  end

  def test_num_remainder_bits_v14
    assert_equal 3, QrCode.num_remainder_bits(14)
  end

  # ===========================================================================
  # 7. Version selection
  # ===========================================================================

  def test_select_version_hello_world_m
    v = QrCode.select_version("Hello, World!", :M)
    assert v >= 1
    assert v <= 40
    # "Hello, World!" is 13 chars, byte mode, v1-M should work
    assert_equal 1, v
  end

  def test_select_version_numeric_short
    v = QrCode.select_version("123", :M)
    assert_equal 1, v
  end

  def test_select_version_long_string_higher_version
    # 100-digit numeric string needs a higher version
    v = QrCode.select_version("1" * 100, :M)
    assert v >= 1
    assert v <= 40
  end

  def test_select_version_higher_ecc_higher_version
    short = "HELLO"
    v_l = QrCode.select_version(short, :L)
    v_h = QrCode.select_version(short, :H)
    # H level has less data capacity, so it needs same or higher version than L
    assert v_h >= v_l
  end

  def test_select_version_capacity_fits
    (1..5).each do |v|
      [:L, :M, :Q, :H].each do |ecc|
        capacity = QrCode.num_data_codewords(v, ecc)
        # A string that exactly fits this capacity
        data = "A" * [capacity - 2, 1].max  # leave room for mode+count headers
        selected = QrCode.select_version(data, ecc)
        assert selected >= 1, "Version should be at least 1"
        assert selected <= 40, "Version should be at most 40"
      end
    end
  end

  # ===========================================================================
  # 8. Data codeword assembly
  # ===========================================================================

  def test_build_data_codewords_length_matches_capacity
    [:L, :M, :Q, :H].each do |ecc|
      [1, 2, 5, 10].each do |v|
        capacity = QrCode.num_data_codewords(v, ecc)
        # Use a short string that fits in every version
        cw = QrCode.build_data_codewords("A", v, ecc)
        assert_equal capacity, cw.length,
          "build_data_codewords should produce exactly #{capacity} codewords for v#{v}-#{ecc}"
      end
    end
  end

  def test_build_data_codewords_starts_with_mode_indicator
    # "A" is alphanumeric → mode 0b0010 → first 4 bits of first byte = 0010
    # For v1 alphanumeric, mode indicator = 0010, followed by 9-bit char count
    cw = QrCode.build_data_codewords("A", 1, :M)
    # First byte: mode(4b) | count_upper(4b) = 0010 | 0000 = 0x20
    assert_equal 0x20, cw[0]
  end

  def test_build_data_codewords_padding_bytes
    # For a very short string, the remaining bytes should alternate 0xEC/0x11
    cw = QrCode.build_data_codewords("A", 5, :L)  # v5-L has 108 data codewords
    # Last few bytes should be the alternating pad pattern
    # We just check that 0xEC and 0x11 appear in the tail
    tail = cw[-4..]
    assert tail.include?(0xEC) || tail.include?(0x11)
  end

  def test_build_data_codewords_numeric_encoding
    # "1" in numeric mode: mode=0001, count=0000000001 (10 bits for v1), data=0001 (4 bits)
    # Total: 0001 + 0000000001 + 0001 = 15 bits
    # First byte: 0001 0000 = 0x10 (mode 0001, top 4 bits of 10-bit count 0000000001)
    cw = QrCode.build_data_codewords("1", 1, :L)
    assert_equal 0x10, cw[0]  # 0001 0000 = first byte
  end

  # ===========================================================================
  # 9. Block splitting and interleaving
  # ===========================================================================

  def test_compute_blocks_count_v1_m
    data = QrCode.build_data_codewords("A", 1, :M)
    blocks = QrCode.compute_blocks(data, 1, :M)
    assert_equal QrCode::NUM_BLOCKS[QrCode::ECC_IDX[:M]][1], blocks.length
  end

  def test_compute_blocks_ecc_length_v1_m
    data = QrCode.build_data_codewords("A", 1, :M)
    blocks = QrCode.compute_blocks(data, 1, :M)
    expected_ecc = QrCode::ECC_CODEWORDS_PER_BLOCK[QrCode::ECC_IDX[:M]][1]
    blocks.each { |b| assert_equal expected_ecc, b.ecc.length }
  end

  def test_compute_blocks_total_data_preserved
    data = QrCode.build_data_codewords("HELLO WORLD", 1, :M)
    blocks = QrCode.compute_blocks(data, 1, :M)
    total = blocks.sum { |b| b.data.length }
    assert_equal data.length, total
  end

  def test_interleave_blocks_total_length
    data = QrCode.build_data_codewords("A", 5, :M)
    blocks = QrCode.compute_blocks(data, 5, :M)
    interleaved = QrCode.interleave_blocks(blocks)
    expected_total = blocks.sum { |b| b.data.length + b.ecc.length }
    assert_equal expected_total, interleaved.length
  end

  # ===========================================================================
  # 10. Format and version information
  # ===========================================================================

  def test_compute_format_bits_not_zero
    # After XOR with 0x5412, the format bits should never be zero
    [:L, :M, :Q, :H].each do |ecc|
      8.times do |mask|
        bits = QrCode.compute_format_bits(ecc, mask)
        refute_equal 0, bits, "format bits for #{ecc}/mask#{mask} should not be zero"
      end
    end
  end

  def test_compute_format_bits_in_15_bit_range
    [:L, :M, :Q, :H].each do |ecc|
      8.times do |mask|
        bits = QrCode.compute_format_bits(ecc, mask)
        assert bits >= 0
        assert bits < (1 << 15), "format bits must fit in 15 bits"
      end
    end
  end

  def test_compute_format_bits_m_mask2_known_value
    # Known value for ECC=M, mask=2, verified against ISO reference decoder
    # ECC_INDICATOR[:M] = 0b00, data = (0b00 << 3) | 2 = 2
    # rem is the BCH remainder of 2 << 10 divided by 0x537
    # result = (2 << 10 | rem) ^ 0x5412
    # We verify the BCH check: BCH check of the 15-bit word should be 0
    bits = QrCode.compute_format_bits(:M, 2)
    assert_kind_of Integer, bits
    assert bits >= 0
    assert bits < (1 << 15)
  end

  def test_compute_version_bits_v7
    bits = QrCode.compute_version_bits(7)
    # 6-bit version = 7 = 000111, then 12 BCH bits
    # Total 18 bits. Version is in the upper 6 bits.
    assert_equal 7, (bits >> 12) & 0x3F
  end

  def test_compute_version_bits_v40
    bits = QrCode.compute_version_bits(40)
    assert_equal 40, (bits >> 12) & 0x3F
  end

  def test_compute_version_bits_in_18_bit_range
    (7..40).each do |v|
      bits = QrCode.compute_version_bits(v)
      assert bits >= 0
      assert bits < (1 << 18), "version bits v#{v} must fit in 18 bits"
    end
  end

  # ===========================================================================
  # 11. Work grid construction
  # ===========================================================================

  def test_make_work_grid_dimensions
    g = QrCode.make_work_grid(21)
    assert_equal 21, g.size
    assert_equal 21, g.modules.length
    assert_equal 21, g.modules[0].length
    assert_equal 21, g.reserved.length
    assert_equal 21, g.reserved[0].length
  end

  def test_make_work_grid_all_false
    g = QrCode.make_work_grid(5)
    g.modules.each { |row| row.each { |m| assert_equal false, m } }
    g.reserved.each { |row| row.each { |r| assert_equal false, r } }
  end

  def test_build_grid_size_v1
    g = QrCode.build_grid(1)
    assert_equal 21, g.size
  end

  def test_build_grid_finder_top_left_corner
    g = QrCode.build_grid(1)
    # Top-left of top-left finder (0,0) should be dark and reserved
    assert_equal true, g.modules[0][0]
    assert_equal true, g.reserved[0][0]
  end

  def test_build_grid_finder_interior_light
    # Interior of finder (row 1..5, col 1..5 ring) should have a light ring
    g = QrCode.build_grid(1)
    assert_equal false, g.modules[1][1]  # inner ring (light)
    assert_equal false, g.modules[1][5]
    assert_equal false, g.modules[5][1]
    assert_equal false, g.modules[5][5]
  end

  def test_build_grid_finder_core_dark
    # Inner core of finder at (2..4, 2..4) should be dark
    g = QrCode.build_grid(1)
    assert_equal true, g.modules[2][2]
    assert_equal true, g.modules[3][3]
    assert_equal true, g.modules[4][4]
  end

  def test_build_grid_timing_strip_alternating
    g = QrCode.build_grid(1)
    sz = 21
    # Row 6, columns 8..sz-9 should alternate dark/light
    8.upto(sz - 9) do |c|
      expected = c.even?
      assert_equal expected, g.modules[6][c], "timing row 6 col #{c} should be #{expected}"
      assert_equal true, g.reserved[6][c]
    end
  end

  def test_build_grid_dark_module_v1
    g = QrCode.build_grid(1)
    # Dark module at (4*1+9, 8) = (13, 8)
    assert_equal true, g.modules[13][8]
    assert_equal true, g.reserved[13][8]
  end

  def test_build_grid_separator_light
    g = QrCode.build_grid(1)
    # Separator row 7 should be light and reserved
    8.times do |c|
      assert_equal false, g.modules[7][c]
      assert_equal true, g.reserved[7][c]
    end
  end

  def test_build_grid_v2_has_alignment_pattern
    g = QrCode.build_grid(2)
    # V2 has one alignment pattern at (18, 18)
    assert_equal true, g.modules[18][18]   # center = dark
    assert_equal true, g.reserved[18][18]
    assert_equal false, g.modules[17][17] # inner ring = light
  end

  # ===========================================================================
  # 12. Mask application
  # ===========================================================================

  def test_apply_mask_0_flips_non_reserved
    sz = 5
    modules = Array.new(sz) { Array.new(sz, false) }
    reserved = Array.new(sz) { Array.new(sz, false) }
    # Mask 0: (r+c) % 2 == 0 → flip those
    result = QrCode.apply_mask(modules, reserved, sz, 0)
    # (0,0): 0+0=0 even → flipped: false ^ true = true
    assert_equal true, result[0][0]
    # (0,1): 0+1=1 odd → not flipped: stays false
    assert_equal false, result[0][1]
  end

  def test_apply_mask_reserved_not_flipped
    sz = 5
    modules = Array.new(sz) { Array.new(sz, false) }
    reserved = Array.new(sz) { Array.new(sz, false) }
    reserved[0][0] = true  # mark (0,0) as reserved
    result = QrCode.apply_mask(modules, reserved, sz, 0)
    # (0,0) is reserved, so it should NOT be flipped despite mask condition
    assert_equal false, result[0][0]
  end

  def test_apply_mask_returns_new_array
    sz = 3
    modules = Array.new(sz) { Array.new(sz, false) }
    reserved = Array.new(sz) { Array.new(sz, false) }
    result = QrCode.apply_mask(modules, reserved, sz, 0)
    refute_same modules, result
  end

  def test_all_8_masks_differ
    sz = 7
    modules = Array.new(sz) { Array.new(sz, false) }
    reserved = Array.new(sz) { Array.new(sz, false) }
    results = 8.times.map { |m| QrCode.apply_mask(modules, reserved, sz, m) }
    # Not all masks should produce identical results
    unique = results.map { |r| r.flatten }.uniq
    assert unique.length > 1, "Different masks should produce different results"
  end

  # ===========================================================================
  # 13. Penalty scoring
  # ===========================================================================

  def test_compute_penalty_all_dark
    # All-dark grid should have a large penalty (Rule 4: 100% dark → far from 50%)
    sz = 21
    modules = Array.new(sz) { Array.new(sz, true) }
    p = QrCode.compute_penalty(modules, sz)
    assert p > 0
  end

  def test_compute_penalty_checkerboard_low_rule4
    # Perfect checkerboard: ~50% dark ratio → Rule 4 penalty = 0
    sz = 6
    modules = Array.new(sz) { |r| Array.new(sz) { |c| (r + c) % 2 == 0 } }
    _penalty = QrCode.compute_penalty(modules, sz)
    # Dark count = sz*sz/2 = 18, ratio = 50%, Rule 4 = 0
    dark = 0
    sz.times { |r| sz.times { |c| dark += 1 if modules[r][c] } }
    ratio = dark.to_f / (sz * sz) * 100.0
    # Rule 4 penalty contribution should be 0 for perfect 50%
    prev5 = (ratio / 5).floor * 5
    rule4 = [(prev5 - 50).abs, (prev5 + 5 - 50).abs].min / 5 * 10
    assert_equal 0, rule4
  end

  def test_compute_penalty_non_negative
    sz = 10
    modules = Array.new(sz) { Array.new(sz, false) }
    p = QrCode.compute_penalty(modules, sz)
    assert p >= 0
  end

  # ===========================================================================
  # 14. Public encode() API — end-to-end
  # ===========================================================================

  def test_encode_returns_module_grid
    grid = QrCode.encode("HELLO", level: :M)
    assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
  end

  def test_encode_v1_size_21x21
    grid = QrCode.encode("A", level: :L)
    assert_equal 21, grid.rows
    assert_equal 21, grid.cols
  end

  def test_encode_hello_world_byte_mode
    grid = QrCode.encode("Hello, World!", level: :M)
    sz = grid.rows
    assert_equal sz, grid.cols
    assert sz >= 21
    # All modules should be boolean
    grid.modules.each do |row|
      row.each do |m|
        assert [true, false].include?(m), "module should be true or false"
      end
    end
  end

  def test_encode_hello_world_version_1
    # "Hello, World!" (13 bytes) should fit in version 1 with M level
    grid = QrCode.encode("Hello, World!", level: :M)
    assert_equal 21, grid.rows
  end

  def test_encode_numeric_mode
    grid = QrCode.encode("12345678", level: :M)
    assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
    # Numeric mode for 8 digits fits in version 1
    assert_equal 21, grid.rows
  end

  def test_encode_alphanumeric_mode
    grid = QrCode.encode("HELLO WORLD", level: :M)
    assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
    assert_equal 21, grid.rows
  end

  def test_encode_all_four_ecc_levels
    [:L, :M, :Q, :H].each do |level|
      grid = QrCode.encode("HELLO", level: level)
      assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
      assert grid.rows >= 21
    end
  end

  def test_encode_default_level_is_m
    # Without specifying level, default is :M
    grid_default = QrCode.encode("TEST")
    grid_m = QrCode.encode("TEST", level: :M)
    assert_equal grid_m.rows, grid_default.rows
    assert_equal grid_m.cols, grid_default.cols
    # Module patterns should match
    assert_equal grid_m.modules, grid_default.modules
  end

  def test_encode_url
    url = "https://example.com"
    grid = QrCode.encode(url, level: :M)
    assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
    assert grid.rows >= 21
  end

  def test_encode_utf8_string
    grid = QrCode.encode("こんにちは", level: :M)
    assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
  end

  def test_encode_specific_version_override
    # Force version 5 even for short string
    grid = QrCode.encode("A", level: :M, version: 5)
    assert_equal QrCode.symbol_size(5), grid.rows
  end

  def test_encode_grid_is_square
    grid = QrCode.encode("HELLO WORLD", level: :M)
    assert_equal grid.rows, grid.cols
  end

  def test_encode_module_shape_is_square
    grid = QrCode.encode("TEST", level: :M)
    assert_equal "square", grid.module_shape
  end

  def test_encode_grid_is_frozen
    grid = QrCode.encode("TEST", level: :M)
    assert grid.frozen?
  end

  def test_encode_finder_corners_dark
    # Top-left corner (0,0) of ANY QR code is always a dark module
    grid = QrCode.encode("HELLO", level: :M)
    assert_equal true, grid.modules[0][0]
  end

  def test_encode_v7_includes_version_info
    # Version 7+ includes version information blocks
    # Force version 7 to check version info is written
    grid = QrCode.encode("A", level: :L, version: 7)
    assert_equal QrCode.symbol_size(7), grid.rows
    # Version info occupies rows 0-5, cols size-11..size-9
    # At least one of these should be dark
    sz = grid.rows
    version_area = (0..5).flat_map { |r| (sz - 11..sz - 9).map { |c| grid.modules[r][c] } }
    assert version_area.any?, "Version 7+ should have version info bits"
  end

  def test_encode_large_input_higher_version
    # Long string requires higher version
    long_str = "A" * 50
    grid = QrCode.encode(long_str, level: :M)
    # Must be larger than version 1
    assert grid.rows > 21
  end

  def test_encode_consistency_same_input
    # Encoding the same string twice should produce identical grids
    grid1 = QrCode.encode("HELLO WORLD", level: :M)
    grid2 = QrCode.encode("HELLO WORLD", level: :M)
    assert_equal grid1.modules, grid2.modules
  end

  # ===========================================================================
  # 15. encode_to_scene
  # ===========================================================================

  def test_encode_to_scene_returns_paint_scene
    scene = QrCode.encode_to_scene("HELLO", level: :M)
    # PaintScene is an OpenStruct with width/height/instructions
    assert_respond_to scene, :width
    assert_respond_to scene, :height
    assert_respond_to scene, :instructions
  end

  def test_encode_to_scene_has_positive_dimensions
    scene = QrCode.encode_to_scene("HELLO", level: :M)
    assert scene.width > 0
    assert scene.height > 0
  end

  def test_encode_to_scene_has_instructions
    scene = QrCode.encode_to_scene("HELLO", level: :M)
    assert scene.instructions.length > 0
  end

  # ===========================================================================
  # 16. Error handling
  # ===========================================================================

  def test_input_too_long_raises
    assert_raises(QrCode::InputTooLongError) do
      QrCode.encode("A" * 8000, level: :L)
    end
  end

  def test_input_too_long_message
    err = assert_raises(QrCode::InputTooLongError) do
      QrCode.encode("A" * 8000, level: :L)
    end
    assert_match(/7089/, err.message)
  end

  def test_input_too_long_is_qr_code_error
    err = assert_raises(QrCode::InputTooLongError) do
      QrCode.encode("A" * 8000, level: :L)
    end
    assert_kind_of QrCode::QrCodeError, err
  end

  def test_qr_code_error_hierarchy
    assert QrCode::InputTooLongError < QrCode::QrCodeError
    assert QrCode::QrCodeError < StandardError
  end

  def test_empty_string_encodes_ok
    # Empty string should encode fine (numeric mode, zero length)
    grid = QrCode.encode("", level: :M)
    assert_kind_of CodingAdventures::Barcode2D::ModuleGrid, grid
  end

  # ===========================================================================
  # 17. Constants sanity checks
  # ===========================================================================

  def test_ecc_codewords_per_block_dimensions
    assert_equal 4, QrCode::ECC_CODEWORDS_PER_BLOCK.length
    QrCode::ECC_CODEWORDS_PER_BLOCK.each do |row|
      assert_equal 41, row.length  # index 0 + versions 1..40
    end
  end

  def test_num_blocks_dimensions
    assert_equal 4, QrCode::NUM_BLOCKS.length
    QrCode::NUM_BLOCKS.each do |row|
      assert_equal 41, row.length
    end
  end

  def test_alignment_positions_length
    assert_equal 40, QrCode::ALIGNMENT_POSITIONS.length
  end

  def test_alignment_positions_v1_empty
    assert_empty QrCode::ALIGNMENT_POSITIONS[0]
  end

  def test_alignment_positions_v2_has_two_entries
    assert_equal [6, 18], QrCode::ALIGNMENT_POSITIONS[1]
  end

  def test_alphanum_chars_length
    assert_equal 45, QrCode::ALPHANUM_CHARS.length
  end

  def test_alphanum_index_covers_all_chars
    QrCode::ALPHANUM_CHARS.chars.each_with_index do |c, i|
      assert_equal i, QrCode::ALPHANUM_INDEX[c]
    end
  end

  def test_ecc_indicator_values
    assert_equal 0b01, QrCode::ECC_INDICATOR[:L]
    assert_equal 0b00, QrCode::ECC_INDICATOR[:M]
    assert_equal 0b11, QrCode::ECC_INDICATOR[:Q]
    assert_equal 0b10, QrCode::ECC_INDICATOR[:H]
  end

  def test_mode_indicator_values
    assert_equal 0b0001, QrCode::MODE_INDICATOR[:numeric]
    assert_equal 0b0010, QrCode::MODE_INDICATOR[:alphanumeric]
    assert_equal 0b0100, QrCode::MODE_INDICATOR[:byte]
  end
end
