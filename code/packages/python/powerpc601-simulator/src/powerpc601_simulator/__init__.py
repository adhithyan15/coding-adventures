"""PowerPC 601 (1992) behavioral simulator — Layer 07u."""

from .simulator import (
    BI_EQ,
    BI_GT,
    BI_LT,
    BI_SO,
    BO_ALWAYS,
    BO_BDNZ,
    BO_BDZ,
    BO_FALSE,
    BO_TRUE,
    HALT,
    SPR_CTR,
    SPR_LR,
    SPR_XER,
    PowerPC601Simulator,
    b_form,
    d_form,
    i_form,
    x_form,
    xfx_form,
    xl_form,
    xo_form,
)
from .state import MASK32, MEM_SIZE, XER_CA, XER_OV, XER_SO, PowerPC601State, make_initial_state

__all__ = [
    # Simulator and state
    "PowerPC601Simulator",
    "PowerPC601State",
    "make_initial_state",
    # Constants
    "HALT",
    "MASK32",
    "MEM_SIZE",
    "XER_CA",
    "XER_OV",
    "XER_SO",
    # SPR numbers
    "SPR_XER",
    "SPR_LR",
    "SPR_CTR",
    # BO constants
    "BO_ALWAYS",
    "BO_TRUE",
    "BO_FALSE",
    "BO_BDNZ",
    "BO_BDZ",
    # BI constants
    "BI_LT",
    "BI_GT",
    "BI_EQ",
    "BI_SO",
    # Encoding helpers
    "i_form",
    "b_form",
    "d_form",
    "x_form",
    "xo_form",
    "xfx_form",
    "xl_form",
]
