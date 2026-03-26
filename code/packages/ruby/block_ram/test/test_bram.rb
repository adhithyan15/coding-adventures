# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for ConfigurableBRAM.
# ============================================================================

class TestConfigurableBRAM < Minitest::Test
  def setup
    @bram = CodingAdventures::BlockRam::ConfigurableBRAM.new(
      total_bits: 256, width: 8
    )
  end

  def test_initial_depth
    assert_equal 32, @bram.depth
  end

  def test_initial_width
    assert_equal 8, @bram.width
  end

  def test_total_bits
    assert_equal 256, @bram.total_bits
  end

  def test_write_and_read_port_a
    data = [1, 0, 1, 0, 1, 0, 1, 0]
    @bram.tick_a(0, address: 0, data_in: data, write_enable: 1)
    @bram.tick_a(1, address: 0, data_in: data, write_enable: 1)

    @bram.tick_a(0, address: 0, data_in: [0] * 8, write_enable: 0)
    result = @bram.tick_a(1, address: 0, data_in: [0] * 8, write_enable: 0)
    assert_equal data, result
  end

  def test_write_and_read_port_b
    data = [0, 1, 0, 1, 0, 1, 0, 1]
    @bram.tick_b(0, address: 1, data_in: data, write_enable: 1)
    @bram.tick_b(1, address: 1, data_in: data, write_enable: 1)

    @bram.tick_b(0, address: 1, data_in: [0] * 8, write_enable: 0)
    result = @bram.tick_b(1, address: 1, data_in: [0] * 8, write_enable: 0)
    assert_equal data, result
  end

  def test_reconfigure_changes_depth
    @bram.reconfigure(width: 16)
    assert_equal 16, @bram.depth
    assert_equal 16, @bram.width
  end

  def test_reconfigure_clears_data
    data = [1, 0, 1, 0, 1, 0, 1, 0]
    @bram.tick_a(0, address: 0, data_in: data, write_enable: 1)
    @bram.tick_a(1, address: 0, data_in: data, write_enable: 1)

    @bram.reconfigure(width: 8)

    @bram.tick_a(0, address: 0, data_in: [0] * 8, write_enable: 0)
    result = @bram.tick_a(1, address: 0, data_in: [0] * 8, write_enable: 0)
    assert_equal [0] * 8, result
  end

  def test_reconfigure_invalid_width
    assert_raises(ArgumentError) { @bram.reconfigure(width: 0) }
    assert_raises(ArgumentError) { @bram.reconfigure(width: 7) }
  end

  def test_invalid_total_bits
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::ConfigurableBRAM.new(total_bits: 0, width: 8)
    end
  end

  def test_invalid_initial_width
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::ConfigurableBRAM.new(total_bits: 256, width: 0)
    end
  end

  def test_width_not_divisor
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::ConfigurableBRAM.new(total_bits: 256, width: 7)
    end
  end

  def test_default_total_bits
    bram = CodingAdventures::BlockRam::ConfigurableBRAM.new
    assert_equal 18_432, bram.total_bits
  end
end
