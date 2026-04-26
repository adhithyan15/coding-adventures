"""device-physics: semiconductor device physics from first principles.

Implements PN junctions and MOSFET threshold voltage from drift-diffusion +
depletion approximation. Foundation for the analog stack
(``mosfet-models``, ``spice-engine``, ``standard-cell-library``).
"""

from device_physics.constants import (
    EG_SI_300K,
    EPS0,
    EPS_OX,
    EPS_SI,
    K_BOLTZMANN,
    MU_N_300K,
    MU_P_300K,
    N_C,
    N_I_300K,
    N_V,
    Q_ELECTRON,
    thermal_voltage,
)
from device_physics.semiconductor import (
    MOSFETParams,
    PNJunction,
    fermi_potential,
    intrinsic_concentration,
)

__version__ = "0.1.0"

__all__ = [
    "EG_SI_300K",
    "EPS0",
    "EPS_OX",
    "EPS_SI",
    "K_BOLTZMANN",
    "MOSFETParams",
    "MU_N_300K",
    "MU_P_300K",
    "N_C",
    "N_I_300K",
    "N_V",
    "PNJunction",
    "Q_ELECTRON",
    "__version__",
    "fermi_potential",
    "intrinsic_concentration",
    "thermal_voltage",
]
