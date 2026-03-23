# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_uuid"

class TestUuid < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Uuid::VERSION
  end
end
