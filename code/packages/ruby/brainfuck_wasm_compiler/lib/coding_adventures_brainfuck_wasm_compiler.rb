# frozen_string_literal: true

require "coding_adventures_brainfuck"
require "coding_adventures_brainfuck_ir_compiler"
require "coding_adventures_ir_to_wasm_compiler"
require "coding_adventures_ir_to_wasm_validator"
require "coding_adventures_wasm_module_encoder"
require "coding_adventures_wasm_validator"

require_relative "coding_adventures/brainfuck_wasm_compiler/version"
require_relative "coding_adventures/brainfuck_wasm_compiler/compiler"

module CodingAdventures
  module BrainfuckWasmCompiler
  end
end
