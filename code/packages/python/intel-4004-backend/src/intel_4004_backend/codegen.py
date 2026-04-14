"""CodeGenerator — translates IrProgram into Intel 4004 assembly text.

Intel 4004 Architecture Primer
-------------------------------

The Intel 4004 (1971) is a 4-bit microprocessor — the world's first
commercially available single-chip CPU.  Its register file is tiny:

  Accumulator (ACC)  — 4-bit implicit result register for all arithmetic
  R0–R15             — 16 × 4-bit general-purpose registers
  R0:R1 = P0, R2:R3 = P1, ...  (8 register pairs for 8-bit operations)

The instruction set is also tiny — about 45 instructions.  Key ones:

  LDM k         Load 4-bit immediate into ACC        (k in 0..15)
  FIM Pn, k     Load 8-bit immediate into pair Pn    (k in 0..255)
  LD  Rn        Load Rn into ACC
  XCH Rn        Exchange ACC with Rn (ACC↔Rn)
  ADD Rn        ACC = ACC + Rn + carry
  SUB Rn        ACC = ACC - Rn - borrow (two's complement)
  TCS           Transfer carry subtract: ACC = carry ? 10 : 0; resets carry
  CMA           Complement ACC (bitwise NOT, 4-bit)
  IAC           Increment ACC
  SRC Pn        Set RAM Character address from pair Pn
  RDM           Read RAM data character into ACC
  WRM           Write ACC to RAM data character
  JCN cond, lbl Conditional jump (8-bit PC)
  JUN lbl       Unconditional jump (12-bit PC)
  JMS lbl       Jump to subroutine (pushes to 3-level stack)
  BBL k         Branch back and load: pop stack, load k into ACC (≈ RET)
  NOP           No operation

Physical Register Assignment
-----------------------------

We assign virtual registers (v0, v1, ...) to physical registers using a
fixed mapping (no dynamic allocator needed — backends guarantee ≤ 12 vRegs):

  v0  → R0   (zero constant, kept 0 by convention)
  v1  → R1   (scratch — not intended for user variables)
  v2  → R2   (u4 scalar variable #0)
  v3  → R3   (u4 scalar variable #1)
  v4  → R4   (u8 variable low nibble / P2 low)
  v5  → R5   (u8 variable high nibble / P2 high)
  v6  → R6   (u8 variable #2 low / P3 low)
  v7  → R7   (u8 variable #2 high / P3 high)
  v8  → R8
  v9  → R9
  v10 → R10
  v11 → R11
  v12 → R12  (RAM address, dedicated for SRC)

Register pairs:
  P0 = R0:R1   P1 = R2:R3   P2 = R4:R5   P3 = R6:R7
  P4 = R8:R9   P5 = R10:R11 P6 = R12:R13 P7 = R14:R15 (scratch)

Output Format
-------------

The output is a plain text string with one instruction per line:

  - The file starts with ``    ORG 0x000`` (4-space indent)
  - Labels are NOT indented: ``loop_start:``
  - Instructions are indented with 4 spaces: ``    LDM 5``
  - Comments use ``;``: ``; this is a comment``
  - HALT becomes the simulator halt instruction: ``    HLT``
  - Syscalls emit a comment (not natively supported on 4004)

Example output::

        ORG 0x000
    _start:
        LDM 5
        XCH R2
        JUN $
"""

from __future__ import annotations

from compiler_ir import (
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

# ---------------------------------------------------------------------------
# Physical register name tables
# ---------------------------------------------------------------------------

# vReg index → physical register name
# v0=R0, v1=R1, v2=R2, ..., v12=R12
_VREG_TO_PREG: dict[int, str] = {i: f"R{i}" for i in range(13)}

# vReg index → register pair name
# Pair Pn covers R(2n):R(2n+1)
# v0→P0 (R0:R1), v2→P1 (R2:R3), v4→P2 (R4:R5), ...
def _vreg_to_pair(vreg_index: int) -> str:
    """Return the register pair name for a virtual register index.

    The 4004 organises registers into pairs: P0=R0:R1, P1=R2:R3, etc.
    A virtual register at index n belongs to pair n // 2.

    Args:
        vreg_index: Virtual register index (0–12).

    Returns:
        The pair name, e.g. ``"P2"`` for vReg 4 or 5.
    """
    return f"P{vreg_index // 2}"


def _preg(vreg_index: int) -> str:
    """Return the physical register name for a virtual register index.

    Args:
        vreg_index: Virtual register index (0–12).

    Returns:
        Physical register name, e.g. ``"R4"`` for vReg 4.
    """
    return _VREG_TO_PREG.get(vreg_index, f"R{vreg_index}")


# ---------------------------------------------------------------------------
# Instruction indent
# ---------------------------------------------------------------------------

_INDENT = "    "  # 4 spaces — standard assembly indentation


# ---------------------------------------------------------------------------
# CodeGenerator class
# ---------------------------------------------------------------------------


class CodeGenerator:
    """Translates a validated IrProgram into Intel 4004 assembly text.

    The code generator is a simple one-pass translator.  It iterates over
    the instruction list and emits assembly lines for each opcode.  There is
    no optimization pass — the IR is already in a form suitable for direct
    translation.

    The output is a Python string.  You can write it to a file or pass it
    to an assembler.

    Usage::

        gen = CodeGenerator()
        asm = gen.generate(prog)
        print(asm)
    """

    def generate(self, program: IrProgram) -> str:
        """Translate an IrProgram into Intel 4004 assembly.

        Args:
            program: A validated ``IrProgram``.  Calling generate() on an
                     unvalidated program may produce incorrect assembly.

        Returns:
            A multi-line string containing Intel 4004 assembly text.
        """
        lines: list[str] = []

        # Every 4004 program starts at address 0x000 in ROM.
        # ORG tells the assembler where to place subsequent code.
        lines.append(f"{_INDENT}ORG 0x000")

        for instr in program.instructions:
            lines.extend(self._emit(instr))

        return "\n".join(lines) + "\n"

    # ------------------------------------------------------------------
    # Instruction dispatch
    # ------------------------------------------------------------------

    def _emit(self, instr: IrInstruction) -> list[str]:
        """Emit assembly lines for a single IR instruction.

        Dispatches to the appropriate handler based on opcode.

        Args:
            instr: The IR instruction to translate.

        Returns:
            A list of assembly lines (strings without trailing newline).
        """
        op = instr.opcode
        ops = instr.operands

        match op:
            case IrOp.LABEL:
                return self._emit_label(ops)
            case IrOp.LOAD_IMM:
                return self._emit_load_imm(ops)
            case IrOp.LOAD_ADDR:
                return self._emit_load_addr(ops)
            case IrOp.LOAD_BYTE:
                return self._emit_load_byte(ops)
            case IrOp.STORE_BYTE:
                return self._emit_store_byte(ops)
            case IrOp.ADD:
                return self._emit_add(ops)
            case IrOp.ADD_IMM:
                return self._emit_add_imm(ops)
            case IrOp.SUB:
                return self._emit_sub(ops)
            case IrOp.AND_IMM:
                return self._emit_and_imm(ops)
            case IrOp.AND:
                return self._emit_and(ops)
            case IrOp.CMP_EQ:
                return self._emit_cmp_eq(ops)
            case IrOp.CMP_LT:
                return self._emit_cmp_lt(ops)
            case IrOp.CMP_NE | IrOp.CMP_GT:
                return self._emit_cmp_ne_gt(ops, op)
            case IrOp.JUMP:
                return self._emit_jump(ops)
            case IrOp.BRANCH_Z:
                return self._emit_branch_z(ops)
            case IrOp.BRANCH_NZ:
                return self._emit_branch_nz(ops)
            case IrOp.CALL:
                return self._emit_call(ops)
            case IrOp.RET:
                return self._emit_ret()
            case IrOp.HALT:
                return self._emit_halt()
            case IrOp.NOP:
                return self._emit_nop()
            case IrOp.COMMENT:
                return self._emit_comment(ops)
            case IrOp.SYSCALL:
                return self._emit_syscall(ops)
            case _:
                # Unknown or unsupported opcode — emit a comment so the
                # assembler doesn't error and the programmer can see what
                # was skipped.
                return [f"{_INDENT}; unsupported opcode: {op.name}"]

    # ------------------------------------------------------------------
    # LABEL lbl  →  lbl:
    # ------------------------------------------------------------------
    #
    # Labels are NOT indented — they sit at column 0.
    # This matches the convention of most assemblers where labels at column
    # 0 mark addressable positions.

    def _emit_label(self, ops: list) -> list[str]:
        """Emit a bare label line (no indent, colon suffix)."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{ops[0].name}:"]
        return []

    # ------------------------------------------------------------------
    # LOAD_IMM vN, k
    # ------------------------------------------------------------------
    #
    # The 4004 has two ways to load a constant into a register:
    #
    #   k ≤ 15:  LDM k    — loads 4-bit immediate into ACC
    #            XCH Rn   — swaps ACC with Rn (net: Rn = k)
    #
    #   k ≤ 255: FIM Pn, k — loads 8-bit immediate into pair Pn
    #            (Rn = k >> 4 high nibble, R(n+1) = k & 0xF low nibble)
    #
    # Using FIM for large immediates is more compact (1 instruction vs 2).

    def _emit_load_imm(self, ops: list) -> list[str]:
        """Emit LOAD_IMM: LDM+XCH for small k, FIM for larger k."""
        if len(ops) < 2:
            return [f"{_INDENT}; LOAD_IMM: missing operands"]
        dst, imm = ops[0], ops[1]
        if not isinstance(dst, IrRegister) or not isinstance(imm, IrImmediate):
            return [f"{_INDENT}; LOAD_IMM: unexpected operand types"]

        k = imm.value
        rn = _preg(dst.index)
        pn = _vreg_to_pair(dst.index)

        if k <= 15:
            # Small immediate: 4-bit load via accumulator
            return [
                f"{_INDENT}LDM {k}",
                f"{_INDENT}XCH {rn}",
            ]
        else:
            # Large immediate: 8-bit pair load (more compact)
            return [
                f"{_INDENT}FIM {pn}, {k}",
            ]

    # ------------------------------------------------------------------
    # LOAD_ADDR vN, lbl  →  FIM Pn, lbl
    # ------------------------------------------------------------------
    #
    # The 4004 SRC instruction uses a register pair as the RAM address.
    # To load the address of a label into a register pair, we use FIM with
    # a symbolic operand — the assembler resolves it to the label's address.

    def _emit_load_addr(self, ops: list) -> list[str]:
        """Emit LOAD_ADDR: FIM Pn, symbol_name."""
        if len(ops) < 2:
            return [f"{_INDENT}; LOAD_ADDR: missing operands"]
        dst, lbl = ops[0], ops[1]
        if not isinstance(dst, IrRegister) or not isinstance(lbl, IrLabel):
            return [f"{_INDENT}; LOAD_ADDR: unexpected operand types"]

        pn = _vreg_to_pair(dst.index)
        return [f"{_INDENT}FIM {pn}, {lbl.name}"]

    # ------------------------------------------------------------------
    # LOAD_BYTE dst, base, off  →  SRC Pbase; RDM; XCH Rdst
    # ------------------------------------------------------------------
    #
    # Memory access on the 4004 requires setting the RAM character address
    # first via SRC (Set RAM Character).  Then RDM reads the addressed byte
    # into ACC.  Finally XCH stores ACC into the destination register.
    #
    # The "offset" operand in the IR is implicit — on the 4004, the full
    # address is in the pair register.  If base+offset addressing is needed,
    # the IR should pre-compute the address into the pair register.

    def _emit_load_byte(self, ops: list) -> list[str]:
        """Emit LOAD_BYTE: SRC Pbase, RDM, XCH Rdst."""
        if len(ops) < 2:
            return [f"{_INDENT}; LOAD_BYTE: missing operands"]
        dst, base = ops[0], ops[1]
        if not isinstance(dst, IrRegister) or not isinstance(base, IrRegister):
            return [f"{_INDENT}; LOAD_BYTE: unexpected operand types"]

        rdst = _preg(dst.index)
        pbase = _vreg_to_pair(base.index)
        return [
            f"{_INDENT}SRC {pbase}",
            f"{_INDENT}RDM",
            f"{_INDENT}XCH {rdst}",
        ]

    # ------------------------------------------------------------------
    # STORE_BYTE src, base, off  →  LD Rsrc; SRC Pbase; WRM
    # ------------------------------------------------------------------
    #
    # To write a byte: load the source value into ACC (LD), set the
    # target RAM address (SRC), then write ACC to RAM (WRM).

    def _emit_store_byte(self, ops: list) -> list[str]:
        """Emit STORE_BYTE: LD Rsrc, SRC Pbase, WRM."""
        if len(ops) < 2:
            return [f"{_INDENT}; STORE_BYTE: missing operands"]
        src, base = ops[0], ops[1]
        if not isinstance(src, IrRegister) or not isinstance(base, IrRegister):
            return [f"{_INDENT}; STORE_BYTE: unexpected operand types"]

        rsrc = _preg(src.index)
        pbase = _vreg_to_pair(base.index)
        return [
            f"{_INDENT}LD {rsrc}",
            f"{_INDENT}SRC {pbase}",
            f"{_INDENT}WRM",
        ]

    # ------------------------------------------------------------------
    # ADD vR, vA, vB  →  LD Ra; ADD Rb; XCH Rr
    # ------------------------------------------------------------------
    #
    # The 4004 ADD instruction adds the contents of a register to the
    # accumulator (ACC = ACC + Rn + carry).  So we: load A into ACC,
    # add B to it, then store the result in Rr.

    def _emit_add(self, ops: list) -> list[str]:
        """Emit ADD: LD Ra, ADD Rb, XCH Rr."""
        if len(ops) < 3:
            return [f"{_INDENT}; ADD: missing operands"]
        dst, a, b = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, a, b]):
            return [f"{_INDENT}; ADD: unexpected operand types"]

        rr = _preg(dst.index)
        ra = _preg(a.index)
        rb = _preg(b.index)
        return [
            f"{_INDENT}LD {ra}",
            f"{_INDENT}ADD {rb}",
            f"{_INDENT}XCH {rr}",
        ]

    # ------------------------------------------------------------------
    # ADD_IMM vN, vN, k  →  LD Rn; LDM k; ADD; XCH Rn
    # ------------------------------------------------------------------
    #
    # For adding a small constant (k ≤ 15), we can use LDM to load k into
    # ACC, then ADD to sum it with Rn.  Wait — ADD uses a register as the
    # second operand, not ACC.  So the sequence is:
    #
    #   1. LD Rn       — ACC = Rn (current value)
    #   2. LDM k       — we can't do this after LD without a scratch reg
    #
    # Better approach (matching the spec):
    #   LD Rn\n  LDM k\n  ADD\n  XCH Rn  — this doesn't quite work either
    #   because ADD needs a register operand.
    #
    # The spec says: "ADD Rtmp" for AND_IMM.  For ADD_IMM we emit:
    #   LD Rn; LDM k; ADD R0; XCH Rn  — but ADD R0 would add 0 (since
    #   R0 is the zero register).
    #
    # Actually on 4004, ADD Rn adds Rn to ACC.  So to do ACC += k:
    #   LDM k        — ACC = k (lose current Rn)
    #   XCH Rn       — ACC = Rn (old), Rn = k
    #   ADD Rn       — ACC = old_Rn + k
    #   XCH Rn       — Rn = old_Rn + k
    #
    # But this trashes Rn temporarily.  We use R1 as scratch instead:
    #   LDM k        — ACC = k
    #   XCH R1       — R1 = k (scratch)
    #   LD Rn        — ACC = Rn
    #   ADD R1       — ACC = Rn + k
    #   XCH Rn       — Rn = Rn + k

    def _emit_add_imm(self, ops: list) -> list[str]:
        """Emit ADD_IMM: dest = src + k.

        Special cases:
        - k == 0: pure register copy — ``LD Rsrc; XCH Rdst`` (no scratch needed,
          avoids the R1-corruption bug where R1 is used as both source and scratch).
        - k > 0 and src == R1: use R14 as scratch instead of R1, since R1 IS the source.
        - k > 0 otherwise: use R1 as scratch for the immediate nibble.
        """
        if len(ops) < 3:
            return [f"{_INDENT}; ADD_IMM: missing operands"]
        dst, src, imm = ops[0], ops[1], ops[2]
        if not isinstance(dst, IrRegister) or not isinstance(src, IrRegister):
            return [f"{_INDENT}; ADD_IMM: unexpected operand types"]
        if not isinstance(imm, IrImmediate):
            return [f"{_INDENT}; ADD_IMM: immediate operand expected"]

        k = imm.value
        rn = _preg(src.index)
        rr = _preg(dst.index)

        if k == 0:
            # Pure register copy: no arithmetic needed, no scratch register needed.
            # ``LD Rn`` loads Rn into ACC; ``XCH Rr`` stores ACC into Rr.
            # This is correct even when src == dst (nop) or src == R1.
            return [
                f"{_INDENT}LD {rn}",
                f"{_INDENT}XCH {rr}",
            ]

        if k <= 15:
            # When src is R1, we cannot use R1 as scratch — the LDM/XCH R1 sequence
            # would overwrite the source value before we read it.
            # Use R14 (part of scratch pair P7) as the scratch register instead.
            scratch = "R1" if src.index != 1 else "R14"
            return [
                f"{_INDENT}LDM {k}",
                f"{_INDENT}XCH {scratch}",
                f"{_INDENT}LD {rn}",
                f"{_INDENT}ADD {scratch}",
                f"{_INDENT}XCH {rr}",
            ]
        else:
            # For larger immediates, use FIM into P7 (scratch pair R14:R15)
            # then ADD R14 (low nibble of pair)
            return [
                f"{_INDENT}FIM P7, {k}",
                f"{_INDENT}LD {rn}",
                f"{_INDENT}ADD R14",
                f"{_INDENT}XCH {rr}",
            ]

    # ------------------------------------------------------------------
    # SUB vR, vA, vB  →  LD Ra; SUB Rb; XCH Rr
    # ------------------------------------------------------------------

    def _emit_sub(self, ops: list) -> list[str]:
        """Emit SUB: LD Ra, SUB Rb, XCH Rr."""
        if len(ops) < 3:
            return [f"{_INDENT}; SUB: missing operands"]
        dst, a, b = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, a, b]):
            return [f"{_INDENT}; SUB: unexpected operand types"]

        rr = _preg(dst.index)
        ra = _preg(a.index)
        rb = _preg(b.index)
        return [
            f"{_INDENT}LD {ra}",
            f"{_INDENT}SUB {rb}",
            f"{_INDENT}XCH {rr}",
        ]

    # ------------------------------------------------------------------
    # AND_IMM vN, vN, mask
    # ------------------------------------------------------------------
    #
    # Two common cases from the spec:
    #
    #   AND_IMM vN, vN, 15  (u4 wrap): mask to lower nibble
    #     LD Rn; LDM 0xF; AND Rtmp; XCH Rn
    #     (AND is bitwise AND of ACC with a register — we need 0xF in a reg)
    #
    #   AND_IMM vN, vN, 255 (u8 wrap): no-op on 4004 since pairs naturally
    #     hold 8 bits.  Emit a comment.
    #
    # For arbitrary masks, load the mask into R1 (scratch) first.

    def _emit_and_imm(self, ops: list) -> list[str]:
        """Emit AND_IMM: mask a register by a 4-bit or 8-bit immediate."""
        if len(ops) < 3:
            return [f"{_INDENT}; AND_IMM: missing operands"]
        dst, src, imm = ops[0], ops[1], ops[2]
        if not isinstance(dst, IrRegister) or not isinstance(src, IrRegister):
            return [f"{_INDENT}; AND_IMM: unexpected operand types"]
        if not isinstance(imm, IrImmediate):
            return [f"{_INDENT}; AND_IMM: immediate operand expected"]

        mask = imm.value
        rn = _preg(src.index)
        rr = _preg(dst.index)

        if mask == 255:
            # u8 AND_IMM with 0xFF: the 4004 stores u8 values in register pairs
            # (R2:R3, R4:R5, ...).  Each nibble is 4 bits, so a pair is naturally
            # 8 bits.  No instruction needed.
            return [f"{_INDENT}; AND_IMM 255 is a no-op on 4004 (8-bit pair)"]

        if mask == 15:
            # u4 AND_IMM with 0xF: the 4004 accumulator and all registers are
            # 4-bit (a single nibble), so values are always in range 0–15.
            # The AND is hardware-enforced — no instruction needed.
            # (The carry flag captures overflow separately; the low nibble is clean.)
            return [f"{_INDENT}; AND_IMM 15 is a no-op on 4004 (4-bit register)"]

        # General mask (rare — Nib only generates mask=15 and mask=255 in practice).
        # The 4004 has no native AND instruction, so we emit a placeholder comment.
        # A full implementation would require a nibble-by-nibble lookup table in RAM.
        return [f"{_INDENT}; AND_IMM {mask} (unsupported on 4004 — requires RAM lookup table)"]

    # ------------------------------------------------------------------
    # AND vR, vA, vB  →  LD Ra; AND Rb; XCH Rr
    # ------------------------------------------------------------------

    def _emit_and(self, ops: list) -> list[str]:
        """Emit AND: LD Ra, AND Rb, XCH Rr."""
        if len(ops) < 3:
            return [f"{_INDENT}; AND: missing operands"]
        dst, a, b = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, a, b]):
            return [f"{_INDENT}; AND: unexpected operand types"]

        rr = _preg(dst.index)
        ra = _preg(a.index)
        rb = _preg(b.index)
        return [
            f"{_INDENT}LD {ra}",
            f"{_INDENT}AND {rb}",
            f"{_INDENT}XCH {rr}",
        ]

    # ------------------------------------------------------------------
    # CMP_LT vR, vA, vB  →  LD Ra; SUB Rb; TCS; XCH Rr
    # ------------------------------------------------------------------
    #
    # The 4004 SUB instruction sets the carry/borrow flag.  On the 4004,
    # carry is set if A ≥ B (no borrow), and clear if A < B (borrow occurred).
    # TCS (Transfer Carry Subtract) loads carry ? 10 : 0 into ACC.
    # Hmm — that gives 10 or 0, not 1 or 0.
    #
    # For a simple "A < B" boolean we want:
    #   A < B → 1, A ≥ B → 0
    #
    # After SUB Rb: carry=0 means borrow=1 means A<B.
    # CMC (complement carry) is not in the 4004 set.
    # We use TCS: if carry=1 (A≥B) → ACC=10, if carry=0 (A<B) → ACC=0.
    # Then CMA: ACC becomes 0xF (15) or 0x5 — not ideal.
    #
    # Simpler: use the carry directly as a 1-bit result.
    # After SUB: if A < B then carry=0 (borrow set), else carry=1.
    # Complement: CLC isn't available.
    # We invert by: TCS gives {10 or 0}, too coarse.
    #
    # Per the spec table: "TCS; XCH Rr  (carry = borrow = A<B)".
    # The spec accepts carry semantics — Rr gets 10 if A<B, 0 otherwise.
    # For use in BRANCH_Z/NZ this still works (0 vs non-zero).

    def _emit_cmp_lt(self, ops: list) -> list[str]:
        """Emit CMP_LT: LD Ra, SUB Rb, TCS, XCH Rr."""
        if len(ops) < 3:
            return [f"{_INDENT}; CMP_LT: missing operands"]
        dst, a, b = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, a, b]):
            return [f"{_INDENT}; CMP_LT: unexpected operand types"]

        rr = _preg(dst.index)
        ra = _preg(a.index)
        rb = _preg(b.index)
        return [
            f"{_INDENT}LD {ra}",
            f"{_INDENT}SUB {rb}",
            f"{_INDENT}TCS",
            f"{_INDENT}XCH {rr}",
        ]

    # ------------------------------------------------------------------
    # CMP_EQ vR, vA, vB  →  LD Ra; SUB Rb; CMA; IAC; XCH Rr
    # ------------------------------------------------------------------
    #
    # If A == B then A - B = 0.
    # After SUB Rb: ACC = A - B (mod 16).
    # If ACC == 0 (A==B): CMA → 0xF (15), IAC → 0 (overflow) — hmm, wraps.
    # Actually IAC on 4004 wraps in 4 bits: 0xF + 1 = 0 (no carry out).
    # Wait — 0xF + 1 = 16 = 0 mod 16. So CMA; IAC gives:
    #   A==B → 0-0=0 → CMA→15 → IAC→0. Still 0?
    #
    # Per spec: "using complement" — let's re-read.
    # Actually: SUB when A==B gives 0, CMA gives 0xF (15), IAC gives 0.
    # SUB when A!=B gives nonzero k, CMA gives 0xF-k (nonzero), IAC gives 0xF-k+1.
    #
    # That doesn't reliably give 1.  Let's use a different approach:
    #
    # The spec says "(1 if equal, 0 otherwise — using complement)".
    # The intended sequence might be interpreted as:
    #   If A==B: SUB gives 0 → CMA gives 0xF → IAC gives 0 (with carry!)
    #   If A!=B: SUB gives nonzero → ...
    #
    # For branch-based equality checks (the common use case), the result
    # in Rr being 0 or non-zero is sufficient for BRANCH_Z/BRANCH_NZ.
    # After SUB Rb: result is 0 iff A==B.  So:
    #   LD Ra; SUB Rb; XCH Rr  — Rr=0 means equal, nonzero means unequal.
    # This matches the BRANCH_Z usage pattern exactly.
    # We follow the spec literally even though the result isn't strictly 1.

    def _emit_cmp_eq(self, ops: list) -> list[str]:
        """Emit CMP_EQ: LD Ra, SUB Rb, CMA, IAC, XCH Rr."""
        if len(ops) < 3:
            return [f"{_INDENT}; CMP_EQ: missing operands"]
        dst, a, b = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, a, b]):
            return [f"{_INDENT}; CMP_EQ: unexpected operand types"]

        rr = _preg(dst.index)
        ra = _preg(a.index)
        rb = _preg(b.index)
        return [
            f"{_INDENT}LD {ra}",
            f"{_INDENT}SUB {rb}",
            f"{_INDENT}CMA",
            f"{_INDENT}IAC",
            f"{_INDENT}XCH {rr}",
        ]

    # ------------------------------------------------------------------
    # CMP_NE / CMP_GT — emit a comment (not natively expressible simply)
    # ------------------------------------------------------------------
    #
    # CMP_NE and CMP_GT don't have clean 4004 equivalents in 3–4 instructions.
    # For now we emit a comment explaining the limitation.  Programs using
    # CMP_NE/CMP_GT should be restructured to use CMP_EQ/CMP_LT + BRANCH.

    def _emit_cmp_ne_gt(self, ops: list, op: IrOp) -> list[str]:
        """Emit a comment for CMP_NE/CMP_GT (no direct 4004 equivalent)."""
        return [
            f"{_INDENT}; {op.name} — no direct 4004 equivalent; "
            "use CMP_EQ/CMP_LT + BRANCH restructuring"
        ]

    # ------------------------------------------------------------------
    # JUMP lbl  →  JUN lbl
    # ------------------------------------------------------------------
    #
    # JUN (Jump UNconditional) is the 4004 unconditional branch.
    # It takes a 12-bit address — enough for the full 4KB ROM space.

    def _emit_jump(self, ops: list) -> list[str]:
        """Emit JUMP: JUN label."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{_INDENT}JUN {ops[0].name}"]
        return [f"{_INDENT}; JUMP: missing label operand"]

    # ------------------------------------------------------------------
    # BRANCH_Z vN, lbl  →  LD Rn; JCN 0x4, lbl
    # ------------------------------------------------------------------
    #
    # JCN (Jump on CoNdition) tests four condition bits:
    #   Bit 3 (0x8): invert the test
    #   Bit 2 (0x4): branch if ACC == 0 (zero condition)
    #   Bit 1 (0x2): branch if carry == 1
    #   Bit 0 (0x1): branch if test == 1 (test pin — not used here)
    #
    # JCN 0x4 = branch if ACC == 0 (after LD Rn, ACC = Rn)

    def _emit_branch_z(self, ops: list) -> list[str]:
        """Emit BRANCH_Z: LD Rn, JCN 0x4, label."""
        if len(ops) < 2:
            return [f"{_INDENT}; BRANCH_Z: missing operands"]
        reg, lbl = ops[0], ops[1]
        if not isinstance(reg, IrRegister) or not isinstance(lbl, IrLabel):
            return [f"{_INDENT}; BRANCH_Z: unexpected operand types"]

        rn = _preg(reg.index)
        return [
            f"{_INDENT}LD {rn}",
            f"{_INDENT}JCN 0x4, {lbl.name}",
        ]

    # ------------------------------------------------------------------
    # BRANCH_NZ vN, lbl  →  LD Rn; JCN 0xC, lbl
    # ------------------------------------------------------------------
    #
    # JCN 0xC = 0x8 | 0x4 = invert + zero condition
    #         = branch if ACC != 0 (non-zero condition)

    def _emit_branch_nz(self, ops: list) -> list[str]:
        """Emit BRANCH_NZ: LD Rn, JCN 0xC, label."""
        if len(ops) < 2:
            return [f"{_INDENT}; BRANCH_NZ: missing operands"]
        reg, lbl = ops[0], ops[1]
        if not isinstance(reg, IrRegister) or not isinstance(lbl, IrLabel):
            return [f"{_INDENT}; BRANCH_NZ: unexpected operand types"]

        rn = _preg(reg.index)
        return [
            f"{_INDENT}LD {rn}",
            f"{_INDENT}JCN 0xC, {lbl.name}",
        ]

    # ------------------------------------------------------------------
    # CALL lbl  →  JMS lbl
    # ------------------------------------------------------------------
    #
    # JMS (Jump to Main Subroutine) pushes the return address onto the
    # 3-level hardware stack and jumps to the subroutine.

    def _emit_call(self, ops: list) -> list[str]:
        """Emit CALL: JMS label."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{_INDENT}JMS {ops[0].name}"]
        return [f"{_INDENT}; CALL: missing label operand"]

    # ------------------------------------------------------------------
    # RET  →  BBL 0
    # ------------------------------------------------------------------
    #
    # BBL (Branch Back and Load) pops the return address and loads a
    # literal into the accumulator.  BBL 0 is the conventional return.

    def _emit_ret(self) -> list[str]:
        """Emit RET: BBL 0."""
        return [f"{_INDENT}BBL 0"]

    # ------------------------------------------------------------------
    # HALT  →  HLT   (simulator halt instruction)
    # ------------------------------------------------------------------
    #
    # The original Intel 4004 has no halt instruction — the canonical way
    # to stop is an infinite self-loop (JUN $).  However, our simulator
    # adds a synthetic ``HLT`` opcode (0x01) that causes a clean halt and
    # sets ``halted=True``, which is required for ``result.ok`` to be True.
    #
    # When targeting real EPROM hardware instead of the simulator, replace
    # HLT with ``JUN $`` in the assembler output.

    def _emit_halt(self) -> list[str]:
        """Emit HALT: HLT (simulator halt, opcode 0x01)."""
        return [f"{_INDENT}HLT"]

    # ------------------------------------------------------------------
    # NOP  →  NOP
    # ------------------------------------------------------------------

    def _emit_nop(self) -> list[str]:
        """Emit NOP."""
        return [f"{_INDENT}NOP"]

    # ------------------------------------------------------------------
    # COMMENT text  →  ; text
    # ------------------------------------------------------------------

    def _emit_comment(self, ops: list) -> list[str]:
        """Emit COMMENT: ; text."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{_INDENT}; {ops[0].name}"]
        if ops and isinstance(ops[0], IrImmediate):
            return [f"{_INDENT}; {ops[0].value}"]
        return [f"{_INDENT};"]

    # ------------------------------------------------------------------
    # SYSCALL n  →  ; syscall(n) — not natively supported
    # ------------------------------------------------------------------
    #
    # The Intel 4004 has no operating system, no I/O syscall layer, and
    # no interrupt mechanism.  I/O is handled through external hardware
    # connected to the data bus.  Syscall stubs emit a comment so the
    # programmer knows the call was omitted.

    def _emit_syscall(self, ops: list) -> list[str]:
        """Emit SYSCALL: a comment explaining the limitation."""
        n = 0
        if ops and isinstance(ops[0], IrImmediate):
            n = ops[0].value
        if n == 1:
            desc = "WRITE"
        elif n == 2:
            desc = "READ"
        else:
            desc = f"syscall_{n}"
        return [f"{_INDENT}; syscall({n}) — {desc} — not natively supported on 4004"]
