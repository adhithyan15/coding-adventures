"""SPARC V8 (1987) behavioral simulator — Layer 07r.

SPARC (Scalable Processor ARChitecture) was designed by Sun Microsystems and
first shipped in 1987.  It is notable for two unusual design choices that set it
apart from all other processors in this simulator series:

  1. Register windows — instead of a flat register file, SPARC has a set of
     overlapping call frames in hardware.  SAVE (procedure entry) and RESTORE
     (procedure exit) rotate the active window, making argument passing free.

  2. Condition code register (PSR.N/Z/V/C) — unlike MIPS (07q), which uses
     dedicated compare instructions (SLT/SLTU) that write 0/1 to a GPR, SPARC
     has a traditional flags register.  This enables compact branch code but
     requires careful pipeline forwarding.

Architecture summary:
  • 32 visible registers at any time: 8 globals + 24 windowed (outs, locals, ins)
  • Physical register file: 8 globals + NWINDOWS(=3) × 16 = 56 registers total
  • NWINDOWS = 3 in this simulator (real chips use 7–32)
  • PSR condition codes: N (negative), Z (zero), V (overflow), C (carry)
  • Y register for 64-bit multiply and 64÷32 divide
  • Big-endian, 64 KB flat memory, fixed 32-bit instructions

Instruction formats (all 32 bits):
  Format 1: [op:2=01][disp30:30]                         — CALL
  Format 2: [op:2=00][rd:5][op2:3][imm22:22]             — SETHI, Bicc, NOP
  Format 3r: [op][rd][op3:6][rs1:5][0][asi:8][rs2:5]     — reg operand
  Format 3i: [op][rd][op3:6][rs1:5][1][simm13:13]        — immediate operand

HALT convention:
  ta 0 (trap always, software trap 0) = 0x91D02000 halts the simulator.
  This matches SPARC Linux convention (ta 1 = sys_exit; ta 0 = our sentinel).

Branch delay slots:
  Real SPARC CPUs execute the instruction in the delay slot before the branch
  takes effect.  This simulator does NOT model delay slots.  Branches take
  effect immediately.  Programs must not rely on delay-slot behaviour.
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .state import (
    HALT_WORD,
    MEM_SIZE,
    NUM_PHYS,
    NWINDOWS,
    SPARCState,
    virt_to_phys,
)

# ── Helpers ────────────────────────────────────────────────────────────────────

def _sext13(v: int) -> int:
    """Sign-extend a 13-bit value to a signed Python int."""
    v &= 0x1FFF
    return v - 0x2000 if v >= 0x1000 else v


def _sext22(v: int) -> int:
    """Sign-extend a 22-bit value to a signed Python int."""
    v &= 0x3FFFFF
    return v - 0x400000 if v >= 0x200000 else v


def _sext30(v: int) -> int:
    """Sign-extend a 30-bit value to a signed Python int."""
    v &= 0x3FFFFFFF
    return v - 0x40000000 if v >= 0x20000000 else v


def _u32(v: int) -> int:
    """Mask to 32 unsigned bits."""
    return v & 0xFFFF_FFFF


def _s32(v: int) -> int:
    """Interpret unsigned 32-bit value as signed Python int."""
    v &= 0xFFFF_FFFF
    return v - 0x1_0000_0000 if v >= 0x8000_0000 else v


# ── Simulator ──────────────────────────────────────────────────────────────────

class SPARCSimulator(Simulator[SPARCState]):
    """Behavioral simulator for the SPARC V8 microprocessor.

    Public API (SIM00 protocol):
        reset()            — return to power-on state
        load(program)      — reset and copy bytes to memory at 0x0000
        step()             — execute one instruction, return StepTrace
        execute(program)   — run until HALT or max_steps, return ExecutionResult
        get_state()        — return frozen SPARCState snapshot

    Internal state:
        _mem   : bytearray(MEM_SIZE)   — 64 KB flat big-endian memory
        _regs  : list[int]             — 56 physical unsigned 32-bit registers
        _cwp   : int                   — current window pointer (0–NWINDOWS−1)
        _psr_n : bool                  — negative flag
        _psr_z : bool                  — zero flag
        _psr_v : bool                  — overflow flag
        _psr_c : bool                  — carry flag
        _y     : int                   — Y register (multiply/divide)
        _pc    : int                   — program counter
        _npc   : int                   — next-PC (pc+4 normally)
        _halted: bool                  — True after ta 0
    """

    def __init__(self) -> None:
        self._mem:        bytearray = bytearray(MEM_SIZE)
        self._regs:       list[int] = [0] * NUM_PHYS
        self._cwp:        int = 0
        self._psr_n:      bool = False
        self._psr_z:      bool = False
        self._psr_v:      bool = False
        self._psr_c:      bool = False
        self._y:          int = 0
        self._pc:         int = 0
        self._npc:        int = 4
        self._halted:     bool = False
        self._save_depth: int = 0   # number of outstanding SAVE frames

    # ── Protocol: reset ───────────────────────────────────────────────────────

    def reset(self) -> None:
        """Return the CPU to power-on state.

        Power-on state:
          PC = 0x0000, nPC = 0x0004
          All 56 physical registers = 0
          CWP = 0, PSR N/Z/V/C = False, Y = 0
          Memory = zeroed
          halted = False
        """
        self._mem[:]      = bytearray(MEM_SIZE)
        self._regs        = [0] * NUM_PHYS
        self._cwp         = 0
        self._psr_n       = False
        self._psr_z       = False
        self._psr_v       = False
        self._psr_c       = False
        self._y           = 0
        self._pc          = 0
        self._npc         = 4
        self._halted      = False
        self._save_depth  = 0

    # ── Protocol: load ────────────────────────────────────────────────────────

    def load(self, program: bytes) -> None:
        """Reset and load program bytes into memory at address 0x0000.

        Args:
            program: raw big-endian machine code (must fit within 64 KB)

        Raises:
            ValueError: if len(program) > MEM_SIZE (64 KB)
        """
        if len(program) > MEM_SIZE:
            msg = f"Program too large: {len(program)} bytes > {MEM_SIZE}"
            raise ValueError(msg)
        self.reset()
        self._mem[:len(program)] = program

    # ── Protocol: get_state ───────────────────────────────────────────────────

    def get_state(self) -> SPARCState:
        """Return a frozen snapshot of the current CPU state."""
        return SPARCState(
            pc     = self._pc,
            npc    = self._npc,
            regs   = tuple(self._regs),
            cwp    = self._cwp,
            psr_n  = self._psr_n,
            psr_z  = self._psr_z,
            psr_v  = self._psr_v,
            psr_c  = self._psr_c,
            y      = self._y,
            memory = tuple(self._mem),
            halted = self._halted,
        )

    # ── Protocol: step ────────────────────────────────────────────────────────

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If the CPU is already halted, returns a no-op HALT trace without
        modifying any state.
        """
        pc_before = self._pc
        if self._halted:
            return StepTrace(
                pc_before   = pc_before,
                pc_after    = pc_before,
                mnemonic    = "HALT",
                description = "HALT (already halted)",
            )
        mnemonic = self._execute_one()
        return StepTrace(
            pc_before   = pc_before,
            pc_after    = self._pc,
            mnemonic    = mnemonic,
            description = f"{mnemonic} @ 0x{pc_before:04X}",
        )

    # ── Protocol: execute ─────────────────────────────────────────────────────

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult:
        """Load and run program until HALT or max_steps exceeded.

        Args:
            program:   raw big-endian machine code
            max_steps: guard against infinite loops (default 100,000)

        Returns:
            ExecutionResult(halted, steps, traces, final_state, error)
        """
        self.load(program)
        traces: list[StepTrace] = []
        error: str | None = None
        steps = 0
        while not self._halted and steps < max_steps:
            try:
                trace = self.step()
            except Exception as exc:  # noqa: BLE001
                error = str(exc)
                break
            traces.append(trace)
            steps += 1
        if not self._halted and error is None:
            error = f"max_steps ({max_steps}) exceeded"
        return ExecutionResult(
            halted      = self._halted,
            steps       = steps,
            traces      = traces,
            final_state = self.get_state(),
            error       = error,
        )

    # =========================================================================
    # Internal helpers
    # =========================================================================

    def _fetch32(self) -> int:
        """Fetch one 32-bit big-endian instruction word from memory at PC.

        Advances PC to nPC, and nPC to nPC+4 (delay-slot compatible, but
        since we have no delay slots nPC is always PC+4).
        """
        addr = self._pc & (MEM_SIZE - 1)
        iw = (self._mem[addr]     << 24 |
              self._mem[addr + 1] << 16 |
              self._mem[addr + 2] <<  8 |
              self._mem[addr + 3])
        # Advance PC pipeline: PC = nPC, nPC = nPC + 4
        self._pc  = self._npc & (MEM_SIZE - 1)
        self._npc = (self._npc + 4) & (MEM_SIZE - 1)
        return iw

    def _set_reg(self, virt: int, val: int) -> None:
        """Write to a virtual register in the current window.

        Writes to r0 (%g0) are silently discarded — %g0 is always zero.
        """
        if virt != 0:
            phys = virt_to_phys(virt, self._cwp)
            self._regs[phys] = val & 0xFFFF_FFFF

    def _get_reg(self, virt: int) -> int:
        """Read a virtual register in the current window."""
        if virt == 0:
            return 0
        return self._regs[virt_to_phys(virt, self._cwp)]

    def _operand2(self, iw: int, rs1: int) -> int:
        """Compute the second ALU operand.

        SPARC Format 3 instructions have a 1-bit 'i' field (bit 13):
          i=0: operand = rs2 (register)
          i=1: operand = sign_extend(simm13)

        Args:
            iw:  full instruction word
            rs1: first source register (not used here, kept for clarity)

        Returns:
            The second operand as an unsigned 32-bit int.
        """
        i = (iw >> 13) & 1
        if i:
            return _u32(_sext13(iw & 0x1FFF))
        rs2 = iw & 0x1F
        return self._get_reg(rs2)

    # ── Memory access ─────────────────────────────────────────────────────────

    def _check_align(self, addr: int, size: int) -> None:
        """Raise ValueError for misaligned memory access."""
        if addr & (size - 1):
            kind = "word" if size == 4 else "halfword"
            msg = f"Misaligned {kind} access at 0x{addr:04X}"
            raise ValueError(msg)

    def _ea(self, rs1: int, iw: int) -> int:
        """Compute effective address: (rs1 + operand2) & (MEM_SIZE - 1)."""
        return _u32(self._get_reg(rs1) + self._operand2(iw, rs1)) & (MEM_SIZE - 1)

    def _load_byte(self, addr: int) -> int:
        return self._mem[addr & (MEM_SIZE - 1)]

    def _load_half(self, addr: int) -> int:
        self._check_align(addr, 2)
        a = addr & (MEM_SIZE - 1)
        return (self._mem[a] << 8) | self._mem[a + 1]

    def _load_word(self, addr: int) -> int:
        self._check_align(addr, 4)
        a = addr & (MEM_SIZE - 1)
        return (self._mem[a]     << 24 |
                self._mem[a + 1] << 16 |
                self._mem[a + 2] <<  8 |
                self._mem[a + 3])

    def _store_byte(self, addr: int, val: int) -> None:
        self._mem[addr & (MEM_SIZE - 1)] = val & 0xFF

    def _store_half(self, addr: int, val: int) -> None:
        self._check_align(addr, 2)
        a = addr & (MEM_SIZE - 1)
        self._mem[a]     = (val >> 8) & 0xFF
        self._mem[a + 1] = val & 0xFF

    def _store_word(self, addr: int, val: int) -> None:
        self._check_align(addr, 4)
        a = addr & (MEM_SIZE - 1)
        self._mem[a]     = (val >> 24) & 0xFF
        self._mem[a + 1] = (val >> 16) & 0xFF
        self._mem[a + 2] = (val >>  8) & 0xFF
        self._mem[a + 3] = val & 0xFF

    # ── Condition code update ─────────────────────────────────────────────────

    def _update_cc_add(self, a: int, b: int, result32: int) -> None:
        """Update PSR condition codes for an ADD-family result.

        Args:
            a, b:       unsigned 32-bit operands (before addition)
            result32:   unsigned 32-bit result (already masked to 32 bits)

        N = result[31]
        Z = (result == 0)
        V = signed overflow: both operands had same sign, result sign differs
        C = carry out of bit 31 (unsigned overflow)
        """
        self._psr_n = bool(result32 >> 31)
        self._psr_z = (result32 == 0)
        # Signed overflow: (+)+(+)=(−) or (−)+(−)=(+)
        self._psr_v = bool(((~a & ~b & result32) | (a & b & ~result32)) >> 31 & 1)
        # Carry: full addition exceeds 2^32
        self._psr_c = ((_u32(a) + _u32(b)) > 0xFFFF_FFFF)

    def _update_cc_sub(self, a: int, b: int, result32: int) -> None:
        """Update PSR condition codes for a SUB-family result (a − b).

        N = result[31]
        Z = (result == 0)
        V = signed overflow: (+)−(−)=(−) or (−)−(+)=(+)
        C = borrow (unsigned a < unsigned b)
        """
        self._psr_n = bool(result32 >> 31)
        self._psr_z = (result32 == 0)
        # Signed overflow for subtraction
        self._psr_v = bool(((a & ~b & ~result32) | (~a & b & result32)) >> 31 & 1)
        # Borrow (carry is inverted for subtraction in SPARC)
        self._psr_c = (_u32(a) < _u32(b))

    def _update_cc_logic(self, result32: int) -> None:
        """Update PSR condition codes for logic (AND/OR/XOR/etc.) results.

        For logic operations V=0 and C=0.
        """
        self._psr_n = bool(result32 >> 31)
        self._psr_z = (result32 == 0)
        self._psr_v = False
        self._psr_c = False

    # ── Branch condition evaluation ───────────────────────────────────────────

    def _branch_taken(self, cond: int) -> bool:
        """Evaluate a Bicc condition code (4-bit field from instruction).

        Conditions (from SPARC V8 manual §A.7):
          8 = BA  (always)
          0 = BN  (never)
          9 = BNE (Z=0)
          1 = BE  (Z=1)
          A = BG  (Z=0 and N=V, signed greater)
          2 = BLE (Z=1 or N!=V, signed ≤)
          B = BGE (N=V, signed ≥)
          3 = BL  (N!=V, signed <)
          C = BGU (C=0 and Z=0, unsigned >)
          4 = BLEU(C=1 or Z=1, unsigned ≤)
          D = BCC (C=0, carry clear / unsigned ≥)
          5 = BCS (C=1, carry set / unsigned <)
          E = BPOS(N=0, positive)
          6 = BNEG(N=1, negative)
          F = BVC (V=0, no overflow)
          7 = BVS (V=1, overflow)
        """
        n, z, v, c = self._psr_n, self._psr_z, self._psr_v, self._psr_c
        match cond & 0xF:
            case 0x8: return True                          # BA
            case 0x0: return False                         # BN
            case 0x9: return not z                         # BNE
            case 0x1: return z                             # BE
            case 0xA: return not z and (n == v)            # BG
            case 0x2: return z or (n != v)                 # BLE
            case 0xB: return n == v                        # BGE
            case 0x3: return n != v                        # BL
            case 0xC: return not c and not z               # BGU
            case 0x4: return c or z                        # BLEU
            case 0xD: return not c                         # BCC
            case 0x5: return c                             # BCS
            case 0xE: return not n                         # BPOS
            case 0x6: return n                             # BNEG
            case 0xF: return not v                         # BVC
            case 0x7: return v                             # BVS
            case _:   return False

    # =========================================================================
    # Instruction execution
    # =========================================================================

    def _execute_one(self) -> str:  # noqa: C901
        """Decode and execute one SPARC instruction.  Returns mnemonic string."""
        # Save PC before fetch so error messages and CALL/JMPL link correctly
        pc_of_instr = self._pc

        iw = self._fetch32()

        # ── HALT check: ta 0 = 0x91D02000 ────────────────────────────────────
        # Any Ticc with condition "always" (cond=8) halts the simulator.
        # op=2, op3=0x3A (Ticc), rd[4:0] = condition field; cond 8 = "always"
        if iw == HALT_WORD:
            self._halted = True
            return "HALT"

        # Decode top-level op field (bits 31:30)
        op = (iw >> 30) & 0x3

        # ── Format 1: CALL ────────────────────────────────────────────────────
        if op == 1:
            return self._exec_call(iw, pc_of_instr)

        # ── Format 2: SETHI / Bicc / NOP ─────────────────────────────────────
        if op == 0:
            return self._exec_fmt2(iw, pc_of_instr)

        # ── Format 3: ALU (op=2) or Memory (op=3) ────────────────────────────
        rd   = (iw >> 25) & 0x1F
        op3  = (iw >> 19) & 0x3F
        rs1  = (iw >> 14) & 0x1F

        if op == 2:
            return self._exec_alu(iw, rd, op3, rs1, pc_of_instr)
        # op == 3
        return self._exec_mem(iw, rd, op3, rs1)

    # ── Format 1: CALL ────────────────────────────────────────────────────────

    def _exec_call(self, iw: int, pc_of_instr: int) -> str:
        """CALL disp30 — %o7 = PC; PC = PC + sign_extend(disp30)*4."""
        disp30 = iw & 0x3FFF_FFFF
        target = _u32(pc_of_instr + _sext30(disp30) * 4) & (MEM_SIZE - 1)
        # %o7 (virtual register 15) = address of this CALL instruction
        self._set_reg(15, pc_of_instr)
        # Override the PC pipeline updated by _fetch32
        self._pc  = target
        self._npc = (target + 4) & (MEM_SIZE - 1)
        return "CALL"

    # ── Format 2: SETHI / Bicc / NOP ─────────────────────────────────────────

    def _exec_fmt2(self, iw: int, pc_of_instr: int) -> str:
        """Handle Format 2 instructions: SETHI, Bicc, NOP."""
        rd   = (iw >> 25) & 0x1F
        op2  = (iw >> 22) & 0x7
        imm22 = iw & 0x3FFFFF

        # NOP: SETHI 0, %g0 — canonical no-op encoding
        if iw == 0x0100_0000:
            return "NOP"

        # op2=4: SETHI rd, imm22  — rd = imm22 << 10
        if op2 == 0x4:
            self._set_reg(rd, (imm22 << 10) & 0xFFFF_FFFF)
            return "SETHI"

        # op2=2: Bicc — branch on integer condition codes
        if op2 == 0x2:
            cond   = (iw >> 25) & 0xF
            # a-bit (annul) = bit 29, ignored in this simulator
            disp22 = iw & 0x3FFFFF
            if self._branch_taken(cond):
                target = _u32(pc_of_instr + _sext22(disp22) * 4) & (MEM_SIZE - 1)
                self._pc  = target
                self._npc = (target + 4) & (MEM_SIZE - 1)
            return _BICC_NAMES.get(cond, f"B{cond:X}")

        raise ValueError(f"Unknown Format-2 op2=0x{op2:X} at PC=0x{pc_of_instr:04X}")

    # ── Format 3: ALU ─────────────────────────────────────────────────────────

    def _exec_alu(self, iw: int, rd: int, op3: int, rs1: int, pc_of_instr: int) -> str:  # noqa: C901
        """Handle Format 3 ALU instructions (op=2), dispatched by op3."""
        a   = self._get_reg(rs1)
        src = self._operand2(iw, rs1)   # rs2 or sext(simm13)

        # ── Shifts (op3 0x25/0x26/0x27) ──────────────────────────────────────
        if op3 == 0x25:
            self._set_reg(rd, _u32(a << (src & 31)))
            return "SLL"
        if op3 == 0x26:
            self._set_reg(rd, a >> (src & 31))
            return "SRL"
        if op3 == 0x27:
            self._set_reg(rd, _u32(_s32(a) >> (src & 31)))
            return "SRA"

        # ── ADD family ────────────────────────────────────────────────────────
        if op3 in (0x00, 0x10):          # ADD / ADDcc
            result = _u32(a + src)
            if op3 == 0x10:
                self._update_cc_add(a, src, result)
            self._set_reg(rd, result)
            return "ADDcc" if op3 == 0x10 else "ADD"

        if op3 in (0x08, 0x18):          # ADDX / ADDXcc (add with carry)
            c_in = int(self._psr_c)
            result = _u32(a + src + c_in)
            if op3 == 0x18:
                # Effective carry = carry from (a + src) or (a + src + c_in) overflow
                self._update_cc_add(a, _u32(src + c_in), result)
            self._set_reg(rd, result)
            return "ADDXcc" if op3 == 0x18 else "ADDX"

        # ── SUB family ────────────────────────────────────────────────────────
        if op3 in (0x04, 0x14):          # SUB / SUBcc
            result = _u32(a - src)
            if op3 == 0x14:
                self._update_cc_sub(a, src, result)
            self._set_reg(rd, result)
            return "SUBcc" if op3 == 0x14 else "SUB"

        if op3 in (0x0C, 0x1C):          # SUBX / SUBXcc (subtract with borrow)
            c_in = int(self._psr_c)
            result = _u32(a - src - c_in)
            if op3 == 0x1C:
                self._update_cc_sub(a, _u32(src + c_in), result)
            self._set_reg(rd, result)
            return "SUBXcc" if op3 == 0x1C else "SUBX"

        # ── Logic family ──────────────────────────────────────────────────────
        if op3 in (0x01, 0x11):          # AND / ANDcc
            result = _u32(a & src)
            if op3 == 0x11: self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "ANDcc" if op3 == 0x11 else "AND"

        if op3 in (0x05, 0x15):          # ANDN / ANDNcc
            result = _u32(a & ~src)
            if op3 == 0x15: self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "ANDNcc" if op3 == 0x15 else "ANDN"

        if op3 in (0x02, 0x12):          # OR / ORcc
            result = _u32(a | src)
            if op3 == 0x12: self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "ORcc" if op3 == 0x12 else "OR"

        if op3 in (0x06, 0x16):          # ORN / ORNcc
            result = _u32(a | ~src)
            if op3 == 0x16: self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "ORNcc" if op3 == 0x16 else "ORN"

        if op3 in (0x03, 0x13):          # XOR / XORcc
            result = _u32(a ^ src)
            if op3 == 0x13: self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "XORcc" if op3 == 0x13 else "XOR"

        if op3 in (0x07, 0x17):          # XNOR / XNORcc
            result = _u32(~(a ^ src))
            if op3 == 0x17: self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "XNORcc" if op3 == 0x17 else "XNOR"

        # ── Multiply ──────────────────────────────────────────────────────────
        if op3 in (0x0A, 0x5A):          # UMUL / UMULcc
            product = _u32(a) * _u32(src)
            self._y  = (product >> 32) & 0xFFFF_FFFF
            result   = product & 0xFFFF_FFFF
            if op3 == 0x5A:
                self._update_cc_logic(result)   # V=C=0 for UMUL per spec
            self._set_reg(rd, result)
            return "UMULcc" if op3 == 0x5A else "UMUL"

        if op3 in (0x0B, 0x5B):          # SMUL / SMULcc
            product = _s32(a) * _s32(src)
            product64 = product & 0xFFFF_FFFF_FFFF_FFFF
            self._y  = (product64 >> 32) & 0xFFFF_FFFF
            result   = product64 & 0xFFFF_FFFF
            if op3 == 0x5B:
                self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "SMULcc" if op3 == 0x5B else "SMUL"

        # ── Divide (64÷32 → 32) ───────────────────────────────────────────────
        if op3 in (0x0E, 0x5E):          # UDIV / UDIVcc
            if src == 0:
                raise ValueError("UDIV by zero")
            dividend = ((_u32(self._y) << 32) | _u32(a))
            q = dividend // _u32(src)
            # Saturate: if quotient > 0xFFFFFFFF, result = 0xFFFFFFFF
            if q > 0xFFFF_FFFF:
                q = 0xFFFF_FFFF
            result = _u32(q)
            if op3 == 0x5E:
                self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "UDIVcc" if op3 == 0x5E else "UDIV"

        if op3 in (0x0F, 0x5F):          # SDIV / SDIVcc
            if src == 0:
                raise ValueError("SDIV by zero")
            dividend = (_s32(self._y) << 32) | _u32(a)
            divisor  = _s32(src)
            q = int(dividend / divisor)    # truncate toward zero
            # Saturate to signed 32-bit range
            if q > 0x7FFF_FFFF:
                q = 0x7FFF_FFFF
            elif q < -0x8000_0000:
                q = -0x8000_0000
            result = _u32(q)
            if op3 == 0x5F:
                self._update_cc_logic(result)
            self._set_reg(rd, result)
            return "SDIVcc" if op3 == 0x5F else "SDIV"

        # ── MULScc (multiply step) ─────────────────────────────────────────────
        if op3 == 0x24:
            # One step of restoring multiply: shift Y:rd right 1, conditionally add
            # N xor V from PSR determines whether to add rs1
            y_lsb = self._y & 1
            add   = _s32(a) if (self._psr_n != self._psr_v) else 0
            shifted = (_u32(src) >> 1) | (y_lsb << 31)
            result  = _u32(shifted + add)
            self._update_cc_add(_u32(shifted), _u32(add) if add >= 0 else _u32(add), result)
            self._y = (_u32(self._get_reg(rd)) >> 1) | ((result & 1) << 31)
            self._set_reg(rd, result)
            return "MULScc"

        # ── Y register ────────────────────────────────────────────────────────
        if op3 == 0x30:                   # WRY: Y = rs1 ^ rs2_or_simm13
            self._y = _u32(a ^ src)
            return "WRY"

        if op3 == 0x28:                   # RDY: rd = Y
            self._set_reg(rd, self._y)
            return "RDY"

        # ── JMPL ──────────────────────────────────────────────────────────────
        if op3 == 0x38:
            # rd = PC of this instruction; PC = rs1 + rs2_or_simm13
            target = _u32(a + src) & (MEM_SIZE - 1)
            self._set_reg(rd, pc_of_instr)
            self._pc  = target
            self._npc = (target + 4) & (MEM_SIZE - 1)
            return "JMPL"

        # ── SAVE / RESTORE (register window rotation) ─────────────────────────
        if op3 == 0x3C:                   # SAVE
            # Compute result in the *current* window before rotating.
            result = _u32(a + src)
            # Window overflow: with NWINDOWS windows, only NWINDOWS-1 nested
            # SAVEs are legal.  The NWINDOWS-th SAVE would wrap around and
            # clobber the outermost live frame.  Hardware uses the WIM register;
            # we track depth directly.
            if self._save_depth >= NWINDOWS - 1:
                raise ValueError(f"Register window overflow at PC=0x{pc_of_instr:04X}")
            new_cwp = (self._cwp - 1) % NWINDOWS
            self._cwp = new_cwp
            self._save_depth += 1
            # Write result into the new window's rd
            self._set_reg(rd, result)
            return "SAVE"

        if op3 == 0x3D:                   # RESTORE
            result = _u32(a + src)
            new_cwp = (self._cwp + 1) % NWINDOWS
            self._cwp = new_cwp
            self._save_depth = max(0, self._save_depth - 1)
            self._set_reg(rd, result)
            return "RESTORE"

        # ── Ticc: trap on integer condition ───────────────────────────────────
        if op3 == 0x3A:
            cond = (iw >> 25) & 0xF
            if cond == 0x8:              # TA — trap always = HALT
                self._halted = True
                return "HALT"
            raise ValueError(
                f"Ticc (trap) with cond=0x{cond:X} at PC=0x{pc_of_instr:04X}"
            )

        raise ValueError(
            f"Unknown ALU op3=0x{op3:02X} at PC=0x{pc_of_instr:04X}"
        )

    # ── Format 3: Memory ──────────────────────────────────────────────────────

    def _exec_mem(self, iw: int, rd: int, op3: int, rs1: int) -> str:
        """Handle Format 3 memory instructions (op=3), dispatched by op3."""
        ea = self._ea(rs1, iw)

        if op3 == 0x00:                  # LD  rd, [ea]
            self._set_reg(rd, self._load_word(ea))
            return "LD"

        if op3 == 0x04:                  # ST  rd, [ea]
            self._store_word(ea, self._get_reg(rd))
            return "ST"

        if op3 == 0x01:                  # LDUB: load unsigned byte
            self._set_reg(rd, self._load_byte(ea))
            return "LDUB"

        if op3 == 0x09:                  # LDSB: load signed byte
            b = self._load_byte(ea)
            self._set_reg(rd, _u32(b - 0x100 if b >= 0x80 else b))
            return "LDSB"

        if op3 == 0x02:                  # LDUH: load unsigned halfword
            self._set_reg(rd, self._load_half(ea))
            return "LDUH"

        if op3 == 0x0A:                  # LDSH: load signed halfword
            h = self._load_half(ea)
            self._set_reg(rd, _u32(h - 0x10000 if h >= 0x8000 else h))
            return "LDSH"

        if op3 == 0x05:                  # STB: store byte
            self._store_byte(ea, self._get_reg(rd))
            return "STB"

        if op3 == 0x06:                  # STH: store halfword
            self._store_half(ea, self._get_reg(rd))
            return "STH"

        raise ValueError(
            f"Unknown memory op3=0x{op3:02X} at PC=0x{(self._pc - 4) & 0xFFFF:04X}"
        )


# ── Branch mnemonic table ──────────────────────────────────────────────────────

_BICC_NAMES: dict[int, str] = {
    0x8: "BA",   0x0: "BN",
    0x9: "BNE",  0x1: "BE",
    0xA: "BG",   0x2: "BLE",
    0xB: "BGE",  0x3: "BL",
    0xC: "BGU",  0x4: "BLEU",
    0xD: "BCC",  0x5: "BCS",
    0xE: "BPOS", 0x6: "BNEG",
    0xF: "BVC",  0x7: "BVS",
}
