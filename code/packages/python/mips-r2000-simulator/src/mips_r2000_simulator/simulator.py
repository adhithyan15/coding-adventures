"""MIPS R2000 (1985) behavioral simulator — Layer 07q.

The MIPS R2000 is the first commercially successful RISC processor, designed by
John Hennessy's team at Stanford and released by MIPS Computer Systems in 1985.
It pioneered the "clean" 32-bit RISC philosophy that influenced virtually every
subsequent processor architecture:

  • 32 general-purpose 32-bit registers (R0 is hardwired zero)
  • Fixed-width 32-bit instructions in three formats: R, I, J
  • Load-store architecture — only LW/SW family touch memory
  • Big-endian byte order (MIPS R2000 default)
  • No condition codes — comparisons write to a GPR (SLT/SLTU)
  • HI:LO registers for 64-bit multiply results and divide quotient/remainder

Historical context:
  Used in SGI IRIS workstations (1986), DEC DECstation (1989), Sony PlayStation 1,
  PlayStation 2, Nintendo 64, and countless embedded controllers.  It is the
  canonical example in Patterson & Hennessy's *Computer Organization and Design*.

=============================================================================
Implementation notes
=============================================================================

Branch delay slots
──────────────────
Real MIPS CPUs execute the instruction in the "delay slot" (immediately after
a branch/jump) before the branch takes effect — a side-effect of the 5-stage
pipeline's instruction fetch.  This simulator does NOT model delay slots.
Branches and jumps take effect immediately.  Programs that rely on delay slot
behavior will not work correctly here.

Signed overflow
───────────────
ADD, ADDI, and SUB raise ValueError on 32-bit signed overflow, matching the
hardware exception behavior.  Use ADDU, ADDIU, SUBU for wrapping arithmetic.

Memory
──────
64 KB flat byte array (indices 0x0000–0xFFFF), big-endian.  PC and effective
addresses are masked to 16 bits.  Misaligned LW/SW raise ValueError.

HALT
────
SYSCALL (opcode 0, funct 0x0C) halts the simulator.  On real MIPS, SYSCALL
traps to the OS kernel; here it is our clean "program done" sentinel, matching
MIPS Linux convention where $v0=4001 exits the process.  BREAK (funct 0x0D)
raises ValueError (software breakpoint / assertion failure).
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .state import (
    HALT_OPCODE_WORD,
    MEM_SIZE,
    NUM_REGS,
    REG_RA,
    MIPSState,
)

# ── Helpers ────────────────────────────────────────────────────────────────────

def _sext16(v: int) -> int:
    """Sign-extend a 16-bit value to a signed Python int."""
    v &= 0xFFFF
    return v - 0x10000 if v >= 0x8000 else v


def _as_signed32(v: int) -> int:
    """Interpret an unsigned 32-bit value as a signed Python int."""
    v &= 0xFFFF_FFFF
    return v - 0x1_0000_0000 if v >= 0x8000_0000 else v


def _as_unsigned32(v: int) -> int:
    """Mask to 32 unsigned bits."""
    return v & 0xFFFF_FFFF


# ── Simulator ──────────────────────────────────────────────────────────────────

class MIPSSimulator(Simulator[MIPSState]):
    """Behavioral simulator for the MIPS R2000 microprocessor.

    Public API (SIM00 protocol):
        reset()            — return to power-on state
        load(program)      — reset and copy bytes to memory at 0x0000
        step()             — execute one instruction, return StepTrace
        execute(program)   — run until HALT or max_steps, return ExecutionResult
        get_state()        — return frozen MIPSState snapshot

    Internal state:
        _mem   : bytearray(MEM_SIZE)   — 64 KB flat big-endian memory
        _regs  : list[int]             — 32 unsigned 32-bit registers
        _hi    : int                   — HI special register
        _lo    : int                   — LO special register
        _pc    : int                   — program counter (16-bit modulo MEM_SIZE)
        _halted: bool                  — True after SYSCALL
    """

    def __init__(self) -> None:
        self._mem:    bytearray = bytearray(MEM_SIZE)
        self._regs:   list[int] = [0] * NUM_REGS
        self._hi:     int = 0
        self._lo:     int = 0
        self._pc:     int = 0
        self._halted: bool = False
        # reset() is called in __init__ to ensure invariants from the start;
        # the bytearray is already zeroed, but reset() is the canonical path.

    # ── Protocol: reset ───────────────────────────────────────────────────────

    def reset(self) -> None:
        """Return the CPU to power-on state.

        Power-on state:
          PC    = 0x0000
          All GPRs = 0 (including R0, which is always 0 anyway)
          HI, LO   = 0
          Memory   = zeroed
          halted   = False
        """
        self._mem[:] = bytearray(MEM_SIZE)
        self._regs   = [0] * NUM_REGS
        self._hi     = 0
        self._lo     = 0
        self._pc     = 0
        self._halted = False

    # ── Protocol: load ────────────────────────────────────────────────────────

    def load(self, program: bytes) -> None:
        """Reset and load program bytes into memory at address 0x0000.

        The program must be a multiple of 4 bytes (one word per instruction)
        and must fit within the 64 KB address space.

        Args:
            program: raw big-endian machine code

        Raises:
            ValueError: if len(program) > MEM_SIZE (64 KB)
        """
        if len(program) > MEM_SIZE:
            msg = f"Program too large: {len(program)} bytes > {MEM_SIZE}"
            raise ValueError(msg)
        self.reset()
        self._mem[:len(program)] = program

    # ── Protocol: get_state ───────────────────────────────────────────────────

    def get_state(self) -> MIPSState:
        """Return a frozen snapshot of the current CPU state."""
        return MIPSState(
            pc     = self._pc,
            regs   = tuple(self._regs),
            hi     = self._hi,
            lo     = self._lo,
            memory = tuple(self._mem),
            halted = self._halted,
        )

    # ── Protocol: step ────────────────────────────────────────────────────────

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If the CPU is halted, returns a no-op HALT trace.
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
        """Fetch one 32-bit big-endian instruction from memory at PC, advance PC."""
        addr = self._pc & (MEM_SIZE - 1)
        # Bounds check: instruction fetch wraps within 64 KB
        iw = (self._mem[addr]     << 24 |
              self._mem[addr + 1] << 16 |
              self._mem[addr + 2] <<  8 |
              self._mem[addr + 3])
        self._pc = (self._pc + 4) & (MEM_SIZE - 1)
        return iw

    def _set_reg(self, rd: int, val: int) -> None:
        """Write to a GPR.  Writes to R0 are silently discarded."""
        if rd != 0:
            self._regs[rd] = val & 0xFFFF_FFFF

    # ── Memory access ─────────────────────────────────────────────────────────

    def _check_align(self, addr: int, size: int) -> None:
        """Raise ValueError for misaligned memory access."""
        if addr & (size - 1):
            msg = (f"Misaligned {'word' if size == 4 else 'halfword'} access "
                   f"at 0x{addr:04X} (addr & {size - 1} != 0)")
            raise ValueError(msg)

    def _mem_addr(self, base: int, offset: int) -> int:
        """Compute effective address: (base + sext(offset)) & (MEM_SIZE - 1)."""
        return (_as_unsigned32(_as_signed32(base) + offset)) & (MEM_SIZE - 1)

    def _load_byte(self, addr: int) -> int:
        """Load one byte from memory."""
        return self._mem[addr & (MEM_SIZE - 1)]

    def _load_half(self, addr: int) -> int:
        """Load big-endian halfword from memory (2-byte aligned)."""
        self._check_align(addr, 2)
        a = addr & (MEM_SIZE - 1)
        return (self._mem[a] << 8) | self._mem[a + 1]

    def _load_word(self, addr: int) -> int:
        """Load big-endian word from memory (4-byte aligned)."""
        self._check_align(addr, 4)
        a = addr & (MEM_SIZE - 1)
        return (self._mem[a]     << 24 |
                self._mem[a + 1] << 16 |
                self._mem[a + 2] <<  8 |
                self._mem[a + 3])

    def _store_byte(self, addr: int, val: int) -> None:
        """Store one byte to memory."""
        self._mem[addr & (MEM_SIZE - 1)] = val & 0xFF

    def _store_half(self, addr: int, val: int) -> None:
        """Store big-endian halfword to memory (2-byte aligned)."""
        self._check_align(addr, 2)
        a = addr & (MEM_SIZE - 1)
        self._mem[a]     = (val >> 8) & 0xFF
        self._mem[a + 1] = val & 0xFF

    def _store_word(self, addr: int, val: int) -> None:
        """Store big-endian word to memory (4-byte aligned)."""
        self._check_align(addr, 4)
        a = addr & (MEM_SIZE - 1)
        self._mem[a]     = (val >> 24) & 0xFF
        self._mem[a + 1] = (val >> 16) & 0xFF
        self._mem[a + 2] = (val >>  8) & 0xFF
        self._mem[a + 3] = val & 0xFF

    # =========================================================================
    # Instruction execution
    # =========================================================================

    def _execute_one(self) -> str:  # noqa: C901 (complex dispatch table)
        """Decode and execute one MIPS instruction.  Returns mnemonic string."""
        iw = self._fetch32()

        # Decode instruction fields
        op     = (iw >> 26) & 0x3F   # opcode (bits 31:26)
        rs     = (iw >> 21) & 0x1F   # source register 1 (bits 25:21)
        rt     = (iw >> 16) & 0x1F   # source register 2 / branch condition (bits 20:16)
        rd     = (iw >> 11) & 0x1F   # destination register (bits 15:11)
        shamt  = (iw >>  6) & 0x1F   # shift amount (bits 10:6)
        funct  = iw & 0x3F            # function code (bits 5:0)
        imm16  = iw & 0xFFFF          # 16-bit immediate (bits 15:0)
        simm   = _sext16(imm16)       # sign-extended immediate
        target = iw & 0x03FF_FFFF     # 26-bit jump target (bits 25:0)

        # ── HALT: SYSCALL ─────────────────────────────────────────────────────
        # SYSCALL has op=0 and funct=0x0C.  Any SYSCALL halts the simulator.
        if iw == HALT_OPCODE_WORD or (op == 0 and funct == 0x0C):
            self._halted = True
            return "HALT"

        # ── NOP canonical encoding ────────────────────────────────────────────
        # 0x00000000 = SLL $zero, $zero, 0 — the MIPS canonical NOP.  All MIPS
        # assemblers and disassemblers display this as "NOP" rather than "SLL".
        if iw == 0x0000_0000:
            return "NOP"

        # ── R-type (op == 0) ─────────────────────────────────────────────────
        if op == 0:
            return self._r_type(rs, rt, rd, shamt, funct)

        # ── REGIMM (op == 1) ─────────────────────────────────────────────────
        if op == 0x01:
            return self._regimm(rs, rt, simm)

        # ── J-type ───────────────────────────────────────────────────────────

        # J addr  — unconditional jump
        if op == 0x02:
            # Target = (PC+4)[31:28] | (target26 << 2)
            # Since PC was already advanced by _fetch32, self._pc == PC+4
            self._pc = (self._pc & 0xF000) | ((target << 2) & 0xFFFF)
            return "J"

        # JAL addr  — jump and link ($ra = PC+4 before jump)
        if op == 0x03:
            ret_addr = self._pc   # _fetch32 already advanced PC to PC+4
            self._pc = (self._pc & 0xF000) | ((target << 2) & 0xFFFF)
            self._set_reg(REG_RA, ret_addr)
            return "JAL"

        # ── I-type branches ───────────────────────────────────────────────────

        # BEQ rs, rt, offset  — branch if rs == rt
        if op == 0x04:
            if self._regs[rs] == self._regs[rt]:
                self._pc = _as_unsigned32(self._pc + (simm << 2)) & (MEM_SIZE - 1)
            return "BEQ"

        # BNE rs, rt, offset  — branch if rs != rt
        if op == 0x05:
            if self._regs[rs] != self._regs[rt]:
                self._pc = _as_unsigned32(self._pc + (simm << 2)) & (MEM_SIZE - 1)
            return "BNE"

        # BLEZ rs, offset  — branch if signed(rs) <= 0
        if op == 0x06:
            if _as_signed32(self._regs[rs]) <= 0:
                self._pc = _as_unsigned32(self._pc + (simm << 2)) & (MEM_SIZE - 1)
            return "BLEZ"

        # BGTZ rs, offset  — branch if signed(rs) > 0
        if op == 0x07:
            if _as_signed32(self._regs[rs]) > 0:
                self._pc = _as_unsigned32(self._pc + (simm << 2)) & (MEM_SIZE - 1)
            return "BGTZ"

        # ── I-type arithmetic / logic ─────────────────────────────────────────

        # ADDI rt, rs, imm  — rt = rs + sext(imm); raises on signed overflow
        if op == 0x08:
            s = _as_signed32(self._regs[rs]) + simm
            if s < -2**31 or s > 2**31 - 1:
                msg = f"ADDI signed overflow: {_as_signed32(self._regs[rs])} + {simm}"
                raise ValueError(msg)
            self._set_reg(rt, _as_unsigned32(s))
            return "ADDI"

        # ADDIU rt, rs, imm  — rt = rs + sext(imm); wraps, no overflow check
        if op == 0x09:
            self._set_reg(rt, _as_unsigned32(self._regs[rs] + simm))
            return "ADDIU"

        # SLTI rt, rs, imm  — rt = (signed(rs) < signed(sext(imm))) ? 1 : 0
        if op == 0x0A:
            self._set_reg(rt, 1 if _as_signed32(self._regs[rs]) < simm else 0)
            return "SLTI"

        # SLTIU rt, rs, imm  — unsigned comparison; imm is still sign-extended
        # but comparison is unsigned.  This is the MIPS spec behaviour.
        if op == 0x0B:
            self._set_reg(rt, 1 if self._regs[rs] < _as_unsigned32(simm) else 0)
            return "SLTIU"

        # ANDI rt, rs, imm  — zero-extend imm16, bitwise AND
        if op == 0x0C:
            self._set_reg(rt, self._regs[rs] & imm16)
            return "ANDI"

        # ORI rt, rs, imm  — zero-extend imm16, bitwise OR
        if op == 0x0D:
            self._set_reg(rt, self._regs[rs] | imm16)
            return "ORI"

        # XORI rt, rs, imm  — zero-extend imm16, bitwise XOR
        if op == 0x0E:
            self._set_reg(rt, self._regs[rs] ^ imm16)
            return "XORI"

        # LUI rt, imm  — load upper 16 bits; lower 16 bits zeroed
        if op == 0x0F:
            self._set_reg(rt, (imm16 << 16) & 0xFFFF_FFFF)
            return "LUI"

        # ── I-type loads ──────────────────────────────────────────────────────

        # LB rt, offset(rs)  — load byte, sign-extend to 32 bits
        if op == 0x20:
            ea = self._mem_addr(self._regs[rs], simm)
            byte = self._load_byte(ea)
            # Sign-extend: if bit 7 set, fill upper 24 bits with 1s
            self._set_reg(rt, _as_unsigned32(byte - 0x100 if byte >= 0x80 else byte))
            return "LB"

        # LH rt, offset(rs)  — load halfword, sign-extend
        if op == 0x21:
            ea = self._mem_addr(self._regs[rs], simm)
            half = self._load_half(ea)
            self._set_reg(rt, _as_unsigned32(half - 0x10000 if half >= 0x8000 else half))
            return "LH"

        # LW rt, offset(rs)  — load word (32-bit)
        if op == 0x23:
            ea = self._mem_addr(self._regs[rs], simm)
            self._set_reg(rt, self._load_word(ea))
            return "LW"

        # LBU rt, offset(rs)  — load byte, zero-extend
        if op == 0x24:
            ea = self._mem_addr(self._regs[rs], simm)
            self._set_reg(rt, self._load_byte(ea))
            return "LBU"

        # LHU rt, offset(rs)  — load halfword, zero-extend
        if op == 0x25:
            ea = self._mem_addr(self._regs[rs], simm)
            self._set_reg(rt, self._load_half(ea))
            return "LHU"

        # ── I-type stores ─────────────────────────────────────────────────────

        # SB rt, offset(rs)  — store least-significant byte
        if op == 0x28:
            ea = self._mem_addr(self._regs[rs], simm)
            self._store_byte(ea, self._regs[rt])
            return "SB"

        # SH rt, offset(rs)  — store least-significant halfword
        if op == 0x29:
            ea = self._mem_addr(self._regs[rs], simm)
            self._store_half(ea, self._regs[rt])
            return "SH"

        # SW rt, offset(rs)  — store word
        if op == 0x2B:
            ea = self._mem_addr(self._regs[rs], simm)
            self._store_word(ea, self._regs[rt])
            return "SW"

        raise ValueError(f"Unknown opcode: 0x{op:02X} (instr=0x{iw:08X}) at PC=0x{(self._pc - 4) & 0xFFFF:04X}")

    # ── R-type dispatch ───────────────────────────────────────────────────────

    def _r_type(self, rs: int, rt: int, rd: int, shamt: int, funct: int) -> str:  # noqa: C901
        """Handle all R-type instructions (op == 0), dispatched by funct."""

        # SLL rd, rt, shamt  — logical left shift by immediate
        if funct == 0x00:
            self._set_reg(rd, (self._regs[rt] << shamt) & 0xFFFF_FFFF)
            return "SLL"

        # SRL rd, rt, shamt  — logical right shift by immediate (zero-fill)
        if funct == 0x02:
            self._set_reg(rd, self._regs[rt] >> shamt)
            return "SRL"

        # SRA rd, rt, shamt  — arithmetic right shift by immediate (sign-fill)
        if funct == 0x03:
            self._set_reg(rd, _as_unsigned32(_as_signed32(self._regs[rt]) >> shamt))
            return "SRA"

        # SLLV rd, rt, rs  — logical left shift by register (rs & 31)
        if funct == 0x04:
            self._set_reg(rd, (self._regs[rt] << (self._regs[rs] & 31)) & 0xFFFF_FFFF)
            return "SLLV"

        # SRLV rd, rt, rs  — logical right shift by register
        if funct == 0x06:
            self._set_reg(rd, self._regs[rt] >> (self._regs[rs] & 31))
            return "SRLV"

        # SRAV rd, rt, rs  — arithmetic right shift by register
        if funct == 0x07:
            self._set_reg(rd, _as_unsigned32(_as_signed32(self._regs[rt]) >> (self._regs[rs] & 31)))
            return "SRAV"

        # JR rs  — jump to register
        if funct == 0x08:
            self._pc = self._regs[rs] & (MEM_SIZE - 1)
            return "JR"

        # JALR rd, rs  — jump and link register; rd = PC+4 (already advanced)
        if funct == 0x09:
            ret_addr = self._pc  # _fetch32 advanced PC to next instruction
            self._pc = self._regs[rs] & (MEM_SIZE - 1)
            self._set_reg(rd, ret_addr)
            return "JALR"

        # BREAK  — software breakpoint (treated as program error, not HALT)
        if funct == 0x0D:
            pc_of_instr = (self._pc - 4) & (MEM_SIZE - 1)
            raise ValueError(f"BREAK instruction at PC=0x{pc_of_instr:04X}")

        # MFHI rd  — move from HI
        if funct == 0x10:
            self._set_reg(rd, self._hi)
            return "MFHI"

        # MTHI rs  — move to HI
        if funct == 0x11:
            self._hi = self._regs[rs]
            return "MTHI"

        # MFLO rd  — move from LO
        if funct == 0x12:
            self._set_reg(rd, self._lo)
            return "MFLO"

        # MTLO rs  — move to LO
        if funct == 0x13:
            self._lo = self._regs[rs]
            return "MTLO"

        # MULT rs, rt  — signed 32×32 → 64 multiply; HI:LO = result
        if funct == 0x18:
            product = _as_signed32(self._regs[rs]) * _as_signed32(self._regs[rt])
            product &= 0xFFFF_FFFF_FFFF_FFFF   # keep 64 bits (Python int is unbounded)
            self._lo = product & 0xFFFF_FFFF
            self._hi = (product >> 32) & 0xFFFF_FFFF
            return "MULT"

        # MULTU rs, rt  — unsigned 32×32 → 64 multiply
        if funct == 0x19:
            product = self._regs[rs] * self._regs[rt]
            self._lo = product & 0xFFFF_FFFF
            self._hi = (product >> 32) & 0xFFFF_FFFF
            return "MULTU"

        # DIV rs, rt  — signed divide; LO = quotient, HI = remainder
        if funct == 0x1A:
            if self._regs[rt] == 0:
                raise ValueError("DIV by zero")
            a = _as_signed32(self._regs[rs])
            b = _as_signed32(self._regs[rt])
            # Python // truncates toward negative infinity; MIPS truncates toward zero.
            q = int(a / b)          # truncate-toward-zero division
            r = a - q * b
            self._lo = _as_unsigned32(q)
            self._hi = _as_unsigned32(r)
            return "DIV"

        # DIVU rs, rt  — unsigned divide
        if funct == 0x1B:
            if self._regs[rt] == 0:
                raise ValueError("DIVU by zero")
            self._lo = self._regs[rs] // self._regs[rt]
            self._hi = self._regs[rs] %  self._regs[rt]
            return "DIVU"

        # ADD rd, rs, rt  — signed add; ValueError on overflow
        if funct == 0x20:
            s = _as_signed32(self._regs[rs]) + _as_signed32(self._regs[rt])
            if s < -(2**31) or s > 2**31 - 1:
                raise ValueError(f"ADD signed overflow: {_as_signed32(self._regs[rs])} + {_as_signed32(self._regs[rt])}")
            self._set_reg(rd, _as_unsigned32(s))
            return "ADD"

        # ADDU rd, rs, rt  — unsigned add; wraps silently
        if funct == 0x21:
            self._set_reg(rd, _as_unsigned32(self._regs[rs] + self._regs[rt]))
            return "ADDU"

        # SUB rd, rs, rt  — signed subtract; ValueError on overflow
        if funct == 0x22:
            s = _as_signed32(self._regs[rs]) - _as_signed32(self._regs[rt])
            if s < -(2**31) or s > 2**31 - 1:
                raise ValueError("SUB signed overflow")
            self._set_reg(rd, _as_unsigned32(s))
            return "SUB"

        # SUBU rd, rs, rt  — unsigned subtract; wraps silently
        if funct == 0x23:
            self._set_reg(rd, _as_unsigned32(self._regs[rs] - self._regs[rt]))
            return "SUBU"

        # AND rd, rs, rt  — bitwise AND
        if funct == 0x24:
            self._set_reg(rd, self._regs[rs] & self._regs[rt])
            return "AND"

        # OR rd, rs, rt  — bitwise OR
        if funct == 0x25:
            self._set_reg(rd, self._regs[rs] | self._regs[rt])
            return "OR"

        # XOR rd, rs, rt  — bitwise XOR
        if funct == 0x26:
            self._set_reg(rd, self._regs[rs] ^ self._regs[rt])
            return "XOR"

        # NOR rd, rs, rt  — bitwise NOR (complement of OR)
        if funct == 0x27:
            self._set_reg(rd, (~(self._regs[rs] | self._regs[rt])) & 0xFFFF_FFFF)
            return "NOR"

        # SLT rd, rs, rt  — set if signed(rs) < signed(rt)
        if funct == 0x2A:
            self._set_reg(rd, 1 if _as_signed32(self._regs[rs]) < _as_signed32(self._regs[rt]) else 0)
            return "SLT"

        # SLTU rd, rs, rt  — set if unsigned(rs) < unsigned(rt)
        if funct == 0x2B:
            self._set_reg(rd, 1 if self._regs[rs] < self._regs[rt] else 0)
            return "SLTU"

        pc_of_instr = (self._pc - 4) & (MEM_SIZE - 1)
        raise ValueError(f"Unknown funct: 0x{funct:02X} at PC=0x{pc_of_instr:04X}")

    # ── REGIMM dispatch ───────────────────────────────────────────────────────

    def _regimm(self, rs: int, rt: int, simm: int) -> str:
        """Handle REGIMM instructions (op == 1), dispatched by rt field."""
        # Branch target: PC is already advanced past the instruction
        target = _as_unsigned32(self._pc + (simm << 2)) & (MEM_SIZE - 1)

        # BLTZ rs, offset  — branch if signed(rs) < 0
        if rt == 0x00:
            if _as_signed32(self._regs[rs]) < 0:
                self._pc = target
            return "BLTZ"

        # BGEZ rs, offset  — branch if signed(rs) >= 0
        if rt == 0x01:
            if _as_signed32(self._regs[rs]) >= 0:
                self._pc = target
            return "BGEZ"

        # BLTZAL rs, offset  — $ra = PC+4; branch if signed(rs) < 0
        if rt == 0x10:
            ret = self._pc  # already past instruction
            self._set_reg(REG_RA, ret)
            if _as_signed32(self._regs[rs]) < 0:
                self._pc = target
            return "BLTZAL"

        # BGEZAL rs, offset  — $ra = PC+4; branch if signed(rs) >= 0
        if rt == 0x11:
            ret = self._pc
            self._set_reg(REG_RA, ret)
            if _as_signed32(self._regs[rs]) >= 0:
                self._pc = target
            return "BGEZAL"

        pc_of_instr = (self._pc - 4) & (MEM_SIZE - 1)
        raise ValueError(f"Unknown REGIMM rt: 0x{rt:02X} at PC=0x{pc_of_instr:04X}")
