# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/affine2d"

class TestAffine2D < Minitest::Test
  include Affine2D
  DELTA = 1e-9

  def test_identity_apply
    pt = Affine.identity.apply_to_point(Point2D::Point.new(3, 4))
    assert_in_delta 3, pt.x, DELTA
    assert_in_delta 4, pt.y, DELTA
  end

  def test_translate
    pt = Affine.translate(2, 3).apply_to_point(Point2D::Point.new(1, 1))
    assert_in_delta 3, pt.x, DELTA
    assert_in_delta 4, pt.y, DELTA
  end

  def test_rotate_90
    pt = Affine.rotate(Trig::PI / 2).apply_to_point(Point2D::Point.new(1, 0))
    assert_in_delta 0, pt.x, DELTA
    assert_in_delta 1, pt.y, DELTA
  end

  def test_scale
    pt = Affine.scale(2, 3).apply_to_point(Point2D::Point.new(1, 1))
    assert_in_delta 2, pt.x, DELTA
    assert_in_delta 3, pt.y, DELTA
  end

  def test_scale_uniform
    pt = Affine.scale_uniform(5).apply_to_point(Point2D::Point.new(2, 3))
    assert_in_delta 10, pt.x, DELTA
    assert_in_delta 15, pt.y, DELTA
  end

  def test_multiply_translate
    a = Affine.translate(1, 0).then(Affine.translate(0, 2))
    pt = a.apply_to_point(Point2D::Point.new(0, 0))
    assert_in_delta 1, pt.x, DELTA
    assert_in_delta 2, pt.y, DELTA
  end

  def test_determinant_identity
    assert_in_delta 1, Affine.identity.determinant, DELTA
  end

  def test_determinant_scale
    assert_in_delta 6, Affine.scale(2, 3).determinant, DELTA
  end

  def test_invert_identity
    inv = Affine.identity.invert
    refute_nil inv
    assert inv.is_identity?
  end

  def test_invert_translate
    a = Affine.translate(3, -5)
    inv = a.invert
    refute_nil inv
    pt = inv.apply_to_point(a.apply_to_point(Point2D::Point.new(1, 2)))
    assert_in_delta 1, pt.x, DELTA
    assert_in_delta 2, pt.y, DELTA
  end

  def test_invert_singular
    # Scale by 0 is singular
    singular = Affine.scale(0, 1)
    assert_nil singular.invert
  end

  def test_is_identity
    assert Affine.identity.is_identity?
    refute Affine.translate(1, 0).is_identity?
  end

  def test_is_translation_only
    assert Affine.translate(5, -3).is_translation_only?
    refute Affine.rotate(0.1).is_translation_only?
  end

  def test_to_array
    arr = Affine.identity.to_array
    assert_equal [1, 0, 0, 1, 0, 0], arr
  end

  def test_apply_to_vector_excludes_translation
    v = Affine.translate(100, 100).apply_to_vector(Point2D::Point.new(1, 0))
    assert_in_delta 1, v.x, DELTA
    assert_in_delta 0, v.y, DELTA
  end

  def test_rotate_around
    # Rotating (1,0) by 90° around (1,0) should give (1,0) itself (pivot stays fixed)
    a = Affine.rotate_around(Trig::PI / 2, 1, 0)
    pt = a.apply_to_point(Point2D::Point.new(1, 0))
    assert_in_delta 1, pt.x, DELTA
    assert_in_delta 0, pt.y, DELTA
  end

  def test_skew_x
    # skew_x(π/4) adds tan(π/4)=1 * y to x
    a = Affine.skew_x(Trig::PI / 4)
    pt = a.apply_to_point(Point2D::Point.new(0, 1))
    assert_in_delta 1, pt.x, DELTA
    assert_in_delta 1, pt.y, DELTA
  end
end
