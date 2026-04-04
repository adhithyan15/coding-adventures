# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_mosaic_vm"

class TestMosaicVm < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::MosaicVm::VERSION
  end
end
