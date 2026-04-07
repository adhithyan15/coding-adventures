# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/coding_adventures/pixel_container"

PC = CodingAdventures::PixelContainer

# =============================================================================
# Tests for CodingAdventures::PixelContainer
# =============================================================================

class TestPixelContainerCreate < Minitest::Test
  def test_create_returns_container
    c = PC.create(4, 4)
    assert_instance_of PC::Container, c
  end

  def test_create_sets_width
    c = PC.create(10, 5)
    assert_equal 10, c.width
  end

  def test_create_sets_height
    c = PC.create(10, 5)
    assert_equal 5, c.height
  end

  def test_create_data_is_binary_string
    c = PC.create(2, 2)
    assert_equal Encoding::ASCII_8BIT, c.data.encoding
  end

  def test_create_data_correct_length
    c = PC.create(3, 4)
    assert_equal 3 * 4 * 4, c.data.bytesize
  end

  def test_create_data_all_zeros
    c = PC.create(2, 2)
    assert c.data.bytes.all?(&:zero?)
  end

  def test_create_raises_on_zero_width
    assert_raises(ArgumentError) { PC.create(0, 5) }
  end

  def test_create_raises_on_zero_height
    assert_raises(ArgumentError) { PC.create(5, 0) }
  end

  def test_create_raises_on_negative_width
    assert_raises(ArgumentError) { PC.create(-1, 5) }
  end

  def test_create_raises_on_negative_height
    assert_raises(ArgumentError) { PC.create(5, -1) }
  end
end

class TestContainerStruct < Minitest::Test
  def test_pixel_count
    c = PC.create(3, 4)
    assert_equal 12, c.pixel_count
  end

  def test_byte_count
    c = PC.create(3, 4)
    assert_equal 48, c.byte_count
  end

  def test_to_s_includes_dimensions
    c = PC.create(640, 480)
    assert_includes c.to_s, "640"
    assert_includes c.to_s, "480"
  end
end

class TestPixelAt < Minitest::Test
  def setup
    @c = PC.create(4, 4)
  end

  def test_pixel_at_default_is_transparent_black
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 0, 0)
  end

  def test_pixel_at_oob_negative_x
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, -1, 0)
  end

  def test_pixel_at_oob_negative_y
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 0, -1)
  end

  def test_pixel_at_oob_x_too_large
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 4, 0)
  end

  def test_pixel_at_oob_y_too_large
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 0, 4)
  end

  def test_pixel_at_returns_array_of_four
    result = PC.pixel_at(@c, 0, 0)
    assert_equal 4, result.length
  end
end

class TestSetPixel < Minitest::Test
  def setup
    @c = PC.create(4, 4)
  end

  def test_set_and_get_pixel
    PC.set_pixel(@c, 1, 2, 255, 128, 64, 200)
    assert_equal [255, 128, 64, 200], PC.pixel_at(@c, 1, 2)
  end

  def test_set_pixel_does_not_affect_neighbours
    PC.set_pixel(@c, 2, 2, 100, 101, 102, 103)
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 1, 2)
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 3, 2)
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 2, 1)
    assert_equal [0, 0, 0, 0], PC.pixel_at(@c, 2, 3)
  end

  def test_set_pixel_oob_is_noop
    PC.set_pixel(@c, 10, 10, 255, 0, 0, 255)
    # nothing should raise and all pixels remain 0
    assert c_all_zero?
  end

  def test_set_pixel_negative_oob_is_noop
    PC.set_pixel(@c, -1, -1, 255, 0, 0, 255)
    assert c_all_zero?
  end

  def test_set_pixel_clamps_channel_values
    # Passing 256 should wrap to 0 (256 & 0xFF == 0)
    PC.set_pixel(@c, 0, 0, 256, 257, 510, 511)
    result = PC.pixel_at(@c, 0, 0)
    assert_equal [0, 1, 254, 255], result
  end

  def test_set_pixel_top_left_corner
    PC.set_pixel(@c, 0, 0, 10, 20, 30, 40)
    assert_equal [10, 20, 30, 40], PC.pixel_at(@c, 0, 0)
  end

  def test_set_pixel_bottom_right_corner
    PC.set_pixel(@c, 3, 3, 50, 60, 70, 80)
    assert_equal [50, 60, 70, 80], PC.pixel_at(@c, 3, 3)
  end

  def test_set_pixel_returns_nil
    result = PC.set_pixel(@c, 0, 0, 1, 2, 3, 4)
    assert_nil result
  end

  private

  def c_all_zero?
    @c.data.bytes.all?(&:zero?)
  end
end

class TestFillPixels < Minitest::Test
  def test_fill_sets_all_pixels
    c = PC.create(3, 3)
    PC.fill_pixels(c, 255, 0, 128, 255)
    (0..2).each do |x|
      (0..2).each do |y|
        assert_equal [255, 0, 128, 255], PC.pixel_at(c, x, y)
      end
    end
  end

  def test_fill_transparent_black_resets_buffer
    c = PC.create(2, 2)
    PC.fill_pixels(c, 1, 2, 3, 4)
    PC.fill_pixels(c, 0, 0, 0, 0)
    assert c.data.bytes.all?(&:zero?)
  end

  def test_fill_returns_nil
    c = PC.create(2, 2)
    assert_nil PC.fill_pixels(c, 0, 0, 0, 0)
  end
end

class TestPixelContainerRoundtrip < Minitest::Test
  def test_set_many_pixels_and_read_back
    c = PC.create(8, 8)
    expected = {}
    (0..7).each do |x|
      (0..7).each do |y|
        r = (x * 31) & 0xFF
        g = (y * 37) & 0xFF
        b = ((x + y) * 13) & 0xFF
        a = 255
        PC.set_pixel(c, x, y, r, g, b, a)
        expected[[x, y]] = [r, g, b, a]
      end
    end
    expected.each do |(x, y), rgba|
      assert_equal rgba, PC.pixel_at(c, x, y), "mismatch at (#{x},#{y})"
    end
  end

  def test_1x1_image
    c = PC.create(1, 1)
    PC.set_pixel(c, 0, 0, 7, 8, 9, 10)
    assert_equal [7, 8, 9, 10], PC.pixel_at(c, 0, 0)
  end

  def test_data_offset_formula
    # Verify raw offset: pixel(2,1) in a 4-wide image is at offset (1*4+2)*4 = 24
    c = PC.create(4, 4)
    PC.set_pixel(c, 2, 1, 0xAA, 0xBB, 0xCC, 0xDD)
    assert_equal 0xAA, c.data.getbyte(24)
    assert_equal 0xBB, c.data.getbyte(25)
    assert_equal 0xCC, c.data.getbyte(26)
    assert_equal 0xDD, c.data.getbyte(27)
  end
end
