# frozen_string_literal: true

require_relative "test_helper"

class RankStatisticsTest < Minitest::Test
  RS = CodingAdventures::RankStatistics
  PBT = CodingAdventures::TestBootstrap::Pbt

  def test_odd_and_even_medians_leave_input_unchanged
    values = [9, 1, 5, 3]
    before = values.dup
    assert_equal 4.0, RS.median(values)
    assert_equal before, values
    assert_equal 5.0, RS.median([9, 1, 5])
  end

  def test_median_handles_finite_extremes_and_subnormals
    max = Float::MAX
    tiny = Float::MIN / 2.0
    assert_equal max, RS.median([max, max])
    assert_equal 0.0, RS.median([-max, max])
    assert_equal tiny, RS.median([tiny, tiny])
    assert_equal 1.5 * tiny, RS.median([tiny, 2 * tiny])
  end

  def test_median_rejects_empty_invalid_and_nonfinite_samples
    assert_raises(ArgumentError) { RS.median([]) }
    assert_raises(TypeError) { RS.median("123") }
    [true, "1", Complex(1, 2), Float::NAN, Float::INFINITY, -Float::INFINITY, 10**1000].each do |value|
      assert_raises(StandardError) { RS.median([value]) }
    end
  end

  def test_interval_matches_exhaustive_sign_assignment_oracle
    1.upto(10) do |n|
      values = Array.new(n) { |index| (index % 3).to_f }
      sign_counts = Array.new(n + 1, 0)
      (0...(1 << n)).each { |mask| sign_counts[mask.digits(2).sum] += 1 }
      1.upto(99) do |confidence|
        rank = exhaustive_rank(sign_counts, n, confidence)
        expected = rank.zero? ? nil : [values.sort[rank - 1], values.sort[n - rank]]
        actual = RS.median_interval(values, confidence)
        expected.nil? ? assert_nil(actual, "n=#{n}, confidence=#{confidence}") : assert_equal(expected, actual, "n=#{n}, confidence=#{confidence}")
      end
    end
  end

  def test_interval_cutoffs_match_independent_multi_limb_pascal_oracle
    [129, 257, 1024].each do |n|
      values = (0...n).map(&:to_f)
      coefficients = binomial_row(n)
      [50, 90, 95, 99].each do |confidence|
        expected_rank = independent_cutoff_rank(coefficients, n, confidence)
        expected = expected_rank.zero? ? nil : [expected_rank - 1.0, n - expected_rank.to_f]
        assert_equal expected, RS.median_interval(values, confidence), "n=#{n}, confidence=#{confidence}"
      end
    end
    assert_operator binomial_row(1024)[512].bit_length, :>, 512
  end

  def test_interval_rejects_invalid_confidence_and_keeps_ties
    [true, 0, 100, 1.5].each do |confidence|
      assert_raises(ArgumentError) { RS.median_interval([1], confidence) }
    end
    assert_equal [0.8, 1.2], RS.median_interval([0.8, 0.8, 1.2, 1.2, 1.2, 0.8], 50)
    assert_nil RS.median_interval([1, 2, 3, 4, 5], 95)
  end

  def test_pbt_median_is_permutation_invariant_and_in_range
    gen = PBT::Gens.list(PBT::Gens.integer(-100_000, 100_000), 1, 24)
    verdict = PBT::Runner.check(0x4D454449, gen, PBT::Options.new(num_tests: 500, ci: :off)) do |values|
      value = RS.median(values)
      value == RS.median(values.reverse) && value >= values.min && value <= values.max
    end
    assert verdict.passed, verdict.message
  end

  def test_pbt_interval_endpoints_are_observed_and_ordered
    values = PBT::Gens.list(PBT::Gens.integer(-32, 32), 1, 24)
    confidence = PBT::Gens.integer(1, 99)
    gen = PBT::Gens.tuple(values, confidence)
    verdict = PBT::Runner.check(0x494E5445, gen, PBT::Options.new(num_tests: 500, ci: :off)) do |sample, level|
      interval = RS.median_interval(sample, level)
      interval.nil? || (interval[0] <= interval[1] && sample.include?(interval[0]) && sample.include?(interval[1]))
    end
    assert verdict.passed, verdict.message
  end

  private

  # Enumerate all sign assignments directly; this oracle shares neither the
  # binomial recurrence nor the cutoff arithmetic implementation.
  def exhaustive_rank(sign_counts, n, confidence)
    rank = 0
    below = 0
    1.upto(n) do |candidate|
      below += sign_counts[candidate - 1]
      rank = candidate if 200 * below <= (100 - confidence) * (1 << n)
    end
    rank
  end

  # Pascal's triangle independently computes all exact coefficients for wide
  # samples; the production code instead updates a single coefficient.
  def binomial_row(n)
    row = [1]
    n.times do
      next_row = Array.new(row.length + 1, 1)
      1.upto(row.length - 1) { |index| next_row[index] = row[index - 1] + row[index] }
      row = next_row
    end
    row
  end

  def independent_cutoff_rank(coefficients, n, confidence)
    denominator = 1 << n
    cumulative = 0
    rank = 0
    1.upto(n) do |candidate|
      cumulative += coefficients[candidate - 1]
      break if 200 * cumulative > (100 - confidence) * denominator

      rank = candidate
    end
    rank
  end
end
