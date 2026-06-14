"""Motorola 68000 gate-level simulator.

=== Overview ===

This module provides a complete Motorola 68000 simulator where every ALU
data-path operation routes through logic gate primitives.  The simulator
implements the ``Simulator[M68KState]`` protocol from ``simulator_protocol``
and is cross-validated against the behavioral simulator in
``motorola_68000_simulator``.

=== Design Philosophy ===

Real hardware:  every bit passes through silicon transistors forming AND, OR,
XOR, NOT, and full-adder gates.  This simulator represents that faithfully
at the ALU level while using Python for control flow (decoding, branching,
memory addressing).

The critical gate-level path:
  - Additions/subtractions → ripple_carry_adder chains (from ``arithmetic``)
  - Logical ops → parallel AND/OR/XOR arrays (from ``logic_gates``)
  - NOT gates → one per bit (from ``logic_gates``)
  - Shifts/rotates → bit-array rewiring
  - Zero detection → NOR tree
  - Overflow detection → XOR of carry-into-MSB and carry-out-of-MSB

=== Memory Model ===

  0x000000–0xFFFFFF  16 MB flat byte array
  0x001000           Program load address
  0x00F000           Initial supervisor stack pointer

=== Effective Addresses ===

The 68000's 12 addressing modes are fully implemented:
  mode 0  Dn — data register direct
  mode 1  An — address register direct
  mode 2  (An) — indirect
  mode 3  (An)+ — postincrement indirect
  mode 4  -(An) — predecrement indirect
  mode 5  d16(An) — displacement indirect
  mode 6  d8(An,Xn) — indexed indirect
  mode 7, reg 0  (xxx).W — absolute short
  mode 7, reg 1  (xxx).L — absolute long
  mode 7, reg 2  d16(PC) — PC-relative
  mode 7, reg 3  d8(PC,Xn) — PC-relative indexed
  mode 7, reg 4  #imm — immediate
"""

from __future__ import annotations

from motorola_68000_simulator.state import M68KState
from simulator_protocol import ExecutionResult, StepTrace

from motorola68k_gatelevel.alu import (
    ALUResult68k,
    add8,
    add16,
    add32,
    and8,
    and16,
    and32,
    asl,
    asr,
    cmp8,
    cmp16,
    cmp32,
    divs,
    divu,
    lsl,
    lsr,
    muls,
    mulu,
    neg8,
    neg16,
    neg32,
    not8,
    not16,
    not32,
    or8,
    or16,
    or32,
    rol,
    ror,
    roxl,
    roxr,
    sub8,
    sub16,
    sub32,
    xor8,
    xor16,
    xor32,
)
from motorola68k_gatelevel.register_file import RegisterFile68k

# ── Constants ──────────────────────────────────────────────────────────────────

_LOAD_ADDR = 0x001000   # program load address
_INIT_SP   = 0x00F000   # initial supervisor stack pointer
_ADDR_MASK = 0x00FF_FFFF
_LONG_MASK = 0xFFFF_FFFF
_WORD_MASK = 0x0000_FFFF
_BYTE_MASK = 0x0000_00FF
_LONG_MSB  = 0x8000_0000
_WORD_MSB  = 0x0000_8000
_BYTE_MSB  = 0x0000_0080
_MEM_SIZE  = 0x100_0000  # 16 MB

# Standard size code table (bits 7–6 of opword)
_SZ_ARITH = {0: 1, 1: 2, 2: 4}
_SZ_MASK  = {1: _BYTE_MASK, 2: _WORD_MASK, 4: _LONG_MASK}
_SZ_MSB   = {1: _BYTE_MSB,  2: _WORD_MSB,  4: _LONG_MSB}

# MOVE instruction uses a different size encoding (bits 13–12)
_SZ_MOVE = {1: 1, 3: 2, 2: 4}

# Condition code evaluators.
# Each takes (N, Z, V, C) bit-integers and returns True if condition holds.
_CC_FUNCS = [
    lambda n, z, v, c: True,                    # T  — always true
    lambda n, z, v, c: False,                   # F  — always false
    lambda n, z, v, c: (not c) and (not z),     # HI
    lambda n, z, v, c: c or z,                  # LS
    lambda n, z, v, c: not c,                   # CC (HS)
    lambda n, z, v, c: bool(c),                 # CS (LO)
    lambda n, z, v, c: not z,                   # NE
    lambda n, z, v, c: bool(z),                 # EQ
    lambda n, z, v, c: not v,                   # VC
    lambda n, z, v, c: bool(v),                 # VS
    lambda n, z, v, c: not n,                   # PL
    lambda n, z, v, c: bool(n),                 # MI
    lambda n, z, v, c: n == v,                  # GE
    lambda n, z, v, c: n != v,                  # LT
    lambda n, z, v, c: (not z) and (n == v),    # GT
    lambda n, z, v, c: z or (n != v),           # LE
]

_CC_NAMES = [
    "T", "F", "HI", "LS",
    "CC", "CS", "NE", "EQ",
    "VC", "VS", "PL", "MI",
    "GE", "LT", "GT", "LE",
]


def _sign_extend(value: int, bits: int) -> int:
    """Sign-extend ``value`` from ``bits``-bit two's-complement to Python int.

    >>> _sign_extend(0xFF, 8)
    -1
    >>> _sign_extend(0x7F, 8)
    127
    >>> _sign_extend(0x8000, 16)
    -32768
    """
    sign = 1 << (bits - 1)
    return (value & (sign - 1)) - (value & sign)


# ── ALU helpers that select the right width ────────────────────────────────────

_ADD_FNS = {1: add8,  2: add16,  4: add32}
_SUB_FNS = {1: sub8,  2: sub16,  4: sub32}
_AND_FNS = {1: and8,  2: and16,  4: and32}
_OR_FNS  = {1: or8,   2: or16,   4: or32}
_XOR_FNS = {1: xor8,  2: xor16,  4: xor32}
_NEG_FNS = {1: neg8,  2: neg16,  4: neg32}
_NOT_FNS = {1: not8,  2: not16,  4: not32}
_CMP_FNS = {1: cmp8,  2: cmp16,  4: cmp32}


class Motorola68kGateLevelSimulator:
    """Motorola 68000 gate-level simulator.

    Every ALU operation (ADD, SUB, AND, OR, XOR, NOT, shifts, rotates) routes
    through logic gate primitives from the ``logic_gates`` and ``arithmetic``
    packages.

    Implements the ``Simulator[M68KState]`` protocol.

    Examples:
        >>> sim = Motorola68kGateLevelSimulator()
        >>> prog = bytes([
        ...     0x70, 0x05,              # MOVEQ #5, D0
        ...     0x72, 0x03,              # MOVEQ #3, D1
        ...     0xD0, 0x81,              # ADD.L D1, D0
        ...     0x4E, 0x72, 0x27, 0x00, # STOP #0x2700
        ... ])
        >>> result = sim.execute(prog)
        >>> result.final_state.d0
        8
    """

    def __init__(self) -> None:
        self._rf   = RegisterFile68k()
        self._mem  = bytearray(_MEM_SIZE)
        self._halted = False
        self._traces: list[StepTrace] = []
        self._pending_interrupt: int = -1   # -1 = none; 0-7 = level
        self._pending_nmi: bool = False

    # ──────────────────────────────────────────────────────────────────────────
    # Protocol methods
    # ──────────────────────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset to power-on state.

        All data registers → 0.  A0–A6 → 0.  A7 → 0x00F000.
        PC → 0x001000.  SR → 0x2700.  Memory zeroed.  Halt cleared.
        """
        self._rf.reset()
        self._mem[:] = b"\x00" * _MEM_SIZE
        self._halted = False
        self._traces = []
        self._pending_interrupt = -1
        self._pending_nmi = False

    def load(self, program: bytes) -> None:
        """Load binary program bytes starting at address 0x001000.

        Args:
            program: Machine code bytes.

        Raises:
            ValueError: If the program is too large to fit in memory.
        """
        end = _LOAD_ADDR + len(program)
        if end > _MEM_SIZE:
            raise ValueError(
                f"Program too large: {len(program)} bytes from 0x{_LOAD_ADDR:06X}"
            )
        self._mem[_LOAD_ADDR:end] = program

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        Raises:
            RuntimeError: If the CPU is halted.
        """
        if self._halted:
            raise RuntimeError("CPU is halted — call reset() before stepping")
        pc_before = self._rf.read_pc()
        mnemonic = self._decode_and_execute()
        pc_after = self._rf.read_pc()
        trace = StepTrace(
            pc_before=pc_before,
            pc_after=pc_after,
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:06X}",
        )
        self._traces.append(trace)
        return trace

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[M68KState]:
        """Load program, reset state, run to STOP/TRAP#15 or max_steps.

        Args:
            program:   Machine code bytes.
            max_steps: Maximum steps before giving up.

        Returns:
            ExecutionResult with final state, step count, and traces.
        """
        self.reset()
        self.load(program)
        steps = 0
        error: str | None = None
        while not self._halted and steps < max_steps:
            try:
                self.step()
            except RuntimeError as exc:
                error = str(exc)
                break
            steps += 1
        if not self._halted and error is None:
            error = f"max_steps ({max_steps}) exceeded"
        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=list(self._traces),
        )

    def get_state(self) -> M68KState:
        """Return a frozen snapshot of the current CPU state."""
        rf = self._rf
        return M68KState(
            d0=rf.read_dn(0, 4), d1=rf.read_dn(1, 4),
            d2=rf.read_dn(2, 4), d3=rf.read_dn(3, 4),
            d4=rf.read_dn(4, 4), d5=rf.read_dn(5, 4),
            d6=rf.read_dn(6, 4), d7=rf.read_dn(7, 4),
            a0=rf.read_an(0), a1=rf.read_an(1),
            a2=rf.read_an(2), a3=rf.read_an(3),
            a4=rf.read_an(4), a5=rf.read_an(5),
            a6=rf.read_an(6), a7=rf.read_an(7),
            pc=rf.read_pc(),
            sr=rf.pack_sr(),
            halted=self._halted,
            memory=tuple(self._mem),
        )

    def set_input_port(self, port: int, value: int) -> None:
        """Set I/O port value (no-op — 68000 has no I/O ports)."""

    def get_output_port(self, port: int) -> int:
        """Read I/O port value (no-op — 68000 has no I/O ports)."""
        return 0

    def interrupt(self, level: int) -> None:
        """Assert an interrupt request at the given priority level (1–7)."""
        self._pending_interrupt = level & 7

    def nmi(self) -> None:
        """Assert a non-maskable interrupt (level 7)."""
        self._pending_nmi = True

    # ──────────────────────────────────────────────────────────────────────────
    # Memory helpers (big-endian)
    # ──────────────────────────────────────────────────────────────────────────

    def _mem_read_byte(self, addr: int) -> int:
        return self._mem[addr & _ADDR_MASK]

    def _mem_read_word(self, addr: int) -> int:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned word read at 0x{a:06X}")
        return (self._mem[a] << 8) | self._mem[a + 1]

    def _mem_read_long(self, addr: int) -> int:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned long read at 0x{a:06X}")
        return (
            (self._mem[a    ] << 24)
            | (self._mem[a + 1] << 16)
            | (self._mem[a + 2] <<  8)
            |  self._mem[a + 3]
        )

    def _mem_read(self, addr: int, sz: int) -> int:
        if sz == 1:
            return self._mem_read_byte(addr)
        if sz == 2:
            return self._mem_read_word(addr)
        return self._mem_read_long(addr)

    def _mem_write_byte(self, addr: int, val: int) -> None:
        self._mem[addr & _ADDR_MASK] = val & _BYTE_MASK

    def _mem_write_word(self, addr: int, val: int) -> None:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned word write at 0x{a:06X}")
        self._mem[a    ] = (val >> 8) & _BYTE_MASK
        self._mem[a + 1] =  val       & _BYTE_MASK

    def _mem_write_long(self, addr: int, val: int) -> None:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned long write at 0x{a:06X}")
        self._mem[a    ] = (val >> 24) & _BYTE_MASK
        self._mem[a + 1] = (val >> 16) & _BYTE_MASK
        self._mem[a + 2] = (val >>  8) & _BYTE_MASK
        self._mem[a + 3] =  val        & _BYTE_MASK

    def _mem_write(self, addr: int, sz: int, val: int) -> None:
        if sz == 1:
            self._mem_write_byte(addr, val)
        elif sz == 2:
            self._mem_write_word(addr, val)
        else:
            self._mem_write_long(addr, val)

    # ──────────────────────────────────────────────────────────────────────────
    # PC-based fetch helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _fetch_word(self) -> int:
        pc = self._rf.read_pc()
        w = self._mem_read_word(pc)
        self._rf.write_pc((pc + 2) & _ADDR_MASK)
        return w

    def _fetch_long(self) -> int:
        pc = self._rf.read_pc()
        v = self._mem_read_long(pc)
        self._rf.write_pc((pc + 4) & _ADDR_MASK)
        return v

    def _fetch_word_signed(self) -> int:
        return _sign_extend(self._fetch_word(), 16)

    def _fetch_imm(self, sz: int) -> int:
        """Fetch immediate value; byte imm occupies a full 16-bit extension word."""
        if sz == 4:
            return self._fetch_long()
        val = self._fetch_word()
        return val & _SZ_MASK[sz]

    # ──────────────────────────────────────────────────────────────────────────
    # Register read/write helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _get_d(self, n: int, sz: int) -> int:
        return self._rf.read_dn(n, sz)

    def _set_d(self, n: int, val: int, sz: int) -> None:
        self._rf.write_dn(n, val, sz)

    def _get_a(self, n: int) -> int:
        return self._rf.read_an(n)

    def _set_a(self, n: int, val: int) -> None:
        self._rf.write_an(n, val & _LONG_MASK)

    # ──────────────────────────────────────────────────────────────────────────
    # CCR / SR helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _commit_flags_add(self, r: ALUResult68k) -> None:
        """Commit all flags for ADD-type operations (C=X=carry)."""
        rf = self._rf
        rf._flag_c = r.flag_c
        rf._flag_x = r.flag_c  # X tracks C for ADD
        rf._flag_n = r.flag_n
        rf._flag_z = r.flag_z
        rf._flag_v = r.flag_v

    def _commit_flags_addx(self, r: ALUResult68k, old_z: int) -> None:
        """Commit flags for ADDX (Z is only cleared, never set)."""
        rf = self._rf
        rf._flag_c = r.flag_c
        rf._flag_x = r.flag_c
        rf._flag_n = r.flag_n
        # ADDX Z rule: Z = old_Z AND (result == 0)
        rf._flag_z = old_z & r.flag_z
        rf._flag_v = r.flag_v

    def _commit_flags_sub(self, r: ALUResult68k) -> None:
        """Commit all flags for SUB-type operations (C=X=borrow)."""
        rf = self._rf
        rf._flag_c = r.flag_c
        rf._flag_x = r.flag_c  # X tracks C for SUB
        rf._flag_n = r.flag_n
        rf._flag_z = r.flag_z
        rf._flag_v = r.flag_v

    def _commit_flags_subx(self, r: ALUResult68k, old_z: int) -> None:
        """Commit flags for SUBX (Z only cleared)."""
        rf = self._rf
        rf._flag_c = r.flag_c
        rf._flag_x = r.flag_c
        rf._flag_n = r.flag_n
        rf._flag_z = old_z & r.flag_z
        rf._flag_v = r.flag_v

    def _commit_flags_logic(self, r: ALUResult68k) -> None:
        """Commit N/Z flags; clear V/C; leave X unchanged."""
        rf = self._rf
        rf._flag_n = r.flag_n
        rf._flag_z = r.flag_z
        rf._flag_v = 0
        rf._flag_c = 0
        # X is unchanged

    def _commit_flags_cmp(self, r: ALUResult68k) -> None:
        """Commit flags for CMP (same as SUB, but X unchanged)."""
        rf = self._rf
        rf._flag_c = r.flag_c
        rf._flag_n = r.flag_n
        rf._flag_z = r.flag_z
        rf._flag_v = r.flag_v
        # X unchanged

    def _commit_flags_move(self, r: ALUResult68k) -> None:
        """Commit N/Z; clear V/C; leave X unchanged (for MOVE operations)."""
        self._commit_flags_logic(r)

    # ──────────────────────────────────────────────────────────────────────────
    # Stack helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _push_long(self, val: int) -> None:
        sp = (self._get_a(7) - 4) & _ADDR_MASK
        self._set_a(7, sp)
        self._mem_write_long(sp, val)

    def _pop_long(self) -> int:
        sp = self._get_a(7)
        val = self._mem_read_long(sp)
        self._set_a(7, (sp + 4) & _ADDR_MASK)
        return val

    def _push_word(self, val: int) -> None:
        sp = (self._get_a(7) - 2) & _ADDR_MASK
        self._set_a(7, sp)
        self._mem_write_word(sp, val)

    def _pop_word(self) -> int:
        sp = self._get_a(7)
        val = self._mem_read_word(sp)
        self._set_a(7, (sp + 2) & _ADDR_MASK)
        return val

    # ──────────────────────────────────────────────────────────────────────────
    # Effective address resolution
    # ──────────────────────────────────────────────────────────────────────────

    def _ea_address(self, mode: int, reg: int, sz: int) -> int:
        """Compute memory address for the given EA field.

        Handles pre/postincrement side-effects.  Reads extension words from PC
        as needed.  Raises ValueError for register-direct (mode 0/1) and
        immediate (mode 7, reg 4) — these have no memory address.
        """
        if mode == 2:   # (An)
            return self._get_a(reg) & _ADDR_MASK

        if mode == 3:   # (An)+ — postincrement
            addr = self._get_a(reg) & _ADDR_MASK
            inc  = max(sz, 2) if reg == 7 else sz  # SP always increments by ≥ 2
            self._set_a(reg, (self._get_a(reg) + inc) & _ADDR_MASK)
            return addr

        if mode == 4:   # -(An) — predecrement
            dec = max(sz, 2) if reg == 7 else sz
            self._set_a(reg, (self._get_a(reg) - dec) & _ADDR_MASK)
            return self._get_a(reg) & _ADDR_MASK

        if mode == 5:   # d16(An)
            d16 = self._fetch_word_signed()
            return (self._get_a(reg) + d16) & _ADDR_MASK

        if mode == 6:   # d8(An,Xn)
            ext  = self._fetch_word()
            d8   = _sign_extend(ext & 0xFF, 8)
            xn_n = (ext >> 12) & 7
            xn_l = (ext >> 11) & 1   # 0=word, 1=long
            da   = (ext >> 15) & 1   # 0=Dn, 1=An
            xn   = self._get_a(xn_n) if da else self._get_d(xn_n, 4)
            xn = _sign_extend(xn & _WORD_MASK, 16) if not xn_l else xn & _LONG_MASK
            return (self._get_a(reg) + xn + d8) & _ADDR_MASK

        if mode == 7:
            if reg == 0:   # (abs).W
                w = self._fetch_word()
                return _sign_extend(w, 16) & _ADDR_MASK
            if reg == 1:   # (abs).L
                return self._fetch_long() & _ADDR_MASK
            if reg == 2:   # d16(PC)
                pc_base = self._rf.read_pc()
                d16     = self._fetch_word_signed()
                return (pc_base + d16) & _ADDR_MASK
            if reg == 3:   # d8(PC,Xn)
                pc_base = self._rf.read_pc()
                ext     = self._fetch_word()
                d8      = _sign_extend(ext & 0xFF, 8)
                xn_n    = (ext >> 12) & 7
                xn_l    = (ext >> 11) & 1
                da      = (ext >> 15) & 1
                xn      = self._get_a(xn_n) if da else self._get_d(xn_n, 4)
                xn = _sign_extend(xn & _WORD_MASK, 16) if not xn_l else xn & _LONG_MASK
                return (pc_base + xn + d8) & _ADDR_MASK

        raise ValueError(f"EA mode {mode}/{reg} has no memory address")

    def _ea_read(self, mode: int, reg: int, sz: int) -> int:
        """Read sz-byte value from effective address."""
        if mode == 0:               # Dn
            return self._get_d(reg, sz)
        if mode == 1:               # An (always 32-bit sign-extended)
            return self._get_a(reg) & _LONG_MASK
        if mode == 7 and reg == 4:  # immediate
            return self._fetch_imm(sz)
        addr = self._ea_address(mode, reg, sz)
        return self._mem_read(addr, sz)

    def _ea_write(self, mode: int, reg: int, sz: int, val: int) -> None:
        """Write sz-byte value to effective address."""
        if mode == 0:   # Dn
            self._set_d(reg, val, sz)
            return
        if mode == 1:   # An — word writes sign-extended to 32 bits
            if sz == 2:
                val = _sign_extend(val & _WORD_MASK, 16) & _LONG_MASK
            self._set_a(reg, val)
            return
        addr = self._ea_address(mode, reg, sz)
        self._mem_write(addr, sz, val)

    def _ea_read_addr(self, mode: int, reg: int, sz: int) -> tuple[int, int]:
        """Read from memory EA; return (value, address) for RMW operations."""
        addr = self._ea_address(mode, reg, sz)
        return self._mem_read(addr, sz), addr

    # ──────────────────────────────────────────────────────────────────────────
    # Exception helper
    # ──────────────────────────────────────────────────────────────────────────

    def _take_exception(self, vector: int) -> None:
        """Push SR and PC, load new PC from exception vector table.

        The 68000 exception mechanism:
          1. Push current SR onto stack (word)
          2. Push current PC onto stack (long)
          3. Load new PC from vector table: address = vector × 4

        Args:
            vector: Exception vector number (0–255).
        """
        self._push_word(self._rf.pack_sr())
        self._push_long(self._rf.read_pc())
        vec_addr = (vector * 4) & _ADDR_MASK
        new_pc = self._mem_read_long(vec_addr)
        self._rf.write_pc(new_pc & _ADDR_MASK)

    # ──────────────────────────────────────────────────────────────────────────
    # Main decode / execute dispatcher
    # ──────────────────────────────────────────────────────────────────────────

    def _decode_and_execute(self) -> str:
        """Fetch one instruction word and dispatch to the appropriate handler."""
        op = self._fetch_word()
        hi = (op >> 12) & 0xF

        if hi == 0x0:
            return self._exec_line0(op)
        if hi in (0x1, 0x2, 0x3):
            return self._exec_move(op)
        if hi == 0x4:
            return self._exec_line4(op)
        if hi == 0x5:
            return self._exec_line5(op)
        if hi == 0x6:
            return self._exec_line6(op)
        if hi == 0x7:
            return self._exec_moveq(op)
        if hi == 0x8:
            return self._exec_line8(op)
        if hi == 0x9:
            return self._exec_line9(op)
        if hi == 0xB:
            return self._exec_lineB(op)
        if hi == 0xC:
            return self._exec_lineC(op)
        if hi == 0xD:
            return self._exec_lineD(op)
        if hi == 0xE:
            return self._exec_lineE(op)

        # A-line or F-line: take unimplemented instruction exception
        vec = 10 if hi == 0xA else 11
        self._take_exception(vec)
        return f"LINE_{hi:X} 0x{op:04X}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 0: immediate / bit operations
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line0(self, op: int) -> str:
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # BTST/BCHG/BCLR/BSET with immediate bit number
        if (op & 0xFF00) == 0x0800:
            return self._exec_bit_imm(op)

        # BTST/BCHG/BCLR/BSET with register bit number
        if (op & 0x0138) == 0x0100 and sz_code <= 3:
            return self._exec_bit_reg(op)

        op8 = (op >> 8) & 0xFF

        if op8 == 0x00:   # ORI
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None:
                raise RuntimeError(f"ORI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            if mode == 7 and reg == 4:   # ORI #imm, CCR
                self._rf.unpack_ccr((self._rf.pack_ccr() | (imm & 0x1F)) & 0x1F)
                return "ORI #imm,CCR"
            if mode == 7 and reg == 5:   # ORI #imm, SR
                self._rf.unpack_sr(self._rf.pack_sr() | (imm & 0xFFFF))
                return "ORI #imm,SR"
            val    = self._ea_read(mode, reg, sz)
            r      = _OR_FNS[sz](val, imm)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_logic(r)
            return f"ORI.{'BWL'[sz_code]} #{imm:#x},<ea>"

        if op8 == 0x02:   # ANDI
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None:
                raise RuntimeError(f"ANDI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            if mode == 7 and reg == 4:   # ANDI #imm, CCR
                self._rf.unpack_ccr((self._rf.pack_ccr() & (imm & 0x1F)) & 0x1F)
                return "ANDI #imm,CCR"
            if mode == 7 and reg == 5:   # ANDI #imm, SR
                self._rf.unpack_sr(self._rf.pack_sr() & (imm | 0xFF00))
                return "ANDI #imm,SR"
            val    = self._ea_read(mode, reg, sz)
            r      = _AND_FNS[sz](val, imm)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_logic(r)
            return f"ANDI.{'BWL'[sz_code]} #{imm:#x},<ea>"

        if op8 == 0x04:   # SUBI
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None:
                raise RuntimeError(f"SUBI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            a   = self._ea_read(mode, reg, sz)
            r   = _SUB_FNS[sz](a, imm, 0)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_sub(r)
            return f"SUBI.{'BWL'[sz_code]} #{imm:#x},<ea>"

        if op8 == 0x06:   # ADDI
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None:
                raise RuntimeError(f"ADDI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            a   = self._ea_read(mode, reg, sz)
            r   = _ADD_FNS[sz](a, imm, 0)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_add(r)
            return f"ADDI.{'BWL'[sz_code]} #{imm:#x},<ea>"

        if op8 == 0x0A:   # EORI
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None:
                raise RuntimeError(f"EORI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            if mode == 7 and reg == 4:   # EORI #imm, CCR
                self._rf.unpack_ccr((self._rf.pack_ccr() ^ (imm & 0x1F)) & 0x1F)
                return "EORI #imm,CCR"
            val    = self._ea_read(mode, reg, sz)
            r      = _XOR_FNS[sz](val, imm)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_logic(r)
            return f"EORI.{'BWL'[sz_code]} #{imm:#x},<ea>"

        if op8 == 0x0C:   # CMPI
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None:
                raise RuntimeError(f"CMPI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            a   = self._ea_read(mode, reg, sz)
            r   = _CMP_FNS[sz](a, imm)
            self._commit_flags_cmp(r)
            return f"CMPI.{'BWL'[sz_code]} #{imm:#x},<ea>"

        raise RuntimeError(f"Unimplemented line-0 opcode 0x{op:04X}")

    def _exec_bit_imm(self, op: int) -> str:
        """BTST/BCHG/BCLR/BSET with immediate bit number."""
        kind = (op >> 6) & 3
        mode = (op >> 3) & 7
        reg  = op & 7
        bit_n = self._fetch_word() & 0x1F
        names = ["BTST", "BCHG", "BCLR", "BSET"]
        if mode == 0:   # register — 32-bit
            bit_n &= 31
            val = self._get_d(reg, 4)
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:
                self._set_d(reg, val ^ (1 << bit_n), 4)
            elif kind == 2:
                self._set_d(reg, val & ~(1 << bit_n), 4)
            elif kind == 3:
                self._set_d(reg, val | (1 << bit_n), 4)
        else:           # memory — 8-bit
            bit_n &= 7
            addr = self._ea_address(mode, reg, 1)
            val  = self._mem_read_byte(addr)
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:
                self._mem_write_byte(addr, val ^ (1 << bit_n))
            elif kind == 2:
                self._mem_write_byte(addr, val & ~(1 << bit_n))
            elif kind == 3:
                self._mem_write_byte(addr, val | (1 << bit_n))
        self._rf._flag_z = 1 if z_val else 0
        return f"{names[kind]} #{bit_n},<ea>"

    def _exec_bit_reg(self, op: int) -> str:
        """BTST/BCHG/BCLR/BSET with register-specified bit number."""
        dn   = (op >> 9) & 7
        kind = (op >> 6) & 3
        mode = (op >> 3) & 7
        reg  = op & 7
        names = ["BTST", "BCHG", "BCLR", "BSET"]
        bit_n = self._get_d(dn, 4)
        if mode == 0:
            bit_n &= 31
            val = self._get_d(reg, 4)
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:
                self._set_d(reg, val ^ (1 << bit_n), 4)
            elif kind == 2:
                self._set_d(reg, val & ~(1 << bit_n), 4)
            elif kind == 3:
                self._set_d(reg, val | (1 << bit_n), 4)
        else:
            bit_n &= 7
            addr = self._ea_address(mode, reg, 1)
            val  = self._mem_read_byte(addr)
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:
                self._mem_write_byte(addr, val ^ (1 << bit_n))
            elif kind == 2:
                self._mem_write_byte(addr, val & ~(1 << bit_n))
            elif kind == 3:
                self._mem_write_byte(addr, val | (1 << bit_n))
        self._rf._flag_z = 1 if z_val else 0
        return f"{names[kind]} D{dn},<ea>"

    # ──────────────────────────────────────────────────────────────────────────
    # Lines 1/2/3: MOVE
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_move(self, op: int) -> str:
        """MOVE / MOVEA.

        Encoding: 00ss DDD ddd MMM mmm
          ss       = size (01=byte, 10=long, 11=word)
          DDD ddd  = destination EA (reg high, mode low)
          MMM mmm  = source EA (mode high, reg low)
        """
        sz_code  = (op >> 12) & 3
        sz       = _SZ_MOVE.get(sz_code)
        if sz is None:
            raise RuntimeError(f"MOVE bad size 0x{op:04X}")
        dst_reg  = (op >> 9) & 7
        dst_mode = (op >> 6) & 7
        src_mode = (op >> 3) & 7
        src_reg  = op & 7

        val = self._ea_read(src_mode, src_reg, sz)

        if dst_mode == 1:   # MOVEA — no flags
            if sz == 2:
                val = _sign_extend(val & _WORD_MASK, 16) & _LONG_MASK
            self._set_a(dst_reg, val)
            return f"MOVEA.{'WL'[sz==4]} <ea>,A{dst_reg}"

        # MOVE: set N/Z, clear V/C, X unchanged — using gate-level OR
        # with 0 to get the flag computation path
        r = _OR_FNS[sz](val & _SZ_MASK[sz], 0)
        self._ea_write(dst_mode, dst_reg, sz, val)
        self._commit_flags_logic(r)
        _sz_suffix = {1: "B", 2: "W", 4: "L"}
        return f"MOVE.{_sz_suffix[sz]} <ea>,<ea>"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 4: miscellaneous
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line4(self, op: int) -> str:  # noqa: PLR0912 PLR0911
        mode    = (op >> 3) & 7
        reg     = op & 7
        sz_code = (op >> 6) & 3

        if op == 0x4E71:
            return "NOP"
        if op == 0x4E70:   # no-op in simulator
            return "RESET"

        if op == 0x4E75:   # RTS
            self._rf.write_pc(self._pop_long() & _ADDR_MASK)
            return "RTS"

        if op == 0x4E77:   # RTR
            ccr = self._pop_word() & 0x1F
            self._rf.unpack_ccr(ccr)
            self._rf.write_pc(self._pop_long() & _ADDR_MASK)
            return "RTR"

        if op == 0x4E73:   # RTE
            sr_val = self._pop_word()
            self._rf.unpack_sr(sr_val)
            self._rf.write_pc(self._pop_long() & _ADDR_MASK)
            return "RTE"

        if op == 0x4E72:   # STOP #imm
            imm = self._fetch_word()
            self._rf.unpack_sr(imm)
            self._halted = True
            return f"STOP #{imm:#06x}"

        if 0x4E40 <= op <= 0x4E4F:   # TRAP #n
            n = op & 0xF
            if n == 15:
                self._halted = True
            else:
                self._take_exception(32 + n)
            return f"TRAP #{n}"

        if op == 0x4AFC:   # ILLEGAL
            self._take_exception(4)
            return "ILLEGAL"

        if 0x4E50 <= op <= 0x4E57:   # LINK An, #d16
            n    = op & 7
            disp = self._fetch_word_signed()
            self._push_long(self._get_a(n))
            self._set_a(n, self._get_a(7))
            self._set_a(7, (self._get_a(7) + disp) & _ADDR_MASK)
            return f"LINK A{n},#{disp}"

        if 0x4E58 <= op <= 0x4E5F:   # UNLK An
            n = op & 7
            self._set_a(7, self._get_a(n))
            self._set_a(n, self._pop_long())
            return f"UNLK A{n}"

        if 0x4840 <= op <= 0x4847:   # SWAP Dn
            n   = op & 7
            val = self._get_d(n, 4)
            sw  = ((val >> 16) | ((val & _WORD_MASK) << 16)) & _LONG_MASK
            self._set_d(n, sw, 4)
            r = _OR_FNS[4](sw, 0)
            self._commit_flags_logic(r)
            return f"SWAP D{n}"

        if 0x4880 <= op <= 0x4887:   # EXT.W Dn
            n = op & 7
            b = _sign_extend(self._get_d(n, 1), 8)
            w = b & _WORD_MASK
            self._set_d(n, w, 2)
            r = _OR_FNS[2](w, 0)
            self._commit_flags_logic(r)
            return f"EXT.W D{n}"

        if 0x48C0 <= op <= 0x48C7:   # EXT.L Dn
            n  = op & 7
            wv = _sign_extend(self._get_d(n, 2), 16)
            lw = wv & _LONG_MASK
            self._set_d(n, lw, 4)
            r = _OR_FNS[4](lw, 0)
            self._commit_flags_logic(r)
            return f"EXT.L D{n}"

        if 0x40C0 <= op <= 0x40C7:   # MOVE SR, Dn
            n = op & 7
            self._set_d(n, self._rf.pack_sr(), 2)
            return f"MOVE SR,D{n}"

        if 0x42C0 <= op <= 0x42C7:   # MOVE CCR, Dn
            n = op & 7
            self._set_d(n, self._rf.pack_ccr(), 2)
            return f"MOVE CCR,D{n}"

        if op == 0x44FC:   # MOVE #imm, CCR
            imm = self._fetch_word()
            self._rf.unpack_ccr(imm & 0x1F)
            return f"MOVE #{imm & 0x1F:#x},CCR"

        if op == 0x46FC:   # MOVE #imm, SR
            imm = self._fetch_word()
            self._rf.unpack_sr(imm & 0xFFFF)
            return f"MOVE #{imm:#06x},SR"

        # ── MOVE <ea>, SR ────────────────────────────────────────────────────
        if (op & 0xFFC0) == 0x46C0:
            val = self._ea_read(mode, reg, 2)
            self._rf.unpack_sr(val & 0xFFFF)
            return "MOVE <ea>,SR"

        # ── MOVE <ea>, CCR ───────────────────────────────────────────────────
        if (op & 0xFFC0) == 0x44C0:
            val = self._ea_read(mode, reg, 2)
            self._rf.unpack_ccr(val & 0x1F)
            return "MOVE <ea>,CCR"

        # ── MOVE SR, <ea> ────────────────────────────────────────────────────
        if (op & 0xFFC0) == 0x40C0:
            self._ea_write(mode, reg, 2, self._rf.pack_sr())
            return "MOVE SR,<ea>"

        # ── NEGX ─────────────────────────────────────────────────────────────
        if (op & 0xFF00) == 0x4000 and sz_code <= 2:
            sz  = _SZ_ARITH[sz_code]
            a   = self._ea_read(mode, reg, sz)
            x   = self._rf._flag_x
            r   = _SUB_FNS[sz](0, a, x)
            self._ea_write(mode, reg, sz, r.result)
            old_z = self._rf._flag_z
            self._commit_flags_sub(r)
            # NEGX Z rule: Z only cleared
            self._rf._flag_z = old_z & r.flag_z
            return f"NEGX.{'BWL'[sz_code]} <ea>"

        # ── CLR ──────────────────────────────────────────────────────────────
        if (op & 0xFF00) == 0x4200 and sz_code <= 2:
            sz = _SZ_ARITH[sz_code]
            self._ea_write(mode, reg, sz, 0)
            self._rf._flag_n = 0
            self._rf._flag_z = 1
            self._rf._flag_v = 0
            self._rf._flag_c = 0
            return f"CLR.{'BWL'[sz_code]} <ea>"

        # ── NEG ──────────────────────────────────────────────────────────────
        if (op & 0xFF00) == 0x4400 and sz_code <= 2:
            sz  = _SZ_ARITH[sz_code]
            src = self._ea_read(mode, reg, sz)
            r   = _NEG_FNS[sz](src)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_sub(r)
            return f"NEG.{'BWL'[sz_code]} <ea>"

        # ── NOT ──────────────────────────────────────────────────────────────
        if (op & 0xFF00) == 0x4600 and sz_code <= 2:
            sz     = _SZ_ARITH[sz_code]
            val    = self._ea_read(mode, reg, sz)
            result = _NOT_FNS[sz](val)
            self._ea_write(mode, reg, sz, result)
            r = _OR_FNS[sz](result, 0)   # gate-level flag computation
            self._commit_flags_logic(r)
            return f"NOT.{'BWL'[sz_code]} <ea>"

        # ── TST ──────────────────────────────────────────────────────────────
        if (op & 0xFF00) == 0x4A00 and sz_code <= 2:
            sz  = _SZ_ARITH[sz_code]
            val = self._ea_read(mode, reg, sz) & _SZ_MASK[sz]
            r   = _OR_FNS[sz](val, 0)
            self._commit_flags_logic(r)
            return f"TST.{'BWL'[sz_code]} <ea>"

        # ── PEA ──────────────────────────────────────────────────────────────
        if (op & 0xFFC0) == 0x4840 and mode >= 2:
            addr = self._ea_address(mode, reg, 4)
            self._push_long(addr)
            return "PEA <ea>"

        # ── LEA ──────────────────────────────────────────────────────────────
        if (op & 0xF1C0) == 0x41C0:
            an   = (op >> 9) & 7
            addr = self._ea_address(mode, reg, 4)
            self._set_a(an, addr)
            return f"LEA <ea>,A{an}"

        # ── JSR ──────────────────────────────────────────────────────────────
        if (op & 0xFFC0) == 0x4E80:
            addr = self._ea_address(mode, reg, 4)
            self._push_long(self._rf.read_pc())
            self._rf.write_pc(addr)
            return "JSR <ea>"

        # ── JMP ──────────────────────────────────────────────────────────────
        if (op & 0xFFC0) == 0x4EC0:
            addr = self._ea_address(mode, reg, 4)
            self._rf.write_pc(addr)
            return "JMP <ea>"

        # ── NBCD ─────────────────────────────────────────────────────────────
        # NBCD <ea>: 0100 1000 00 mmm rrr — negate BCD with extend.
        # Valid for all data alterable EAs: Dn (mode 0), memory modes, etc.
        # Mode 1 (An) is not a valid destination for byte operations.
        if (op & 0xFFC0) == 0x4800 and mode != 1:
            val = self._ea_read(mode, reg, 1)
            x   = self._rf._flag_x
            result = (0x9A - val - x) & 0xFF
            self._ea_write(mode, reg, 1, result)
            # C and X set if result != 0
            c = 1 if result != 0 else 0
            self._rf._flag_c = c
            self._rf._flag_x = c
            old_z = self._rf._flag_z
            self._rf._flag_z = old_z & (1 if result == 0 else 0)
            return "NBCD <ea>"

        # ── CHK ──────────────────────────────────────────────────────────────
        if (op & 0xF1C0) == 0x4180:
            dn  = (op >> 9) & 7
            val = self._ea_read(mode, reg, 2)
            dn_val = self._get_d(dn, 2)
            # Sign-extend for comparison
            dn_s  = _sign_extend(dn_val, 16)
            val_s = _sign_extend(val, 16)
            if dn_s < 0 or dn_s > val_s:
                self._take_exception(6)
            return f"CHK <ea>,D{dn}"

        # ── MOVEM ────────────────────────────────────────────────────────────
        if (op & 0xFB80) == 0x4880:   # registers → memory (bit 10 determines dir)
            return self._exec_movem(op)
        if (op & 0xFB80) == 0x4C80:
            return self._exec_movem(op)

        raise RuntimeError(f"Unimplemented line-4 opcode 0x{op:04X}")

    def _exec_movem(self, op: int) -> str:
        """MOVEM — move multiple registers to/from memory."""
        sz     = 4 if (op >> 6) & 1 else 2
        mode   = (op >> 3) & 7
        reg    = op & 7
        to_mem = not bool(op & 0x0400)   # bit 10: 0=to_mem, 1=from_mem
        mask   = self._fetch_word()

        if to_mem:
            if mode == 4:   # predecrement: mask is REVERSED — bit 0=A7..bit7=A0, bit8=D7..bit15=D0
                # Push in reverse order (A7..A0 first, then D7..D0) so pop restores in correct order.
                # In the reversed mask: bit (15 - i) corresponds to D[i] and bit (7 - i) to A[i].
                for i in range(7, -1, -1):   # A7..A0
                    if mask & (1 << (7 - i)):
                        val = self._get_a(i)
                        dec = max(sz, 2) if reg == 7 else sz
                        self._set_a(reg, (self._get_a(reg) - dec) & _ADDR_MASK)
                        self._mem_write(self._get_a(reg), sz, val)
                for i in range(7, -1, -1):   # D7..D0
                    if mask & (1 << (15 - i)):
                        val = self._get_d(i, sz)
                        dec = max(sz, 2) if reg == 7 else sz
                        self._set_a(reg, (self._get_a(reg) - dec) & _ADDR_MASK)
                        self._mem_write(self._get_a(reg), sz, val)
            else:
                addr = self._ea_address(mode, reg, sz)
                for i in range(8):
                    if mask & (1 << i):
                        self._mem_write(addr, sz, self._get_d(i, sz))
                        addr = (addr + sz) & _ADDR_MASK
                for i in range(8):
                    if mask & (1 << (8 + i)):
                        self._mem_write(addr, sz, self._get_a(i))
                        addr = (addr + sz) & _ADDR_MASK
        else:  # from memory
            addr = self._ea_address(mode, reg, sz)
            for i in range(8):
                if mask & (1 << i):
                    val = self._mem_read(addr, sz)
                    if sz == 2:
                        val = _sign_extend(val, 16) & _LONG_MASK
                    self._set_d(i, val, 4)
                    addr = (addr + sz) & _ADDR_MASK
            for i in range(8):
                if mask & (1 << (8 + i)):
                    val = self._mem_read(addr, sz)
                    if sz == 2:
                        val = _sign_extend(val, 16) & _LONG_MASK
                    self._set_a(i, val)
                    addr = (addr + sz) & _ADDR_MASK

        return f"MOVEM.{'WL'[sz==4]} {'regs,<ea>' if to_mem else '<ea>,regs'}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 5: ADDQ / SUBQ / Scc / DBcc
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line5(self, op: int) -> str:
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7
        cc      = (op >> 8) & 0xF

        if sz_code == 3:
            if mode == 1:   # DBcc
                n_flag = self._rf._flag_n
                z_flag = self._rf._flag_z
                v_flag = self._rf._flag_v
                c_flag = self._rf._flag_c
                cond   = _CC_FUNCS[cc](n_flag, z_flag, v_flag, c_flag)
                disp   = self._fetch_word_signed()
                if not cond:
                    count = _sign_extend(self._get_d(reg, 2), 16)
                    count = (count - 1) & _WORD_MASK
                    self._set_d(reg, count, 2)
                    if count != 0xFFFF:   # not -1
                        pc = self._rf.read_pc()
                        self._rf.write_pc((pc - 2 + disp) & _ADDR_MASK)
                return f"DB{_CC_NAMES[cc]} D{reg},d16"

            # Scc: set byte on condition
            n_flag = self._rf._flag_n
            z_flag = self._rf._flag_z
            v_flag = self._rf._flag_v
            c_flag = self._rf._flag_c
            cond   = _CC_FUNCS[cc](n_flag, z_flag, v_flag, c_flag)
            val    = 0xFF if cond else 0x00
            self._ea_write(mode, reg, 1, val)
            return f"S{_CC_NAMES[cc]} <ea>"

        # ADDQ or SUBQ
        sz  = _SZ_ARITH.get(sz_code)
        if sz is None:
            raise RuntimeError(f"ADDQ/SUBQ bad size 0x{op:04X}")
        data = (op >> 9) & 7
        imm  = 8 if data == 0 else data   # 0 encodes 8

        if not (op & 0x0100):   # ADDQ
            if mode == 1:   # ADDQ to An — no flags affected
                val = self._get_a(reg)
                # Sign-extend imm if sz==2 (hardware quirk: full 32-bit add)
                r = add32(val, imm, 0)
                self._set_a(reg, r.result)
            else:
                a  = self._ea_read(mode, reg, sz)
                r  = _ADD_FNS[sz](a, imm, 0)
                self._ea_write(mode, reg, sz, r.result)
                self._commit_flags_add(r)
        else:   # SUBQ
            if mode == 1:   # SUBQ from An — no flags affected
                val = self._get_a(reg)
                r   = sub32(val, imm, 0)
                self._set_a(reg, r.result)
            else:
                a   = self._ea_read(mode, reg, sz)
                r   = _SUB_FNS[sz](a, imm, 0)
                self._ea_write(mode, reg, sz, r.result)
                self._commit_flags_sub(r)
        return f"{'ADDQ' if not (op & 0x0100) else 'SUBQ'}.{'BWL'[sz_code]} #{imm},<ea>"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 6: BRA / BSR / Bcc
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line6(self, op: int) -> str:
        cc    = (op >> 8) & 0xF
        disp8 = op & 0xFF
        # The branch base is PC after the opword (and extension word if any).
        disp = self._fetch_word_signed() if disp8 == 0 else _sign_extend(disp8, 8)
        pc_after_op = self._rf.read_pc()  # already advanced past opword (+ ext if any)

        if cc == 0:   # BRA
            self._rf.write_pc((pc_after_op - (0 if disp8 != 0 else 2) + disp) & _ADDR_MASK)
            return f"BRA d{disp:+d}"

        if cc == 1:   # BSR
            self._push_long(pc_after_op - (0 if disp8 != 0 else 0))
            self._rf.write_pc((pc_after_op - (0 if disp8 != 0 else 2) + disp) & _ADDR_MASK)
            return f"BSR d{disp:+d}"

        # Bcc
        n = self._rf._flag_n
        z = self._rf._flag_z
        v = self._rf._flag_v
        c = self._rf._flag_c
        cond = _CC_FUNCS[cc](n, z, v, c)
        if cond:
            self._rf.write_pc((pc_after_op - (0 if disp8 != 0 else 2) + disp) & _ADDR_MASK)
        return f"B{_CC_NAMES[cc]} d{disp:+d}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 7: MOVEQ
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_moveq(self, op: int) -> str:
        """MOVEQ #imm, Dn — sign-extended 8-bit immediate to full 32-bit Dn."""
        dn  = (op >> 9) & 7
        imm = _sign_extend(op & 0xFF, 8) & _LONG_MASK
        self._set_d(dn, imm, 4)
        r = _OR_FNS[4](imm, 0)
        self._commit_flags_logic(r)
        return f"MOVEQ #{_sign_extend(op & 0xFF, 8)},D{dn}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 8: OR / DIVU / DIVS / SBCD
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line8(self, op: int) -> str:
        dn     = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode   = (op >> 3) & 7
        reg    = op & 7

        if opmode == 3:   # DIVU <ea>, Dn
            src = self._ea_read(mode, reg, 2)
            if src == 0:
                self._take_exception(5)
                return "DIVU #0,Dn"
            try:
                packed, overflow = divu(self._get_d(dn, 4), src)
                if overflow:
                    self._rf._flag_v = 1
                    self._rf._flag_n = 1
                    self._rf._flag_c = 0
                    self._rf._flag_z = 0
                else:
                    self._set_d(dn, packed, 4)
                    q16 = packed & _WORD_MASK
                    self._rf._flag_v = 0
                    self._rf._flag_c = 0
                    self._rf._flag_n = (q16 >> 15) & 1
                    self._rf._flag_z = 1 if q16 == 0 else 0
            except ZeroDivisionError:
                self._take_exception(5)
            return f"DIVU <ea>,D{dn}"

        if opmode == 7:   # DIVS <ea>, Dn
            src = self._ea_read(mode, reg, 2)
            if src == 0:
                self._take_exception(5)
                return "DIVS #0,Dn"
            try:
                result = divs(self._get_d(dn, 4), src)
                packed, overflow = result
                if overflow:
                    self._rf._flag_v = 1
                    self._rf._flag_n = 1
                    self._rf._flag_c = 0
                else:
                    self._set_d(dn, packed, 4)
                    q16 = packed & _WORD_MASK
                    q_s = _sign_extend(q16, 16)
                    self._rf._flag_v = 0
                    self._rf._flag_c = 0
                    self._rf._flag_n = 1 if q_s < 0 else 0
                    self._rf._flag_z = 1 if q_s == 0 else 0
            except ZeroDivisionError:
                self._take_exception(5)
            return f"DIVS <ea>,D{dn}"

        if (op & 0x01F0) == 0x0100:   # SBCD
            rx = reg
            ry = dn
            x  = self._rf._flag_x
            if mode == 0:   # SBCD Dx, Dy
                a = self._get_d(ry, 1)
                b = self._get_d(rx, 1)
            else:            # SBCD -(Ax), -(Ay)
                self._set_a(rx, (self._get_a(rx) - 1) & _ADDR_MASK)
                b = self._mem_read_byte(self._get_a(rx))
                self._set_a(ry, (self._get_a(ry) - 1) & _ADDR_MASK)
                a = self._mem_read_byte(self._get_a(ry))
            result = a - b - x
            if (result & 0xF) > 9 or result < 0:
                result -= 6
            if result < 0 or result > 0x99:
                result -= 0x60
                c = 1
            else:
                c = 0
            result &= 0xFF
            if mode == 0:
                self._set_d(ry, result, 1)
            else:
                self._mem_write_byte(self._get_a(ry), result)
            self._rf._flag_c = c
            self._rf._flag_x = c
            old_z = self._rf._flag_z
            self._rf._flag_z = old_z & (1 if result == 0 else 0)
            return "SBCD"

        # OR
        sz = _SZ_ARITH.get(opmode & 3)
        if sz is None:
            sz = _SZ_ARITH.get((opmode >> 1) & 3, 2)
        if opmode <= 2:   # <ea> → Dn
            sz  = _SZ_ARITH.get(opmode, 2)
            src = self._ea_read(mode, reg, sz)
            dst = self._get_d(dn, sz)
            r   = _OR_FNS[sz](dst, src)
            self._set_d(dn, r.result, sz)
            self._commit_flags_logic(r)
        else:             # Dn → <ea>
            sz  = _SZ_ARITH.get(opmode - 4, 2)
            src = self._get_d(dn, sz)
            dst = self._ea_read(mode, reg, sz)
            r   = _OR_FNS[sz](dst, src)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_logic(r)
        return f"OR.{'BWL'[list(_SZ_ARITH.values()).index(sz)]} <ea>,D{dn}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 9: SUB / SUBA / SUBX
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line9(self, op: int) -> str:
        dn     = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode   = (op >> 3) & 7
        reg    = op & 7

        if opmode in (3, 7):   # SUBA
            sz  = 4 if opmode == 7 else 2
            src = self._ea_read(mode, reg, sz)
            if sz == 2:
                src = _sign_extend(src, 16) & _LONG_MASK
            r = sub32(self._get_a(dn), src, 0)
            self._set_a(dn, r.result)
            return f"SUBA.{'WL'[opmode==7]} <ea>,A{dn}"

        # SUBX: bit 8=1, bits 7-6=sz (00=B,01=W,10=L), bit 5=0 (Dn) or 1 (An predec)
        # opmode values: 4=SUBX.B, 5=SUBX.W, 6=SUBX.L
        if opmode in (4, 5, 6) and mode == 0:   # SUBX Dm, Dn
            sz_idx = {4: 0, 5: 1, 6: 2}[opmode]
            sz  = _SZ_ARITH[sz_idx]
            x   = self._rf._flag_x
            a   = self._get_d(dn, sz)
            b   = self._get_d(reg, sz)
            r   = _SUB_FNS[sz](a, b, x)
            self._set_d(dn, r.result, sz)
            old_z = self._rf._flag_z
            self._commit_flags_subx(r, old_z)
            return f"SUBX D{reg},D{dn}"

        if opmode in (4, 5, 6) and mode == 4:   # SUBX -(Am), -(An)
            sz_idx = {4: 0, 5: 1, 6: 2}[opmode]
            sz  = _SZ_ARITH[sz_idx]
            x   = self._rf._flag_x
            dec = max(sz, 2) if reg == 7 else sz
            self._set_a(reg, (self._get_a(reg) - dec) & _ADDR_MASK)
            b   = self._mem_read(self._get_a(reg), sz)
            dec2 = max(sz, 2) if dn == 7 else sz
            self._set_a(dn, (self._get_a(dn) - dec2) & _ADDR_MASK)
            a   = self._mem_read(self._get_a(dn), sz)
            r   = _SUB_FNS[sz](a, b, x)
            self._mem_write(self._get_a(dn), sz, r.result)
            old_z = self._rf._flag_z
            self._commit_flags_subx(r, old_z)
            return f"SUBX -(A{reg}),-(A{dn})"

        # SUB <ea>, Dn or SUB Dn, <ea>
        if opmode <= 2:
            sz  = _SZ_ARITH[opmode]
            src = self._ea_read(mode, reg, sz)
            dst = self._get_d(dn, sz)
            r   = _SUB_FNS[sz](dst, src, 0)
            self._set_d(dn, r.result, sz)
            self._commit_flags_sub(r)
        else:
            sz  = _SZ_ARITH[opmode - 4]
            src = self._get_d(dn, sz)
            dst = self._ea_read(mode, reg, sz)
            r   = _SUB_FNS[sz](dst, src, 0)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_sub(r)
        return f"SUB.{'BWL'[opmode if opmode<=2 else opmode-4]} <ea>,D{dn}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line B: CMP / CMPA / EOR / CMPM
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineB(self, op: int) -> str:
        dn     = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode   = (op >> 3) & 7
        reg    = op & 7

        if opmode in (3, 7):   # CMPA
            sz  = 4 if opmode == 7 else 2
            src = self._ea_read(mode, reg, sz)
            if sz == 2:
                src = _sign_extend(src, 16) & _LONG_MASK
            r = cmp32(self._get_a(dn), src)
            self._commit_flags_cmp(r)
            return f"CMPA.{'WL'[opmode==7]} <ea>,A{dn}"

        if opmode <= 2:   # CMP
            sz  = _SZ_ARITH[opmode]
            src = self._ea_read(mode, reg, sz)
            dst = self._get_d(dn, sz)
            r   = _CMP_FNS[sz](dst, src)
            self._commit_flags_cmp(r)
            return f"CMP.{'BWL'[opmode]} <ea>,D{dn}"

        # opmode 4,5,6 — EOR or CMPM
        sz_idx = opmode - 4
        sz     = _SZ_ARITH[sz_idx]

        if mode == 1:   # CMPM (An)+, (An)+
            inc_src = max(sz, 2) if reg == 7 else sz
            src_addr = self._get_a(reg) & _ADDR_MASK
            self._set_a(reg, (self._get_a(reg) + inc_src) & _ADDR_MASK)
            inc_dst = max(sz, 2) if dn == 7 else sz
            dst_addr = self._get_a(dn) & _ADDR_MASK
            self._set_a(dn, (self._get_a(dn) + inc_dst) & _ADDR_MASK)
            src = self._mem_read(src_addr, sz)
            dst = self._mem_read(dst_addr, sz)
            r   = _CMP_FNS[sz](dst, src)
            self._commit_flags_cmp(r)
            return f"CMPM (A{reg})+,(A{dn})+"

        # EOR Dn, <ea>
        src = self._get_d(dn, sz)
        dst = self._ea_read(mode, reg, sz)
        r   = _XOR_FNS[sz](dst, src)
        self._ea_write(mode, reg, sz, r.result)
        self._commit_flags_logic(r)
        return f"EOR.{'BWL'[sz_idx]} D{dn},<ea>"

    # ──────────────────────────────────────────────────────────────────────────
    # Line C: AND / MULU / MULS / EXG / ABCD
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineC(self, op: int) -> str:
        dn     = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode   = (op >> 3) & 7
        reg    = op & 7

        if opmode == 3:   # MULU
            src = self._ea_read(mode, reg, 2)
            result32, n, z = mulu(self._get_d(dn, 4), src)
            self._set_d(dn, result32, 4)
            self._rf._flag_n = n
            self._rf._flag_z = z
            self._rf._flag_v = 0
            self._rf._flag_c = 0
            return f"MULU <ea>,D{dn}"

        if opmode == 7:   # MULS
            src = self._ea_read(mode, reg, 2)
            result32, n, z = muls(self._get_d(dn, 4), src)
            self._set_d(dn, result32, 4)
            self._rf._flag_n = n
            self._rf._flag_z = z
            self._rf._flag_v = 0
            self._rf._flag_c = 0
            return f"MULS <ea>,D{dn}"

        if (op & 0x01F0) == 0x0100:   # ABCD
            # ABCD Rx, Ry (register mode) or ABCD -(Ax), -(Ay) (predecrement).
            # Encoding: 1100 yyy 1 0000 0 xxx  where yyy=Ry(dst), xxx=Rx(src).
            # BCD addition: the hardware adds the two BCD bytes plus the X flag
            # using the standard DAA (decimal-adjust-after-add) algorithm:
            #   1. Binary-add the two bytes and X.
            #   2. If lower nibble > 9, add 6 to produce carry into upper nibble.
            #   3. If upper nibble > 9 (or there is a carry from step 2 that
            #      pushes the upper nibble over 9), add 0x60.
            rx = reg
            ry = dn
            x  = self._rf._flag_x
            if mode == 0:
                a = self._get_d(ry, 1)
                b = self._get_d(rx, 1)
            else:
                self._set_a(rx, (self._get_a(rx) - 1) & _ADDR_MASK)
                b = self._mem_read_byte(self._get_a(rx))
                self._set_a(ry, (self._get_a(ry) - 1) & _ADDR_MASK)
                a = self._mem_read_byte(self._get_a(ry))
            # Standard BCD-adjust algorithm.
            result = a + b + x          # raw binary sum (up to 0x1FE + 1)
            if (result & 0xF) > 9:      # lower nibble overflow: adjust +6
                result += 6
            c = 1 if result > 0x99 else 0
            if c:                       # upper nibble overflow: adjust +0x60
                result += 0x60
            bcd = result & 0xFF
            if mode == 0:
                self._set_d(ry, bcd, 1)
            else:
                self._mem_write_byte(self._get_a(ry), bcd)
            self._rf._flag_c = c
            self._rf._flag_x = c
            old_z = self._rf._flag_z
            self._rf._flag_z = old_z & (1 if bcd == 0 else 0)
            return "ABCD"

        if (op & 0x01F8) == 0x0140:   # EXG Dn, Dn
            a, b = self._get_d(dn, 4), self._get_d(reg, 4)
            self._set_d(dn, b, 4)
            self._set_d(reg, a, 4)
            return f"EXG D{dn},D{reg}"

        if (op & 0x01F8) == 0x0148:   # EXG An, An
            a, b = self._get_a(dn), self._get_a(reg)
            self._set_a(dn, b)
            self._set_a(reg, a)
            return f"EXG A{dn},A{reg}"

        if (op & 0x01F8) == 0x0188:   # EXG Dn, An
            a, b = self._get_d(dn, 4), self._get_a(reg)
            self._set_d(dn, b, 4)
            self._set_a(reg, a)
            return f"EXG D{dn},A{reg}"

        # AND
        if opmode <= 2:
            sz  = _SZ_ARITH[opmode]
            src = self._ea_read(mode, reg, sz)
            dst = self._get_d(dn, sz)
            r   = _AND_FNS[sz](dst, src)
            self._set_d(dn, r.result, sz)
            self._commit_flags_logic(r)
        else:
            sz  = _SZ_ARITH[opmode - 4]
            src = self._get_d(dn, sz)
            dst = self._ea_read(mode, reg, sz)
            r   = _AND_FNS[sz](dst, src)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_logic(r)
        return f"AND.{'BWL'[opmode if opmode<=2 else opmode-4]} <ea>,D{dn}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line D: ADD / ADDA / ADDX
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineD(self, op: int) -> str:
        dn     = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode   = (op >> 3) & 7
        reg    = op & 7

        if opmode in (3, 7):   # ADDA
            sz  = 4 if opmode == 7 else 2
            src = self._ea_read(mode, reg, sz)
            if sz == 2:
                src = _sign_extend(src, 16) & _LONG_MASK
            r = add32(self._get_a(dn), src, 0)
            self._set_a(dn, r.result)
            return f"ADDA.{'WL'[opmode==7]} <ea>,A{dn}"

        # ADDX: bit 8=1, bits 7-6=sz (00=B,01=W,10=L), bit 5=0 (Dn) or 1 (An predec)
        # opmode values: 4=ADDX.B, 5=ADDX.W, 6=ADDX.L
        if opmode in (4, 5, 6) and mode == 0:   # ADDX Dm, Dn
            sz_idx = {4: 0, 5: 1, 6: 2}[opmode]
            sz  = _SZ_ARITH[sz_idx]
            x   = self._rf._flag_x
            a   = self._get_d(dn, sz)
            b   = self._get_d(reg, sz)
            r   = _ADD_FNS[sz](a, b, x)
            self._set_d(dn, r.result, sz)
            old_z = self._rf._flag_z
            self._commit_flags_addx(r, old_z)
            return f"ADDX D{reg},D{dn}"

        if opmode in (4, 5, 6) and mode == 4:   # ADDX -(Am), -(An)
            sz_idx = {4: 0, 5: 1, 6: 2}[opmode]
            sz  = _SZ_ARITH[sz_idx]
            x   = self._rf._flag_x
            dec = max(sz, 2) if reg == 7 else sz
            self._set_a(reg, (self._get_a(reg) - dec) & _ADDR_MASK)
            b   = self._mem_read(self._get_a(reg), sz)
            dec2 = max(sz, 2) if dn == 7 else sz
            self._set_a(dn, (self._get_a(dn) - dec2) & _ADDR_MASK)
            a   = self._mem_read(self._get_a(dn), sz)
            r   = _ADD_FNS[sz](a, b, x)
            self._mem_write(self._get_a(dn), sz, r.result)
            old_z = self._rf._flag_z
            self._commit_flags_addx(r, old_z)
            return f"ADDX -(A{reg}),-(A{dn})"

        # ADD <ea>, Dn or ADD Dn, <ea>
        if opmode <= 2:
            sz  = _SZ_ARITH[opmode]
            src = self._ea_read(mode, reg, sz)
            dst = self._get_d(dn, sz)
            r   = _ADD_FNS[sz](dst, src, 0)
            self._set_d(dn, r.result, sz)
            self._commit_flags_add(r)
        else:
            sz  = _SZ_ARITH[opmode - 4]
            src = self._get_d(dn, sz)
            dst = self._ea_read(mode, reg, sz)
            r   = _ADD_FNS[sz](dst, src, 0)
            self._ea_write(mode, reg, sz, r.result)
            self._commit_flags_add(r)
        return f"ADD.{'BWL'[opmode if opmode<=2 else opmode-4]} <ea>,D{dn}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line E: shifts and rotates
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineE(self, op: int) -> str:
        sz_code = (op >> 6) & 3
        dir_bit = (op >> 8) & 1   # 0=right, 1=left
        # For memory shifts (sz_code==3), shift type is in bits 11-9.
        # For register/immediate shifts, shift type is in bits 4-3.
        # 00=AS, 01=LS, 10=ROX, 11=RO
        reg     = op & 7

        if sz_code == 3:   # memory shift/rotate — 1 bit only on word
            mode = (op >> 3) & 7
            shift_t = (op >> 9) & 3  # bits 11-9 for memory shifts
            val, addr = self._ea_read_addr(mode, reg, 2)
            count = 1
            result, c, v = self._do_shift(val, count, 2, shift_t, dir_bit)
            self._mem_write(addr, 2, result)
            self._apply_shift_flags(result, 2, c, v, shift_t, count)
            names = ["AS", "LS", "ROX", "RO"]
            return f"{names[shift_t]}{'LR'[not dir_bit]}.W <ea>"

        # Register/immediate shift: shift type in bits 4-3, ir_bit in bit 5.
        shift_t = (op >> 3) & 3  # bits 4-3: 00=AS, 01=LS, 10=ROX, 11=RO
        ir_bit  = (op >> 5) & 1  # 0=count is immediate, 1=count from Dn

        sz = _SZ_ARITH[sz_code]
        if ir_bit:   # count from register (bits 11-9 = register number)
            count = self._get_d((op >> 9) & 7, 4) % 64
        else:        # immediate count in bits 11-9 (0 means 8)
            count = (op >> 9) & 7
            if count == 0:
                count = 8

        val    = self._get_d(reg, sz)
        result, c, v = self._do_shift(val, count, sz, shift_t, dir_bit)
        self._set_d(reg, result, sz)
        self._apply_shift_flags(result, sz, c, v, shift_t, count)
        names = ["AS", "LS", "ROX", "RO"]
        return f"{names[shift_t]}{'LR'[not dir_bit]}.{'BWL'[sz_code]} D{reg}"

    def _do_shift(
        self, val: int, count: int, sz: int, shift_t: int, dir_left: int
    ) -> tuple[int, int, int]:
        """Perform shift/rotate and return (result, c_flag, v_flag).

        shift_t: 0=AS, 1=LS, 2=ROX, 3=RO
        dir_left: 1=left, 0=right
        """
        x = self._rf._flag_x
        width = sz * 8

        if shift_t == 0:   # AS (arithmetic)
            if dir_left:
                result, c, v = asl(val, count, width)
            else:
                result, c = asr(val, count, width)
                v = 0
        elif shift_t == 1:  # LS (logical)
            if dir_left:
                result, c = lsl(val, count, width)
                v = 0
            else:
                result, c = lsr(val, count, width)
                v = 0
        elif shift_t == 2:  # ROX (rotate through X)
            if dir_left:
                result, c = roxl(val, count, width, x)
            else:
                result, c = roxr(val, count, width, x)
            v = 0
        else:               # RO (rotate)
            if dir_left:
                result, c = rol(val, count, width)
            else:
                result, c = ror(val, count, width)
            v = 0

        return result, c, v

    def _apply_shift_flags(
        self, result: int, sz: int, c: int, v: int, shift_t: int, count: int
    ) -> None:
        """Apply condition code flags after a shift/rotate."""
        width = sz * 8
        mask  = (1 << width) - 1
        n = (result >> (width - 1)) & 1
        z = 1 if (result & mask) == 0 else 0

        self._rf._flag_n = n
        self._rf._flag_z = z
        self._rf._flag_v = v

        if count == 0:
            # Count=0: C=0 for all except rotate (C=unchanged for RO/ROX)
            if shift_t in (0, 1):
                self._rf._flag_c = 0
            # X unchanged for count=0
        else:
            self._rf._flag_c = c
            if shift_t in (0, 1, 2):   # AS/LS/ROX update X
                self._rf._flag_x = c
            # RO does not update X
