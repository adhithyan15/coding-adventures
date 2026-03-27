import pytest

from coding_adventures_resistor import (
    Resistor,
    parallel_equivalent,
    series_equivalent,
    voltage_divider,
)


class TestConstruction:
    def test_stores_nominal_properties(self) -> None:
        resistor = Resistor(
            1_000.0, tolerance=0.01, tempco_ppm_per_c=100.0, power_rating_watts=0.25
        )

        assert resistor.resistance_ohms == 1_000.0
        assert resistor.tolerance == 0.01
        assert resistor.tempco_ppm_per_c == 100.0
        assert resistor.power_rating_watts == 0.25

    def test_rejects_zero_resistance(self) -> None:
        with pytest.raises(ValueError, match="Resistance must be > 0 ohms"):
            Resistor(0.0)

    def test_rejects_negative_tolerance(self) -> None:
        with pytest.raises(ValueError, match="Tolerance must be >= 0"):
            Resistor(100.0, tolerance=-0.01)

    def test_rejects_non_positive_power_rating(self) -> None:
        with pytest.raises(ValueError, match="Power rating must be > 0 watts"):
            Resistor(100.0, power_rating_watts=0.0)


class TestOhmsLaw:
    def test_conductance_is_inverse_of_resistance(self) -> None:
        resistor = Resistor(200.0)
        assert resistor.conductance() == pytest.approx(0.005, abs=1e-12)

    def test_current_for_voltage(self) -> None:
        resistor = Resistor(1_000.0)
        assert resistor.current_for_voltage(5.0) == pytest.approx(0.005, abs=1e-12)

    def test_voltage_for_current(self) -> None:
        resistor = Resistor(1_000.0)
        assert resistor.voltage_for_current(0.005) == pytest.approx(5.0, abs=1e-12)

    def test_negative_voltage_produces_negative_current(self) -> None:
        resistor = Resistor(100.0)
        assert resistor.current_for_voltage(-2.0) == pytest.approx(-0.02, abs=1e-12)


class TestPowerAndEnergy:
    def test_power_formulas_agree(self) -> None:
        resistor = Resistor(1_000.0)
        by_voltage = resistor.power_for_voltage(5.0)
        by_current = resistor.power_for_current(0.005)
        assert by_voltage == pytest.approx(0.025, abs=1e-12)
        assert by_current == pytest.approx(0.025, abs=1e-12)

    def test_energy_for_voltage(self) -> None:
        resistor = Resistor(100.0)
        assert resistor.energy_for_voltage(10.0, 2.0) == pytest.approx(2.0, abs=1e-12)

    def test_energy_rejects_negative_duration(self) -> None:
        resistor = Resistor(100.0)
        with pytest.raises(ValueError, match="Duration must be >= 0 seconds"):
            resistor.energy_for_current(0.1, -1.0)

    def test_power_rating_check_for_voltage(self) -> None:
        resistor = Resistor(100.0, power_rating_watts=0.25)
        assert resistor.is_within_power_rating_for_voltage(5.0)
        assert not resistor.is_within_power_rating_for_voltage(10.0)


class TestToleranceAndTemperature:
    def test_tolerance_bounds(self) -> None:
        resistor = Resistor(10_000.0, tolerance=0.01)
        assert resistor.min_resistance() == pytest.approx(9_900.0, abs=1e-12)
        assert resistor.max_resistance() == pytest.approx(10_100.0, abs=1e-12)

    def test_temperature_adjustment(self) -> None:
        resistor = Resistor(1_000.0, tempco_ppm_per_c=100.0)
        assert resistor.resistance_at_temperature(75.0) == pytest.approx(
            1_005.0, abs=1e-12
        )

    def test_temperature_adjustment_below_reference(self) -> None:
        resistor = Resistor(1_000.0, tempco_ppm_per_c=100.0)
        assert resistor.resistance_at_temperature(-25.0) == pytest.approx(
            995.0, abs=1e-12
        )


class TestNetworkHelpers:
    def test_series_equivalent(self) -> None:
        resistors = [Resistor(100.0), Resistor(200.0), Resistor(300.0)]
        assert series_equivalent(resistors) == pytest.approx(600.0, abs=1e-12)

    def test_parallel_equivalent(self) -> None:
        resistors = [Resistor(1_000.0), Resistor(1_000.0)]
        assert parallel_equivalent(resistors) == pytest.approx(500.0, abs=1e-12)

    def test_empty_series_rejected(self) -> None:
        with pytest.raises(ValueError, match="At least one resistor is required"):
            series_equivalent([])

    def test_voltage_divider(self) -> None:
        top = Resistor(1_000.0)
        bottom = Resistor(1_000.0)
        assert voltage_divider(5.0, top, bottom) == pytest.approx(2.5, abs=1e-12)
