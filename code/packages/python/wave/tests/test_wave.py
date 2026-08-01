import math
import sys

import pytest
from coding_adventures_wave import Wave
from trig import PI


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

    @pytest.mark.parametrize(
        ("amplitude", "frequency", "phase"),
        [
            (math.nan, 1.0, 0.0),
            (math.inf, 1.0, 0.0),
            (1.0, math.nan, 0.0),
            (1.0, math.inf, 0.0),
            (1.0, 1.0, math.nan),
            (1.0, 1.0, math.inf),
        ],
    )
    def test_nonfinite_parameters_raise(
        self, amplitude: float, frequency: float, phase: float
    ) -> None:
        with pytest.raises(ValueError, match="finite"):
            Wave(amplitude=amplitude, frequency=frequency, phase=phase)

    def test_angular_frequency_overflow_raises(self) -> None:
        with pytest.raises(ValueError, match="Angular frequency"):
            Wave(amplitude=1.0, frequency=sys.float_info.max)


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

    @pytest.mark.parametrize("time", [math.nan, math.inf, -math.inf])
    def test_nonfinite_time_raises(self, time: float) -> None:
        w = Wave(amplitude=0.0, frequency=1.0)
        with pytest.raises(ValueError, match="Time must be finite"):
            w.evaluate(time)

    def test_extreme_inputs_stay_finite_and_bounded(self) -> None:
        zero = Wave(
            amplitude=0.0,
            frequency=1e300,
            phase=PI / 2,
        )
        result = zero.evaluate(sys.float_info.max)
        assert result == 0.0
        assert math.copysign(1.0, result) == 1.0

        wave = Wave(
            amplitude=sys.float_info.max,
            frequency=1e300,
            phase=PI / 2,
        )
        result = wave.evaluate(sys.float_info.max)
        assert math.isfinite(result)
        assert abs(result) <= sys.float_info.max

        subnormal = Wave(
            amplitude=1.0,
            frequency=float.fromhex("0x0.0000000000001p-1022"),
            phase=sys.float_info.max,
        )
        assert math.isinf(subnormal.period())
        result = subnormal.evaluate(sys.float_info.max)
        assert math.isfinite(result)
        assert abs(result) <= 1.0


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
