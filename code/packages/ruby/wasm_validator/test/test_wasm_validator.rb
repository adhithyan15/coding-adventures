# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_validator"

class TestWasmValidator < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::WasmValidator::VERSION
  end
end
