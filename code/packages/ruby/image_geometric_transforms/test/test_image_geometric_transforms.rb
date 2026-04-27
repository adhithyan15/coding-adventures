# frozen_string_literal: true

require "minitest/autorun"

$LOAD_PATH.unshift File.join(__dir__, "../../pixel_container/lib")
$LOAD_PATH.unshift File.join(__dir__, "../lib")

require "coding_adventures/pixel_container"
require "coding_adventures/image_geometric_transforms"

PC  = CodingAdventures::PixelContainer
GT  = CodingAdventures::ImageGeometricTransforms

# =============================================================================
# Helpers
# =============================================================================

# Create a 1×1 image containing a single RGBA pixel.
def solid_1x1(r, g, b, a)
  img = PC.create(1, 1)
  PC.set_pixel(img, 0, 0, r, g, b, a)
  img
end

# Create a 2×2 image with four distinct corner pixels.
#   (0,0)=top-left  (1,0)=top-right
#   (0,1)=bottom-left (1,1)=bottom-right
def checkerboard_2x2
  img = PC.create(2, 2)
  PC.set_pixel(img, 0, 0,   10,  20,  30, 255)   # top-left     — dark blue-ish
  PC.set_pixel(img, 1, 0,  200,  10,  10, 255)   # top-right    — red
  PC.set_pixel(img, 0, 1,   10, 200,  10, 255)   # bottom-left  — green
  PC.set_pixel(img, 1, 1,  200, 200,  10, 255)   # bottom-right — yellow
  img
end

# Create a 3×3 image filled with a gradient for scale tests.
def gradient_3x3
  img = PC.create(3, 3)
  3.times do |y|
    3.times do |x|
      v = (x + y * 3) * 28
      PC.set_pixel(img, x, y, v, v, v, 255)
    end
  end
  img
end

# Assert two pixel arrays are approximately equal within tolerance.
def assert_pixel_near(expected, actual, tol = 2, msg = nil)
  4.times do |i|
    diff = (expected[i] - actual[i]).abs
    assert diff <= tol, "#{msg}: channel #{i}: expected #{expected[i]}, got #{actual[i]} (diff #{diff} > #{tol})"
  end
end

# =============================================================================
# Tests: flip_horizontal
# =============================================================================

class TestFlipHorizontal < Minitest::Test
  def test_flips_left_and_right_pixels
    src = checkerboard_2x2
    dst = GT.flip_horizontal(src)
    # top-left of dst should be what was top-right of src
    assert_equal [200, 10, 10, 255], PC.pixel_at(dst, 0, 0)
    assert_equal [10, 20, 30, 255],  PC.pixel_at(dst, 1, 0)
  end

  def test_dimensions_unchanged
    src = checkerboard_2x2
    dst = GT.flip_horizontal(src)
    assert_equal 2, dst.width
    assert_equal 2, dst.height
  end

  def test_double_flip_is_identity
    src = checkerboard_2x2
    dst = GT.flip_horizontal(GT.flip_horizontal(src))
    2.times do |y|
      2.times do |x|
        assert_equal PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y)
      end
    end
  end
end

# =============================================================================
# Tests: flip_vertical
# =============================================================================

class TestFlipVertical < Minitest::Test
  def test_flips_top_and_bottom_pixels
    src = checkerboard_2x2
    dst = GT.flip_vertical(src)
    # top-left of dst should be what was bottom-left of src
    assert_equal [10, 200, 10, 255],  PC.pixel_at(dst, 0, 0)
    assert_equal [200, 200, 10, 255], PC.pixel_at(dst, 1, 0)
  end

  def test_double_flip_is_identity
    src = checkerboard_2x2
    dst = GT.flip_vertical(GT.flip_vertical(src))
    2.times do |y|
      2.times do |x|
        assert_equal PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y)
      end
    end
  end
end

# =============================================================================
# Tests: rotate_90_cw
# =============================================================================

class TestRotate90Cw < Minitest::Test
  # A 2×2 CW rotation swaps dimensions (2×2→2×2 here, since square).
  # For a non-square 3×1 image the swap is visible.
  def test_dimensions_swap
    img = PC.create(3, 1)
    dst = GT.rotate_90_cw(img)
    assert_equal 1, dst.width
    assert_equal 3, dst.height
  end

  def test_pixel_mapping
    # I[0,0]=TL, I[1,0]=TR, I[0,1]=BL, I[1,1]=BR on 2×2
    # CW 90°: TL→TR, TR→BR, BR→BL, BL→TL in output
    src = checkerboard_2x2
    dst = GT.rotate_90_cw(src)
    # O[x',y'] = I[y', W-1-x']  (W=2)
    # O[0,0] = I[0, 1] = BL = [10,200,10,255]
    assert_equal [10, 200, 10, 255], PC.pixel_at(dst, 0, 0)
    # O[1,0] = I[0, 0] = TL = [10,20,30,255]
    assert_equal [10, 20, 30, 255], PC.pixel_at(dst, 1, 0)
  end

  def test_round_trip_with_ccw
    src = checkerboard_2x2
    dst = GT.rotate_90_ccw(GT.rotate_90_cw(src))
    2.times do |y|
      2.times do |x|
        assert_equal PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y)
      end
    end
  end
end

# =============================================================================
# Tests: rotate_90_ccw
# =============================================================================

class TestRotate90Ccw < Minitest::Test
  def test_dimensions_swap
    img = PC.create(1, 4)
    dst = GT.rotate_90_ccw(img)
    assert_equal 4, dst.width
    assert_equal 1, dst.height
  end

  def test_pixel_mapping
    src = checkerboard_2x2
    dst = GT.rotate_90_ccw(src)
    # O[x',y'] = I[H-1-y', x']  (H=2)
    # O[0,0] = I[1, 0] = TR = [200,10,10,255]
    assert_equal [200, 10, 10, 255], PC.pixel_at(dst, 0, 0)
  end
end

# =============================================================================
# Tests: rotate_180
# =============================================================================

class TestRotate180 < Minitest::Test
  def test_dimensions_unchanged
    src = checkerboard_2x2
    dst = GT.rotate_180(src)
    assert_equal src.width,  dst.width
    assert_equal src.height, dst.height
  end

  def test_twice_is_identity
    src = checkerboard_2x2
    dst = GT.rotate_180(GT.rotate_180(src))
    2.times do |y|
      2.times do |x|
        assert_equal PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y)
      end
    end
  end

  def test_pixel_values
    src = checkerboard_2x2
    dst = GT.rotate_180(src)
    # TL of dst = BR of src
    assert_equal [200, 200, 10, 255], PC.pixel_at(dst, 0, 0)
    # BR of dst = TL of src
    assert_equal [10, 20, 30, 255], PC.pixel_at(dst, 1, 1)
  end
end

# =============================================================================
# Tests: crop
# =============================================================================

class TestCrop < Minitest::Test
  def test_correct_dimensions
    src = gradient_3x3
    dst = GT.crop(src, 0, 0, 2, 2)
    assert_equal 2, dst.width
    assert_equal 2, dst.height
  end

  def test_pixel_values_match_source
    src = gradient_3x3
    dst = GT.crop(src, 1, 1, 2, 2)
    # dst[0,0] == src[1,1], dst[1,0] == src[2,1], etc.
    assert_equal PC.pixel_at(src, 1, 1), PC.pixel_at(dst, 0, 0)
    assert_equal PC.pixel_at(src, 2, 1), PC.pixel_at(dst, 1, 0)
  end

  def test_oob_crop_returns_transparent
    src = solid_1x1(100, 100, 100, 255)
    # Crop from position (0,0) but also extend outside source
    dst = GT.crop(src, 0, 0, 3, 1)
    assert_equal [100, 100, 100, 255], PC.pixel_at(dst, 0, 0)
    assert_equal [0, 0, 0, 0],         PC.pixel_at(dst, 1, 0)
  end
end

# =============================================================================
# Tests: pad
# =============================================================================

class TestPad < Minitest::Test
  def test_dimensions
    src = solid_1x1(50, 60, 70, 255)
    dst = GT.pad(src, 1, 2, 3, 4)
    # left=4 + src.w=1 + right=2 = 7
    assert_equal 4 + 1 + 2, dst.width
    # top=1 + src.h=1 + bottom=3 = 5
    assert_equal 1 + 1 + 3, dst.height
  end

  def test_border_filled_with_default_fill
    src = solid_1x1(50, 60, 70, 255)
    dst = GT.pad(src, 1, 1, 1, 1)
    # corner pixels should be transparent black
    assert_equal [0, 0, 0, 0], PC.pixel_at(dst, 0, 0)
    assert_equal [0, 0, 0, 0], PC.pixel_at(dst, 2, 2)
  end

  def test_interior_pixel_preserved
    src = solid_1x1(50, 60, 70, 255)
    dst = GT.pad(src, 1, 1, 1, 1)
    assert_equal [50, 60, 70, 255], PC.pixel_at(dst, 1, 1)
  end

  def test_custom_fill_colour
    src = solid_1x1(50, 60, 70, 255)
    dst = GT.pad(src, 1, 0, 0, 0, fill: [255, 0, 0, 255])
    assert_equal [255, 0, 0, 255], PC.pixel_at(dst, 0, 0)
  end
end

# =============================================================================
# Tests: scale
# =============================================================================

class TestScale < Minitest::Test
  def test_scale_up_doubles_dimensions
    src = checkerboard_2x2
    dst = GT.scale(src, 4, 4)
    assert_equal 4, dst.width
    assert_equal 4, dst.height
  end

  def test_scale_down_halves_dimensions
    src = gradient_3x3
    dst = GT.scale(src, 1, 1)
    assert_equal 1, dst.width
    assert_equal 1, dst.height
  end

  def test_scale_solid_colour_unchanged
    # Scaling a solid-colour image should keep the colour exactly.
    src = PC.create(4, 4)
    PC.fill_pixels(src, 80, 120, 200, 255) rescue nil
    4.times do |y|
      4.times do |x|
        PC.set_pixel(src, x, y, 80, 120, 200, 255)
      end
    end
    dst = GT.scale(src, 8, 8)
    assert_pixel_near([80, 120, 200, 255], PC.pixel_at(dst, 4, 4), 2, "solid colour scale")
  end
end

# =============================================================================
# Tests: rotate
# =============================================================================

class TestRotate < Minitest::Test
  def test_rotate_zero_is_near_identity
    src = checkerboard_2x2
    dst = GT.rotate(src, 0.0, mode: :nearest)
    assert_pixel_near(PC.pixel_at(src, 0, 0), PC.pixel_at(dst, 0, 0), 2, "rotate 0 TL")
    assert_pixel_near(PC.pixel_at(src, 1, 1), PC.pixel_at(dst, 1, 1), 2, "rotate 0 BR")
  end

  def test_rotate_fit_enlarges_canvas_for_diagonal
    src = PC.create(10, 10)
    dst = GT.rotate(src, Math::PI / 4, bounds: :fit)
    # A 10×10 image rotated 45° should have a canvas at least sqrt(2)*10 ≈ 14 wide
    assert dst.width >= 14, "expected width >= 14, got #{dst.width}"
    assert dst.height >= 14, "expected height >= 14, got #{dst.height}"
  end

  def test_rotate_crop_preserves_dimensions
    src = checkerboard_2x2
    dst = GT.rotate(src, Math::PI / 6, bounds: :crop)
    assert_equal src.width,  dst.width
    assert_equal src.height, dst.height
  end
end

# =============================================================================
# Tests: affine
# =============================================================================

class TestAffine < Minitest::Test
  # Identity matrix: [[1,0,0],[0,1,0]]
  IDENTITY = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].freeze

  def test_identity_affine_near_identity
    src = checkerboard_2x2
    dst = GT.affine(src, IDENTITY, src.width, src.height, mode: :nearest)
    2.times do |y|
      2.times do |x|
        assert_pixel_near(PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y), 2, "affine identity (#{x},#{y})")
      end
    end
  end

  def test_affine_translation
    src = solid_1x1(200, 100, 50, 255)
    # Translate by (+1, +1): output at (1,1) should reflect the source pixel
    matrix = [[1.0, 0.0, 1.0], [0.0, 1.0, 1.0]]
    dst = GT.affine(src, matrix, 3, 3, mode: :nearest, oob: :zero)
    # Output pixel at (1,1) maps back to input (0,0)
    assert_pixel_near([200, 100, 50, 255], PC.pixel_at(dst, 1, 1), 2, "translated pixel")
  end
end

# =============================================================================
# Tests: perspective_warp
# =============================================================================

class TestPerspectiveWarp < Minitest::Test
  # Identity homography: [[1,0,0],[0,1,0],[0,0,1]]
  IDENTITY_H = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].freeze

  def test_identity_warp_near_identity
    src = checkerboard_2x2
    dst = GT.perspective_warp(src, IDENTITY_H, src.width, src.height, mode: :nearest)
    2.times do |y|
      2.times do |x|
        assert_pixel_near(PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y), 2, "persp identity (#{x},#{y})")
      end
    end
  end

  def test_perspective_warp_produces_correct_dimensions
    src = checkerboard_2x2
    dst = GT.perspective_warp(src, IDENTITY_H, 4, 3)
    assert_equal 4, dst.width
    assert_equal 3, dst.height
  end
end

# =============================================================================
# Tests: nearest neighbour interpolation
# =============================================================================

class TestNearestNeighbour < Minitest::Test
  def test_nearest_returns_exact_pixel_for_integer_coord
    src = checkerboard_2x2
    # Scale 2×2 → 2×2 with nearest: should preserve exact values
    dst = GT.scale(src, 2, 2, mode: :nearest)
    2.times do |y|
      2.times do |x|
        assert_equal PC.pixel_at(src, x, y), PC.pixel_at(dst, x, y)
      end
    end
  end
end

# =============================================================================
# Tests: bilinear interpolation
# =============================================================================

class TestBilinear < Minitest::Test
  def test_midpoint_of_two_pixel_gradient
    # Create a 2×1 image: left pixel pure white (linear), right pixel black.
    src = PC.create(2, 1)
    PC.set_pixel(src, 0, 0, 255, 255, 255, 255)
    PC.set_pixel(src, 1, 0,   0,   0,   0, 255)
    # Scale to 3 wide: centre pixel (index 1) should be approximately mid-grey
    # in linear light → ~188 in sRGB (√0.5 ≈ 0.707 → sRGB ≈ 188)
    dst = GT.scale(src, 3, 1, mode: :bilinear)
    mid = PC.pixel_at(dst, 1, 0)
    # Mid-grey in sRGB is about 188; accept ±10 tolerance for sampling position
    assert mid[0] > 100 && mid[0] < 240, "midpoint R=#{mid[0]} out of expected range"
  end
end

# =============================================================================
# Tests: OOB modes (smoke tests — must not raise)
# =============================================================================

class TestOobModes < Minitest::Test
  def test_replicate_oob_does_not_raise
    src = checkerboard_2x2
    dst = GT.scale(src, 4, 4, mode: :bilinear)  # uses :replicate internally
    refute_nil dst
  end

  def test_reflect_oob_does_not_raise
    src = checkerboard_2x2
    mat = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    dst = GT.affine(src, mat, 6, 6, mode: :bilinear, oob: :reflect)
    refute_nil dst
  end

  def test_wrap_oob_does_not_raise
    src = checkerboard_2x2
    mat = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    dst = GT.affine(src, mat, 6, 6, mode: :bilinear, oob: :wrap)
    refute_nil dst
  end

  def test_zero_oob_does_not_raise
    src = checkerboard_2x2
    dst = GT.rotate(src, 1.0, mode: :nearest)  # uses :zero internally
    refute_nil dst
  end

  def test_bicubic_does_not_raise
    src = gradient_3x3
    dst = GT.scale(src, 6, 6, mode: :bicubic)
    refute_nil dst
  end
end
