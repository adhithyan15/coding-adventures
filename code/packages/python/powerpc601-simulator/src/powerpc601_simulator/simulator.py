"""
PowerPC 601 (1992) Behavioral Simulator
=========================================

The PowerPC 601 was the first chip produced by the AIM alliance (Apple, IBM,
Motorola) and powered the original Power Macintosh.  This module implements a
behavioral simulation of its integer instruction set.

Architecture at a glance
------------------------

  Word width:   32 bits (big-endian)

  Registers:
    GPR0–GPR31  32-bit general-purpose registers
                GPR0 is treated as 0 when used as rA in effective-address
                calculations (loads, stores, addi, addis), but otherwise
                holds its actual value.
    LR          Link Register — stores return address after bl/bctrl
    CTR         Count Register — decremented by bdnz-style branches;
                also used as indirect branch target
    XER         Fixed-Point Exception Register:
                  bit 31 (MSB) = SO (Summary Overflow)
                  bit 30       = OV (Overflow)
                  bit 29       = CA (Carry)
    CR          Condition Register — 8 × 4-bit fields CR0…CR7.
                CR0 occupies the most-significant nibble (bits [31:28]).
                Each field: [LT(3), GT(2), EQ(1), SO(0)] from high to low.
    CIA         Current Instruction Address (the program counter)

  Instruction encoding:
    All instructions are exactly 32 bits (4 bytes), big-endian.
    Primary opcode occupies bits [31:26] (MSB side in PPC notation = Python
    bit-positions 31..26 of a big-endian 32-bit word).

  Memory:   65 536 byte-addressed bytes.  Big-endian word reads/writes.

  HALT:     Instruction word 0x00000000 (all zeros).  Not a valid PowerPC
            instruction; the simulator halts immediately on encountering it.

Instruction formats
-------------------

I-form (branch unconditional):
  [31:26] OPCD  [25: 2] LI (24-bit signed)  [1] AA  [0] LK

B-form (branch conditional):
  [31:26] OPCD  [25:21] BO  [20:16] BI  [15:2] BD (14-bit signed)  [1] AA  [0] LK

D-form (load/store/arithmetic immediate):
  [31:26] OPCD  [25:21] rD  [20:16] rA  [15:0] imm (16-bit)

X-form (register–register, logic, compare, shift):
  [31:26] OPCD  [25:21] rS  [20:16] rA  [15:11] rB  [10:1] XO  [0] Rc

XO-form (integer arithmetic):
  [31:26] OPCD  [25:21] rD  [20:16] rA  [15:11] rB  [10] OE  [9:1] XO  [0] Rc

XFX-form (move to/from special registers):
  [31:26] OPCD  [25:21] rS  [20:11] SPR (split encoding)  [10:1] XO  [0] –

XL-form (branch via LR/CTR):
  [31:26] OPCD  [25:21] BO  [20:16] BI  [15:11] BH  [10:1] XO  [0] LK

Signed arithmetic
-----------------
All arithmetic uses Python integers masked to 32 bits.  Signed operations
sign-extend the 32-bit result before comparison.  XER[CA] is set on
carry-out from bit 31 for addc/adde/subfic; XER[SO/OV] are not simulated
(OE=0 for all instructions in this behavioral subset).
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .state import (
    MASK32,
    MEM_SIZE,
    XER_CA,
    XER_SO,
    PowerPC601State,
    make_initial_state,
    sext16,
    sext32,
)

# ── HALT word ───────────────────────────────────────────────────────────────────

HALT: bytes = b"\x00\x00\x00\x00"

# ── SPR numbers ─────────────────────────────────────────────────────────────────

SPR_XER: int = 1    # Fixed-point exception register
SPR_LR:  int = 8    # Link register
SPR_CTR: int = 9    # Count register

# ── Branch BO field constants ────────────────────────────────────────────────────
# (BO[0] = bit 4 = most significant of the 5-bit BO integer)

BO_ALWAYS: int = 20   # 0b10100 — branch always; ignore CTR and CR
BO_TRUE:   int = 18   # 0b10010 — branch if CR[BI] = 1; don't test CTR
BO_FALSE:  int = 16   # 0b10000 — branch if CR[BI] = 0; don't test CTR
BO_BDNZ:   int = 4    # 0b00100 — decrement CTR; branch if CTR ≠ 0; ignore CR
BO_BDZ:    int = 12   # 0b01100 — decrement CTR; branch if CTR = 0; ignore CR

# ── CR bit indices (BI field values for CR0) ────────────────────────────────────

BI_LT: int = 0   # CR0.LT — less than
BI_GT: int = 1   # CR0.GT — greater than
BI_EQ: int = 2   # CR0.EQ — equal
BI_SO: int = 3   # CR0.SO — summary overflow

# ── Primary opcodes ─────────────────────────────────────────────────────────────

PO_SUBFIC   = 8    # D-form: SIMM - rA, set CA
PO_CMPLI    = 10   # D-form: unsigned compare immediate
PO_CMPI     = 11   # D-form: signed compare immediate
PO_ADDI     = 14   # D-form: add immediate
PO_ADDIS    = 15   # D-form: add immediate shifted
PO_BC       = 16   # B-form: branch conditional
PO_B        = 18   # I-form: branch (unconditional/link)
PO_BX       = 19   # XL-form: bclr / bcctr
PO_ORI      = 24   # D-form: or immediate
PO_ORIS     = 25   # D-form: or immediate shifted
PO_XORI     = 26   # D-form: xor immediate
PO_ANDI_DOT = 28   # D-form: and immediate (sets CR0)
PO_ANDIS_DOT = 29  # D-form: and immediate shifted (sets CR0)
PO_X31      = 31   # X/XO/XFX-form: most register-register ops

PO_LWZ  = 32   # D-form: load word and zero
PO_LWZU = 33   # D-form: load word and zero with update
PO_LBZ  = 34   # D-form: load byte and zero
PO_LBZU = 35   # D-form: load byte and zero with update
PO_STW  = 36   # D-form: store word
PO_STWU = 37   # D-form: store word with update
PO_STB  = 38   # D-form: store byte
PO_STBU = 39   # D-form: store byte with update
PO_LHZ  = 40   # D-form: load halfword and zero
PO_LHZU = 41   # D-form: load halfword and zero with update
PO_LHA  = 42   # D-form: load halfword algebraic (sign-extend)
PO_STH  = 44   # D-form: store halfword

# ── Secondary opcodes for PO_X31 ────────────────────────────────────────────────
# X-form secondary opcodes use bits [10:1] (10-bit XO from a 32-bit word)

XO_CMP    = 0     # X-form: signed compare
XO_ADDC   = 10    # XO-form: add carrying
XO_CNTLZW = 26   # X-form: count leading zeros word
XO_AND    = 28    # X-form: and
XO_CMPL   = 32    # X-form: unsigned compare
XO_SUBF   = 40    # XO-form: subtract from
XO_NOR    = 124   # X-form: nor
XO_ADDE   = 138   # XO-form: add extended
XO_NEG    = 104   # XO-form: negate
XO_MFCR   = 19    # XFX-form: move from CR
XO_MTCRF  = 144   # XFX-form: move to CR fields
XO_OR     = 444   # X-form: or
XO_MULLW  = 235   # XO-form: multiply low word
XO_XOR    = 316   # X-form: xor
XO_MFSPR  = 339   # XFX-form: move from SPR
XO_NAND   = 476   # X-form: nand
XO_DIVWU  = 459   # XO-form: divide word unsigned
XO_MTSPR  = 467   # XFX-form: move to SPR
XO_ADD    = 266   # XO-form: add
XO_DIVW   = 491   # XO-form: divide word signed
XO_SLW    = 24    # X-form: shift left word
XO_SRW    = 536   # X-form: shift right word
XO_SRAW   = 792   # X-form: shift right algebraic word
XO_SRAWI  = 824   # X-form: shift right algebraic word immediate

# ── Secondary opcodes for PO_BX (opcode 19) ─────────────────────────────────────

XO_BCLR  = 16    # XL-form: branch to LR
XO_BCCTR = 528   # XL-form: branch to CTR


# ── Instruction encoding helpers ─────────────────────────────────────────────────


def i_form(opcode: int, byte_offset: int, AA: int = 0, LK: int = 0) -> bytes:
    """
    Encode an I-form instruction (branch unconditional).

    byte_offset — signed byte offset from CIA (or absolute address if AA=1).
    LK=1 sets the link bit (saves CIA+4 → LR).

    Encoding: [OPCD:6][LI:24][AA:1][LK:1]
    LI is the byte_offset divided by 4 (instructions are 4-byte aligned).
    """
    LI = (byte_offset >> 2) & 0xFF_FFFF  # 24-bit field
    v = (opcode << 26) | (LI << 2) | (AA << 1) | LK
    return v.to_bytes(4, "big")


def b_form(
    opcode: int, BO: int, BI: int, byte_offset: int, AA: int = 0, LK: int = 0
) -> bytes:
    """
    Encode a B-form instruction (branch conditional).

    byte_offset — signed byte offset from CIA (or absolute if AA=1).
    BO — 5-bit branch options (use BO_TRUE, BO_FALSE, BO_ALWAYS, BO_BDNZ, BO_BDZ).
    BI — 5-bit CR bit index (BI_LT=0, BI_GT=1, BI_EQ=2, BI_SO=3 for CR0).
    """
    BD = (byte_offset >> 2) & 0x3FFF  # 14-bit field
    v = (opcode << 26) | (BO << 21) | (BI << 16) | (BD << 2) | (AA << 1) | LK
    return v.to_bytes(4, "big")


def d_form(opcode: int, rD: int, rA: int, imm: int) -> bytes:
    """
    Encode a D-form instruction (immediate arithmetic, load, store, compare).

    imm — 16-bit immediate; stored as-is (caller responsible for masking).
    For signed operands pass the signed value; it will be masked to 16 bits.
    For unsigned operands (ori, xori, andi.) pass the unsigned value directly.
    """
    v = (opcode << 26) | (rD << 21) | (rA << 16) | (imm & 0xFFFF)
    return v.to_bytes(4, "big")


def x_form(opcode: int, rS: int, rA: int, rB: int, xo: int, rc: int = 0) -> bytes:
    """
    Encode an X-form instruction (register–register, logic, shift, compare).

    [OPCD:6][rS:5][rA:5][rB:5][XO:10][Rc:1]
    """
    v = (opcode << 26) | (rS << 21) | (rA << 16) | (rB << 11) | (xo << 1) | rc
    return v.to_bytes(4, "big")


def xo_form(
    opcode: int, rD: int, rA: int, rB: int, oe: int, xo: int, rc: int = 0
) -> bytes:
    """
    Encode an XO-form instruction (integer arithmetic).

    [OPCD:6][rD:5][rA:5][rB:5][OE:1][XO:9][Rc:1]
    """
    v = (opcode << 26) | (rD << 21) | (rA << 16) | (rB << 11) | (oe << 10) | (xo << 1) | rc
    return v.to_bytes(4, "big")


def xfx_form(opcode: int, rS: int, spr: int, xo: int) -> bytes:
    """
    Encode an XFX-form instruction (move to/from special registers).

    The 10-bit SPR is stored split: SPR[5:9] at bits [20:16], SPR[0:4] at bits [15:11].

    [OPCD:6][rS:5][SPR[5:9]:5][SPR[0:4]:5][XO:10][0:1]
    """
    spr_enc = ((spr & 0x1F) << 5) | ((spr >> 5) & 0x1F)
    v = (opcode << 26) | (rS << 21) | (spr_enc << 11) | (xo << 1)
    return v.to_bytes(4, "big")


def xl_form(opcode: int, BO: int, BI: int, BH: int, xo: int, lk: int = 0) -> bytes:
    """
    Encode an XL-form instruction (branch via LR or CTR).

    [OPCD:6][BO:5][BI:5][BH:5][XO:10][LK:1]
    """
    v = (opcode << 26) | (BO << 21) | (BI << 16) | (BH << 11) | (xo << 1) | lk
    return v.to_bytes(4, "big")


# ── Simulator ───────────────────────────────────────────────────────────────────


class PowerPC601Simulator(Simulator[PowerPC601State]):
    """
    Behavioral simulator for the PowerPC 601 (1992) integer instruction set.

    Implements the SIM00 Simulator[PowerPC601State] protocol:
      reset()       — zero all state
      load(prog)    — reset + copy program bytes into memory
      step()        — fetch-decode-execute one instruction; return StepTrace
      execute(prog) — load + step loop until HALT or max_steps
      get_state()   — return frozen state snapshot

    Simplifications:
    - No floating-point (FPR0–31 not simulated)
    - No MMU / virtual memory
    - OE=0 / Rc=0 for arithmetic (XER[SO/OV] never set by arithmetic)
    - Only compare instructions and andi./andis. update CR
    - Misaligned accesses succeed (address masked to alignment boundary)
    - No hardware exceptions; unknown opcodes emit an ERROR trace
    """

    def __init__(self) -> None:
        self._state: PowerPC601State = make_initial_state()

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers, memory, CIA, and halted flag."""
        self._state = make_initial_state()

    def load(self, program: bytes) -> None:
        """
        Reset the simulator and copy program bytes into memory starting at address 0.

        The program is written byte-by-byte.  Bytes beyond MEM_SIZE are silently
        truncated.  Instructions are executed starting at CIA=0.
        """
        self.reset()
        mem = list(self._state.memory)
        for i, b in enumerate(program):
            if i >= MEM_SIZE:
                break
            mem[i] = b
        self._state = PowerPC601State(
            cia=0,
            gpr=self._state.gpr,
            lr=self._state.lr,
            ctr=self._state.ctr,
            xer=self._state.xer,
            cr=self._state.cr,
            memory=tuple(mem),
            halted=False,
        )

    def step(self) -> StepTrace:
        """
        Fetch, decode, and execute one instruction at CIA.

        Returns a StepTrace with the PC before/after, mnemonic, and description.
        If already halted, returns a no-op HALT trace without advancing CIA.
        """
        cia = self._state.cia

        if self._state.halted:
            return StepTrace(
                pc_before=cia,
                pc_after=cia,
                mnemonic="HALT",
                description="Simulator is halted.",
            )

        instr = self._fetch32(cia)

        if instr == 0:
            # All-zeros word — HALT
            self._state = PowerPC601State(
                cia=cia,
                gpr=self._state.gpr,
                lr=self._state.lr,
                ctr=self._state.ctr,
                xer=self._state.xer,
                cr=self._state.cr,
                memory=self._state.memory,
                halted=True,
            )
            return StepTrace(
                pc_before=cia,
                pc_after=cia,
                mnemonic="HALT",
                description=f"HALT at CIA=0x{cia:04X}.",
            )

        return self._execute_instr(instr, cia)

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[PowerPC601State]:
        """
        Load program and step until halted or max_steps exceeded.

        Returns an ExecutionResult with final state, trace list, and status.
        """
        self.load(program)
        traces: list[StepTrace] = []
        for _ in range(max_steps):
            trace = self.step()
            traces.append(trace)
            if trace.mnemonic.startswith("ERROR:"):
                return ExecutionResult(
                    halted=False,
                    steps=len(traces),
                    final_state=self._state,
                    traces=traces,
                    error=trace.mnemonic,
                )
            if self._state.halted:
                return ExecutionResult(
                    halted=True,
                    steps=len(traces),
                    final_state=self._state,
                    traces=traces,
                    error=None,
                )
        return ExecutionResult(
            halted=False,
            steps=max_steps,
            final_state=self._state,
            traces=traces,
            error=f"max_steps={max_steps} exceeded",
        )

    def get_state(self) -> PowerPC601State:
        """Return an immutable snapshot of the current simulator state."""
        return self._state

    # ── Memory helpers ────────────────────────────────────────────────────────

    def _fetch32(self, addr: int) -> int:
        """Read a big-endian 32-bit word from memory at addr."""
        addr = addr & (MEM_SIZE - 1)
        m = self._state.memory
        return (m[addr] << 24) | (m[addr + 1] << 16) | (m[addr + 2] << 8) | m[addr + 3]

    def _load32(self, addr: int) -> int:
        """Load a big-endian 32-bit word from memory (word-aligned)."""
        addr = addr & ~3 & (MEM_SIZE - 1)
        m = self._state.memory
        return (m[addr] << 24) | (m[addr + 1] << 16) | (m[addr + 2] << 8) | m[addr + 3]

    def _load16z(self, addr: int) -> int:
        """Load a big-endian 16-bit halfword (zero-extended)."""
        addr = addr & ~1 & (MEM_SIZE - 1)
        m = self._state.memory
        return (m[addr] << 8) | m[addr + 1]

    def _load16a(self, addr: int) -> int:
        """Load a big-endian 16-bit halfword (sign-extended to 32 bits)."""
        v = self._load16z(addr)
        if v & 0x8000:
            return v - 0x10000
        return v

    def _load8(self, addr: int) -> int:
        """Load a single byte (zero-extended)."""
        return self._state.memory[addr & (MEM_SIZE - 1)]

    def _store32(self, addr: int, val: int, gpr: list[int],
                 lr: int, ctr: int, xer: int, cr: int) -> tuple[int, ...]:
        """Return a new memory tuple with a 32-bit big-endian word written at addr."""
        addr = addr & ~3 & (MEM_SIZE - 1)
        val = val & MASK32
        mem = list(self._state.memory)
        mem[addr]     = (val >> 24) & 0xFF
        mem[addr + 1] = (val >> 16) & 0xFF
        mem[addr + 2] = (val >> 8)  & 0xFF
        mem[addr + 3] =  val        & 0xFF
        return tuple(mem)

    def _store16(self, addr: int, val: int) -> tuple[int, ...]:
        """Return a new memory tuple with a 16-bit big-endian halfword written."""
        addr = addr & ~1 & (MEM_SIZE - 1)
        val = val & 0xFFFF
        mem = list(self._state.memory)
        mem[addr]     = (val >> 8) & 0xFF
        mem[addr + 1] =  val       & 0xFF
        return tuple(mem)

    def _store8(self, addr: int, val: int) -> tuple[int, ...]:
        """Return a new memory tuple with a single byte written."""
        addr = addr & (MEM_SIZE - 1)
        mem = list(self._state.memory)
        mem[addr] = val & 0xFF
        return tuple(mem)

    # ── CR helpers ────────────────────────────────────────────────────────────

    def _update_cr_field(
        self, cr: int, field: int, lt: int, gt: int, eq: int, so: int
    ) -> int:
        """
        Update one 4-bit CR field (0=CR0, 7=CR7).

        The nibble bits are [LT, GT, EQ, SO] from most to least significant.
        Shifts the nibble into the correct position within the 32-bit CR integer.
        """
        nibble = (lt << 3) | (gt << 2) | (eq << 1) | so
        shift = 28 - field * 4
        mask = 0xF << shift
        return (cr & ~mask) | (nibble << shift)

    def _compare_signed(
        self, a: int, b: int, xer: int, cr: int, field: int
    ) -> int:
        """Set a CR field from signed comparison of a and b."""
        sa, sb = sext32(a), sext32(b)
        lt = 1 if sa < sb else 0
        gt = 1 if sa > sb else 0
        eq = 1 if sa == sb else 0
        so = 1 if (xer & XER_SO) else 0
        return self._update_cr_field(cr, field, lt, gt, eq, so)

    def _compare_unsigned(
        self, a: int, b: int, xer: int, cr: int, field: int
    ) -> int:
        """Set a CR field from unsigned comparison of a and b."""
        ua, ub = a & MASK32, b & MASK32
        lt = 1 if ua < ub else 0
        gt = 1 if ua > ub else 0
        eq = 1 if ua == ub else 0
        so = 1 if (xer & XER_SO) else 0
        return self._update_cr_field(cr, field, lt, gt, eq, so)

    def _update_cr0_from_result(self, result: int, xer: int, cr: int) -> int:
        """Update CR0 based on a 32-bit arithmetic/logical result (signed)."""
        s = sext32(result)
        lt = 1 if s < 0 else 0
        gt = 1 if s > 0 else 0
        eq = 1 if s == 0 else 0
        so = 1 if (xer & XER_SO) else 0
        return self._update_cr_field(cr, 0, lt, gt, eq, so)

    # ── Branch evaluation ─────────────────────────────────────────────────────

    def _eval_branch(self, bo: int, bi: int, ctr: int, cr: int) -> tuple[bool, int]:
        """
        Evaluate a conditional branch.

        Returns (should_branch, new_ctr).

        BO bit layout (bit 4 = MSB of the 5-bit integer, as extracted
        from the instruction word with `(instr >> 21) & 0x1F`):
          BO[0] = (bo >> 4) & 1  — if 1, don't decrement/test CTR
          BO[1] = (bo >> 3) & 1  — CTR test: 0 = branch if CTR≠0, 1 = if CTR=0
          BO[2] = (bo >> 2) & 1  — if 1, don't test CR condition bit
          BO[3] = (bo >> 1) & 1  — CR test: 1 = branch if bit=1, 0 = branch if bit=0
          BO[4] = bo & 1         — branch prediction hint (ignored)
        """
        bo0 = (bo >> 4) & 1
        bo1 = (bo >> 3) & 1
        bo2 = (bo >> 2) & 1
        bo3 = (bo >> 1) & 1

        # CTR condition
        if bo0 == 0:
            ctr = (ctr - 1) & MASK32
            ctr_ok = (ctr != 0) if bo1 == 0 else (ctr == 0)
        else:
            ctr_ok = True

        # CR condition
        if bo2 == 0:
            cr_bit = (cr >> (31 - bi)) & 1
            cond_ok = (cr_bit == bo3)
        else:
            cond_ok = True

        return ctr_ok and cond_ok, ctr

    # ── Effective address ─────────────────────────────────────────────────────

    @staticmethod
    def _ea(gpr: list[int], ra: int, d: int) -> int:
        """Compute effective address for D-form load/store/addi.

        If rA = 0, the base is 0 (not GPR0's contents).
        d is the sign-extended 16-bit displacement.
        """
        base = 0 if ra == 0 else gpr[ra]
        return (base + d) & MASK32

    # ── Main instruction dispatch ─────────────────────────────────────────────

    def _execute_instr(self, instr: int, cia: int) -> StepTrace:
        """
        Decode and execute a 32-bit instruction word.

        Extracts the primary opcode from bits [31:26] of the 32-bit integer
        (where bit 31 is the MSB in our Python representation of a big-endian
        word).  Dispatches to the appropriate handler.
        """
        opcd = (instr >> 26) & 0x3F

        if opcd == PO_ADDI:
            return self._exec_addi(instr, cia)
        elif opcd == PO_ADDIS:
            return self._exec_addis(instr, cia)
        elif opcd == PO_SUBFIC:
            return self._exec_subfic(instr, cia)
        elif opcd == PO_ORI:
            return self._exec_ori(instr, cia)
        elif opcd == PO_ORIS:
            return self._exec_oris(instr, cia)
        elif opcd == PO_XORI:
            return self._exec_xori(instr, cia)
        elif opcd == PO_ANDI_DOT:
            return self._exec_andi_dot(instr, cia)
        elif opcd == PO_ANDIS_DOT:
            return self._exec_andis_dot(instr, cia)
        elif opcd == PO_CMPI:
            return self._exec_cmpi(instr, cia)
        elif opcd == PO_CMPLI:
            return self._exec_cmpli(instr, cia)
        elif opcd == PO_LWZ:
            return self._exec_lwz(instr, cia)
        elif opcd == PO_LWZU:
            return self._exec_lwzu(instr, cia)
        elif opcd == PO_LBZ:
            return self._exec_lbz(instr, cia)
        elif opcd == PO_LBZU:
            return self._exec_lbzu(instr, cia)
        elif opcd == PO_LHZ:
            return self._exec_lhz(instr, cia)
        elif opcd == PO_LHZU:
            return self._exec_lhzu(instr, cia)
        elif opcd == PO_LHA:
            return self._exec_lha(instr, cia)
        elif opcd == PO_STW:
            return self._exec_stw(instr, cia)
        elif opcd == PO_STWU:
            return self._exec_stwu(instr, cia)
        elif opcd == PO_STB:
            return self._exec_stb(instr, cia)
        elif opcd == PO_STBU:
            return self._exec_stbu(instr, cia)
        elif opcd == PO_STH:
            return self._exec_sth(instr, cia)
        elif opcd == PO_B:
            return self._exec_b(instr, cia)
        elif opcd == PO_BC:
            return self._exec_bc(instr, cia)
        elif opcd == PO_BX:
            return self._exec_bx(instr, cia)
        elif opcd == PO_X31:
            return self._exec_x31(instr, cia)
        else:
            # Unknown opcode — emit an error trace and halt
            self._state = PowerPC601State(
                cia=cia,
                gpr=self._state.gpr,
                lr=self._state.lr,
                ctr=self._state.ctr,
                xer=self._state.xer,
                cr=self._state.cr,
                memory=self._state.memory,
                halted=True,
            )
            return StepTrace(
                pc_before=cia,
                pc_after=cia,
                mnemonic=f"ERROR: unknown opcode {opcd}",
                description=f"Unknown primary opcode {opcd} (instr=0x{instr:08X}) at CIA=0x{cia:04X}.",
            )

    # ── Instruction implementations ───────────────────────────────────────────

    # D-form helpers

    def _exec_addi(self, instr: int, cia: int) -> StepTrace:
        """
        addi rD, rA, SIMM   (opcode 14)

        rD = (rA == 0 ? 0 : GPR[rA]) + sign_extend(SIMM)

        The special rA=0 rule lets 'li rD, val' be encoded as 'addi rD, 0, val'.
        """
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        simm = sext16(instr & 0xFFFF)
        base = 0 if ra == 0 else self._state.gpr[ra]
        result = (base + simm) & MASK32
        gpr = list(self._state.gpr)
        gpr[rd] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        mnem = f"addi r{rd}, r{ra}, {simm}"
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=mnem,
                         description=f"r{rd} = 0x{result:08X}")

    def _exec_addis(self, instr: int, cia: int) -> StepTrace:
        """
        addis rD, rA, SIMM  (opcode 15)

        rD = (rA == 0 ? 0 : GPR[rA]) + (SIMM << 16)

        'lis rD, val' = 'addis rD, 0, val' — loads a 16-bit constant into the
        upper half of rD (lower half zeroed).
        """
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        simm = sext16(instr & 0xFFFF)
        base = 0 if ra == 0 else self._state.gpr[ra]
        result = (base + (simm << 16)) & MASK32
        gpr = list(self._state.gpr)
        gpr[rd] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        mnem = f"addis r{rd}, r{ra}, {simm}"
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=mnem,
                         description=f"r{rd} = 0x{result:08X}")

    def _exec_subfic(self, instr: int, cia: int) -> StepTrace:
        """
        subfic rD, rA, SIMM  (opcode 8)

        rD = SIMM - GPR[rA]; sets XER[CA] on borrow-out.

        The carry flag CA is set if there is no borrow, i.e., SIMM >= GPR[rA]
        (unsigned).  PowerPC uses "carry = no borrow" convention.
        """
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        simm = sext16(instr & 0xFFFF) & MASK32
        ra_val = self._state.gpr[ra]
        # subfic: result = ~rA + SIMM + 1  (subtract from: SIMM - rA)
        result = (simm - ra_val) & MASK32
        # CA set if ~rA + SIMM + 1 >= 2^32 i.e. no borrow
        # Equivalently: (SIMM as unsigned) >= (rA as unsigned)
        ca = 1 if (simm & MASK32) >= (ra_val & MASK32) else 0
        xer = (self._state.xer & ~XER_CA) | (XER_CA if ca else 0)
        gpr = list(self._state.gpr)
        gpr[rd] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        mnem = f"subfic r{rd}, r{ra}, {sext16(instr & 0xFFFF)}"
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=mnem,
                         description=f"r{rd} = 0x{result:08X}, CA={ca}")

    def _exec_ori(self, instr: int, cia: int) -> StepTrace:
        """ori rA, rS, UIMM — rA = GPR[rS] | UIMM (zero-extended)."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        uimm = instr & 0xFFFF
        result = (self._state.gpr[rs] | uimm) & MASK32
        gpr = list(self._state.gpr)
        gpr[ra] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"ori r{ra}, r{rs}, {uimm}",
                         description=f"r{ra} = 0x{result:08X}")

    def _exec_oris(self, instr: int, cia: int) -> StepTrace:
        """oris rA, rS, UIMM — rA = GPR[rS] | (UIMM << 16)."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        uimm = instr & 0xFFFF
        result = (self._state.gpr[rs] | (uimm << 16)) & MASK32
        gpr = list(self._state.gpr)
        gpr[ra] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"oris r{ra}, r{rs}, {uimm}",
                         description=f"r{ra} = 0x{result:08X}")

    def _exec_xori(self, instr: int, cia: int) -> StepTrace:
        """xori rA, rS, UIMM — rA = GPR[rS] ^ UIMM (zero-extended)."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        uimm = instr & 0xFFFF
        result = (self._state.gpr[rs] ^ uimm) & MASK32
        gpr = list(self._state.gpr)
        gpr[ra] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"xori r{ra}, r{rs}, {uimm}",
                         description=f"r{ra} = 0x{result:08X}")

    def _exec_andi_dot(self, instr: int, cia: int) -> StepTrace:
        """andi. rA, rS, UIMM — rA = rS & UIMM; always updates CR0."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        uimm = instr & 0xFFFF
        result = (self._state.gpr[rs] & uimm) & MASK32
        gpr = list(self._state.gpr)
        gpr[ra] = result
        new_cr = self._update_cr0_from_result(result, self._state.xer, self._state.cr)
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"andi. r{ra}, r{rs}, {uimm}",
                         description=f"r{ra} = 0x{result:08X}; CR0 updated")

    def _exec_andis_dot(self, instr: int, cia: int) -> StepTrace:
        """andis. rA, rS, UIMM — rA = rS & (UIMM << 16); updates CR0."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        uimm = instr & 0xFFFF
        result = (self._state.gpr[rs] & (uimm << 16)) & MASK32
        gpr = list(self._state.gpr)
        gpr[ra] = result
        new_cr = self._update_cr0_from_result(result, self._state.xer, self._state.cr)
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"andis. r{ra}, r{rs}, {uimm}",
                         description=f"r{ra} = 0x{result:08X}; CR0 updated")

    def _exec_cmpi(self, instr: int, cia: int) -> StepTrace:
        """
        cmpwi crfD, rA, SIMM  (opcode 11)

        Signed comparison of GPR[rA] with sign_extend(SIMM).
        Updates CR field crfD (bits [8:6] of the instruction).
        """
        crfd = (instr >> 23) & 0x7
        ra   = (instr >> 16) & 0x1F
        simm = sext16(instr & 0xFFFF)
        new_cr = self._compare_signed(
            self._state.gpr[ra], simm & MASK32, self._state.xer, self._state.cr, crfd
        )
        self._state = PowerPC601State(
            cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4,
                         mnemonic=f"cmpwi cr{crfd}, r{ra}, {simm}",
                         description=f"CR{crfd} = 0x{(new_cr >> (28 - crfd * 4)) & 0xF:X}")

    def _exec_cmpli(self, instr: int, cia: int) -> StepTrace:
        """
        cmplwi crfD, rA, UIMM  (opcode 10)

        Unsigned comparison of GPR[rA] with UIMM (zero-extended).
        """
        crfd = (instr >> 23) & 0x7
        ra   = (instr >> 16) & 0x1F
        uimm = instr & 0xFFFF
        new_cr = self._compare_unsigned(
            self._state.gpr[ra], uimm, self._state.xer, self._state.cr, crfd
        )
        self._state = PowerPC601State(
            cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4,
                         mnemonic=f"cmplwi cr{crfd}, r{ra}, {uimm}",
                         description=f"CR{crfd} = 0x{(new_cr >> (28 - crfd * 4)) & 0xF:X}")

    # Load instructions

    def _exec_lwz(self, instr: int, cia: int) -> StepTrace:
        """lwz rD, d(rA) — load 4-byte word, zero-extend to 32 bits."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load32(ea)
        gpr = list(self._state.gpr)
        gpr[rd] = val
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lwz r{rd}, {d}(r{ra})",
                         description=f"r{rd} = MEM[0x{ea:04X}] = 0x{val:08X}")

    def _exec_lwzu(self, instr: int, cia: int) -> StepTrace:
        """lwzu rD, d(rA) — load word, update rA = EA."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load32(ea)
        gpr = list(self._state.gpr)
        gpr[rd] = val
        gpr[ra] = ea
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lwzu r{rd}, {d}(r{ra})",
                         description=f"r{rd} = MEM[0x{ea:04X}] = 0x{val:08X}; r{ra} updated")

    def _exec_lbz(self, instr: int, cia: int) -> StepTrace:
        """lbz rD, d(rA) — load byte, zero-extended."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load8(ea)
        gpr = list(self._state.gpr)
        gpr[rd] = val
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lbz r{rd}, {d}(r{ra})",
                         description=f"r{rd} = MEM[0x{ea:04X}] = 0x{val:02X}")

    def _exec_lbzu(self, instr: int, cia: int) -> StepTrace:
        """lbzu rD, d(rA) — load byte, zero-extended, update rA."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load8(ea)
        gpr = list(self._state.gpr)
        gpr[rd] = val
        gpr[ra] = ea
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lbzu r{rd}, {d}(r{ra})",
                         description=f"r{rd} = 0x{val:02X}; r{ra} = 0x{ea:04X}")

    def _exec_lhz(self, instr: int, cia: int) -> StepTrace:
        """lhz rD, d(rA) — load 16-bit halfword, zero-extended."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load16z(ea)
        gpr = list(self._state.gpr)
        gpr[rd] = val
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lhz r{rd}, {d}(r{ra})",
                         description=f"r{rd} = 0x{val:04X}")

    def _exec_lhzu(self, instr: int, cia: int) -> StepTrace:
        """lhzu rD, d(rA) — load halfword zero-extended, update rA."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load16z(ea)
        gpr = list(self._state.gpr)
        gpr[rd] = val
        gpr[ra] = ea
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lhzu r{rd}, {d}(r{ra})",
                         description=f"r{rd} = 0x{val:04X}; r{ra} = 0x{ea:04X}")

    def _exec_lha(self, instr: int, cia: int) -> StepTrace:
        """lha rD, d(rA) — load halfword algebraic (sign-extended to 32 bits)."""
        rd = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        val = self._load16a(ea) & MASK32
        gpr = list(self._state.gpr)
        gpr[rd] = val
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"lha r{rd}, {d}(r{ra})",
                         description=f"r{rd} = 0x{val:08X}")

    # Store instructions

    def _exec_stw(self, instr: int, cia: int) -> StepTrace:
        """stw rS, d(rA) — store 4-byte word."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        new_mem = self._store32(ea, self._state.gpr[rs],
                                list(self._state.gpr), self._state.lr,
                                self._state.ctr, self._state.xer, self._state.cr)
        self._state = PowerPC601State(
            cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=new_mem, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"stw r{rs}, {d}(r{ra})",
                         description=f"MEM[0x{ea:04X}] = 0x{self._state.memory[ea & ~3 & (MEM_SIZE-1)]:02X}...")

    def _exec_stwu(self, instr: int, cia: int) -> StepTrace:
        """stwu rS, d(rA) — store word, update rA = EA."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        new_mem = self._store32(ea, self._state.gpr[rs],
                                list(self._state.gpr), self._state.lr,
                                self._state.ctr, self._state.xer, self._state.cr)
        gpr = list(self._state.gpr)
        gpr[ra] = ea
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=new_mem, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"stwu r{rs}, {d}(r{ra})",
                         description=f"MEM[0x{ea:04X}] = r{rs}; r{ra} = 0x{ea:04X}")

    def _exec_stb(self, instr: int, cia: int) -> StepTrace:
        """stb rS, d(rA) — store low byte of rS."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        new_mem = self._store8(ea, self._state.gpr[rs] & 0xFF)
        self._state = PowerPC601State(
            cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=new_mem, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"stb r{rs}, {d}(r{ra})",
                         description=f"MEM[0x{ea:04X}] = 0x{self._state.gpr[rs] & 0xFF:02X}")

    def _exec_stbu(self, instr: int, cia: int) -> StepTrace:
        """stbu rS, d(rA) — store byte, update rA."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        new_mem = self._store8(ea, self._state.gpr[rs] & 0xFF)
        gpr = list(self._state.gpr)
        gpr[ra] = ea
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=new_mem, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"stbu r{rs}, {d}(r{ra})",
                         description=f"MEM[0x{ea:04X}] = 0x{self._state.gpr[rs] & 0xFF:02X}; r{ra} = 0x{ea:04X}")

    def _exec_sth(self, instr: int, cia: int) -> StepTrace:
        """sth rS, d(rA) — store low 16-bit halfword of rS."""
        rs = (instr >> 21) & 0x1F
        ra = (instr >> 16) & 0x1F
        d  = sext16(instr & 0xFFFF)
        ea = self._ea(list(self._state.gpr), ra, d)
        new_mem = self._store16(ea, self._state.gpr[rs] & 0xFFFF)
        self._state = PowerPC601State(
            cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=new_mem, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=f"sth r{rs}, {d}(r{ra})",
                         description=f"MEM[0x{ea:04X}] = 0x{self._state.gpr[rs] & 0xFFFF:04X}")

    # Branch instructions

    def _exec_b(self, instr: int, cia: int) -> StepTrace:
        """
        b / bl  (opcode 18, I-form)

        LI is the 24-bit signed field; byte offset = sign_extend(LI) << 2.
        If AA=0, branch is PC-relative (CIA + offset).
        If LK=1, CIA+4 is saved to LR before branching.
        """
        # Extract LI from bits [25:2], sign-extend the 24-bit field
        li = (instr >> 2) & 0xFF_FFFF
        if li & 0x80_0000:
            li -= 0x100_0000
        byte_off = li << 2
        aa = (instr >> 1) & 1
        lk =  instr       & 1
        lr = self._state.lr
        if lk:
            lr = cia + 4
        target = (byte_off if aa else cia + byte_off) & MASK32
        self._state = PowerPC601State(
            cia=target, gpr=self._state.gpr, lr=lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        mnem = f"bl 0x{target:04X}" if lk else f"b 0x{target:04X}"
        return StepTrace(pc_before=cia, pc_after=target, mnemonic=mnem,
                         description=f"CIA → 0x{target:04X}" + (f"; LR = 0x{lr:04X}" if lk else ""))

    def _exec_bc(self, instr: int, cia: int) -> StepTrace:
        """
        bc BO, BI, BD  (opcode 16, B-form)

        Conditional branch.  BD is 14-bit signed; byte offset = BD << 2.
        BO and BI control CTR decrement and CR bit testing.
        """
        bo = (instr >> 21) & 0x1F
        bi = (instr >> 16) & 0x1F
        bd = (instr >> 2) & 0x3FFF
        if bd & 0x2000:
            bd -= 0x4000
        byte_off = bd << 2
        aa = (instr >> 1) & 1
        lk =  instr       & 1

        should_branch, new_ctr = self._eval_branch(bo, bi, self._state.ctr, self._state.cr)
        lr = self._state.lr
        if lk:
            lr = cia + 4
        if should_branch:
            target = (byte_off if aa else cia + byte_off) & MASK32
        else:
            target = cia + 4
        self._state = PowerPC601State(
            cia=target, gpr=self._state.gpr, lr=lr, ctr=new_ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=target,
                         mnemonic=f"bc {bo}, {bi}, 0x{(cia + byte_off) & MASK32:04X}",
                         description=f"branch {'taken' if should_branch else 'not taken'}; CIA → 0x{target:04X}")

    def _exec_bx(self, instr: int, cia: int) -> StepTrace:
        """
        bclr / bcctr  (opcode 19, XL-form)

        XO=16:  branch to LR (blr = bclr 20, 0)
        XO=528: branch to CTR (bctr = bcctr 20, 0; bctrl = bcctr 20, 0, lk=1)
        """
        bo  = (instr >> 21) & 0x1F
        bi  = (instr >> 16) & 0x1F
        xo  = (instr >>  1) & 0x3FF
        lk  =  instr        & 1

        should_branch, new_ctr = self._eval_branch(bo, bi, self._state.ctr, self._state.cr)
        lr_val = self._state.lr
        new_lr = cia + 4 if lk else lr_val

        if xo == XO_BCLR:
            branch_target = lr_val & ~3  # branch to LR (aligned)
            mnem = "blr" if (bo == BO_ALWAYS and not lk) else f"bclr {bo}, {bi}"
        elif xo == XO_BCCTR:
            branch_target = self._state.ctr & ~3  # branch to CTR
            mnem = "bctrl" if lk else ("bctr" if bo == BO_ALWAYS else f"bcctr {bo}, {bi}")
        else:
            # Unknown XL sub-opcode
            self._state = PowerPC601State(
                cia=cia, gpr=self._state.gpr, lr=lr_val, ctr=self._state.ctr,
                xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=True,
            )
            return StepTrace(pc_before=cia, pc_after=cia,
                             mnemonic=f"ERROR: unknown XL xo={xo}",
                             description=f"Unknown XL XO={xo}")

        target = branch_target if should_branch else cia + 4
        self._state = PowerPC601State(
            cia=target, gpr=self._state.gpr, lr=new_lr, ctr=new_ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=target, mnemonic=mnem,
                         description=f"CIA → 0x{target:04X}" + (f"; LR = 0x{new_lr:04X}" if lk else ""))

    def _exec_x31(self, instr: int, cia: int) -> StepTrace:
        """
        Dispatch all OPCD=31 instructions by their secondary opcode (XO).

        The XO field occupies bits [10:1] (10 bits) for X-form,
        or bits [9:1] (9 bits) for XO-form.  We check the 10-bit field
        first; arithmetic XO-form opcodes fit in 9 bits so they never
        collide with the 10-bit values (the OE bit at position 10 is 0
        for the ops we support).
        """
        xo10 = (instr >> 1) & 0x3FF   # 10-bit secondary opcode (X-form)
        xo9  = xo10 & 0x1FF            # 9-bit secondary opcode (XO-form, OE=0 subset)
        rd   = (instr >> 21) & 0x1F
        ra   = (instr >> 16) & 0x1F
        rb   = (instr >> 11) & 0x1F

        # ── Compare (X-form, XO bits [10:1]) ─────────────────────────────────
        if xo10 == XO_CMP:
            crfd = (instr >> 23) & 0x7
            new_cr = self._compare_signed(
                self._state.gpr[ra], self._state.gpr[rb],
                self._state.xer, self._state.cr, crfd,
            )
            self._state = PowerPC601State(
                cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
                xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"cmpw cr{crfd}, r{ra}, r{rb}",
                             description=f"CR{crfd} = 0x{(new_cr >> (28 - crfd * 4)) & 0xF:X}")

        if xo10 == XO_CMPL:
            crfd = (instr >> 23) & 0x7
            new_cr = self._compare_unsigned(
                self._state.gpr[ra], self._state.gpr[rb],
                self._state.xer, self._state.cr, crfd,
            )
            self._state = PowerPC601State(
                cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
                xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"cmplw cr{crfd}, r{ra}, r{rb}",
                             description=f"CR{crfd} = 0x{(new_cr >> (28 - crfd * 4)) & 0xF:X}")

        # ── Logical / shift (X-form) ───────────────────────────────────────────
        if xo10 == XO_AND:
            return self._x31_logic(instr, cia, rd, ra, rb, "and",
                                   self._state.gpr[rd] & self._state.gpr[rb])
        if xo10 == XO_OR:
            rs = rd  # X-form uses rS at bits [25:21]
            result = self._state.gpr[rs] | self._state.gpr[rb]
            return self._x31_logic(instr, cia, rs, ra, rb, "or", result)
        if xo10 == XO_XOR:
            rs = rd
            result = self._state.gpr[rs] ^ self._state.gpr[rb]
            return self._x31_logic(instr, cia, rs, ra, rb, "xor", result)
        if xo10 == XO_NAND:
            result = ~(self._state.gpr[rd] & self._state.gpr[rb])
            return self._x31_logic(instr, cia, rd, ra, rb, "nand", result)
        if xo10 == XO_NOR:
            rs = rd
            result = ~(self._state.gpr[rs] | self._state.gpr[rb])
            return self._x31_logic(instr, cia, rs, ra, rb, "nor", result)
        if xo10 == XO_CNTLZW:
            val = self._state.gpr[rd] & MASK32  # rS is at rd field in X-form
            result = 0
            if val == 0:
                result = 32
            else:
                v = val
                while not (v & 0x8000_0000):
                    result += 1
                    v <<= 1
            gpr = list(self._state.gpr)
            gpr[ra] = result
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"cntlzw r{ra}, r{rd}",
                             description=f"r{ra} = {result} (leading zeros of r{rd}=0x{val:08X})")

        if xo10 == XO_SLW:
            rs = rd
            n = self._state.gpr[rb] & 0x3F
            result = 0 if n >= 32 else (self._state.gpr[rs] << n) & MASK32
            return self._x31_logic(instr, cia, rs, ra, rb, "slw", result)

        if xo10 == XO_SRW:
            rs = rd
            n = self._state.gpr[rb] & 0x3F
            result = 0 if n >= 32 else (self._state.gpr[rs] & MASK32) >> n
            return self._x31_logic(instr, cia, rs, ra, rb, "srw", result)

        if xo10 == XO_SRAW:
            rs = rd
            n = self._state.gpr[rb] & 0x3F
            n = min(n, 31)
            src = sext32(self._state.gpr[rs])
            result = (src >> n) & MASK32
            # CA = 1 if src negative and any 1-bits shifted out
            ca = 1 if (src < 0 and (self._state.gpr[rs] & ((1 << n) - 1))) else 0
            xer = (self._state.xer & ~XER_CA) | (XER_CA if ca else 0)
            gpr = list(self._state.gpr)
            gpr[ra] = result
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"sraw r{ra}, r{rs}, r{rb}",
                             description=f"r{ra} = 0x{result:08X}, CA={ca}")

        if xo10 == XO_SRAWI:
            rs = rd
            sh = rb  # shift amount is in the rB field (5-bit SH)
            src = sext32(self._state.gpr[rs])
            result = (src >> sh) & MASK32
            ca = 1 if (src < 0 and (self._state.gpr[rs] & ((1 << sh) - 1))) else 0
            xer = (self._state.xer & ~XER_CA) | (XER_CA if ca else 0)
            gpr = list(self._state.gpr)
            gpr[ra] = result
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"srawi r{ra}, r{rs}, {sh}",
                             description=f"r{ra} = 0x{result:08X}, CA={ca}")

        # ── Move to/from special registers (XFX-form) ─────────────────────────
        if xo10 == XO_MFSPR:
            spr_enc = (instr >> 11) & 0x3FF
            spr = ((spr_enc & 0x1F) << 5) | (spr_enc >> 5)
            if spr == SPR_LR:
                val = self._state.lr
                spr_name = "LR"
            elif spr == SPR_CTR:
                val = self._state.ctr
                spr_name = "CTR"
            elif spr == SPR_XER:
                val = self._state.xer
                spr_name = "XER"
            else:
                val = 0
                spr_name = f"SPR{spr}"
            gpr = list(self._state.gpr)
            gpr[rd] = val
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"mfspr r{rd}, {spr_name}",
                             description=f"r{rd} = {spr_name} = 0x{val:08X}")

        if xo10 == XO_MTSPR:
            spr_enc = (instr >> 11) & 0x3FF
            spr = ((spr_enc & 0x1F) << 5) | (spr_enc >> 5)
            val = self._state.gpr[rd]  # rS is in the rD field for mtspr
            new_lr  = self._state.lr
            new_ctr = self._state.ctr
            new_xer = self._state.xer
            if spr == SPR_LR:
                new_lr = val & MASK32
                spr_name = "LR"
            elif spr == SPR_CTR:
                new_ctr = val & MASK32
                spr_name = "CTR"
            elif spr == SPR_XER:
                new_xer = val & MASK32
                spr_name = "XER"
            else:
                spr_name = f"SPR{spr}"
            self._state = PowerPC601State(
                cia=cia + 4, gpr=self._state.gpr, lr=new_lr, ctr=new_ctr,
                xer=new_xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"mtspr {spr_name}, r{rd}",
                             description=f"{spr_name} = 0x{val:08X}")

        if xo10 == XO_MFCR:
            gpr = list(self._state.gpr)
            gpr[rd] = self._state.cr
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"mfcr r{rd}",
                             description=f"r{rd} = CR = 0x{self._state.cr:08X}")

        if xo10 == XO_MTCRF:
            fxm = (instr >> 12) & 0xFF  # 8-bit field mask (bits [19:12])
            rs_val = self._state.gpr[rd]  # rS is at rd bits
            new_cr = self._state.cr
            for bit in range(8):
                if fxm & (0x80 >> bit):
                    shift = 28 - bit * 4
                    mask = 0xF << shift
                    nibble = (rs_val >> shift) & 0xF
                    new_cr = (new_cr & ~mask) | (nibble << shift)
            self._state = PowerPC601State(
                cia=cia + 4, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
                xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"mtcrf 0x{fxm:02X}, r{rd}",
                             description=f"CR = 0x{new_cr:08X}")

        # ── XO-form arithmetic (secondary opcode is 9-bit, OE=0) ──────────────
        if xo9 == XO_ADD:
            result = (self._state.gpr[ra] + self._state.gpr[rb]) & MASK32
            return self._x31_gpr(cia, rd, result, f"add r{rd}, r{ra}, r{rb}")

        if xo9 == XO_SUBF:
            result = (self._state.gpr[rb] - self._state.gpr[ra]) & MASK32
            return self._x31_gpr(cia, rd, result, f"subf r{rd}, r{ra}, r{rb}")

        if xo9 == XO_NEG:
            result = (-self._state.gpr[ra]) & MASK32
            return self._x31_gpr(cia, rd, result, f"neg r{rd}, r{ra}")

        if xo9 == XO_ADDC:
            full = self._state.gpr[ra] + self._state.gpr[rb]
            result = full & MASK32
            ca = 1 if full > MASK32 else 0
            xer = (self._state.xer & ~XER_CA) | (XER_CA if ca else 0)
            gpr = list(self._state.gpr)
            gpr[rd] = result
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"addc r{rd}, r{ra}, r{rb}",
                             description=f"r{rd} = 0x{result:08X}, CA={ca}")

        if xo9 == XO_ADDE:
            ca_in = 1 if (self._state.xer & XER_CA) else 0
            full = self._state.gpr[ra] + self._state.gpr[rb] + ca_in
            result = full & MASK32
            ca = 1 if full > MASK32 else 0
            xer = (self._state.xer & ~XER_CA) | (XER_CA if ca else 0)
            gpr = list(self._state.gpr)
            gpr[rd] = result
            self._state = PowerPC601State(
                cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
                xer=xer, cr=self._state.cr, memory=self._state.memory, halted=False,
            )
            return StepTrace(pc_before=cia, pc_after=cia + 4,
                             mnemonic=f"adde r{rd}, r{ra}, r{rb}",
                             description=f"r{rd} = 0x{result:08X}, CA={ca}")

        if xo9 == XO_MULLW:
            # signed multiply, keep low 32 bits
            a = sext32(self._state.gpr[ra])
            b = sext32(self._state.gpr[rb])
            result = (a * b) & MASK32
            return self._x31_gpr(cia, rd, result, f"mullw r{rd}, r{ra}, r{rb}")

        if xo9 == XO_DIVW:
            a = sext32(self._state.gpr[ra])
            b = sext32(self._state.gpr[rb])
            if b == 0:
                result = 0  # division by zero — undefined; return 0
            else:
                result = int(a / b) & MASK32  # truncate toward zero
            return self._x31_gpr(cia, rd, result, f"divw r{rd}, r{ra}, r{rb}")

        if xo9 == XO_DIVWU:
            a = self._state.gpr[ra] & MASK32
            b = self._state.gpr[rb] & MASK32
            result = (a // b if b != 0 else 0) & MASK32
            return self._x31_gpr(cia, rd, result, f"divwu r{rd}, r{ra}, r{rb}")

        # Unknown OPCD=31 secondary opcode
        self._state = PowerPC601State(
            cia=cia, gpr=self._state.gpr, lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=True,
        )
        return StepTrace(pc_before=cia, pc_after=cia,
                         mnemonic=f"ERROR: unknown x31 xo={xo10}",
                         description=f"Unknown OPCD=31 XO={xo10} (instr=0x{instr:08X}) at CIA=0x{cia:04X}.")

    # ── X31 helper: write GPR result ──────────────────────────────────────────

    def _x31_gpr(self, cia: int, rd: int, result: int, mnem: str) -> StepTrace:
        """Commit a computed result to GPR[rd] and advance CIA by 4."""
        result = result & MASK32
        gpr = list(self._state.gpr)
        gpr[rd] = result
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=self._state.cr, memory=self._state.memory, halted=False,
        )
        return StepTrace(pc_before=cia, pc_after=cia + 4, mnemonic=mnem,
                         description=f"r{rd} = 0x{result:08X}")

    def _x31_logic(
        self, instr: int, cia: int, rs: int, ra: int, rb: int, mnem: str, result: int
    ) -> StepTrace:
        """
        Write a logical result to GPR[rA] and advance CIA.

        In X-form logic instructions, the result goes to rA (not rD/rS).
        rS is the "source" in the rD slot; rB is the second source.
        result is the raw Python integer — it will be masked to 32 bits.
        """
        result = result & MASK32
        gpr = list(self._state.gpr)
        gpr[ra] = result
        rc = instr & 1
        new_cr = self._state.cr
        if rc:
            new_cr = self._update_cr0_from_result(result, self._state.xer, self._state.cr)
        self._state = PowerPC601State(
            cia=cia + 4, gpr=tuple(gpr), lr=self._state.lr, ctr=self._state.ctr,
            xer=self._state.xer, cr=new_cr, memory=self._state.memory, halted=False,
        )
        dot = "." if rc else ""
        return StepTrace(pc_before=cia, pc_after=cia + 4,
                         mnemonic=f"{mnem}{dot} r{ra}, r{rs}, r{rb}",
                         description=f"r{ra} = 0x{result:08X}")
