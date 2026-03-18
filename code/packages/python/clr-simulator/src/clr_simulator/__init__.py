"""CLR IL Simulator — Layer 4f of the computing stack.

A simulator for Microsoft's Common Language Runtime Intermediate Language,
the bytecode that powers .NET (C#, F#, VB.NET).
"""

from clr_simulator.simulator import (
    CEQ_BYTE,
    CGT_BYTE,
    CLROpcode,
    CLRSimulator,
    CLRTrace,
    CLT_BYTE,
    assemble_clr,
    encode_ldc_i4,
    encode_ldloc,
    encode_stloc,
)

__all__ = [
    "CEQ_BYTE",
    "CGT_BYTE",
    "CLROpcode",
    "CLRSimulator",
    "CLRTrace",
    "CLT_BYTE",
    "assemble_clr",
    "encode_ldc_i4",
    "encode_ldloc",
    "encode_stloc",
]
