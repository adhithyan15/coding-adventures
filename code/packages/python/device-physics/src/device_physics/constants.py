"""Physical constants used throughout the device-physics package.

All values in SI units unless otherwise noted. Sources are standard
semiconductor physics references (Sedra/Smith, Pierret, Streetman/Banerjee).
"""

from __future__ import annotations

K_BOLTZMANN = 1.380649e-23  # J/K
Q_ELECTRON = 1.602176634e-19  # C
EPS0 = 8.8541878128e-12  # F/m, vacuum permittivity
EPS_SI = 11.7 * EPS0  # silicon permittivity
EPS_OX = 3.9 * EPS0  # SiO2 permittivity

# Silicon at 300 K
N_I_300K = 1.0e16  # /m^3 (≈ 1e10 /cm^3)
N_C = 2.8e25  # /m^3, effective DOS in conduction band
N_V = 1.04e25  # /m^3, effective DOS in valence band
EG_SI_300K = 1.12  # eV, silicon bandgap at 300 K

# Mobility (lightly doped Si, low field, 300 K)
MU_N_300K = 1350e-4  # m^2/V·s, electron mobility
MU_P_300K = 480e-4  # m^2/V·s, hole mobility


def thermal_voltage(T: float = 300.0) -> float:
    """V_T = kT/q. Returns volts."""
    return K_BOLTZMANN * T / Q_ELECTRON
