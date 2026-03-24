# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_bitset.rb -- Gem entry point
# --------------------------------------------------------------------------
#
# This is the file that gets loaded when someone writes:
#
#   require "coding_adventures_bitset"
#
# It pulls in the internal files in dependency order:
#   1. version   -- the VERSION constant (no dependencies)
#   2. bitset    -- the Bitset class itself
#
# Usage:
#   require "coding_adventures_bitset"
#
#   bs = CodingAdventures::Bitset::Bitset.new(100)
#   bs.set(42)
#   bs.test?(42)  # => true
# --------------------------------------------------------------------------

require_relative "coding_adventures/bitset/version"
require_relative "coding_adventures/bitset"
