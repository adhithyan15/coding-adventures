"""Intel 8080 state snapshot — a frozen, immutable view of the CPU.

The Intel 8080 has considerably more state than the 8008: a 16-bit address
space means the memory array is 65,536 bytes (4× the 8008's 16 KiB), the
stack pointer is a real 16-bit register in the CPU (not an internal push-down
stack), and there are 256 input and 256 output I/O ports.

All fields are plain Python scalars or tuples, so snapshots are hashable and
can be stored in sets or used as dict keys when needed.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Intel8080State:
    """Immutable snapshot of the Intel 8080 CPU + memory + I/O ports.

    Register layout
    ---------------
    The 8080 has seven 8-bit working registers (A, B, C, D, E, H, L) plus
    a 16-bit stack pointer (SP) and a 16-bit program counter (PC).  The
    "M" pseudo-register is not stored here — it is an indirect reference to
    memory[H:L] that is resolved at execution time.

    Flag register
    -------------
    Flags are stored as individual booleans rather than a packed byte.  The
    packed form can be reconstructed with ``flags_byte`` when needed (e.g.
    for PUSH PSW).

    Memory
    ------
    65,536-byte tuple.  Immutable by Python's tuple semantics, so callers
    cannot accidentally mutate a snapshot.

    I/O ports
    ---------
    The 8080 has 256 IN ports and 256 OUT ports.  Both are stored as tuples
    of ints (0–255).  Input ports can be pre-loaded before execution; output
    ports are written by OUT instructions.
    """

    # ── Registers ────────────────────────────────────────────────────────────
    a: int  # Accumulator (0–255)
    b: int  # Register B (0–255)
    c: int  # Register C (0–255)
    d: int  # Register D (0–255)
    e: int  # Register E (0–255)
    h: int  # Register H (0–255)
    l: int  # Register L (0–255)  # noqa: E741
    sp: int  # Stack pointer (0–65535)
    pc: int  # Program counter (0–65535)

    # ── Flags ────────────────────────────────────────────────────────────────
    flag_s: bool  # Sign: result bit 7 was 1
    flag_z: bool  # Zero: result was 0x00
    flag_ac: bool  # Auxiliary carry: carry out of bit 3
    flag_p: bool  # Parity: result had even number of 1-bits
    flag_cy: bool  # Carry: carry/borrow out of bit 7

    # ── Control ──────────────────────────────────────────────────────────────
    interrupts_enabled: bool  # INTE flip-flop (EI/DI instructions)
    halted: bool  # True after HLT executes

    # ── Memory & I/O ─────────────────────────────────────────────────────────
    memory: tuple[int, ...]  # 65536 bytes, immutable
    input_ports: tuple[int, ...]  # 256 input port values
    output_ports: tuple[int, ...]  # 256 output port values

    # ─────────────────────────────────────────────────────────────────────────
    # Derived / computed properties
    # ─────────────────────────────────────────────────────────────────────────

    @property
    def hl(self) -> int:
        """16-bit HL pair: (H << 8) | L."""
        return (self.h << 8) | self.l

    @property
    def bc(self) -> int:
        """16-bit BC pair: (B << 8) | C."""
        return (self.b << 8) | self.c

    @property
    def de(self) -> int:
        """16-bit DE pair: (D << 8) | E."""
        return (self.d << 8) | self.e

    @property
    def flags_byte(self) -> int:
        """Reconstruct the 8-bit flags register byte.

        The 8080 flags byte layout (per Intel 8080 System Reference Manual):

            bit 7 = S   (sign)
            bit 6 = Z   (zero)
            bit 5 = 0   (always zero)
            bit 4 = AC  (auxiliary carry)
            bit 3 = 0   (always zero)
            bit 2 = P   (parity)
            bit 1 = 1   (always one — this is the documented fixed bit)
            bit 0 = CY  (carry)
        """
        return (
            (int(self.flag_s) << 7)
            | (int(self.flag_z) << 6)
            | (0 << 5)
            | (int(self.flag_ac) << 4)
            | (0 << 3)
            | (int(self.flag_p) << 2)
            | (1 << 1)  # always 1
            | int(self.flag_cy)
        )
