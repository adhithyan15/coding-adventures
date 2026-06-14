"""Intel 8080 gate-level simulator.

Routes every arithmetic/logic operation through real gate functions.
Implements Simulator[Intel8080State] from the SIM00 protocol.
"""

from __future__ import annotations

from intel8080_gatelevel.alu import ALU8080, ALUResult8080
from intel8080_gatelevel.bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
)
from intel8080_gatelevel.control import ControlUnit, FlagRegister
from intel8080_gatelevel.decoder import DecodedInstruction, Decoder8080
from intel8080_gatelevel.register_file import (
    PAIR_BC,
    PAIR_DE,
    PAIR_HL,
    PAIR_SP,
    REG_A,
    REG_B,
    REG_C,
    REG_D,
    REG_E,
    REG_H,
    REG_L,
    REG_M,
    Register8,
    Register16,
    RegisterFile,
)
from intel8080_gatelevel.simulator import Intel8080GateLevelSimulator

__all__ = [
    # Main simulator
    "Intel8080GateLevelSimulator",
    # ALU
    "ALU8080",
    "ALUResult8080",
    # Decoder
    "Decoder8080",
    "DecodedInstruction",
    # Register file
    "Register8",
    "Register16",
    "RegisterFile",
    "REG_A",
    "REG_B",
    "REG_C",
    "REG_D",
    "REG_E",
    "REG_H",
    "REG_L",
    "REG_M",
    "PAIR_BC",
    "PAIR_DE",
    "PAIR_HL",
    "PAIR_SP",
    # Control unit
    "ControlUnit",
    "FlagRegister",
    # Bit helpers
    "int_to_bits",
    "bits_to_int",
    "compute_parity",
    "compute_zero",
    "add_8bit",
    "add_16bit",
    "invert_8bit",
]
