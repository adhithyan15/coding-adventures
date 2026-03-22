# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for SinglePortRAM and DualPortRAM.
# ============================================================================

class TestSinglePortRAM < Minitest::Test
  def setup
    @ram = CodingAdventures::BlockRam::SinglePortRAM.new(depth: 4, width: 4)
  end

  def test_depth_and_width
    assert_equal 4, @ram.depth
    assert_equal 4, @ram.width
  end

  def test_initial_read_is_zeros
    # Rising edge read
    @ram.tick(0, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    result = @ram.tick(1, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    assert_equal [0, 0, 0, 0], result
  end

  def test_write_and_read_back
    data = [1, 0, 1, 1]
    # Write on rising edge
    @ram.tick(0, address: 0, data_in: data, write_enable: 1)
    @ram.tick(1, address: 0, data_in: data, write_enable: 1)
    # Read on next rising edge
    @ram.tick(0, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    result = @ram.tick(1, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    assert_equal data, result
  end

  def test_read_first_mode
    ram = CodingAdventures::BlockRam::SinglePortRAM.new(
      depth: 4, width: 4,
      read_mode: CodingAdventures::BlockRam::ReadMode::READ_FIRST
    )
    # Write data to address 0
    ram.tick(0, address: 0, data_in: [1, 1, 0, 0], write_enable: 1)
    ram.tick(1, address: 0, data_in: [1, 1, 0, 0], write_enable: 1)
    # Now write new data -- read_first should return OLD value
    ram.tick(0, address: 0, data_in: [0, 0, 1, 1], write_enable: 1)
    result = ram.tick(1, address: 0, data_in: [0, 0, 1, 1], write_enable: 1)
    assert_equal [1, 1, 0, 0], result
  end

  def test_write_first_mode
    ram = CodingAdventures::BlockRam::SinglePortRAM.new(
      depth: 4, width: 4,
      read_mode: CodingAdventures::BlockRam::ReadMode::WRITE_FIRST
    )
    # Write data -- write_first should return NEW value
    ram.tick(0, address: 0, data_in: [1, 0, 1, 0], write_enable: 1)
    result = ram.tick(1, address: 0, data_in: [1, 0, 1, 0], write_enable: 1)
    assert_equal [1, 0, 1, 0], result
  end

  def test_no_change_mode
    ram = CodingAdventures::BlockRam::SinglePortRAM.new(
      depth: 4, width: 4,
      read_mode: CodingAdventures::BlockRam::ReadMode::NO_CHANGE
    )
    # Read first to populate last_read
    ram.tick(0, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    ram.tick(1, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    # Write -- no_change should return previous read (all zeros)
    ram.tick(0, address: 0, data_in: [1, 1, 1, 1], write_enable: 1)
    result = ram.tick(1, address: 0, data_in: [1, 1, 1, 1], write_enable: 1)
    assert_equal [0, 0, 0, 0], result
  end

  def test_no_change_on_falling_edge
    # Clock stays at 1 (no rising edge) -> returns last_read
    result = @ram.tick(0, address: 0, data_in: [0, 0, 0, 0], write_enable: 0)
    assert_equal [0, 0, 0, 0], result
  end

  def test_dump
    data = [1, 0, 1, 0]
    @ram.tick(0, address: 2, data_in: data, write_enable: 1)
    @ram.tick(1, address: 2, data_in: data, write_enable: 1)

    contents = @ram.dump
    assert_equal [0, 0, 0, 0], contents[0]
    assert_equal data, contents[2]
  end

  def test_validates_address
    assert_raises(ArgumentError) do
      @ram.tick(0, address: 4, data_in: [0, 0, 0, 0], write_enable: 0)
    end
    assert_raises(TypeError) do
      @ram.tick(0, address: "0", data_in: [0, 0, 0, 0], write_enable: 0)
    end
  end

  def test_validates_data_in
    assert_raises(ArgumentError) do
      @ram.tick(0, address: 0, data_in: [0, 0], write_enable: 0)
    end
    assert_raises(TypeError) do
      @ram.tick(0, address: 0, data_in: 123, write_enable: 0)
    end
  end

  def test_invalid_depth
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::SinglePortRAM.new(depth: 0, width: 4)
    end
  end

  def test_invalid_width
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::SinglePortRAM.new(depth: 4, width: 0)
    end
  end
end

class TestDualPortRAM < Minitest::Test
  def setup
    @ram = CodingAdventures::BlockRam::DualPortRAM.new(depth: 4, width: 4)
  end

  def test_depth_and_width
    assert_equal 4, @ram.depth
    assert_equal 4, @ram.width
  end

  def test_write_a_read_b
    zeros = [0, 0, 0, 0]
    data = [1, 0, 1, 0]

    # Write via port A
    @ram.tick(0,
      address_a: 0, data_in_a: data, write_enable_a: 1,
      address_b: 0, data_in_b: zeros, write_enable_b: 0)
    @ram.tick(1,
      address_a: 0, data_in_a: data, write_enable_a: 1,
      address_b: 0, data_in_b: zeros, write_enable_b: 0)

    # Read via port B
    @ram.tick(0,
      address_a: 0, data_in_a: zeros, write_enable_a: 0,
      address_b: 0, data_in_b: zeros, write_enable_b: 0)
    _, out_b = @ram.tick(1,
      address_a: 0, data_in_a: zeros, write_enable_a: 0,
      address_b: 0, data_in_b: zeros, write_enable_b: 0)

    assert_equal data, out_b
  end

  def test_simultaneous_read_different_addresses
    zeros = [0, 0, 0, 0]
    data_a = [1, 1, 0, 0]
    data_b = [0, 0, 1, 1]

    # Write to addresses 0 and 1
    @ram.tick(0,
      address_a: 0, data_in_a: data_a, write_enable_a: 1,
      address_b: 1, data_in_b: data_b, write_enable_b: 1)
    @ram.tick(1,
      address_a: 0, data_in_a: data_a, write_enable_a: 1,
      address_b: 1, data_in_b: data_b, write_enable_b: 1)

    # Read both simultaneously
    @ram.tick(0,
      address_a: 0, data_in_a: zeros, write_enable_a: 0,
      address_b: 1, data_in_b: zeros, write_enable_b: 0)
    out_a, out_b = @ram.tick(1,
      address_a: 0, data_in_a: zeros, write_enable_a: 0,
      address_b: 1, data_in_b: zeros, write_enable_b: 0)

    assert_equal data_a, out_a
    assert_equal data_b, out_b
  end

  def test_write_collision_raises
    data = [1, 0, 1, 0]

    @ram.tick(0,
      address_a: 0, data_in_a: data, write_enable_a: 1,
      address_b: 0, data_in_b: data, write_enable_b: 1)

    assert_raises(CodingAdventures::BlockRam::WriteCollisionError) do
      @ram.tick(1,
        address_a: 0, data_in_a: data, write_enable_a: 1,
        address_b: 0, data_in_b: data, write_enable_b: 1)
    end
  end

  def test_write_collision_error_has_address
    data = [1, 0, 1, 0]
    @ram.tick(0,
      address_a: 2, data_in_a: data, write_enable_a: 1,
      address_b: 2, data_in_b: data, write_enable_b: 1)

    error = assert_raises(CodingAdventures::BlockRam::WriteCollisionError) do
      @ram.tick(1,
        address_a: 2, data_in_a: data, write_enable_a: 1,
        address_b: 2, data_in_b: data, write_enable_b: 1)
    end
    assert_equal 2, error.address
  end

  def test_no_rising_edge_returns_last_read
    zeros = [0, 0, 0, 0]
    out_a, out_b = @ram.tick(0,
      address_a: 0, data_in_a: zeros, write_enable_a: 0,
      address_b: 0, data_in_b: zeros, write_enable_b: 0)
    assert_equal zeros, out_a
    assert_equal zeros, out_b
  end

  def test_invalid_depth
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::DualPortRAM.new(depth: 0, width: 4)
    end
  end

  def test_invalid_width
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::DualPortRAM.new(depth: 4, width: 0)
    end
  end
end
