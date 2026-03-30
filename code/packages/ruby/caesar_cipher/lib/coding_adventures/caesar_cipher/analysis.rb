# frozen_string_literal: true

# ============================================================================
# CodingAdventures::CaesarCipher — Cryptanalysis Tools
# ============================================================================
#
# The Caesar cipher is famously easy to break.  With only 25 possible
# non-trivial shifts (shift = 0 is the identity), an attacker can simply
# try all of them ("brute force") and look for readable text.
#
# But we can do better!  By analyzing the frequency of letters in the
# ciphertext and comparing them to the known frequency distribution of
# letters in English, we can automatically identify the most likely shift
# without any human inspection.  This is called **frequency analysis**.
#
# ---------------------------------------------------------------------------
# A brief history of frequency analysis
# ---------------------------------------------------------------------------
#
# The technique was first described by the Arab polymath Al-Kindi in the
# 9th century, in his work "A Manuscript on Deciphering Cryptographic
# Messages."  He observed that in any language, certain letters appear more
# often than others.  In English:
#
#   - 'E' is the most common letter (~12.7% of all letters)
#   - 'T' is second (~9.1%)
#   - 'A' is third (~8.2%)
#   - The least common letters are 'Z' (~0.07%) and 'Q' (~0.10%)
#
# If we encrypt a long English text with a Caesar cipher (shift = 5), then
# the most common letter in the ciphertext won't be 'E' anymore — it will
# be 'J' (because E + 5 = J).  By finding the most common letter in the
# ciphertext and seeing how far it is from 'E', we can guess the shift.
#
# But single-letter frequency alone can be fooled by short texts.  A more
# robust approach uses the **chi-squared statistic** to compare the entire
# frequency distribution of the ciphertext against the expected English
# distribution.  That's what our `frequency_analysis` method does.
#
# ---------------------------------------------------------------------------
# The chi-squared statistic
# ---------------------------------------------------------------------------
#
# The chi-squared (chi^2) statistic measures how well an observed
# distribution matches an expected distribution.  The formula is:
#
#              26
#   chi^2  =  SUM  (observed_i - expected_i)^2 / expected_i
#             i=1
#
# Where:
#   - observed_i  = count of the i-th letter in the decrypted text
#   - expected_i  = expected count based on English frequencies
#                 = total_letters * english_frequency[i]
#
# A LOWER chi^2 means the distribution is CLOSER to English.  So we try
# all 26 shifts, compute chi^2 for each, and pick the shift with the
# lowest score.
#
# ============================================================================

module CodingAdventures
  module CaesarCipher
    # -----------------------------------------------------------------------
    # English letter frequency table
    # -----------------------------------------------------------------------
    #
    # These values represent the relative frequency of each letter in a
    # large corpus of English text.  They are expressed as proportions
    # (decimals that sum to approximately 1.0).
    #
    # Source: Various analyses of English text corpora.  The exact numbers
    # vary slightly between sources, but the relative ordering is consistent.
    #
    # The most frequent letters spell out the mnemonic "ETAOIN SHRDLU",
    # which was once a common sight on Linotype machines.
    #
    ENGLISH_FREQUENCIES = {
      "a" => 0.08167, "b" => 0.01492, "c" => 0.02782, "d" => 0.04253,
      "e" => 0.12702, "f" => 0.02228, "g" => 0.02015, "h" => 0.06094,
      "i" => 0.06966, "j" => 0.00153, "k" => 0.00772, "l" => 0.04025,
      "m" => 0.02406, "n" => 0.06749, "o" => 0.07507, "p" => 0.01929,
      "q" => 0.00095, "r" => 0.05987, "s" => 0.06327, "t" => 0.09056,
      "u" => 0.02758, "v" => 0.00978, "w" => 0.02360, "x" => 0.00150,
      "y" => 0.01974, "z" => 0.00074
    }.freeze

    # -----------------------------------------------------------------------
    # brute_force(ciphertext) -> Array<[Integer, String]>
    # -----------------------------------------------------------------------
    #
    # Tries all 25 non-trivial shifts (1 through 25) and returns each
    # possible decryption.  Shift 0 is omitted because it would just
    # return the ciphertext unchanged.
    #
    # This is the simplest attack against the Caesar cipher.  A human can
    # scan the 25 results and quickly spot which one is readable English.
    # For automated cracking, use `frequency_analysis` instead.
    #
    # Parameters:
    #   ciphertext [String] — the encrypted text to crack
    #
    # Returns:
    #   An Array of 25 pairs `[shift, decrypted_text]`, where `shift` is
    #   the shift value (1..25) and `decrypted_text` is what the ciphertext
    #   decodes to under that shift.
    #
    # Example:
    #   results = brute_force("KHOOR")
    #   results.find { |shift, text| text == "HELLO" }
    #   #=> [3, "HELLO"]
    #
    def self.brute_force(ciphertext)
      # Try every shift from 1 to 25 (skip 0 — that's the identity).
      # For each shift, decrypt the ciphertext and collect the result.
      (1..25).map { |shift|
        [shift, decrypt(ciphertext, shift)]
      }
    end

    # -----------------------------------------------------------------------
    # frequency_analysis(ciphertext) -> [Integer, String]
    # -----------------------------------------------------------------------
    #
    # Uses the chi-squared statistic to automatically determine the most
    # likely shift used to encrypt the ciphertext.
    #
    # Algorithm:
    #
    #   1. For each possible shift (0..25):
    #      a. Decrypt the ciphertext using that shift.
    #      b. Count the frequency of each letter in the decrypted text.
    #      c. Compute the chi-squared statistic comparing the observed
    #         frequencies against the expected English frequencies.
    #
    #   2. Return the shift with the LOWEST chi-squared value, along with
    #      the decrypted text for that shift.
    #
    # Parameters:
    #   ciphertext [String] — the encrypted text to analyze
    #
    # Returns:
    #   A pair `[best_shift, decrypted_text]` where `best_shift` is the
    #   shift value (0..25) that produces a frequency distribution closest
    #   to English, and `decrypted_text` is the result of decrypting with
    #   that shift.
    #
    # Edge cases:
    #   - If the ciphertext contains no letters, returns `[0, ciphertext]`
    #     because there's nothing to analyze.
    #   - Works best on longer texts (50+ characters).  Short texts may
    #     not have enough letter diversity for reliable analysis.
    #
    # Example:
    #   shift, plaintext = frequency_analysis("KHOOR ZRUOG")
    #   shift     #=> 3
    #   plaintext #=> "HELLO WORLD"
    #
    def self.frequency_analysis(ciphertext)
      # Count the total number of letters in the ciphertext.  We need this
      # to compute expected counts.  Non-letter characters are ignored.
      total_letters = ciphertext.count("a-zA-Z")

      # Edge case: if there are no letters, we can't do frequency analysis.
      # Return shift 0 (identity) since we have no information.
      return [0, ciphertext] if total_letters.zero?

      # Try all 26 shifts (0..25) and compute the chi-squared statistic
      # for each one.
      best_shift = 0
      best_chi_squared = Float::INFINITY
      best_plaintext = ciphertext

      (0..25).each do |shift|
        # Decrypt with this candidate shift.
        candidate = decrypt(ciphertext, shift)

        # Count how many times each lowercase letter appears.  We convert
        # to lowercase first so 'A' and 'a' are counted together.
        observed_counts = Hash.new(0)
        candidate.each_char do |char|
          lower = char.downcase
          observed_counts[lower] += 1 if lower.match?(/[a-z]/)
        end

        # Compute the chi-squared statistic.
        #
        # For each letter of the alphabet:
        #   expected = total_letters * frequency_of_that_letter_in_English
        #   observed = how many times that letter appears in our candidate
        #   chi^2   += (observed - expected)^2 / expected
        #
        # We skip letters with zero expected count to avoid division by
        # zero, but in practice every English letter has a non-zero
        # frequency in our table.
        chi_squared = 0.0

        ENGLISH_FREQUENCIES.each do |letter, frequency|
          expected = total_letters * frequency
          observed = observed_counts[letter] || 0
          chi_squared += ((observed - expected)**2) / expected
        end

        # Keep track of the shift with the lowest chi-squared value.
        # Lower chi-squared = better fit to English.
        if chi_squared < best_chi_squared
          best_chi_squared = chi_squared
          best_shift = shift
          best_plaintext = candidate
        end
      end

      [best_shift, best_plaintext]
    end
  end
end
