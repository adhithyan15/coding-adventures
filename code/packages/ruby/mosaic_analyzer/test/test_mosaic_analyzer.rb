# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_mosaic_analyzer"

class TestMosaicAnalyzer < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::MosaicAnalyzer::VERSION
  end
end
