# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_stats"

# Tests for descriptive statistics, frequency analysis, and cryptanalysis.
#
# Each test verifies the parity test vectors from the ST01 spec, ensuring
# identical results across all language implementations.

class TestDescriptive < Minitest::Test
  D = CodingAdventures::Stats::Descriptive

  # ── Mean ────────────────────────────────────────────────────────────

  def test_mean_parity
    # ST01 parity: mean([1,2,3,4,5]) -> 3.0
    assert_equal 3.0, D.mean([1, 2, 3, 4, 5])
  end

  def test_mean_single
    assert_equal 42.0, D.mean([42.0])
  end

  def test_mean_negative
    assert_equal 0.0, D.mean([-3, -1, 0, 1, 3])
  end

  def test_mean_large_dataset
    assert_equal 50.5, D.mean((1..100).to_a)
  end

  def test_mean_empty_raises
    assert_raises(ArgumentError) { D.mean([]) }
  end

  def test_mean_floating_point
    assert_in_delta 0.2, D.mean([0.1, 0.2, 0.3]), 1e-10
  end

  # ── Median ──────────────────────────────────────────────────────────

  def test_median_odd
    assert_equal 3.0, D.median([1, 2, 3, 4, 5])
  end

  def test_median_even
    assert_equal 2.5, D.median([1, 2, 3, 4])
  end

  def test_median_single
    assert_equal 7.0, D.median([7.0])
  end

  def test_median_two_values
    assert_equal 2.0, D.median([1.0, 3.0])
  end

  def test_median_unsorted
    assert_equal 3.0, D.median([5, 1, 3, 2, 4])
  end

  def test_median_empty_raises
    assert_raises(ArgumentError) { D.median([]) }
  end

  # ── Mode ────────────────────────────────────────────────────────────

  def test_mode_parity
    assert_equal 2.0, D.mode([1, 2, 2, 3])
  end

  def test_mode_single
    assert_equal 5.0, D.mode([5.0])
  end

  def test_mode_tie_first_wins
    # 1 and 3 both appear twice; 1 appears first.
    assert_equal 1.0, D.mode([1, 3, 1, 3])
  end

  def test_mode_all_same
    assert_equal 7.0, D.mode([7, 7, 7])
  end

  def test_mode_empty_raises
    assert_raises(ArgumentError) { D.mode([]) }
  end

  # ── Variance ────────────────────────────────────────────────────────

  def test_variance_sample_parity
    result = D.variance([2, 4, 4, 4, 5, 5, 7, 9])
    assert_in_delta 4.571428571428571, result, 1e-10
  end

  def test_variance_population_parity
    result = D.variance([2, 4, 4, 4, 5, 5, 7, 9], population: true)
    assert_in_delta 4.0, result, 1e-10
  end

  def test_variance_zero
    assert_equal 0.0, D.variance([5, 5, 5, 5], population: true)
  end

  def test_variance_single_population
    assert_equal 0.0, D.variance([42.0], population: true)
  end

  def test_variance_single_sample_raises
    assert_raises(ArgumentError) { D.variance([42.0]) }
  end

  def test_variance_empty_raises
    assert_raises(ArgumentError) { D.variance([]) }
  end

  # ── Standard Deviation ──────────────────────────────────────────────

  def test_standard_deviation_sample
    result = D.standard_deviation([2, 4, 4, 4, 5, 5, 7, 9])
    assert_in_delta Math.sqrt(4.571428571428571), result, 1e-10
  end

  def test_standard_deviation_population
    result = D.standard_deviation([2, 4, 4, 4, 5, 5, 7, 9], population: true)
    assert_in_delta 2.0, result, 1e-10
  end

  # ── Min / Max / Range ───────────────────────────────────────────────

  def test_min
    assert_equal 1.0, D.min([3, 1, 4, 1, 5])
  end

  def test_max
    assert_equal 5.0, D.max([3, 1, 4, 1, 5])
  end

  def test_range
    assert_equal 7.0, D.range([2, 4, 4, 4, 5, 5, 7, 9])
  end

  def test_range_single
    assert_equal 0.0, D.range([5.0])
  end

  def test_negative_min_max_range
    assert_equal(-5.0, D.min([-5, -1, 0, 3]))
    assert_equal 3.0, D.max([-5, -1, 0, 3])
    assert_equal 8.0, D.range([-5, -1, 0, 3])
  end

  def test_min_empty_raises
    assert_raises(ArgumentError) { D.min([]) }
  end

  def test_max_empty_raises
    assert_raises(ArgumentError) { D.max([]) }
  end
end

class TestFrequency < Minitest::Test
  F = CodingAdventures::Stats::Frequency

  # ── Frequency Count ─────────────────────────────────────────────────

  def test_frequency_count_parity
    result = F.frequency_count("Hello")
    assert_equal({"H" => 1, "E" => 1, "L" => 2, "O" => 1}, result)
  end

  def test_frequency_count_case_insensitive
    assert_equal({"A" => 3}, F.frequency_count("AaA"))
  end

  def test_frequency_count_ignores_non_alpha
    result = F.frequency_count("A1 B! C?")
    assert_equal({"A" => 1, "B" => 1, "C" => 1}, result)
  end

  def test_frequency_count_empty
    assert_equal({}, F.frequency_count(""))
  end

  def test_frequency_count_numbers_only
    assert_equal({}, F.frequency_count("12345"))
  end

  def test_frequency_count_full_alphabet
    result = F.frequency_count("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    assert_equal 26, result.length
    ("A".."Z").each { |l| assert_equal 1, result[l] }
  end

  # ── Frequency Distribution ──────────────────────────────────────────

  def test_frequency_distribution_uniform
    result = F.frequency_distribution("AABB")
    assert_in_delta 0.5, result["A"], 1e-10
    assert_in_delta 0.5, result["B"], 1e-10
  end

  def test_frequency_distribution_empty
    assert_equal({}, F.frequency_distribution(""))
  end

  def test_frequency_distribution_sums_to_one
    result = F.frequency_distribution("HELLO WORLD")
    total = result.values.sum
    assert_in_delta 1.0, total, 1e-6
  end

  def test_frequency_distribution_single
    assert_equal({"A" => 1.0}, F.frequency_distribution("AAA"))
  end

  # ── Chi-Squared ─────────────────────────────────────────────────────

  def test_chi_squared_parity
    result = F.chi_squared([10, 20, 30], [20, 20, 20])
    assert_in_delta 10.0, result, 1e-10
  end

  def test_chi_squared_perfect_match
    result = F.chi_squared([10, 20, 30], [10, 20, 30])
    assert_in_delta 0.0, result, 1e-10
  end

  def test_chi_squared_length_mismatch_raises
    assert_raises(ArgumentError) { F.chi_squared([1, 2], [1, 2, 3]) }
  end

  def test_chi_squared_single
    result = F.chi_squared([5.0], [10.0])
    assert_in_delta 2.5, result, 1e-10
  end

  # ── Chi-Squared Text ────────────────────────────────────────────────

  def test_chi_squared_text_perfect
    result = F.chi_squared_text("A" * 100, {"A" => 1.0})
    assert_in_delta 0.0, result, 1e-10
  end

  def test_chi_squared_text_empty
    assert_equal 0.0, F.chi_squared_text("", {"A" => 0.5})
  end

  def test_chi_squared_text_english_vs_random
    ef = CodingAdventures::Stats::Cryptanalysis::ENGLISH_FREQUENCIES
    english = "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG"
    random_text = "Z" * 33
    chi_english = F.chi_squared_text(english, ef)
    chi_random = F.chi_squared_text(random_text, ef)
    assert chi_english < chi_random, "English should score lower than random"
  end

  def test_chi_squared_text_case_insensitive
    freq = {"H" => 0.2, "E" => 0.2, "L" => 0.4, "O" => 0.2}
    r1 = F.chi_squared_text("HELLO", freq)
    r2 = F.chi_squared_text("hello", freq)
    assert_in_delta r1, r2, 1e-10
  end
end

class TestCryptanalysis < Minitest::Test
  C = CodingAdventures::Stats::Cryptanalysis

  # ── Index of Coincidence ────────────────────────────────────────────

  def test_ic_parity
    result = C.index_of_coincidence("AABB")
    assert_in_delta(1.0 / 3.0, result, 1e-10)
  end

  def test_ic_all_same
    assert_in_delta 1.0, C.index_of_coincidence("AAAA"), 1e-10
  end

  def test_ic_all_different
    assert_in_delta 0.0, C.index_of_coincidence("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), 1e-10
  end

  def test_ic_english_range
    result = C.index_of_coincidence("TOBEORNOTTOBETHATISTHEQUESTION")
    assert result > 0.0, "IC should be > 0.0 for English text"
  end

  def test_ic_empty
    assert_equal 0.0, C.index_of_coincidence("")
  end

  def test_ic_single
    assert_equal 0.0, C.index_of_coincidence("A")
  end

  def test_ic_case_insensitive
    a = C.index_of_coincidence("aabb")
    b = C.index_of_coincidence("AABB")
    assert_in_delta a, b, 1e-10
  end

  def test_ic_ignores_non_alpha
    a = C.index_of_coincidence("A A B B")
    b = C.index_of_coincidence("AABB")
    assert_in_delta a, b, 1e-10
  end

  # ── Entropy ─────────────────────────────────────────────────────────

  def test_entropy_single_repeated
    assert_in_delta 0.0, C.entropy("AAAA"), 1e-10
  end

  def test_entropy_two_equal
    assert_in_delta 1.0, C.entropy("ABABABAB"), 1e-10
  end

  def test_entropy_uniform_26
    result = C.entropy("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    assert_in_delta Math.log2(26), result, 1e-6
  end

  def test_entropy_empty
    assert_equal 0.0, C.entropy("")
  end

  def test_entropy_case_insensitive
    assert_in_delta C.entropy("aabb"), C.entropy("AABB"), 1e-10
  end

  def test_entropy_increases_with_diversity
    low = C.entropy("AAAB")
    high = C.entropy("ABCD")
    assert high > low, "More diverse text should have higher entropy"
  end

  # ── English Frequencies ─────────────────────────────────────────────

  def test_english_frequencies_has_26
    assert_equal 26, C::ENGLISH_FREQUENCIES.length
  end

  def test_english_frequencies_sums_to_one
    total = C::ENGLISH_FREQUENCIES.values.sum
    assert_in_delta 1.0, total, 0.001
  end

  def test_english_frequencies_e_most_frequent
    max_letter = C::ENGLISH_FREQUENCIES.max_by { |_, v| v }[0]
    assert_equal "E", max_letter
  end

  def test_english_frequencies_spot_check
    assert_in_delta 0.08167, C::ENGLISH_FREQUENCIES["A"], 1e-5
    assert_in_delta 0.12702, C::ENGLISH_FREQUENCIES["E"], 1e-5
    assert_in_delta 0.00074, C::ENGLISH_FREQUENCIES["Z"], 1e-5
  end
end
