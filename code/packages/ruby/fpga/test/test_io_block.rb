# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for IOBlock.
# ============================================================================

class TestIOBlock < Minitest::Test
  def test_input_mode
    io = CodingAdventures::FPGA::IOBlock.new("pin_0", mode: CodingAdventures::FPGA::IOMode::INPUT)
    io.drive_pad(1)
    assert_equal 1, io.read_internal
    assert_equal 1, io.read_pad
  end

  def test_output_mode
    io = CodingAdventures::FPGA::IOBlock.new("led_0", mode: CodingAdventures::FPGA::IOMode::OUTPUT)
    io.drive_internal(1)
    assert_equal 1, io.read_internal
    assert_equal 1, io.read_pad
  end

  def test_tristate_mode
    io = CodingAdventures::FPGA::IOBlock.new("bus_0", mode: CodingAdventures::FPGA::IOMode::TRISTATE)
    io.drive_internal(1)
    assert_equal 1, io.read_internal
    assert_nil io.read_pad
  end

  def test_configure_mode
    io = CodingAdventures::FPGA::IOBlock.new("pin_0")
    assert_equal CodingAdventures::FPGA::IOMode::INPUT, io.mode

    io.configure(CodingAdventures::FPGA::IOMode::OUTPUT)
    assert_equal CodingAdventures::FPGA::IOMode::OUTPUT, io.mode
  end

  def test_name
    io = CodingAdventures::FPGA::IOBlock.new("sensor_in")
    assert_equal "sensor_in", io.name
  end

  def test_empty_name_raises
    assert_raises(ArgumentError) { CodingAdventures::FPGA::IOBlock.new("") }
  end

  def test_non_string_name_raises
    assert_raises(ArgumentError) { CodingAdventures::FPGA::IOBlock.new(123) }
  end

  def test_invalid_drive_pad_value
    io = CodingAdventures::FPGA::IOBlock.new("pin_0")
    assert_raises(ArgumentError) { io.drive_pad(2) }
  end

  def test_invalid_drive_internal_value
    io = CodingAdventures::FPGA::IOBlock.new("pin_0")
    assert_raises(ArgumentError) { io.drive_internal(2) }
  end

  def test_invalid_mode
    io = CodingAdventures::FPGA::IOBlock.new("pin_0")
    assert_raises(ArgumentError) { io.configure(:invalid) }
  end

  def test_output_mode_drive_low
    io = CodingAdventures::FPGA::IOBlock.new("led_0", mode: CodingAdventures::FPGA::IOMode::OUTPUT)
    io.drive_internal(0)
    assert_equal 0, io.read_pad
  end
end
