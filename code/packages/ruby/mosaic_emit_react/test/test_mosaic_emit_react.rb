# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_mosaic_emit_react"

class TestMosaicEmitReact < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::MosaicEmitReact::VERSION
  end
end
