# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/bezier2d"

class TestQuadraticBezier < Minitest::Test
  include Point2D
  DELTA = 1e-9

  def q
    @q ||= Bezier2D::QuadraticBezier.new(
      Point.new(0, 0), Point.new(1, 2), Point.new(2, 0)
    )
  end

  def test_eval_at_zero
    p = q.eval(0)
    assert_in_delta 0, p.x, DELTA
    assert_in_delta 0, p.y, DELTA
  end

  def test_eval_at_one
    p = q.eval(1)
    assert_in_delta 2, p.x, DELTA
    assert_in_delta 0, p.y, DELTA
  end

  def test_eval_midpoint
    p = q.eval(0.5)
    assert_in_delta 1, p.x, DELTA
    assert_in_delta 1, p.y, DELTA
  end

  def test_split_midpoints
    left, right = q.split(0.5)
    m = q.eval(0.5)
    assert_in_delta m.x, left.p2.x, DELTA
    assert_in_delta m.x, right.p0.x, DELTA
  end

  def test_polyline_straight
    straight = Bezier2D::QuadraticBezier.new(
      Point.new(0, 0), Point.new(1, 0), Point.new(2, 0)
    )
    pts = straight.polyline(0.1)
    assert_equal 2, pts.length
  end

  def test_bbox_contains_endpoints
    bb = q.bbox
    assert bb.x <= 0
    assert bb.x + bb.width >= 2
  end

  def test_elevate_equivalent
    c = q.elevate
    [0, 0.25, 0.5, 0.75, 1].each do |t|
      qp = q.eval(t)
      cp = c.eval(t)
      assert_in_delta qp.x, cp.x, DELTA
      assert_in_delta qp.y, cp.y, DELTA
    end
  end
end

class TestCubicBezier < Minitest::Test
  include Point2D
  DELTA = 1e-9

  def c
    @c ||= Bezier2D::CubicBezier.new(
      Point.new(0, 0), Point.new(1, 2),
      Point.new(3, 2), Point.new(4, 0)
    )
  end

  def test_eval_at_zero
    p = c.eval(0)
    assert_in_delta 0, p.x, DELTA
    assert_in_delta 0, p.y, DELTA
  end

  def test_eval_at_one
    p = c.eval(1)
    assert_in_delta 4, p.x, DELTA
    assert_in_delta 0, p.y, DELTA
  end

  def test_eval_symmetric_midpoint
    # Symmetric cubic: midpoint x should be 2
    p = c.eval(0.5)
    assert_in_delta 2, p.x, DELTA
  end

  def test_split_midpoints
    left, right = c.split(0.5)
    m = c.eval(0.5)
    assert_in_delta m.x, left.p3.x, DELTA
    assert_in_delta m.x, right.p0.x, DELTA
  end

  def test_polyline_straight
    straight = Bezier2D::CubicBezier.new(
      Point.new(0, 0), Point.new(1, 0), Point.new(2, 0), Point.new(3, 0)
    )
    pts = straight.polyline(0.1)
    assert_equal 2, pts.length
  end

  def test_bbox_contains_samples
    bb = c.bbox
    (0..20).each do |i|
      p = c.eval(i / 20.0)
      assert p.x >= bb.x - 1e-6
      assert p.x <= bb.x + bb.width + 1e-6
    end
  end
end
