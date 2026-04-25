# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_rng"

class TestRng < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Rng::VERSION
  end
end
