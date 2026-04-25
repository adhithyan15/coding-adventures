"""Semiconductor physics primitives.

Implements:
- Intrinsic carrier concentration vs temperature.
- Fermi potential as a function of doping.
- PN junction: built-in voltage, depletion width, Shockley diode equation.
- MOSFET threshold voltage with body effect.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import exp, log, sqrt

from device_physics.constants import (
    EG_SI_300K,
    EPS_OX,
    EPS_SI,
    MU_N_300K,
    MU_P_300K,
    N_I_300K,
    Q_ELECTRON,
    thermal_voltage,
)


def intrinsic_concentration(T: float = 300.0) -> float:
    """Intrinsic carrier concentration n_i(T) [/m^3].

    Uses the standard temperature scaling n_i ∝ T^(3/2) * exp(-Eg/(2kT)).
    """
    if T == 300.0:
        return N_I_300K
    if T < 100.0:
        # Boltzmann statistics break down at low T; reject as an out-of-spec
        # request rather than return a nonsensical 0.
        raise ValueError(f"T={T} K below model validity (>= 100 K)")
    factor = (T / 300.0) ** 1.5
    bandgap_term = exp(
        -(EG_SI_300K / (2.0 * thermal_voltage(T))) * (1.0 - T / 300.0)
    )
    return N_I_300K * factor * bandgap_term


def fermi_potential(N: float, *, kind: str, T: float = 300.0) -> float:
    """Fermi potential phi_F.

    For p-type silicon (kind='p'): returns positive.
    For n-type silicon (kind='n'): returns negative.

    `N` is the dopant concentration in /m^3.
    """
    if N <= 0:
        raise ValueError(f"doping N must be > 0, got {N}")
    n_i = intrinsic_concentration(T)
    magnitude = thermal_voltage(T) * log(N / n_i)
    if kind == "p":
        return +magnitude
    if kind == "n":
        return -magnitude
    raise ValueError(f"kind must be 'p' or 'n', got {kind!r}")


# ---------------------------------------------------------------------------
# PN junction
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class PNJunction:
    """A PN junction with given doping levels and area.

    Default carrier lifetimes are typical for moderately-doped silicon.
    """

    N_A: float  # acceptor doping in p-side (/m^3)
    N_D: float  # donor doping in n-side (/m^3)
    A: float  # area (m^2)
    T: float = 300.0  # temperature (K)
    tau_n: float = 1e-6  # electron lifetime (s)
    tau_p: float = 1e-6  # hole lifetime (s)

    def __post_init__(self) -> None:
        if self.N_A <= 0 or self.N_D <= 0:
            raise ValueError(f"doping must be > 0, got N_A={self.N_A}, N_D={self.N_D}")
        if self.A <= 0:
            raise ValueError(f"area A must be > 0, got {self.A}")

    def built_in_voltage(self) -> float:
        n_i = intrinsic_concentration(self.T)
        return thermal_voltage(self.T) * log((self.N_A * self.N_D) / (n_i**2))

    def depletion_width(self, V_applied: float = 0.0) -> float:
        """Depletion-region width [m].

        Forward bias narrows it; reverse bias widens it. ``V_applied`` is
        positive for forward bias.
        """
        phi_bi = self.built_in_voltage()
        if V_applied >= phi_bi:
            # Beyond built-in voltage; approximate to zero. Real device would
            # be in heavy injection.
            return 0.0
        return sqrt(
            (2.0 * EPS_SI / Q_ELECTRON)
            * ((self.N_A + self.N_D) / (self.N_A * self.N_D))
            * (phi_bi - V_applied)
        )

    def saturation_current(self) -> float:
        """Saturation current I_S [A] from minority-carrier diffusion."""
        n_i = intrinsic_concentration(self.T)
        V_T = thermal_voltage(self.T)
        D_n = MU_N_300K * V_T  # Einstein relation
        D_p = MU_P_300K * V_T
        L_n = sqrt(D_n * self.tau_n)
        L_p = sqrt(D_p * self.tau_p)
        return (
            Q_ELECTRON
            * self.A
            * n_i**2
            * (D_n / (L_n * self.N_A) + D_p / (L_p * self.N_D))
        )

    def current(self, V: float) -> float:
        """Shockley diode equation: I = I_S × (exp(V/V_T) - 1)."""
        V_T = thermal_voltage(self.T)
        return self.saturation_current() * (exp(V / V_T) - 1.0)


# ---------------------------------------------------------------------------
# MOSFET threshold
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class MOSFETParams:
    """Physical parameters of a MOSFET, sufficient to compute V_t.

    'NMOS' has p-type body; 'PMOS' has n-type body.
    """

    type: str  # 'NMOS' or 'PMOS'
    L: float  # channel length (m)
    W: float  # channel width (m)
    T_ox: float  # gate-oxide thickness (m)
    N_body: float  # body doping (/m^3)
    phi_MS: float  # gate-body work-function difference (V)
    Q_ox: float = 0.0  # oxide trapped charge per area (C/m^2)
    T: float = 300.0

    def __post_init__(self) -> None:
        if self.type not in ("NMOS", "PMOS"):
            raise ValueError(f"type must be NMOS or PMOS, got {self.type!r}")
        if self.L <= 0 or self.W <= 0:
            raise ValueError(f"L and W must be > 0, got L={self.L}, W={self.W}")
        if self.T_ox <= 0:
            raise ValueError(f"T_ox must be > 0, got {self.T_ox}")
        if self.N_body <= 0:
            raise ValueError(f"N_body must be > 0, got {self.N_body}")

    @property
    def C_ox(self) -> float:
        """Oxide capacitance per unit area [F/m^2]."""
        return EPS_OX / self.T_ox

    @property
    def V_FB(self) -> float:
        """Flat-band voltage."""
        return self.phi_MS - self.Q_ox / self.C_ox

    @property
    def phi_F(self) -> float:
        """Magnitude of Fermi potential of the body."""
        body_kind = "p" if self.type == "NMOS" else "n"
        return abs(fermi_potential(self.N_body, kind=body_kind, T=self.T))

    @property
    def gamma(self) -> float:
        """Body-effect coefficient [V^(1/2)]."""
        return sqrt(2.0 * EPS_SI * Q_ELECTRON * self.N_body) / self.C_ox

    def threshold_voltage(self, V_SB: float = 0.0) -> float:
        """Threshold voltage with optional source-body bias.

        For NMOS: returns the V_GS needed to invert the channel.
        Body-effect raises V_t when V_SB > 0 (source above body).
        """
        if -2.0 * self.phi_F > V_SB:
            raise ValueError(
                f"V_SB={V_SB} below 2*phi_F={2 * self.phi_F}; body-source forward biased"
            )
        V_t0 = self.V_FB + 2.0 * self.phi_F + self.gamma * sqrt(2.0 * self.phi_F)
        return V_t0 + self.gamma * (
            sqrt(2.0 * self.phi_F + V_SB) - sqrt(2.0 * self.phi_F)
        )
