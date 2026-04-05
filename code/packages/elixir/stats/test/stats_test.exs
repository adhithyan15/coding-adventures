defmodule CodingAdventures.StatsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Stats.Descriptive
  alias CodingAdventures.Stats.Frequency
  alias CodingAdventures.Stats.Cryptanalysis

  # ─────────────────────────────────────────────────────────────────
  # Descriptive Statistics
  # ─────────────────────────────────────────────────────────────────

  describe "mean/1" do
    test "parity test vector" do
      assert_in_delta Descriptive.mean([1, 2, 3, 4, 5]), 3.0, 1.0e-10
    end

    test "worked example" do
      assert_in_delta Descriptive.mean([2, 4, 4, 4, 5, 5, 7, 9]), 5.0, 1.0e-10
    end

    test "single element" do
      assert Descriptive.mean([42]) == 42.0
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.mean([]) end
    end
  end

  describe "median/1" do
    test "odd-length list" do
      assert Descriptive.median([1, 3, 5]) == 3
    end

    test "even-length list" do
      assert_in_delta Descriptive.median([2, 4, 4, 4, 5, 5, 7, 9]), 4.5, 1.0e-10
    end

    test "unsorted input" do
      assert Descriptive.median([5, 1, 3]) == 3
    end

    test "single element" do
      assert Descriptive.median([7]) == 7
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.median([]) end
    end
  end

  describe "mode/1" do
    test "most frequent value" do
      assert Descriptive.mode([2, 4, 4, 4, 5, 5, 7, 9]) == 4
    end

    test "first occurrence wins tie" do
      assert Descriptive.mode([1, 2, 1, 2, 3]) == 1
    end

    test "single element" do
      assert Descriptive.mode([99]) == 99
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.mode([]) end
    end
  end

  describe "variance/2" do
    @values [2, 4, 4, 4, 5, 5, 7, 9]

    test "sample variance (parity test vector)" do
      assert_in_delta Descriptive.variance(@values), 4.571428571428571, 1.0e-10
    end

    test "population variance (parity test vector)" do
      assert_in_delta Descriptive.variance(@values, population: true), 4.0, 1.0e-10
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.variance([]) end
    end

    test "raises on single-element sample variance" do
      assert_raise ArgumentError, ~r/at least 2/, fn -> Descriptive.variance([5]) end
    end

    test "allows single-element population variance" do
      assert Descriptive.variance([5], population: true) == 0.0
    end
  end

  describe "standard_deviation/2" do
    test "sample standard deviation" do
      values = [2, 4, 4, 4, 5, 5, 7, 9]
      expected = :math.sqrt(4.571428571428571)
      assert_in_delta Descriptive.standard_deviation(values), expected, 1.0e-10
    end

    test "population standard deviation" do
      values = [2, 4, 4, 4, 5, 5, 7, 9]
      assert_in_delta Descriptive.standard_deviation(values, population: true), 2.0, 1.0e-10
    end
  end

  describe "min_val/1" do
    test "finds minimum" do
      assert Descriptive.min_val([2, 4, 4, 4, 5, 5, 7, 9]) == 2
    end

    test "handles negatives" do
      assert Descriptive.min_val([-3, -1, 0, 5]) == -3
    end

    test "single element" do
      assert Descriptive.min_val([42]) == 42
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.min_val([]) end
    end
  end

  describe "max_val/1" do
    test "finds maximum" do
      assert Descriptive.max_val([2, 4, 4, 4, 5, 5, 7, 9]) == 9
    end

    test "handles negatives" do
      assert Descriptive.max_val([-3, -1, 0, 5]) == 5
    end

    test "single element" do
      assert Descriptive.max_val([42]) == 42
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.max_val([]) end
    end
  end

  describe "range_val/1" do
    test "computes max - min" do
      assert Descriptive.range_val([2, 4, 4, 4, 5, 5, 7, 9]) == 7
    end

    test "returns 0 for identical values" do
      assert Descriptive.range_val([5, 5, 5]) == 0
    end

    test "raises on empty list" do
      assert_raise ArgumentError, ~r/empty/, fn -> Descriptive.range_val([]) end
    end
  end

  # ─────────────────────────────────────────────────────────────────
  # Frequency Analysis
  # ─────────────────────────────────────────────────────────────────

  describe "frequency_count/1" do
    test "counts letters case-insensitively" do
      counts = Frequency.frequency_count("Hello!")
      assert Map.get(counts, "H") == 1
      assert Map.get(counts, "E") == 1
      assert Map.get(counts, "L") == 2
      assert Map.get(counts, "O") == 1
    end

    test "ignores non-alphabetic characters" do
      counts = Frequency.frequency_count("123!@#")
      assert map_size(counts) == 0
    end

    test "handles empty string" do
      counts = Frequency.frequency_count("")
      assert map_size(counts) == 0
    end
  end

  describe "frequency_distribution/1" do
    test "converts counts to proportions" do
      dist = Frequency.frequency_distribution("AABB")
      assert_in_delta Map.get(dist, "A"), 0.5, 1.0e-10
      assert_in_delta Map.get(dist, "B"), 0.5, 1.0e-10
    end

    test "proportions sum to 1.0" do
      dist = Frequency.frequency_distribution("HELLO WORLD")
      total = dist |> Map.values() |> Enum.sum()
      assert_in_delta total, 1.0, 1.0e-10
    end

    test "handles empty string" do
      dist = Frequency.frequency_distribution("")
      assert map_size(dist) == 0
    end
  end

  describe "chi_squared/2" do
    test "parity test vector" do
      assert_in_delta Frequency.chi_squared([10, 20, 30], [20, 20, 20]), 10.0, 1.0e-10
    end

    test "returns 0 when observed equals expected" do
      assert_in_delta Frequency.chi_squared([20, 20, 20], [20, 20, 20]), 0.0, 1.0e-10
    end

    test "raises on mismatched lengths" do
      assert_raise ArgumentError, ~r/same length/, fn ->
        Frequency.chi_squared([1, 2], [1])
      end
    end

    test "raises on empty lists" do
      assert_raise ArgumentError, ~r/empty/, fn ->
        Frequency.chi_squared([], [])
      end
    end

    test "raises when expected contains zero" do
      assert_raise ArgumentError, ~r/zero/, fn ->
        Frequency.chi_squared([1], [0])
      end
    end
  end

  describe "chi_squared_text/2" do
    test "returns 0 for perfect match" do
      result = Frequency.chi_squared_text("AAAA", %{"A" => 1.0})
      assert_in_delta result, 0.0, 1.0e-10
    end

    test "returns 0 for empty text" do
      assert Frequency.chi_squared_text("", Cryptanalysis.english_frequencies()) == 0.0
    end

    test "returns positive for non-English text" do
      result = Frequency.chi_squared_text("ZZZZZZZZZZ", Cryptanalysis.english_frequencies())
      assert result > 0
    end
  end

  # ─────────────────────────────────────────────────────────────────
  # Cryptanalysis Helpers
  # ─────────────────────────────────────────────────────────────────

  describe "index_of_coincidence/1" do
    test "parity test vector: AABB" do
      # A=2, B=2, N=4 => IC = (2*1 + 2*1) / (4*3) = 4/12 = 1/3
      assert_in_delta Cryptanalysis.index_of_coincidence("AABB"), 1 / 3, 1.0e-10
    end

    test "returns 1.0 for repeated single letter" do
      assert_in_delta Cryptanalysis.index_of_coincidence("AAAA"), 1.0, 1.0e-10
    end

    test "returns 0 for text shorter than 2 chars" do
      assert Cryptanalysis.index_of_coincidence("A") == 0.0
      assert Cryptanalysis.index_of_coincidence("") == 0.0
    end

    test "returns 0 for uniform alphabet (each letter once)" do
      ic = Cryptanalysis.index_of_coincidence("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
      assert ic == 0.0
    end
  end

  describe "entropy/1" do
    test "maximum entropy for uniform distribution" do
      alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
      assert_in_delta Cryptanalysis.entropy(alphabet), :math.log2(26), 0.01
    end

    test "returns 0 for single letter repeated" do
      assert_in_delta Cryptanalysis.entropy("AAAA"), 0.0, 1.0e-10
    end

    test "returns 1.0 for two equally frequent letters" do
      assert_in_delta Cryptanalysis.entropy("AABB"), 1.0, 1.0e-10
    end

    test "returns 0 for empty string" do
      assert Cryptanalysis.entropy("") == 0.0
    end
  end

  describe "english_frequencies/0" do
    test "has 26 entries" do
      freq = Cryptanalysis.english_frequencies()
      assert map_size(freq) == 26
    end

    test "frequencies sum to approximately 1.0" do
      freq = Cryptanalysis.english_frequencies()
      total = freq |> Map.values() |> Enum.sum()
      assert_in_delta total, 1.0, 0.01
    end

    test "E is the most common letter" do
      freq = Cryptanalysis.english_frequencies()
      assert Map.get(freq, "E") > Map.get(freq, "T")
    end

    test "Z is the least common letter" do
      freq = Cryptanalysis.english_frequencies()
      z_freq = Map.get(freq, "Z")

      Enum.each(freq, fn {letter, letter_freq} ->
        if letter != "Z" do
          assert letter_freq >= z_freq,
            "Expected #{letter} (#{letter_freq}) >= Z (#{z_freq})"
        end
      end)
    end
  end
end
