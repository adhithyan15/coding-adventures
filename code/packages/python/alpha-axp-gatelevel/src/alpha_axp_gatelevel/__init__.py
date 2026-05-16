"""alpha_axp_gatelevel — DEC Alpha AXP 21064 (1992) gate-level simulator.

Layer 07s2 in the historical CPU simulator series.

Every arithmetic and logic operation in the data path routes through logic
gate primitives (AND, OR, XOR, NOT from logic_gates) and a ripple-carry
adder (from arithmetic), exactly like the real silicon.

Public API
──────────
  AlphaAXPGateLevelSimulator — implements Simulator[AlphaState]
  RegisterFile64              — 32 × 64-bit gate-level register file
  ALUResult64                 — dataclass returned by ALU operations
  decode_instruction          — combinational 32-bit instruction decoder
  add_64bit, add_128bit       — 64/128-bit adders via ripple_carry_adder
  int_to_bits, bits_to_int    — integer ↔ LSB-first bit-list bridge
"""

from __future__ import annotations

from .alu import (
    ALUResult64,
    addl,
    addq,
    andq,
    bicq,
    cmpeq,
    cmple,
    cmplt,
    cmpule,
    cmpult,
    eqvq,
    mull,
    mulq,
    ornot,
    orq,
    s4addl,
    s4addq,
    s4subl,
    s4subq,
    s8addl,
    s8addq,
    s8subl,
    s8subq,
    sll64,
    sra64,
    srl64,
    subl,
    subq,
    umulh,
    xorq,
)
from .bits import (
    add_32bit,
    add_64bit,
    add_128bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_32bit,
    invert_64bit,
    sext32_to_64,
    shl_64,
    shr_64_arith,
    shr_64_logical,
)
from .decoder import decode_instruction
from .register_file import RegisterFile64
from .simulator import AlphaAXPGateLevelSimulator

__all__ = [
    # Simulator
    "AlphaAXPGateLevelSimulator",
    # Register file
    "RegisterFile64",
    # ALU
    "ALUResult64",
    "addq", "subq", "addl", "subl",
    "andq", "orq", "xorq", "bicq", "ornot", "eqvq",
    "sll64", "srl64", "sra64",
    "cmpeq", "cmplt", "cmple", "cmpult", "cmpule",
    "s4addq", "s8addq", "s4addl", "s8addl",
    "s4subq", "s8subq", "s4subl", "s8subl",
    "mulq", "umulh", "mull",
    # Bits
    "int_to_bits", "bits_to_int",
    "add_64bit", "add_128bit", "add_32bit",
    "invert_64bit", "invert_32bit",
    "compute_zero",
    "shl_64", "shr_64_logical", "shr_64_arith",
    "sext32_to_64",
    # Decoder
    "decode_instruction",
]
