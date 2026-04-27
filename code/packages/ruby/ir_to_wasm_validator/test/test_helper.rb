# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_ir_to_wasm_validator"

IR = CodingAdventures::CompilerIr
ITWV = CodingAdventures::IrToWasmValidator
ITWC = CodingAdventures::IrToWasmCompiler
