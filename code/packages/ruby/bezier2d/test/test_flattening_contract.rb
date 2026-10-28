# frozen_string_literal: true

require "json"
require "minitest/autorun"
require_relative "../lib/bezier2d"

class TestSafeFlatteningContract < Minitest::Test
  include Point2D

  FIXTURE_PATH = File.expand_path(
    "../../../../specs/fixtures/bezier2d-flattening-v1/cases.json", __dir__
  )
  FIXTURE = JSON.parse(File.read(FIXTURE_PATH, encoding: "UTF-8"))
  CASES = FIXTURE.fetch("cases")

  def test_fixture_metadata_matches_implementation_limits
    assert_equal 1, FIXTURE.fetch("version")
    assert_equal 32, FIXTURE.fetch("max_depth")
    assert_equal 65_535, FIXTURE.fetch("max_subdivisions")
  end

  def curve_for(test_case)
    points = test_case.fetch("control_points").map { |x, y| Point.new(x, y) }
    if test_case.fetch("degree") == 2
      Bezier2D::QuadraticBezier.new(*points)
    else
      Bezier2D::CubicBezier.new(*points)
    end
  end

  def test_neutral_flattening_cases
    CASES.each do |test_case|
      curve = curve_for(test_case)
      disposition = test_case.fetch("expected_disposition")
      if disposition == "invalid-tolerance"
        error = assert_raises(ArgumentError, test_case.fetch("id")) do
          curve.polyline(test_case.fetch("tolerance"))
        end
        assert_match(/tolerance/, error.message)
        next
      end
      if test_case["expected_termination"] == "budget-error"
        error = assert_raises(RuntimeError, test_case.fetch("id")) do
          curve.polyline(test_case.fetch("tolerance"))
        end
        assert_match(/budget/, error.message)
        next
      end

      points = curve.polyline(test_case.fetch("tolerance"))
      assert_equal curve.p0, points.first, test_case.fetch("id")
      assert_equal(test_case.fetch("degree") == 2 ? curve.p2 : curve.p3,
                   points.last, test_case.fetch("id"))
      assert points.all? { |point| point.x.finite? && point.y.finite? }
      disposition == "chord" ? assert_equal(2, points.length) : assert_operator(points.length, :>, 2)
    end
  end

  def test_tolerance_must_be_finite_and_positive
    curve = Bezier2D::QuadraticBezier.new(Point.new(0, 0), Point.new(0.5, 0), Point.new(1, 0))
    [0.0, -1.0, Float::INFINITY, -Float::INFINITY, Float::NAN].each do |tolerance|
      error = assert_raises(ArgumentError) { curve.polyline(tolerance) }
      assert_match(/tolerance/, error.message)
    end
  end

  def test_control_points_must_be_finite
    [Float::INFINITY, -Float::INFINITY, Float::NAN].each do |coordinate|
      curve = Bezier2D::CubicBezier.new(
        Point.new(0, 0), Point.new(coordinate, 1), Point.new(0, -1), Point.new(0, 0)
      )
      error = assert_raises(ArgumentError) { curve.polyline(0.1) }
      assert_match(/control/, error.message)
    end
  end

  def test_nonfinite_derived_geometry_fails_without_a_result
    curve = Bezier2D::QuadraticBezier.new(
      Point.new(1e308, 0), Point.new(0, 1), Point.new(-1e308, 0)
    )
    result = nil
    error = assert_raises(RuntimeError) { result = curve.polyline(0.1) }
    assert_match(/finite/, error.message)
    assert_nil result
  end

  def test_subnormal_geometry_cannot_underflow_into_a_chord
    curve = Bezier2D::QuadraticBezier.new(
      Point.new(0, 0), Point.new(0, 1e-200), Point.new(1e-200, 0)
    )
    error = assert_raises(RuntimeError) { curve.polyline(5e-324) }
    assert_match(/budget/, error.message)
  end
end
