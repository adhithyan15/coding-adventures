"""CodeGenerator — translates IrProgram into Intel 8008 assembly text.

Intel 8008 Architecture Primer
-------------------------------

The Intel 8008 (1972) is an 8-bit microprocessor — the world's first
single-chip 8-bit CPU.  It has a small but capable register file:

  A     — Accumulator: 8-bit implicit result register for all ALU ops.
           Every arithmetic and logical instruction uses A as one operand
           and places the result in A.  A is scratch — not preserved.
  B, C, D, E — Four 8-bit general-purpose data registers.
  H, L  — High and Low bytes of the 14-bit memory address register.
           H:L = (H << 8) | L.  Used for all memory operations via M.
  M     — Pseudo-register: the memory byte at address H:L.
           MOV A, M reads; MOV M, A writes.  H:L must be set first.

Key 8008 instructions:
  MVI  r, d8    Load 8-bit immediate into register r (1 byte → A,B,C,D,E,H,L)
  MOV  dst, src Copy register src into register dst
  ADD  r        A ← A + r            (sets Z, S, P, CY)
  ADC  r        A ← A + r + CY       (add with carry)
  SUB  r        A ← A − r            (CY = borrow)
  SBB  r        A ← A − r − CY      (subtract with borrow)
  ANA  r        A ← A AND r         (CY = 0)
  ORA  r        A ← A OR r          (CY = 0)
  XRA  r        A ← A XOR r         (CY = 0)
  CMP  r        flags ← A − r; A unchanged
  ADI  d8       A ← A + d8
  ACI  d8       A ← A + d8 + CY
  CPI  d8       flags ← A − d8; A unchanged
  XRI  d8       A ← A XOR d8
  RLC           rotate A left circular:  CY ← bit7, bit0 ← old bit7
  RRC           rotate A right circular: CY ← bit0, bit7 ← old bit0
  RAL           rotate A left through carry:  CY ← bit7, bit0 ← old CY
  RAR           rotate A right through carry: CY ← bit0, bit7 ← old CY
  IN   p        A ← input port p (p ∈ 0–7)
  OUT  p        output port p ← A (p ∈ 0–23)
  JMP  a14      unconditional jump (3 bytes)
  CAL  a14      call subroutine: push PC, jump to a14 (3 bytes)
  RFC           return if carry false — standard unconditional return
  JTZ  a14      jump if Z=1 (result was zero)
  JFZ  a14      jump if Z=0 (result was non-zero)
  JTC  a14      jump if CY=1 (carry set / borrow)
  JFP  a14      jump if P=0 (parity odd)
  HLT           halt the processor (0xFF)
  ORG  addr     assembler directive: set location counter

Physical Register Assignment
-----------------------------

Virtual registers (v0, v1, v2, ...) map to 8008 physical registers via a
fixed, validation-enforced table (the validator rejects programs with more
than 6 distinct virtual registers):

  v0 → B   (constant zero, preloaded to 0 at _start)
  v1 → C   (scratch / return value register)
  v2 → D   (1st local / 1st argument slot)
  v3 → E   (2nd local / 2nd argument slot)
  v4 → H   (3rd local / 3rd argument slot — careful: memory ops use H:L)
  v5 → L   (4th local / 4th argument slot — careful: memory ops use H:L)

The accumulator A is implicit in all ALU operations.  A is never assigned
a virtual register index — it is always scratch.

Calling Convention (Oct → 8008)
---------------------------------

Arguments are passed in B, C, D, E (v0, v1, v2, v3 in the oct-ir-compiler's
v-register numbering).  The oct-ir-compiler stages args into v2, v3, v4, v5
before a CALL; because the calling convention uses the same register slots as
the local variables, this works correctly.

Return value: placed in C (v1) by the callee, then moved to A immediately
before RFC so the caller reads it from A.  The caller may then MOV its
destination register from A.

LOAD_ADDR / LOAD_BYTE / STORE_BYTE Memory Access
-------------------------------------------------

Static variables live in the 8 KB RAM region (0x2000–0x3FFE).  The 8008
accesses RAM only through the M pseudo-register (memory at H:L).

The IR emits a LOAD_ADDR + LOAD_BYTE pair to read a static:

  LOAD_ADDR v1, symbol    -- set H:L = address of symbol
  LOAD_BYTE v1, v1, v0    -- v1 = RAM[H:L + 0] = RAM[H:L]

The code generator:
  1. For LOAD_ADDR  → emits  MVI H, hi(symbol); MVI L, lo(symbol)
     (H and L are updated; the "destination register" operand is ignored
      because H:L is the implicit address register on the 8008.)
  2. For LOAD_BYTE  → emits  MVI A, 0; ADD M; MOV Rdst, A   (safe group-10 path; MOV A, M = 0x7E = CAL!)
  3. For STORE_BYTE → emits  MOV A, Rsrc; MOV M, A

hi(addr) = (addr >> 8) & 0x3F   (high 6 bits of 14-bit address)
lo(addr) = addr & 0xFF           (low 8 bits)

The assembler resolves symbolic addresses; the code generator emits
``hi(symbol_name)`` and ``lo(symbol_name)`` as textual directives that the
assembler expands.

Comparison Materialisation
---------------------------

The 8008 CMP instruction sets flags but places no result in a register.
To materialise a boolean result (0 or 1) into a destination register, the
code generator emits an optimistic-load / conditional-branch sequence:

  CMP_EQ Rdst, Ra, Rb:
    MOV  A, Ra
    CMP  Rb            ; Z=1 iff Ra == Rb
    MVI  Rdst, 1       ; assume equal
    JTZ  cmp_N_done    ; if Z (equal) → keep 1
    MVI  Rdst, 0       ; else overwrite with 0
  cmp_N_done:

Each comparison gets a unique suffix ``N`` (monotonically incrementing)
so labels do not collide within or across functions.

Output Format
-------------

The output is a plain string.  One assembly line per line::

    ORG 0x0000
_start:
    MVI  B, 0
    CAL  _fn_main
    HLT
_fn_main:
    MVI  D, 42
    MOV  A, D
    RFC

  - ORG directive at address 0x0000 (4-space indent)
  - Labels: column 0, colon suffix
  - Instructions: 4-space indent, mnemonic left-aligned, operands after a space
  - Immediate values: decimal (e.g. ``42``) or hex with ``0x`` prefix (e.g. ``0xFF``)
  - Comments start with ``; ``
"""

from __future__ import annotations

from dataclasses import dataclass, field

from compiler_ir import (
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

# ---------------------------------------------------------------------------
# Physical register table
# ---------------------------------------------------------------------------
#
# Virtual register index → 8008 physical register name.
# v0=B (zero constant), v1=C (scratch/return), v2=D..v5=L.
# A is the implicit accumulator and is never an IR register.

_VREG_TO_PREG: dict[int, str] = {
    0: "B",  # zero constant
    1: "C",  # scratch / return value
    2: "D",  # 1st local / 1st arg
    3: "E",  # 2nd local / 2nd arg
    4: "H",  # 3rd local / 3rd arg  (careful: H is also memory high byte)
    5: "L",  # 4th local / 4th arg  (careful: L is also memory low byte)
}

# Fixed registers used in SYSCALL expansions.
# ADC/SBB/rotation args are staged into v2=D, v3=E by the IR compiler.
# Results land in v1=C.
_REG_ARG0 = "D"   # v2 — first syscall argument
_REG_ARG1 = "E"   # v3 — second syscall argument
_REG_RESULT = "C"  # v1 — syscall result

# Assembly indentation: 4 spaces.
_INDENT = "    "


def _preg(vreg_index: int) -> str:
    """Return the physical 8008 register name for a virtual register index.

    Args:
        vreg_index: Virtual register index (0–5, or beyond for error cases).

    Returns:
        Physical register name, e.g. ``"D"`` for vReg 2.
        Falls back to ``"B"`` for unmapped indices (should not happen on valid IR).
    """
    return _VREG_TO_PREG.get(vreg_index, "B")


def _load_a(reg: str) -> list[str]:
    """Emit the shortest safe sequence that loads physical register ``reg`` into A.

    Why not ``MOV A, {reg}`` for all registers?
    --------------------------------------------
    The Intel 8008 opcode space has three ``MOV A, *`` slots that are NOT
    register copies — they are occupied by other instructions:

        MOV A, C = 0x79 = 01_111_001
            → ``IN 7`` (read input port 7).  SSS=001 in group-01 is always IN.

        MOV A, H = 0x7C = 01_111_100
            → ``JMP`` (unconditional jump, 3-byte!).  Fetches the next 2 bytes
              as an address and jumps there — catastrophic.

        MOV A, M = 0x7E = 01_111_110
            → ``CAL`` (subroutine call, 3-byte!).  Pushes the PC and jumps to
              the address in the next 2 bytes — equally catastrophic.

    Safe workaround for all three — use the group-10 ALU path:

        MVI  A, 0      ; A ← 0   (does not modify flags)
        ADD  {reg}     ; A ← 0 + {reg} = {reg}   (CY = 0 since result ≤ 255)

    In group-10, SSS field 001=C, 100=H, 110=M are correctly decoded as
    register/memory reads without any hardware conflicts.

    All other registers (B=0, D=2, E=3, L=5, A=7) are safe via ``MOV A, {reg}``.

    Args:
        reg: Physical 8008 register name (``"B"``, ``"C"``, ``"D"``, ``"E"``,
             ``"H"``, ``"L"``, ``"M"``, or ``"A"``).

    Returns:
        A list of assembly strings (possibly empty for A, or 1–2 items otherwise).
    """
    if reg == "A":
        return []  # already in accumulator, no instruction needed
    if reg in ("C", "H", "M"):
        # Dangerous: MOV A, C → IN 7 / MOV A, H → JMP / MOV A, M → CAL
        # All three conflict with other instructions in group-01.
        # Fix: use group-10 ALU where SSS is always interpreted as a register.
        return [
            f"{_INDENT}MVI  A, 0",    # clear A without touching flags
            f"{_INDENT}ADD  {reg}",   # A = 0 + {reg} = {reg}  (CY = 0)
        ]
    return [f"{_INDENT}MOV  A, {reg}"]


# ---------------------------------------------------------------------------
# CodeGenerator
# ---------------------------------------------------------------------------


@dataclass
class CodeGenerator:
    """Translate a validated IrProgram into Intel 8008 assembly text.

    The generator is a one-pass, instruction-by-instruction translator.
    It holds a label counter (``_label_count``) used to produce unique
    local labels for comparison materialisation and the parity intrinsic.

    Usage::

        gen = CodeGenerator()
        asm_text = gen.generate(prog)
        # write asm_text to a .asm file or pass to intel-8008-assembler
    """

    _label_count: int = field(default=0, init=False)

    def generate(self, program: IrProgram) -> str:
        """Translate an IrProgram into Intel 8008 assembly text.

        Walks the instruction list and emits 8008 assembly for each IR opcode.
        Precedes the instruction stream with the ``ORG 0x0000`` origin directive.

        Args:
            program: A validated ``IrProgram``.  Calling ``generate()`` on an
                     unvalidated program may produce incorrect or incomplete assembly.

        Returns:
            A multi-line string of Intel 8008 assembly.
        """
        lines: list[str] = []

        # Every 8008 program begins at ROM address 0x0000.
        # ORG tells the assembler the starting location counter.
        lines.append(f"{_INDENT}ORG 0x0000")

        for instr in program.instructions:
            lines.extend(self._emit(instr))

        return "\n".join(lines) + "\n"

    # ------------------------------------------------------------------
    # Instruction dispatch
    # ------------------------------------------------------------------

    def _emit(self, instr: IrInstruction) -> list[str]:
        """Emit assembly lines for a single IR instruction.

        Args:
            instr: The IR instruction to translate.

        Returns:
            A list of assembly line strings (no trailing newline on each).
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
            case IrOp.AND:
                return self._emit_and(ops)
            case IrOp.OR:
                return self._emit_or(ops)
            case IrOp.XOR:
                return self._emit_xor(ops)
            case IrOp.NOT:
                return self._emit_not(ops)
            case IrOp.CMP_EQ:
                return self._emit_cmp_eq(ops)
            case IrOp.CMP_NE:
                return self._emit_cmp_ne(ops)
            case IrOp.CMP_LT:
                return self._emit_cmp_lt(ops)
            case IrOp.CMP_GT:
                return self._emit_cmp_gt(ops)
            case IrOp.BRANCH_Z:
                return self._emit_branch_z(ops)
            case IrOp.BRANCH_NZ:
                return self._emit_branch_nz(ops)
            case IrOp.JUMP:
                return self._emit_jump(ops)
            case IrOp.CALL:
                return self._emit_call(ops)
            case IrOp.RET:
                return self._emit_ret()
            case IrOp.HALT:
                return self._emit_halt()
            case IrOp.SYSCALL:
                return self._emit_syscall(ops)
            case IrOp.NOP:
                return self._emit_nop()
            case IrOp.COMMENT:
                return self._emit_comment(ops)
            case _:
                # Unknown opcode — emit a comment and continue rather than crashing.
                return [f"{_INDENT}; unsupported opcode: {op.name}"]

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _next_label(self) -> str:
        """Generate a unique local label suffix and advance the counter."""
        lbl = f"cmp_{self._label_count}"
        self._label_count += 1
        return lbl

    # ------------------------------------------------------------------
    # LABEL name  →  name:
    # ------------------------------------------------------------------
    #
    # Labels sit at column 0 with a colon suffix.  They do not advance the
    # address counter — the assembler records the current address for this label.

    def _emit_label(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit a label definition at column 0."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{ops[0].name}:"]
        return []

    # ------------------------------------------------------------------
    # LOAD_IMM Rdst, imm  →  MVI Rdst, imm
    # ------------------------------------------------------------------
    #
    # MVI (Move Immediate) loads an 8-bit constant into any named register.
    # On the 8008 this is always a 2-byte instruction: opcode + byte operand.
    #
    # Example: LOAD_IMM v1, 42  →  MVI C, 42

    def _emit_load_imm(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit LOAD_IMM: MVI Rdst, imm."""
        if len(ops) < 2:
            return [f"{_INDENT}; LOAD_IMM: missing operands"]
        dst, imm = ops[0], ops[1]
        if not isinstance(dst, IrRegister) or not isinstance(imm, IrImmediate):
            return [f"{_INDENT}; LOAD_IMM: unexpected operand types"]
        return [f"{_INDENT}MVI  {_preg(dst.index)}, {imm.value}"]

    # ------------------------------------------------------------------
    # LOAD_ADDR Rdst, symbol  →  MVI H, hi(symbol)  /  MVI L, lo(symbol)
    # ------------------------------------------------------------------
    #
    # Static variables live in the 8008 RAM region (0x2000–0x3FFE).  To
    # access RAM, the code generator must load the 14-bit address into H:L.
    #
    # The IR instruction carries a destination virtual register (Rdst) that
    # is conceptually "the address", but on the 8008 addresses must live in
    # H:L — there is no way to store a 14-bit address in a single 8-bit
    # register.  We ignore Rdst and write H:L directly.
    #
    # The assembler resolves ``hi(symbol)`` and ``lo(symbol)`` to the high
    # and low bytes of the symbol's ROM address:
    #   hi(addr) = (addr >> 8) & 0x3F   (upper 6 bits of 14-bit address)
    #   lo(addr) = addr & 0xFF           (lower 8 bits)
    #
    # Example: LOAD_ADDR v1, counter
    #   →  MVI H, hi(counter)
    #      MVI L, lo(counter)

    def _emit_load_addr(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit LOAD_ADDR: MVI H, hi(symbol); MVI L, lo(symbol)."""
        if len(ops) < 2:
            return [f"{_INDENT}; LOAD_ADDR: missing operands"]
        lbl = ops[1]
        if not isinstance(lbl, IrLabel):
            return [f"{_INDENT}; LOAD_ADDR: label operand expected"]
        return [
            f"{_INDENT}MVI  H, hi({lbl.name})",
            f"{_INDENT}MVI  L, lo({lbl.name})",
        ]

    # ------------------------------------------------------------------
    # LOAD_BYTE Rdst, Rbase, Rzero  →  MVI A, 0 / ADD M / MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # The 8008 reads RAM through the M pseudo-register.  H:L must be
    # pointing to the target address (set by the preceding LOAD_ADDR).
    #
    # The "base" and "offset" operands from the IR are ignored here because:
    #   - The address is implicitly in H:L (set by LOAD_ADDR)
    #   - The offset v0 is always 0 (no address arithmetic needed)
    #   - H:L is the only address register on the 8008
    #
    # ⚠️  IMPORTANT: We CANNOT emit ``MOV A, M`` here!
    # M has code 110.  In Group 01 (MOV), SSS=110 is decoded as the CAL
    # (subroutine Call) instruction — a 3-byte instruction that pushes the
    # PC and jumps to the address in the next 2 bytes:
    #
    #     MOV A, M = 0x7E = 01_111_110
    #     group=01, ddd=A(7), sss=110 → CAL unconditional!
    #
    # This is catastrophic — instead of reading memory, the CPU calls an
    # arbitrary subroutine address built from the next 2 bytes (whatever
    # follows in the instruction stream).
    #
    # Fix: use the group-10 ALU path via _load_a("M"):
    #   MVI  A, 0   ; prime accumulator (A = 0)
    #   ADD  M      ; A = 0 + M = M  (group-10, sss=M=110 is safe here)
    #   MOV  Rdst, A ; copy accumulator to destination
    #
    # In Group 10 (ALU), SSS=110 correctly reads the memory byte at H:L
    # (the M pseudo-register) without triggering CAL.
    #
    # Example: LOAD_BYTE v1, v1, v0  →  MVI A, 0; ADD M; MOV C, A

    def _emit_load_byte(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit LOAD_BYTE: load M into A safely via group-10 ALU; MOV Rdst, A.

        Uses ``_load_a("M")`` (= ``MVI A, 0; ADD M``) instead of the naive
        ``MOV A, M`` which encodes as 0x7E = CAL (unconditional subroutine call)
        in group-01 — a catastrophic hardware conflict.  See class docstring.
        """
        if not ops or not isinstance(ops[0], IrRegister):
            return [f"{_INDENT}; LOAD_BYTE: missing destination register"]
        rdst = _preg(ops[0].index)
        lines = [*_load_a("M")]   # MVI A, 0; ADD M  (safe group-10 path)
        if rdst != "A":
            lines.append(f"{_INDENT}MOV  {rdst}, A")
        return lines

    # ------------------------------------------------------------------
    # STORE_BYTE Rsrc, Rbase, Rzero  →  MOV A, Rsrc  /  MOV M, A
    # ------------------------------------------------------------------
    #
    # Writes a byte to RAM at H:L (set by the preceding LOAD_ADDR).
    # Like LOAD_BYTE, Rbase and Rzero are ignored.
    #
    # Steps:
    #   1. MOV A, Rsrc — load the value into the accumulator
    #   2. MOV M, A    — write accumulator to RAM at H:L
    #
    # Example: STORE_BYTE v1, v1, v0  →  MOV A, C; MOV M, A

    def _emit_store_byte(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit STORE_BYTE: load Rsrc into A safely; MOV M, A."""
        if not ops or not isinstance(ops[0], IrRegister):
            return [f"{_INDENT}; STORE_BYTE: missing source register"]
        rsrc = _preg(ops[0].index)
        return [*_load_a(rsrc), f"{_INDENT}MOV  M, A"]

    # ------------------------------------------------------------------
    # ADD Rdst, Ra, Rb  →  MOV A, Ra  /  ADD Rb  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # The 8008 ADD instruction adds a register to the accumulator:
    #   A ← A + Rb      (sets Z, S, P, CY)
    #
    # We must first load Ra into A, then add Rb.  The result in A is moved
    # to Rdst.  If Rdst == A that final MOV is implicit — but since none
    # of our virtual registers map to A, we always emit the MOV.

    def _emit_add(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit ADD: load Ra into A safely; ADD Rb; MOV Rdst, A."""
        if len(ops) < 3:
            return [f"{_INDENT}; ADD: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; ADD: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        return [*_load_a(rra), f"{_INDENT}ADD  {rrb}", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # ADD_IMM Rdst, Ra, imm  →  MOV A, Ra  /  ADI imm  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # ADI (Add Immediate) adds an 8-bit constant to the accumulator:
    #   A ← A + d8      (sets Z, S, P, CY)
    #
    # Special case — imm == 0 (register copy):
    #   This is the IR idiom for "move register to register" (the IR has no
    #   dedicated MOV instruction for registers).  We can emit just two
    #   instructions: MOV A, Ra; MOV Rdst, A.
    #
    # Example: ADD_IMM v2, v1, 0  →  MOV A, C; MOV D, A   (copy v1 → v2)
    # Example: ADD_IMM v1, v2, 5  →  MOV A, D; ADI 5; MOV C, A

    def _emit_add_imm(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit ADD_IMM: load Ra into A safely; [ADI imm;] MOV Rdst, A."""
        if len(ops) < 3:
            return [f"{_INDENT}; ADD_IMM: missing operands"]
        dst, src, imm = ops[0], ops[1], ops[2]
        if not isinstance(dst, IrRegister) or not isinstance(src, IrRegister):
            return [f"{_INDENT}; ADD_IMM: unexpected register operands"]
        if not isinstance(imm, IrImmediate):
            return [f"{_INDENT}; ADD_IMM: immediate operand expected"]

        rdst = _preg(dst.index)
        rsrc = _preg(src.index)

        if imm.value == 0:
            # Pure register copy — no arithmetic, no immediate.
            # Load Rsrc safely into A (using _load_a to avoid MOV A, C = IN 7),
            # then store A into Rdst.
            return [*_load_a(rsrc), f"{_INDENT}MOV  {rdst}, A"]

        return [*_load_a(rsrc), f"{_INDENT}ADI  {imm.value}", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # SUB Rdst, Ra, Rb  →  MOV A, Ra  /  SUB Rb  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # The 8008 SUB instruction:  A ← A − Rb  (CY = borrow).
    # The borrow semantics are used by CMP_LT and CMP_GT.

    def _emit_sub(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit SUB: load Ra into A safely; SUB Rb; MOV Rdst, A."""
        if len(ops) < 3:
            return [f"{_INDENT}; SUB: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; SUB: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        return [*_load_a(rra), f"{_INDENT}SUB  {rrb}", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # AND Rdst, Ra, Rb  →  MOV A, Ra  /  ANA Rb  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # ANA (AND register): A ← A AND Rb (CY cleared by AND ops on the 8008).
    # Used for Oct's `&` bitwise AND operator.

    def _emit_and(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit AND: load Ra into A safely; ANA Rb; MOV Rdst, A."""
        if len(ops) < 3:
            return [f"{_INDENT}; AND: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; AND: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        return [*_load_a(rra), f"{_INDENT}ANA  {rrb}", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # OR Rdst, Ra, Rb  →  MOV A, Ra  /  ORA Rb  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # ORA (OR register): A ← A OR Rb (CY cleared).
    # Used for Oct's `|` bitwise OR operator.

    def _emit_or(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit OR: load Ra into A safely; ORA Rb; MOV Rdst, A."""
        if len(ops) < 3:
            return [f"{_INDENT}; OR: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; OR: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        return [*_load_a(rra), f"{_INDENT}ORA  {rrb}", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # XOR Rdst, Ra, Rb  →  MOV A, Ra  /  XRA Rb  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # XRA (XOR register): A ← A XOR Rb (CY cleared).
    # Used for Oct's `^` bitwise XOR operator.
    #
    # Historical note: the Nib/4004 backend had no XOR instruction and
    # emulated it via SUB.  The 8008 has native XRA, so this is a 3-instruction
    # sequence — no emulation needed.

    def _emit_xor(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit XOR: load Ra into A safely; XRA Rb; MOV Rdst, A."""
        if len(ops) < 3:
            return [f"{_INDENT}; XOR: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; XOR: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        return [*_load_a(rra), f"{_INDENT}XRA  {rrb}", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # NOT Rdst, Ra  →  MOV A, Ra  /  XRI 0xFF  /  MOV Rdst, A
    # ------------------------------------------------------------------
    #
    # The 8008 has no dedicated bitwise NOT instruction.  Instead:
    #   XRI 0xFF = XOR A with all-ones = flip every bit (bitwise NOT).
    #
    # A ← A XOR 0xFF = ~A   (all 8 bits complemented)
    #
    # This maps directly to Oct's `~` bitwise NOT operator.

    def _emit_not(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit NOT: load Ra into A safely; XRI 0xFF; MOV Rdst, A."""
        if len(ops) < 2:
            return [f"{_INDENT}; NOT: missing operands"]
        dst, ra = ops[0], ops[1]
        if not isinstance(dst, IrRegister) or not isinstance(ra, IrRegister):
            return [f"{_INDENT}; NOT: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        return [*_load_a(rra), f"{_INDENT}XRI  0xFF", f"{_INDENT}MOV  {rdst}, A"]

    # ------------------------------------------------------------------
    # CMP_EQ Rdst, Ra, Rb
    # ------------------------------------------------------------------
    #
    # Set Rdst = 1 if Ra == Rb, else 0.
    #
    # The 8008 CMP instruction sets Z=1 iff Ra == Rb (after A − Rb).
    # We use an optimistic-load approach: assume equal (load 1), then
    # branch past the "load 0" if Z was set.
    #
    # Truth table for CMP_EQ:
    #   Ra == Rb → Z=1 → JTZ taken → Rdst stays 1  ✓
    #   Ra != Rb → Z=0 → JTZ not taken → MVI Rdst, 0  ✓
    #
    # Assembly:
    #   MOV  A, Ra
    #   CMP  Rb          ; Z=1 iff Ra==Rb
    #   MVI  Rdst, 1     ; assume equal
    #   JTZ  cmp_N_done  ; if Z, skip the "not equal" path
    #   MVI  Rdst, 0
    # cmp_N_done:

    def _emit_cmp_eq(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit CMP_EQ: boolean equality materialisation."""
        if len(ops) < 3:
            return [f"{_INDENT}; CMP_EQ: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; CMP_EQ: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        done = self._next_label()
        return [
            *_load_a(rra),
            f"{_INDENT}CMP  {rrb}",
            f"{_INDENT}MVI  {rdst}, 1",
            f"{_INDENT}JTZ  {done}",
            f"{_INDENT}MVI  {rdst}, 0",
            f"{done}:",
        ]

    # ------------------------------------------------------------------
    # CMP_NE Rdst, Ra, Rb
    # ------------------------------------------------------------------
    #
    # Set Rdst = 1 if Ra != Rb, else 0.
    #
    # Opposite of CMP_EQ: assume not-equal (load 0), branch if Z=1 (equal).
    #
    #   MOV  A, Ra
    #   CMP  Rb          ; Z=1 iff Ra==Rb
    #   MVI  Rdst, 0     ; assume not-equal
    #   JTZ  cmp_N_done  ; if Z (equal) → skip, keep 0
    #   MVI  Rdst, 1
    # cmp_N_done:

    def _emit_cmp_ne(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit CMP_NE: boolean inequality materialisation."""
        if len(ops) < 3:
            return [f"{_INDENT}; CMP_NE: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; CMP_NE: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        done = self._next_label()
        return [
            *_load_a(rra),
            f"{_INDENT}CMP  {rrb}",
            f"{_INDENT}MVI  {rdst}, 0",
            f"{_INDENT}JTZ  {done}",
            f"{_INDENT}MVI  {rdst}, 1",
            f"{done}:",
        ]

    # ------------------------------------------------------------------
    # CMP_LT Rdst, Ra, Rb
    # ------------------------------------------------------------------
    #
    # Set Rdst = 1 if Ra < Rb (unsigned), else 0.
    #
    # 8008 SUB/CMP semantics: A − Rb sets CY=1 (carry/borrow) iff A < Rb
    # (unsigned subtraction borrows).  JTC = jump if carry true (CY=1).
    #
    #   MOV  A, Ra
    #   CMP  Rb          ; CY=1 iff Ra < Rb (borrow = unsigned less-than)
    #   MVI  Rdst, 1     ; assume less-than
    #   JTC  cmp_N_done  ; if CY (Ra < Rb) → keep 1
    #   MVI  Rdst, 0
    # cmp_N_done:

    def _emit_cmp_lt(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit CMP_LT: unsigned less-than materialisation."""
        if len(ops) < 3:
            return [f"{_INDENT}; CMP_LT: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; CMP_LT: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        done = self._next_label()
        return [
            *_load_a(rra),
            f"{_INDENT}CMP  {rrb}",
            f"{_INDENT}MVI  {rdst}, 1",
            f"{_INDENT}JTC  {done}",
            f"{_INDENT}MVI  {rdst}, 0",
            f"{done}:",
        ]

    # ------------------------------------------------------------------
    # CMP_GT Rdst, Ra, Rb
    # ------------------------------------------------------------------
    #
    # Set Rdst = 1 if Ra > Rb (unsigned), else 0.
    #
    # Trick: Ra > Rb ⟺ Rb < Ra.  So we swap the operands:
    #   MOV A, Rb; CMP Ra → CY=1 iff Rb < Ra iff Ra > Rb.
    #
    #   MOV  A, Rb       ; Rb in accumulator (operand swap!)
    #   CMP  Ra          ; CY=1 iff Rb < Ra i.e. Ra > Rb
    #   MVI  Rdst, 1     ; assume greater-than
    #   JTC  cmp_N_done  ; if CY → keep 1
    #   MVI  Rdst, 0
    # cmp_N_done:

    def _emit_cmp_gt(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit CMP_GT: unsigned greater-than via operand-swap trick."""
        if len(ops) < 3:
            return [f"{_INDENT}; CMP_GT: missing operands"]
        dst, ra, rb = ops[0], ops[1], ops[2]
        if not all(isinstance(o, IrRegister) for o in [dst, ra, rb]):
            return [f"{_INDENT}; CMP_GT: unexpected operand types"]
        rdst = _preg(dst.index)
        rra = _preg(ra.index)
        rrb = _preg(rb.index)
        done = self._next_label()
        return [
            *_load_a(rrb),               # note: Rb goes into A (swap!)
            f"{_INDENT}CMP  {rra}",       # CY=1 iff Rb < Ra i.e. Ra > Rb
            f"{_INDENT}MVI  {rdst}, 1",
            f"{_INDENT}JTC  {done}",
            f"{_INDENT}MVI  {rdst}, 0",
            f"{done}:",
        ]

    # ------------------------------------------------------------------
    # BRANCH_Z Rcond, lbl  →  MOV A, Rcond  /  CPI 0  /  JTZ lbl
    # ------------------------------------------------------------------
    #
    # Jump to lbl if the condition register is zero.
    #
    # CPI 0 = compare accumulator with immediate 0.  Sets Z=1 iff A == 0.
    # JTZ = jump if zero true (Z=1).
    #
    # Used for: if(!cond), while(!cond), the false branch of if/else.

    def _emit_branch_z(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit BRANCH_Z: load Rcond into A safely; CPI 0; JTZ lbl."""
        if len(ops) < 2:
            return [f"{_INDENT}; BRANCH_Z: missing operands"]
        reg, lbl = ops[0], ops[1]
        if not isinstance(reg, IrRegister) or not isinstance(lbl, IrLabel):
            return [f"{_INDENT}; BRANCH_Z: unexpected operand types"]
        rn = _preg(reg.index)
        return [*_load_a(rn), f"{_INDENT}CPI  0", f"{_INDENT}JTZ  {lbl.name}"]

    # ------------------------------------------------------------------
    # BRANCH_NZ Rcond, lbl  →  MOV A, Rcond  /  CPI 0  /  JFZ lbl
    # ------------------------------------------------------------------
    #
    # Jump to lbl if the condition register is non-zero.
    #
    # JFZ = jump if zero false (Z=0), i.e. result was not zero.
    #
    # Used for: loop-back jumps in while, logical OR short-circuit.

    def _emit_branch_nz(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit BRANCH_NZ: load Rcond into A safely; CPI 0; JFZ lbl."""
        if len(ops) < 2:
            return [f"{_INDENT}; BRANCH_NZ: missing operands"]
        reg, lbl = ops[0], ops[1]
        if not isinstance(reg, IrRegister) or not isinstance(lbl, IrLabel):
            return [f"{_INDENT}; BRANCH_NZ: unexpected operand types"]
        rn = _preg(reg.index)
        return [*_load_a(rn), f"{_INDENT}CPI  0", f"{_INDENT}JFZ  {lbl.name}"]

    # ------------------------------------------------------------------
    # JUMP lbl  →  JMP lbl
    # ------------------------------------------------------------------
    #
    # JMP is the 8008 unconditional branch — a 3-byte instruction with a
    # 14-bit address.  Unlike the 4004's page-relative JCN, JMP can target
    # any address in the full 16 KB ROM without alignment constraints.

    def _emit_jump(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit JUMP: JMP label."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{_INDENT}JMP  {ops[0].name}"]
        return [f"{_INDENT}; JUMP: missing label operand"]

    # ------------------------------------------------------------------
    # CALL lbl  →  CAL lbl
    # ------------------------------------------------------------------
    #
    # CAL (Call subroutine): push PC+3 onto the 8-level hardware stack and
    # jump to the subroutine address.  The callee returns via RFC.
    # CAL is a 3-byte instruction.

    def _emit_call(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit CALL: CAL label."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{_INDENT}CAL  {ops[0].name}"]
        return [f"{_INDENT}; CALL: missing label operand"]

    # ------------------------------------------------------------------
    # RET  →  MVI A, 0  /  ADD C  /  RFC
    # ------------------------------------------------------------------
    #
    # The Oct calling convention places the return value in v1=C.
    # Before returning, we copy C to A (the 8008 return-value register).
    # RFC (Return if Carry False) is the standard unconditional return.
    #
    # ⚠️  IMPORTANT: We CANNOT emit ``MOV A, C`` here!
    # The C register has code 001.  In Group 01 (MOV), the SSS=001 field
    # is ALWAYS decoded as the IN instruction (read input port), not as
    # a register source:
    #
    #     MOV A, C = 0x79 = 01_111_001
    #     group=01, ddd=A(7), sss=001 → simulator decodes as IN 7!
    #
    # So ``MOV A, C`` reads input port 7 into A, not register C.
    # This silently corrupts the return value and is very hard to debug.
    #
    # Fix: use the ALU path to copy C → A without touching MOV:
    #   MVI A, 0   ; prime accumulator (A = 0)
    #   ADD C      ; A = 0 + C = C  (Group 10, sss=C=001 is safe here)
    #   RFC        ; return (CY=0 because 0+C never overflows for C≤255)
    #
    # In Group 10, SSS=001 means "register C" (not IN) because the group=10
    # ALU handler always reads a register, never triggers IN.  So ADD C
    # correctly reads the C register and produces A = C with CY=0.
    #
    # For void functions, A will hold garbage (C = scratch); callers must
    # ignore it.  This is safe because callers only read A for non-void
    # return types.

    def _emit_ret(self) -> list[str]:
        """Emit RET: MVI A, 0; ADD C; RFC.

        Uses the ALU path (MVI+ADD) instead of ``MOV A, C`` to avoid the
        ``C register SSS=001 → IN 7`` decoding trap in the Group 01 MOV handler.
        See class docstring for the full explanation.
        """
        return [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  {_REG_RESULT}",
            f"{_INDENT}RFC",
        ]

    # ------------------------------------------------------------------
    # HALT  →  HLT
    # ------------------------------------------------------------------
    #
    # HLT (0xFF) halts the Intel 8008 processor.  The program counter stops
    # advancing and the CPU waits for a reset signal.  This is emitted at
    # the end of _start after the main function returns.

    def _emit_halt(self) -> list[str]:
        """Emit HALT: HLT."""
        return [f"{_INDENT}HLT"]

    # ------------------------------------------------------------------
    # NOP  →  (comment — 8008 has no NOP)
    # ------------------------------------------------------------------
    #
    # The Intel 8008 has no dedicated NOP instruction.  The closest
    # equivalent is ``MOV A, A`` (self-copy, 1 byte, no visible side effect
    # except flag updates).  We emit a comment instead of polluting the
    # binary with unnecessary instructions.

    def _emit_nop(self) -> list[str]:
        """Emit NOP: comment (8008 has no true NOP)."""
        return [f"{_INDENT}; NOP (no-op; omitted on 8008)"]

    # ------------------------------------------------------------------
    # COMMENT  →  ; text
    # ------------------------------------------------------------------

    def _emit_comment(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit a comment line."""
        if ops and isinstance(ops[0], IrLabel):
            return [f"{_INDENT}; {ops[0].name}"]
        if ops and isinstance(ops[0], IrImmediate):
            return [f"{_INDENT}; {ops[0].value}"]
        return [f"{_INDENT};"]

    # ------------------------------------------------------------------
    # SYSCALL  →  inline 8008 hardware intrinsic
    # ------------------------------------------------------------------
    #
    # The Oct IR uses SYSCALL to represent the 10 hardware intrinsics.
    # Each SYSCALL number maps to a unique inline instruction sequence.
    # The mapping (from the Oct specification, OCT00):
    #
    #   3  = adc(a, b)  → ADC (add with carry):
    #          a is in v2=D, b is in v3=E, result lands in v1=C
    #   4  = sbb(a, b)  → SBB (subtract with borrow):
    #          same register layout as adc
    #   11 = rlc(a)     → RLC rotate left circular;   a in v2=D
    #   12 = rrc(a)     → RRC rotate right circular;  a in v2=D
    #   13 = ral(a)     → RAL rotate left through carry;  a in v2=D
    #   14 = rar(a)     → RAR rotate right through carry; a in v2=D
    #   15 = carry()    → ACI 0 trick to materialise CY into A
    #   16 = parity(a)  → ORA A + conditional branch;  a in v2=D
    #   20+p = in(p)    → IN p instruction (p in 0–7)
    #   40+p = out(p,v) → OUT p instruction (v in v2=D)
    #
    # All results land in v1=C (the scratch/return register).

    def _emit_syscall(self, ops: list) -> list[str]:  # noqa: ANN001
        """Emit SYSCALL: expand to the appropriate 8008 inline sequence."""
        if not ops or not isinstance(ops[0], IrImmediate):
            return [f"{_INDENT}; SYSCALL: missing number operand"]
        num = ops[0].value

        if num == 3:
            return self._emit_syscall_adc()
        if num == 4:
            return self._emit_syscall_sbb()
        if num == 11:
            return self._emit_syscall_rotate("RLC")
        if num == 12:
            return self._emit_syscall_rotate("RRC")
        if num == 13:
            return self._emit_syscall_rotate("RAL")
        if num == 14:
            return self._emit_syscall_rotate("RAR")
        if num == 15:
            return self._emit_syscall_carry()
        if num == 16:
            return self._emit_syscall_parity()
        if 20 <= num <= 27:
            return self._emit_syscall_in(num - 20)
        if 40 <= num <= 63:
            return self._emit_syscall_out(num - 40)

        # Unrecognised syscall — the validator should have caught this.
        return [f"{_INDENT}; SYSCALL {num}: unrecognised (validator missed it?)"]

    def _emit_syscall_adc(self) -> list[str]:
        """SYSCALL 3 — adc(a, b): A ← D + E + CY → result in C.

        The 8008 ADC (Add with Carry) instruction: A ← A + Rb + CY.
        a is staged in v2=D, b in v3=E by the IR compiler.

        Sequence:
          MOV  A, D      ; load a into accumulator
          ADC  E         ; A = a + b + CY (add with existing carry)
          MOV  C, A      ; store result in v1=C
        """
        return [
            f"{_INDENT}MOV  A, {_REG_ARG0}",
            f"{_INDENT}ADC  {_REG_ARG1}",
            f"{_INDENT}MOV  {_REG_RESULT}, A",
        ]

    def _emit_syscall_sbb(self) -> list[str]:
        """SYSCALL 4 — sbb(a, b): A ← D − E − CY → result in C.

        The 8008 SBB (Subtract with Borrow) instruction: A ← A − Rb − CY.
        Used for multi-byte subtraction where the borrow from the low byte
        propagates into the high byte.

        Sequence:
          MOV  A, D      ; load a
          SBB  E         ; A = a - b - CY
          MOV  C, A      ; result in v1=C
        """
        return [
            f"{_INDENT}MOV  A, {_REG_ARG0}",
            f"{_INDENT}SBB  {_REG_ARG1}",
            f"{_INDENT}MOV  {_REG_RESULT}, A",
        ]

    def _emit_syscall_rotate(self, mnemonic: str) -> list[str]:
        """SYSCALL 11–14 — rlc/rrc/ral/rar: rotate D → result in C.

        All four rotations follow the same pattern:
          1. Load the argument (v2=D) into A.
          2. Apply the rotation to A.
          3. Store A into the result register (v1=C).

        The rotation updates CY with the bit that was rotated out, so a
        subsequent ``carry()`` call reads the rotated bit.

        Args:
            mnemonic: One of ``"RLC"``, ``"RRC"``, ``"RAL"``, ``"RAR"``.
        """
        return [
            f"{_INDENT}MOV  A, {_REG_ARG0}",
            f"{_INDENT}{mnemonic}",
            f"{_INDENT}MOV  {_REG_RESULT}, A",
        ]

    def _emit_syscall_carry(self) -> list[str]:
        """SYSCALL 15 — carry(): materialise CY into C.

        The 8008 has no "read carry to register" instruction.  The classic
        trick uses ACI 0 (Add with Carry Immediate 0):

          A ← A + 0 + CY = CY   (if we prime A = 0 first)

        So: MVI A, 0; ACI 0 gives A = CY (either 0 or 1).

        Note: ACI 0 itself sets CY=0 (no overflow of 0+0+CY for CY∈{0,1}
        when A was 0), but that's fine — carry() is only valid immediately
        after the arithmetic that set CY, before any other ALU op.

        Sequence:
          MVI  A, 0      ; prime accumulator
          ACI  0         ; A = 0 + 0 + CY = CY
          MOV  C, A      ; result (0 or 1) in v1=C
        """
        return [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ACI  0",
            f"{_INDENT}MOV  {_REG_RESULT}, A",
        ]

    def _emit_syscall_parity(self) -> list[str]:
        """SYSCALL 16 — parity(a): materialise parity flag from D into C.

        The P flag is set by any ALU result:  P=1 iff popcount(A) is even.
        The trick to refresh all flags from A without changing A is ORA A
        (OR A with itself — result unchanged, flags updated).

        Then we branch on the parity flag:
          P=1 (even parity) → result = 1
          P=0 (odd parity)  → result = 0

        JFP (Jump if Parity False) = jump if P=0 (odd parity).

        Sequence:
          MOV  A, D          ; load argument into A
          ORA  A             ; refresh flags from A; P=1 iff popcount even
          MVI  C, 0          ; assume odd parity
          JFP  par_N_done    ; if P=0 (odd) → keep 0, done
          MVI  C, 1          ; even parity: set 1
        par_N_done:
        """
        done = self._next_label()
        return [
            f"{_INDENT}MOV  A, {_REG_ARG0}",
            f"{_INDENT}ORA  A",
            f"{_INDENT}MVI  {_REG_RESULT}, 0",
            f"{_INDENT}JFP  {done}",
            f"{_INDENT}MVI  {_REG_RESULT}, 1",
            f"{done}:",
        ]

    def _emit_syscall_in(self, port: int) -> list[str]:
        """SYSCALL 20+p — in(p): read input port p → result in C.

        The 8008 IN p instruction reads the 8-bit value on input port p
        (p ∈ 0–7) into the accumulator.  The port number is encoded in
        the opcode itself — no address bus cycle needed.

        We then copy A to v1=C so the result is in the standard return
        register for Oct intrinsics.

        Sequence:
          IN   p      ; A ← input port p
          MOV  C, A   ; result in v1=C
        """
        return [
            f"{_INDENT}IN   {port}",
            f"{_INDENT}MOV  {_REG_RESULT}, A",
        ]

    def _emit_syscall_out(self, port: int) -> list[str]:
        """SYSCALL 40+p — out(p, val): write D to output port p.

        The 8008 OUT p instruction writes the accumulator to output port p
        (p ∈ 0–23).  The port number is encoded in the opcode.

        The value to write (val) is staged in v2=D by the IR compiler.
        We copy D to A and then output.

        Sequence:
          MOV  A, D   ; load val from v2=D
          OUT  p      ; output port p ← A
        """
        return [
            f"{_INDENT}MOV  A, {_REG_ARG0}",
            f"{_INDENT}OUT  {port}",
        ]
