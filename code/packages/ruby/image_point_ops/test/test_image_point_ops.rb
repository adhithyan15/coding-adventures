# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures/image_point_ops"

# Alias for brevity
Ops = CodingAdventures::ImagePointOps
PC  = CodingAdventures::PixelContainer

def solid(r, g, b, a)
  img = PC.create(1, 1)
  PC.set_pixel(img, 0, 0, r, g, b, a)
  img
end

class TestImagePointOps < Minitest::Test
  def test_dimensions_preserved
    img = PC.create(3, 5)
    out = Ops.invert(img)
    assert_equal 3, out.width
    assert_equal 5, out.height
  end

  def test_invert_rgb
    out = Ops.invert(solid(10, 100, 200, 255))
    assert_equal [245, 155, 55, 255], PC.pixel_at(out, 0, 0)
  end

  def test_invert_preserves_alpha
    out = Ops.invert(solid(10, 100, 200, 128))
    assert_equal 128, PC.pixel_at(out, 0, 0)[3]
  end

  def test_double_invert_identity
    img = solid(30, 80, 180, 255)
    assert_equal PC.pixel_at(img, 0, 0), PC.pixel_at(Ops.invert(Ops.invert(img)), 0, 0)
  end

  def test_threshold_above
    out = Ops.threshold(solid(200, 200, 200, 255), 128)
    assert_equal [255, 255, 255, 255], PC.pixel_at(out, 0, 0)
  end

  def test_threshold_below
    out = Ops.threshold(solid(50, 50, 50, 255), 128)
    assert_equal [0, 0, 0, 255], PC.pixel_at(out, 0, 0)
  end

  def test_threshold_luminance_white
    out = Ops.threshold_luminance(solid(255, 255, 255, 255), 128)
    assert_equal [255, 255, 255, 255], PC.pixel_at(out, 0, 0)
  end

  def test_posterize_two_levels
    out = Ops.posterize(solid(50, 50, 50, 255), 2)
    r = PC.pixel_at(out, 0, 0)[0]
    assert [0, 255].include?(r), "expected 0 or 255, got #{r}"
  end

  def test_swap_rgb_bgr
    out = Ops.swap_rgb_bgr(solid(255, 0, 0, 255))
    assert_equal [0, 0, 255, 255], PC.pixel_at(out, 0, 0)
  end

  def test_extract_channel_red
    out = Ops.extract_channel(solid(100, 150, 200, 255), 0)
    assert_equal [100, 0, 0, 255], PC.pixel_at(out, 0, 0)
  end

  def test_extract_channel_green
    out = Ops.extract_channel(solid(100, 150, 200, 255), 1)
    assert_equal [0, 150, 0, 255], PC.pixel_at(out, 0, 0)
  end

  def test_brightness_clamps_high
    out = Ops.brightness(solid(250, 10, 10, 255), 20)
    r, g, = PC.pixel_at(out, 0, 0)
    assert_equal 255, r
    assert_equal 30, g
  end

  def test_brightness_clamps_low
    out = Ops.brightness(solid(5, 10, 10, 255), -20)
    r, = PC.pixel_at(out, 0, 0)
    assert_equal 0, r
  end

  def test_contrast_identity
    img = solid(100, 150, 200, 255)
    out = Ops.contrast(img, 1.0)
    orig = PC.pixel_at(img, 0, 0)
    result = PC.pixel_at(out, 0, 0)
    3.times { |i| assert (result[i] - orig[i]).abs <= 1 }
  end

  def test_gamma_identity
    img = solid(100, 150, 200, 255)
    out = Ops.gamma(img, 1.0)
    orig = PC.pixel_at(img, 0, 0)
    result = PC.pixel_at(out, 0, 0)
    assert (result[0] - orig[0]).abs <= 1
  end

  def test_gamma_brightens_midtones
    out = Ops.gamma(solid(128, 128, 128, 255), 0.5)
    r, = PC.pixel_at(out, 0, 0)
    assert r > 128
  end

  def test_exposure_plus_one
    img = solid(100, 100, 100, 255)
    out = Ops.exposure(img, 1.0)
    r, = PC.pixel_at(out, 0, 0)
    assert r > PC.pixel_at(img, 0, 0)[0]
  end

  def test_greyscale_white_stays_white
    %i[rec709 bt601 average].each do |method|
      out = Ops.greyscale(solid(255, 255, 255, 255), method)
      assert_equal [255, 255, 255, 255], PC.pixel_at(out, 0, 0)
    end
  end

  def test_greyscale_black_stays_black
    out = Ops.greyscale(solid(0, 0, 0, 255))
    assert_equal [0, 0, 0, 255], PC.pixel_at(out, 0, 0)
  end

  def test_greyscale_equal_channels
    out = Ops.greyscale(solid(100, 100, 100, 255))
    r, g, b, = PC.pixel_at(out, 0, 0)
    assert_equal r, g
    assert_equal g, b
  end

  def test_sepia_preserves_alpha
    out = Ops.sepia(solid(128, 128, 128, 200))
    assert_equal 200, PC.pixel_at(out, 0, 0)[3]
  end

  def test_colour_matrix_identity
    img = solid(80, 120, 200, 255)
    id = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
    out = Ops.colour_matrix(img, id)
    orig = PC.pixel_at(img, 0, 0)
    result = PC.pixel_at(out, 0, 0)
    3.times { |i| assert (result[i] - orig[i]).abs <= 1 }
  end

  def test_saturate_zero_gives_grey
    out = Ops.saturate(solid(200, 100, 50, 255), 0.0)
    r, g, b, = PC.pixel_at(out, 0, 0)
    assert_equal r, g
    assert_equal g, b
  end

  def test_hue_rotate_360_identity
    img = solid(200, 80, 40, 255)
    out = Ops.hue_rotate(img, 360.0)
    orig = PC.pixel_at(img, 0, 0)
    result = PC.pixel_at(out, 0, 0)
    3.times { |i| assert (result[i] - orig[i]).abs <= 2, "channel #{i}: #{result[i]} vs #{orig[i]}" }
  end

  def test_srgb_linear_roundtrip
    img = solid(100, 150, 200, 255)
    out = Ops.linear_to_srgb_image(Ops.srgb_to_linear_image(img))
    orig = PC.pixel_at(img, 0, 0)
    result = PC.pixel_at(out, 0, 0)
    3.times { |i| assert (result[i] - orig[i]).abs <= 2 }
  end

  def test_apply_lut1d_invert
    lut = Array.new(256) { |i| 255 - i }
    out = Ops.apply_lut1d_u8(solid(100, 0, 200, 255), lut, lut, lut)
    assert_equal [155, 255, 55, 255], PC.pixel_at(out, 0, 0)
  end

  def test_build_lut1d_u8_identity
    lut = Ops.build_lut1d_u8 { |v| v }
    256.times { |i| assert (lut[i] - i).abs <= 1, "index #{i}: #{lut[i]}" }
  end

  def test_build_gamma_lut_identity
    lut = Ops.build_gamma_lut(1.0)
    256.times { |i| assert (lut[i] - i).abs <= 1, "index #{i}: #{lut[i]}" }
  end
end
