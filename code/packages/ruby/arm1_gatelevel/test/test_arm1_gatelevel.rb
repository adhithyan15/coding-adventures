# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_arm1_gatelevel"

class TestArm1Gatelevel < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Arm1Gatelevel::VERSION
  end
end
