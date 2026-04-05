# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/reed_solomon"

class TestReedSolomon < Minitest::Test
  # ===========================================================================
  # Version
  # ===========================================================================

  def test_version_is_semver
    parts = ReedSolomon::VERSION.split(".")
    assert_equal 3, parts.length
    assert parts.all? { |p| p.match?(/^\d+$/) }
  end

  # ===========================================================================
  # BuildGenerator
  # ===========================================================================

  def test_build_generator_degree
    [2, 4, 8, 16].each do |n_check|
      g = ReedSolomon.build_generator(n_check)
      assert_equal n_check + 1, g.length,
        "build_generator(#{n_check}) should have length #{n_check + 1}"
    end
  end

  def test_build_generator_monic
    [2, 4, 8, 16].each do |n_check|
      g = ReedSolomon.build_generator(n_check)
      assert_equal 1, g.last,
        "build_generator(#{n_check}) should be monic (last coefficient = 1)"
    end
  end

  def test_build_generator_known_n_check_2
    # g(x) = (x+2)(x+4) = x² + 6x + 8  →  LE: [8, 6, 1]
    assert_equal [8, 6, 1], ReedSolomon.build_generator(2)
  end

  def test_build_generator_alpha_roots
    # Every α^i for i=1..n_check must be a root of g(x).
    [2, 4, 8].each do |n_check|
      g = ReedSolomon.build_generator(n_check)
      1.upto(n_check) do |i|
        alpha_i = GF256.power(2, i)
        # Evaluate g at alpha_i using Horner (LE polynomial)
        acc = 0
        g.reverse_each { |c| acc = GF256.add(GF256.multiply(acc, alpha_i), c) }
        assert_equal 0, acc,
          "g(α^#{i}) = #{acc} ≠ 0 for n_check=#{n_check}"
      end
    end
  end

  def test_build_generator_raises_zero
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.build_generator(0) }
  end

  def test_build_generator_raises_odd
    [1, 3, 5, 7].each do |odd|
      assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.build_generator(odd) }
    end
  end

  # ===========================================================================
  # Encode
  # ===========================================================================

  def test_encode_output_length
    msg = (1..10).to_a
    [2, 4, 8].each do |n_check|
      cw = ReedSolomon.encode(msg, n_check)
      assert_equal msg.length + n_check, cw.length
    end
  end

  def test_encode_message_preserved
    msg = [10, 20, 30, 40, 50]
    [2, 4, 8].each do |n_check|
      cw = ReedSolomon.encode(msg, n_check)
      assert_equal msg, cw[0, msg.length]
    end
  end

  def test_encode_zero_message_gives_zero_codeword
    msg = Array.new(5, 0)
    cw = ReedSolomon.encode(msg, 4)
    assert cw.all?(&:zero?), "all-zero message should produce all-zero codeword"
  end

  def test_encode_empty_message
    cw = ReedSolomon.encode([], 4)
    assert_equal 4, cw.length
  end

  def test_encode_single_byte
    msg = [0xAB]
    cw = ReedSolomon.encode(msg, 4)
    assert_equal 5, cw.length
    assert_equal 0xAB, cw[0]
  end

  def test_encode_max_valid_length
    # 247 + 8 = 255 — exactly at the GF(256) block size limit
    msg = Array.new(247, 0)
    cw = ReedSolomon.encode(msg, 8)
    assert_equal 255, cw.length
  end

  def test_encode_raises_odd_n_check
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.encode([1, 2, 3], 3) }
  end

  def test_encode_raises_zero_n_check
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.encode([1, 2, 3], 0) }
  end

  def test_encode_raises_exceeds_max_length
    msg = Array.new(248, 0)  # 248 + 8 = 256 > 255
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.encode(msg, 8) }
  end

  # ===========================================================================
  # Syndromes
  # ===========================================================================

  def test_syndromes_zero_on_valid_codeword
    [2, 4, 8].each do |n_check|
      msg = (1..10).to_a
      cw = ReedSolomon.encode(msg, n_check)
      s = ReedSolomon.syndromes(cw, n_check)
      assert_equal n_check, s.length
      assert s.all?(&:zero?),
        "n_check=#{n_check}: non-zero syndrome #{s} on valid codeword"
    end
  end

  def test_syndromes_nonzero_after_corruption
    msg = "hello world".bytes
    cw = ReedSolomon.encode(msg, 8)
    corrupted = cw.dup
    corrupted[0] ^= 0xFF
    s = ReedSolomon.syndromes(corrupted, 8)
    refute s.all?(&:zero?), "Expected non-zero syndrome after corruption"
  end

  def test_syndromes_empty_message_codeword
    cw = ReedSolomon.encode([], 4)
    s = ReedSolomon.syndromes(cw, 4)
    assert s.all?(&:zero?)
  end

  # ===========================================================================
  # Round-Trip
  # ===========================================================================

  def test_round_trip_ascii
    msg = "Hello, World!".bytes
    [2, 4, 8].each do |n_check|
      cw = ReedSolomon.encode(msg, n_check)
      recovered = ReedSolomon.decode(cw, n_check)
      assert_equal msg, recovered, "n_check=#{n_check}: round-trip failed"
    end
  end

  def test_round_trip_all_zero
    msg = Array.new(20, 0)
    cw = ReedSolomon.encode(msg, 4)
    assert_equal msg, ReedSolomon.decode(cw, 4)
  end

  def test_round_trip_all_ff
    msg = Array.new(20, 0xFF)
    cw = ReedSolomon.encode(msg, 4)
    assert_equal msg, ReedSolomon.decode(cw, 4)
  end

  def test_round_trip_empty
    cw = ReedSolomon.encode([], 4)
    assert_equal [], ReedSolomon.decode(cw, 4)
  end

  def test_round_trip_single_byte
    [0x00, 0x01, 0xAB, 0xFF].each do |b|
      msg = [b]
      cw = ReedSolomon.encode(msg, 4)
      assert_equal msg, ReedSolomon.decode(cw, 4), "byte 0x#{b.to_s(16)}: round-trip failed"
    end
  end

  def test_round_trip_max_length
    msg = Array.new(247) { |i| i % 256 }
    cw = ReedSolomon.encode(msg, 8)
    assert_equal msg, ReedSolomon.decode(cw, 8)
  end

  # ===========================================================================
  # Error Correction
  # ===========================================================================

  def corrupt(cw, positions, magnitudes)
    result = cw.dup
    positions.zip(magnitudes) { |pos, mag| result[pos] ^= mag }
    result
  end

  def test_single_error_every_position
    # n_check=2 → t=1: correct every single corrupted byte
    msg = (1..10).to_a
    cw = ReedSolomon.encode(msg, 2)
    cw.length.times do |pos|
      corrupted = corrupt(cw, [pos], [0x5A])
      recovered = ReedSolomon.decode(corrupted, 2)
      assert_equal msg, recovered, "pos=#{pos}: single-error correction failed"
    end
  end

  def test_two_errors
    msg = (1..10).to_a
    cw = ReedSolomon.encode(msg, 4)
    n = cw.length
    (0...n).step(3) do |pos1|
      (pos1 + 1...n).step(4) do |pos2|
        corrupted = corrupt(cw, [pos1, pos2], [0xDE, 0xAD])
        recovered = ReedSolomon.decode(corrupted, 4)
        assert_equal msg, recovered, "pos1=#{pos1},pos2=#{pos2}: failed"
      end
    end
  end

  def test_four_errors
    msg = "Reed-Solomon".bytes
    cw = ReedSolomon.encode(msg, 8)
    corrupted = corrupt(cw, [0, 3, 7, 10], [0xFF, 0xAA, 0x55, 0x0F])
    assert_equal msg, ReedSolomon.decode(corrupted, 8)
  end

  def test_every_error_magnitude
    msg = [1, 2, 3, 4, 5]
    cw = ReedSolomon.encode(msg, 2)
    (1..255).each do |mag|
      corrupted = corrupt(cw, [0], [mag])
      recovered = ReedSolomon.decode(corrupted, 2)
      assert_equal msg, recovered, "mag=0x#{mag.to_s(16)}: failed"
    end
  end

  def test_check_bytes_can_be_corrupted
    msg = (1..10).to_a
    cw = ReedSolomon.encode(msg, 4)
    corrupted = corrupt(cw, [msg.length, msg.length + 1], [0xAA, 0xBB])
    assert_equal msg, ReedSolomon.decode(corrupted, 4)
  end

  # ===========================================================================
  # Capacity Limits
  # ===========================================================================

  def test_t_plus_1_errors_raises
    msg = Array.new(10, 0)
    cw = ReedSolomon.encode(msg, 4)
    corrupted = corrupt(cw, [0, 3, 7], [0xAA, 0xBB, 0xCC])
    assert_raises(ReedSolomon::TooManyErrors) { ReedSolomon.decode(corrupted, 4) }
  end

  def test_exactly_at_capacity
    msg = Array.new(10, 0)
    cw = ReedSolomon.encode(msg, 4)
    corrupted = corrupt(cw, [0, 5], [0xAA, 0xBB])
    assert_equal msg, ReedSolomon.decode(corrupted, 4)
  end

  def test_five_errors_raises_for_n_check_8
    msg = "Hello".bytes
    cw = ReedSolomon.encode(msg, 8)
    5.times { |i| cw[i] ^= (i + 1) * 17 }
    assert_raises(ReedSolomon::TooManyErrors) { ReedSolomon.decode(cw, 8) }
  end

  # ===========================================================================
  # ErrorLocator
  # ===========================================================================

  def test_error_locator_no_errors
    cw = ReedSolomon.encode("hello world".bytes, 8)
    s = ReedSolomon.syndromes(cw, 8)
    lam = ReedSolomon.error_locator(s)
    assert_equal [1], lam
  end

  def test_error_locator_one_error
    msg = Array.new(5, 0)
    cw = ReedSolomon.encode(msg, 8)
    cw[2] ^= 0x77
    s = ReedSolomon.syndromes(cw, 8)
    lam = ReedSolomon.error_locator(s)
    assert_equal 2, lam.length
    assert_equal 1, lam[0]
  end

  def test_error_locator_two_errors
    msg = Array.new(10, 0)
    cw = ReedSolomon.encode(msg, 8)
    cw[1] ^= 0xAA
    cw[8] ^= 0xBB
    s = ReedSolomon.syndromes(cw, 8)
    lam = ReedSolomon.error_locator(s)
    assert_equal 3, lam.length
    assert_equal 1, lam[0]
  end

  # ===========================================================================
  # Decode Validation
  # ===========================================================================

  def test_decode_raises_odd_n_check
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.decode(Array.new(10), 3) }
  end

  def test_decode_raises_zero_n_check
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.decode(Array.new(10), 0) }
  end

  def test_decode_raises_too_short
    assert_raises(ReedSolomon::InvalidInput) { ReedSolomon.decode([1, 2, 3], 4) }
  end

  def test_decode_exactly_n_check_length
    cw = ReedSolomon.encode([], 4)
    assert_equal [], ReedSolomon.decode(cw, 4)
  end

  # ===========================================================================
  # Test Vectors (cross-validated with Rust, TypeScript, Python, Go)
  # ===========================================================================

  def test_vector_generator_n_check_2
    # g(x) = (x+2)(x+4) = x² + 6x + 8  →  LE: [8, 6, 1]
    assert_equal [8, 6, 1], ReedSolomon.build_generator(2)
  end

  def test_vector_round_trip
    msg = [1, 2, 3, 4, 5, 6, 7, 8]
    n_check = 8
    cw = ReedSolomon.encode(msg, n_check)
    assert_equal 16, cw.length
    assert_equal msg, cw[0, 8]           # systematic
    s = ReedSolomon.syndromes(cw, n_check)
    assert s.all?(&:zero?), "non-zero syndrome on valid codeword"
    assert_equal msg, ReedSolomon.decode(cw, n_check)
  end

  def test_vector_known_correction
    msg = "Reed-Solomon".bytes
    n_check = 8
    cw = ReedSolomon.encode(msg, n_check)
    corrupted = corrupt(cw, [0, 3, 7, 10], [0xFF, 0xAA, 0x55, 0x0F])
    assert_equal msg, ReedSolomon.decode(corrupted, n_check)
  end

  def test_vector_alternating_bits
    msg = ([0xAA, 0x55] * 10)
    n_check = 8
    cw = ReedSolomon.encode(msg, n_check)
    s = ReedSolomon.syndromes(cw, n_check)
    assert s.all?(&:zero?), "non-zero syndrome on alternating-bit codeword"
    assert_equal msg, ReedSolomon.decode(cw, n_check)
  end

  def test_vector_all_255
    msg = Array.new(20, 0xFF)
    cw = ReedSolomon.encode(msg, 4)
    assert_equal msg, ReedSolomon.decode(cw, 4)
  end
end
