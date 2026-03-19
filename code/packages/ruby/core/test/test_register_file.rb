# frozen_string_literal: true

require_relative "test_helper"

# Tests for RegisterFile -- the Core's general-purpose register storage.
class TestRegisterFileBasic < Minitest::Test
  def test_read_write
    rf = CodingAdventures::Core::RegisterFile.new
    rf.write(1, 42)
    assert_equal 42, rf.read(1)

    # Overwrite.
    rf.write(1, 100)
    assert_equal 100, rf.read(1)
  end

  def test_zero_register_enabled
    cfg = CodingAdventures::Core::RegisterFileConfig.new(count: 16, width: 32, zero_register: true)
    rf = CodingAdventures::Core::RegisterFile.new(cfg)

    rf.write(0, 999)
    assert_equal 0, rf.read(0), "R0 should always read 0"
  end

  def test_zero_register_disabled
    cfg = CodingAdventures::Core::RegisterFileConfig.new(count: 16, width: 32, zero_register: false)
    rf = CodingAdventures::Core::RegisterFile.new(cfg)

    rf.write(0, 999)
    assert_equal 999, rf.read(0), "R0 should be writable when zero register is disabled"
  end

  def test_out_of_range_read
    rf = CodingAdventures::Core::RegisterFile.new
    assert_equal 0, rf.read(100), "out-of-range read should return 0"
    assert_equal 0, rf.read(-1), "negative index read should return 0"
  end

  def test_out_of_range_write
    rf = CodingAdventures::Core::RegisterFile.new
    # Should not raise.
    rf.write(100, 42)
    rf.write(-1, 42)
  end

  def test_values
    rf = CodingAdventures::Core::RegisterFile.new
    rf.write(1, 10)
    rf.write(2, 20)

    vals = rf.values
    assert_equal rf.count, vals.length
    assert_equal 10, vals[1]
    assert_equal 20, vals[2]
  end

  def test_reset
    rf = CodingAdventures::Core::RegisterFile.new
    rf.write(1, 42)
    rf.write(5, 99)
    rf.reset

    assert_equal 0, rf.read(1)
    assert_equal 0, rf.read(5)
  end

  def test_to_s
    rf = CodingAdventures::Core::RegisterFile.new
    rf.write(1, 42)
    s = rf.to_s
    refute_empty s
    assert_includes s, "R1=42"
  end

  def test_bit_width_masking
    cfg = CodingAdventures::Core::RegisterFileConfig.new(count: 4, width: 8, zero_register: false)
    rf = CodingAdventures::Core::RegisterFile.new(cfg)

    rf.write(1, 0xABCD) # only low 8 bits should be stored
    assert_equal 0xCD, rf.read(1)
  end

  def test_count_and_width
    rf = CodingAdventures::Core::RegisterFile.new
    assert_equal 16, rf.count
    assert_equal 32, rf.width
  end

  def test_config_accessor
    cfg = CodingAdventures::Core::RegisterFileConfig.new(count: 8, width: 64, zero_register: false)
    rf = CodingAdventures::Core::RegisterFile.new(cfg)
    assert_equal 8, rf.config.count
    assert_equal 64, rf.config.width
  end

  def test_default_config
    rf = CodingAdventures::Core::RegisterFile.new(nil)
    assert_equal 16, rf.count
    assert_equal 32, rf.width
  end
end
