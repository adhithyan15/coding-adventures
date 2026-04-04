# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_mosaic_lexer"

class TestMosaicLexer < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::MosaicLexer::VERSION
  end
end
