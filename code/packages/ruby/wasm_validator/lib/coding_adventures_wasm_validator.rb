# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
require "coding_adventures_wasm_leb128"
require "coding_adventures_wasm_types"
require "coding_adventures_wasm_opcodes"
require "coding_adventures_wasm_module_parser"
require "coding_adventures_virtual_machine"

require_relative "coding_adventures/wasm_validator/version"
require_relative "coding_adventures/wasm_validator/validator"

module CodingAdventures
  # WebAssembly 1.0 structural validator.
  module WasmValidator
  end
end
