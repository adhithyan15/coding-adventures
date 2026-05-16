"""simulator.py — Gate-level DEC Alpha AXP 21064 simulator.

This is the top-level integration module.  It composes:
  - RegisterFile64     — 32 × 64-bit GPRs and PC stored as bit lists
  - decoder            — pure combinational instruction decode
  - alu.py functions   — all data-path ops route through gate primitives

Every integer arithmetic/logic operation on register values routes through
gate functions (AND, OR, XOR, NOT) and ripple_carry_adder — NO Python
operators in the execution path for ALU/register operations.

Python arithmetic IS used for:
  - Memory address computation (ea = Rb_value + sext16(disp))
  - Memory indexing (self._mem[addr])
  - Loop control (for i in range(N))
  - PC alignment (target & ~3)

These are host-machine bookkeeping operations, not simulated data-path ops.

Execution model
───────────────
Each call to step():
  1. PC_before = current PC
  2. Fetch 4 bytes from memory[PC..PC+3] as little-endian 32-bit word
  3. PC ← nPC (already incremented by fetch)
  4. Decode the instruction word
  5. Dispatch to the appropriate handler
  6. Return StepTrace(pc_before, pc_after, mnemonic, description)

nPC tracking
────────────
Alpha uses both PC (current instruction address) and nPC (next instruction,
normally PC+4).  Branches modify nPC.  After fetch, PC = old_nPC.

Memory layout
─────────────
64 KiB flat, little-endian.  The program is loaded starting at address 0.
Uninitialized memory is zero (which decodes as HALT — a safety net).
"""

from __future__ import annotations

from alpha_axp_simulator.state import (
    MEM_SIZE,
    AlphaState,
)
from logic_gates import AND, NOT, OR
from simulator_protocol import ExecutionResult, StepTrace

from .alu import (
    addl,
    addq,
    andq,
    bicq,
    cmpeq,
    cmple,
    cmplt,
    cmpule,
    cmpult,
    eqvq,
    mull,
    mulq,
    ornot,
    orq,
    s4addl,
    s4addq,
    s4subl,
    s4subq,
    s8addl,
    s8addq,
    s8subl,
    s8subq,
    sll64,
    sra64,
    srl64,
    subl,
    subq,
    umulh,
    xorq,
)
from .bits import (
    bits_to_int,
    compute_zero,
    int_to_bits,
)
from .register_file import RegisterFile64

# ── Constants ──────────────────────────────────────────────────────────────────

_MASK64: int = 0xFFFF_FFFF_FFFF_FFFF
_MASK32: int = 0xFFFF_FFFF
_MASK16: int = 0xFFFF
_MASK8:  int = 0xFF
_MEM_MASK: int = MEM_SIZE - 1  # 0xFFFF for 64 KiB


# ── Sign-extension helpers (used in memory addressing — not data-path) ─────────

def _sext16_int(v: int) -> int:
    v = v & 0xFFFF
    if v >= 0x8000:
        v -= 0x10000
    return v


def _sext32_int(v: int) -> int:
    """Sign-extend 32-bit to Python int (for LDL result)."""
    v = v & _MASK32
    if v >= 0x8000_0000:
        v -= 0x1_0000_0000
    return v


def _u64(v: int) -> int:
    return v & _MASK64


# ── AlphaAXPGateLevelSimulator ─────────────────────────────────────────────────


class AlphaAXPGateLevelSimulator:
    """Gate-level DEC Alpha AXP 21064 simulator.

    Every ALU and register operation routes through logic gate primitives.
    Implements the SIM00 Simulator[AlphaState] protocol.

    Architecture highlights:
      - 32 × 64-bit GPRs (r31 hardwired zero)
      - 64-bit PC + nPC
      - 64 KiB little-endian flat memory
      - HALT = all-zeros word (call_pal 0)
      - No condition codes — comparisons write 0/1 to destination register

    Usage
    ─────
    >>> sim = AlphaAXPGateLevelSimulator()
    >>> import struct
    >>> halt = struct.pack('<I', 0x00000000)
    >>> result = sim.execute(halt)
    >>> result.halted
    True
    """

    def __init__(self) -> None:
        self._rf = RegisterFile64()
        self._mem: bytearray = bytearray(MEM_SIZE)
        self._npc: int = 4
        self._halted: bool = False

    # ── SIM00 Protocol ─────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset all CPU state: registers, memory, PC=0, nPC=4, halted=False."""
        self._rf.reset()
        self._mem = bytearray(MEM_SIZE)
        self._npc = 4
        self._halted = False

    def load(self, program: bytes, origin: int = 0) -> None:
        """Reset then copy program bytes into memory starting at `origin`.

        Raises ValueError if the program exceeds memory.
        """
        if len(program) + origin > MEM_SIZE:
            raise ValueError(
                f"Program ({len(program)} bytes at {origin}) exceeds memory ({MEM_SIZE})"
            )
        self.reset()
        self._mem[origin: origin + len(program)] = program

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If already halted, returns a 'HALT' trace without advancing.
        """
        if self._halted:
            pc = self._rf.read_pc()
            return StepTrace(
                pc_before=pc,
                pc_after=pc,
                mnemonic="HALT",
                description="HALT (already halted)",
            )
        pc_before = self._rf.read_pc()
        try:
            mnemonic = self._execute_one()
        except (ValueError, IndexError) as exc:
            self._halted = True
            msg = f"ERROR: {exc}"
            return StepTrace(
                pc_before=pc_before,
                pc_after=self._rf.read_pc(),
                mnemonic=msg,
                description=msg,
            )
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._rf.read_pc(),
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:04X}",
        )

    def execute(
        self,
        program: bytes,
        origin: int = 0,
        max_steps: int = 100_000,
    ) -> ExecutionResult:
        """Load program and run until HALT or max_steps exceeded."""
        self.load(program, origin)
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
        """Return an immutable snapshot of the current CPU state."""
        return AlphaState(
            pc=self._rf.read_pc(),
            npc=self._npc,
            regs=self._rf.get_regs_tuple(),
            memory=tuple(self._mem),
            halted=self._halted,
        )

    # Input/output ports (no-op — Alpha has no I/O ports in this model)
    def set_input_port(self, port: int, value: int) -> None:
        pass

    def get_output_port(self, port: int) -> int:
        return 0

    def interrupt(self) -> None:
        pass

    def nmi(self) -> None:
        pass

    # ── Register access helpers ────────────────────────────────────────────────

    def _read_reg(self, n: int) -> int:
        return self._rf.read_reg(n)

    def _write_reg(self, n: int, value: int) -> None:
        self._rf.write_reg(n, value)

    # ── Memory helpers (little-endian) ─────────────────────────────────────────

    def _read_byte(self, addr: int) -> int:
        return self._mem[addr & _MEM_MASK]

    def _read_long(self, addr: int) -> int:
        """Read 4-byte little-endian unsigned longword (4-byte alignment required)."""
        a = addr & _MEM_MASK
        if addr & 3:
            raise ValueError(f"Unaligned LDL at 0x{addr:04X}")
        return (
            self._mem[a]
            | (self._mem[(a + 1) & _MEM_MASK] << 8)
            | (self._mem[(a + 2) & _MEM_MASK] << 16)
            | (self._mem[(a + 3) & _MEM_MASK] << 24)
        )

    def _read_quad(self, addr: int) -> int:
        """Read 8-byte little-endian quadword (8-byte alignment required)."""
        a = addr & _MEM_MASK
        if addr & 7:
            raise ValueError(f"Unaligned LDQ at 0x{addr:04X}")
        result = 0
        for i in range(8):
            result |= self._mem[(a + i) & _MEM_MASK] << (8 * i)
        return result

    def _read_quad_unaligned(self, addr: int) -> int:
        """Read 8-byte little-endian quadword aligned to 8-byte boundary."""
        a = (addr & _MEM_MASK) & ~7
        result = 0
        for i in range(8):
            result |= self._mem[(a + i) & _MEM_MASK] << (8 * i)
        return result

    def _write_long(self, addr: int, val: int) -> None:
        a = addr & _MEM_MASK
        if addr & 3:
            raise ValueError(f"Unaligned STL at 0x{addr:04X}")
        for i in range(4):
            self._mem[(a + i) & _MEM_MASK] = (val >> (8 * i)) & _MASK8

    def _write_quad(self, addr: int, val: int) -> None:
        a = addr & _MEM_MASK
        if addr & 7:
            raise ValueError(f"Unaligned STQ at 0x{addr:04X}")
        for i in range(8):
            self._mem[(a + i) & _MEM_MASK] = (val >> (8 * i)) & _MASK8

    def _write_quad_unaligned(self, addr: int, val: int) -> None:
        a = (addr & _MEM_MASK) & ~7
        for i in range(8):
            self._mem[(a + i) & _MEM_MASK] = (val >> (8 * i)) & _MASK8

    # ── Instruction fetch ──────────────────────────────────────────────────────

    def _fetch_word(self) -> int:
        """Fetch 32-bit little-endian instruction at current PC; advance PC.

        After fetch:
          RF.PC  = old nPC
          nPC    = old nPC + 4
        """
        pc = self._rf.read_pc()
        a = pc & _MEM_MASK
        iw = (
            self._mem[a]
            | (self._mem[(a + 1) & _MEM_MASK] << 8)
            | (self._mem[(a + 2) & _MEM_MASK] << 16)
            | (self._mem[(a + 3) & _MEM_MASK] << 24)
        )
        # Advance: PC ← nPC, nPC ← nPC + 4
        self._rf.write_pc(self._npc & _MEM_MASK)
        self._npc = (self._npc + 4) & _MEM_MASK
        return iw

    # ── Operate format decode ──────────────────────────────────────────────────

    def _decode_operate(self, iw: int) -> tuple[int, int, int, int]:
        """Decode Operate-format: return (ra_val, src_val, func7, rc_reg).

        When i_bit=0: src_val = Rb register value
        When i_bit=1: src_val = zero-extended 8-bit literal
        """
        ra   = (iw >> 21) & 0x1F
        i_b  = (iw >> 12) & 1
        func = (iw >> 5) & 0x7F
        rc   = iw & 0x1F
        if i_b:
            lit = (iw >> 13) & 0xFF
            return self._read_reg(ra), lit, func, rc
        rb = (iw >> 16) & 0x1F
        return self._read_reg(ra), self._read_reg(rb), func, rc

    # ── Top-level dispatch ─────────────────────────────────────────────────────

    def _execute_one(self) -> str:
        """Fetch and execute one instruction. Returns mnemonic string."""
        pc_of_instr = self._rf.read_pc()
        iw = self._fetch_word()
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
        if op in (0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
                  0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F):
            return self._exec_mem(iw, op, pc_of_instr)
        if 0x30 <= op <= 0x3F:
            return self._exec_branch(iw, op, pc_of_instr)
        raise ValueError(f"Unknown opcode 0x{op:02X} at PC=0x{pc_of_instr:04X}")

    # ── PALcode ────────────────────────────────────────────────────────────────

    def _exec_palcode(self, iw: int, pc_of_instr: int) -> str:
        """Handle call_pal instructions.  Only HALT (palcode=0) is implemented."""
        palcode = iw & 0x03FF_FFFF
        if palcode == 0:
            self._halted = True
            return "HALT"
        raise ValueError(f"Unsupported PALcode 0x{palcode:07X} at PC=0x{pc_of_instr:04X}")

    # ── INTA: Integer Arithmetic (op=0x10) ─────────────────────────────────────

    def _exec_inta(self, iw: int, pc_of_instr: int) -> str:
        """Integer arithmetic: ADD, SUB, scaled add/sub, compare."""
        a, src, func, rc = self._decode_operate(iw)

        # ── ADD longword ───────────────────────────────────────────────────────
        if func in (0x00, 0x40):    # ADDL, ADDLV
            self._write_reg(rc, addl(a, src).result)
            return "ADDL"
        # ── ADD quadword ───────────────────────────────────────────────────────
        if func in (0x20, 0x60):    # ADDQ, ADDQV
            self._write_reg(rc, addq(a, src).result)
            return "ADDQ"
        # ── SUB longword ───────────────────────────────────────────────────────
        if func in (0x09, 0x49):    # SUBL, SUBLV
            self._write_reg(rc, subl(a, src).result)
            return "SUBL"
        # ── SUB quadword ───────────────────────────────────────────────────────
        if func in (0x29, 0x69):    # SUBQ, SUBQV
            self._write_reg(rc, subq(a, src).result)
            return "SUBQ"
        # ── Scaled add/sub ─────────────────────────────────────────────────────
        if func == 0x02:   # S4ADDL
            self._write_reg(rc, s4addl(a, src).result)
            return "S4ADDL"
        if func == 0x22:   # S4ADDQ
            self._write_reg(rc, s4addq(a, src).result)
            return "S4ADDQ"
        if func == 0x0B:   # S4SUBL
            self._write_reg(rc, s4subl(a, src).result)
            return "S4SUBL"
        if func == 0x2B:   # S4SUBQ
            self._write_reg(rc, s4subq(a, src).result)
            return "S4SUBQ"
        if func == 0x12:   # S8ADDL
            self._write_reg(rc, s8addl(a, src).result)
            return "S8ADDL"
        if func == 0x32:   # S8ADDQ
            self._write_reg(rc, s8addq(a, src).result)
            return "S8ADDQ"
        if func == 0x1B:   # S8SUBL
            self._write_reg(rc, s8subl(a, src).result)
            return "S8SUBL"
        if func == 0x3B:   # S8SUBQ
            self._write_reg(rc, s8subq(a, src).result)
            return "S8SUBQ"
        # ── Compare (write 0 or 1 to Rc) ──────────────────────────────────────
        if func == 0x2D:   # CMPEQ
            self._write_reg(rc, cmpeq(a, src))
            return "CMPEQ"
        if func == 0x4D:   # CMPLT
            self._write_reg(rc, cmplt(a, src))
            return "CMPLT"
        if func == 0x6D:   # CMPLE
            self._write_reg(rc, cmple(a, src))
            return "CMPLE"
        if func == 0x3D:   # CMPULT
            self._write_reg(rc, cmpult(a, src))
            return "CMPULT"
        if func in (0x5D, 0x7D):   # CMPULE / CMPBGE (treat CMPBGE as CMPULE fallback)
            self._write_reg(rc, cmpule(a, src))
            return "CMPULE"
        if func == 0x4B:   # CMPBGE — byte-by-byte comparison
            return self._exec_cmpbge(a, src, rc)
        # Overflow variants of MUL (handled in INTM but sometimes mis-dispatched)
        if func in (0x18, 0x58):   # MULL, MULLV
            self._write_reg(rc, mull(a, src))
            return "MULL"
        if func in (0x38, 0x78):   # MULQ, MULQV
            self._write_reg(rc, mulq(a, src))
            return "MULQ"
        raise ValueError(f"Unknown INTA func 0x{func:02X} at PC=0x{pc_of_instr:04X}")

    def _exec_cmpbge(self, a: int, b: int, rc: int) -> str:
        """CMPBGE Ra,Rb,Rc — byte-by-byte unsigned comparison.

        For each byte i (0–7), set bit i of Rc if Ra[byte_i] >= Rb[byte_i].
        """
        result = 0
        for i in range(8):
            ra_byte = (a >> (i * 8)) & _MASK8
            rb_byte = (b >> (i * 8)) & _MASK8
            # unsigned >=: NOT(ra < rb) = NOT(cmpult)
            lt = NOT(cmpult(ra_byte, rb_byte))
            result |= lt << i
        self._write_reg(rc, result)
        return "CMPBGE"

    # ── INTL: Integer Logical (op=0x11) ───────────────────────────────────────

    def _exec_intl(self, iw: int, pc_of_instr: int) -> str:
        """Integer logical and conditional-move instructions."""
        a, src, func, rc = self._decode_operate(iw)
        cur_rc = self._read_reg(rc)

        if func == 0x00:   # AND
            self._write_reg(rc, andq(a, src).result)
            return "AND"
        if func == 0x08:   # BIC
            self._write_reg(rc, bicq(a, src).result)
            return "BIC"
        if func == 0x20:   # BIS (OR)
            self._write_reg(rc, orq(a, src).result)
            return "BIS"
        if func == 0x28:   # ORNOT
            self._write_reg(rc, ornot(a, src).result)
            return "ORNOT"
        if func == 0x40:   # XOR
            self._write_reg(rc, xorq(a, src).result)
            return "XOR"
        if func == 0x48:   # EQV (XNOR)
            self._write_reg(rc, eqvq(a, src).result)
            return "EQV"

        # ── Conditional moves ─────────────────────────────────────────────────
        # All CMOVs: if condition(Ra) then Rc←src else Rc unchanged.
        # We use gate-level tests: AND, NOT, compute_zero, sign bit extraction.

        if func == 0x14:   # CMOVLBS — if Ra[0]==1
            ra_bits = int_to_bits(a, 64)
            if AND(ra_bits[0], 1):
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVLBS"

        if func == 0x16:   # CMOVLBC — if Ra[0]==0
            ra_bits = int_to_bits(a, 64)
            if NOT(ra_bits[0]):
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVLBC"

        if func == 0x24:   # CMOVEQ — if Ra==0
            ra_bits = int_to_bits(a, 64)
            ra_zero = compute_zero(ra_bits)
            if ra_zero:
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVEQ"

        if func == 0x26:   # CMOVNE — if Ra!=0
            ra_bits = int_to_bits(a, 64)
            ra_zero = compute_zero(ra_bits)
            if NOT(ra_zero):
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVNE"

        if func == 0x44:   # CMOVLT — if signed Ra < 0 (bit 63 = 1)
            ra_bits = int_to_bits(a, 64)
            if ra_bits[63]:  # sign bit
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVLT"

        if func == 0x46:   # CMOVGE — if signed Ra >= 0 (bit 63 = 0)
            ra_bits = int_to_bits(a, 64)
            if NOT(ra_bits[63]):
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVGE"

        if func == 0x64:   # CMOVLE — if signed Ra <= 0 (bit63==1 OR Ra==0)
            ra_bits = int_to_bits(a, 64)
            ra_zero = compute_zero(ra_bits)
            is_neg = ra_bits[63]
            if OR(is_neg, ra_zero):
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVLE"

        if func == 0x66:   # CMOVGT — if signed Ra > 0 (bit63==0 AND Ra!=0)
            ra_bits = int_to_bits(a, 64)
            ra_zero = compute_zero(ra_bits)
            is_neg = ra_bits[63]
            if AND(NOT(is_neg), NOT(ra_zero)):
                self._write_reg(rc, src)
            else:
                self._write_reg(rc, cur_rc)
            return "CMOVGT"

        # AMASK: Rc = Ra & ~Rb (same as BIC)
        if func == 0x61:
            self._write_reg(rc, bicq(a, src).result)
            return "AMASK"

        # IMPLVER: Rc = 0 (report EV3 = oldest implementation)
        if func == 0x6C:
            self._write_reg(rc, 0)
            return "IMPLVER"

        raise ValueError(f"Unknown INTL func 0x{func:02X} at PC=0x{pc_of_instr:04X}")

    # ── INTS: Integer Shift and Byte Manipulation (op=0x12) ───────────────────

    def _exec_ints(self, iw: int, pc_of_instr: int) -> str:
        """Shift, ZAP, and byte manipulation instructions."""
        a, src, func, rc = self._decode_operate(iw)
        shift = src & 63     # shift amount (bits 5:0)
        boff  = (src & 7) * 8  # byte offset in bits

        if func == 0x39:   # SLL
            self._write_reg(rc, sll64(a, shift).result)
            return "SLL"
        if func == 0x34:   # SRL
            self._write_reg(rc, srl64(a, shift).result)
            return "SRL"
        if func == 0x3A:   # SRA
            self._write_reg(rc, sra64(a, shift).result)
            return "SRA"

        # ── ZAP / ZAPNOT ──────────────────────────────────────────────────────
        # ZAP:    zero byte i of Ra where bit i of src (low 8 bits) is 1
        # ZAPNOT: zero byte i of Ra where bit i of src is 0 (keep where 1)
        #
        # Gate-level: for each byte i in 0..7:
        #   ZAP:    result_byte = AND(ra_byte_bits[j], NOT(rb_control_bit))
        #   ZAPNOT: result_byte = AND(ra_byte_bits[j], rb_control_bit)

        if func == 0x30:   # ZAP
            a_bits = int_to_bits(a & _MASK64, 64)
            src_bits = int_to_bits(src & _MASK64, 64)
            result_bits = []
            for i in range(8):
                ctrl = src_bits[i]          # bit i controls byte i
                not_ctrl = NOT(ctrl)
                for j in range(8):
                    bit_pos = i * 8 + j
                    # zero byte where ctrl==1: result = AND(a_bit, NOT(ctrl))
                    result_bits.append(AND(a_bits[bit_pos], not_ctrl))
            self._write_reg(rc, bits_to_int(result_bits))
            return "ZAP"

        if func == 0x31:   # ZAPNOT
            a_bits = int_to_bits(a & _MASK64, 64)
            src_bits = int_to_bits(src & _MASK64, 64)
            result_bits = []
            for i in range(8):
                ctrl = src_bits[i]
                for j in range(8):
                    bit_pos = i * 8 + j
                    # keep byte where ctrl==1: result = AND(a_bit, ctrl)
                    result_bits.append(AND(a_bits[bit_pos], ctrl))
            self._write_reg(rc, bits_to_int(result_bits))
            return "ZAPNOT"

        # ── Extract byte/word/long/quad (right-aligned) ────────────────────────
        if func == 0x06:   # EXTBL
            self._write_reg(rc, (a >> boff) & _MASK8)
            return "EXTBL"
        if func == 0x16:   # EXTWL
            self._write_reg(rc, (a >> boff) & _MASK16)
            return "EXTWL"
        if func == 0x26:   # EXTLL
            self._write_reg(rc, (a >> boff) & _MASK32)
            return "EXTLL"
        if func == 0x36:   # EXTQL
            self._write_reg(rc, (a >> boff) & _MASK64)
            return "EXTQL"

        # ── Insert byte/word/long/quad ─────────────────────────────────────────
        if func == 0x0B:   # INSBL
            self._write_reg(rc, _u64((a & _MASK8) << boff))
            return "INSBL"
        if func == 0x1B:   # INSWL
            self._write_reg(rc, _u64((a & _MASK16) << boff))
            return "INSWL"
        if func == 0x2B:   # INSLL
            self._write_reg(rc, _u64((a & _MASK32) << boff))
            return "INSLL"
        if func == 0x3B:   # INSQL
            self._write_reg(rc, _u64(a << boff))
            return "INSQL"

        # ── Mask bytes ─────────────────────────────────────────────────────────
        if func == 0x02:   # MSKBL
            mask = _MASK8 << boff
            self._write_reg(rc, _u64(a & ~mask))
            return "MSKBL"
        if func == 0x12:   # MSKWL
            mask = _MASK16 << boff
            self._write_reg(rc, _u64(a & ~mask))
            return "MSKWL"
        if func == 0x22:   # MSKLL
            mask = _MASK32 << boff
            self._write_reg(rc, _u64(a & ~mask))
            return "MSKLL"
        if func == 0x32:   # MSKQL
            mask = _MASK64 << boff
            self._write_reg(rc, _u64(a & ~mask))
            return "MSKQL"

        # ── Sign extend ────────────────────────────────────────────────────────
        if func == 0x00:   # SEXTB
            v = a & _MASK8
            if v >= 0x80:
                v -= 0x100
            self._write_reg(rc, _u64(v))
            return "SEXTB"
        if func == 0x01:   # SEXTW
            v = a & _MASK16
            if v >= 0x8000:
                v -= 0x10000
            self._write_reg(rc, _u64(v))
            return "SEXTW"

        # Extract high variants
        if func == 0x3C:   # SRA (alternate encoding in some assemblers)
            self._write_reg(rc, sra64(a, shift).result)
            return "SRA"

        raise ValueError(f"Unknown INTS func 0x{func:02X} at PC=0x{pc_of_instr:04X}")

    # ── INTM: Integer Multiply (op=0x13) ───────────────────────────────────────

    def _exec_intm(self, iw: int, pc_of_instr: int) -> str:
        """Integer multiply: MULL, MULQ, UMULH."""
        a, src, func, rc = self._decode_operate(iw)

        if func in (0x00, 0x40):   # MULL, MULLV
            self._write_reg(rc, mull(a, src))
            return "MULL"
        if func in (0x20, 0x60):   # MULQ, MULQV
            self._write_reg(rc, mulq(a, src))
            return "MULQ"
        if func == 0x30:           # UMULH
            self._write_reg(rc, umulh(a, src))
            return "UMULH"
        raise ValueError(f"Unknown INTM func 0x{func:02X} at PC=0x{pc_of_instr:04X}")

    # ── Memory loads and stores ────────────────────────────────────────────────

    def _exec_mem(self, iw: int, op: int, pc_of_instr: int) -> str:
        """Memory load and store instructions.

        Memory format: [op:6][Ra:5][Rb:5][disp16:16]
          ea = Rb + sext16(disp16)
        """
        ra  = (iw >> 21) & 0x1F
        rb  = (iw >> 16) & 0x1F
        d16 = iw & 0xFFFF
        base = self._read_reg(rb)
        ea = (base + _sext16_int(d16)) & _MEM_MASK

        # ── Loads ─────────────────────────────────────────────────────────────
        if op == 0x08:   # LDA: Ra = Rb + sext16(disp) — no memory access
            self._write_reg(ra, (base + _sext16_int(d16)) & _MASK64)
            return "LDA"
        if op == 0x09:   # LDAH: Ra = Rb + sext16(disp) * 65536
            self._write_reg(ra, _u64(base + _sext16_int(d16) * 65536))
            return "LDAH"
        if op in (0x28, 0x2A):   # LDL, LDL_L — sign-extend 32-bit
            raw = self._read_long(ea)
            self._write_reg(ra, _u64(_sext32_int(raw)))
            return "LDL"
        if op in (0x29, 0x2B):   # LDQ, LDQ_L
            self._write_reg(ra, self._read_quad(ea))
            return "LDQ"
        if op == 0x0A:   # LDBU — byte unsigned
            self._write_reg(ra, self._read_byte(ea))
            return "LDBU"
        if op == 0x0B:   # LDQ_U — unaligned quadword
            self._write_reg(ra, self._read_quad_unaligned(ea))
            return "LDQ_U"

        # ── Stores ────────────────────────────────────────────────────────────
        src = self._read_reg(ra)
        if op in (0x2C, 0x2E):   # STL, STL_C
            self._write_long(ea, src & _MASK32)
            if op == 0x2E:   # STL_C: always succeeds → Ra = 1
                self._write_reg(ra, 1)
            return "STL"
        if op in (0x2D, 0x2F):   # STQ, STQ_C
            self._write_quad(ea, src)
            if op == 0x2F:   # STQ_C: always succeeds → Ra = 1
                self._write_reg(ra, 1)
            return "STQ"
        if op == 0x0F:   # STQ_U — unaligned
            self._write_quad_unaligned(ea, src)
            return "STQ_U"

        raise ValueError(f"Unknown memory op 0x{op:02X} at PC=0x{pc_of_instr:04X}")

    # ── Branch instructions ────────────────────────────────────────────────────

    def _exec_branch(self, iw: int, op: int, pc_of_instr: int) -> str:
        """Branch instructions.

        Branch format: [op:6][Ra:5][disp21:21]
        Target = (pc_of_instr + 4) + sext21(disp21) * 4

        After _fetch_word(), PC has already advanced to pc_of_instr + 4.
        So target = current_pc + sext21(disp21) * 4.
        """
        ra     = (iw >> 21) & 0x1F
        disp21 = iw & 0x1F_FFFF
        # Sign-extend disp21
        disp21_s = disp21 - 0x20_0000 if disp21 >= 0x10_0000 else disp21
        # PC is already at pc_of_instr+4 after fetch; target is relative to instr+4.
        target = (pc_of_instr + 4 + disp21_s * 4) & _MEM_MASK
        val = self._read_reg(ra)

        taken = False
        mnemonic = "BR"

        if op == 0x30:   # BR — unconditional
            taken, mnemonic = True, "BR"
        elif op == 0x34:   # BSR — branch and save return address
            self._write_reg(ra, pc_of_instr + 4)
            taken, mnemonic = True, "BSR"
        elif op in (0x31, 0x32, 0x33, 0x35, 0x36, 0x37):
            # Floating-point branches: treat as NOP (not taken)
            taken = False
            mnemonic = {0x31: "FBEQ", 0x32: "FBLT", 0x33: "FBLE",
                        0x35: "FBNE", 0x36: "FBGE", 0x37: "FBGT"}[op]
        elif op == 0x39:   # BEQ — branch if Ra == 0
            val_bits = int_to_bits(val, 64)
            taken = bool(compute_zero(val_bits))
            mnemonic = "BEQ"
        elif op == 0x3D:   # BNE — branch if Ra != 0
            val_bits = int_to_bits(val, 64)
            taken = bool(NOT(compute_zero(val_bits)))
            mnemonic = "BNE"
        elif op == 0x3A:   # BLT — branch if signed Ra < 0 (bit 63 = 1)
            val_bits = int_to_bits(val, 64)
            taken = bool(val_bits[63])
            mnemonic = "BLT"
        elif op == 0x3B:   # BLE — branch if signed Ra <= 0 (bit63==1 OR Ra==0)
            val_bits = int_to_bits(val, 64)
            taken = bool(OR(val_bits[63], compute_zero(val_bits)))
            mnemonic = "BLE"
        elif op == 0x3F:   # BGT — branch if signed Ra > 0 (bit63==0 AND Ra!=0)
            val_bits = int_to_bits(val, 64)
            taken = bool(AND(NOT(val_bits[63]), NOT(compute_zero(val_bits))))
            mnemonic = "BGT"
        elif op == 0x3E:   # BGE — branch if signed Ra >= 0 (bit63==0 OR Ra==0)
            val_bits = int_to_bits(val, 64)
            taken = bool(NOT(val_bits[63]))
            mnemonic = "BGE"
        elif op == 0x38:   # BLBC — branch if Ra[0] == 0
            val_bits = int_to_bits(val, 64)
            taken = bool(NOT(val_bits[0]))
            mnemonic = "BLBC"
        elif op == 0x3C:   # BLBS — branch if Ra[0] == 1
            val_bits = int_to_bits(val, 64)
            taken = bool(val_bits[0])
            mnemonic = "BLBS"

        if taken:
            self._rf.write_pc(target)
            self._npc = (target + 4) & _MEM_MASK

        return mnemonic

    # ── Jump instructions (op=0x1A) ────────────────────────────────────────────

    def _exec_jump(self, iw: int, pc_of_instr: int) -> str:
        """Jump instructions: JMP, JSR, RET, JSR_COROUTINE.

        Jump format: [0x1A:6][Ra:5][Rb:5][func:2][hint:14]
        All variants: PC = Rb & ~3
        """
        ra   = (iw >> 21) & 0x1F
        rb   = (iw >> 16) & 0x1F
        func = (iw >> 14) & 0x3
        link = (pc_of_instr + 4) & _MASK64
        target = self._read_reg(rb) & ~3 & _MEM_MASK

        self._rf.write_pc(target)
        self._npc = (target + 4) & _MEM_MASK

        mnemonics = {0: "JMP", 1: "JSR", 2: "RET", 3: "JSR_COROUTINE"}
        mnemonic = mnemonics.get(func, "JMP")

        # All jump variants write link to Ra (RET also writes, but Ra is r31)
        self._write_reg(ra, link)
        return mnemonic
