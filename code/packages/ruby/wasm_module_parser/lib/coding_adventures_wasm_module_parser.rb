# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_wasm_leb128"
require "coding_adventures_wasm_types"
require "coding_adventures_wasm_opcodes"

require_relative "coding_adventures/wasm_module_parser/version"
require_relative "coding_adventures/wasm_module_parser/parser"

module CodingAdventures
  # WASM binary module parser: decodes .wasm files into structured WasmModule data
  module WasmModuleParser
  end
end
