# frozen_string_literal: true

# coding_adventures_wasm_types.rb — main entry point
#
# This file is the require target for the gem. It loads all sub-modules
# in dependency order.
#
# Usage:
#   require "coding_adventures_wasm_types"
#
#   include CodingAdventures::WasmTypes
#   ft = FuncType.new([VALUE_TYPE[:i32]], [VALUE_TYPE[:i64]])

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_wasm_leb128"

require_relative "coding_adventures/wasm_types/version"
require_relative "coding_adventures/wasm_types/types"

module CodingAdventures
  # WASM 1.0 type system: ValueType, FuncType, Limits, MemoryType, TableType, GlobalType, WasmModule
  module WasmTypes
  end
end
