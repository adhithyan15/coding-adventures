# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for SRAMCell and SRAMArray.
# ============================================================================

class TestSRAMCell < Minitest::Test
  def setup
    @cell = CodingAdventures::BlockRam::SRAMCell.new
  end

  def test_initial_value_is_zero
    assert_equal 0, @cell.value
  end

  def test_read_when_selected
    assert_equal 0, @cell.read(1)
  end

  def test_read_when_not_selected
    assert_nil @cell.read(0)
  end

  def test_write_when_selected
    @cell.write(1, 1)
    assert_equal 1, @cell.value
    assert_equal 1, @cell.read(1)
  end

  def test_write_when_not_selected
    @cell.write(0, 1)
    assert_equal 0, @cell.value
  end

  def test_overwrite
    @cell.write(1, 1)
    assert_equal 1, @cell.value
    @cell.write(1, 0)
    assert_equal 0, @cell.value
  end

  def test_validates_word_line
    assert_raises(TypeError) { @cell.read("1") }
    assert_raises(ArgumentError) { @cell.read(2) }
  end

  def test_validates_bit_line
    assert_raises(TypeError) { @cell.write(1, "1") }
    assert_raises(ArgumentError) { @cell.write(1, 2) }
  end
end

class TestSRAMArray < Minitest::Test
  def setup
    @arr = CodingAdventures::BlockRam::SRAMArray.new(4, 8)
  end

  def test_shape
    assert_equal [4, 8], @arr.shape
  end

  def test_initial_values_are_zero
    assert_equal [0] * 8, @arr.read(0)
    assert_equal [0] * 8, @arr.read(3)
  end

  def test_write_and_read
    data = [1, 0, 1, 0, 0, 1, 0, 1]
    @arr.write(0, data)
    assert_equal data, @arr.read(0)
  end

  def test_write_does_not_affect_other_rows
    @arr.write(0, [1, 1, 1, 1, 1, 1, 1, 1])
    assert_equal [0] * 8, @arr.read(1)
  end

  def test_invalid_row_raises
    assert_raises(ArgumentError) { @arr.read(-1) }
    assert_raises(ArgumentError) { @arr.read(4) }
    assert_raises(TypeError) { @arr.read("0") }
  end

  def test_write_wrong_data_length_raises
    assert_raises(ArgumentError) { @arr.write(0, [1, 0, 1]) }
  end

  def test_write_non_array_raises
    assert_raises(TypeError) { @arr.write(0, 123) }
  end

  def test_invalid_rows_raises
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::SRAMArray.new(0, 8)
    end
  end

  def test_invalid_cols_raises
    assert_raises(ArgumentError) do
      CodingAdventures::BlockRam::SRAMArray.new(4, 0)
    end
  end

  def test_small_array
    arr = CodingAdventures::BlockRam::SRAMArray.new(1, 1)
    arr.write(0, [1])
    assert_equal [1], arr.read(0)
  end
end
