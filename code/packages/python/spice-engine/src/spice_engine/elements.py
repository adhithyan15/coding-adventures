"""Circuit element data classes."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Resistor:
    """R<name> n+ n- value"""

    name: str
    n_plus: str  # node identifier
    n_minus: str
    resistance: float  # ohms


@dataclass(frozen=True, slots=True)
class Capacitor:
    """C<name> n+ n- value"""

    name: str
    n_plus: str
    n_minus: str
    capacitance: float  # farads
    initial_voltage: float = 0.0


@dataclass(frozen=True, slots=True)
class Inductor:
    """L<name> n+ n- value"""

    name: str
    n_plus: str
    n_minus: str
    inductance: float  # henries


@dataclass(frozen=True, slots=True)
class VoltageSource:
    """V<name> n+ n- value"""

    name: str
    n_plus: str
    n_minus: str
    voltage: float  # volts


@dataclass(frozen=True, slots=True)
class CurrentSource:
    """I<name> n+ n- value (current flows from n+ to n-)"""

    name: str
    n_plus: str
    n_minus: str
    current: float  # amperes


@dataclass(frozen=True, slots=True)
class Diode:
    """Simple diode using Shockley equation."""

    name: str
    anode: str
    cathode: str
    Is: float = 1e-15  # saturation current
    Vt: float = 0.02585  # thermal voltage


@dataclass(frozen=True, slots=True)
class Mosfet:
    """A MOSFET instance backed by a mosfet_models.MOSFET model."""

    name: str
    drain: str
    gate: str
    source: str
    body: str
    model: object  # mosfet_models.MOSFET; using `object` to avoid Protocol overhead


Element = Resistor | Capacitor | Inductor | VoltageSource | CurrentSource | Diode | Mosfet
