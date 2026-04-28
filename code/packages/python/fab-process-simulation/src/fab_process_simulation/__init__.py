"""fab-process-simulation: 1-D analytical CMOS process flow."""

from fab_process_simulation.process import (
    DEAL_GROVE_DRY_1000C_A,
    DEAL_GROVE_DRY_1000C_B,
    DIFFUSIVITY_1000C,
    IMPLANT_RANGES,
    CrossSection,
    Layer,
    deal_grove_oxidation,
    deposit,
    diffuse,
    etch,
    implant,
)

__version__ = "0.1.0"

__all__ = [
    "CrossSection",
    "DEAL_GROVE_DRY_1000C_A",
    "DEAL_GROVE_DRY_1000C_B",
    "DIFFUSIVITY_1000C",
    "IMPLANT_RANGES",
    "Layer",
    "__version__",
    "deal_grove_oxidation",
    "deposit",
    "diffuse",
    "etch",
    "implant",
]
