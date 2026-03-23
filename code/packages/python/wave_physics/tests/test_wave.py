"""
Tests for the Wave class
=========================

We test the fundamental wave equation y(t) = A * sin(2*pi*f*t + phi)
by checking known values at specific times, verifying derived quantities,
and ensuring input validation catches bad parameters.
"""

import pytest

from trig import PI
from wave_physics import Wave


# ── Construction and properties ───────────────────────────────────────────


class TestWaveConstruction:
    """Verify that Wave stores parameters correctly."""

    def test_default_phase_is_zero(self):
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.phase == 0.0

    def test_stores_amplitude(self):
        w = Wave(amplitude=3.5, frequency=2.0)
        assert w.amplitude == 3.5

    def test_stores_frequency(self):
        w = Wave(amplitude=1.0, frequency=440.0)
        assert w.frequency == 440.0

    def test_stores_phase(self):
        w = Wave(amplitude=1.0, frequency=1.0, phase=1.23)
        assert w.phase == 1.23

    def test_zero_amplitude_allowed(self):
        """A zero-amplitude wave is a valid degenerate case (flat line)."""
        w = Wave(amplitude=0.0, frequency=1.0)
        assert w.amplitude == 0.0

    def test_repr(self):
        w = Wave(amplitude=2.0, frequency=3.0, phase=0.5)
        assert repr(w) == "Wave(amplitude=2.0, frequency=3.0, phase=0.5)"


# ── Validation ────────────────────────────────────────────────────────────


class TestWaveValidation:
    """Verify that invalid parameters are rejected."""

    def test_negative_amplitude_raises(self):
        with pytest.raises(ValueError, match="Amplitude must be >= 0"):
            Wave(amplitude=-1.0, frequency=1.0)

    def test_zero_frequency_raises(self):
        with pytest.raises(ValueError, match="Frequency must be > 0"):
            Wave(amplitude=1.0, frequency=0.0)

    def test_negative_frequency_raises(self):
        with pytest.raises(ValueError, match="Frequency must be > 0"):
            Wave(amplitude=1.0, frequency=-5.0)


# ── Derived quantities ───────────────────────────────────────────────────


class TestDerivedQuantities:
    """Verify period() and angular_frequency()."""

    def test_period_is_reciprocal_of_frequency(self):
        w = Wave(amplitude=1.0, frequency=4.0)
        assert w.period() == pytest.approx(0.25, abs=1e-10)

    def test_period_440hz(self):
        """Concert A (440 Hz) has a period of ~2.27 ms."""
        w = Wave(amplitude=1.0, frequency=440.0)
        assert w.period() == pytest.approx(1.0 / 440.0, abs=1e-10)

    def test_angular_frequency_1hz(self):
        """1 Hz -> omega = 2*PI rad/s."""
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.angular_frequency() == pytest.approx(2.0 * PI, abs=1e-10)

    def test_angular_frequency_general(self):
        """f Hz -> omega = 2*PI*f rad/s."""
        w = Wave(amplitude=1.0, frequency=10.0)
        assert w.angular_frequency() == pytest.approx(2.0 * PI * 10.0, abs=1e-10)


# ── evaluate() at known points ───────────────────────────────────────────


class TestEvaluate:
    """Test the wave equation at analytically known points."""

    def test_zero_phase_at_t_zero(self):
        """sin(0) = 0, so any wave with phase=0 evaluates to 0 at t=0."""
        w = Wave(amplitude=5.0, frequency=100.0)
        assert w.evaluate(0.0) == pytest.approx(0.0, abs=1e-10)

    def test_peak_at_quarter_period(self):
        """At t = T/4, the angle is PI/2, so sin = 1 and y = amplitude."""
        w = Wave(amplitude=1.0, frequency=1.0)
        # T = 1.0, so T/4 = 0.25
        assert w.evaluate(0.25) == pytest.approx(1.0, abs=1e-10)

    def test_zero_at_half_period(self):
        """At t = T/2, the angle is PI, so sin = 0."""
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.evaluate(0.5) == pytest.approx(0.0, abs=1e-10)

    def test_trough_at_three_quarter_period(self):
        """At t = 3T/4, the angle is 3*PI/2, so sin = -1."""
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.evaluate(0.75) == pytest.approx(-1.0, abs=1e-10)

    def test_full_period_returns_to_zero(self):
        """At t = T, the angle is 2*PI, so sin = 0 (back to start)."""
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.evaluate(1.0) == pytest.approx(0.0, abs=1e-10)

    def test_amplitude_scaling(self):
        """The peak value should equal the amplitude."""
        w = Wave(amplitude=3.0, frequency=2.0)
        # T = 0.5, T/4 = 0.125
        assert w.evaluate(0.125) == pytest.approx(3.0, abs=1e-10)

    def test_higher_frequency(self):
        """A 10 Hz wave reaches its peak at t = 1/40 = 0.025 s."""
        w = Wave(amplitude=2.0, frequency=10.0)
        # T = 0.1, T/4 = 0.025
        assert w.evaluate(0.025) == pytest.approx(2.0, abs=1e-10)


# ── Phase offset tests ──────────────────────────────────────────────────


class TestPhaseOffset:
    """Verify that the phase parameter shifts the wave correctly."""

    def test_phase_pi_half_starts_at_peak(self):
        """With phase = PI/2, sin(PI/2) = 1, so evaluate(0) = amplitude."""
        w = Wave(amplitude=1.0, frequency=1.0, phase=PI / 2)
        assert w.evaluate(0.0) == pytest.approx(1.0, abs=1e-10)

    def test_phase_pi_starts_at_zero_going_down(self):
        """With phase = PI, sin(PI) = 0, and the wave descends."""
        w = Wave(amplitude=1.0, frequency=1.0, phase=PI)
        assert w.evaluate(0.0) == pytest.approx(0.0, abs=1e-10)

    def test_phase_3pi_half_starts_at_trough(self):
        """With phase = 3*PI/2, sin(3*PI/2) = -1."""
        w = Wave(amplitude=1.0, frequency=1.0, phase=3 * PI / 2)
        assert w.evaluate(0.0) == pytest.approx(-1.0, abs=1e-10)

    def test_phase_with_amplitude(self):
        """Phase PI/2 with amplitude 5 -> evaluate(0) = 5."""
        w = Wave(amplitude=5.0, frequency=1.0, phase=PI / 2)
        assert w.evaluate(0.0) == pytest.approx(5.0, abs=1e-10)


# ── Periodicity tests ───────────────────────────────────────────────────


class TestPeriodicity:
    """A wave must repeat: y(t) == y(t + T) for any t."""

    def test_periodicity_at_arbitrary_time(self):
        """Value at t should equal value at t + period."""
        w = Wave(amplitude=2.5, frequency=3.0, phase=0.7)
        t = 0.137  # arbitrary time
        T = w.period()
        assert w.evaluate(t) == pytest.approx(w.evaluate(t + T), abs=1e-10)

    def test_periodicity_multiple_periods(self):
        """Value at t should equal value at t + N*T for any integer N."""
        w = Wave(amplitude=1.0, frequency=5.0)
        t = 0.042
        T = w.period()
        for n in range(1, 5):
            assert w.evaluate(t) == pytest.approx(
                w.evaluate(t + n * T), abs=1e-10
            ), f"Failed at n={n}"

    def test_periodicity_with_phase(self):
        """Periodicity holds regardless of phase offset."""
        w = Wave(amplitude=1.0, frequency=2.0, phase=PI / 3)
        t = 0.08
        T = w.period()
        assert w.evaluate(t) == pytest.approx(w.evaluate(t + T), abs=1e-10)


# ── Zero-amplitude edge case ────────────────────────────────────────────


class TestZeroAmplitude:
    """A zero-amplitude wave is always zero, regardless of time or phase."""

    def test_always_zero(self):
        w = Wave(amplitude=0.0, frequency=1.0, phase=PI / 2)
        for t in [0.0, 0.25, 0.5, 0.75, 1.0]:
            assert w.evaluate(t) == pytest.approx(0.0, abs=1e-10)
