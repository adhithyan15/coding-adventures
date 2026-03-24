# frozen_string_literal: true

# coding_adventures_wasm_leb128.rb — main entry point
#
# This file is the require target for the gem. It loads all sub-modules
# in dependency order.
#
# Usage:
#   require "coding_adventures_wasm_leb128"
#
#   include CodingAdventures::WasmLeb128
#   encode_unsigned(624485)  # => "\xE5\x8E\x26"

require_relative "coding_adventures/wasm_leb128/version"
require_relative "coding_adventures/wasm_leb128/leb128"

module CodingAdventures
  # LEB128 variable-length integer encoding for WASM binary format
  module WasmLeb128
  end
end
