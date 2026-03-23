# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/trig"

# =============================================================================
# Tests for the Trig module
# =============================================================================
#
# These tests verify our from-scratch Maclaurin series implementations of
# sine and cosine against known mathematical identities and special values.
#
# We use assert_in_delta with a tolerance of 1e-10, which is well within the
# precision of our 20-term series approximation for range-reduced inputs.

class TestTrig < Minitest::Test
  DELTA = 1e-10

  # ---------------------------------------------------------------------------
  # sin(x) — Special Values
  # ---------------------------------------------------------------------------

  def test_sin_zero
    # sin(0) = 0 — the origin of the unit circle
    assert_in_delta 0.0, Trig.sin(0), DELTA
  end

  def test_sin_pi_over_2
    # sin(PI/2) = 1 — the top of the unit circle
    assert_in_delta 1.0, Trig.sin(Trig::PI / 2), DELTA
  end

  def test_sin_pi
    # sin(PI) = 0 — halfway around the unit circle
    assert_in_delta 0.0, Trig.sin(Trig::PI), DELTA
  end

  def test_sin_3pi_over_2
    # sin(3*PI/2) = -1 — the bottom of the unit circle
    assert_in_delta(-1.0, Trig.sin(3 * Trig::PI / 2), DELTA)
  end

  def test_sin_two_pi
    # sin(2*PI) = 0 — full circle, back to the start
    assert_in_delta 0.0, Trig.sin(Trig::TWO_PI), DELTA
  end

  def test_sin_pi_over_6
    # sin(PI/6) = 0.5 — a well-known exact value (30 degrees)
    assert_in_delta 0.5, Trig.sin(Trig::PI / 6), DELTA
  end

  def test_sin_pi_over_4
    # sin(PI/4) = sqrt(2)/2 — the 45-degree angle
    assert_in_delta Math.sqrt(2) / 2, Trig.sin(Trig::PI / 4), DELTA
  end

  # ---------------------------------------------------------------------------
  # cos(x) — Special Values
  # ---------------------------------------------------------------------------

  def test_cos_zero
    # cos(0) = 1 — the starting point on the unit circle
    assert_in_delta 1.0, Trig.cos(0), DELTA
  end

  def test_cos_pi_over_2
    # cos(PI/2) = 0 — perpendicular to the x-axis
    assert_in_delta 0.0, Trig.cos(Trig::PI / 2), DELTA
  end

  def test_cos_pi
    # cos(PI) = -1 — opposite side of the unit circle
    assert_in_delta(-1.0, Trig.cos(Trig::PI), DELTA)
  end

  def test_cos_3pi_over_2
    # cos(3*PI/2) = 0
    assert_in_delta 0.0, Trig.cos(3 * Trig::PI / 2), DELTA
  end

  def test_cos_two_pi
    # cos(2*PI) = 1 — full circle
    assert_in_delta 1.0, Trig.cos(Trig::TWO_PI), DELTA
  end

  def test_cos_pi_over_3
    # cos(PI/3) = 0.5 — a well-known exact value (60 degrees)
    assert_in_delta 0.5, Trig.cos(Trig::PI / 3), DELTA
  end

  def test_cos_pi_over_4
    # cos(PI/4) = sqrt(2)/2 — the 45-degree angle
    assert_in_delta Math.sqrt(2) / 2, Trig.cos(Trig::PI / 4), DELTA
  end

  # ---------------------------------------------------------------------------
  # Odd/Even Symmetry
  # ---------------------------------------------------------------------------
  #
  # sin(x) is an odd function:  sin(-x) = -sin(x)
  # cos(x) is an even function: cos(-x) =  cos(x)
  #
  # These are fundamental properties that follow directly from the series:
  # - sin has only odd powers of x, so negating x negates the result
  # - cos has only even powers of x, so negating x has no effect

  def test_sin_odd_symmetry
    [0.5, 1.0, 2.0, Trig::PI / 3, Trig::PI / 7].each do |x|
      assert_in_delta(-Trig.sin(x), Trig.sin(-x), DELTA,
        "sin(-#{x}) should equal -sin(#{x})")
    end
  end

  def test_cos_even_symmetry
    [0.5, 1.0, 2.0, Trig::PI / 3, Trig::PI / 7].each do |x|
      assert_in_delta Trig.cos(x), Trig.cos(-x), DELTA,
        "cos(-#{x}) should equal cos(#{x})"
    end
  end

  # ---------------------------------------------------------------------------
  # Pythagorean Identity: sin^2(x) + cos^2(x) = 1
  # ---------------------------------------------------------------------------
  #
  # This is arguably the most important identity in trigonometry. It holds
  # for ALL values of x, and it follows from the definition of sine and
  # cosine on the unit circle (a circle of radius 1).

  def test_pythagorean_identity
    values = [0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0,
              Trig::PI / 6, Trig::PI / 4, Trig::PI / 3,
              Trig::PI / 2, Trig::PI, -1.0, -2.5]

    values.each do |x|
      sin_sq = Trig.sin(x) ** 2
      cos_sq = Trig.cos(x) ** 2
      assert_in_delta 1.0, sin_sq + cos_sq, DELTA,
        "sin^2(#{x}) + cos^2(#{x}) should equal 1"
    end
  end

  # ---------------------------------------------------------------------------
  # Large Inputs (testing range reduction)
  # ---------------------------------------------------------------------------
  #
  # These tests verify that range reduction works correctly for large inputs.
  # Without range reduction, computing sin(1000*PI) with a Maclaurin series
  # would require thousands of terms. With range reduction, 20 terms suffice.

  def test_sin_large_multiple_of_pi
    # sin(1000*PI) should be very close to 0
    assert_in_delta 0.0, Trig.sin(1000 * Trig::PI), 1e-8
  end

  def test_cos_large_multiple_of_pi
    # cos(1000*PI) = cos(0) = 1 (1000 is even)
    assert_in_delta 1.0, Trig.cos(1000 * Trig::PI), 1e-8
  end

  def test_sin_large_negative
    # sin(-500*PI) should be close to 0
    assert_in_delta 0.0, Trig.sin(-500 * Trig::PI), 1e-8
  end

  def test_pythagorean_identity_large_input
    # The identity should hold even for large inputs
    x = 12345.6789
    sin_sq = Trig.sin(x) ** 2
    cos_sq = Trig.cos(x) ** 2
    assert_in_delta 1.0, sin_sq + cos_sq, DELTA
  end

  # ---------------------------------------------------------------------------
  # Degree/Radian Conversion
  # ---------------------------------------------------------------------------

  def test_radians_180_is_pi
    assert_in_delta Trig::PI, Trig.radians(180), DELTA
  end

  def test_degrees_pi_is_180
    assert_in_delta 180.0, Trig.degrees(Trig::PI), DELTA
  end

  def test_radians_0
    assert_in_delta 0.0, Trig.radians(0), DELTA
  end

  def test_degrees_0
    assert_in_delta 0.0, Trig.degrees(0), DELTA
  end

  def test_radians_90_is_pi_over_2
    assert_in_delta Trig::PI / 2, Trig.radians(90), DELTA
  end

  def test_degrees_pi_over_2_is_90
    assert_in_delta 90.0, Trig.degrees(Trig::PI / 2), DELTA
  end

  def test_radians_360_is_two_pi
    assert_in_delta Trig::TWO_PI, Trig.radians(360), DELTA
  end

  def test_degrees_two_pi_is_360
    assert_in_delta 360.0, Trig.degrees(Trig::TWO_PI), DELTA
  end

  def test_radians_negative
    assert_in_delta(-Trig::PI / 2, Trig.radians(-90), DELTA)
  end

  def test_degrees_negative
    assert_in_delta(-180.0, Trig.degrees(-Trig::PI), DELTA)
  end

  # ---------------------------------------------------------------------------
  # Round-trip: radians(degrees(x)) == x
  # ---------------------------------------------------------------------------

  def test_round_trip_conversion
    [0, 30, 45, 60, 90, 180, 270, 360, -45, -90].each do |deg|
      assert_in_delta deg.to_f, Trig.degrees(Trig.radians(deg)), DELTA,
        "Round-trip failed for #{deg} degrees"
    end
  end

  # ---------------------------------------------------------------------------
  # Integration: sin and cos with degree inputs
  # ---------------------------------------------------------------------------

  def test_sin_of_30_degrees
    assert_in_delta 0.5, Trig.sin(Trig.radians(30)), DELTA
  end

  def test_cos_of_60_degrees
    assert_in_delta 0.5, Trig.cos(Trig.radians(60)), DELTA
  end

  def test_sin_of_45_degrees
    assert_in_delta Math.sqrt(2) / 2, Trig.sin(Trig.radians(45)), DELTA
  end

  def test_cos_of_45_degrees
    assert_in_delta Math.sqrt(2) / 2, Trig.cos(Trig.radians(45)), DELTA
  end
end
