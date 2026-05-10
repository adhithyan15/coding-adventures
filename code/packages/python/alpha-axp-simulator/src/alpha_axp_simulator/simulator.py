"""AlphaSimulator — DEC Alpha AXP 21064 (1992) behavioral simulator.

This module implements a full behavioral model of the DEC Alpha AXP 21064
integer instruction set.  It follows the SIM00 Simulator[AlphaState] protocol.

Architecture overview
─────────────────────
The Alpha AXP 21064 was Digital Equipment Corporation's first 64-bit RISC
processor, introduced in February 1992.  Designed by Richard Sites and his
team, it achieved 200 MIPS at 200 MHz — roughly 5× faster than the Intel
486 at the time.

Key design principles:
  1. No condition codes — comparisons write 0/1 to GPRs (like MIPS SLT)
  2. No delay slots — unlike MIPS R2000, branches take effect immediately
  3. Flat register file — no windows (unlike SPARC), 32 × 64-bit registers
  4. 64-bit throughout — no 32-bit half-ops at the ISA level (ADDL/SUBL
     operate on 32-bit halves but sign-extend the result to 64 bits)
  5. Little-endian — unique in this simulator series; all prior simulators
     (MIPS, SPARC, Z80, PDP-11, Motorola 68000) are big-endian

Instruction formats (all 32-bit fixed-length):
  Memory:  [op:6][Ra:5][Rb:5][disp16:16]  — ea = Rb + sext(disp16)
  Branch:  [op:6][Ra:5][disp21:21]         — target = (PC+4) + sext(disp21)*4
  Operate: [op:6][Ra:5][Rb/lit:5+1][func:7][Rc:5] — i-bit selects reg/imm
  Jump:    [op:6=0x1A][Ra:5][Rb:5][func:2][hint:14]
  PALcode: [op:6=0x00][palcode:26]  — HALT = 0x00000000

Memory model: 64 KiB flat, little-endian.  Quadword (8-byte) and longword
(4-byte) operations require aligned addresses.

HALT: call_pal 0x0000 = the all-zeros word 0x00000000.
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .state import (
    MEM_SIZE,
    NUM_REGS,
    REG_ZERO,
    AlphaState,
)

# ── Bit-width masks ────────────────────────────────────────────────────────────

MASK64: int = 0xFFFF_FFFF_FFFF_FFFF
MASK32: int = 0xFFFF_FFFF
MASK16: int = 0xFFFF
MASK8:  int = 0xFF


# ── Numeric helpers ────────────────────────────────────────────────────────────

def _u64(v: int) -> int:
    """Mask result to unsigned 64 bits."""
    return int(v) & MASK64


def _u32(v: int) -> int:
    """Mask to unsigned 32 bits."""
    return int(v) & MASK32


def _s64(v: int) -> int:
    """Interpret unsigned 64-bit value as signed Python int."""
    v = int(v) & MASK64
    if v >= 0x8000_0000_0000_0000:
        v -= 0x1_0000_0000_0000_0000
    return v


def _s32(v: int) -> int:
    """Interpret unsigned 32-bit value as signed Python int."""
    v = int(v) & MASK32
    if v >= 0x8000_0000:
        v -= 0x1_0000_0000
    return v


def _sext8(v: int) -> int:
    """Sign-extend 8-bit value to full Python int."""
    v = int(v) & MASK8
    if v >= 0x80:
        v -= 0x100
    return v


def _sext16(v: int) -> int:
    """Sign-extend 16-bit displacement to Python int (for memory format)."""
    v = int(v) & MASK16
    if v >= 0x8000:
        v -= 0x10000
    return v


def _sext21(v: int) -> int:
    """Sign-extend 21-bit branch displacement to Python int."""
    v = int(v) & 0x1F_FFFF
    if v >= 0x10_0000:
        v -= 0x20_0000
    return v


def _sext32(v: int) -> int:
    """Sign-extend 32-bit result to 64 bits and mask (for ADDL/SUBL/MULL).

    Alpha's longword arithmetic operates on the low 32 bits and then
    sign-extends the result to fill all 64 bits.  This ensures a 32-bit
    negative value (high bit set) appears negative in 64-bit context.

      ADDL r1=0x7FFFFFFF, 1, r2:
        32-bit result: 0x80000000
        After sext32 → u64: 0xFFFFFFFF80000000  (negative when signed)
    """
    v = int(v) & MASK32
    if v >= 0x8000_0000:
        v -= 0x1_0000_0000
    return _u64(v)


# ── Memory opcode set ─────────────────────────────────────────────────────────

_MEM_OPS: frozenset[int] = frozenset({
    0x28,   # LDL  — load longword, sign-extend to 64
    0x29,   # LDQ  — load quadword
    0x2A,   # LDL_L — load longword locked (treat as LDL)
    0x2B,   # LDQ_L — load quadword locked (treat as LDQ)
    0x0A,   # LDBU — load byte unsigned
    0x0C,   # LDWU — load word unsigned
    0x2C,   # STL  — store longword
    0x2D,   # STQ  — store quadword
    0x0E,   # STB  — store byte
    0x0D,   # STW  — store word
})

_BRANCH_OPS: frozenset[int] = frozenset({
    0x39,   # BEQ  — branch if equal (ra==0)
    0x3D,   # BNE  — branch if not equal
    0x3A,   # BLT  — branch if less than (signed)
    0x3B,   # BLE  — branch if less-or-equal (signed)
    0x3F,   # BGT  — branch if greater than (signed)
    0x3E,   # BGE  — branch if greater-or-equal (signed)
    0x38,   # BLBC — branch if low bit clear
    0x3C,   # BLBS — branch if low bit set
    0x30,   # BR   — branch always (unconditional)
    0x34,   # BSR  — branch and save return address
})


# ── AlphaSimulator ─────────────────────────────────────────────────────────────

class AlphaSimulator(Simulator[AlphaState]):
    """Behavioral simulator for the DEC Alpha AXP 21064 (1992).

    Implements Simulator[AlphaState] from the SIM00 protocol:
      reset()              — zero all state; PC=0, nPC=4
      load(data)           — reset then copy bytes to memory[0..len(data)-1]
      step()               — execute one instruction; return StepTrace
      execute(data, ...)   — load + run loop; return ExecutionResult
      get_state()          — return frozen AlphaState snapshot

    Internal state uses a flat list for registers and a bytearray for memory
    to allow O(1) writes during simulation.  Both are converted to tuples on
    get_state() to produce the immutable snapshot.
    """

    def __init__(self) -> None:
        self._regs:    list[int]   = [0] * NUM_REGS
        self._mem:     bytearray   = bytearray(MEM_SIZE)
        self._pc:      int         = 0x0000
        self._npc:     int         = 0x0004
        self._halted:  bool        = False

    # ── SIM00 Protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all CPU state: registers, memory, PC=0, nPC=4, halted=False."""
        self._regs    = [0] * NUM_REGS
        self._mem     = bytearray(MEM_SIZE)
        self._pc      = 0x0000
        self._npc     = 0x0004
        self._halted  = False

    def load(self, program: bytes) -> None:
        """Reset, then copy program bytes into memory starting at address 0.

        Raises ValueError if len(program) > MEM_SIZE (65536).
        """
        if len(program) > MEM_SIZE:
            raise ValueError(
                f"Program is {len(program)} bytes; exceeds memory size {MEM_SIZE}"
            )
        self.reset()
        self._mem[: len(program)] = program

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If the CPU is already halted, returns a HALT trace without advancing PC.
        """
        if self._halted:
            return StepTrace(
                pc_before=self._pc,
                pc_after=self._pc,
                mnemonic="HALT",
                description="HALT (already halted)",
            )
        pc_before = self._pc
        try:
            mnemonic = self._execute_one()
        except (ValueError, IndexError) as exc:
            self._halted = True
            err_msg = f"ERROR: {exc}"
            return StepTrace(
                pc_before=pc_before,
                pc_after=self._pc,
                mnemonic=err_msg,
                description=err_msg,
            )
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._pc,
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:04X}",
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> ExecutionResult:
        """Load program and run until HALT or max_steps exceeded.

        Returns an ExecutionResult with:
          ok          — True if execution ended normally (HALT reached)
          halted      — True if CPU is halted
          error       — None on success; error message string on failure
          steps       — total instructions executed
          traces      — list of StepTrace for every instruction
          final_state — frozen AlphaState after execution
        """
        self.load(program)
        traces: list[StepTrace] = []
        for _ in range(max_steps):
            trace = self.step()
            traces.append(trace)
            if self._halted:
                halted_by_error = trace.mnemonic.startswith("ERROR")
                return ExecutionResult(
                    halted=True,
                    error=(
                        trace.mnemonic[len("ERROR: "):]
                        if halted_by_error
                        else None
                    ),
                    steps=len(traces),
                    traces=traces,
                    final_state=self.get_state(),
                )
        return ExecutionResult(
            halted=False,
            error=f"max_steps ({max_steps}) exceeded",
            steps=max_steps,
            traces=traces,
            final_state=self.get_state(),
        )

    def get_state(self) -> AlphaState:
        """Return an immutable snapshot of the current CPU state.

        The returned AlphaState is a frozen dataclass; its regs and memory
        fields are tuples, so external code cannot mutate simulator state
        through the snapshot.
        """
        return AlphaState(
            pc=self._pc,
            npc=self._npc,
            regs=tuple(self._regs),
            memory=tuple(self._mem),
            halted=self._halted,
        )

    # ── Register access ───────────────────────────────────────────────────────

    def _get_reg(self, n: int) -> int:
        """Read register n.  r31 always returns 0."""
        if n == REG_ZERO:
            return 0
        return self._regs[n]

    def _set_reg(self, n: int, val: int) -> None:
        """Write register n.  Writes to r31 are silently discarded."""
        if n != REG_ZERO:
            self._regs[n] = _u64(val)

    # ── Little-endian memory helpers ─────────────────────────────────────────

    def _load_byte(self, addr: int) -> int:
        """Load 1 byte (no alignment requirement)."""
        return self._mem[addr & (MEM_SIZE - 1)]

    def _load_word(self, addr: int) -> int:
        """Load 2-byte little-endian word (2-byte alignment required)."""
        a = addr & (MEM_SIZE - 1)
        if addr & 1:
            raise ValueError(
                f"Unaligned word load at 0x{addr:04X} (requires 2-byte alignment)"
            )
        return self._mem[a] | (self._mem[a + 1] << 8)

    def _load_long(self, addr: int) -> int:
        """Load 4-byte little-endian longword (4-byte alignment required).

        Returns an UNSIGNED 32-bit value; callers sign-extend as needed.
        """
        a = addr & (MEM_SIZE - 1)
        if addr & 3:
            raise ValueError(
                f"Unaligned longword load at 0x{addr:04X} (requires 4-byte alignment)"
            )
        return (
            self._mem[a]
            | (self._mem[a + 1] << 8)
            | (self._mem[a + 2] << 16)
            | (self._mem[a + 3] << 24)
        )

    def _load_quad(self, addr: int) -> int:
        """Load 8-byte little-endian quadword (8-byte alignment required)."""
        a = addr & (MEM_SIZE - 1)
        if addr & 7:
            raise ValueError(
                f"Unaligned quadword load at 0x{addr:04X} (requires 8-byte alignment)"
            )
        result = 0
        for i in range(8):
            result |= self._mem[(a + i) & (MEM_SIZE - 1)] << (8 * i)
        return result

    def _store_byte(self, addr: int, val: int) -> None:
        self._mem[addr & (MEM_SIZE - 1)] = val & MASK8

    def _store_word(self, addr: int, val: int) -> None:
        a = addr & (MEM_SIZE - 1)
        if addr & 1:
            raise ValueError(
                f"Unaligned word store at 0x{addr:04X} (requires 2-byte alignment)"
            )
        self._mem[a]     = val & MASK8
        self._mem[a + 1] = (val >> 8) & MASK8

    def _store_long(self, addr: int, val: int) -> None:
        a = addr & (MEM_SIZE - 1)
        if addr & 3:
            raise ValueError(
                f"Unaligned longword store at 0x{addr:04X} (requires 4-byte alignment)"
            )
        for i in range(4):
            self._mem[(a + i) & (MEM_SIZE - 1)] = (val >> (8 * i)) & MASK8

    def _store_quad(self, addr: int, val: int) -> None:
        a = addr & (MEM_SIZE - 1)
        if addr & 7:
            raise ValueError(
                f"Unaligned quadword store at 0x{addr:04X} (requires 8-byte alignment)"
            )
        for i in range(8):
            self._mem[(a + i) & (MEM_SIZE - 1)] = (val >> (8 * i)) & MASK8

    # ── Little-endian instruction fetch ──────────────────────────────────────

    def _fetch32(self) -> int:
        """Read the 32-bit little-endian instruction at PC and advance PC.

        Alpha instructions are stored little-endian in memory:
          byte[0] = bits[7:0]   (least significant)
          byte[1] = bits[15:8]
          byte[2] = bits[23:16]
          byte[3] = bits[31:24] (most significant)

        After fetch:
          self._pc  = old self._npc
          self._npc = old self._npc + 4
        """
        a = self._pc & (MEM_SIZE - 1)
        iw = (
            self._mem[a]
            | (self._mem[a + 1] << 8)
            | (self._mem[a + 2] << 16)
            | (self._mem[a + 3] << 24)
        )
        self._pc  = self._npc & (MEM_SIZE - 1)
        self._npc = (self._npc + 4) & (MEM_SIZE - 1)
        return iw

    # ── Top-level instruction dispatch ────────────────────────────────────────

    def _execute_one(self) -> str:
        """Fetch and execute one instruction.  Returns the mnemonic string.

        Dispatch table:
          0x00 → PALcode (HALT and other privileged operations)
          0x10 → INTA  (integer arithmetic: ADD, SUB, CMP)
          0x11 → INTL  (integer logical: AND, OR, XOR, CMOV)
          0x12 → INTS  (integer shift and byte manipulation)
          0x13 → INTM  (integer multiply: MUL, UMULH)
          0x1A → Jump  (JMP, JSR, RET)
          0x28–0x2D, 0x0A, 0x0C–0x0E → Memory loads/stores
          0x30–0x3F → Branches (BEQ, BNE, BLT, BLE, BGT, BGE, BR, BSR, ...)
        """
        pc_of_instr = self._pc
        iw = self._fetch32()
        op = (iw >> 26) & 0x3F

        if op == 0x00:
            return self._exec_palcode(iw, pc_of_instr)
        if op == 0x10:
            return self._exec_inta(iw, pc_of_instr)
        if op == 0x11:
            return self._exec_intl(iw, pc_of_instr)
        if op == 0x12:
            return self._exec_ints(iw, pc_of_instr)
        if op == 0x13:
            return self._exec_intm(iw, pc_of_instr)
        if op == 0x1A:
            return self._exec_jump(iw, pc_of_instr)
        if op in _MEM_OPS:
            return self._exec_mem(iw, op, pc_of_instr)
        if op in _BRANCH_OPS:
            return self._exec_branch(iw, op, pc_of_instr)
        raise ValueError(
            f"Unknown opcode 0x{op:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── Operate instruction decode ────────────────────────────────────────────

    def _decode_operate(self, iw: int) -> tuple[int, int, int, int]:
        """Decode Operate-format instruction.

        Returns (ra_val, src_val, func, rc_reg) where:
          ra_val  — value of register Ra
          src_val — value of Rb (i_bit=0) or zero-extended 8-bit literal (i_bit=1)
          func    — 7-bit function code
          rc_reg  — destination register number (for _set_reg)

        Note: the 8-bit literal is ZERO-EXTENDED (unsigned 0–255), not sign-
        extended.  This contrasts with SPARC's 13-bit simm which is signed.

        Layout:
          i_bit=0: [op:6][Ra:5][Rb:5][0][0][func:7][Rc:5]
          i_bit=1: [op:6][Ra:5][lit8:8][1][func:7][Rc:5]
        """
        ra  = (iw >> 21) & 0x1F
        i_b = (iw >> 12) & 1
        func = (iw >> 5) & 0x7F
        rc  = iw & 0x1F
        if i_b:
            lit = (iw >> 13) & 0xFF   # 8-bit unsigned literal
            return self._get_reg(ra), lit, func, rc
        rb = (iw >> 16) & 0x1F
        return self._get_reg(ra), self._get_reg(rb), func, rc

    # ── PALcode (opcode 0x00) ─────────────────────────────────────────────────

    def _exec_palcode(self, iw: int, pc_of_instr: int) -> str:
        """Handle call_pal instructions.

        Only call_pal 0x0000 (HALT) is implemented.  All other PALcode
        values raise ValueError.

        The HALT instruction is the all-zeros word 0x00000000.  This means
        uninitialized memory (which is zeroed on reset) will halt the
        simulator if execution reaches it — a convenient safety net.
        """
        palcode = iw & 0x03FF_FFFF
        if palcode == 0x0000:
            self._halted = True
            return "HALT"
        raise ValueError(
            f"Unsupported PALcode 0x{palcode:07X} at PC=0x{pc_of_instr:04X}"
        )

    # ── INTA: Integer Arithmetic (opcode 0x10) ────────────────────────────────

    def _exec_inta(self, iw: int, pc_of_instr: int) -> str:
        """Integer arithmetic instructions: ADD, SUB, MUL, CMP.

        Longword (L) variants sign-extend their 32-bit result to 64 bits.
        Quadword (Q) variants are full 64-bit.
        Compare instructions write 0 or 1 (never modify condition flags).

        Overflow-trapping variants (V suffix: ADDLV, ADDQV, etc.) are treated
        as identical to their non-V counterparts — we simulate without traps.
        """
        a, src, func, rc = self._decode_operate(iw)

        # ── ADD ───────────────────────────────────────────────────────────────
        if func in (0x00, 0x40):   # ADDL, ADDLV — longword add, sign-extend
            self._set_reg(rc, _sext32(_u32(a) + _u32(src)))
            return "ADDL"
        if func in (0x20, 0x60):   # ADDQ, ADDQV — quadword add
            self._set_reg(rc, _u64(a + src))
            return "ADDQ"

        # ── SUB ───────────────────────────────────────────────────────────────
        if func in (0x09, 0x49):   # SUBL, SUBLV
            self._set_reg(rc, _sext32(_u32(a) - _u32(src)))
            return "SUBL"
        if func in (0x29, 0x69):   # SUBQ, SUBQV
            self._set_reg(rc, _u64(a - src))
            return "SUBQ"

        # ── MUL ───────────────────────────────────────────────────────────────
        if func in (0x18, 0x58):   # MULL, MULLV
            self._set_reg(rc, _sext32(_u32(a) * _u32(src)))
            return "MULL"
        if func in (0x38, 0x78):   # MULQ, MULQV — lower 64 bits
            self._set_reg(rc, _u64(a * src))
            return "MULQ"

        # ── Scaled add (S4ADDL, S8ADDL, S4SUBL, S8SUBL) ─────────────────────
        # Not in our primary instruction set but occasionally generated.
        # S4ADDL: rc = sext32(Ra*4 + src); S4ADDQ: rc = Ra*4 + src (64-bit)
        # S8ADDL, etc. Included for completeness.
        if func == 0x02:    # S4ADDL
            self._set_reg(rc, _sext32(_u32(a * 4) + _u32(src)))
            return "S4ADDL"
        if func == 0x22:    # S4ADDQ
            self._set_reg(rc, _u64(a * 4 + src))
            return "S4ADDQ"
        if func == 0x0B:    # S4SUBL
            self._set_reg(rc, _sext32(_u32(a * 4) - _u32(src)))
            return "S4SUBL"
        if func == 0x2B:    # S4SUBQ
            self._set_reg(rc, _u64(a * 4 - src))
            return "S4SUBQ"
        if func == 0x12:    # S8ADDL
            self._set_reg(rc, _sext32(_u32(a * 8) + _u32(src)))
            return "S8ADDL"
        if func == 0x32:    # S8ADDQ
            self._set_reg(rc, _u64(a * 8 + src))
            return "S8ADDQ"
        if func == 0x1B:    # S8SUBL
            self._set_reg(rc, _sext32(_u32(a * 8) - _u32(src)))
            return "S8SUBL"
        if func == 0x3B:    # S8SUBQ
            self._set_reg(rc, _u64(a * 8 - src))
            return "S8SUBQ"

        # ── Compare — result is 0 or 1 in Rc ─────────────────────────────────
        # Signed comparisons interpret both operands as signed 64-bit integers.
        # Unsigned comparisons use the raw unsigned 64-bit values.

        if func == 0x2D:    # CMPEQ — equal
            self._set_reg(rc, 1 if a == src else 0)
            return "CMPEQ"
        if func == 0x4D:    # CMPLT — signed less-than
            self._set_reg(rc, 1 if _s64(a) < _s64(src) else 0)
            return "CMPLT"
        if func == 0x6D:    # CMPLE — signed less-than-or-equal
            self._set_reg(rc, 1 if _s64(a) <= _s64(src) else 0)
            return "CMPLE"
        if func == 0x3D:    # CMPULT — unsigned less-than
            self._set_reg(rc, 1 if a < src else 0)
            return "CMPULT"
        if func == 0x7D:    # CMPULE — unsigned less-than-or-equal
            self._set_reg(rc, 1 if a <= src else 0)
            return "CMPULE"

        raise ValueError(
            f"Unknown INTA func 0x{func:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── INTL: Integer Logical (opcode 0x11) ──────────────────────────────────

    def _exec_intl(self, iw: int, pc_of_instr: int) -> str:
        """Integer logical and conditional-move instructions.

        Logical ops are 64-bit clean (no sign-extension needed).
        CMOV variants preserve Rc unchanged when the condition is false.

        BIS r31, imm8, Rd  is the standard immediate-load idiom (MOV):
          a = r31 = 0;  0 | imm8 = imm8  → Rd = imm8
        """
        a, src, func, rc = self._decode_operate(iw)
        cur_rc = self._get_reg(rc)   # needed for CMOV false-branch

        if func == 0x00:   # AND
            self._set_reg(rc, _u64(a & src))
            return "AND"
        if func == 0x08:   # BIC (AND-NOT)
            self._set_reg(rc, _u64(a & ~src))
            return "BIC"
        if func == 0x20:   # BIS (OR — mnemonic: Bit Set)
            self._set_reg(rc, _u64(a | src))
            return "BIS"
        if func == 0x28:   # ORNOT (OR with complement)
            self._set_reg(rc, _u64(a | ~src))
            return "ORNOT"
        if func == 0x40:   # XOR
            self._set_reg(rc, _u64(a ^ src))
            return "XOR"
        if func == 0x48:   # EQV (XNOR — XOR with complement)
            self._set_reg(rc, _u64(a ^ ~src))
            return "EQV"

        # ── Conditional moves ─────────────────────────────────────────────────
        # CMOVXX Ra, Rb_or_lit, Rc:
        #   if condition(Ra) is true:  Rc ← src (Rb or literal)
        #   if condition(Ra) is false: Rc unchanged
        # Condition tests Ra (the first operand), not Rc.

        if func == 0x14:   # CMOVLBS — condition: low bit of Ra set
            self._set_reg(rc, src if (a & 1) else cur_rc)
            return "CMOVLBS"
        if func == 0x16:   # CMOVLBC — condition: low bit of Ra clear
            self._set_reg(rc, src if not (a & 1) else cur_rc)
            return "CMOVLBC"
        if func == 0x24:   # CMOVEQ — condition: Ra == 0
            self._set_reg(rc, src if a == 0 else cur_rc)
            return "CMOVEQ"
        if func == 0x26:   # CMOVNE — condition: Ra != 0
            self._set_reg(rc, src if a != 0 else cur_rc)
            return "CMOVNE"
        if func == 0x44:   # CMOVLT — condition: signed Ra < 0
            self._set_reg(rc, src if _s64(a) < 0 else cur_rc)
            return "CMOVLT"
        if func == 0x46:   # CMOVGE — condition: signed Ra >= 0
            self._set_reg(rc, src if _s64(a) >= 0 else cur_rc)
            return "CMOVGE"
        if func == 0x64:   # CMOVLE — condition: signed Ra <= 0
            self._set_reg(rc, src if _s64(a) <= 0 else cur_rc)
            return "CMOVLE"
        if func == 0x66:   # CMOVGT — condition: signed Ra > 0
            self._set_reg(rc, src if _s64(a) > 0 else cur_rc)
            return "CMOVGT"

        # AMASK and IMPLVER are sometimes emitted by compilers.
        # AMASK: Rc = Ra & ~src  (architecture mask — same as BIC here)
        if func == 0x61:   # AMASK
            self._set_reg(rc, _u64(a & ~src))
            return "AMASK"
        if func == 0x6C:   # IMPLVER — implementation version
            self._set_reg(rc, 0)   # report "EV3" (oldest), simplest
            return "IMPLVER"

        raise ValueError(
            f"Unknown INTL func 0x{func:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── INTS: Integer Shift and Byte Manipulation (opcode 0x12) ──────────────

    def _exec_ints(self, iw: int, pc_of_instr: int) -> str:
        """Shift and byte-manipulation instructions.

        The Alpha byte manipulation instructions exist because the architecture
        supports only aligned memory accesses.  To read an unaligned N-byte
        value, code:
          1. Loads the aligned quadword containing the bytes
          2. Uses EXT/INS/MSK to extract, insert, or zero the desired bytes

        Byte offset: low 3 bits of src (src & 7).
        Shift amount: low 6 bits of src (src & 63).
        """
        a, src, func, rc = self._decode_operate(iw)
        shift  = src & 63        # shift amount (for SLL/SRL/SRA)
        boff   = (src & 7) * 8  # byte offset in bits (for EXT/INS/MSK)

        # ── Shifts ────────────────────────────────────────────────────────────

        if func == 0x39:   # SLL — logical left shift
            self._set_reg(rc, _u64(a << shift))
            return "SLL"
        if func == 0x34:   # SRL — logical right shift (zero-fill)
            self._set_reg(rc, (a & MASK64) >> shift)
            return "SRL"
        if func == 0x3C:   # SRA — arithmetic right shift (sign-fill)
            result = _s64(a) >> shift   # Python >> on signed int is arithmetic
            self._set_reg(rc, _u64(result))
            return "SRA"

        # ── Extract bytes (EXT*L family) — right-aligned result ──────────────
        # EXTBL: zero-extend byte at byte-position (boff//8) in Ra
        # The "shift-right and mask" formulation:
        #   rc = (Ra >> boff) & width_mask
        # This extracts width bytes starting at byte boff//8.

        if func == 0x06:   # EXTBL — extract byte
            self._set_reg(rc, (a >> boff) & MASK8)
            return "EXTBL"
        if func == 0x16:   # EXTWL — extract word (2 bytes)
            self._set_reg(rc, (a >> boff) & MASK16)
            return "EXTWL"
        if func == 0x26:   # EXTLL — extract longword (4 bytes)
            self._set_reg(rc, (a >> boff) & MASK32)
            return "EXTLL"
        if func == 0x36:   # EXTQL — extract quadword (all 8 bytes)
            self._set_reg(rc, (a >> boff) & MASK64)
            return "EXTQL"

        # ── Insert bytes (INS*L family) — left-aligned insertion ─────────────
        # INSBL: place Ra[7:0] at byte-position boff//8 in Rc.
        # The "shift-left and mask" formulation:
        #   rc = (Ra & width_mask) << boff   (capped to 64 bits)

        if func == 0x0B:   # INSBL — insert byte
            self._set_reg(rc, _u64((a & MASK8) << boff))
            return "INSBL"
        if func == 0x1B:   # INSWL — insert word
            self._set_reg(rc, _u64((a & MASK16) << boff))
            return "INSWL"
        if func == 0x2B:   # INSLL — insert longword
            self._set_reg(rc, _u64((a & MASK32) << boff))
            return "INSLL"
        if func == 0x3B:   # INSQL — insert quadword
            self._set_reg(rc, _u64(a << boff))
            return "INSQL"

        # ── Mask bytes (MSK*L family) — zero selected byte lane(s) ──────────
        # MSKBL: zero the byte at byte-position boff//8 in Ra.
        # mask = width_mask << boff  (bits to zero), then Ra & ~mask.

        if func == 0x02:   # MSKBL — mask byte
            mask = MASK8 << boff
            self._set_reg(rc, _u64(a & ~mask))
            return "MSKBL"
        if func == 0x12:   # MSKWL — mask word
            mask = MASK16 << boff
            self._set_reg(rc, _u64(a & ~mask))
            return "MSKWL"
        if func == 0x22:   # MSKLL — mask longword
            mask = MASK32 << boff
            self._set_reg(rc, _u64(a & ~mask))
            return "MSKLL"
        if func == 0x32:   # MSKQL — mask quadword
            mask = MASK64 << boff
            self._set_reg(rc, _u64(a & ~mask))
            return "MSKQL"

        # ── ZAP / ZAPNOT — conditional byte-lane zeroing ─────────────────────
        # ZAP (func=0x30):    zero byte i of Ra where bit i of src is SET
        # ZAPNOT (func=0x31): zero byte i of Ra where bit i of src is NOT set
        #                     (i.e. keep byte where src bit is set)
        #
        # Both operate on the low 8 bits of src as an 8-bit mask, one bit per
        # byte lane.  bit 0 corresponds to the least-significant byte (byte 0).

        if func == 0x30:   # ZAP
            result = 0
            for i in range(8):
                if not (src >> i) & 1:   # keep byte where mask bit is CLEAR
                    result |= a & (MASK8 << (i * 8))
            self._set_reg(rc, result)
            return "ZAP"
        if func == 0x31:   # ZAPNOT
            result = 0
            for i in range(8):
                if (src >> i) & 1:       # keep byte where mask bit is SET
                    result |= a & (MASK8 << (i * 8))
            self._set_reg(rc, result)
            return "ZAPNOT"

        # ── Sign-extend ───────────────────────────────────────────────────────

        if func == 0x00:   # SEXTB — sign-extend byte to 64 bits
            self._set_reg(rc, _u64(_sext8(a & MASK8)))
            return "SEXTB"
        if func == 0x01:   # SEXTW — sign-extend word to 64 bits
            self._set_reg(rc, _u64(_sext16(a & MASK16)))
            return "SEXTW"

        raise ValueError(
            f"Unknown INTS func 0x{func:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── INTM: Integer Multiply (opcode 0x13) ─────────────────────────────────

    def _exec_intm(self, iw: int, pc_of_instr: int) -> str:
        """Integer multiply instructions.

        MULQ gives the lower 64 bits of the 64×64 product.
        UMULH gives the upper 64 bits of the unsigned 64×64 product.
        MULL sign-extends the 32-bit product to 64 bits.

        The overflow-trapping variants (MULLV, MULQV) are treated identically
        to their non-V counterparts.
        """
        a, src, func, rc = self._decode_operate(iw)

        if func in (0x00, 0x40):   # MULL, MULLV
            self._set_reg(rc, _sext32(_u32(a) * _u32(src)))
            return "MULL"
        if func in (0x20, 0x60):   # MULQ, MULQV — lower 64 bits
            self._set_reg(rc, _u64(a * src))
            return "MULQ"
        if func == 0x30:           # UMULH — upper 64 bits of unsigned product
            full = (_u64(a) * _u64(src))
            self._set_reg(rc, (full >> 64) & MASK64)
            return "UMULH"

        raise ValueError(
            f"Unknown INTM func 0x{func:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── Memory loads and stores (various opcodes) ─────────────────────────────

    def _exec_mem(self, iw: int, op: int, pc_of_instr: int) -> str:
        """Memory load and store instructions.

        Memory format: [op:6][Ra:5][Rb:5][disp16:16]
          Ra = destination for loads, source for stores
          Rb = base register
          ea = Rb + sign_extend(disp16)   — effective address

        All addresses wrap at MEM_SIZE (64 KiB).
        Alpha memory is little-endian.
        """
        ra   = (iw >> 21) & 0x1F
        rb   = (iw >> 16) & 0x1F
        d16  = iw & 0xFFFF
        ea   = _u64(self._get_reg(rb) + _sext16(d16)) & (MEM_SIZE - 1)

        # ── Loads ─────────────────────────────────────────────────────────────
        if op in (0x28, 0x2A):   # LDL, LDL_L — load longword (sign-extend)
            val = _sext32(self._load_long(ea))
            self._set_reg(ra, _u64(val))
            return "LDL"
        if op in (0x29, 0x2B):   # LDQ, LDQ_L — load quadword
            self._set_reg(ra, self._load_quad(ea))
            return "LDQ"
        if op == 0x0A:           # LDBU — load byte unsigned (zero-extend)
            self._set_reg(ra, self._load_byte(ea))
            return "LDBU"
        if op == 0x0C:           # LDWU — load word unsigned (zero-extend)
            self._set_reg(ra, self._load_word(ea))
            return "LDWU"

        # ── Stores ────────────────────────────────────────────────────────────
        src = self._get_reg(ra)
        if op == 0x2C:           # STL — store longword (low 32 bits)
            self._store_long(ea, src & MASK32)
            return "STL"
        if op == 0x2D:           # STQ — store quadword
            self._store_quad(ea, src)
            return "STQ"
        if op == 0x0E:           # STB — store byte (low 8 bits)
            self._store_byte(ea, src & MASK8)
            return "STB"
        if op == 0x0D:           # STW — store word (low 16 bits)
            self._store_word(ea, src & MASK16)
            return "STW"

        raise ValueError(
            f"Unknown memory opcode 0x{op:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── Branch instructions ───────────────────────────────────────────────────

    def _exec_branch(self, iw: int, op: int, pc_of_instr: int) -> str:
        """Branch instructions.

        Branch format: [op:6][Ra:5][disp21:21]

        Target = (PC_of_branch + 4) + sign_extend(disp21) × 4

        Note the + 4: Alpha uses PC+4 as the branch base, not PC.  Since
        _fetch32 already advanced self._pc to self._npc (= PC_of_branch + 4),
        we compute: target = (pc_of_instr + 4) + sext(disp21) * 4.
        This is equivalent to: self._pc + sext(disp21) * 4 at call time.

        BSR additionally saves PC_of_branch + 4 into Ra (the return address).
        BR writes to Ra=r31 (which discards the value — an Alpha NOP idiom).
        """
        ra     = (iw >> 21) & 0x1F
        disp21 = iw & 0x1F_FFFF
        target = _u64(pc_of_instr + 4 + _sext21(disp21) * 4) & (MEM_SIZE - 1)
        val    = self._get_reg(ra)

        # Evaluate the branch condition.
        taken = False
        mnemonic = "BR"

        if op == 0x30:   # BR — always taken (unconditional)
            taken, mnemonic = True, "BR"
        elif op == 0x34:   # BSR — branch and save return address
            self._set_reg(ra, pc_of_instr + 4)   # save PC+4 first
            taken, mnemonic = True, "BSR"
        elif op == 0x39:   # BEQ — branch if Ra == 0
            taken, mnemonic = val == 0, "BEQ"
        elif op == 0x3D:   # BNE — branch if Ra != 0
            taken, mnemonic = val != 0, "BNE"
        elif op == 0x3A:   # BLT — branch if signed Ra < 0
            taken, mnemonic = _s64(val) < 0, "BLT"
        elif op == 0x3B:   # BLE — branch if signed Ra <= 0
            taken, mnemonic = _s64(val) <= 0, "BLE"
        elif op == 0x3F:   # BGT — branch if signed Ra > 0
            taken, mnemonic = _s64(val) > 0, "BGT"
        elif op == 0x3E:   # BGE — branch if signed Ra >= 0
            taken, mnemonic = _s64(val) >= 0, "BGE"
        elif op == 0x38:   # BLBC — branch if low bit clear
            taken, mnemonic = (val & 1) == 0, "BLBC"
        elif op == 0x3C:   # BLBS — branch if low bit set
            taken, mnemonic = (val & 1) == 1, "BLBS"

        if taken and op != 0x34:   # BSR handled target above
            self._pc  = target
            self._npc = (target + 4) & (MEM_SIZE - 1)

        if taken and op == 0x34:   # BSR target
            self._pc  = target
            self._npc = (target + 4) & (MEM_SIZE - 1)

        return mnemonic

    # ── Jump instructions (opcode 0x1A) ──────────────────────────────────────

    def _exec_jump(self, iw: int, pc_of_instr: int) -> str:
        """Jump instructions: JMP, JSR, RET, JSR_COROUTINE.

        Jump format: [0x1A:6][Ra:5][Rb:5][func:2][hint:14]

        All four variants jump to (Rb & ~3) — the hint field is advisory for
        branch prediction and is ignored here.

          JMP (func=00): Ra = PC+4 (return link, discarded if Ra=r31)
          JSR (func=01): Ra = PC+4 (return link)
          RET (func=10): Ra field unused (typically r31), just jumps to Rb
          JSR_COROUTINE (func=11): same as JMP

        Typical subroutine call/return idiom:
          BSR r26, subroutine    ; call  (r26 = PC+4)
          ...subroutine body...
          RET r31, (r26)         ; return to r26

        Note: BSR is used for near calls (21-bit offset), JSR for indirect
        calls (through a register).
        """
        ra   = (iw >> 21) & 0x1F
        rb   = (iw >> 16) & 0x1F
        func = (iw >> 14) & 0x3
        link = _u64(pc_of_instr + 4)   # return address = instruction PC + 4
        target = self._get_reg(rb) & ~3 & (MEM_SIZE - 1)

        self._pc  = target
        self._npc = (target + 4) & (MEM_SIZE - 1)

        if func == 0x00:   # JMP
            self._set_reg(ra, link)   # save link (discarded if ra=r31)
            return "JMP"
        if func == 0x01:   # JSR
            self._set_reg(ra, link)
            return "JSR"
        if func == 0x02:   # RET — ra field typically r31 (discard link)
            # The link value is NOT saved to ra for RET (ra is the hint reg,
            # not the link register).  Ra is written but since it's typically
            # r31, it's discarded anyway.
            self._set_reg(ra, link)
            return "RET"
        # func == 0x03: JSR_COROUTINE
        self._set_reg(ra, link)
        return "JSR_COROUTINE"
