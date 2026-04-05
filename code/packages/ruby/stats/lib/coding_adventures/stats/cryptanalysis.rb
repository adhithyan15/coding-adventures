# frozen_string_literal: true

# Cryptanalysis helpers -- tools for breaking classical ciphers.
#
# Index of Coincidence (IC)
# =========================
# The IC measures the probability that two randomly chosen letters from a text
# are the same. Invented by William Friedman in 1922.
#
#   IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))
#
# Key values:
#   English text:  IC ~ 0.0667
#   Random text:   IC ~ 0.0385 (1/26)
#
# Shannon Entropy
# ===============
# Measures the information content (or "surprise") in a text.
#
#   H = -Sum(p_i * log2(p_i))
#
# Maximum for 26 letters: log2(26) ~ 4.700 bits.

module CodingAdventures
  module Stats
    module Cryptanalysis
      module_function

      # Standard English letter frequencies (Lewand, 2000).
      # Keys are uppercase single-character strings.
      ENGLISH_FREQUENCIES = {
        "A" => 0.08167, "B" => 0.01492, "C" => 0.02782, "D" => 0.04253,
        "E" => 0.12702, "F" => 0.02228, "G" => 0.02015, "H" => 0.06094,
        "I" => 0.06966, "J" => 0.00153, "K" => 0.00772, "L" => 0.04025,
        "M" => 0.02406, "N" => 0.06749, "O" => 0.07507, "P" => 0.01929,
        "Q" => 0.00095, "R" => 0.05987, "S" => 0.06327, "T" => 0.09056,
        "U" => 0.02758, "V" => 0.00978, "W" => 0.02360, "X" => 0.00150,
        "Y" => 0.01974, "Z" => 0.00074
      }.freeze

      # Index of Coincidence: probability that two random letters match.
      #
      # Formula: IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))
      #
      # Returns 0.0 for texts with fewer than 2 letters.
      #
      # Worked example for "AABB":
      #   counts: A=2, B=2, N=4
      #   numerator = 2*1 + 2*1 = 4
      #   denominator = 4*3 = 12
      #   IC = 4/12 = 0.333...
      def index_of_coincidence(text)
        counts = Frequency.frequency_count(text)
        n = counts.values.sum

        return 0.0 if n < 2

        # Numerator: Sum of n_i * (n_i - 1) for each letter.
        numerator = counts.values.sum { |c| c * (c - 1) }

        # Denominator: N * (N - 1).
        denominator = n * (n - 1)

        numerator.to_f / denominator
      end

      # Shannon entropy of the letter distribution in bits.
      #
      # Formula: H = -Sum(p_i * log2(p_i))
      #
      # Returns 0.0 for empty text.
      def entropy(text)
        counts = Frequency.frequency_count(text)
        total = counts.values.sum

        return 0.0 if total == 0

        h = 0.0
        counts.each_value do |count|
          next unless count > 0

          p = count.to_f / total
          h -= p * Math.log2(p)
        end
        h
      end
    end
  end
end
