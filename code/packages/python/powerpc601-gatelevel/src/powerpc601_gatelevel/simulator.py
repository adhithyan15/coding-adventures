"""simulator.py — Gate-level PowerPC 601 simulator.

This is the top-level integration module.  It composes:
  - RegisterFilePPC  — 32 GPRs, LR, CTR, XER, CR, CIA stored as bit lists
  - decode_instruction — pure combinational instruction decode
  - alu.py functions  — all data-path ops route through gate primitives

Every integer arithmetic/logic operation on register values routes through
gate functions (AND, OR, XOR, NOT) and ripple_carry_adder — NO Python
operators (+, -, &, |, ^, ~, *, /) in the execution path for ALU/register
operations on register values.

Python arithmetic IS used for:
  - Memory address computation (ea = base + displacement)
  - Memory indexing (self._mem[addr])
  - Loop control (for i in range(N), range(31,-1,-1))
  - PC alignment (target & ~3)
  - Byte shift counts in big-endian memory packing (<<24, <<16, <<8)

These are host-machine bookkeeping operations, not simulated data-path ops.

Execution model
───────────────
Each call to step():
  1. Check if halted
  2. Fetch 4 bytes from memory[CIA..CIA+3] as big-endian 32-bit word
  3. Advance CIA by 4 (via gate-level increment)
  4. Decode instruction using decode_instruction()
  5. Dispatch to instruction handler
  6. Return StepTrace(pc_before, pc_after, mnemonic, description)

Memory layout
─────────────
64 KiB flat, big-endian.  The program is loaded starting at origin (default 0).
Uninitialized memory is zero (decodes as HALT — a safety net).

CR0 update (Rc bit)
───────────────────
When Rc=1 on arithmetic/logical instructions, CR0 is updated:
  LT = sign bit of result (bit 31)
  GT = not zero AND not negative (result > 0 in signed interpretation)
  EQ = all-zero flag
  SO = current XER[SO] bit

XER flag bits (Python bit weights)
───────────────────────────────────
  XER_SO = 1 << 31  (bit 31 in our Python int representation)
  XER_OV = 1 << 30
  XER_CA = 1 << 29

BO field decoding
─────────────────
BO[4:0] (5 bits, with BO[0] = bit 4 = the most significant):
  BO[0] = (bo >> 4) & 1  — 0: test CTR;  1: don't test CTR
  BO[1] = (bo >> 3) & 1  — 0: CTR != 0;  1: CTR == 0
  BO[2] = (bo >> 2) & 1  — 0: test CR[BI]; 1: don't test CR
  BO[3] = (bo >> 1) & 1  — 1: branch if CR[BI]=1; 0: branch if CR[BI]=0
  BO[4] = bo & 1         — branch prediction hint (ignored)
"""

from __future__ import annotations

from logic_gates import AND, NOT, OR, XOR
from powerpc601_simulator.state import (
    MASK32,
    MEM_SIZE,
    PowerPC601State,
)
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .alu import (
    add32,
    and32,
    andc32,
    cmp32,
    cmpl32,
    cntlzw,
    divw,
    divwu,
    eqv32,
    mul32_hi_signed,
    mul32_hi_unsigned,
    mul32_lo,
    nand32,
    nor32,
    or32,
    orc32,
    rotl32,
    sll32,
    sra32,
    srl32,
    xor32,
)
from .bits import (
    add_32bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_32bit,
)
from .decoder import decode_instruction
from .register_file import RegisterFilePPC

# ── Memory constants ──────────────────────────────────────────────────────────

_MEM_MASK: int = MEM_SIZE - 1   # 0xFFFF for 64 KiB
_MASK32: int = MASK32            # 0xFFFFFFFF


def _mask_addr(addr: int) -> int:
    """Mask address to memory range (address arithmetic)."""
    return addr & _MEM_MASK


# ── Mask generation for RLWINM/RLWIMI/RLWNM ──────────────────────────────────


def _mask_from_mb_me(mb: int, me: int) -> int:
    """Generate a 32-bit mask from MB (mask begin) and ME (mask end).

    In PowerPC, mask bit i is included if MB <= i <= ME (wrapping if MB > ME).
    Bit numbering: bit 0 = MSB (leftmost, bit 31 in Python), bit 31 = LSB.

    This is address/control logic, not a data-path operation.
    """
    # Convert PPC bit numbering (0=MSB) to Python bit weights
    mask = 0
    for i in range(32):
        # i is PPC bit number (0=MSB, 31=LSB)
        included = mb <= i <= me if mb <= me else i >= mb or i <= me
        if included:
            # PPC bit i has Python weight 2^(31 - i)
            mask |= 1 << (31 - i)
    return mask


# ── CR update helper ──────────────────────────────────────────────────────────


def _update_cr0_from_result(result: int, xer_so: int, rf: RegisterFilePPC) -> None:
    """Update CR0 based on a 32-bit arithmetic/logical result.

    CR0 bits:
      LT = bit 31 of result (sign bit)
      GT = NOT(zero) AND NOT(LT) — strictly positive
      EQ = all-zero flag
      SO = current XER[SO] bit

    Uses gate-level compute_zero, AND, NOT, OR.
    """
    result_bits = int_to_bits(result & _MASK32, 32)
    lt = result_bits[31]   # sign bit
    zero = compute_zero(result_bits)
    gt = AND(NOT(zero), NOT(lt))
    so = xer_so
    rf.set_cr_field(0, lt, gt, zero, so)


# ── Branch evaluation ─────────────────────────────────────────────────────────


def _eval_branch(bo: int, bi: int, ctr: int, cr: int) -> tuple[bool, int]:
    """Evaluate a conditional branch.

    Returns (should_branch, new_ctr).

    BO[4:0] layout (bo is a 5-bit integer extracted from instruction):
      (bo >> 4) & 1 = 0: test CTR; 1: ignore CTR
      (bo >> 3) & 1 = 0: branch if CTR != 0; 1: branch if CTR == 0
      (bo >> 2) & 1 = 0: test CR[BI]; 1: ignore CR
      (bo >> 1) & 1 = 1: branch if CR[BI] = 1; 0: branch if CR[BI] = 0
      (bo)      & 1 = branch prediction hint (ignored)
    """
    bo0 = (bo >> 4) & 1  # don't test CTR if 1
    bo1 = (bo >> 3) & 1  # CTR condition: 0=CTR!=0, 1=CTR==0
    bo2 = (bo >> 2) & 1  # don't test CR if 1
    bo3 = (bo >> 1) & 1  # CR condition: 1=branch-if-1, 0=branch-if-0

    new_ctr = ctr
    ctr_ok = True
    if bo0 == 0:
        # Decrement CTR: use gate-level subtract (CTR - 1 = CTR + NOT(1) + 1 for 32-bit)
        new_ctr, _carry, _ov = add_32bit(ctr & _MASK32, invert_32bit(1), 1)
        new_ctr = new_ctr & _MASK32
        ctr_nonzero = NOT(compute_zero(int_to_bits(new_ctr, 32)))
        # bo1=0: CTR!=0; bo1=1: CTR==0
        ctr_ok = bool(ctr_nonzero) if bo1 == 0 else not bool(ctr_nonzero)

    cr_ok = True
    if bo2 == 0:
        # Get CR bit BI (BI=0 → CR bit 31 in Python, i.e., MSB)
        cr_bit = (cr >> (31 - bi)) & 1
        cr_ok = bool(cr_bit) == bool(bo3)

    return ctr_ok and cr_ok, new_ctr


# ── Main simulator class ──────────────────────────────────────────────────────


class PowerPC601GateLevelSimulator(Simulator[PowerPC601State]):
    """Gate-level PowerPC 601 simulator implementing Simulator[PowerPC601State].

    Every 32-bit ALU operation (ADD, SUB, AND, OR, XOR, NOT, shifts,
    multiply, divide) routes through logic gate primitives from the
    logic_gates and arithmetic packages.

    State is tracked internally as a RegisterFilePPC (bit lists) + memory
    bytearray.  get_state() synthesizes an immutable PowerPC601State snapshot.

    Example
    ───────
    >>> import struct
    >>> sim = PowerPC601GateLevelSimulator()
    >>> # ADDI r3, 0, 42; HALT
    >>> prog = struct.pack(">II", (14<<26)|(3<<21)|(0<<16)|42, 0)
    >>> result = sim.execute(prog)
    >>> result.final_state.gpr[3]
    42
    """

    def __init__(self) -> None:
        self._rf = RegisterFilePPC()
        self._mem: bytearray = bytearray(MEM_SIZE)
        self._halted: bool = False

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers, memory, CIA, and halted flag."""
        self._rf.reset()
        self._mem = bytearray(MEM_SIZE)
        self._halted = False

    def load(self, program: bytes, origin: int = 0) -> None:
        """Reset the simulator and copy program bytes into memory at origin.

        Parameters
        ──────────
        program : bytes to load
        origin  : start address (default 0)
        """
        self.reset()
        self._rf.write_cia(origin)
        for i, b in enumerate(program):
            addr = origin + i  # address arithmetic
            if addr >= MEM_SIZE:
                break
            self._mem[addr] = b

    def step(self) -> StepTrace:
        """Fetch, decode, and execute one instruction at CIA.

        Returns a StepTrace with the PC before/after, mnemonic, and description.
        If already halted, returns a HALT trace without advancing CIA.
        """
        cia = self._rf.read_cia()

        if self._halted:
            return StepTrace(
                pc_before=cia,
                pc_after=cia,
                mnemonic="HALT",
                description="Simulator is halted.",
            )

        word = self._fetch_word(cia)

        if word == 0:
            self._halted = True
            return StepTrace(
                pc_before=cia,
                pc_after=cia,
                mnemonic="HALT",
                description=f"HALT at CIA=0x{cia:04X}.",
            )

        # CIA is already advanced in _fetch_word (by 4)
        return self._execute(word, cia)

    def execute(
        self, program: bytes, origin: int = 0, max_steps: int = 100_000
    ) -> ExecutionResult[PowerPC601State]:
        """Load program and step until halted or max_steps exceeded."""
        self.load(program, origin)
        traces: list[StepTrace] = []
        for _ in range(max_steps):
            trace = self.step()
            traces.append(trace)
            if trace.mnemonic.startswith("ERROR:"):
                return ExecutionResult(
                    halted=False,
                    steps=len(traces),
                    final_state=self.get_state(),
                    traces=traces,
                    error=trace.mnemonic,
                )
            if self._halted:
                return ExecutionResult(
                    halted=True,
                    steps=len(traces),
                    final_state=self.get_state(),
                    traces=traces,
                    error=None,
                )
        return ExecutionResult(
            halted=False,
            steps=max_steps,
            final_state=self.get_state(),
            traces=traces,
            error=f"max_steps={max_steps} exceeded",
        )

    def get_state(self) -> PowerPC601State:
        """Return an immutable snapshot of the current simulator state."""
        return PowerPC601State(
            cia=self._rf.read_cia(),
            gpr=self._rf.get_gprs_tuple(),
            lr=self._rf.read_lr(),
            ctr=self._rf.read_ctr(),
            xer=self._rf.read_xer(),
            cr=self._rf.read_cr(),
            memory=tuple(self._mem),
            halted=self._halted,
        )

    def set_input_port(self, port: int, value: int) -> None:
        """No-op: PowerPC 601 has no I/O ports."""

    def get_output_port(self, port: int) -> int:
        """No-op: PowerPC 601 has no I/O ports."""
        return 0

    def interrupt(self) -> None:
        """No-op: interrupts not simulated."""

    def nmi(self) -> None:
        """No-op: NMI not simulated."""

    # ── Memory helpers ─────────────────────────────────────────────────────────

    def _fetch_word(self, cia: int) -> int:
        """Read a big-endian 32-bit word from memory at cia, then advance CIA by 4.

        Memory layout: big-endian, so byte 0 is most significant.
        mem[cia]   = bits [31:24]
        mem[cia+1] = bits [23:16]
        mem[cia+2] = bits [15:8]
        mem[cia+3] = bits [7:0]

        CIA is incremented by 4 using gate-level add_32bit.
        """
        a = _mask_addr(cia)
        word = (self._mem[a] << 24) | (self._mem[a + 1] << 16) | \
               (self._mem[a + 2] << 8) | self._mem[a + 3]
        self._rf.increment_cia(4)
        return word

    def _load32(self, addr: int) -> int:
        """Load a big-endian 32-bit word (word-aligned)."""
        a = addr & ~3 & _MEM_MASK
        return (self._mem[a] << 24) | (self._mem[a + 1] << 16) | \
               (self._mem[a + 2] << 8) | self._mem[a + 3]

    def _load16z(self, addr: int) -> int:
        """Load a big-endian 16-bit halfword (zero-extended)."""
        a = addr & ~1 & _MEM_MASK
        return (self._mem[a] << 8) | self._mem[a + 1]

    def _load16a(self, addr: int) -> int:
        """Load a big-endian 16-bit halfword (sign-extended to 32 bits)."""
        v = self._load16z(addr)
        if v & 0x8000:
            return (v - 0x10000) & _MASK32
        return v

    def _load8(self, addr: int) -> int:
        """Load a single byte."""
        return self._mem[addr & _MEM_MASK]

    def _store32(self, addr: int, val: int) -> None:
        """Store a big-endian 32-bit word (word-aligned)."""
        a = addr & ~3 & _MEM_MASK
        val = val & _MASK32
        self._mem[a]     = (val >> 24) & 0xFF
        self._mem[a + 1] = (val >> 16) & 0xFF
        self._mem[a + 2] = (val >> 8)  & 0xFF
        self._mem[a + 3] =  val        & 0xFF

    def _store16(self, addr: int, val: int) -> None:
        """Store a big-endian 16-bit halfword."""
        a = addr & ~1 & _MEM_MASK
        val = val & 0xFFFF
        self._mem[a]     = (val >> 8) & 0xFF
        self._mem[a + 1] =  val       & 0xFF

    def _store8(self, addr: int, val: int) -> None:
        """Store a single byte."""
        self._mem[addr & _MEM_MASK] = val & 0xFF

    # ── Effective address helpers ──────────────────────────────────────────────

    def _ea(self, ra: int, d: int) -> int:
        """Compute effective address for D-form load/store.

        If rA = 0, the base is 0 (not GPR[0]).
        d is the sign-extended 16-bit displacement.
        """
        base = 0 if ra == 0 else self._rf.read_gpr(ra)
        return (base + d) & _MASK32

    def _ea_x(self, ra: int, rb: int) -> int:
        """Compute effective address for X-form indexed load/store.

        If rA = 0, base is 0; otherwise GPR[rA].
        """
        base = 0 if ra == 0 else self._rf.read_gpr(ra)
        rb_val = self._rf.read_gpr(rb)
        return (base + rb_val) & _MASK32

    # ── XER helpers ────────────────────────────────────────────────────────────

    def _xer_get_ca(self) -> int:
        """Get XER[CA] bit (1 or 0)."""
        xer = self._rf.read_xer()
        return (xer >> 29) & 1

    def _xer_set_ca(self, ca: int) -> None:
        """Set XER[CA] bit without changing other XER bits."""
        xer = self._rf.read_xer()
        # AND with NOT(XER_CA) to clear, then OR with CA<<29 to set
        xer_bits = int_to_bits(xer, 32)
        ca_bit_pos = 29  # bit 29 in Python int = XER_CA
        xer_bits[ca_bit_pos] = AND(ca, 1)
        self._rf.write_xer(bits_to_int(xer_bits))

    def _xer_get_so(self) -> int:
        """Get XER[SO] bit."""
        xer = self._rf.read_xer()
        return (xer >> 31) & 1

    def _xer_set_ov_so(self, ov: int) -> None:
        """Set XER[OV] and OR into XER[SO] (once set, SO stays until cleared)."""
        xer = self._rf.read_xer()
        xer_bits = int_to_bits(xer, 32)
        xer_bits[30] = AND(ov, 1)   # OV bit
        xer_bits[31] = OR(xer_bits[31], AND(ov, 1))  # SO: sticky
        self._rf.write_xer(bits_to_int(xer_bits))

    # ── CR0 update ─────────────────────────────────────────────────────────────

    def _update_cr0(self, result: int) -> None:
        """Update CR0 from arithmetic/logical result (called when Rc=1)."""
        so = self._xer_get_so()
        _update_cr0_from_result(result, so, self._rf)

    # ── CR field update for compare ────────────────────────────────────────────

    def _set_cr_cmp(self, field: int, lt: int, gt: int, eq: int) -> None:
        """Set a CR compare field with SO from XER."""
        so = self._xer_get_so()
        self._rf.set_cr_field(field, lt, gt, eq, so)

    # ── Instruction dispatch ───────────────────────────────────────────────────

    def _execute(self, word: int, cia: int) -> StepTrace:
        """Decode and execute one instruction word."""
        dec = decode_instruction(word)
        op = dec["op"]

        cia_after = self._rf.read_cia()  # already incremented by _fetch_word

        dispatch = {
            14: self._exec_addi,
            15: self._exec_addis,
            8:  self._exec_subfic,
            12: self._exec_addic,
            13: self._exec_addic_dot,
            11: self._exec_cmpi,
            10: self._exec_cmpli,
            24: self._exec_ori,
            25: self._exec_oris,
            26: self._exec_xori,
            27: self._exec_xoris,
            28: self._exec_andi_dot,
            29: self._exec_andis_dot,
            20: self._exec_rlwimi,
            21: self._exec_rlwinm,
            23: self._exec_rlwnm,
            18: self._exec_b,
            16: self._exec_bc,
            19: self._exec_bx,
            31: self._exec_x31,
            32: self._exec_lwz,
            33: self._exec_lwzu,
            34: self._exec_lbz,
            35: self._exec_lbzu,
            36: self._exec_stw,
            37: self._exec_stwu,
            38: self._exec_stb,
            39: self._exec_stbu,
            40: self._exec_lhz,
            41: self._exec_lhzu,
            42: self._exec_lha,
            43: self._exec_lhau,
            44: self._exec_sth,
            45: self._exec_sthu,
            46: self._exec_lmw,
            47: self._exec_stmw,
        }

        handler = dispatch.get(op)
        if handler:
            return handler(dec, cia, cia_after)

        # Unknown opcode
        self._halted = True
        return StepTrace(
            pc_before=cia,
            pc_after=cia,
            mnemonic=f"ERROR: unknown opcode {op}",
            description=f"Unknown primary opcode {op} at CIA=0x{cia:04X}.",
        )

    # ── D-form arithmetic ──────────────────────────────────────────────────────

    def _exec_addi(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ADDI rD, rA, SIMM — rD = (rA==0 ? 0 : rA) + SIMM."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        base = 0 if ra == 0 else self._rf.read_gpr(ra)
        # Gate-level add: use add_32bit
        simm_u = simm & _MASK32
        result, _carry, _ov = add_32bit(base & _MASK32, simm_u, 0)
        self._rf.write_gpr(rd, result)
        return StepTrace(cia, cia_after,
                         f"addi r{rd}, r{ra}, {simm}",
                         f"r{rd} = 0x{result:08X}")

    def _exec_addis(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ADDIS rD, rA, SIMM — rD = (rA==0 ? 0 : rA) + (SIMM << 16)."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        base = 0 if ra == 0 else self._rf.read_gpr(ra)
        shifted_imm = (simm & _MASK32) << 16 & _MASK32
        result, _carry, _ov = add_32bit(base & _MASK32, shifted_imm, 0)
        self._rf.write_gpr(rd, result)
        return StepTrace(cia, cia_after,
                         f"addis r{rd}, r{ra}, {simm}",
                         f"r{rd} = 0x{result:08X}")

    def _exec_subfic(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """SUBFIC rD, rA, SIMM — rD = SIMM - rA; set XER[CA]."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ra_val = self._rf.read_gpr(ra)
        simm_u = simm & _MASK32
        # NOT(rA) + SIMM + 1
        not_ra = invert_32bit(ra_val)
        result_r = add32(not_ra, simm_u, carry_in=1)
        self._rf.write_gpr(rd, result_r.result)
        self._xer_set_ca(result_r.carry)
        return StepTrace(cia, cia_after,
                         f"subfic r{rd}, r{ra}, {simm}",
                         f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

    def _exec_addic(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ADDIC rD, rA, SIMM — rD = rA + SIMM; set XER[CA]."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ra_val = self._rf.read_gpr(ra)
        result_r = add32(ra_val, simm & _MASK32)
        self._rf.write_gpr(rd, result_r.result)
        self._xer_set_ca(result_r.carry)
        return StepTrace(cia, cia_after,
                         f"addic r{rd}, r{ra}, {simm}",
                         f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

    def _exec_addic_dot(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ADDIC. rD, rA, SIMM — same as ADDIC but also sets CR0."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ra_val = self._rf.read_gpr(ra)
        result_r = add32(ra_val, simm & _MASK32)
        self._rf.write_gpr(rd, result_r.result)
        self._xer_set_ca(result_r.carry)
        self._update_cr0(result_r.result)
        return StepTrace(cia, cia_after,
                         f"addic. r{rd}, r{ra}, {simm}",
                         f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

    def _exec_cmpi(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """CMPI crfD, rA, SIMM — signed compare, update CR field."""
        ra, simm, crfd = dec["ra"], dec["simm"], dec["crfd"]
        ra_val = self._rf.read_gpr(ra)
        lt, gt, eq = cmp32(ra_val, simm & _MASK32)
        self._set_cr_cmp(crfd, lt, gt, eq)
        cr = self._rf.read_cr()
        return StepTrace(cia, cia_after,
                         f"cmpi cr{crfd}, r{ra}, {simm}",
                         f"CR{crfd} = 0x{(cr >> (28 - crfd * 4)) & 0xF:X}")

    def _exec_cmpli(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """CMPLI crfD, rA, UIMM — unsigned compare, update CR field."""
        ra, uimm, crfd = dec["ra"], dec["uimm"], dec["crfd"]
        ra_val = self._rf.read_gpr(ra)
        lt, gt, eq = cmpl32(ra_val, uimm)
        self._set_cr_cmp(crfd, lt, gt, eq)
        cr = self._rf.read_cr()
        return StepTrace(cia, cia_after,
                         f"cmpli cr{crfd}, r{ra}, {uimm}",
                         f"CR{crfd} = 0x{(cr >> (28 - crfd * 4)) & 0xF:X}")

    def _exec_ori(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ORI rA, rS, UIMM — rA = rS | UIMM."""
        rs, ra, uimm = dec["rd"], dec["ra"], dec["uimm"]
        result = or32(self._rf.read_gpr(rs), uimm).result
        self._rf.write_gpr(ra, result)
        return StepTrace(cia, cia_after, f"ori r{ra}, r{rs}, {uimm}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_oris(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ORIS rA, rS, UIMM — rA = rS | (UIMM << 16)."""
        rs, ra, uimm = dec["rd"], dec["ra"], dec["uimm"]
        result = or32(self._rf.read_gpr(rs), uimm << 16).result
        self._rf.write_gpr(ra, result)
        return StepTrace(cia, cia_after, f"oris r{ra}, r{rs}, {uimm}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_xori(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """XORI rA, rS, UIMM — rA = rS ^ UIMM."""
        rs, ra, uimm = dec["rd"], dec["ra"], dec["uimm"]
        result = xor32(self._rf.read_gpr(rs), uimm).result
        self._rf.write_gpr(ra, result)
        return StepTrace(cia, cia_after, f"xori r{ra}, r{rs}, {uimm}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_xoris(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """XORIS rA, rS, UIMM — rA = rS ^ (UIMM << 16)."""
        rs, ra, uimm = dec["rd"], dec["ra"], dec["uimm"]
        result = xor32(self._rf.read_gpr(rs), uimm << 16).result
        self._rf.write_gpr(ra, result)
        return StepTrace(cia, cia_after, f"xoris r{ra}, r{rs}, {uimm}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_andi_dot(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ANDI. rA, rS, UIMM — rA = rS & UIMM; always sets CR0."""
        rs, ra, uimm = dec["rd"], dec["ra"], dec["uimm"]
        result = and32(self._rf.read_gpr(rs), uimm).result
        self._rf.write_gpr(ra, result)
        self._update_cr0(result)
        return StepTrace(cia, cia_after, f"andi. r{ra}, r{rs}, {uimm}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_andis_dot(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """ANDIS. rA, rS, UIMM — rA = rS & (UIMM << 16); always sets CR0."""
        rs, ra, uimm = dec["rd"], dec["ra"], dec["uimm"]
        result = and32(self._rf.read_gpr(rs), uimm << 16).result
        self._rf.write_gpr(ra, result)
        self._update_cr0(result)
        return StepTrace(cia, cia_after, f"andis. r{ra}, r{rs}, {uimm}",
                         f"r{ra} = 0x{result:08X}")

    # ── Rotate/mask instructions ───────────────────────────────────────────────

    def _exec_rlwimi(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """RLWIMI rA, rS, SH, MB, ME — rotate left, insert using mask."""
        rs, ra, sh, mb, me, rc = dec["rd"], dec["ra"], dec["rb"], dec["mb"], dec["me"], dec["rc"]
        src = self._rf.read_gpr(rs)
        ra_val = self._rf.read_gpr(ra)
        rotated = rotl32(src, sh).result
        mask = _mask_from_mb_me(mb, me)
        # rA = (rotated & mask) | (rA & ~mask)
        r_and_m = and32(rotated, mask).result
        ra_and_not_m = and32(ra_val, invert_32bit(mask)).result
        result = or32(r_and_m, ra_and_not_m).result
        self._rf.write_gpr(ra, result)
        if rc:
            self._update_cr0(result)
        return StepTrace(cia, cia_after,
                         f"rlwimi r{ra}, r{rs}, {sh}, {mb}, {me}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_rlwinm(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """RLWINM[.] rA, rS, SH, MB, ME — rotate left, AND with mask."""
        rs, ra, sh, mb, me, rc = dec["rd"], dec["ra"], dec["rb"], dec["mb"], dec["me"], dec["rc"]
        src = self._rf.read_gpr(rs)
        rotated = rotl32(src, sh).result
        mask = _mask_from_mb_me(mb, me)
        result = and32(rotated, mask).result
        self._rf.write_gpr(ra, result)
        if rc:
            self._update_cr0(result)
        return StepTrace(cia, cia_after,
                         f"rlwinm r{ra}, r{rs}, {sh}, {mb}, {me}",
                         f"r{ra} = 0x{result:08X}")

    def _exec_rlwnm(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """RLWNM[.] rA, rS, rB, MB, ME — rotate left by register, AND with mask."""
        rs, ra, rb, mb, me, rc = dec["rd"], dec["ra"], dec["rb"], dec["mb"], dec["me"], dec["rc"]
        src = self._rf.read_gpr(rs)
        shamt = self._rf.read_gpr(rb) & 31
        rotated = rotl32(src, shamt).result
        mask = _mask_from_mb_me(mb, me)
        result = and32(rotated, mask).result
        self._rf.write_gpr(ra, result)
        if rc:
            self._update_cr0(result)
        return StepTrace(cia, cia_after,
                         f"rlwnm r{ra}, r{rs}, r{rb}, {mb}, {me}",
                         f"r{ra} = 0x{result:08X}")

    # ── Branch instructions ────────────────────────────────────────────────────

    def _exec_b(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """B[L][A] — branch, optionally set LR, optionally absolute."""
        li, aa, lk = dec["li"], dec["aa"], dec["lk"]
        if lk:
            self._rf.write_lr(cia_after)  # CIA+4 already in cia_after
        target = li & _MASK32 if aa else (cia + li) & _MASK32
        self._rf.write_cia(target)
        mn = "bl" if lk else "b"
        return StepTrace(cia, target, f"{mn} 0x{target:04X}",
                         f"CIA → 0x{target:04X}" + (f"; LR = 0x{cia_after:04X}" if lk else ""))

    def _exec_bc(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """BC[L][A] — branch conditional."""
        bo, bi, bd, aa, lk = dec["bo"], dec["bi"], dec["bd"], dec["aa"], dec["lk"]
        ctr = self._rf.read_ctr()
        cr = self._rf.read_cr()
        should_branch, new_ctr = _eval_branch(bo, bi, ctr, cr)
        self._rf.write_ctr(new_ctr)
        if lk:
            self._rf.write_lr(cia_after)
        if should_branch:
            target = bd & _MASK32 if aa else (cia + bd) & _MASK32
        else:
            target = cia_after
        self._rf.write_cia(target)
        return StepTrace(cia, target,
                         f"bc {bo}, {bi}, 0x{target:04X}",
                         f"branch {'taken' if should_branch else 'not taken'}; CIA → 0x{target:04X}")

    def _exec_bx(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """BCLR/BCCTR (opcode 19) — branch via LR or CTR."""
        bo, bi, xo, lk = dec["bo"], dec["bi"], dec["xo"], dec["lk"]
        ctr = self._rf.read_ctr()
        cr = self._rf.read_cr()
        should_branch, new_ctr = _eval_branch(bo, bi, ctr, cr)

        # Handle CR manipulation opcodes first (they share opcode 19)
        if xo in (257, 289, 225, 417, 193, 449, 33, 129):
            return self._exec_cr_op(dec, cia, cia_after, xo)
        if xo == 0:  # MCRF
            return self._exec_mcrf(dec, cia, cia_after)
        if xo == 150:  # ISYNC
            return StepTrace(cia, cia_after, "isync", "instruction sync (no-op)")

        if lk:
            self._rf.write_lr(cia_after)

        if xo == 16:   # BCLR
            branch_target = self._rf.read_lr() & ~3
            mnem = "blr" if (bo == 20 and not lk) else f"bclr {bo}, {bi}"
        elif xo == 528:  # BCCTR
            branch_target = ctr & ~3
            self._rf.write_ctr(new_ctr)
            mnem = "bctr" if (bo == 20 and not lk) else f"bcctr {bo}, {bi}"
        else:
            self._halted = True
            return StepTrace(cia, cia, f"ERROR: unknown xl xo={xo}",
                             f"Unknown XL XO={xo}")

        if xo == 16:
            # For BCLR, still update CTR if needed
            self._rf.write_ctr(new_ctr)

        target = branch_target if should_branch else cia_after
        self._rf.write_cia(target)
        return StepTrace(cia, target, mnem,
                         f"CIA → 0x{target:04X}")

    def _exec_cr_op(self, dec: dict, cia: int, cia_after: int, xo: int) -> StepTrace:
        """CR logical operations: CRAND, CRNAND, CROR, CRNOR, CRXOR, CREQV, CRANDC, CRORC."""
        # In XL-form CR ops: BO=BT, BI=BA, BH=BB
        bt = dec["bo"]   # destination CR bit
        ba = dec["bi"]   # source CR bit A
        bb = dec["bh"]   # source CR bit B
        cr = self._rf.read_cr()
        bit_a = (cr >> (31 - ba)) & 1
        bit_b = (cr >> (31 - bb)) & 1

        result_bit: int
        if xo == 257:    # CRAND
            result_bit = AND(bit_a, bit_b)
            mn = "crand"
        elif xo == 289:  # CRNAND
            result_bit = NOT(AND(bit_a, bit_b))
            mn = "crnand"
        elif xo == 225:  # CROR
            result_bit = OR(bit_a, bit_b)
            mn = "cror"
        elif xo == 417:  # CRNOR
            result_bit = NOT(OR(bit_a, bit_b))
            mn = "crnor"
        elif xo == 193:  # CRXOR
            result_bit = XOR(bit_a, bit_b)
            mn = "crxor"
        elif xo == 449:  # CREQV
            result_bit = NOT(XOR(bit_a, bit_b))
            mn = "creqv"
        elif xo == 33:   # CRANDC
            result_bit = AND(bit_a, NOT(bit_b))
            mn = "crandc"
        else:            # xo == 129: CRORC
            result_bit = OR(bit_a, NOT(bit_b))
            mn = "crorc"

        # Write result_bit into CR at position BT (0=MSB)
        cr_bits = int_to_bits(cr, 32)
        cr_bits[31 - bt] = AND(result_bit, 1)
        self._rf.write_cr(bits_to_int(cr_bits))
        return StepTrace(cia, cia_after, f"{mn} {bt}, {ba}, {bb}",
                         f"CR[{bt}] = {result_bit}")

    def _exec_mcrf(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """MCRF CRD, CRS — copy CR field."""
        crd = (dec["bo"] >> 2) & 0x7  # bits [25:23] = crfD in BO slot
        crs = (dec["bi"] >> 2) & 0x7  # bits [20:18] = crfS in BI slot
        cr = self._rf.read_cr()
        src_shift = 28 - crs * 4
        nibble = (cr >> src_shift) & 0xF
        lt = (nibble >> 3) & 1
        gt = (nibble >> 2) & 1
        eq = (nibble >> 1) & 1
        so = nibble & 1
        self._rf.set_cr_field(crd, lt, gt, eq, so)
        return StepTrace(cia, cia_after, f"mcrf cr{crd}, cr{crs}",
                         f"CR{crd} = CR{crs}")

    # ── Opcode 31 (X/XO/XFX-form) ─────────────────────────────────────────────

    def _exec_x31(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """Dispatch opcode 31 instructions by XO field."""
        xo = dec["xo"]
        xo9 = dec["xo9"]
        rd, ra, rb, rc, oe = dec["rd"], dec["ra"], dec["rb"], dec["rc"], dec["oe"]

        # ── Compare (X-form) ───────────────────────────────────────────────
        if xo == 0:   # CMP — signed compare
            crfd = dec["crfd"]
            lt, gt, eq = cmp32(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._set_cr_cmp(crfd, lt, gt, eq)
            cr = self._rf.read_cr()
            return StepTrace(cia, cia_after,
                             f"cmp cr{crfd}, r{ra}, r{rb}",
                             f"CR{crfd} = 0x{(cr >> (28 - crfd * 4)) & 0xF:X}")

        if xo == 32:  # CMPL — unsigned compare
            crfd = dec["crfd"]
            lt, gt, eq = cmpl32(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._set_cr_cmp(crfd, lt, gt, eq)
            cr = self._rf.read_cr()
            return StepTrace(cia, cia_after,
                             f"cmpl cr{crfd}, r{ra}, r{rb}",
                             f"CR{crfd} = 0x{(cr >> (28 - crfd * 4)) & 0xF:X}")

        # ── CNTLZW (X-form) ────────────────────────────────────────────────
        if xo == 26:
            rs = rd
            result = cntlzw(self._rf.read_gpr(rs)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after,
                             f"cntlzw r{ra}, r{rs}",
                             f"r{ra} = {result}")

        # ── X-form logical ─────────────────────────────────────────────────
        if xo == 28:   # AND
            rs = rd
            result = and32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"and r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 444:  # OR
            rs = rd
            result = or32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"or r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 316:  # XOR
            rs = rd
            result = xor32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"xor r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 476:  # NAND
            rs = rd
            result = nand32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"nand r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 124:  # NOR
            rs = rd
            result = nor32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"nor r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 284:  # EQV
            rs = rd
            result = eqv32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"eqv r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 60:   # ANDC
            rs = rd
            result = andc32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"andc r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 412:  # ORC
            rs = rd
            result = orc32(self._rf.read_gpr(rs), self._rf.read_gpr(rb)).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"orc r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        # ── Shifts (X-form) ────────────────────────────────────────────────
        if xo == 24:   # SLW
            rs = rd
            shamt = self._rf.read_gpr(rb) & 0x3F
            result = sll32(self._rf.read_gpr(rs), shamt).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"slw r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 536:  # SRW
            rs = rd
            shamt = self._rf.read_gpr(rb) & 0x3F
            result = srl32(self._rf.read_gpr(rs), shamt).result
            self._rf.write_gpr(ra, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"srw r{ra}, r{rs}, r{rb}", f"r{ra} = 0x{result:08X}")

        if xo == 792:  # SRAW
            rs = rd
            shamt = self._rf.read_gpr(rb) & 0x3F
            result_r, ca = sra32(self._rf.read_gpr(rs), shamt)
            self._rf.write_gpr(ra, result_r.result)
            self._xer_set_ca(ca)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"sraw r{ra}, r{rs}, r{rb}",
                             f"r{ra} = 0x{result_r.result:08X}, CA={ca}")

        if xo == 824:  # SRAWI
            rs = rd
            sh = rb  # shift amount from rB field
            result_r, ca = sra32(self._rf.read_gpr(rs), sh)
            self._rf.write_gpr(ra, result_r.result)
            self._xer_set_ca(ca)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"srawi r{ra}, r{rs}, {sh}",
                             f"r{ra} = 0x{result_r.result:08X}, CA={ca}")

        # ── Move to/from SPR (XFX-form) ────────────────────────────────────
        if xo == 339:  # MFSPR
            spr = dec["spr"]
            if spr == 8:
                val = self._rf.read_lr()
                spr_name = "LR"
            elif spr == 9:
                val = self._rf.read_ctr()
                spr_name = "CTR"
            elif spr == 1:
                val = self._rf.read_xer()
                spr_name = "XER"
            else:
                val = 0
                spr_name = f"SPR{spr}"
            self._rf.write_gpr(rd, val)
            return StepTrace(cia, cia_after, f"mfspr r{rd}, {spr_name}",
                             f"r{rd} = {spr_name} = 0x{val:08X}")

        if xo == 467:  # MTSPR
            spr = dec["spr"]
            rs_val = self._rf.read_gpr(rd)  # rS at rd slot
            if spr == 8:
                self._rf.write_lr(rs_val)
                spr_name = "LR"
            elif spr == 9:
                self._rf.write_ctr(rs_val)
                spr_name = "CTR"
            elif spr == 1:
                self._rf.write_xer(rs_val)
                spr_name = "XER"
            else:
                spr_name = f"SPR{spr}"
            return StepTrace(cia, cia_after, f"mtspr {spr_name}, r{rd}",
                             f"{spr_name} = 0x{rs_val:08X}")

        if xo == 19:   # MFCR
            self._rf.write_gpr(rd, self._rf.read_cr())
            return StepTrace(cia, cia_after, f"mfcr r{rd}",
                             f"r{rd} = CR = 0x{self._rf.read_cr():08X}")

        if xo == 144:  # MTCRF
            fxm = dec["fxm"]
            rs_val = self._rf.read_gpr(rd)
            cr = self._rf.read_cr()
            cr_bits = int_to_bits(cr, 32)
            rs_bits = int_to_bits(rs_val, 32)
            for bit in range(8):
                if fxm & (0x80 >> bit):
                    shift = 28 - bit * 4
                    # Copy the 4-bit nibble from rS to CR
                    for k in range(4):
                        cr_bits[shift + k] = rs_bits[shift + k]
            self._rf.write_cr(bits_to_int(cr_bits))
            return StepTrace(cia, cia_after, f"mtcrf 0x{fxm:02X}, r{rd}",
                             f"CR = 0x{self._rf.read_cr():08X}")

        # ── Indexed loads/stores ────────────────────────────────────────────
        if xo == 23:   # LWZX
            ea = self._ea_x(ra, rb)
            val = self._load32(ea)
            self._rf.write_gpr(rd, val)
            return StepTrace(cia, cia_after, f"lwzx r{rd}, r{ra}, r{rb}",
                             f"r{rd} = MEM[0x{ea:04X}] = 0x{val:08X}")

        if xo == 55:   # LWZUX
            ea = self._ea_x(ra, rb)
            val = self._load32(ea)
            self._rf.write_gpr(rd, val)
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"lwzux r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:08X}; r{ra} = 0x{ea:04X}")

        if xo == 87:   # LBZX
            ea = self._ea_x(ra, rb)
            val = self._load8(ea)
            self._rf.write_gpr(rd, val)
            return StepTrace(cia, cia_after, f"lbzx r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:02X}")

        if xo == 119:  # LBZUX
            ea = self._ea_x(ra, rb)
            val = self._load8(ea)
            self._rf.write_gpr(rd, val)
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"lbzux r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:02X}")

        if xo == 279:  # LHZX
            ea = self._ea_x(ra, rb)
            val = self._load16z(ea)
            self._rf.write_gpr(rd, val)
            return StepTrace(cia, cia_after, f"lhzx r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:04X}")

        if xo == 311:  # LHZUX
            ea = self._ea_x(ra, rb)
            val = self._load16z(ea)
            self._rf.write_gpr(rd, val)
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"lhzux r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:04X}")

        if xo == 343:  # LHAX
            ea = self._ea_x(ra, rb)
            val = self._load16a(ea)
            self._rf.write_gpr(rd, val)
            return StepTrace(cia, cia_after, f"lhax r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:08X}")

        if xo == 375:  # LHAUX
            ea = self._ea_x(ra, rb)
            val = self._load16a(ea)
            self._rf.write_gpr(rd, val)
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"lhaux r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{val:08X}")

        if xo == 151:  # STWX
            ea = self._ea_x(ra, rb)
            self._store32(ea, self._rf.read_gpr(rd))
            return StepTrace(cia, cia_after, f"stwx r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rd):08X}")

        if xo == 183:  # STWUX
            ea = self._ea_x(ra, rb)
            self._store32(ea, self._rf.read_gpr(rd))
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"stwux r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = r{rd}; r{ra} = 0x{ea:04X}")

        if xo == 215:  # STBX
            ea = self._ea_x(ra, rb)
            self._store8(ea, self._rf.read_gpr(rd))
            return StepTrace(cia, cia_after, f"stbx r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rd) & 0xFF:02X}")

        if xo == 247:  # STBUX
            ea = self._ea_x(ra, rb)
            self._store8(ea, self._rf.read_gpr(rd))
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"stbux r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rd) & 0xFF:02X}")

        if xo == 407:  # STHX
            ea = self._ea_x(ra, rb)
            self._store16(ea, self._rf.read_gpr(rd))
            return StepTrace(cia, cia_after, f"sthx r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rd) & 0xFFFF:04X}")

        if xo == 439:  # STHUX
            ea = self._ea_x(ra, rb)
            self._store16(ea, self._rf.read_gpr(rd))
            self._rf.write_gpr(ra, ea)
            return StepTrace(cia, cia_after, f"sthux r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rd) & 0xFFFF:04X}")

        if xo == 20:   # LWARX (load word and reserve — treat as regular load)
            ea = self._ea_x(ra, rb)
            val = self._load32(ea)
            self._rf.write_gpr(rd, val)
            return StepTrace(cia, cia_after, f"lwarx r{rd}, r{ra}, r{rb}",
                             f"r{rd} = MEM[0x{ea:04X}] = 0x{val:08X}")

        if xo == 150:  # STWCX. (store word conditional — always succeed)
            ea = self._ea_x(ra, rb)
            self._store32(ea, self._rf.read_gpr(rd))
            # Always succeed: set CR0[EQ]=1, clear LT/GT, copy SO
            so = self._xer_get_so()
            self._rf.set_cr_field(0, 0, 0, 1, so)
            return StepTrace(cia, cia_after, f"stwcx. r{rd}, r{ra}, r{rb}",
                             f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rd):08X}; EQ=1")

        # ── XO-form arithmetic ─────────────────────────────────────────────
        # xo9 is the 9-bit XO (bits [9:1]), which overlaps with X-form XO
        # when OE=0.  We check by masking to 9 bits.

        if xo9 == 266:   # ADD[O][.]
            result_r = add32(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"add r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 10:    # ADDC[O][.]
            result_r = add32(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"addc r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

        if xo9 == 138:   # ADDE[O][.]
            ca = self._xer_get_ca()
            result_r = add32(self._rf.read_gpr(ra), self._rf.read_gpr(rb), carry_in=ca)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"adde r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

        if xo9 == 234:   # ADDME[O][.] — rD = rA + (-1) + CA
            ca = self._xer_get_ca()
            # -1 in 32-bit = 0xFFFFFFFF
            result_r = add32(self._rf.read_gpr(ra), 0xFFFFFFFF, carry_in=ca)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"addme r{rd}, r{ra}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 202:   # ADDZE[O][.] — rD = rA + 0 + CA
            ca = self._xer_get_ca()
            result_r = add32(self._rf.read_gpr(ra), 0, carry_in=ca)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"addze r{rd}, r{ra}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 40:    # SUBF[O][.] — rD = NOT(rA) + rB + 1
            not_ra = invert_32bit(self._rf.read_gpr(ra))
            result_r = add32(not_ra, self._rf.read_gpr(rb), carry_in=1)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"subf r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 8:     # SUBFC[O][.] — like SUBF + set CA
            not_ra = invert_32bit(self._rf.read_gpr(ra))
            result_r = add32(not_ra, self._rf.read_gpr(rb), carry_in=1)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"subfc r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

        if xo9 == 136:   # SUBFE[O][.] — rD = NOT(rA) + rB + CA
            ca = self._xer_get_ca()
            not_ra = invert_32bit(self._rf.read_gpr(ra))
            result_r = add32(not_ra, self._rf.read_gpr(rb), carry_in=ca)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"subfe r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result_r.result:08X}, CA={result_r.carry}")

        if xo9 == 232:   # SUBFME[O][.] — rD = NOT(rA) + (-1) + CA
            ca = self._xer_get_ca()
            not_ra = invert_32bit(self._rf.read_gpr(ra))
            result_r = add32(not_ra, 0xFFFFFFFF, carry_in=ca)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"subfme r{rd}, r{ra}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 200:   # SUBFZE[O][.] — rD = NOT(rA) + 0 + CA
            ca = self._xer_get_ca()
            not_ra = invert_32bit(self._rf.read_gpr(ra))
            result_r = add32(not_ra, 0, carry_in=ca)
            self._xer_set_ca(result_r.carry)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"subfze r{rd}, r{ra}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 104:   # NEG[O][.] — rD = NOT(rA) + 1
            not_ra = invert_32bit(self._rf.read_gpr(ra))
            result_r = add32(not_ra, 0, carry_in=1)
            if oe:
                self._xer_set_ov_so(result_r.overflow)
            self._rf.write_gpr(rd, result_r.result)
            if rc:
                self._update_cr0(result_r.result)
            return StepTrace(cia, cia_after, f"neg r{rd}, r{ra}",
                             f"r{rd} = 0x{result_r.result:08X}")

        if xo9 == 75:    # MULHW[.]
            result = mul32_hi_signed(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._rf.write_gpr(rd, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"mulhw r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result:08X}")

        if xo9 == 11:    # MULHWU[.]
            result = mul32_hi_unsigned(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._rf.write_gpr(rd, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"mulhwu r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result:08X}")

        if xo9 == 235:   # MULLW[O][.]
            lo, hi, _ov = mul32_lo(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            # Signed overflow: hi != sign-extended lo
            lo_sign = (lo >> 31) & 1
            sign_ext_hi = 0xFFFFFFFF if lo_sign else 0
            ov_flag = 0 if hi == sign_ext_hi else 1
            if oe:
                self._xer_set_ov_so(ov_flag)
            self._rf.write_gpr(rd, lo)
            if rc:
                self._update_cr0(lo)
            return StepTrace(cia, cia_after, f"mullw r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{lo:08X}")

        if xo9 == 491:   # DIVW[O][.]
            result = divw(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._rf.write_gpr(rd, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"divw r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result:08X}")

        if xo9 == 459:   # DIVWU[O][.]
            result = divwu(self._rf.read_gpr(ra), self._rf.read_gpr(rb))
            self._rf.write_gpr(rd, result)
            if rc:
                self._update_cr0(result)
            return StepTrace(cia, cia_after, f"divwu r{rd}, r{ra}, r{rb}",
                             f"r{rd} = 0x{result:08X}")

        # Unknown opcode 31 XO
        self._halted = True
        return StepTrace(cia, cia,
                         f"ERROR: unknown x31 xo={xo}",
                         f"Unknown OPCD=31 XO={xo} at CIA=0x{cia:04X}.")

    # ── D-form loads/stores ────────────────────────────────────────────────────

    def _exec_lwz(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LWZ rD, d(rA) — load word zero."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load32(ea)
        self._rf.write_gpr(rd, val)
        return StepTrace(cia, cia_after, f"lwz r{rd}, {simm}(r{ra})",
                         f"r{rd} = MEM[0x{ea:04X}] = 0x{val:08X}")

    def _exec_lwzu(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LWZU rD, d(rA) — load word zero with update."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load32(ea)
        self._rf.write_gpr(rd, val)
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"lwzu r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:08X}; r{ra} = 0x{ea:04X}")

    def _exec_lbz(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LBZ rD, d(rA) — load byte zero."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load8(ea)
        self._rf.write_gpr(rd, val)
        return StepTrace(cia, cia_after, f"lbz r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:02X}")

    def _exec_lbzu(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LBZU rD, d(rA) — load byte zero with update."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load8(ea)
        self._rf.write_gpr(rd, val)
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"lbzu r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:02X}; r{ra} = 0x{ea:04X}")

    def _exec_lhz(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LHZ rD, d(rA) — load halfword zero."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load16z(ea)
        self._rf.write_gpr(rd, val)
        return StepTrace(cia, cia_after, f"lhz r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:04X}")

    def _exec_lhzu(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LHZU rD, d(rA) — load halfword zero with update."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load16z(ea)
        self._rf.write_gpr(rd, val)
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"lhzu r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:04X}; r{ra} = 0x{ea:04X}")

    def _exec_lha(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LHA rD, d(rA) — load halfword algebraic (sign-extended)."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load16a(ea)
        self._rf.write_gpr(rd, val)
        return StepTrace(cia, cia_after, f"lha r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:08X}")

    def _exec_lhau(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LHAU rD, d(rA) — load halfword algebraic with update."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        val = self._load16a(ea)
        self._rf.write_gpr(rd, val)
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"lhau r{rd}, {simm}(r{ra})",
                         f"r{rd} = 0x{val:08X}; r{ra} = 0x{ea:04X}")

    def _exec_stw(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STW rS, d(rA) — store word."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        self._store32(ea, self._rf.read_gpr(rs))
        return StepTrace(cia, cia_after, f"stw r{rs}, {simm}(r{ra})",
                         f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rs):08X}")

    def _exec_stwu(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STWU rS, d(rA) — store word with update."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        self._store32(ea, self._rf.read_gpr(rs))
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"stwu r{rs}, {simm}(r{ra})",
                         f"MEM[0x{ea:04X}] = r{rs}; r{ra} = 0x{ea:04X}")

    def _exec_stb(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STB rS, d(rA) — store byte."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        self._store8(ea, self._rf.read_gpr(rs) & 0xFF)
        return StepTrace(cia, cia_after, f"stb r{rs}, {simm}(r{ra})",
                         f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rs) & 0xFF:02X}")

    def _exec_stbu(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STBU rS, d(rA) — store byte with update."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        self._store8(ea, self._rf.read_gpr(rs) & 0xFF)
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"stbu r{rs}, {simm}(r{ra})",
                         f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rs) & 0xFF:02X}")

    def _exec_sth(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STH rS, d(rA) — store halfword."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        self._store16(ea, self._rf.read_gpr(rs) & 0xFFFF)
        return StepTrace(cia, cia_after, f"sth r{rs}, {simm}(r{ra})",
                         f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rs) & 0xFFFF:04X}")

    def _exec_sthu(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STHU rS, d(rA) — store halfword with update."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        self._store16(ea, self._rf.read_gpr(rs) & 0xFFFF)
        self._rf.write_gpr(ra, ea)
        return StepTrace(cia, cia_after, f"sthu r{rs}, {simm}(r{ra})",
                         f"MEM[0x{ea:04X}] = 0x{self._rf.read_gpr(rs) & 0xFFFF:04X}")

    def _exec_lmw(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """LMW rD, d(rA) — load multiple words: GPRs rD..31 from ea."""
        rd, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        for r in range(rd, 32):
            val = self._load32(ea)
            self._rf.write_gpr(r, val)
            ea = (ea + 4) & _MASK32
        return StepTrace(cia, cia_after, f"lmw r{rd}, {simm}(r{ra})",
                         f"loaded {32 - rd} regs from 0x{self._ea(ra, simm):04X}")

    def _exec_stmw(self, dec: dict, cia: int, cia_after: int) -> StepTrace:
        """STMW rS, d(rA) — store multiple words: GPRs rS..31 to ea."""
        rs, ra, simm = dec["rd"], dec["ra"], dec["simm"]
        ea = self._ea(ra, simm)
        for r in range(rs, 32):
            self._store32(ea, self._rf.read_gpr(r))
            ea = (ea + 4) & _MASK32
        return StepTrace(cia, cia_after, f"stmw r{rs}, {simm}(r{ra})",
                         f"stored {32 - rs} regs to 0x{self._ea(ra, simm):04X}")
