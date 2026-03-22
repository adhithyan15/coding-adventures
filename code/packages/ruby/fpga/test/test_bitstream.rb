# frozen_string_literal: true

require_relative "test_helper"
require "json"
require "tempfile"

# ============================================================================
# Tests for Bitstream configuration.
# ============================================================================

class TestBitstream < Minitest::Test
  def test_from_hash_empty
    bs = CodingAdventures::FPGA::Bitstream.from_hash({})
    assert_equal({}, bs.clbs)
    assert_equal({}, bs.routing)
    assert_equal({}, bs.io)
    assert_equal 4, bs.lut_k
  end

  def test_from_hash_with_clb
    config = {
      "clbs" => {
        "clb_0" => {
          "slice0" => {
            "lut_a" => [0] * 16,
            "lut_b" => [1] + [0] * 15,
            "ff_a" => true,
            "ff_b" => false,
            "carry" => true
          },
          "slice1" => {}
        }
      }
    }
    bs = CodingAdventures::FPGA::Bitstream.from_hash(config)

    assert bs.clbs.key?("clb_0")
    assert_equal [0] * 16, bs.clbs["clb_0"].slice0.lut_a
    assert_equal true, bs.clbs["clb_0"].slice0.ff_a_enabled
    assert_equal true, bs.clbs["clb_0"].slice0.carry_enabled
    assert_equal false, bs.clbs["clb_0"].slice0.ff_b_enabled
  end

  def test_from_hash_with_routing
    config = {
      "routing" => {
        "sw_0" => [
          {"src" => "clb_out", "dst" => "east"},
          {"src" => "north", "dst" => "south"}
        ]
      }
    }
    bs = CodingAdventures::FPGA::Bitstream.from_hash(config)

    assert bs.routing.key?("sw_0")
    assert_equal 2, bs.routing["sw_0"].length
    assert_equal "clb_out", bs.routing["sw_0"][0].source
    assert_equal "east", bs.routing["sw_0"][0].destination
  end

  def test_from_hash_with_io
    config = {
      "io" => {
        "pin_A0" => {"mode" => "input"},
        "pin_B0" => {"mode" => "output"},
        "pin_C0" => {"mode" => "tristate"}
      }
    }
    bs = CodingAdventures::FPGA::Bitstream.from_hash(config)

    assert_equal "input", bs.io["pin_A0"].mode
    assert_equal "output", bs.io["pin_B0"].mode
    assert_equal "tristate", bs.io["pin_C0"].mode
  end

  def test_from_hash_custom_lut_k
    config = {"lut_k" => 3}
    bs = CodingAdventures::FPGA::Bitstream.from_hash(config)
    assert_equal 3, bs.lut_k
  end

  def test_from_json_file
    config = {
      "clbs" => {
        "clb_0" => {
          "slice0" => {"lut_a" => [0] * 16},
          "slice1" => {}
        }
      },
      "io" => {"pin_A" => {"mode" => "input"}}
    }

    tmpfile = Tempfile.new(["bitstream", ".json"])
    tmpfile.write(JSON.generate(config))
    tmpfile.close

    bs = CodingAdventures::FPGA::Bitstream.from_json(tmpfile.path)
    assert bs.clbs.key?("clb_0")
    assert_equal "input", bs.io["pin_A"].mode
  ensure
    tmpfile&.unlink
  end

  def test_default_values
    bs = CodingAdventures::FPGA::Bitstream.new
    assert_equal({}, bs.clbs)
    assert_equal({}, bs.routing)
    assert_equal({}, bs.io)
    assert_equal 4, bs.lut_k
  end
end
