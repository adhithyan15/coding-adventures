# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_hash_functions"

class HashFunctionsTest < Minitest::Test
  HF = CodingAdventures::HashFunctions

  def test_fnv1a_vectors
    assert_equal 2_166_136_261, HF.fnv1a32("")
    assert_equal 3_826_002_220, HF.fnv1a32("a")
    assert_equal 440_920_331, HF.fnv1a32("abc")
    assert_equal 1_335_831_723, HF.fnv1a32("hello")
    assert_equal 3_214_735_720, HF.fnv1a32("foobar")

    assert_equal 14_695_981_039_346_656_037, HF.fnv1a64("")
    assert_equal 12_638_187_200_555_641_996, HF.fnv1a64("a")
    assert_equal 16_654_208_175_385_433_931, HF.fnv1a64("abc")
    assert_equal 11_831_194_018_420_276_491, HF.fnv1a64("hello")
  end

  def test_djb2_vectors
    assert_equal 5381, HF.djb2("")
    assert_equal 177_670, HF.djb2("a")
    assert_equal 193_485_963, HF.djb2("abc")
    assert_equal 210_714_636_441, HF.djb2("hello")
  end

  def test_polynomial_rolling
    assert_equal 0, HF.polynomial_rolling("")
    assert_equal 97, HF.polynomial_rolling("a")
    assert_equal 3105, HF.polynomial_rolling("ab")
    assert_equal 96_354, HF.polynomial_rolling("abc")
    assert_operator HF.polynomial_rolling("hello world", modulus: 100), :<, 100
    assert_raises(ArgumentError) { HF.polynomial_rolling("x", modulus: 0) }
  end

  def test_murmur3_vectors
    assert_equal 0, HF.murmur3_32("", seed: 0)
    assert_equal 0x514E28B7, HF.murmur3_32("", seed: 1)
    assert_equal 0x3C2569B2, HF.murmur3_32("a")
    assert_equal 0xB3DD93FA, HF.murmur3_32("abc")
  end

  def test_murmur3_tail_paths_and_seed
    %w[abcd abcde abcdef abcdefg].each { |input| assert_kind_of Integer, HF.murmur3_32(input) }
    refute_equal HF.murmur3_32("hello", seed: 0), HF.murmur3_32("hello", seed: 1)
  end

  def test_analysis_helpers
    score = HF.avalanche_score(->(data) { HF.fnv1a32(data) }, output_bits: 32, sample_size: 8)
    assert_operator score, :>=, 0.0
    assert_operator score, :<=, 1.0

    chi2 = HF.distribution_test(->(_data) { 0 }, %w[a b c d], num_buckets: 4)
    assert_equal 12.0, chi2
  end

  def test_analysis_rejects_invalid_inputs
    fnv = ->(data) { HF.fnv1a32(data) }

    assert_raises(ArgumentError) { HF.avalanche_score(fnv, output_bits: 0) }
    assert_raises(ArgumentError) { HF.avalanche_score(fnv, output_bits: 32, sample_size: 0) }
    assert_raises(ArgumentError) { HF.distribution_test(fnv, [], num_buckets: 4) }
    assert_raises(ArgumentError) { HF.distribution_test(fnv, ["x"], num_buckets: 0) }
  end
end
