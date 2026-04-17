# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "tmpdir"
require "coding_adventures_brainfuck_wasm_compiler"
require "coding_adventures_wasm_runtime"

BWC = CodingAdventures::BrainfuckWasmCompiler
WR = CodingAdventures::WasmRuntime
