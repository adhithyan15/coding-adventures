# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_bitset_native.rb — Entry point for the gem
# --------------------------------------------------------------------------
#
# This file is the main require target for the gem. It loads:
# 1. The compiled Rust native extension (.so/.bundle/.dll)
# 2. The version constant
#
# The native extension defines:
#   CodingAdventures::BitsetNative::Bitset
#
# which is a Rust-backed bitset (compact boolean array packed into 64-bit
# words) with the same API as the pure Rust bitset crate, exposed to Ruby
# for high-performance bit manipulation.

require_relative "coding_adventures/bitset_native/version"

# Load the compiled native extension
# Ruby will search for bitset_native.so (Linux),
# bitset_native.bundle (macOS), or bitset_native.dll (Windows)
require "bitset_native"
