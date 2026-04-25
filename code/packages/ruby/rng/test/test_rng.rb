# frozen_string_literal: true

# test_rng.rb — unit tests for coding_adventures_rng.
#
# test_helper must be the FIRST require so SimpleCov starts before anything
# else is loaded.  This ensures accurate coverage measurement.
require_relative "test_helper"

# ── Helper ────────────────────────────────────────────────────────────────────

# Collect the first n values from a generator using a given method.
def take(gen, method, n)
  Array.new(n) { gen.public_send(method) }
end

# ── LCG tests ─────────────────────────────────────────────────────────────────

class TestLCG < Minitest::Test
  # Reference values produced by the Go implementation for seed=1.
  SEED1_U32 = [1_817_669_548, 2_187_888_307, 2_784_682_393].freeze

  def test_version_exists
    refute_nil CodingAdventures::Rng::VERSION
  end

  # The first three next_u32 outputs for seed=1 must match the Go reference.
  def test_lcg_seed1_reference_values
    g = CodingAdventures::Rng::LCG.new(1)
    assert_equal SEED1_U32, take(g, :next_u32, 3)
  end

  # All outputs must be in [0, 2^32).
  def test_lcg_next_u32_range
    g = CodingAdventures::Rng::LCG.new(42)
    100.times { assert_includes 0...2**32, g.next_u32 }
  end

  # Seed 0 must be accepted and produce valid outputs.
  def test_lcg_seed_zero
    g = CodingAdventures::Rng::LCG.new(0)
    v = g.next_u32
    assert_includes 0...2**32, v
  end

  # Two generators with the same seed must produce identical streams.
  def test_lcg_reproducibility
    a = CodingAdventures::Rng::LCG.new(999)
    b = CodingAdventures::Rng::LCG.new(999)
    assert_equal take(a, :next_u32, 20), take(b, :next_u32, 20)
  end

  # Two generators with different seeds must diverge after at most a few steps.
  def test_lcg_different_seeds_differ
    a = CodingAdventures::Rng::LCG.new(1)
    b = CodingAdventures::Rng::LCG.new(2)
    refute_equal take(a, :next_u32, 10), take(b, :next_u32, 10)
  end

  # next_u64 must return a value in [0, 2^64).
  def test_lcg_next_u64_range
    g = CodingAdventures::Rng::LCG.new(7)
    10.times { assert_includes 0...2**64, g.next_u64 }
  end

  # next_u64 is defined as (hi << 32) | lo from two consecutive next_u32 calls.
  def test_lcg_next_u64_composition
    g1 = CodingAdventures::Rng::LCG.new(5)
    g2 = CodingAdventures::Rng::LCG.new(5)
    expected = (g2.next_u32 << 32) | g2.next_u32
    assert_equal expected, g1.next_u64
  end

  # next_float must lie in [0.0, 1.0).
  def test_lcg_next_float_range
    g = CodingAdventures::Rng::LCG.new(13)
    200.times do
      f = g.next_float
      assert f >= 0.0, "float #{f} < 0"
      assert f < 1.0,  "float #{f} >= 1"
    end
  end

  # next_int_in_range must always lie within [min, max].
  def test_lcg_next_int_in_range_bounds
    g = CodingAdventures::Rng::LCG.new(17)
    200.times { assert_includes 1..6, g.next_int_in_range(1, 6) }
  end

  # A range of size 1 must always return exactly min.
  def test_lcg_next_int_in_range_single_value
    g = CodingAdventures::Rng::LCG.new(0)
    10.times { assert_equal 5, g.next_int_in_range(5, 5) }
  end

  # A wider range must cover all possible values eventually.
  def test_lcg_next_int_in_range_all_values_covered
    g = CodingAdventures::Rng::LCG.new(99)
    seen = (1..10).each_with_object(Hash.new(0)) do |_, acc|
      acc[g.next_int_in_range(1, 10)] += 1
    end
    # After 1000 draws the chi-squared test would be overkill; just verify the
    # full range was covered at some point across a generous sample.
    g2 = CodingAdventures::Rng::LCG.new(99)
    seen2 = Array.new(1000) { g2.next_int_in_range(1, 10) }.uniq.sort
    assert_equal (1..10).to_a, seen2
  end

  # Negative ranges must work correctly.
  def test_lcg_negative_range
    g = CodingAdventures::Rng::LCG.new(3)
    200.times { assert_includes(-10..-1, g.next_int_in_range(-10, -1)) }
  end

  # A large power-of-two range triggers the threshold=0 fast path.
  def test_lcg_power_of_two_range
    g = CodingAdventures::Rng::LCG.new(2)
    200.times { assert_includes 0..255, g.next_int_in_range(0, 255) }
  end
end

# ── Xorshift64 tests ──────────────────────────────────────────────────────────

class TestXorshift64 < Minitest::Test
  SEED1_U32 = [1_082_269_761, 201_397_313, 1_854_285_353].freeze

  def test_xorshift64_seed1_reference_values
    g = CodingAdventures::Rng::Xorshift64.new(1)
    assert_equal SEED1_U32, take(g, :next_u32, 3)
  end

  # Seed 0 must be silently replaced with 1 (fixed-point protection).
  def test_xorshift64_seed_zero_replaced_with_one
    g_zero = CodingAdventures::Rng::Xorshift64.new(0)
    g_one  = CodingAdventures::Rng::Xorshift64.new(1)
    assert_equal g_one.next_u32, g_zero.next_u32
  end

  def test_xorshift64_next_u32_range
    g = CodingAdventures::Rng::Xorshift64.new(42)
    100.times { assert_includes 0...2**32, g.next_u32 }
  end

  def test_xorshift64_reproducibility
    a = CodingAdventures::Rng::Xorshift64.new(555)
    b = CodingAdventures::Rng::Xorshift64.new(555)
    assert_equal take(a, :next_u32, 20), take(b, :next_u32, 20)
  end

  def test_xorshift64_different_seeds_differ
    a = CodingAdventures::Rng::Xorshift64.new(10)
    b = CodingAdventures::Rng::Xorshift64.new(11)
    refute_equal take(a, :next_u32, 10), take(b, :next_u32, 10)
  end

  def test_xorshift64_next_u64_range
    g = CodingAdventures::Rng::Xorshift64.new(8)
    10.times { assert_includes 0...2**64, g.next_u64 }
  end

  def test_xorshift64_next_u64_composition
    g1 = CodingAdventures::Rng::LCG.new(5)
    g2 = CodingAdventures::Rng::LCG.new(5)
    expected = (g2.next_u32 << 32) | g2.next_u32
    assert_equal expected, g1.next_u64
  end

  def test_xorshift64_next_float_range
    g = CodingAdventures::Rng::Xorshift64.new(14)
    200.times do
      f = g.next_float
      assert f >= 0.0
      assert f < 1.0
    end
  end

  def test_xorshift64_next_int_in_range_bounds
    g = CodingAdventures::Rng::Xorshift64.new(18)
    200.times { assert_includes 1..6, g.next_int_in_range(1, 6) }
  end

  def test_xorshift64_next_int_single_value
    g = CodingAdventures::Rng::Xorshift64.new(0)
    10.times { assert_equal 7, g.next_int_in_range(7, 7) }
  end

  def test_xorshift64_state_not_zero_after_many_steps
    g = CodingAdventures::Rng::Xorshift64.new(1)
    # The generator should never produce all-zero output (which would mean
    # it hit the fixed point 0 in state).
    1000.times { refute_equal 0, g.next_u64 }
  end
end

# ── PCG32 tests ───────────────────────────────────────────────────────────────

class TestPCG32 < Minitest::Test
  SEED1_U32 = [1_412_771_199, 1_791_099_446, 124_312_908].freeze

  def test_pcg32_seed1_reference_values
    g = CodingAdventures::Rng::PCG32.new(1)
    assert_equal SEED1_U32, take(g, :next_u32, 3)
  end

  def test_pcg32_seed_zero_valid
    g = CodingAdventures::Rng::PCG32.new(0)
    v = g.next_u32
    assert_includes 0...2**32, v
  end

  def test_pcg32_next_u32_range
    g = CodingAdventures::Rng::PCG32.new(123)
    100.times { assert_includes 0...2**32, g.next_u32 }
  end

  def test_pcg32_reproducibility
    a = CodingAdventures::Rng::PCG32.new(777)
    b = CodingAdventures::Rng::PCG32.new(777)
    assert_equal take(a, :next_u32, 20), take(b, :next_u32, 20)
  end

  def test_pcg32_different_seeds_differ
    a = CodingAdventures::Rng::PCG32.new(100)
    b = CodingAdventures::Rng::PCG32.new(200)
    refute_equal take(a, :next_u32, 10), take(b, :next_u32, 10)
  end

  def test_pcg32_next_u64_range
    g = CodingAdventures::Rng::PCG32.new(9)
    10.times { assert_includes 0...2**64, g.next_u64 }
  end

  def test_pcg32_next_u64_composition
    g1 = CodingAdventures::Rng::PCG32.new(5)
    g2 = CodingAdventures::Rng::PCG32.new(5)
    expected = (g2.next_u32 << 32) | g2.next_u32
    assert_equal expected, g1.next_u64
  end

  def test_pcg32_next_float_range
    g = CodingAdventures::Rng::PCG32.new(15)
    200.times do
      f = g.next_float
      assert f >= 0.0
      assert f < 1.0
    end
  end

  def test_pcg32_next_int_in_range_bounds
    g = CodingAdventures::Rng::PCG32.new(19)
    200.times { assert_includes 1..6, g.next_int_in_range(1, 6) }
  end

  def test_pcg32_next_int_single_value
    g = CodingAdventures::Rng::PCG32.new(0)
    10.times { assert_equal 3, g.next_int_in_range(3, 3) }
  end

  def test_pcg32_negative_range
    g = CodingAdventures::Rng::PCG32.new(5)
    200.times { assert_includes(-5..0, g.next_int_in_range(-5, 0)) }
  end

  # PCG32's output permutation should produce different values than LCG for
  # the same seed, demonstrating the permutation is actually applied.
  def test_pcg32_differs_from_lcg
    pcg = CodingAdventures::Rng::PCG32.new(1)
    lcg = CodingAdventures::Rng::LCG.new(1)
    refute_equal take(pcg, :next_u32, 10), take(lcg, :next_u32, 10)
  end

  # A large power-of-two range triggers the threshold=0 fast path.
  def test_pcg32_power_of_two_range
    g = CodingAdventures::Rng::PCG32.new(4)
    200.times { assert_includes 0..1023, g.next_int_in_range(0, 1023) }
  end
end
