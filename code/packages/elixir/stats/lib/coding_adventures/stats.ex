# # CodingAdventures.Stats
#
# Statistics, frequency analysis, and cryptanalysis helpers.
#
# This module provides three categories of functions:
#
# 1. **Descriptive statistics** — mean, median, mode, variance,
#    standard deviation, min, max, range.
#
# 2. **Frequency analysis** — letter frequency counting, frequency
#    distributions, chi-squared tests.
#
# 3. **Cryptanalysis helpers** — index of coincidence, Shannon entropy,
#    and standard English letter frequency tables.
#
# All functions are pure (no side effects, no mutation of inputs).

defmodule CodingAdventures.Stats do
  @moduledoc """
  Statistics, frequency analysis, and cryptanalysis helpers.

  ## Submodules

  - `CodingAdventures.Stats.Descriptive` — mean, median, mode, variance, etc.
  - `CodingAdventures.Stats.Frequency` — frequency counts, chi-squared tests
  - `CodingAdventures.Stats.Cryptanalysis` — index of coincidence, entropy
  """
end

# ─────────────────────────────────────────────────────────────────────
# Descriptive Statistics
# ─────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.Stats.Descriptive do
  @moduledoc """
  Descriptive statistics: central tendency and spread.

  All functions take a list of numbers. Empty lists raise `ArgumentError`.
  """

  @doc """
  # Mean (Arithmetic Average)

  The mean answers: "If we spread the total equally among all values,
  what would each value be?"

  ## Formula

      mean = sum(values) / n

  ## Examples

      iex> CodingAdventures.Stats.Descriptive.mean([1, 2, 3, 4, 5])
      3.0

      iex> CodingAdventures.Stats.Descriptive.mean([2, 4, 4, 4, 5, 5, 7, 9])
      5.0
  """
  def mean([]), do: raise(ArgumentError, "Cannot compute mean of an empty list")

  def mean(values) when is_list(values) do
    Enum.sum(values) / length(values)
  end

  @doc """
  # Median

  The median is the "middle" value when data is sorted. Unlike the mean,
  it is robust against outliers. A billionaire walking into a room doesn't
  change the median income much, but it skews the mean enormously.

  ## Algorithm

  1. Sort the values in ascending order.
  2. If the count is odd, return the middle element.
  3. If the count is even, return the average of the two middle elements.

  ## Examples

      iex> CodingAdventures.Stats.Descriptive.median([1, 3, 5])
      3

      iex> CodingAdventures.Stats.Descriptive.median([2, 4, 4, 4, 5, 5, 7, 9])
      4.5
  """
  def median([]), do: raise(ArgumentError, "Cannot compute median of an empty list")

  def median(values) when is_list(values) do
    sorted = Enum.sort(values)
    n = length(sorted)
    mid = div(n, 2)

    if rem(n, 2) != 0 do
      # Odd length: return middle element.
      Enum.at(sorted, mid)
    else
      # Even length: average the two middle elements.
      (Enum.at(sorted, mid - 1) + Enum.at(sorted, mid)) / 2
    end
  end

  @doc """
  # Mode

  The mode is the most frequently occurring value. When multiple values
  share the highest frequency, the first occurrence in the original list
  wins (deterministic tie-breaking).

  ## Examples

      iex> CodingAdventures.Stats.Descriptive.mode([2, 4, 4, 4, 5, 5, 7, 9])
      4

      iex> CodingAdventures.Stats.Descriptive.mode([1, 2, 1, 2, 3])
      1
  """
  def mode([]), do: raise(ArgumentError, "Cannot compute mode of an empty list")

  def mode(values) when is_list(values) do
    # Build a frequency map. We iterate the original list to find the
    # first value with the maximum frequency (preserving insertion order).
    freq_map =
      Enum.reduce(values, %{}, fn val, acc ->
        Map.update(acc, val, 1, &(&1 + 1))
      end)

    max_count = freq_map |> Map.values() |> Enum.max()

    # Return the first value in the original list that has max_count.
    Enum.find(values, fn val -> Map.get(freq_map, val) == max_count end)
  end

  @doc """
  # Variance

  Variance measures how spread out the data is from the mean.

  ## Formula

      variance = sum((x_i - mean)^2) / d

  Where d is:
  - n if `population: true` (the entire population)
  - n-1 if `population: false` (the default — Bessel's correction for samples)

  ## Why n-1? (Bessel's Correction)

  When computing variance from a sample, the sample mean is "pulled toward"
  the data points, systematically underestimating the true spread. Dividing
  by n-1 compensates for this bias.

  ## Examples

      iex> CodingAdventures.Stats.Descriptive.variance([2, 4, 4, 4, 5, 5, 7, 9])
      4.571428571428571

      iex> CodingAdventures.Stats.Descriptive.variance([2, 4, 4, 4, 5, 5, 7, 9], population: true)
      4.0
  """
  def variance(values, opts \\ [])

  def variance([], _opts), do: raise(ArgumentError, "Cannot compute variance of an empty list")

  def variance(values, opts) when is_list(values) do
    population = Keyword.get(opts, :population, false)
    n = length(values)

    if not population and n < 2 do
      raise ArgumentError, "Sample variance requires at least 2 values"
    end

    avg = mean(values)

    sum_sq_dev =
      Enum.reduce(values, 0.0, fn val, acc ->
        acc + :math.pow(val - avg, 2)
      end)

    divisor = if population, do: n, else: n - 1
    sum_sq_dev / divisor
  end

  @doc """
  # Standard Deviation

  The standard deviation is the square root of the variance. While variance
  is in "squared units," standard deviation brings us back to the original
  units, making it more interpretable.

  ## The 68-95-99.7 Rule

  For normally distributed data:
  - ~68% of values fall within 1 standard deviation of the mean
  - ~95% fall within 2 standard deviations
  - ~99.7% fall within 3 standard deviations
  """
  def standard_deviation(values, opts \\ []) do
    :math.sqrt(variance(values, opts))
  end

  @doc """
  # Minimum

  Returns the smallest value in a list.
  """
  def min_val([]), do: raise(ArgumentError, "Cannot compute min of an empty list")
  def min_val(values) when is_list(values), do: Enum.min(values)

  @doc """
  # Maximum

  Returns the largest value in a list.
  """
  def max_val([]), do: raise(ArgumentError, "Cannot compute max of an empty list")
  def max_val(values) when is_list(values), do: Enum.max(values)

  @doc """
  # Range

  The range is the simplest measure of spread: max - min.

  ## Example

      iex> CodingAdventures.Stats.Descriptive.range_val([2, 4, 4, 4, 5, 5, 7, 9])
      7
  """
  def range_val([]), do: raise(ArgumentError, "Cannot compute range of an empty list")

  def range_val(values) when is_list(values) do
    max_val(values) - min_val(values)
  end
end

# ─────────────────────────────────────────────────────────────────────
# Frequency Analysis
# ─────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.Stats.Frequency do
  @moduledoc """
  Frequency analysis: letter counting, frequency distributions,
  and chi-squared tests.

  All text analysis functions are case-insensitive and only count
  A-Z characters.
  """

  @doc """
  # Frequency Count

  Counts how many times each letter (A-Z) appears in the text.
  Non-alphabetic characters are ignored. Case-insensitive.

  ## Example

      iex> CodingAdventures.Stats.Frequency.frequency_count("Hello!")
      %{"H" => 1, "E" => 1, "L" => 2, "O" => 1}
  """
  def frequency_count(text) when is_binary(text) do
    text
    |> String.upcase()
    |> String.graphemes()
    |> Enum.filter(fn ch -> ch >= "A" and ch <= "Z" end)
    |> Enum.reduce(%{}, fn ch, acc ->
      Map.update(acc, ch, 1, &(&1 + 1))
    end)
  end

  @doc """
  # Frequency Distribution

  Converts raw letter counts into proportions (0.0 to 1.0).
  Normalizes the data so texts of different lengths can be compared.

  ## Formula

      proportion(letter) = count(letter) / total_letter_count
  """
  def frequency_distribution(text) when is_binary(text) do
    counts = frequency_count(text)
    total = counts |> Map.values() |> Enum.sum()

    if total == 0 do
      %{}
    else
      Map.new(counts, fn {letter, count} -> {letter, count / total} end)
    end
  end

  @doc """
  # Chi-Squared Statistic

  Measures how well observed data matches expected data. A value of 0
  means perfect agreement; larger values indicate greater divergence.

  ## Formula

      chi2 = sum( (observed_i - expected_i)^2 / expected_i )

  ## Example

      iex> CodingAdventures.Stats.Frequency.chi_squared([10, 20, 30], [20, 20, 20])
      10.0
  """
  def chi_squared(observed, expected) when is_list(observed) and is_list(expected) do
    if length(observed) != length(expected) do
      raise ArgumentError, "Observed and expected lists must have the same length"
    end

    if observed == [] do
      raise ArgumentError, "Lists must not be empty"
    end

    Enum.zip(observed, expected)
    |> Enum.with_index()
    |> Enum.reduce(0.0, fn {{obs, exp}, idx}, acc ->
      if exp == 0 do
        raise ArgumentError, "Expected value at index #{idx} must not be zero"
      end

      diff = obs - exp
      acc + diff * diff / exp
    end)
  end

  @doc """
  # Chi-Squared for Text

  Convenience function that computes chi-squared of a text against
  an expected frequency table (like english_frequencies).

  This is how you break a Caesar cipher: try all 26 shifts, compute
  chi-squared for each, and pick the shift with the lowest value.
  """
  def chi_squared_text(text, expected_freq) when is_binary(text) and is_map(expected_freq) do
    counts = frequency_count(text)
    total = counts |> Map.values() |> Enum.sum()

    if total == 0 do
      0.0
    else
      # For each letter A-Z, compute (observed - expected)^2 / expected.
      Enum.reduce(?A..?Z, 0.0, fn code, acc ->
        letter = <<code::utf8>>
        observed = Map.get(counts, letter, 0)
        expected = total * Map.get(expected_freq, letter, 0.0)

        if expected > 0.0 do
          diff = observed - expected
          acc + diff * diff / expected
        else
          acc
        end
      end)
    end
  end
end

# ─────────────────────────────────────────────────────────────────────
# Cryptanalysis Helpers
# ─────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.Stats.Cryptanalysis do
  @moduledoc """
  Cryptanalysis helpers: index of coincidence, Shannon entropy,
  and English frequency tables.
  """

  alias CodingAdventures.Stats.Frequency

  @doc """
  # English Letter Frequencies

  Standard frequencies of each letter (A-Z) in typical English text.
  The mnemonic "ETAOIN SHRDLU" captures the most common letters.
  """
  def english_frequencies do
    %{
      "A" => 0.08167, "B" => 0.01492, "C" => 0.02782, "D" => 0.04253,
      "E" => 0.12702, "F" => 0.02228, "G" => 0.02015, "H" => 0.06094,
      "I" => 0.06966, "J" => 0.00153, "K" => 0.00772, "L" => 0.04025,
      "M" => 0.02406, "N" => 0.06749, "O" => 0.07507, "P" => 0.01929,
      "Q" => 0.00095, "R" => 0.05987, "S" => 0.06327, "T" => 0.09056,
      "U" => 0.02758, "V" => 0.00978, "W" => 0.02360, "X" => 0.00150,
      "Y" => 0.01974, "Z" => 0.00074
    }
  end

  @doc """
  # Index of Coincidence (IC)

  The IC measures the probability that two randomly chosen letters from
  a text are the same.

  ## Formula

      IC = sum(n_i * (n_i - 1)) / (N * (N - 1))

  ## Expected Values

  | Text Type        | IC Value          |
  |-----------------|-------------------|
  | English text     | ~0.0667           |
  | Random (uniform) | ~0.0385 (= 1/26) |

  ## Example

      iex> CodingAdventures.Stats.Cryptanalysis.index_of_coincidence("AABB")
      0.3333333333333333
  """
  def index_of_coincidence(text) when is_binary(text) do
    counts = Frequency.frequency_count(text)

    # N = total alphabetic characters.
    n = counts |> Map.values() |> Enum.sum()

    if n < 2 do
      0.0
    else
      # Numerator: sum of n_i * (n_i - 1).
      numerator =
        counts
        |> Map.values()
        |> Enum.reduce(0, fn c, acc -> acc + c * (c - 1) end)

      # Denominator: N * (N - 1).
      denominator = n * (n - 1)

      numerator / denominator
    end
  end

  @doc """
  # Shannon Entropy

  Shannon entropy measures the average "information content" per symbol.
  It answers: "How many bits do we need, on average, to encode each symbol?"

  ## Formula

      H = -sum(p_i * log2(p_i))

  ## Expected Values

  | Distribution      | Entropy            |
  |------------------|--------------------|
  | Uniform 26 chars  | log2(26) ~ 4.700   |
  | English text      | ~4.0 - 4.5         |
  | Single letter     | 0.0                |
  """
  def entropy(text) when is_binary(text) do
    dist = Frequency.frequency_distribution(text)

    if map_size(dist) == 0 do
      0.0
    else
      dist
      |> Map.values()
      |> Enum.reduce(0.0, fn p, acc ->
        if p > 0 do
          acc - p * :math.log2(p)
        else
          acc
        end
      end)
    end
  end
end
