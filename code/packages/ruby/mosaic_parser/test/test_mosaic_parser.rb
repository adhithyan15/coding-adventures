# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_mosaic_parser"

class TestMosaicParser < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::MosaicParser::VERSION
  end
end
