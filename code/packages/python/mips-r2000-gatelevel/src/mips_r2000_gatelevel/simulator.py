"""simulator.py — MIPS R2000 gate-level simulator.

This is the top-level execution engine.  It implements ``Simulator[MIPSState]``
from the simulator-protocol package, matching the same external API as the
behavioral ``MIPSSimulator`` in the ``mips_r2000_simulator`` package.

Architecture overview
──────────────────────
The gate-level simulator is structured as a pipeline:

  1. FETCH   — read 4 bytes from memory at PC (big-endian), increment PC via
               gate-level adder.
  2. DECODE  — extract instruction fields using bit-slice operations (decoder.py).
  3. EXECUTE — dispatch to per-instruction handler.  Every arithmetic / logic
               operation goes through alu.py (which calls ripple_carry_adder,
               AND, OR, XOR, NOT).
  4. WRITEBACK — store results into the register file (bit arrays).

Difference from behavioral simulator
──────────────────────────────────────
The behavioral MIPSSimulator uses Python arithmetic directly:
    self._regs[rd] = (self._regs[rs] + self._regs[rt]) & 0xFFFF_FFFF

The gate-level simulator routes every data-path operation through gates:
    result = add32(self._rf.read_reg(rs), self._rf.read_reg(rt))
    self._rf.write_reg(rd, result.result)

Both produce identical results for any valid MIPS R2000 program.

Memory model
────────────
64 KB flat bytearray, big-endian.  Words are always 4-byte aligned.
The big-endian load uses Python indexing (memory access, not arithmetic):
    word = (mem[a] << 24) | (mem[a+1] << 16) | (mem[a+2] << 8) | mem[a+3]
These are Python integer ops used for *memory decoding* (allowed), not
for data-path computation.

Signed overflow detection
──────────────────────────
ADD, ADDI, SUB raise ValueError on signed overflow.
ADDU, ADDIU, SUBU silently wrap (use result directly without overflow check).

Branch / Jump handling
───────────────────────
No delay slots.  The PC is already incremented past the instruction when
branch targets are computed:
    branch_target = PC_after_fetch + (sext(offset) * 4)
    jump_target   = (PC_after_fetch & 0xF0000000) | (target26 << 2)
"""

from __future__ import annotations

from mips_r2000_simulator.state import (
    HALT_OPCODE_WORD,
    MEM_SIZE,
    NUM_REGS,
    REG_RA,
    MIPSState,
)
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .alu import (
    add32,
    and32,
    div32,
    divu32,
    mult32,
    multu32,
    nor32,
    or32,
    sll32,
    slt32,
    sltu32,
    sra32,
    srl32,
    sub32,
    xor32,
)
from .bits import bits_to_int, int_to_bits, shl_32
from .decoder import decode_instruction
from .register_file import RegisterFile32

# ── Helpers ────────────────────────────────────────────────────────────────────

_MEM_MASK = MEM_SIZE - 1  # 0xFFFF for 64KB address space


def _sext16(v: int) -> int:
    """Sign-extend a 16-bit value to a signed Python int.

    Used only for address arithmetic (effective address calculation), which
    is allowed to use Python integers since it's memory indexing, not
    data-path computation.
    """
    v &= 0xFFFF
    return v - 0x10000 if v >= 0x8000 else v


def _as_unsigned32(v: int) -> int:
    """Mask to 32 unsigned bits."""
    return v & 0xFFFF_FFFF


# ── Simulator class ────────────────────────────────────────────────────────────


class MIPSR2000GateLevelSimulator(Simulator[MIPSState]):
    """Gate-level MIPS R2000 simulator.

    Every arithmetic and logic ALU operation routes through:
      - ``ripple_carry_adder`` from the ``arithmetic`` package
      - ``AND``, ``OR``, ``XOR``, ``NOT`` from the ``logic_gates`` package

    Implements ``Simulator[MIPSState]`` for compatibility with the behavioral
    simulator and compiler pipeline.

    Internal state:
        _mem:    bytearray(MEM_SIZE)   — 64 KB flat big-endian memory
        _rf:     RegisterFile32        — 32 GPRs + HI/LO/PC as bit arrays
        _halted: bool                  — True after SYSCALL
    """

    _MAX_MULT_ITER = 32   # exactly 32 iterations for 32-bit multiply
    _MAX_DIV_ITER = 32    # exactly 32 iterations for long division

    def __init__(self) -> None:
        self._mem: bytearray = bytearray(MEM_SIZE)
        self._rf: RegisterFile32 = RegisterFile32()
        self._halted: bool = False

    # ── Protocol: reset ───────────────────────────────────────────────────────

    def reset(self) -> None:
        """Return the CPU to power-on state.

        All registers zeroed, memory cleared, PC = 0, halted = False.
        """
        self._mem[:] = bytearray(MEM_SIZE)
        self._rf = RegisterFile32()
        self._halted = False

    # ── Protocol: load ────────────────────────────────────────────────────────

    def load(self, program: bytes, origin: int = 0) -> None:
        """Reset and load program bytes into memory at ``origin``.

        Args:
            program: Raw big-endian MIPS machine code.
            origin:  Start address (default 0).

        Raises:
            ValueError: if the program doesn't fit in 64 KB.
        """
        if len(program) > MEM_SIZE:
            msg = f"Program too large: {len(program)} bytes > {MEM_SIZE}"
            raise ValueError(msg)
        self.reset()
        self._mem[origin : origin + len(program)] = program

    # ── Protocol: get_state ───────────────────────────────────────────────────

    def get_state(self) -> MIPSState:
        """Return a frozen snapshot of the current CPU state."""
        regs = tuple(self._rf.read_reg(i) for i in range(NUM_REGS))
        return MIPSState(
            pc=self._rf.read_pc(),
            regs=regs,
            hi=self._rf.read_hi(),
            lo=self._rf.read_lo(),
            memory=tuple(self._mem),
            halted=self._halted,
        )

    # ── Protocol: step ────────────────────────────────────────────────────────

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace."""
        pc_before = self._rf.read_pc()
        if self._halted:
            return StepTrace(
                pc_before=pc_before,
                pc_after=pc_before,
                mnemonic="HALT",
                description="HALT (already halted)",
            )
        mnemonic = self._execute_one()
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._rf.read_pc(),
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:04X}",
        )

    # ── Protocol: execute ─────────────────────────────────────────────────────

    def execute(
        self, program: bytes, origin: int = 0, max_steps: int = 100_000
    ) -> ExecutionResult:
        """Load and run program until HALT or max_steps exceeded."""
        self.load(program, origin)
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
            halted=self._halted,
            steps=steps,
            traces=traces,
            final_state=self.get_state(),
            error=error,
        )

    # ── Protocol: I/O ports (no-op for MIPS) ─────────────────────────────────

    def set_input_port(self, port: int, value: int) -> None:
        """No-op: MIPS R2000 has no memory-mapped I/O ports in this model."""

    def get_output_port(self, port: int) -> int:
        """No-op: return 0 for any port."""
        return 0

    def interrupt(self) -> None:
        """No-op: no interrupt model in this simulator."""

    def nmi(self) -> None:
        """No-op: no NMI model in this simulator."""

    # =========================================================================
    # Internal: instruction fetch
    # =========================================================================

    def _fetch_word(self) -> int:
        """Fetch one 32-bit big-endian word from memory at PC, increment PC by 4.

        Memory fetch is allowed to use Python integer operations (bit shifts and
        OR) because this is memory *decoding* (assembling bytes into a word),
        not data-path arithmetic.  The PC increment itself uses the gate-level
        adder via register_file.increment_pc().

        Returns:
            32-bit instruction word as an unsigned integer.
        """
        addr = self._rf.read_pc() & _MEM_MASK
        # Big-endian word assembly from 4 bytes
        iw = (
            self._mem[addr] << 24
            | self._mem[addr + 1] << 16
            | self._mem[addr + 2] << 8
            | self._mem[addr + 3]
        )
        # Gate-level PC increment via ripple-carry adder
        self._rf.increment_pc(4)
        return iw

    # =========================================================================
    # Internal: memory access helpers
    # =========================================================================

    def _check_align(self, addr: int, size: int) -> None:
        """Raise ValueError for misaligned access."""
        if addr & (size - 1):
            kind = "word" if size == 4 else "halfword"
            msg = f"Misaligned {kind} access at 0x{addr:04X}"
            raise ValueError(msg)

    def _eff_addr(self, base: int, offset: int) -> int:
        """Compute effective address: (base + sext(offset)) & MEM_MASK.

        Address arithmetic is allowed to use Python integers (memory indexing).
        """
        return _as_unsigned32(base + offset) & _MEM_MASK

    def _load_byte(self, addr: int) -> int:
        return self._mem[addr & _MEM_MASK]

    def _load_half(self, addr: int) -> int:
        self._check_align(addr, 2)
        a = addr & _MEM_MASK
        return (self._mem[a] << 8) | self._mem[a + 1]

    def _load_word(self, addr: int) -> int:
        self._check_align(addr, 4)
        a = addr & _MEM_MASK
        return (
            self._mem[a] << 24
            | self._mem[a + 1] << 16
            | self._mem[a + 2] << 8
            | self._mem[a + 3]
        )

    def _store_byte(self, addr: int, val: int) -> None:
        self._mem[addr & _MEM_MASK] = val & 0xFF

    def _store_half(self, addr: int, val: int) -> None:
        self._check_align(addr, 2)
        a = addr & _MEM_MASK
        self._mem[a] = (val >> 8) & 0xFF
        self._mem[a + 1] = val & 0xFF

    def _store_word(self, addr: int, val: int) -> None:
        self._check_align(addr, 4)
        a = addr & _MEM_MASK
        self._mem[a] = (val >> 24) & 0xFF
        self._mem[a + 1] = (val >> 16) & 0xFF
        self._mem[a + 2] = (val >> 8) & 0xFF
        self._mem[a + 3] = val & 0xFF

    # =========================================================================
    # Internal: instruction execution
    # =========================================================================

    def _execute_one(self) -> str:  # noqa: C901 — complex by necessity
        """Fetch, decode, and execute one MIPS instruction.  Returns mnemonic."""
        iw = self._fetch_word()

        # HALT: SYSCALL (op=0, funct=0x0C) — any SYSCALL halts the simulator
        if iw == HALT_OPCODE_WORD or (iw >> 26 == 0 and (iw & 0x3F) == 0x0C):
            self._halted = True
            return "HALT"

        # NOP: canonical SLL $zero, $zero, 0
        if iw == 0x0000_0000:
            return "NOP"

        d = decode_instruction(iw)
        op = d["op"]

        if op == 0:
            return self._exec_r_type(d)
        if op == 0x01:
            return self._exec_regimm(d)

        # J-type
        if op == 0x02:
            return self._exec_j(d)
        if op == 0x03:
            return self._exec_jal(d)

        # I-type branches
        if op == 0x04:
            return self._exec_beq(d)
        if op == 0x05:
            return self._exec_bne(d)
        if op == 0x06:
            return self._exec_blez(d)
        if op == 0x07:
            return self._exec_bgtz(d)

        # I-type arithmetic / logic
        if op == 0x08:
            return self._exec_addi(d)
        if op == 0x09:
            return self._exec_addiu(d)
        if op == 0x0A:
            return self._exec_slti(d)
        if op == 0x0B:
            return self._exec_sltiu(d)
        if op == 0x0C:
            return self._exec_andi(d)
        if op == 0x0D:
            return self._exec_ori(d)
        if op == 0x0E:
            return self._exec_xori(d)
        if op == 0x0F:
            return self._exec_lui(d)

        # I-type loads
        if op == 0x20:
            return self._exec_lb(d)
        if op == 0x21:
            return self._exec_lh(d)
        if op == 0x22:
            return self._exec_lwl(d)
        if op == 0x23:
            return self._exec_lw(d)
        if op == 0x24:
            return self._exec_lbu(d)
        if op == 0x25:
            return self._exec_lhu(d)
        if op == 0x26:
            return self._exec_lwr(d)

        # I-type stores
        if op == 0x28:
            return self._exec_sb(d)
        if op == 0x29:
            return self._exec_sh(d)
        if op == 0x2A:
            return self._exec_swl(d)
        if op == 0x2B:
            return self._exec_sw(d)
        if op == 0x2E:
            return self._exec_swr(d)

        pc_instr = (self._rf.read_pc() - 4) & _MEM_MASK
        raise ValueError(
            f"Unknown opcode: 0x{op:02X} (instr=0x{iw:08X}) at PC=0x{pc_instr:04X}"
        )

    # =========================================================================
    # R-type dispatch
    # =========================================================================

    def _exec_r_type(self, d: dict) -> str:  # noqa: C901
        """Dispatch R-type instruction by funct field."""
        rs = d["rs"]
        rt = d["rt"]
        rd = d["rd"]
        shamt = d["shamt"]
        funct = d["funct"]
        pc_after = self._rf.read_pc()

        rs_val = self._rf.read_reg(rs)
        rt_val = self._rf.read_reg(rt)

        # ── Shifts ────────────────────────────────────────────────────────────
        if funct == 0x00:  # SLL rd, rt, shamt
            r = sll32(rt_val, shamt)
            self._rf.write_reg(rd, r.result)
            return "SLL"

        if funct == 0x02:  # SRL rd, rt, shamt
            r = srl32(rt_val, shamt)
            self._rf.write_reg(rd, r.result)
            return "SRL"

        if funct == 0x03:  # SRA rd, rt, shamt
            r = sra32(rt_val, shamt)
            self._rf.write_reg(rd, r.result)
            return "SRA"

        if funct == 0x04:  # SLLV rd, rt, rs
            # Shift amount from register, masked to 5 bits (0–31)
            sa_bits = int_to_bits(rs_val, 32)
            sa = bits_to_int(sa_bits[0:5])  # lower 5 bits only
            r = sll32(rt_val, sa)
            self._rf.write_reg(rd, r.result)
            return "SLLV"

        if funct == 0x06:  # SRLV rd, rt, rs
            sa_bits = int_to_bits(rs_val, 32)
            sa = bits_to_int(sa_bits[0:5])
            r = srl32(rt_val, sa)
            self._rf.write_reg(rd, r.result)
            return "SRLV"

        if funct == 0x07:  # SRAV rd, rt, rs
            sa_bits = int_to_bits(rs_val, 32)
            sa = bits_to_int(sa_bits[0:5])
            r = sra32(rt_val, sa)
            self._rf.write_reg(rd, r.result)
            return "SRAV"

        # ── Jumps ─────────────────────────────────────────────────────────────
        if funct == 0x08:  # JR rs
            self._rf.write_pc(rs_val & _MEM_MASK)
            return "JR"

        if funct == 0x09:  # JALR rd, rs
            # rd = return address (current PC, already past the instruction)
            self._rf.write_reg(rd, pc_after)
            self._rf.write_pc(rs_val & _MEM_MASK)
            return "JALR"

        # ── BREAK ─────────────────────────────────────────────────────────────
        if funct == 0x0D:
            pc_instr = (self._rf.read_pc() - 4) & _MEM_MASK
            raise ValueError(f"BREAK instruction at PC=0x{pc_instr:04X}")

        # ── HI/LO moves ───────────────────────────────────────────────────────
        if funct == 0x10:  # MFHI rd
            self._rf.write_reg(rd, self._rf.read_hi())
            return "MFHI"

        if funct == 0x11:  # MTHI rs
            self._rf.write_hi(rs_val)
            return "MTHI"

        if funct == 0x12:  # MFLO rd
            self._rf.write_reg(rd, self._rf.read_lo())
            return "MFLO"

        if funct == 0x13:  # MTLO rs
            self._rf.write_lo(rs_val)
            return "MTLO"

        # ── Multiply ──────────────────────────────────────────────────────────
        if funct == 0x18:  # MULT rs, rt (signed)
            hi, lo = mult32(rs_val, rt_val)
            self._rf.write_hi(hi)
            self._rf.write_lo(lo)
            return "MULT"

        if funct == 0x19:  # MULTU rs, rt (unsigned)
            hi, lo = multu32(rs_val, rt_val)
            self._rf.write_hi(hi)
            self._rf.write_lo(lo)
            return "MULTU"

        # ── Divide ────────────────────────────────────────────────────────────
        if funct == 0x1A:  # DIV rs, rt (signed)
            q, r2 = div32(rs_val, rt_val)
            self._rf.write_lo(q)
            self._rf.write_hi(r2)
            return "DIV"

        if funct == 0x1B:  # DIVU rs, rt (unsigned)
            q, r2 = divu32(rs_val, rt_val)
            self._rf.write_lo(q)
            self._rf.write_hi(r2)
            return "DIVU"

        # ── Arithmetic ────────────────────────────────────────────────────────
        if funct == 0x20:  # ADD rd, rs, rt (signed, raises on overflow)
            r = add32(rs_val, rt_val)
            if r.overflow:
                raise ValueError(
                    f"ADD signed overflow: 0x{rs_val:08X} + 0x{rt_val:08X}"
                )
            self._rf.write_reg(rd, r.result)
            return "ADD"

        if funct == 0x21:  # ADDU rd, rs, rt (wraps silently)
            r = add32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "ADDU"

        if funct == 0x22:  # SUB rd, rs, rt (signed, raises on overflow)
            r = sub32(rs_val, rt_val)
            if r.overflow:
                raise ValueError(
                    f"SUB signed overflow: 0x{rs_val:08X} - 0x{rt_val:08X}"
                )
            self._rf.write_reg(rd, r.result)
            return "SUB"

        if funct == 0x23:  # SUBU rd, rs, rt (wraps silently)
            r = sub32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "SUBU"

        if funct == 0x24:  # AND rd, rs, rt
            r = and32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "AND"

        if funct == 0x25:  # OR rd, rs, rt
            r = or32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "OR"

        if funct == 0x26:  # XOR rd, rs, rt
            r = xor32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "XOR"

        if funct == 0x27:  # NOR rd, rs, rt
            r = nor32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "NOR"

        if funct == 0x2A:  # SLT rd, rs, rt (signed)
            r = slt32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "SLT"

        if funct == 0x2B:  # SLTU rd, rs, rt (unsigned)
            r = sltu32(rs_val, rt_val)
            self._rf.write_reg(rd, r.result)
            return "SLTU"

        pc_instr = (self._rf.read_pc() - 4) & _MEM_MASK
        raise ValueError(f"Unknown funct: 0x{funct:02X} at PC=0x{pc_instr:04X}")

    # =========================================================================
    # REGIMM dispatch (op=0x01)
    # =========================================================================

    def _exec_regimm(self, d: dict) -> str:
        """Dispatch REGIMM instructions by rt field."""
        rs = d["rs"]
        rt = d["rt"]
        offset = d["imm16"]  # already sign-extended by decoder

        rs_val = self._rf.read_reg(rs)
        rs_bits = int_to_bits(rs_val, 32)
        rs_negative = rs_bits[31]  # sign bit

        # branch target = PC_after_fetch + (sext(offset) << 2)
        pc_now = self._rf.read_pc()

        if rt == 0x00:  # BLTZ: branch if rs < 0
            if rs_negative:
                target = _as_unsigned32(pc_now + offset * 4) & _MEM_MASK
                self._rf.write_pc(target)
            return "BLTZ"

        if rt == 0x01:  # BGEZ: branch if rs >= 0
            if not rs_negative:
                target = _as_unsigned32(pc_now + offset * 4) & _MEM_MASK
                self._rf.write_pc(target)
            return "BGEZ"

        if rt == 0x10:  # BLTZAL: $ra = PC+4; branch if rs < 0
            self._rf.write_reg(REG_RA, pc_now)
            if rs_negative:
                target = _as_unsigned32(pc_now + offset * 4) & _MEM_MASK
                self._rf.write_pc(target)
            return "BLTZAL"

        if rt == 0x11:  # BGEZAL: $ra = PC+4; branch if rs >= 0
            self._rf.write_reg(REG_RA, pc_now)
            if not rs_negative:
                target = _as_unsigned32(pc_now + offset * 4) & _MEM_MASK
                self._rf.write_pc(target)
            return "BGEZAL"

        pc_instr = (self._rf.read_pc() - 4) & _MEM_MASK
        raise ValueError(f"Unknown REGIMM rt: 0x{rt:02X} at PC=0x{pc_instr:04X}")

    # =========================================================================
    # J-type instructions
    # =========================================================================

    def _exec_j(self, d: dict) -> str:
        """J addr — unconditional jump."""
        target26 = d["target26"]
        pc_now = self._rf.read_pc()  # already past instruction
        # Target = (PC & 0xF0000000) | (target26 << 2), masked to MEM_SIZE
        shifted = shl_32(target26, 2)
        new_pc = ((pc_now & 0xF000) | (shifted & 0xFFFF)) & _MEM_MASK
        self._rf.write_pc(new_pc)
        return "J"

    def _exec_jal(self, d: dict) -> str:
        """JAL addr — jump and link ($ra = PC+4, already advanced)."""
        target26 = d["target26"]
        pc_now = self._rf.read_pc()
        ret_addr = pc_now
        shifted = shl_32(target26, 2)
        new_pc = ((pc_now & 0xF000) | (shifted & 0xFFFF)) & _MEM_MASK
        self._rf.write_reg(REG_RA, ret_addr)
        self._rf.write_pc(new_pc)
        return "JAL"

    # =========================================================================
    # Branch instructions
    # =========================================================================

    def _exec_beq(self, d: dict) -> str:
        """BEQ rs, rt, offset — branch if rs == rt."""
        rs_val = self._rf.read_reg(d["rs"])
        rt_val = self._rf.read_reg(d["rt"])
        # Equality test: XOR then zero-check
        eq_r = xor32(rs_val, rt_val)
        if eq_r.zero:
            pc_now = self._rf.read_pc()
            target = _as_unsigned32(pc_now + d["imm16"] * 4) & _MEM_MASK
            self._rf.write_pc(target)
        return "BEQ"

    def _exec_bne(self, d: dict) -> str:
        """BNE rs, rt, offset — branch if rs != rt."""
        rs_val = self._rf.read_reg(d["rs"])
        rt_val = self._rf.read_reg(d["rt"])
        eq_r = xor32(rs_val, rt_val)
        if not eq_r.zero:
            pc_now = self._rf.read_pc()
            target = _as_unsigned32(pc_now + d["imm16"] * 4) & _MEM_MASK
            self._rf.write_pc(target)
        return "BNE"

    def _exec_blez(self, d: dict) -> str:
        """BLEZ rs, offset — branch if signed(rs) <= 0."""
        rs_val = self._rf.read_reg(d["rs"])
        rs_bits = int_to_bits(rs_val, 32)
        rs_neg = rs_bits[31]
        # zero check
        from .bits import compute_zero as _cz
        rs_zero_flag = _cz(rs_bits)
        if rs_neg or rs_zero_flag:
            pc_now = self._rf.read_pc()
            target = _as_unsigned32(pc_now + d["imm16"] * 4) & _MEM_MASK
            self._rf.write_pc(target)
        return "BLEZ"

    def _exec_bgtz(self, d: dict) -> str:
        """BGTZ rs, offset — branch if signed(rs) > 0."""
        rs_val = self._rf.read_reg(d["rs"])
        rs_bits = int_to_bits(rs_val, 32)
        rs_neg = rs_bits[31]
        from .bits import compute_zero as _cz
        rs_zero_flag = _cz(rs_bits)
        if (not rs_neg) and (not rs_zero_flag):
            pc_now = self._rf.read_pc()
            target = _as_unsigned32(pc_now + d["imm16"] * 4) & _MEM_MASK
            self._rf.write_pc(target)
        return "BGTZ"

    # =========================================================================
    # I-type arithmetic / logic
    # =========================================================================

    def _exec_addi(self, d: dict) -> str:
        """ADDI rt, rs, imm — signed add immediate; raises on overflow."""
        rs_val = self._rf.read_reg(d["rs"])
        imm = d["imm16"]  # already sign-extended
        imm_u = _as_unsigned32(imm)
        r = add32(rs_val, imm_u)
        if r.overflow:
            raise ValueError(f"ADDI signed overflow: 0x{rs_val:08X} + {imm}")
        self._rf.write_reg(d["rt"], r.result)
        return "ADDI"

    def _exec_addiu(self, d: dict) -> str:
        """ADDIU rt, rs, imm — unsigned add immediate; wraps silently."""
        rs_val = self._rf.read_reg(d["rs"])
        imm_u = _as_unsigned32(d["imm16"])
        r = add32(rs_val, imm_u)
        self._rf.write_reg(d["rt"], r.result)
        return "ADDIU"

    def _exec_slti(self, d: dict) -> str:
        """SLTI rt, rs, imm — set if signed(rs) < sext(imm)."""
        rs_val = self._rf.read_reg(d["rs"])
        imm_u = _as_unsigned32(d["imm16"])
        r = slt32(rs_val, imm_u)
        self._rf.write_reg(d["rt"], r.result)
        return "SLTI"

    def _exec_sltiu(self, d: dict) -> str:
        """SLTIU rt, rs, imm — set if unsigned(rs) < unsigned(sext(imm))."""
        rs_val = self._rf.read_reg(d["rs"])
        imm_u = _as_unsigned32(d["imm16"])
        r = sltu32(rs_val, imm_u)
        self._rf.write_reg(d["rt"], r.result)
        return "SLTIU"

    def _exec_andi(self, d: dict) -> str:
        """ANDI rt, rs, imm — AND with zero-extended immediate."""
        rs_val = self._rf.read_reg(d["rs"])
        # Zero-extend: mask off upper 16 bits of the sign-extended immediate
        imm16_bits = int_to_bits(d["imm16"] & 0xFFFF, 32)
        imm_u = bits_to_int(imm16_bits)
        r = and32(rs_val, imm_u)
        self._rf.write_reg(d["rt"], r.result)
        return "ANDI"

    def _exec_ori(self, d: dict) -> str:
        """ORI rt, rs, imm — OR with zero-extended immediate."""
        rs_val = self._rf.read_reg(d["rs"])
        imm_u = d["imm16"] & 0xFFFF  # zero-extend (mask off sign-extension)
        r = or32(rs_val, imm_u)
        self._rf.write_reg(d["rt"], r.result)
        return "ORI"

    def _exec_xori(self, d: dict) -> str:
        """XORI rt, rs, imm — XOR with zero-extended immediate."""
        rs_val = self._rf.read_reg(d["rs"])
        imm_u = d["imm16"] & 0xFFFF
        r = xor32(rs_val, imm_u)
        self._rf.write_reg(d["rt"], r.result)
        return "XORI"

    def _exec_lui(self, d: dict) -> str:
        """LUI rt, imm — load imm into upper 16 bits, lower 16 = 0."""
        imm16 = d["imm16"] & 0xFFFF
        # Shift imm16 left by 16: shl_32 on the zero-extended value
        val = shl_32(imm16, 16)
        self._rf.write_reg(d["rt"], val)
        return "LUI"

    # =========================================================================
    # Load instructions
    # =========================================================================

    def _exec_lb(self, d: dict) -> str:
        """LB rt, off(rs) — load byte, sign-extend."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        byte = self._load_byte(ea)
        # Sign-extend: if bit 7 is set, fill upper 24 bits with 1
        byte_bits = int_to_bits(byte, 8)
        sign = byte_bits[7]
        extended = byte_bits + [sign] * 24
        self._rf.write_reg(d["rt"], bits_to_int(extended))
        return "LB"

    def _exec_lh(self, d: dict) -> str:
        """LH rt, off(rs) — load halfword, sign-extend."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        half = self._load_half(ea)
        half_bits = int_to_bits(half, 16)
        sign = half_bits[15]
        extended = half_bits + [sign] * 16
        self._rf.write_reg(d["rt"], bits_to_int(extended))
        return "LH"

    def _exec_lwl(self, d: dict) -> str:
        """LWL rt, off(rs) — unaligned load left.

        LWL loads bytes from the most significant byte of the aligned word
        that contains the effective address through the specified byte, merging
        them into the most significant bytes of the destination register.
        """
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        byte_offset = ea & 3  # byte position within aligned word (0=MSB, 3=LSB, big-endian)
        word_addr = ea & ~3  # aligned word address
        mem_word = self._load_word(word_addr)
        rt_val = self._rf.read_reg(d["rt"])
        # LWL loads bytes from mem[word_addr..ea] into the HIGH bytes of rt.
        # big-endian byte order: byte 0 is MSB, byte 3 is LSB.
        # byte_offset=0: load mem byte 0 only  → rt = (mem_word & 0xFF000000) | (rt & 0x00FFFFFF)
        # byte_offset=1: load mem bytes 0,1    → rt = (mem_word & 0xFFFF0000) | (rt & 0x0000FFFF)
        # byte_offset=2: load mem bytes 0,1,2  → rt = (mem_word & 0xFFFFFF00) | (rt & 0x000000FF)
        # byte_offset=3: load all 4 bytes      → rt = mem_word
        # Number of LOW bits preserved from rt = (3 - byte_offset) * 8
        shift = (3 - byte_offset) * 8  # low bits to keep from rt
        if shift == 0:
            # byte_offset=3: load full word
            result = mem_word
        else:
            mem_mask = 0xFFFF_FFFF ^ ((1 << shift) - 1)  # high bits from mem
            rt_mask = (1 << shift) - 1                    # low bits from rt
            result = (mem_word & mem_mask) | (rt_val & rt_mask)
        self._rf.write_reg(d["rt"], result & 0xFFFF_FFFF)
        return "LWL"

    def _exec_lw(self, d: dict) -> str:
        """LW rt, off(rs) — load word (4-byte aligned)."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        word = self._load_word(ea)
        self._rf.write_reg(d["rt"], word)
        return "LW"

    def _exec_lbu(self, d: dict) -> str:
        """LBU rt, off(rs) — load byte, zero-extend."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        self._rf.write_reg(d["rt"], self._load_byte(ea))
        return "LBU"

    def _exec_lhu(self, d: dict) -> str:
        """LHU rt, off(rs) — load halfword, zero-extend."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        self._rf.write_reg(d["rt"], self._load_half(ea))
        return "LHU"

    def _exec_lwr(self, d: dict) -> str:
        """LWR rt, off(rs) — unaligned load right.

        LWR loads bytes from the specified byte through the least significant
        byte of the aligned word, merging them into the least significant
        bytes of the destination register.
        """
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        byte_offset = ea & 3  # 0=MSB, 3=LSB in big-endian
        word_addr = ea & ~3
        mem_word = self._load_word(word_addr)
        rt_val = self._rf.read_reg(d["rt"])
        # LWR loads bytes from mem[ea..word_end] into the LOW bytes of rt.
        # byte_offset=0: load all 4 bytes      → rt = mem_word
        # byte_offset=1: load bytes 1,2,3 only → rt = (rt & 0xFF000000) | (mem_word & 0x00FFFFFF)
        # byte_offset=2: load bytes 2,3        → rt = (rt & 0xFFFF0000) | (mem_word & 0x0000FFFF)
        # byte_offset=3: load byte 3 only      → rt = (rt & 0xFFFFFF00) | (mem_word & 0x000000FF)
        # Number of HIGH bits preserved from rt = byte_offset * 8
        shift = byte_offset * 8  # high bits to keep from rt
        if shift == 0:
            # byte_offset=0: load full word
            result = mem_word
        else:
            rt_mask = 0xFFFF_FFFF ^ ((1 << shift) - 1)  # keep high (shift) bits of rt? No:
            # Actually: high shift bits = bits [31..32-shift]. Mask = ones in top 'shift' bits.
            # rt_mask has the top 'shift' bits set: ~((1<<(32-shift))-1) & 0xFFFFFFFF
            # Simpler: rt keeps the top 'shift' bits
            rt_mask = 0xFFFF_FFFF ^ ((1 << (32 - shift)) - 1)
            mem_mask = (1 << (32 - shift)) - 1
            result = (rt_val & rt_mask) | (mem_word & mem_mask)
        self._rf.write_reg(d["rt"], result & 0xFFFF_FFFF)
        return "LWR"

    # =========================================================================
    # Store instructions
    # =========================================================================

    def _exec_sb(self, d: dict) -> str:
        """SB rt, off(rs) — store least-significant byte."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        rt_val = self._rf.read_reg(d["rt"])
        # Extract low byte via bit-list
        rt_bits = int_to_bits(rt_val, 32)
        byte_val = bits_to_int(rt_bits[0:8])
        self._store_byte(ea, byte_val)
        return "SB"

    def _exec_sh(self, d: dict) -> str:
        """SH rt, off(rs) — store least-significant halfword."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        rt_val = self._rf.read_reg(d["rt"])
        rt_bits = int_to_bits(rt_val, 32)
        half_val = bits_to_int(rt_bits[0:16])
        self._store_half(ea, half_val)
        return "SH"

    def _exec_swl(self, d: dict) -> str:
        """SWL rt, off(rs) — unaligned store left."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        byte_offset = ea & 3  # 0=MSB, 3=LSB in big-endian
        word_addr = ea & ~3
        rt_val = self._rf.read_reg(d["rt"])
        mem_word = self._load_word(word_addr)
        # SWL stores the most significant (byte_offset+1) bytes of rt into memory.
        # byte_offset=0: store rt byte 0 (MSB) → mem byte 0
        # byte_offset=1: store rt bytes 0,1   → mem bytes 0,1
        # byte_offset=2: store rt bytes 0,1,2 → mem bytes 0,1,2
        # byte_offset=3: store rt all 4 bytes → mem (full store)
        # Low (3-byte_offset)*8 bits of mem word are preserved.
        shift = (3 - byte_offset) * 8  # low bits preserved from mem
        if shift == 0:
            result = rt_val
        else:
            mem_mask = (1 << shift) - 1  # keep low bits from memory
            rt_mask = 0xFFFF_FFFF ^ mem_mask  # take high bits from rt
            result = (rt_val & rt_mask) | (mem_word & mem_mask)
        self._store_word(word_addr, result & 0xFFFF_FFFF)
        return "SWL"

    def _exec_sw(self, d: dict) -> str:
        """SW rt, off(rs) — store word (4-byte aligned)."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        self._store_word(ea, self._rf.read_reg(d["rt"]))
        return "SW"

    def _exec_swr(self, d: dict) -> str:
        """SWR rt, off(rs) — unaligned store right."""
        ea = self._eff_addr(self._rf.read_reg(d["rs"]), d["imm16"])
        byte_offset = ea & 3  # 0=MSB, 3=LSB in big-endian
        word_addr = ea & ~3
        rt_val = self._rf.read_reg(d["rt"])
        mem_word = self._load_word(word_addr)
        # SWR stores the least significant (4-byte_offset) bytes of rt into memory.
        # byte_offset=0: store rt all 4 bytes → mem (full store)
        # byte_offset=1: store rt bytes 1,2,3 → mem bytes 1,2,3 (keep mem byte 0)
        # byte_offset=2: store rt bytes 2,3   → mem bytes 2,3 (keep mem bytes 0,1)
        # byte_offset=3: store rt byte 3 only → mem byte 3 (keep mem bytes 0,1,2)
        # High byte_offset*8 bits of mem word are preserved.
        shift = byte_offset * 8  # high bits preserved from mem
        if shift == 0:
            result = rt_val
        else:
            rt_low_bits = 32 - shift
            rt_mask = (1 << rt_low_bits) - 1   # keep low (32-shift) bits from rt
            mem_mask = 0xFFFF_FFFF ^ rt_mask    # keep high shift bits from mem
            result = (mem_word & mem_mask) | (rt_val & rt_mask)
        self._store_word(word_addr, result & 0xFFFF_FFFF)
        return "SWR"
