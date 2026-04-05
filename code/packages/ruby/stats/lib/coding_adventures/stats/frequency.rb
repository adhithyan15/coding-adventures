# frozen_string_literal: true

# Frequency analysis -- tools for counting and comparing letter distributions.
#
# These functions are essential for classical cryptanalysis. When you intercept
# an encrypted message, the first thing you do is count how often each letter
# appears. By comparing the frequency distribution of a ciphertext against
# known English frequencies, you can determine what kind of cipher was used.
#
# Key Concepts
# ============
#
# Frequency count: Raw count of each letter (A-Z, case-insensitive).
# Frequency distribution: Each count divided by the total letter count.
# Chi-squared: Measures how well an observed distribution matches expected.

module CodingAdventures
  module Stats
    module Frequency
      module_function

      # Count occurrences of each letter (A-Z) in the text, case-insensitive.
      #
      # Non-alphabetic characters are silently ignored. The returned hash maps
      # uppercase letter strings to their counts.
      #
      # Example:
      #   frequency_count("Hello") => {"H"=>1, "E"=>1, "L"=>2, "O"=>1}
      def frequency_count(text)
        counts = {}
        text.upcase.each_char do |ch|
          next unless ch.match?(/[A-Z]/)

          counts[ch] = (counts[ch] || 0) + 1
        end
        counts
      end

      # Proportion of each letter (A-Z) in the text.
      #
      # Each proportion is count / total_letters. The proportions sum to
      # approximately 1.0.
      def frequency_distribution(text)
        counts = frequency_count(text)
        total = counts.values.sum
        return {} if total == 0

        counts.transform_values { |c| c.to_f / total }
      end

      # Chi-squared statistic for two parallel arrays of values.
      #
      # Formula: chi_squared = Sum((O_i - E_i)^2 / E_i)
      #
      # Both arrays must have the same length.
      #
      # Example:
      #   chi_squared([10, 20, 30], [20, 20, 20]) => 10.0
      def chi_squared(observed, expected)
        unless observed.length == expected.length
          raise ArgumentError, "observed and expected must have the same length"
        end

        observed.zip(expected).sum do |o, e|
          ((o - e)**2).to_f / e
        end
      end

      # Chi-squared comparing text letter frequencies to expected frequencies.
      #
      # Steps:
      # 1. Count letters in text (A-Z, case-insensitive).
      # 2. For each letter in expected_freq, compute expected_count = freq * total.
      # 3. Compute chi-squared over all letters.
      def chi_squared_text(text, expected_freq)
        counts = frequency_count(text)
        total = counts.values.sum
        return 0.0 if total == 0

        expected_freq.sum do |letter, freq|
          observed_count = (counts[letter.upcase] || 0).to_f
          expected_count = freq * total
          if expected_count > 0
            ((observed_count - expected_count)**2) / expected_count
          else
            0.0
          end
        end
      end
    end
  end
end
