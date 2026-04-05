# frozen_string_literal: true

# The main entry point for the CodingAdventures::ScytaleCipher gem.
#
# This file requires all the components of the Scytale cipher implementation:
# - version.rb: the VERSION constant
# - cipher.rb: the encrypt, decrypt, and brute_force module methods

require_relative "coding_adventures/scytale_cipher/version"
require_relative "coding_adventures/scytale_cipher/cipher"
