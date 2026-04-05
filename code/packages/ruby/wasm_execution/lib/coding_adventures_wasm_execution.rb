# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_wasm_leb128"
require "coding_adventures_wasm_types"
require "coding_adventures_wasm_opcodes"
require "coding_adventures_wasm_module_parser"
require "coding_adventures_virtual_machine"

require_relative "coding_adventures/wasm_execution/version"

module CodingAdventures
  # WebAssembly 1.0 wasm-execution
  module WasmExecution
  end
end
