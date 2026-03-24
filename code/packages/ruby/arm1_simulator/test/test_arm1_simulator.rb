# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_arm1_simulator"

class TestArm1Simulator < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Arm1Simulator::VERSION
  end
end
