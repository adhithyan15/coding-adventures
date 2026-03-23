"""
Tests for the trig package.

These tests verify our from-scratch Taylor series implementations of sin and
cos against known mathematical identities and special values.  We use
pytest.approx with an absolute tolerance of 1e-10 throughout.
"""

import pytest

from trig import PI, sin, cos, radians, degrees


# ---------------------------------------------------------------------------
# Tolerance used for all approximate comparisons
# ---------------------------------------------------------------------------

TOL = 1e-10


# ---------------------------------------------------------------------------
# Special values — sin
# ---------------------------------------------------------------------------

class TestSinSpecialValues:
    """Verify sin at well-known angles."""

    def test_sin_zero(self) -> None:
        """sin(0) is exactly 0."""
        assert sin(0) == 0

    def test_sin_pi_over_2(self) -> None:
        """sin(pi/2) = 1."""
        assert sin(PI / 2) == pytest.approx(1.0, abs=TOL)

    def test_sin_pi(self) -> None:
        """sin(pi) = 0."""
        assert sin(PI) == pytest.approx(0.0, abs=TOL)

    def test_sin_3pi_over_2(self) -> None:
        """sin(3*pi/2) = -1."""
        assert sin(3 * PI / 2) == pytest.approx(-1.0, abs=TOL)

    def test_sin_2pi(self) -> None:
        """sin(2*pi) = 0."""
        assert sin(2 * PI) == pytest.approx(0.0, abs=TOL)

    def test_sin_pi_over_6(self) -> None:
        """sin(pi/6) = 0.5."""
        assert sin(PI / 6) == pytest.approx(0.5, abs=TOL)

    def test_sin_pi_over_4(self) -> None:
        """sin(pi/4) = sqrt(2)/2."""
        assert sin(PI / 4) == pytest.approx(0.7071067811865476, abs=TOL)

    def test_sin_pi_over_3(self) -> None:
        """sin(pi/3) = sqrt(3)/2."""
        assert sin(PI / 3) == pytest.approx(0.8660254037844386, abs=TOL)


# ---------------------------------------------------------------------------
# Special values — cos
# ---------------------------------------------------------------------------

class TestCosSpecialValues:
    """Verify cos at well-known angles."""

    def test_cos_zero(self) -> None:
        """cos(0) is exactly 1."""
        assert cos(0) == 1.0

    def test_cos_pi_over_2(self) -> None:
        """cos(pi/2) = 0."""
        assert cos(PI / 2) == pytest.approx(0.0, abs=TOL)

    def test_cos_pi(self) -> None:
        """cos(pi) = -1."""
        assert cos(PI) == pytest.approx(-1.0, abs=TOL)

    def test_cos_3pi_over_2(self) -> None:
        """cos(3*pi/2) = 0."""
        assert cos(3 * PI / 2) == pytest.approx(0.0, abs=TOL)

    def test_cos_2pi(self) -> None:
        """cos(2*pi) = 1."""
        assert cos(2 * PI) == pytest.approx(1.0, abs=TOL)

    def test_cos_pi_over_6(self) -> None:
        """cos(pi/6) = sqrt(3)/2."""
        assert cos(PI / 6) == pytest.approx(0.8660254037844386, abs=TOL)

    def test_cos_pi_over_4(self) -> None:
        """cos(pi/4) = sqrt(2)/2."""
        assert cos(PI / 4) == pytest.approx(0.7071067811865476, abs=TOL)

    def test_cos_pi_over_3(self) -> None:
        """cos(pi/3) = 0.5."""
        assert cos(PI / 3) == pytest.approx(0.5, abs=TOL)


# ---------------------------------------------------------------------------
# Symmetry properties
# ---------------------------------------------------------------------------

class TestSymmetry:
    """Verify fundamental symmetry identities."""

    def test_sin_is_odd(self) -> None:
        """sin(-x) = -sin(x) — sine is an odd function."""
        for x in [0.5, 1.0, 1.5, 2.0, 2.7, PI / 4, PI / 3]:
            assert sin(-x) == pytest.approx(-sin(x), abs=TOL)

    def test_cos_is_even(self) -> None:
        """cos(-x) = cos(x) — cosine is an even function."""
        for x in [0.5, 1.0, 1.5, 2.0, 2.7, PI / 4, PI / 3]:
            assert cos(-x) == pytest.approx(cos(x), abs=TOL)


# ---------------------------------------------------------------------------
# Pythagorean identity
# ---------------------------------------------------------------------------

class TestPythagoreanIdentity:
    """sin^2(x) + cos^2(x) = 1 for all x."""

    def test_pythagorean_identity(self) -> None:
        """Check the identity at many angles."""
        test_angles = [
            0, PI / 6, PI / 4, PI / 3, PI / 2, PI,
            3 * PI / 2, 2 * PI, -1.0, -2.5, 0.1, 3.0, 5.5,
        ]
        for x in test_angles:
            s = sin(x)
            c = cos(x)
            assert s * s + c * c == pytest.approx(1.0, abs=TOL)


# ---------------------------------------------------------------------------
# Large inputs (tests range reduction)
# ---------------------------------------------------------------------------

class TestLargeInputs:
    """Verify correctness for large angles, which stress range reduction."""

    def test_sin_1000_pi(self) -> None:
        """sin(1000*pi) should be approximately 0."""
        assert sin(1000 * PI) == pytest.approx(0.0, abs=TOL)

    def test_cos_1000_pi(self) -> None:
        """cos(1000*pi) should be approximately 1 or -1.
        1000 is even, so cos(1000*pi) = cos(0) = 1."""
        assert cos(1000 * PI) == pytest.approx(1.0, abs=TOL)

    def test_sin_large_positive(self) -> None:
        """sin(100) should match the Pythagorean identity."""
        s = sin(100)
        c = cos(100)
        assert s * s + c * c == pytest.approx(1.0, abs=TOL)

    def test_sin_large_negative(self) -> None:
        """sin(-100) = -sin(100)."""
        assert sin(-100) == pytest.approx(-sin(100), abs=TOL)


# ---------------------------------------------------------------------------
# Degree / Radian conversions
# ---------------------------------------------------------------------------

class TestConversions:
    """Verify degree-radian conversion functions."""

    def test_radians_180(self) -> None:
        """180 degrees = pi radians."""
        assert radians(180) == pytest.approx(PI, abs=TOL)

    def test_radians_90(self) -> None:
        """90 degrees = pi/2 radians."""
        assert radians(90) == pytest.approx(PI / 2, abs=TOL)

    def test_radians_360(self) -> None:
        """360 degrees = 2*pi radians."""
        assert radians(360) == pytest.approx(2 * PI, abs=TOL)

    def test_radians_0(self) -> None:
        """0 degrees = 0 radians."""
        assert radians(0) == 0.0

    def test_degrees_pi(self) -> None:
        """pi radians = 180 degrees."""
        assert degrees(PI) == pytest.approx(180.0, abs=TOL)

    def test_degrees_pi_over_2(self) -> None:
        """pi/2 radians = 90 degrees."""
        assert degrees(PI / 2) == pytest.approx(90.0, abs=TOL)

    def test_degrees_0(self) -> None:
        """0 radians = 0 degrees."""
        assert degrees(0) == 0.0

    def test_round_trip_degrees_to_radians(self) -> None:
        """Converting degrees -> radians -> degrees should be identity."""
        for deg in [0, 30, 45, 60, 90, 120, 180, 270, 360]:
            assert degrees(radians(deg)) == pytest.approx(float(deg), abs=TOL)

    def test_round_trip_radians_to_degrees(self) -> None:
        """Converting radians -> degrees -> radians should be identity."""
        for rad in [0, PI / 6, PI / 4, PI / 3, PI / 2, PI, 2 * PI]:
            assert radians(degrees(rad)) == pytest.approx(rad, abs=TOL)


# ---------------------------------------------------------------------------
# Integration: sin and cos with degree input
# ---------------------------------------------------------------------------

class TestWithDegrees:
    """Use radians() to feed degree values into sin/cos."""

    def test_sin_30_degrees(self) -> None:
        """sin(30 degrees) = 0.5."""
        assert sin(radians(30)) == pytest.approx(0.5, abs=TOL)

    def test_cos_60_degrees(self) -> None:
        """cos(60 degrees) = 0.5."""
        assert cos(radians(60)) == pytest.approx(0.5, abs=TOL)

    def test_sin_45_degrees(self) -> None:
        """sin(45 degrees) = sqrt(2)/2."""
        assert sin(radians(45)) == pytest.approx(0.7071067811865476, abs=TOL)
