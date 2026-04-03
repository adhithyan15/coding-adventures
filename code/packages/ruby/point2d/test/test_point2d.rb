# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/point2d"

class TestPoint2D < Minitest::Test
  include Point2D
  DELTA = 1e-9

  def test_add
    p = Point.new(1, 2).add(Point.new(3, 4))
    assert_in_delta 4, p.x, DELTA
    assert_in_delta 6, p.y, DELTA
  end

  def test_subtract
    p = Point.new(5, 3).subtract(Point.new(2, 1))
    assert_in_delta 3, p.x, DELTA
    assert_in_delta 2, p.y, DELTA
  end

  def test_scale
    p = Point.new(2, 3).scale(2)
    assert_in_delta 4, p.x, DELTA
    assert_in_delta 6, p.y, DELTA
  end

  def test_negate
    p = Point.new(1, -2).negate
    assert_in_delta(-1, p.x, DELTA)
    assert_in_delta 2, p.y, DELTA
  end

  def test_dot
    assert_in_delta 11, Point.new(1, 2).dot(Point.new(3, 4)), DELTA
  end

  def test_cross
    assert_in_delta(-2, Point.new(1, 2).cross(Point.new(3, 4)), DELTA)
  end

  def test_magnitude
    assert_in_delta 5, Point.new(3, 4).magnitude, DELTA
  end

  def test_magnitude_squared
    assert_in_delta 25, Point.new(3, 4).magnitude_squared, DELTA
  end

  def test_normalize
    n = Point.new(3, 4).normalize
    assert_in_delta 1, n.magnitude, DELTA
  end

  def test_normalize_zero
    n = Point.new(0, 0).normalize
    assert_in_delta 0, n.x, DELTA
    assert_in_delta 0, n.y, DELTA
  end

  def test_distance
    assert_in_delta 5, Point.new(0, 0).distance(Point.new(3, 4)), DELTA
  end

  def test_lerp_start
    p = Point.new(0, 0).lerp(Point.new(10, 10), 0)
    assert_in_delta 0, p.x, DELTA
  end

  def test_lerp_end
    p = Point.new(0, 0).lerp(Point.new(10, 10), 1)
    assert_in_delta 10, p.x, DELTA
  end

  def test_lerp_mid
    p = Point.new(0, 0).lerp(Point.new(10, 0), 0.5)
    assert_in_delta 5, p.x, DELTA
  end

  def test_perpendicular
    pp = Point.new(1, 0).perpendicular
    assert_in_delta 0, pp.x, DELTA
    assert_in_delta 1, pp.y, DELTA
  end

  def test_angle
    assert_in_delta Trig::PI / 4, Point.new(1, 1).angle, DELTA
  end
end

class TestRect < Minitest::Test
  include Point2D
  DELTA = 1e-9

  def r
    Rect.new(0, 0, 10, 10)
  end

  def test_contains_point_inside
    assert r.contains_point?(Point.new(5, 5))
  end

  def test_contains_point_outside
    refute r.contains_point?(Point.new(10, 5))
  end

  def test_union
    r2 = Rect.new(5, 5, 10, 10)
    u = r.union(r2)
    assert_in_delta 0, u.x, DELTA
    assert_in_delta 0, u.y, DELTA
    assert_in_delta 15, u.width, DELTA
    assert_in_delta 15, u.height, DELTA
  end

  def test_intersection
    r2 = Rect.new(5, 5, 10, 10)
    i = r.intersection(r2)
    refute_nil i
    assert_in_delta 5, i.x, DELTA
    assert_in_delta 5, i.width, DELTA
  end

  def test_intersection_disjoint
    r2 = Rect.new(20, 20, 5, 5)
    assert_nil r.intersection(r2)
  end

  def test_expand_by
    e = r.expand_by(2)
    assert_in_delta(-2, e.x, DELTA)
    assert_in_delta 14, e.width, DELTA
  end
end
