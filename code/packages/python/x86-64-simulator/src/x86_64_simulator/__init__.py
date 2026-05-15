"""x86-64 (AMD64) behavioral simulator — Layer 07w.

Implements the SIM00 Simulator[X86_64State] protocol for the x86-64 ISA in
64-bit long mode (integer instructions only; no SSE/AVX/FPU).
"""

from x86_64_simulator.simulator import StepTrace, X86_64Simulator
from x86_64_simulator.state import MEM_SIZE, X86_64State

__version__ = "0.1.0"

__all__ = [
    "MEM_SIZE",
    "StepTrace",
    "X86_64Simulator",
    "X86_64State",
    "__version__",
]
