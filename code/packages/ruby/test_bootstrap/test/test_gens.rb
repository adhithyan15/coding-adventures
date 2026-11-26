# frozen_string_literal: true

require_relative "test_helper"

class TestGens < HookedTest
  G = Pbt::Gens

  def rng(seed = 1) = Prng::SplitMix64.new(seed)

  def draws(gen, count = 200, seed = 1)
    r = rng(seed)
    Array.new(count) { gen.sample(r) }
  end

  def test_sampling_is_deterministic_per_seed
    gen = G.list(G.integer(-50, 50), 0, 10)
    assert_equal draws(gen, 20, 9), draws(gen, 20, 9)
    refute_equal draws(gen, 20, 9), draws(gen, 20, 10)
  end

  def test_primitives_stay_in_range
    assert(draws(G.u8).all? { |v| v.between?(0, 255) })
    assert(draws(G.u64).all? { |v| v.between?(0, 2**64 - 1) })
    assert(draws(G.i128).all? { |v| v.between?(-(2**127), 2**127 - 1) })
    assert(draws(G.integer(-3, 3)).all? { |v| v.between?(-3, 3) })
    assert_equal [true, false].sort_by(&:to_s), draws(G.bool).uniq.sort_by(&:to_s)
    assert_equal [4], draws(G.integer(4, 4), 5).uniq
    assert_equal [7], draws(G.constant(7), 5).uniq
  end

  def test_i128_reads_two_words_high_first_as_twos_complement
    r = rng(42)
    hi = r.next_u64
    lo = r.next_u64
    v = (hi << 64) | lo
    v -= 2**128 if v >= 2**127
    assert_equal v, G.i128.sample(rng(42))
    assert_equal 4, G.gen_primitive(42, "i128", 4).size
  end

  def test_integer_validation_and_wide_ranges
    assert_raises(ArgumentError) { G.integer(5, 4) }
    assert_raises(TypeError) { G.integer(1.0, 4) }
    wide = G.integer(-(2**200), 2**200)
    assert(draws(wide, 20).all? { |v| v.abs <= 2**200 })
  end

  def test_bytes_length_and_encoding
    out = draws(G.bytes(2, 5))
    assert(out.all? { |b| b.bytesize.between?(2, 5) && b.encoding == Encoding::BINARY })
    assert_equal [""], draws(G.bytes(0, 0), 3).map { |b| b.to_s }.uniq
    assert_raises(ArgumentError) { G.bytes(3, 2) }
    assert_raises(ArgumentError) { G.bytes(0, Pbt::Limits::MAX_INPUT_BYTES + 1) }
    assert_raises(ArgumentError) { G.bytes(-1, 2) }
    assert_raises(TypeError) { G.bytes(1.5, 2) }
  end

  def test_fixed_length_bytes_still_consume_the_length_word
    # bounded(1) consumes a word, so bytes(4,4) draws 1 + 4 words.
    r = rng(5)
    G.bytes(4, 4).sample(r)
    expected = rng(5)
    5.times { expected.next_u64 }
    assert_equal expected.state, r.state
  end

  def test_list_lengths_are_inclusive_and_respect_bounds
    out = draws(G.list(G.u8, 1, 3), 300)
    assert_equal [1, 2, 3], out.map(&:size).uniq.sort
    assert_raises(ArgumentError) { G.list(G.u8, 2, 1) }
    assert_raises(ArgumentError) { G.list(G.u8, 0, Pbt::Limits::MAX_LIST_LEN + 1) }
  end

  def test_map_transforms_and_drops_shrinking
    g = G.map(G.integer(0, 9)) { |x| x * 2 }
    assert(draws(g).all?(&:even?))
    assert_equal [50, 0], g.shrink(50, 100) { |_v| true }
  end

  def test_flat_map_samples_dependently
    g = G.flat_map(G.integer(1, 3)) { |n| G.list(G.constant(n), n, n) }
    assert(draws(g).all? { |l| l.size == l.first })
  end

  def test_filter_keeps_only_satisfying_values_and_shrinks_within_them
    g = G.filter(G.integer(0, 1000)) { |x| x.even? }
    assert(draws(g).all?(&:even?))
    value, = g.shrink(900, 1000) { |x| x >= 11 }
    assert_predicate value, :even? # an odd candidate is not a valid input, so it is never accepted
    assert_operator value, :>=, 11
  end

  def test_filter_exhaustion_is_raised_after_exactly_max_retries
    calls = 0
    g = G.filter(G.integer(0, 9), 7) do |_x|
      calls += 1
      false
    end
    assert_raises(Pbt::ExhaustedError) { g.sample(rng) }
    assert_equal 7, calls
  end

  def test_filter_validation
    assert_raises(ArgumentError) { G.filter(G.u8, 0) { true } }
    assert_raises(ArgumentError) { G.filter(G.u8, Pbt::Limits::MAX_RETRIES + 1) { true } }
    refute_nil G.filter(G.u8, Pbt::Limits::MAX_RETRIES) { true }
    assert_raises(ArgumentError) { G.filter(G.u8) }
    assert_raises(TypeError) { G.filter(G.u8, "x") { true } }
  end

  def test_one_of_is_uniform_and_validated
    out = draws(G.one_of(G.constant(:a), G.constant(:b), G.constant(:c)), 900)
    %i[a b c].each { |s| assert_in_delta 300, out.count(s), 70 }
    assert_raises(ArgumentError) { G.one_of }
  end

  def test_frequency_follows_weights_and_never_picks_zero_weight
    g = G.frequency([1, G.constant(:a)], [3, G.constant(:b)], [0, G.constant(:never)])
    out = draws(g, 2000)
    refute_includes out, :never
    assert_in_delta 500, out.count(:a), 100
    assert_in_delta 1500, out.count(:b), 100
  end

  def test_frequency_validation
    assert_raises(ArgumentError) { G.frequency }
    assert_raises(ArgumentError) { G.frequency([0, G.u8]) }
    assert_raises(ArgumentError) { G.frequency([-1, G.u8], [2, G.u8]) }
    assert_raises(ArgumentError) { G.frequency([2**32, G.u8], [1, G.u8]) }
    assert_raises(TypeError) { G.frequency(["x", G.u8]) }
    refute_nil G.frequency([2**32, G.u8])
  end

  def test_tuple_samples_in_order_and_shrinks_each_coordinate
    g = G.tuple(G.integer(0, 1000), G.integer(0, 1000))
    assert(draws(g, 10).all? { |t| t.size == 2 })
    minimal, steps = g.shrink([900, 800], 1000) { |t| t[0] + t[1] >= 100 }
    assert_equal 100, minimal.sum
    assert_operator steps, :>, 0
    assert_raises(ArgumentError) { G.tuple }
    assert_raises(ArgumentError) { G.tuple(*Array.new(65) { G.u8 }) }
  end

  def test_tuple_of_three_unlocks_across_passes
    g = G.tuple(G.integer(0, 100), G.integer(0, 100), G.bool)
    minimal, = g.shrink([90, 90, true], 1000) { |t| t[0] * t[1] >= 100 || t[2] }
    assert_equal true, minimal[2] || minimal[0] * minimal[1] >= 100
  end

  def test_bool_shrinks_to_false_only_when_false_fails
    assert_equal [false, 1], G.bool.shrink(true, 10) { |_v| true }
    assert_equal [true, 0], G.bool.shrink(true, 10) { |v| v }
    assert_equal [false, 0], G.bool.shrink(false, 10) { |_v| true }
    assert_equal [true, 0], G.bool.shrink(true, 0) { |_v| true }
  end

  def test_integer_shrinks_toward_the_in_range_point_nearest_zero
    assert_equal [10, 1], G.integer(10, 100).shrink(80, 100) { |_v| true }
    assert_equal [-10, 1], G.integer(-100, -10).shrink(-80, 100) { |_v| true }
    assert_equal [0, 1], G.integer(-5, 5).shrink(4, 100) { |_v| true }
  end

  def test_list_shrink_respects_min_len_and_uses_the_element_shrinker
    g = G.list(G.integer(0, 100), 2, 6)
    minimal, = g.shrink([50, 60, 70, 80], 1000) { |l| l.sum >= 100 }
    assert_equal 100, minimal.sum
    assert_operator minimal.size, :>=, 2
  end

  def test_bytes_shrink_respects_min_len
    minimal, = G.bytes(3, 8).shrink("\x05\x06\x07\x08\x09".b, 1000) { |_b| true }
    assert_equal ("\x00" * 3).b, minimal
  end

  def test_a_gen_without_a_shrinker_returns_its_value
    g = Pbt::Gen.new(->(_r) { 1 })
    assert_equal [5, 0], g.shrink(5, 10) { |_v| true }
    assert_raises(ArgumentError) { Pbt::Gen.new(nil) }
    assert_raises(ArgumentError) { Pbt::Gen.new(->(_r) { 1 }, 5) }
    assert_raises(ArgumentError) { g.shrink(5, 10) }
    assert_raises(ArgumentError) { g.shrink(5, -1) { true } }
    assert_equal 3, Pbt::Gen.of { |_r| 3 }.sample(rng)
  end

  def test_corpus_entry_points_validate
    assert_raises(ArgumentError) { G.gen_primitive(1, "float", 1) }
    assert_raises(ArgumentError) { G.gen_primitive(1, "bool", Pbt::Limits::MAX_GEN_COUNT + 1) }
    assert_raises(ArgumentError) { G.gen_primitive(1, "bool", -1) }
    assert_raises(ArgumentError) { G.gen_bounded(1, 5, 4, 1) }
    assert_raises(TypeError) { G.gen_bounded(1, 1.0, 4, 1) }
    assert_raises(ArgumentError) { G.gen_bounded(1, 0, 2**4000, 1_000_000) }
    assert_raises(ArgumentError) { G.gen_bytes(1, 0, 0, Pbt::Limits::MAX_GEN_COUNT + 1) } # even with max_len 0
    assert_raises(ArgumentError) { G.gen_bytes(1, 0, 100_000, 100) }
    assert_equal 3, G.gen_bytes(1, 0, 0, 3).size
  end

  def test_gen_bounded_uses_bounded_draws
    out = G.gen_bounded(7, 10, 20, 8)
    assert_equal [12, 10, 10, 10, 17, 17, 11, 19], out
    wide = G.gen_bounded(1, 0, 2**100, 3)
    assert(wide.all? { |v| v.between?(0, 2**100) })
  end
end
