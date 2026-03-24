# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_wasm_types"

require_relative "coding_adventures/wasm_opcodes/version"
require_relative "coding_adventures/wasm_opcodes/opcodes"

module CodingAdventures
  # Complete WASM 1.0 opcode table with metadata (name, immediates, stack effects, category)
  module WasmOpcodes
  end
end
