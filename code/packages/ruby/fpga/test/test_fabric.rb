# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for FPGAFabric (top-level FPGA model).
# ============================================================================

class TestFPGAFabric < Minitest::Test
  def setup
    and_tt = [0] * 16
    and_tt[3] = 1

    config = {
      "clbs" => {
        "clb_0" => {
          "slice0" => {"lut_a" => and_tt, "lut_b" => and_tt},
          "slice1" => {"lut_a" => and_tt, "lut_b" => and_tt}
        }
      },
      "routing" => {
        "sw_0" => [
          {"src" => "clb_out", "dst" => "east"}
        ]
      },
      "io" => {
        "in_a" => {"mode" => "input"},
        "in_b" => {"mode" => "input"},
        "out" => {"mode" => "output"},
        "tri" => {"mode" => "tristate"}
      }
    }

    @bs = CodingAdventures::FPGA::Bitstream.from_hash(config)
    @fpga = CodingAdventures::FPGA::FPGAFabric.new(@bs)
  end

  def test_evaluate_clb
    out = @fpga.evaluate_clb("clb_0",
      slice0_inputs_a: [1, 1, 0, 0], slice0_inputs_b: [1, 1, 0, 0],
      slice1_inputs_a: [0, 0, 0, 0], slice1_inputs_b: [0, 0, 0, 0],
      clock: 0)

    assert_equal 1, out.slice0.output_a  # AND(1,1)=1
    assert_equal 0, out.slice1.output_a  # AND(0,0)=0
  end

  def test_evaluate_clb_unknown_raises
    assert_raises(KeyError) do
      @fpga.evaluate_clb("nonexistent",
        slice0_inputs_a: [0, 0, 0, 0], slice0_inputs_b: [0, 0, 0, 0],
        slice1_inputs_a: [0, 0, 0, 0], slice1_inputs_b: [0, 0, 0, 0],
        clock: 0)
    end
  end

  def test_route
    result = @fpga.route("sw_0", {"clb_out" => 1})
    assert_equal({"east" => 1}, result)
  end

  def test_route_unknown_raises
    assert_raises(KeyError) { @fpga.route("unknown", {}) }
  end

  def test_set_input_and_read
    @fpga.set_input("in_a", 1)
    # Read the input pin (it's in INPUT mode)
    assert_equal 1, @fpga.read_output("in_a")
  end

  def test_set_input_unknown_raises
    assert_raises(KeyError) { @fpga.set_input("unknown", 0) }
  end

  def test_read_output_unknown_raises
    assert_raises(KeyError) { @fpga.read_output("unknown") }
  end

  def test_drive_output
    @fpga.drive_output("out", 1)
    assert_equal 1, @fpga.read_output("out")
  end

  def test_drive_output_unknown_raises
    assert_raises(KeyError) { @fpga.drive_output("unknown", 0) }
  end

  def test_tristate_output
    @fpga.drive_output("tri", 1)
    assert_nil @fpga.read_output("tri")
  end

  def test_accessors
    assert @fpga.clbs.key?("clb_0")
    assert @fpga.switches.key?("sw_0")
    assert @fpga.ios.key?("in_a")
    assert_equal @bs.lut_k, @fpga.bitstream.lut_k
  end

  def test_empty_config
    bs = CodingAdventures::FPGA::Bitstream.from_hash({})
    fpga = CodingAdventures::FPGA::FPGAFabric.new(bs)
    assert_equal({}, fpga.clbs)
    assert_equal({}, fpga.switches)
    assert_equal({}, fpga.ios)
  end
end
