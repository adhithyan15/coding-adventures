# frozen_string_literal: true

# Cryptanalysis Tools for the Vigenere Cipher
# =============================================
#
# Breaking the "unbreakable" cipher uses two statistical techniques:
#
# Phase 1: Find Key Length (Index of Coincidence)
# ------------------------------------------------
# The IC measures how likely two random letters from a text are the same.
#   English: IC ~ 0.0667 (uneven distribution: lots of E, T, A)
#   Random:  IC ~ 0.0385 (uniform: 1/26)
#
# When we guess the CORRECT key length k and group every k-th letter,
# each group was encrypted with a single Caesar shift. The IC of each
# group will be close to English. Wrong guesses give lower IC.
#
# Phase 2: Find Each Key Letter (Chi-Squared Test)
# --------------------------------------------------
# For each group (one per key position), try all 26 shifts and pick the
# one that produces letter frequencies closest to English (lowest chi-squared).

module CodingAdventures
  module VigenereCipher
    # Standard English letter frequencies (A-Z).
    # E is most common (~12.7%), Z is rarest (~0.07%).
    ENGLISH_FREQUENCIES = [
      0.08167, # A
      0.01492, # B
      0.02782, # C
      0.04253, # D
      0.12702, # E
      0.02228, # F
      0.02015, # G
      0.06094, # H
      0.06966, # I
      0.00153, # J
      0.00772, # K
      0.04025, # L
      0.02406, # M
      0.06749, # N
      0.07507, # O
      0.01929, # P
      0.00095, # Q
      0.05987, # R
      0.06327, # S
      0.09056, # T
      0.02758, # U
      0.00978, # V
      0.02360, # W
      0.00150, # X
      0.01974, # Y
      0.00074  # Z
    ].freeze

    # Estimate key length using Index of Coincidence analysis.
    #
    # For each candidate length k (2..max_length), splits ciphertext into
    # k groups and averages their IC. The k with the highest average IC
    # is most likely the correct key length.
    #
    # @param ciphertext [String] the encrypted text
    # @param max_length [Integer] maximum key length to try (default 20)
    # @return [Integer] the estimated key length
    def self.find_key_length(ciphertext, max_length = 20)
      letters = ciphertext.gsub(/[^A-Za-z]/, "").upcase

      # Compute average IC for each candidate key length
      ic_scores = (2..max_length).map do |k|
        # Split into k groups: group i gets letters at positions i, i+k, i+2k, ...
        groups = Array.new(k) { +"" }
        letters.chars.each_with_index do |ch, i|
          groups[i % k] << ch
        end

        # Average IC across all groups
        total_ic = groups.sum { |g| index_of_coincidence(g) }
        avg_ic = total_ic / k.to_f
        [k, avg_ic]
      end

      # Find the best IC value
      best_ic = ic_scores.map(&:last).max

      # Among all key lengths within 5% of the best IC, choose the shortest.
      # This avoids selecting multiples of the true key length (e.g., 12
      # instead of 6), since multiples also produce high IC.
      threshold = best_ic * 0.95
      candidates = ic_scores.select { |_, ic| ic >= threshold }
      candidates.min_by(&:first).first
    end

    # Find each key letter using chi-squared frequency analysis.
    #
    # @param ciphertext [String] the encrypted text
    # @param key_length [Integer] the known or estimated key length
    # @return [String] the recovered key (uppercase)
    def self.find_key(ciphertext, key_length)
      letters = ciphertext.gsub(/[^A-Za-z]/, "").upcase

      key_chars = (0...key_length).map do |pos|
        # Extract every key_length-th letter starting at pos
        group = (pos...letters.length).step(key_length).map { |i| letters[i] }

        # Try all 26 shifts, find lowest chi-squared
        best_shift = 0
        best_chi2 = Float::INFINITY

        26.times do |shift|
          counts = Array.new(26, 0)
          group.each do |ch|
            decrypted = (ch.ord - "A".ord - shift + 26) % 26
            counts[decrypted] += 1
          end

          chi2 = chi_squared(counts, group.length)
          if chi2 < best_chi2
            best_chi2 = chi2
            best_shift = shift
          end
        end

        ("A".ord + best_shift).chr
      end

      key_chars.join
    end

    # Automatically break a Vigenere cipher: find key and decrypt.
    #
    # Combines find_key_length + find_key + decrypt.
    # Works best on ciphertexts of 200+ characters.
    #
    # @param ciphertext [String] the encrypted text
    # @return [Array(String, String)] [recovered_key, decrypted_plaintext]
    def self.break_cipher(ciphertext)
      key_length = find_key_length(ciphertext)
      key = find_key(ciphertext, key_length)
      plaintext = decrypt(ciphertext, key)
      [key, plaintext]
    end

    # Calculate Index of Coincidence for a string of uppercase letters.
    #
    # IC = sum(f_i * (f_i - 1)) / (N * (N - 1))
    #
    # @param text [String] uppercase letters only
    # @return [Float] the IC value
    def self.index_of_coincidence(text)
      n = text.length
      return 0.0 if n < 2

      counts = Array.new(26, 0)
      text.each_char { |ch| counts[ch.ord - "A".ord] += 1 }

      numerator = counts.sum { |f| f * (f - 1) }
      numerator.to_f / (n * (n - 1))
    end
    private_class_method :index_of_coincidence

    # Calculate chi-squared statistic against English frequencies.
    #
    # @param counts [Array<Integer>] 26-element array of letter counts
    # @param total [Integer] total number of letters
    # @return [Float] chi-squared value (lower = better fit)
    def self.chi_squared(counts, total)
      return Float::INFINITY if total == 0

      chi2 = 0.0
      26.times do |i|
        expected = total * ENGLISH_FREQUENCIES[i]
        chi2 += ((counts[i] - expected)**2) / expected if expected > 0
      end
      chi2
    end
    private_class_method :chi_squared
  end
end
