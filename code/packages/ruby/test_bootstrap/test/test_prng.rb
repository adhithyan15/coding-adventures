# frozen_string_literal: true

require_relative "test_helper"

class TestPrng < HookedTest
  SM = Prng::SplitMix64

  def test_seed_zero_first_output_is_pinned
    assert_equal 0xe220a8397b1dcdaf, SM.new(0).next_u64
  end

  def test_state_never_leaves_64_bits
    rng = SM.new(-1)
    1000.times do
      assert_operator rng.next_u64, :<, 2**64
      assert_operator rng.state, :<, 2**64
    end
    assert_equal Prng::MASK64, SM.new(-1).state
    assert_equal SM.new(5).next_u64, SM.new(5 + 2**64).next_u64
  end

  def test_strict_integer_arguments
    assert_raises(TypeError) { SM.new(true) }
    assert_raises(TypeError) { SM.new(1.5) }
    assert_raises(TypeError) { SM.new(0).bounded("3") }
    assert_raises(TypeError) { SM.new(0).bounded_big(nil) }
    assert_raises(TypeError) { Prng.fnv1a64(1) }
    assert_raises(TypeError) { Prng.sub_seed("1", "a") }
    assert_raises(TypeError) { Prng.sub_seed(1, :a) }
  end

  def test_fnv1a64_vectors
    assert_equal 14695981039346656037, Prng.fnv1a64("")
    assert_equal 12638187200555641996, Prng.fnv1a64("a")
    assert_equal 9625390261332436968, Prng.fnv1a64("foobar")
    assert_equal Prng.fnv1a64("é"), Prng.fnv1a64("é".b)
  end

  def test_sub_seed_vectors_from_the_independent_reference
    assert_equal 7249934457332604747, Prng.sub_seed(42, "prop/a")
    assert_equal 14087677454934409008, Prng.sub_seed(0, "")
    assert_equal 4559403196206768639, Prng.sub_seed(2**64 + 5, "x")
  end

  def test_bounded_one_consumes_exactly_one_word
    a = SM.new(7)
    assert_equal 0, a.bounded(1)
    b = SM.new(7)
    b.next_u64
    assert_equal b.next_u64, a.next_u64 # the second word is next in both
    assert_equal b.state, a.state
    assert_equal 309689372594955804, SM.new(7).tap(&:next_u64).next_u64
  end

  def test_bounded_rejects_out_of_range
    assert_raises(ArgumentError) { SM.new(0).bounded(0) }
    assert_raises(ArgumentError) { SM.new(0).bounded(-3) }
    assert_raises(ArgumentError) { SM.new(0).bounded(2**64 + 1) }
    assert_raises(ArgumentError) { SM.new(0).bounded_big(0) }
  end

  def test_bounded_two_to_the_64_is_one_raw_word
    assert_equal 7134611160154358618, SM.new(5).bounded(2**64)
    assert_equal 7134611160154358618, SM.new(5).bounded_big(2**64)
  end

  # The finalizer is a bijection, so a stream word of exactly 0 comes from state 0,
  # i.e. from seed (0 - gamma) mod 2^64.  bounded(3) has threshold 1, so that 0
  # MUST be rejected; a (-n) % n threshold (0 in Ruby) would keep it and answer 0.
  def test_forced_rejection_draw
    seed = (0 - Prng::GOLDEN_GAMMA) & Prng::MASK64
    assert_equal 7046029254386353131, seed
    probe = SM.new(seed)
    assert_equal 0, probe.next_u64
    second = probe.next_u64
    assert_equal 16294208416658607535, second
    assert_equal 1, second % 3
    assert_equal 1, SM.new(seed).bounded(3) # rejected the 0, answered from the 2nd word
  end

  def test_threshold_is_not_a_negative_modulo
    assert_equal 0, -3 % 3 # the trap: in Ruby this is always 0
    assert_equal 1, ((2**64) - 3) % 3
  end

  # n = 2^63 + 1 has threshold 2^63 - 1: almost half the stream is rejected.
  # Seed 3's first word (2092789425003139053) is below it, so a correct bounded
  # answers from the second word; a wrong threshold answers first % n.
  def test_two_to_the_63_plus_one_threshold
    n = (2**63) + 1
    assert_equal 2**63 - 1, ((2**64) - n) % n
    assert_equal 3694763184872335752, SM.new(3).bounded(n)
    refute_equal 2092789425003139053 % n, SM.new(3).bounded(n)
    # seed 1's first word is above the threshold: accepted immediately
    assert_equal 1227844342346046656, SM.new(1).bounded(n)
  end

  def test_bounded_big_thirds_of_a_wide_span
    span = 3 * (2**126)
    rng = SM.new(11)
    low = 0
    mid = 0
    n = 6000
    n.times do
      v = rng.bounded_big(span)
      assert_operator v, :<, span
      low += 1 if v < 2**126
      mid += 1 if v >= 2**126 && v < 2 * (2**126)
    end
    assert_in_delta 1.0 / 3, low.fdiv(n), 0.04 # an unrejected raw % span would give 1/2
    assert_in_delta 1.0 / 3, mid.fdiv(n), 0.04
  end

  def test_bounded_big_small_spans_match_bounded
    assert_equal SM.new(9).bounded(100), SM.new(9).bounded_big(100)
  end

  def test_bounded_is_uniform_enough
    rng = SM.new(1)
    counts = Array.new(5, 0)
    5000.times { counts[rng.bounded(5)] += 1 }
    counts.each { |c| assert_in_delta 1000, c, 150 }
  end
end
