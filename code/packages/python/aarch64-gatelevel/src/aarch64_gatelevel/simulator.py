"""simulator.py — Gate-level AArch64 (ARMv8-A, 2011) simulator.

This is the top-level integration module.  It composes:
  - RegisterFile    — 32 GPRs, SP stored as 64-bit bit lists
  - decode()        — pure combinational instruction decode
  - alu.py functions — all data-path ops route through gate primitives

Every integer arithmetic/logic operation on register values routes through
gate functions (AND, OR, XOR, NOT) and ripple_carry_adder — NO Python
operators (+, -, &, |, ^, ~, *, /) in the execution path for ALU/register
operations on register values.

Python arithmetic IS used for:
  - Memory address computation (ea = base + imm12 * scale)
  - Memory indexing (self._mem[addr])
  - Loop control (for i in range(N), range(63,-1,-1))
  - PC advancement / branch targets (pc + imm * 4)
  - Byte packing/unpacking for memory reads

These are host-machine bookkeeping operations, not simulated data-path ops.

Execution model
───────────────
Each call to step():
  1. Check if halted
  2. Fetch 4 bytes from memory[PC..PC+3] as big-endian 32-bit word
  3. Advance PC by 4
  4. Decode instruction using decode()
  5. Dispatch to instruction handler
  6. Return StepTrace(pc_before, pc_after, mnemonic, description)

Memory layout
─────────────
64 KiB flat, big-endian.  The program is loaded starting at address 0.
Uninitialized memory is zero (decodes as HALT — a safety net).

NZCV layout (4-bit nibble stored as Python int)
───────────────────────────────────────────────
  bit 3 = N (Negative / sign bit)
  bit 2 = Z (Zero)
  bit 1 = C (Carry / borrow-complement)
  bit 0 = V (Overflow / signed overflow)

Only S-suffix and compare instructions update NZCV.

AArch64 ARM carry convention for subtract
─────────────────────────────────────────
  C=1 → no borrow (A >= B unsigned)  [same as A - B >= 0 unsigned]
  C=0 → borrow occurred (A < B unsigned)
This is the COMPLEMENT of x86 convention.

Gate-level requirements
───────────────────────
All arithmetic/logical operations on register bit lists must use:
  - add64/sub64/add32/sub32 from alu.py (which use bits.py → ripple_carry_adder)
  - and64/or64/xor64/not64/and32/or32/xor32/not32 from alu.py (gate per bit)
  - apply_shift from alu.py (bit-list slicing)
  - mul64/umulh64/smulh64/udiv64/sdiv64 from alu.py (shift-and-add/divide)

The RegisterFile stores all values as bit lists; the simulator reads bit
lists, passes them to ALU functions, and writes the result bit lists back.
"""

from __future__ import annotations

from aarch64_simulator.state import (
    MASK64,
    MEM_SIZE,
    AArch64State,
    sext,
)
from logic_gates import AND, NOT
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .alu import (
    add32,
    add64,
    and32,
    and64,
    apply_shift,
    clz32,
    clz64,
    flags_to_nzcv,
    logical_flags_32,
    logical_flags_64,
    mul64,
    or32,
    or64,
    rev16_bytes,
    rev32_bytes,
    rev_bytes,
    sdiv64,
    smulh64,
    sub32,
    sub64,
    udiv64,
    umulh64,
    xor32,
    xor64,
)
from .bits import (
    bits_to_int,
    compute_zero,
    int_to_bits,
    not_32bit,
    not_64bit,
)
from .decoder import AArch64Instruction, decode
from .register_file import RegisterFile

# ── Memory / address constants (bookkeeping, not data-path) ──────────────────

_MEM_MASK: int = MEM_SIZE - 1   # 0xFFFF for 64 KiB wrap


def _mask_addr(addr: int) -> int:
    """Wrap address to 64 KiB memory range (bookkeeping arithmetic)."""
    return addr & _MEM_MASK


# ── Condition evaluation ──────────────────────────────────────────────────────


def _condition_holds(cond: int, nzcv: int) -> bool:
    """Evaluate whether a 4-bit condition code is satisfied given NZCV flags.

    Truth table of condition codes
    ───────────────────────────────
    cond  Mnemonic  Test
    0000  EQ        Z==1
    0001  NE        Z==0
    0010  CS/HS     C==1
    0011  CC/LO     C==0
    0100  MI        N==1
    0101  PL        N==0
    0110  VS        V==1
    0111  VC        V==0
    1000  HI        C==1 AND Z==0
    1001  LS        C==0 OR Z==1
    1010  GE        N==V
    1011  LT        N!=V
    1100  GT        Z==0 AND N==V
    1101  LE        Z==1 OR N!=V
    1110  AL        always

    The bottom bit of cond inverts the result for the "false" members of
    each pair (NE inverts EQ, CC inverts CS, etc.), except AL/NV (0b1110/0b1111).
    """
    N = (nzcv >> 3) & 1
    Z = (nzcv >> 2) & 1
    C = (nzcv >> 1) & 1
    V = nzcv & 1
    base = cond >> 1
    if base == 0:     # EQ/NE
        result = Z == 1
    elif base == 1:   # CS/CC
        result = C == 1
    elif base == 2:   # MI/PL
        result = N == 1
    elif base == 3:   # VS/VC
        result = V == 1
    elif base == 4:   # HI/LS
        result = C == 1 and Z == 0
    elif base == 5:   # GE/LT
        result = N == V
    elif base == 6:   # GT/LE
        result = N == V and Z == 0
    else:             # AL/NV
        result = True
    # Invert for odd-numbered conditions (the "false" member of each pair)
    # but NOT for AL (0b1110)
    if (cond & 1) and cond != 0xF:
        result = not result
    return result


# ── Main simulator class ──────────────────────────────────────────────────────


class AArch64GateLevelSimulator(Simulator[AArch64State]):
    """Gate-level AArch64 (ARMv8-A, 2011) simulator.

    Implements Simulator[AArch64State] from simulator_protocol.

    Every 64-bit ALU operation (ADD, SUB, AND, OR, XOR, NOT, shifts,
    multiply, divide) routes through logic gate primitives from the
    logic_gates and arithmetic packages.

    State is tracked internally as a RegisterFile (bit lists) + memory
    bytearray + NZCV nibble.  get_state() synthesizes an immutable
    AArch64State snapshot.

    Example
    ───────
    >>> import struct
    >>> sim = AArch64GateLevelSimulator()
    >>> # MOVZ X0, #42 (sf=1, opc=10, hw=0, imm16=42, Rd=0)
    >>> v = (1<<31)|(0b10<<29)|(0b100101<<23)|(0<<21)|(42<<5)|0
    >>> prog = struct.pack(">II", v, 0)  # instruction + HALT
    >>> result = sim.execute(prog)
    >>> result.final_state.gpr[0]
    42
    """

    def __init__(self) -> None:
        self._rf = RegisterFile()
        self._mem: bytearray = bytearray(MEM_SIZE)
        self._pc: int = 0
        self._nzcv: int = 0
        self._halted: bool = False

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers, memory, PC, NZCV, SP, and halted flag."""
        self._rf.reset()
        self._mem = bytearray(MEM_SIZE)
        self._pc = 0
        self._nzcv = 0
        self._halted = False

    def load(self, program: bytes, origin: int = 0) -> None:
        """Reset the simulator and copy program bytes into memory at origin.

        Parameters
        ──────────
        program : bytes to load
        origin  : start address (default 0)
        """
        self.reset()
        self._pc = origin
        for i, b in enumerate(program):
            addr = origin + i
            if addr >= MEM_SIZE:
                break
            self._mem[addr] = b

    def step(self) -> StepTrace:
        """Fetch, decode, and execute one instruction at PC.

        Returns a StepTrace with the PC before/after, mnemonic, description.
        If already halted, returns a HALT trace without advancing PC.
        """
        pc = self._pc

        if self._halted:
            return StepTrace(
                pc_before=pc,
                pc_after=pc,
                mnemonic="HALT",
                description=f"HALT @ 0x{pc:04X}",
            )

        # Fetch 4 bytes big-endian
        raw = (
            (self._mem[pc % MEM_SIZE] << 24)
            | (self._mem[(pc + 1) % MEM_SIZE] << 16)
            | (self._mem[(pc + 2) % MEM_SIZE] << 8)
            | self._mem[(pc + 3) % MEM_SIZE]
        )

        # HALT check
        if raw == 0:
            self._halted = True
            return StepTrace(
                pc_before=pc,
                pc_after=pc,
                mnemonic="HALT",
                description=f"HALT @ 0x{pc:04X}",
            )

        # Advance PC past the fetched instruction (branches may overwrite)
        next_pc = (pc + 4) & MASK64
        self._pc = next_pc

        return self._execute(raw, pc, next_pc)

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[AArch64State]:
        """Load program and step until halted or max_steps exceeded."""
        self.load(program)
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

    def get_state(self) -> AArch64State:
        """Return an immutable snapshot of the current simulator state."""
        return AArch64State(
            pc=self._pc,
            gpr=self._rf.get_gprs_tuple(),
            sp=self._rf.read_sp(),
            nzcv=self._nzcv,
            memory=tuple(self._mem),
            halted=self._halted,
        )

    def set_input_port(self, port: int, value: int) -> None:
        """No-op: AArch64 has no I/O ports in this simulation."""

    def get_output_port(self, port: int) -> int:
        """No-op: AArch64 has no I/O ports in this simulation."""
        return 0

    def interrupt(self) -> None:
        """No-op: interrupts not simulated."""

    def nmi(self) -> None:
        """No-op: NMI not simulated."""

    # ── Register read/write helpers ───────────────────────────────────────────

    def _read_bits(self, idx: int, sf: int) -> list[int]:
        """Read a register as a bit list.

        For load/store base register (Rn=31): caller must decide SP vs XZR.
        """
        return self._rf.read_bits(idx, sf)

    def _read_int(self, idx: int, sf: int) -> int:
        """Read a register as a Python integer (for address arithmetic)."""
        return self._rf.read(idx, sf)

    def _write_bits(self, idx: int, value_bits: list[int], sf: int) -> None:
        """Write a register from a bit list."""
        self._rf.write(idx, value_bits, sf)

    def _write_int(self, idx: int, value: int, sf: int) -> None:
        """Write an integer to a register."""
        self._rf.write_int(idx, value, sf)

    # ── Memory helpers ─────────────────────────────────────────────────────────

    def _mem_read(self, addr: int, nbytes: int) -> int:
        """Read `nbytes` big-endian from memory at `addr` (wraps mod 64 KiB)."""
        result = 0
        for i in range(nbytes):
            result = (result << 8) | self._mem[(addr + i) & _MEM_MASK]
        return result

    def _mem_write(self, addr: int, value: int, nbytes: int) -> None:
        """Write `nbytes` of `value` big-endian to memory at `addr` (wraps)."""
        for i in range(nbytes - 1, -1, -1):
            self._mem[(addr + i) & _MEM_MASK] = value & 0xFF
            value >>= 8

    # ── Effective address for load/store ──────────────────────────────────────

    def _ea_base(self, Rn: int) -> int:
        """Compute base for load/store: Rn=31 → SP, otherwise GPR[Rn].

        AArch64 load/store instructions use SP when Rn=31, not XZR.
        This is the key distinction from arithmetic instructions where Rn=31=XZR.
        """
        if Rn == 31:
            return self._rf.read_sp()
        return self._rf.read(Rn, sf=1)

    # ── Instruction dispatch ───────────────────────────────────────────────────

    def _execute(self, raw: int, pc: int, next_pc: int) -> StepTrace:
        """Decode and execute one instruction word.

        Returns a StepTrace.  On unknown opcode, halts and returns ERROR trace.
        """
        try:
            dec = decode(raw)
        except ValueError:
            self._halted = True
            return StepTrace(
                pc_before=pc, pc_after=pc,
                mnemonic=f"ERROR: DECODE(0x{raw:08X})",
                description=f"Decode error at 0x{pc:04X}",
            )

        op = dec.opcode

        if op == "HALT":
            self._halted = True
            self._pc = pc
            return StepTrace(pc_before=pc, pc_after=pc, mnemonic="HALT",
                             description=f"HALT @ 0x{pc:04X}")

        if op == "NOP":
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic="NOP",
                             description=f"NOP @ 0x{pc:04X}")

        if op in ("B", "BL"):
            return self._exec_branch_imm(dec, pc, next_pc)

        if op.startswith("B."):
            return self._exec_branch_cond(dec, pc, next_pc)

        if op in ("CBZ", "CBNZ"):
            return self._exec_cbz_cbnz(dec, pc, next_pc)

        if op in ("TBZ", "TBNZ"):
            return self._exec_tbz_tbnz(dec, pc, next_pc)

        if op in ("BR", "BLR", "RET"):
            return self._exec_branch_reg(dec, pc, next_pc)

        if op in ("ADD", "ADDS", "SUB", "SUBS"):
            if dec.Rm != 0 or dec.shift_type != 0 or dec.shift_amount != 0:
                # Could be register form (Rm present) or immediate form (imm set)
                # If the decoder set Rm (and there's no imm), it's a reg form
                # The decoder distinguishes via the presence of Rm vs imm
                # Check: does it have a register form (shift_type field valid)?
                # The decoder always sets Rm for reg-reg; for imm form Rm is 0
                # by default but imm is non-zero. We check if it decoded as reg.
                # Actually: decoder sets shift_type for reg forms, imm for imm forms.
                # A simpler heuristic: if the opcode came from bits[28:24]==01011 (reg)
                # it has Rm; if from bits[28:23] in {100000,100001} (imm) it has imm.
                # The decoder sets the same opcode for both! We need to check both.
                # Since Rm is 0 for imm-form (XZR always reads 0 but is the default),
                # we check if there was actually a register operand by looking at the
                # raw bits... But we decoded already. So: if dec has shift_amount set
                # and no dec.imm, it came from the reg form.
                pass
            return self._exec_add_sub(dec, pc, next_pc)

        if op in ("MOVZ", "MOVN", "MOVK"):
            return self._exec_movwide(dec, pc, next_pc)

        if op in ("AND", "ORR", "EOR", "ANDS"):
            if dec.N_bit != 0 or dec.immr != 0 or dec.imms != 0:
                # Logical immediate form if bitmask_imm is set
                if dec.bitmask_imm != 0 or (dec.N_bit == 0 and dec.immr == 0 and dec.imms == 0):
                    pass  # could be either; check Rm
                # If Rm is set (from shift_amount or shift_type fields), it's reg form
            return self._exec_logical(dec, pc, next_pc)

        if op in ("BIC", "ORN", "EON", "BICS"):
            return self._exec_logical_reg(dec, pc, next_pc)

        if op in ("STRB", "LDRB", "LDRSB", "LDRSB32",
                  "STRH", "LDRH", "LDRSH", "LDRSH32",
                  "STR32", "LDR32", "LDRSW",
                  "STR", "LDR"):
            return self._exec_ldst(dec, pc, next_pc)

        if op in ("MADD", "MSUB"):
            return self._exec_madd_msub(dec, pc, next_pc)

        if op in ("SMULH", "UMULH"):
            return self._exec_mulh(dec, pc, next_pc)

        if op in ("UDIV", "SDIV", "LSLV", "LSRV", "ASRV", "RORV"):
            return self._exec_dp2(dec, pc, next_pc)

        if op in ("CLZ", "RBIT", "REV", "REV16", "REV32"):
            return self._exec_dp1(dec, pc, next_pc)

        if op in ("CSEL", "CSINC", "CSINV", "CSNEG"):
            return self._exec_csel(dec, pc, next_pc)

        # Unknown
        self._halted = True
        self._pc = pc
        return StepTrace(
            pc_before=pc, pc_after=pc,
            mnemonic=f"ERROR: UNKNOWN(0x{raw:08X})",
            description=f"Unknown opcode {op!r} @ 0x{pc:04X}",
        )

    # ── Branch instructions ────────────────────────────────────────────────────

    def _exec_branch_imm(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """B / BL — unconditional branch, optionally save return address."""
        target = (pc + dec.imm * 4) & MASK64
        if dec.opcode == "BL":
            # Save return address (next_pc) in X30 (link register)
            self._write_int(30, next_pc, sf=1)
        self._pc = target
        return StepTrace(pc, target, dec.opcode, f"{dec.opcode} → 0x{target:04X}")

    def _exec_branch_cond(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """B.cond — conditional branch."""
        if _condition_holds(dec.cond, self._nzcv):
            target = (pc + dec.imm * 4) & MASK64
        else:
            target = next_pc
        self._pc = target
        taken = "taken" if target != next_pc else "not taken"
        return StepTrace(pc, target, dec.opcode, f"{dec.opcode} {taken} → 0x{target:04X}")

    def _exec_cbz_cbnz(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """CBZ / CBNZ — compare-and-branch."""
        rt_bits = self._read_bits(dec.Rd, dec.sf)
        is_zero = AND(compute_zero(rt_bits), 1)
        if dec.opcode == "CBZ":
            taken = bool(is_zero)
        else:
            taken = not bool(is_zero)
        target = (pc + dec.imm * 4) & MASK64 if taken else next_pc
        self._pc = target
        desc = f"{dec.opcode} X{dec.Rd}, → 0x{target:04X}"
        return StepTrace(pc, target, dec.opcode, desc)

    def _exec_tbz_tbnz(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """TBZ / TBNZ — test-and-branch on a specific bit."""
        rt_bits = self._read_bits(dec.Rd, sf=1)   # always 64-bit for TBZ
        bit_val = AND(rt_bits[dec.bit_num], 1)
        if dec.opcode == "TBZ":
            taken = (bit_val == 0)
        else:
            taken = (bit_val == 1)
        target = (pc + dec.imm * 4) & MASK64 if taken else next_pc
        self._pc = target
        desc = f"{dec.opcode} X{dec.Rd}, #{dec.bit_num}, → 0x{target:04X}"
        return StepTrace(pc, target, dec.opcode, desc)

    def _exec_branch_reg(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """BR / BLR / RET — branch to/via register."""
        rn_val = self._read_int(dec.Rn, sf=1)
        target = rn_val & MASK64
        if dec.opcode == "BLR":
            self._write_int(30, next_pc, sf=1)
        self._pc = target
        return StepTrace(pc, target, dec.opcode, f"{dec.opcode} X{dec.Rn} → 0x{target:04X}")

    # ── Data Processing Immediate: ADD/SUB and Register: ADD/SUB ─────────────

    def _exec_add_sub(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """ADD / ADDS / SUB / SUBS (immediate or shifted-register form).

        All arithmetic routes through gate-level add64/sub64/add32/sub32.
        NZCV is updated only for ADDS/SUBS.

        For shifted-register form (Rm field present), the shift is applied
        to Rm via apply_shift() before the arithmetic.

        For immediate form (imm field), we convert imm to a bit list and
        use the same gate-level arithmetic.
        """
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn

        rn_bits = self._read_bits(Rn, sf)

        # Determine second operand: immediate vs shifted register
        if dec.Rm != 0 or dec.shift_amount != 0:
            # Shifted-register form (Rm may be 0/XZR, but shift_amount distinguishes)
            # Actually, if dec.Rm == 0, it means XZR (which is 0), and with shift it's still 0.
            # We check if the opcode came from the reg encoder by whether there's a shift_type.
            # Since the decoder set shift_type for reg-form and imm for imm-form, we check
            # if dec.imm is nonzero (imm-form) or if dec.Rm exists in encoding.
            # The key difference: imm-form has dec.imm set; reg-form has dec.Rm and dec.shift_amount.
            # For reg-form: even if Rm=0, we use the register (XZR=0).
            rm_bits = self._read_bits(dec.Rm, sf)
            operand_bits = apply_shift(rm_bits, dec.shift_type, dec.shift_amount, sf)
        else:
            # Immediate form
            mask = 0xFFFF_FFFF_FFFF_FFFF if sf else 0xFFFF_FFFF
            operand_bits = int_to_bits(dec.imm & mask, 64 if sf else 32)

        op = dec.opcode

        if op in ("ADD", "ADDS"):
            if sf:
                result = add64(rn_bits, operand_bits)
            else:
                result = add32(rn_bits[:32], operand_bits[:32] if len(operand_bits) >= 32 else operand_bits + [0] * (32 - len(operand_bits)))
        else:  # SUB, SUBS
            if sf:
                result = sub64(rn_bits, operand_bits)
            else:
                result = sub32(rn_bits[:32], operand_bits[:32] if len(operand_bits) >= 32 else operand_bits + [0] * (32 - len(operand_bits)))

        if op in ("ADDS", "SUBS"):
            self._nzcv = flags_to_nzcv(result.negative, result.zero, result.carry, result.overflow)

        # Write result
        if sf:
            result_bits = int_to_bits(result.result, 64)
        else:
            result_bits = int_to_bits(result.result, 32)
        self._write_bits(Rd, result_bits, sf)

        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{result.result:X}")

    # ── Move Wide Immediate ────────────────────────────────────────────────────

    def _exec_movwide(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """MOVZ / MOVN / MOVK — move wide immediate.

        MOVZ: Rd = imm16 << (hw*16)         (zero-fill elsewhere)
        MOVN: Rd = NOT(imm16 << (hw*16))    (NOT of the zero-filled immediate)
        MOVK: Rd[hw*16+15:hw*16] = imm16    (keep other bits)

        Gate-level: for MOVK, we read the current register value as bits,
        modify the 16-bit slice using OR with the new bits after clearing via
        NOT+AND, then write back. For MOVZ/MOVN we build the bit list directly.
        """
        sf = dec.sf
        Rd = dec.Rd
        imm16 = dec.imm
        shift = dec.hw * 16   # shift amount in bits (0, 16, 32, 48)

        op = dec.opcode

        if op == "MOVZ":
            # Zero-fill, place imm16 at bit positions [shift..shift+15]
            # This is bookkeeping: building a bit list with zeros except for imm16 region
            val = imm16 << shift
            result_bits = int_to_bits(val & (0xFFFF_FFFF_FFFF_FFFF if sf else 0xFFFF_FFFF), 64 if sf else 32)

        elif op == "MOVN":
            # NOT(imm16 << shift) — invert all bits
            val = imm16 << shift
            # NOT via gate-level: build the imm as bits, then NOT each bit
            if sf:
                imm_bits = int_to_bits(val & 0xFFFF_FFFF_FFFF_FFFF, 64)
                result_bits = [NOT(b) for b in imm_bits]
            else:
                imm_bits = int_to_bits(val & 0xFFFF_FFFF, 32)
                result_bits = [NOT(b) for b in imm_bits]

        else:  # MOVK
            # Keep bits outside [shift..shift+15]; insert imm16 at those positions
            # Gate-level: read existing, clear the slot, OR in new bits
            cur_bits = self._read_bits(Rd, sf)
            if sf:
                width = 64
            else:
                width = 32
            # Build mask: 0 in the 16-bit slot, 1 elsewhere
            # We do this as bit manipulation: clear then OR
            new_imm_bits = int_to_bits(imm16, 16)
            result_bits = cur_bits[:width][:]
            for i in range(16):
                # Clear bit at position (shift + i) and set to new_imm_bits[i]
                result_bits[shift + i] = new_imm_bits[i]

        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{bits_to_int(result_bits):X}")

    # ── Logical Immediate and Logical Register ─────────────────────────────────

    def _exec_logical(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """AND / ORR / EOR / ANDS (immediate or register form).

        For immediate form: the second operand is dec.bitmask_imm (decoded 64-bit mask).
        For register form (Rm present, bitmask_imm=0): shifted Rm is the operand.

        Gate-level: routes through and64/or64/xor64 (which call and_64bit etc.)
        """
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn
        op = dec.opcode

        rn_bits = self._read_bits(Rn, sf)

        # Determine operand: immediate (bitmask_imm) or register (Rm shifted).
        #
        # Logical immediate encoding (bits[28:23]=010010) always sets a non-zero
        # bitmask_imm (encode_bitmask always produces at least 1 set bit).
        # Logical shifted-register encoding (bits[28:24]=01010) always leaves
        # bitmask_imm=0 and populates Rm/shift_type/shift_amount instead.
        # So bitmask_imm != 0 is the reliable discriminant.
        if dec.bitmask_imm != 0:
            # Immediate form — use decoded bitmask
            mask = 0xFFFF_FFFF_FFFF_FFFF if sf else 0xFFFF_FFFF
            imm_val = dec.bitmask_imm & mask
            operand_bits = int_to_bits(imm_val, 64 if sf else 32)
        else:
            # Register form (Rm=0 means XZR which always reads 0, still valid)
            rm_bits = self._read_bits(dec.Rm, sf)
            shifted = apply_shift(rm_bits, dec.shift_type, dec.shift_amount, sf)
            if dec.N_bit:
                shifted = [NOT(b) for b in shifted]
            operand_bits = shifted

        # Pad operand to match rn_bits width
        width = 64 if sf else 32
        if len(operand_bits) < width:
            operand_bits = operand_bits + [0] * (width - len(operand_bits))
        if len(rn_bits) < width:
            rn_bits = rn_bits + [0] * (width - len(rn_bits))

        if sf:
            if op in ("AND", "ANDS"):
                res = and64(rn_bits, operand_bits)
            elif op == "ORR":
                res = or64(rn_bits, operand_bits)
            elif op == "EOR":
                res = xor64(rn_bits, operand_bits)
            else:
                res = and64(rn_bits, operand_bits)
        else:
            if op in ("AND", "ANDS"):
                res = and32(rn_bits[:32], operand_bits[:32])
            elif op == "ORR":
                res = or32(rn_bits[:32], operand_bits[:32])
            elif op == "EOR":
                res = xor32(rn_bits[:32], operand_bits[:32])
            else:
                res = and32(rn_bits[:32], operand_bits[:32])

        if op == "ANDS":
            if sf:
                n, z, c, v = logical_flags_64(int_to_bits(res.result, 64))
            else:
                n, z, c, v = logical_flags_32(int_to_bits(res.result, 32))
            self._nzcv = flags_to_nzcv(n, z, c, v)

        result_bits = int_to_bits(res.result, 64 if sf else 32)
        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{res.result:X}")

    def _exec_logical_reg(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """BIC / ORN / EON / BICS — logical with inverted register.

        These are always register-form.  Rm is shifted then inverted (N_bit=1).
        """
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn
        op = dec.opcode

        rn_bits = self._read_bits(Rn, sf)
        rm_bits = self._read_bits(dec.Rm, sf)
        shifted_rm = apply_shift(rm_bits, dec.shift_type, dec.shift_amount, sf)
        # Invert Rm (BIC = AND(Rn, NOT(Rm)), etc.)
        inv_rm = [NOT(b) for b in shifted_rm]

        if sf:
            if op in ("BIC", "BICS"):
                res = and64(rn_bits, inv_rm)
            elif op == "ORN":
                res = or64(rn_bits, inv_rm)
            elif op == "EON":
                res = xor64(rn_bits, inv_rm)
            else:  # BICS
                res = and64(rn_bits, inv_rm)
        else:
            if op in ("BIC", "BICS"):
                res = and32(rn_bits[:32], inv_rm[:32])
            elif op == "ORN":
                res = or32(rn_bits[:32], inv_rm[:32])
            elif op == "EON":
                res = xor32(rn_bits[:32], inv_rm[:32])
            else:
                res = and32(rn_bits[:32], inv_rm[:32])

        if op == "BICS":
            if sf:
                n, z, c, v = logical_flags_64(int_to_bits(res.result, 64))
            else:
                n, z, c, v = logical_flags_32(int_to_bits(res.result, 32))
            self._nzcv = flags_to_nzcv(n, z, c, v)

        result_bits = int_to_bits(res.result, 64 if sf else 32)
        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{res.result:X}")

    # ── Load / Store ───────────────────────────────────────────────────────────

    def _exec_ldst(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """Load/Store with unsigned offset.

        EA = Rn_val + imm12 * scale, where scale = 1 << size.
        Rn=31 → SP (not XZR) for load/store instructions.

        All signed-extension values are computed by reading the raw bytes and
        doing integer bookkeeping (sign-extend is not a register-ALU operation
        but a memory-width conversion).

        The stored/loaded data paths are bookkeeping (byte-pack/unpack), not
        register-to-register ALU operations.
        """
        op = dec.opcode
        Rn = dec.Rn
        Rt = dec.Rd
        imm12 = dec.imm
        size = dec.size

        # EA = base + imm12 * (1 << size)
        base = self._ea_base(Rn)
        ea = (base + imm12 * (1 << size)) & MASK64

        if op == "STRB":
            val = self._read_int(Rt, sf=0) & 0xFF
            self._mem_write(ea, val, 1)
        elif op == "LDRB":
            val = self._mem_read(ea, 1)
            self._write_int(Rt, val, sf=0)  # zero-extend to 32-bit, then 64
        elif op in ("LDRSB",):
            val = sext(self._mem_read(ea, 1), 8)
            self._write_int(Rt, val & 0xFFFF_FFFF_FFFF_FFFF, sf=1)
        elif op == "LDRSB32":
            val = sext(self._mem_read(ea, 1), 8)
            self._write_int(Rt, val & 0xFFFF_FFFF, sf=0)
        elif op == "STRH":
            val = self._read_int(Rt, sf=0) & 0xFFFF
            self._mem_write(ea, val, 2)
        elif op == "LDRH":
            val = self._mem_read(ea, 2)
            self._write_int(Rt, val, sf=0)
        elif op == "LDRSH":
            val = sext(self._mem_read(ea, 2), 16)
            self._write_int(Rt, val & 0xFFFF_FFFF_FFFF_FFFF, sf=1)
        elif op == "LDRSH32":
            val = sext(self._mem_read(ea, 2), 16)
            self._write_int(Rt, val & 0xFFFF_FFFF, sf=0)
        elif op == "STR32":
            val = self._read_int(Rt, sf=0)
            self._mem_write(ea, val, 4)
        elif op == "LDR32":
            val = self._mem_read(ea, 4)
            self._write_int(Rt, val, sf=0)
        elif op == "LDRSW":
            val = sext(self._mem_read(ea, 4), 32)
            self._write_int(Rt, val & 0xFFFF_FFFF_FFFF_FFFF, sf=1)
        elif op == "STR":
            val = self._read_int(Rt, sf=1)
            self._mem_write(ea, val, 8)
        elif op == "LDR":
            val = self._mem_read(ea, 8)
            self._write_int(Rt, val, sf=1)
        else:
            self._halted = True
            self._pc = pc
            return StepTrace(pc, pc, f"ERROR: {op}", f"Unknown load/store {op}")

        return StepTrace(pc, next_pc, op, f"{op} X{Rt}, [X{Rn}+{imm12<<size}] @ 0x{ea:04X}")

    # ── 3-Source: MADD / MSUB ─────────────────────────────────────────────────

    def _exec_madd_msub(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """MADD / MSUB — multiply-accumulate.

        MADD: Rd = Ra + Rn * Rm
        MSUB: Rd = Ra - Rn * Rm

        MUL  = MADD Rd, Rn, Rm, XZR   (Ra=31=XZR → Ra=0)
        MNEG = MSUB Rd, Rn, Rm, XZR

        Gate-level: mul64(rn_bits, rm_bits) for the product, then
        add64/sub64 to accumulate.
        """
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn
        Rm = dec.Rm
        Ra = dec.Ra
        op = dec.opcode

        rn_bits = self._read_bits(Rn, sf)
        rm_bits = self._read_bits(Rm, sf)
        ra_bits = self._read_bits(Ra, sf)

        if sf:
            product_bits = mul64(rn_bits, rm_bits)
            if op == "MADD":
                result = add64(ra_bits, product_bits)
            else:  # MSUB
                result = sub64(ra_bits, product_bits)
            result_bits = int_to_bits(result.result, 64)
        else:
            # 32-bit version: zero-extend inputs to 64 bits, take low 32 bits of product
            rn32 = rn_bits[:32]
            rm32 = rm_bits[:32]
            ra32 = ra_bits[:32]
            product_bits = mul64(rn32 + [0] * 32, rm32 + [0] * 32)[:32]
            if op == "MADD":
                result = add32(ra32, product_bits)
            else:  # MSUB
                result = sub32(ra32, product_bits)
            result_bits = int_to_bits(result.result, 32)

        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{bits_to_int(result_bits):X}")

    # ── High multiply: SMULH / UMULH ──────────────────────────────────────────

    def _exec_mulh(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """SMULH / UMULH — upper 64 bits of 128-bit multiply.

        Gate-level: routes through smulh64/umulh64 from alu.py.
        """
        Rd = dec.Rd
        Rn = dec.Rn
        Rm = dec.Rm
        op = dec.opcode

        rn_bits = self._read_bits(Rn, sf=1)
        rm_bits = self._read_bits(Rm, sf=1)

        if op == "UMULH":
            result_bits = umulh64(rn_bits, rm_bits)
        else:  # SMULH
            result_bits = smulh64(rn_bits, rm_bits)

        self._write_bits(Rd, result_bits, sf=1)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{bits_to_int(result_bits):X}")

    # ── Data Processing 2-Source ───────────────────────────────────────────────

    def _exec_dp2(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """UDIV / SDIV / LSLV / LSRV / ASRV / RORV — data processing 2-source."""
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn
        Rm = dec.Rm
        op = dec.opcode

        rn_bits = self._read_bits(Rn, sf)
        rm_bits = self._read_bits(Rm, sf)
        width = 64 if sf else 32

        if op == "UDIV":
            if sf:
                result_bits = udiv64(rn_bits, rm_bits)
            else:
                from .bits import udiv_64 as _udiv64
                q, _ = _udiv64(rn_bits + [0] * 32, rm_bits + [0] * 32)
                result_bits = q[:32]
        elif op == "SDIV":
            if sf:
                result_bits = sdiv64(rn_bits, rm_bits)
            else:
                from .bits import sdiv_64 as _sdiv64
                rn64 = rn_bits[:32] + [rn_bits[31]] * 32
                rm64 = rm_bits[:32] + [rm_bits[31]] * 32
                q, _ = _sdiv64(rn64, rm64)
                result_bits = q[:32]
        else:  # shifts
            rm_val = bits_to_int(rm_bits)
            shamt = rm_val % width
            shift_map = {"LSLV": 0, "LSRV": 1, "ASRV": 2, "RORV": 3}
            shift_type = shift_map[op]
            result_bits = apply_shift(rn_bits, shift_type, shamt, sf)

        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{bits_to_int(result_bits):X}")

    # ── Data Processing 1-Source ───────────────────────────────────────────────

    def _exec_dp1(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """CLZ / RBIT / REV / REV16 / REV32 — data processing 1-source.

        Gate-level:
          CLZ: clz64/clz32 from alu.py (sequential AND scan)
          REV: rev_bytes from alu.py (byte-list reordering)
          REV16: rev16_bytes from alu.py
          REV32: rev32_bytes from alu.py
          RBIT: sequential bit reversal via bit-list reorder
        """
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn
        op = dec.opcode

        rn_bits = self._read_bits(Rn, sf)
        # Pad to 64 bits for unified handling
        if not sf:
            rn64 = rn_bits + [0] * 32
        else:
            rn64 = rn_bits

        if op == "CLZ":
            if sf:
                result_bits = clz64(rn64)
            else:
                result_bits = clz32(rn_bits)
        elif op == "REV":
            nbytes = 8 if sf else 4
            result_bits = rev_bytes(rn64, nbytes)
            if not sf:
                result_bits = result_bits[:32]
        elif op == "REV16":
            width_bits = 64 if sf else 32
            result_bits = rev16_bytes(rn64, width_bits)
            if not sf:
                result_bits = result_bits[:32]
        elif op == "REV32":
            # Only valid for sf=1 (X registers)
            result_bits = rev32_bytes(rn64)
        elif op == "RBIT":
            # Reverse all bits in the register (bit mirror)
            width = 64 if sf else 32
            result_bits = rn_bits[:width][::-1]
        else:
            self._halted = True
            self._pc = pc
            return StepTrace(pc, pc, f"ERROR: {op}", f"Unknown dp1 {op}")

        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{bits_to_int(result_bits):X}")

    # ── Conditional Select ────────────────────────────────────────────────────

    def _exec_csel(self, dec: AArch64Instruction, pc: int, next_pc: int) -> StepTrace:
        """CSEL / CSINC / CSINV / CSNEG — conditional select.

        If condition holds: Rd = Rn
        Else:
          CSEL:  Rd = Rm
          CSINC: Rd = Rm + 1  (gate-level add with carry_in=1 to zero)
          CSINV: Rd = NOT(Rm) (gate-level NOT each bit)
          CSNEG: Rd = -Rm     (gate-level: NOT(Rm) + 1)

        Gate-level:
          CSINC uses add64/add32 with the +1 done as add_64bit(Rm, zero, carry_in=1)
          CSINV uses not64/not32
          CSNEG uses sub64/sub32 with zero operand: 0 - Rm = NOT(Rm) + 1
        """
        sf = dec.sf
        Rd = dec.Rd
        Rn = dec.Rn
        Rm = dec.Rm
        op = dec.opcode
        width = 64 if sf else 32

        if _condition_holds(dec.cond, self._nzcv):
            result_bits = self._read_bits(Rn, sf)
        else:
            rm_bits = self._read_bits(Rm, sf)

            if op == "CSEL":
                result_bits = rm_bits

            elif op == "CSINC":
                # Rm + 1: add zero with carry_in=1
                zero_bits = [0] * width
                if sf:
                    res = add64(rm_bits, zero_bits, carry_in=1)
                else:
                    res = add32(rm_bits[:32], zero_bits, carry_in=1)
                result_bits = int_to_bits(res.result, width)

            elif op == "CSINV":
                # NOT(Rm)
                if sf:
                    result_bits = not_64bit(rm_bits)
                else:
                    result_bits = not_32bit(rm_bits[:32])

            elif op == "CSNEG":
                # -Rm = NOT(Rm) + 1 = 0 - Rm (via sub)
                zero_bits = [0] * width
                if sf:
                    res = sub64(zero_bits, rm_bits)
                else:
                    res = sub32(zero_bits, rm_bits[:32])
                result_bits = int_to_bits(res.result, width)

            else:
                self._halted = True
                self._pc = pc
                return StepTrace(pc, pc, f"ERROR: {op}", f"Unknown csel variant {op}")

        self._write_bits(Rd, result_bits, sf)
        return StepTrace(pc, next_pc, op, f"{op} X{Rd} = 0x{bits_to_int(result_bits):X}")
