"""Intel 8086 (1978) behavioral simulator — Layer 07m.

Public API
----------
``X86Simulator``
    The main simulator class.  Implements ``Simulator[X86State]``
    from ``simulator_protocol`` (SIM00).

``X86State``
    Frozen dataclass snapshot of the 8086 CPU state at any instant.
    Fields: 16-bit registers (AX/BX/CX/DX/SI/DI/SP/BP), segment registers
    (CS/DS/SS/ES), IP, individual flag booleans (cf/pf/af/zf/sf/tf/if_/df/of),
    halted, input_ports, output_ports, memory (1 MB).
    Properties: al/ah/bl/bh/cl/ch/dl/dh, ax_signed, al_signed, flags.

Quick start
-----------
>>> from intel_8086_simulator import X86Simulator
>>> sim = X86Simulator()
>>> prog = bytes([
...     0xB8, 0x0A, 0x00,   # MOV AX, 10
...     0xBB, 0x14, 0x00,   # MOV BX, 20
...     0x01, 0xD8,          # ADD AX, BX
...     0xF4,                # HLT
... ])
>>> result = sim.execute(prog)
>>> result.ok
True
>>> result.final_state.ax
30
"""

from intel_8086_simulator.simulator import X86Simulator
from intel_8086_simulator.state import X86State

__all__ = ["X86Simulator", "X86State"]
