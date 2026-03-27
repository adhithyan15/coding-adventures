from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Resistor:
    """Ideal resistor with a few first-order real-world properties."""

    resistance_ohms: float
    tolerance: float = 0.0
    tempco_ppm_per_c: float = 0.0
    power_rating_watts: float | None = None

    def __post_init__(self) -> None:
        if self.resistance_ohms <= 0:
            raise ValueError(
                f"Resistance must be > 0 ohms, got {self.resistance_ohms}."
            )

        if self.tolerance < 0:
            raise ValueError(f"Tolerance must be >= 0, got {self.tolerance}.")

        if self.power_rating_watts is not None and self.power_rating_watts <= 0:
            raise ValueError(
                "Power rating must be > 0 watts when provided, "
                f"got {self.power_rating_watts}."
            )

    def conductance(self) -> float:
        return 1.0 / self.resistance_ohms

    def current_for_voltage(self, voltage: float) -> float:
        return voltage / self.resistance_ohms

    def voltage_for_current(self, current: float) -> float:
        return current * self.resistance_ohms

    def power_for_voltage(self, voltage: float) -> float:
        return (voltage * voltage) / self.resistance_ohms

    def power_for_current(self, current: float) -> float:
        return (current * current) * self.resistance_ohms

    def energy_for_voltage(self, voltage: float, duration_seconds: float) -> float:
        _validate_duration(duration_seconds)
        return self.power_for_voltage(voltage) * duration_seconds

    def energy_for_current(self, current: float, duration_seconds: float) -> float:
        _validate_duration(duration_seconds)
        return self.power_for_current(current) * duration_seconds

    def min_resistance(self) -> float:
        return self.resistance_ohms * (1.0 - self.tolerance)

    def max_resistance(self) -> float:
        return self.resistance_ohms * (1.0 + self.tolerance)

    def resistance_at_temperature(
        self, celsius: float, reference_celsius: float = 25.0
    ) -> float:
        alpha = self.tempco_ppm_per_c * 1e-6
        delta_t = celsius - reference_celsius
        return self.resistance_ohms * (1.0 + alpha * delta_t)

    def is_within_power_rating_for_voltage(self, voltage: float) -> bool:
        if self.power_rating_watts is None:
            return True
        return self.power_for_voltage(voltage) <= self.power_rating_watts

    def is_within_power_rating_for_current(self, current: float) -> bool:
        if self.power_rating_watts is None:
            return True
        return self.power_for_current(current) <= self.power_rating_watts


def series_equivalent(resistors: list[Resistor]) -> float:
    _validate_resistor_list(resistors)
    return sum(resistor.resistance_ohms for resistor in resistors)


def parallel_equivalent(resistors: list[Resistor]) -> float:
    _validate_resistor_list(resistors)
    reciprocal_sum = sum(1.0 / resistor.resistance_ohms for resistor in resistors)
    return 1.0 / reciprocal_sum


def voltage_divider(vin: float, r_top: Resistor, r_bottom: Resistor) -> float:
    total = r_top.resistance_ohms + r_bottom.resistance_ohms
    return vin * (r_bottom.resistance_ohms / total)


def _validate_duration(duration_seconds: float) -> None:
    if duration_seconds < 0:
        raise ValueError(
            f"Duration must be >= 0 seconds, got {duration_seconds}."
        )


def _validate_resistor_list(resistors: list[Resistor]) -> None:
    if not resistors:
        raise ValueError("At least one resistor is required.")
