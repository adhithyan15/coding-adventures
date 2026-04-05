# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
require "coding_adventures_wasm_leb128"
require "coding_adventures_wasm_types"
require "coding_adventures_wasm_opcodes"
require "coding_adventures_wasm_module_parser"
require "coding_adventures_virtual_machine"
require "coding_adventures_wasm_execution"
require "coding_adventures_wasm_validator"

require_relative "coding_adventures/wasm_runtime/version"
require_relative "coding_adventures/wasm_runtime/wasm_instance"
require_relative "coding_adventures/wasm_runtime/wasi_stub"
require_relative "coding_adventures/wasm_runtime/wasm_runtime"

module CodingAdventures
  # WebAssembly 1.0 runtime --- parse, validate, instantiate, execute.
  module WasmRuntime
  end
end
