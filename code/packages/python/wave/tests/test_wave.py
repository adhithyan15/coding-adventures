import pytest

from trig import PI
from coding_adventures_wave import Wave


class TestWaveConstruction:
    def test_default_phase_is_zero(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.phase == 0.0

    def test_stores_parameters(self) -> None:
        w = Wave(amplitude=3.5, frequency=2.0, phase=1.25)
        assert w.amplitude == 3.5
        assert w.frequency == 2.0
        assert w.phase == 1.25

    def test_zero_amplitude_allowed(self) -> None:
        w = Wave(amplitude=0.0, frequency=1.0)
        assert w.amplitude == 0.0

    def test_repr(self) -> None:
        w = Wave(amplitude=2.0, frequency=3.0, phase=0.5)
        assert repr(w) == "Wave(amplitude=2.0, frequency=3.0, phase=0.5)"


class TestWaveValidation:
    def test_negative_amplitude_raises(self) -> None:
        with pytest.raises(ValueError, match="Amplitude must be >= 0"):
            Wave(amplitude=-1.0, frequency=1.0)

    def test_zero_frequency_raises(self) -> None:
        with pytest.raises(ValueError, match="Frequency must be > 0"):
            Wave(amplitude=1.0, frequency=0.0)

    def test_negative_frequency_raises(self) -> None:
        with pytest.raises(ValueError, match="Frequency must be > 0"):
            Wave(amplitude=1.0, frequency=-5.0)


class TestDerivedQuantities:
    def test_period(self) -> None:
        w = Wave(amplitude=1.0, frequency=4.0)
        assert w.period() == pytest.approx(0.25, abs=1e-10)

    def test_angular_frequency(self) -> None:
        w = Wave(amplitude=1.0, frequency=10.0)
        assert w.angular_frequency() == pytest.approx(2.0 * PI * 10.0, abs=1e-10)


class TestEvaluate:
    def test_zero_phase_at_t_zero(self) -> None:
        w = Wave(amplitude=5.0, frequency=100.0)
        assert w.evaluate(0.0) == pytest.approx(0.0, abs=1e-10)

    def test_peak_at_quarter_period(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.evaluate(0.25) == pytest.approx(1.0, abs=1e-10)

    def test_zero_at_half_period(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.evaluate(0.5) == pytest.approx(0.0, abs=1e-10)

    def test_trough_at_three_quarter_period(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0)
        assert w.evaluate(0.75) == pytest.approx(-1.0, abs=1e-10)

    def test_periodicity(self) -> None:
        w = Wave(amplitude=2.5, frequency=3.0, phase=0.7)
        t = 0.137
        assert w.evaluate(t) == pytest.approx(w.evaluate(t + w.period()), abs=1e-10)


class TestPhaseOffset:
    def test_phase_pi_half_starts_at_peak(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0, phase=PI / 2)
        assert w.evaluate(0.0) == pytest.approx(1.0, abs=1e-10)

    def test_phase_3pi_half_starts_at_trough(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0, phase=3 * PI / 2)
        assert w.evaluate(0.0) == pytest.approx(-1.0, abs=1e-10)


class TestZeroAmplitude:
    def test_always_zero(self) -> None:
        w = Wave(amplitude=0.0, frequency=1.0, phase=PI / 2)
        for t in [0.0, 0.25, 0.5, 0.75, 1.0]:
            assert w.evaluate(t) == pytest.approx(0.0, abs=1e-10)
