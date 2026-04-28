"""mosfet-models: MOSFET I-V models with a uniform interface for SPICE.

v0.1.0 ships the Level-1 (Shockley square-law) model. EKV and BSIM3v3
subsets land in v0.2.0 alongside the SPICE engine integration.
"""

from mosfet_models.level1 import (
    Level1Params,
    MosResult,
    evaluate_level1,
)
from mosfet_models.mosfet import MOSFET, Level1Model, MosfetModel, MosfetType

__version__ = "0.1.0"

__all__ = [
    "MOSFET",
    "Level1Model",
    "Level1Params",
    "MosResult",
    "MosfetModel",
    "MosfetType",
    "__version__",
    "evaluate_level1",
]
