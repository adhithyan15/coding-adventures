# frozen_string_literal: true

# ============================================================================
# CodingAdventures::CaesarCipher — Encryption & Decryption
# ============================================================================
#
# This file implements the core Caesar cipher operations: encrypting a
# plaintext message, decrypting a ciphertext message, and the special-case
# ROT13 transform.
#
# ---------------------------------------------------------------------------
# How the shift works — a visual walkthrough
# ---------------------------------------------------------------------------
#
# The English alphabet has 26 letters.  We can number them 0-25:
#
#   A=0  B=1  C=2  D=3  E=4  F=5  G=6  H=7  I=8  J=9  K=10 L=11 M=12
#   N=13 O=14 P=15 Q=16 R=17 S=18 T=19 U=20 V=21 W=22 X=23 Y=24 Z=25
#
# To encrypt a letter with shift `s`:
#
#   1. Convert the letter to its number:        'H' -> 7
#   2. Add the shift:                           7 + 3 = 10
#   3. Wrap around using modulo 26:             10 % 26 = 10
#   4. Convert back to a letter:                10 -> 'K'
#
# Wrapping example (shift = 3):
#
#   'X' -> 23 -> 23 + 3 = 26 -> 26 % 26 = 0 -> 'A'
#   'Y' -> 24 -> 24 + 3 = 27 -> 27 % 26 = 1 -> 'B'
#   'Z' -> 25 -> 25 + 3 = 28 -> 28 % 26 = 2 -> 'C'
#
# To decrypt, we simply shift in the opposite direction (subtract instead
# of add).  In our implementation we reuse `encrypt` with a negated shift,
# which is mathematically equivalent.
#
# ---------------------------------------------------------------------------
# What about non-letter characters?
# ---------------------------------------------------------------------------
#
# The Caesar cipher only operates on letters.  Digits, spaces, punctuation,
# and any other characters pass through unchanged.  This is historically
# accurate — Caesar's original cipher only concerned itself with the Latin
# alphabet.
#
# ---------------------------------------------------------------------------
# Case preservation
# ---------------------------------------------------------------------------
#
# We preserve the case of each letter.  If the input letter is uppercase,
# the output letter is uppercase; likewise for lowercase.  We detect the
# case by checking which range the character's ASCII code point falls in:
#
#   Uppercase: 'A'.ord (65) .. 'Z'.ord (90)
#   Lowercase: 'a'.ord (97) .. 'z'.ord (122)
#
# ============================================================================

module CodingAdventures
  module CaesarCipher
    # The number of letters in the English alphabet.  We use this as the
    # modulus for wrapping shifts around the alphabet.
    ALPHABET_SIZE = 26

    # The ASCII code point for uppercase 'A'.  We subtract this from an
    # uppercase letter's code point to get its 0-based position in the
    # alphabet (A=0, B=1, ..., Z=25).
    UPPER_A = "A".ord # 65

    # The ASCII code point for lowercase 'a'.  Same idea as UPPER_A but
    # for lowercase letters.
    LOWER_A = "a".ord # 97

    # ----------------------------------------------------------------------
    # encrypt(text, shift) -> String
    # ----------------------------------------------------------------------
    #
    # Encrypts `text` using the Caesar cipher with the given `shift`.
    #
    # Parameters:
    #   text  [String]  — the plaintext message to encrypt
    #   shift [Integer] — how many positions to shift each letter (can be
    #                      negative, and values outside 0..25 are handled
    #                      correctly via modular arithmetic)
    #
    # Returns:
    #   A new String with every letter shifted, non-letters unchanged.
    #
    # Examples:
    #   encrypt("HELLO", 3)        #=> "KHOOR"
    #   encrypt("hello", 3)        #=> "khoor"
    #   encrypt("Hello, World!", 3) #=> "Khoor, Zruog!"
    #   encrypt("ABC", -1)         #=> "ZAB"
    #   encrypt("xyz", 3)          #=> "abc"
    #
    # How it works, step by step:
    #
    #   1. We normalize the shift to 0..25 using `shift % 26`.  This
    #      handles negative shifts and shifts larger than 26 gracefully.
    #      For example, a shift of -1 becomes 25, and a shift of 29
    #      becomes 3.
    #
    #   2. We iterate over each character in the text using `.chars.map`.
    #      For each character:
    #
    #      a. If it's an uppercase letter (A-Z), we:
    #         - Find its 0-based position: `char.ord - UPPER_A`
    #         - Add the shift and wrap: `(position + shift) % 26`
    #         - Convert back to a character: `(UPPER_A + new_position).chr`
    #
    #      b. If it's a lowercase letter (a-z), same thing but using
    #         LOWER_A as the base.
    #
    #      c. Otherwise, we pass the character through unchanged.
    #
    #   3. We join all the characters back into a single string.
    #
    def self.encrypt(text, shift)
      # Normalize shift to 0..25.  Ruby's modulo always returns a
      # non-negative result when the divisor is positive, so
      # (-1) % 26 == 25, which is exactly what we want.
      normalized_shift = shift % ALPHABET_SIZE

      text.chars.map { |char|
        case char
        when "A".."Z"
          # Uppercase letter: shift within the uppercase range.
          #
          #   position     = distance from 'A' (0-25)
          #   new_position = shifted position, wrapped around
          #   result       = new letter as a character
          position = char.ord - UPPER_A
          new_position = (position + normalized_shift) % ALPHABET_SIZE
          (UPPER_A + new_position).chr

        when "a".."z"
          # Lowercase letter: same algorithm, different base.
          position = char.ord - LOWER_A
          new_position = (position + normalized_shift) % ALPHABET_SIZE
          (LOWER_A + new_position).chr

        else
          # Non-alphabetic character: pass through unchanged.
          # This includes digits, spaces, punctuation, Unicode, etc.
          char
        end
      }.join
    end

    # ----------------------------------------------------------------------
    # decrypt(text, shift) -> String
    # ----------------------------------------------------------------------
    #
    # Decrypts `text` that was encrypted with the given `shift`.
    #
    # Decryption is the inverse of encryption.  If we encrypted by shifting
    # forward by `s`, we decrypt by shifting backward by `s`.  Shifting
    # backward by `s` is the same as shifting forward by `26 - s` (or
    # equivalently, shifting by `-s`).
    #
    # We simply delegate to `encrypt` with a negated shift, which is both
    # elegant and avoids code duplication.
    #
    # Parameters:
    #   text  [String]  — the ciphertext to decrypt
    #   shift [Integer] — the shift that was used during encryption
    #
    # Returns:
    #   The original plaintext String.
    #
    # Examples:
    #   decrypt("KHOOR", 3)         #=> "HELLO"
    #   decrypt("khoor", 3)         #=> "hello"
    #   decrypt("Khoor, Zruog!", 3) #=> "Hello, World!"
    #
    def self.decrypt(text, shift)
      encrypt(text, -shift)
    end

    # ----------------------------------------------------------------------
    # rot13(text) -> String
    # ----------------------------------------------------------------------
    #
    # ROT13 ("rotate by 13 places") is a special case of the Caesar cipher
    # where the shift is exactly 13 — half the alphabet.
    #
    # What makes ROT13 special?
    #
    # Because the English alphabet has 26 letters, and 13 is exactly half
    # of 26, applying ROT13 twice returns the original text:
    #
    #   rot13(rot13("HELLO")) == "HELLO"
    #
    # This means the same function encrypts AND decrypts!  This property
    # made ROT13 popular on Usenet in the 1980s for hiding spoilers and
    # punchlines — readers could easily decode by applying ROT13 again.
    #
    # The mapping under ROT13:
    #
    #   A <-> N    B <-> O    C <-> P    D <-> Q    E <-> R    F <-> S
    #   G <-> T    H <-> U    I <-> V    J <-> W    K <-> X    L <-> Y
    #   M <-> Z
    #
    # Parameters:
    #   text [String] — the text to transform
    #
    # Returns:
    #   The ROT13-transformed String.
    #
    # Examples:
    #   rot13("HELLO")         #=> "URYYB"
    #   rot13("URYYB")         #=> "HELLO"
    #   rot13("Hello, World!") #=> "Uryyb, Jbeyq!"
    #
    def self.rot13(text)
      encrypt(text, 13)
    end
  end
end
