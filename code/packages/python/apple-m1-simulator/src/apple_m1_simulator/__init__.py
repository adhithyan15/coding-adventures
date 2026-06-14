"""
coding-adventures-apple-m1-simulator
======================================

Behavioral simulator for the Apple M1 (AArch64 + NEON/AdvSIMD, 2020).

Layer 07z in the coding-adventures simulator series.  Extends the AArch64
integer base (07v) with 32 × 128-bit NEON registers, scalar FP arithmetic,
FP load/store, and vector integer/FP operations.

Exports
-------
AppleM1Simulator  -- SIM00-compliant simulator; call execute(program) to run
AppleM1State      -- Immutable state snapshot (frozen dataclass)
"""

from .simulator import AppleM1Simulator
from .state import AppleM1State

__all__ = ["AppleM1Simulator", "AppleM1State"]
