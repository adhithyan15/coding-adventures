# frozen_string_literal: true

# The Vigenere Cipher: A Polyalphabetic Substitution
# ====================================================
#
# The Vigenere cipher shifts each letter by a different amount determined by
# a repeating keyword. Unlike Caesar (single shift), each position uses a
# different shift based on the keyword letter (A=0, B=1, ..., Z=25).
#
# Example: encrypt("ATTACKATDAWN", "LEMON")
#
#     Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
#     Keyword:    L  E  M  O  N  L  E  M  O  N  L  E
#     Shift:      11 4  12 14 13 11 4  12 14 13 11 4
#     Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
#
# Non-alphabetic characters pass through unchanged and do NOT advance
# the keyword position. Case is preserved.
#
# Decryption reverses the process by shifting BACKWARD.

module CodingAdventures
  module VigenereCipher
    # Encrypt plaintext using the Vigenere cipher.
    #
    # @param plaintext [String] text to encrypt (any characters allowed)
    # @param key [String] keyword (non-empty, alphabetic only)
    # @return [String] ciphertext with case preserved
    # @raise [ArgumentError] if key is empty or contains non-alpha characters
    def self.encrypt(plaintext, key)
      validate_key!(key)

      key_shifts = key.upcase.chars.map { |c| c.ord - "A".ord }
      key_len = key_shifts.length
      key_index = 0

      plaintext.chars.map do |ch|
        if ch =~ /[A-Za-z]/
          base = ch =~ /[A-Z]/ ? "A".ord : "a".ord
          shifted = (ch.ord - base + key_shifts[key_index % key_len]) % 26
          key_index += 1
          (base + shifted).chr
        else
          ch
        end
      end.join
    end

    # Decrypt ciphertext that was encrypted with the Vigenere cipher.
    #
    # @param ciphertext [String] text to decrypt
    # @param key [String] keyword used during encryption
    # @return [String] decrypted plaintext with case preserved
    # @raise [ArgumentError] if key is empty or contains non-alpha characters
    def self.decrypt(ciphertext, key)
      validate_key!(key)

      key_shifts = key.upcase.chars.map { |c| c.ord - "A".ord }
      key_len = key_shifts.length
      key_index = 0

      ciphertext.chars.map do |ch|
        if ch =~ /[A-Za-z]/
          base = ch =~ /[A-Z]/ ? "A".ord : "a".ord
          # Subtract shift and add 26 to prevent negative before mod
          shifted = (ch.ord - base - key_shifts[key_index % key_len] + 26) % 26
          key_index += 1
          (base + shifted).chr
        else
          ch
        end
      end.join
    end

    # Validate that the key is non-empty and alphabetic only.
    #
    # @param key [String] the key to validate
    # @raise [ArgumentError] if key is invalid
    def self.validate_key!(key)
      raise ArgumentError, "Key must not be empty" if key.empty?
      raise ArgumentError, "Key must contain only letters, got #{key.inspect}" unless key.match?(/\A[A-Za-z]+\z/)
    end
    private_class_method :validate_key!
  end
end
