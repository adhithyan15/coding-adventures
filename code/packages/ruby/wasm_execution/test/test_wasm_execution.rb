# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_execution"

class TestWasmExecution < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::WasmExecution::VERSION
  end
end
