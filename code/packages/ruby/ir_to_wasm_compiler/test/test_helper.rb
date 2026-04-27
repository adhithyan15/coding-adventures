# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_ir_to_wasm_compiler"

IR = CodingAdventures::CompilerIr
ITWC = CodingAdventures::IrToWasmCompiler
WT = CodingAdventures::WasmTypes
