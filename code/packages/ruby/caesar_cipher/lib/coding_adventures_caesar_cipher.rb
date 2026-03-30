# frozen_string_literal: true

# ============================================================================
# coding_adventures_caesar_cipher.rb — Gem entry point
# ============================================================================
#
# This file is the top-level entry point that RubyGems loads when someone
# adds `require "coding_adventures_caesar_cipher"` to their code. Its only
# job is to delegate to the real module file that lives under the
# conventional `coding_adventures/caesar_cipher` namespace.
#
# Why the indirection?  RubyGems expects the file name to match the gem name
# (underscores for hyphens), but our module hierarchy is
# `CodingAdventures::CaesarCipher`, which maps to the path
# `coding_adventures/caesar_cipher`.  This shim bridges the two conventions.
# ============================================================================

require_relative "coding_adventures/caesar_cipher"
