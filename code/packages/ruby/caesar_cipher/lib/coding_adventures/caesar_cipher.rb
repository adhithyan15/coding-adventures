# frozen_string_literal: true

# ============================================================================
# CodingAdventures::CaesarCipher — Main Module
# ============================================================================
#
# The Caesar cipher is one of the oldest known encryption techniques, dating
# back to Julius Caesar himself, who reportedly used a shift of 3 to protect
# military communications.  The idea is beautifully simple:
#
#   - Pick a number (the "shift" or "key"), say 3.
#   - Replace every letter in the message with the letter that is `shift`
#     positions later in the alphabet.
#   - Wrap around when you reach the end: after Z comes A.
#
# Example with shift = 3:
#
#   Plain:   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
#   Cipher:  D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
#
#   "HELLO" encrypts to "KHOOR"
#
# This module provides:
#
#   1. **Encryption / Decryption** (`cipher.rb`)
#      - `encrypt(text, shift)` — apply the shift to every letter
#      - `decrypt(text, shift)` — reverse the shift
#      - `rot13(text)` — the special case where shift = 13
#
#   2. **Cryptanalysis** (`analysis.rb`)
#      - `brute_force(ciphertext)` — try all 25 non-trivial shifts
#      - `frequency_analysis(ciphertext)` — use English letter frequencies
#        and the chi-squared statistic to find the most likely shift
#
# ============================================================================

require_relative "caesar_cipher/version"
require_relative "caesar_cipher/cipher"
require_relative "caesar_cipher/analysis"

module CodingAdventures
  module CaesarCipher
  end
end
