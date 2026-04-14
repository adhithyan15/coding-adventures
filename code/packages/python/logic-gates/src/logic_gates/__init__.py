"""Logic Gates — Layer 1 of the computing stack.

Fundamental logic gate implementations: AND, OR, NOT, XOR, NAND, NOR, XNOR.
Also includes NAND-derived gates (all gates built from NAND only),
multi-input variants, sequential logic (latches, flip-flops, registers),
and combinational circuits (MUX, DEMUX, decoder, encoder, tri-state buffer).
"""

from logic_gates.combinational import (
    decoder,
    demux,
    encoder,
    mux2,
    mux4,
    mux8,
    mux_n,
    priority_encoder,
    tri_state,
)
from logic_gates.gates import (
    AND,
    NAND,
    NOR,
    NOT,
    OR,
    XNOR,
    XOR,
    AND_N,
    OR_N,
    XOR_N,
    nand_and,
    nand_not,
    nand_or,
    nand_xor,
)
from logic_gates.sequential import (
    counter,
    d_flip_flop,
    d_latch,
    register,
    shift_register,
    sr_latch,
)

__all__ = [
    # Fundamental gates
    "NOT",
    "AND",
    "OR",
    "XOR",
    # Composite gates
    "NAND",
    "NOR",
    "XNOR",
    # NAND-derived gates
    "nand_not",
    "nand_and",
    "nand_or",
    "nand_xor",
    # Multi-input gates
    "AND_N",
    "OR_N",
    "XOR_N",
    # Sequential logic
    "sr_latch",
    "d_latch",
    "d_flip_flop",
    "register",
    "shift_register",
    "counter",
    # Combinational circuits
    "mux2",
    "mux4",
    "mux8",
    "mux_n",
    "demux",
    "decoder",
    "encoder",
    "priority_encoder",
    "tri_state",
]
