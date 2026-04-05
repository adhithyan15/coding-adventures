# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_runtime"

class TestWasmRuntime < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::WasmRuntime::VERSION
  end
end
