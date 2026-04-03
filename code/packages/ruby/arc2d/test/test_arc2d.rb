# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/arc2d"

class TestCenterArc < Minitest::Test
  DELTA = 1e-6

  def unit_arc
    @unit_arc ||= Arc2D::CenterArc.new(
      Point2D::Point.new(0, 0),
      1, 1,
      0, Trig::PI / 2,
      0
    )
  end

  def test_eval_at_zero
    p = unit_arc.eval(0)
    assert_in_delta 1, p.x, DELTA
    assert_in_delta 0, p.y, DELTA
  end

  def test_eval_at_one
    p = unit_arc.eval(1)
    assert_in_delta 0, p.x, DELTA
    assert_in_delta 1, p.y, DELTA
  end

  def test_eval_midpoint
    p = unit_arc.eval(0.5)
    expected = 1.0 / Math.sqrt(2)
    assert_in_delta expected, p.x, DELTA
    assert_in_delta expected, p.y, DELTA
  end

  def test_bbox_full_circle
    full = Arc2D::CenterArc.new(
      Point2D::Point.new(0, 0), 1, 1, 0, Trig::TWO_PI, 0
    )
    bb = full.bbox
    assert bb.x <= -0.99
    assert bb.x + bb.width >= 0.99
  end

  def test_to_cubic_beziers_endpoints
    curves = unit_arc.to_cubic_beziers
    refute_empty curves
    assert_in_delta 1, curves.first.p0.x, DELTA
    assert_in_delta 0, curves.first.p0.y, DELTA
    assert_in_delta 0, curves.last.p3.x, DELTA
    assert_in_delta 1, curves.last.p3.y, DELTA
  end
end

class TestSvgArc < Minitest::Test
  DELTA = 1e-6

  def test_degenerate_same_point
    arc = Arc2D::SvgArc.new(
      Point2D::Point.new(1, 1), Point2D::Point.new(1, 1),
      1, 1, 0, false, false
    )
    assert_nil arc.to_center_arc
  end

  def test_degenerate_zero_radius
    arc = Arc2D::SvgArc.new(
      Point2D::Point.new(0, 0), Point2D::Point.new(1, 0),
      0, 1, 0, false, false
    )
    assert_nil arc.to_center_arc
  end

  def test_semicircle_center
    # From (1,0) to (-1,0) with r=1, sweep=true → center at (0,0)
    arc = Arc2D::SvgArc.new(
      Point2D::Point.new(1, 0), Point2D::Point.new(-1, 0),
      1, 1, 0, false, true
    )
    ca = arc.to_center_arc
    refute_nil ca
    assert_in_delta 0, ca.center.x, DELTA
    assert_in_delta 0, ca.center.y, DELTA
    assert_in_delta 1, ca.rx, DELTA
  end

  def test_semicircle_endpoints_reproduced
    arc = Arc2D::SvgArc.new(
      Point2D::Point.new(1, 0), Point2D::Point.new(-1, 0),
      1, 1, 0, false, true
    )
    ca = arc.to_center_arc
    refute_nil ca
    p0 = ca.eval(0)
    p1 = ca.eval(1)
    assert_in_delta 1, p0.x, DELTA
    assert_in_delta 0, p0.y, DELTA
    assert_in_delta(-1, p1.x, DELTA)
    assert_in_delta 0, p1.y, DELTA
  end
end
