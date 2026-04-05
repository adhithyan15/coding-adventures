# frozen_string_literal: true

# The main entry point for the CodingAdventures::VigenereCipher gem.
#
# This file requires all components of the Vigenere cipher implementation:
# - version.rb: the VERSION constant
# - cipher.rb: encrypt and decrypt module methods
# - analysis.rb: cryptanalysis tools (find_key_length, find_key, break_cipher)

require_relative "coding_adventures/vigenere_cipher/version"
require_relative "coding_adventures/vigenere_cipher/cipher"
require_relative "coding_adventures/vigenere_cipher/analysis"
