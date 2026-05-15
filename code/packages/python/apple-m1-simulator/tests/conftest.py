"""
Test helpers and fixtures for apple-m1-simulator tests.

This module re-exports all instruction-encoding helper functions from the
simulator (for convenience in tests) and provides a `run()` helper that
assembles a program, executes it, and returns the final state.
"""

from __future__ import annotations

import pytest

from apple_m1_simulator import AppleM1Simulator, AppleM1State
from apple_m1_simulator.simulator import (
    COND_AL,
    COND_CC,
    COND_CS,
    COND_EQ,
    COND_GE,
    COND_GT,
    COND_HI,
    COND_LE,
    COND_LS,
    COND_LT,
    COND_MI,
    COND_NE,
    COND_PL,
    COND_VC,
    COND_VS,
    HALT,
    branch_cond,
    branch_imm,
    branch_reg,
    cbz_cbnz,
    csel_enc,
    dp_imm,
    dp_reg,
    fcvtzs,
    fmov_fp_to_gpr_d,
    fmov_fp_to_gpr_s,
    fmov_gpr_to_fp_d,
    fmov_gpr_to_fp_s,
    fp_cmp,
    fp_dp1src,
    fp_dp2src,
    fp_ldst_uoff,
    ldst_uoff,
    logic_imm,
    logic_reg,
    madd_msub,
    movwide,
    neon_3reg_same,
    neon_dup_gpr,
    scvtf,
    tbz_tbnz,
    ucvtf,
)

# Re-export HALT so tests can just do `from conftest import HALT` (via fixture scope)
__all__ = [
    "run",
    "HALT",
    "dp_imm", "dp_reg", "logic_imm", "logic_reg", "movwide",
    "ldst_uoff", "branch_imm", "branch_cond", "cbz_cbnz",
    "branch_reg", "madd_msub", "csel_enc", "tbz_tbnz",
    "fp_dp1src", "fp_dp2src", "fp_cmp",
    "fmov_gpr_to_fp_d", "fmov_fp_to_gpr_d",
    "fmov_gpr_to_fp_s", "fmov_fp_to_gpr_s",
    "fcvtzs", "scvtf", "ucvtf",
    "fp_ldst_uoff", "neon_3reg_same", "neon_dup_gpr",
    "COND_EQ", "COND_NE", "COND_CS", "COND_CC", "COND_MI", "COND_PL",
    "COND_VS", "COND_VC", "COND_HI", "COND_LS", "COND_GE", "COND_LT",
    "COND_GT", "COND_LE", "COND_AL",
]


def run(instructions: list[bytes], max_steps: int = 1000) -> AppleM1State:
    """
    Assemble a list of instruction byte sequences, execute them, return state.

    HALT is automatically appended if not already present.

    Example::
        state = run([
            movwide(1, 0b10, 0, 42, 0),   # MOVZ X0, #42
            HALT,
        ])
        assert state.x0 == 42
    """
    prog = b"".join(instructions)
    if not prog.endswith(HALT):
        prog += HALT
    sim = AppleM1Simulator()
    result = sim.execute(prog, max_steps=max_steps)
    return result.final_state


@pytest.fixture
def sim() -> AppleM1Simulator:
    """Return a freshly reset AppleM1Simulator."""
    return AppleM1Simulator()
