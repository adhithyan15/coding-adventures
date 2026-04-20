"""GE-225 two-pass code generator.

Overview
--------

This module translates a target-independent ``IrProgram`` into a flat binary
image of GE-225 20-bit machine words. The GE-225 is the 1960-era General
Electric mainframe that ran Dartmouth's BASIC time-sharing system in 1964.

Architecture summary
--------------------

The GE-225 is a *word-addressed accumulator machine*:

- Memory: 20-bit words, addressed from 0.
- Accumulator (A): the sole arithmetic register. All computation routes through it.
- Q register: the lower half of the 40-bit double-word used by multiply/divide.
- N register: 6-bit typewriter code latch (loaded via shift, printed via TYP).

Every IR virtual register maps to a *spill slot* — a dedicated memory word in
the data segment. Operations follow a strict load-compute-store rhythm:

  LDA [vA]      ; A = spill[vA]
  ADD [vB]      ; A = A + spill[vB]
  STA [vDst]    ; spill[vDst] = A

Instruction encoding
--------------------

Memory-reference instructions pack into one 20-bit word::

  [19:15] opcode   (5 bits)
  [14:13] modifier (2 bits — X register group; always 0 in this backend)
  [12:0]  address  (13 bits — direct word address)

Fixed-word instructions use pre-defined 20-bit constants (documented octal
values from the GE-225 manual, retrieved via ``assemble_fixed``).

Conditional branches (BZE/BNZ/BMI/BPL) are *skip-next-if-true*: when the
condition holds they advance PC by 2 (skipping the next word); otherwise by 1.
Far conditional jumps therefore require a two-word pair::

  BZE               ; skip next if A==0
  BRU target        ; jump (executed only when A==0)

Halt convention
---------------

The GE-225 has no HALT instruction. We use a self-loop stub::

  code_end: BRU code_end   ; spin forever

The ``HALT`` IR opcode emits ``BRU code_end``. The integration layer detects
halt by checking ``trace.address == halt_address`` after each ``step()``.

Memory layout
-------------

::

  ┌──────────────────────────────────────┐
  │ addr 0           : TON (prologue)    │
  │ addr 1 … code_end-1 : IR code        │
  │ addr code_end    : BRU code_end      │  ← halt stub
  │ addr data_base …: spill slots (v0…)  │
  │ addr …           : constants table   │
  └──────────────────────────────────────┘

- ``data_base = code_end + 1``
- ``spill_addr(N) = data_base + N``
- ``const_addr(K) = data_base + n_regs + K``
"""

from __future__ import annotations

from dataclasses import dataclass

from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ge225_simulator import assemble_fixed, assemble_shift, encode_instruction, pack_words

# ---------------------------------------------------------------------------
# GE-225 base memory-reference opcodes (5-bit field, octal)
# ---------------------------------------------------------------------------

_OP_LDA = 0o00  # A = mem[ea]
_OP_ADD = 0o01  # A = A + mem[ea]
_OP_SUB = 0o02  # A = A - mem[ea]
_OP_STA = 0o03  # mem[ea] = A
_OP_MPY = 0o15  # A,Q = Q × mem[ea] + A  (40-bit accumulate multiply)
_OP_DVD = 0o16  # A = (A,Q) ÷ mem[ea]  Q = remainder
_OP_BRU = 0o26  # PC = ea  (unconditional branch)

# ---------------------------------------------------------------------------
# GE-225 fixed-word instruction constants
# ---------------------------------------------------------------------------
# These are the documented 20-bit words from the GE-225 programming manual.
# ``assemble_fixed(mnemonic)`` looks them up by name and returns the word.

_W_TON = assemble_fixed("TON")   # turn typewriter on; required before any TYP
_W_TYP = assemble_fixed("TYP")   # print character code held in N register
_W_NOP = assemble_fixed("NOP")   # no operation
_W_LDZ = assemble_fixed("LDZ")   # A = 0
_W_LDO = assemble_fixed("LDO")   # A = 1
_W_LAQ = assemble_fixed("LAQ")   # A = Q  (copy lower product word after MPY)
_W_LQA = assemble_fixed("LQA")   # Q = A  (seed Q before MPY or DVD)
_W_ADO = assemble_fixed("ADO")   # A = A + 1  (add one, no memory access)
_W_SBO = assemble_fixed("SBO")   # A = A - 1  (subtract one)
_W_BMI = assemble_fixed("BMI")   # skip next if A < 0 (sign bit set)
_W_BPL = assemble_fixed("BPL")   # skip next if A >= 0 (sign bit clear)
_W_BZE = assemble_fixed("BZE")   # skip next if A == 0
_W_BNZ = assemble_fixed("BNZ")   # skip next if A != 0
_W_BOD = assemble_fixed("BOD")   # skip next if A is odd (bit 0 == 1)
_W_SAN6 = assemble_shift("SAN", 6)  # shift A[5:0] into N; used before TYP


# ---------------------------------------------------------------------------
# Public types
# ---------------------------------------------------------------------------


@dataclass
class CompileResult:
    """Output of ``compile_to_ge225``.

    Attributes:
        binary:       Packed GE-225 binary image (3 bytes per word, big-endian).
        halt_address: Word address of the halt stub (``BRU halt_address``).
        data_base:    First data-segment word address (= ``code_end + 1``).
        label_map:    Maps IR label names to their resolved code word addresses.
    """

    binary: bytes
    halt_address: int
    data_base: int
    label_map: dict[str, int]


class CodeGenError(Exception):
    """Raised when the IR program cannot be translated to GE-225 code.

    Causes include unsupported IR opcodes, ``AND_IMM`` with a non-1 immediate,
    a constant that does not fit in a GE-225 20-bit signed word, or a branch
    to an undefined label.
    """


# ---------------------------------------------------------------------------
# GE-225 word-size constraints
# ---------------------------------------------------------------------------
#
# The GE-225 uses 20-bit two's-complement words.
# Signed range: -524 288 (−2^19) to 524 287 (2^19 − 1).

_GE225_WORD_MIN: int = -(1 << 19)   # -524 288
_GE225_WORD_MAX: int =  (1 << 19) - 1  # 524 287

# The V1 GE-225 backend supports exactly this set of IR opcodes.
# Any opcode absent from this set is rejected by validate_for_ge225().
_GE225_SUPPORTED_OPCODES: frozenset[IrOp] = frozenset({
    IrOp.LABEL,
    IrOp.COMMENT,
    IrOp.NOP,
    IrOp.HALT,
    IrOp.LOAD_IMM,
    IrOp.ADD_IMM,
    IrOp.ADD,
    IrOp.SUB,
    IrOp.AND_IMM,
    IrOp.MUL,
    IrOp.DIV,
    IrOp.CMP_EQ,
    IrOp.CMP_NE,
    IrOp.CMP_LT,
    IrOp.CMP_GT,
    IrOp.JUMP,
    IrOp.BRANCH_Z,
    IrOp.BRANCH_NZ,
    IrOp.SYSCALL,
})


# ---------------------------------------------------------------------------
# Pre-flight validator
# ---------------------------------------------------------------------------


def validate_for_ge225(program: IrProgram) -> list[str]:
    """Inspect ``program`` for GE-225 backend incompatibilities without
    generating any code.

    Checks performed:

    1. **Opcode support** — every opcode must be in ``_GE225_SUPPORTED_OPCODES``.
       Opcodes absent from the V1 GE-225 backend (e.g. ``LOAD_BYTE``,
       ``STORE_WORD``, ``CALL``) are rejected immediately so the caller gets a
       precise diagnostic rather than a mid-compilation crash.

    2. **Constant range** — every ``IrImmediate`` value in a ``LOAD_IMM`` or
       ``ADD_IMM`` instruction must fit in a GE-225 20-bit signed word
       (−524 288 to 524 287).  The GE-225 backend stores constants in a
       data-segment constants table; a value that overflows 20 bits would be
       silently truncated by the ``value & 0xFFFFF`` mask, producing corrupt
       digit extraction or wrong arithmetic — exactly the bug that bit PR #941.

    3. **SYSCALL number** — only ``SYSCALL 1`` (print character) is wired up
       in the V1 GE-225 backend.  Any other syscall number is rejected.

    4. **AND_IMM immediate** — only ``imm == 1`` is supported; the GE-225
       backend uses it for the parity/odd-bit test and has no general bitwise-
       AND instruction.

    Args:
        program: The ``IrProgram`` to inspect.

    Returns:
        A list of human-readable error strings.  An empty list means the
        program is compatible with the GE-225 V1 backend.

    Example::

        from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
        prog = IrProgram(entry_label="_start")
        prog.append(IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1_000_000_000)]))
        errors = validate_for_ge225(prog)
        # errors == ["LOAD_IMM: constant 1,000,000,000 does not fit in a GE-225
        #             20-bit signed word (valid range -524,288 to 524,287)"]
    """
    errors: list[str] = []

    for instr in program.instructions:
        op = instr.opcode

        # ── Rule 1: opcode must be in the supported set ─────────────────────
        if op not in _GE225_SUPPORTED_OPCODES:
            errors.append(
                f"unsupported opcode {op.name} in V1 GE-225 backend"
            )
            continue  # no point checking operands of an unsupported opcode

        # ── Rule 2: constant range on LOAD_IMM and ADD_IMM ──────────────────
        if op in (IrOp.LOAD_IMM, IrOp.ADD_IMM):
            for operand in instr.operands:
                if isinstance(operand, IrImmediate):
                    v = operand.value
                    if not (_GE225_WORD_MIN <= v <= _GE225_WORD_MAX):
                        errors.append(
                            f"{op.name}: constant {v:,} overflows GE-225 20-bit "
                            f"signed word (valid range "
                            f"{_GE225_WORD_MIN:,} to {_GE225_WORD_MAX:,})"
                        )

        # ── Rule 3: SYSCALL number ───────────────────────────────────────────
        elif op == IrOp.SYSCALL:
            for operand in instr.operands:
                if isinstance(operand, IrImmediate) and operand.value != 1:
                    errors.append(
                        f"unsupported SYSCALL {operand.value}: "
                        f"only SYSCALL 1 (print char) is wired in the V1 GE-225 backend"
                    )
                    break

        # ── Rule 4: AND_IMM must use immediate 1 ────────────────────────────
        elif op == IrOp.AND_IMM:
            for operand in instr.operands:
                if isinstance(operand, IrImmediate) and operand.value != 1:
                    errors.append(
                        f"unsupported AND_IMM immediate {operand.value}: "
                        f"only AND_IMM 1 is supported (odd-bit test)"
                    )
                    break

    return errors


# ---------------------------------------------------------------------------
# Public function
# ---------------------------------------------------------------------------


def compile_to_ge225(program: IrProgram) -> CompileResult:
    """Compile an ``IrProgram`` to a GE-225 binary image.

    This is a three-pass process:

    - **Pre-flight**: ``validate_for_ge225`` inspects every instruction for
      opcode support, constant range, and syscall compatibility.  If any
      violation is found a ``CodeGenError`` is raised before any code is
      generated, giving the caller a precise diagnostic.
    - Pass 0: scan all instructions to collect virtual register indices and
      build the constants table (unique integer values for ``LOAD_IMM`` and
      ``ADD_IMM`` with non-trivial immediates).
    - Pass 1: walk the instruction list to compute each instruction's code-word
      address; record label addresses in the label map.
    - Pass 2: emit GE-225 machine words for each instruction, using the fully
      resolved addresses from pass 1.

    Args:
        program: An ``IrProgram`` to translate.

    Returns:
        A ``CompileResult`` containing the binary, halt address, data base, and
        the resolved label map.

    Raises:
        CodeGenError: if the program fails pre-flight validation (unsupported
            opcode, constant out of 20-bit range, unsupported syscall number,
            unsupported AND_IMM immediate), or if a branch target label is
            undefined during code generation.
    """
    errors = validate_for_ge225(program)
    if errors:
        joined = "; ".join(errors)
        raise CodeGenError(
            f"IR program failed GE-225 pre-flight validation "
            f"({len(errors)} error{'s' if len(errors) != 1 else ''}): {joined}"
        )
    return _CodeGen(program).compile()


# ---------------------------------------------------------------------------
# Internal two-pass assembler
# ---------------------------------------------------------------------------


class _CodeGen:
    """Three-pass GE-225 assembler (internal implementation).

    The three passes are:

    Pass 0 — ``_pass0()``
        Scan every operand to find the maximum virtual register index (which
        determines how many spill slots are needed) and to assign a consecutive
        index to each unique constant value referenced by ``LOAD_IMM`` and non-
        trivial ``ADD_IMM`` instructions.

    Pass 1 — ``_pass1()``
        Walk the instruction list, computing the GE-225 word count for each IR
        instruction and accumulating a ``word_addr`` counter. Labels record their
        address in ``_label_map`` at zero cost. After pass 1, ``_code_end`` and
        ``_data_base`` are known.

    Pass 2 — ``_pass2()``
        Walk the instruction list again, emitting the exact GE-225 words for
        each instruction using the addresses computed in pass 1.
    """

    def __init__(self, program: IrProgram) -> None:
        self._program = program
        self._max_reg: int = 0
        self._const_map: dict[int, int] = {}  # value → const_index (insertion order)
        self._label_map: dict[str, int] = {}
        self._code_end: int = 0
        self._data_base: int = 0

    # ------------------------------------------------------------------
    # Top-level
    # ------------------------------------------------------------------

    def compile(self) -> CompileResult:
        """Run all three passes and return the compiled result."""
        self._pass0()
        self._pass1()
        words = self._pass2()
        return CompileResult(
            binary=pack_words(words),
            halt_address=self._code_end,
            data_base=self._data_base,
            label_map=self._label_map,
        )

    # ------------------------------------------------------------------
    # Pass 0: collect registers and constants
    # ------------------------------------------------------------------

    def _pass0(self) -> None:
        """Scan all instructions to find the maximum register index and build
        the constants table.

        After this pass:
        - ``_max_reg``: highest virtual register index seen anywhere.
        - ``_const_map``: maps each unique LOAD_IMM / non-trivial ADD_IMM
          constant value to its sequential table index.
        """
        for instr in self._program.instructions:
            for operand in instr.operands:
                if isinstance(operand, IrRegister):
                    self._max_reg = max(self._max_reg, operand.index)
            self._record_constants(instr)

    def _record_constants(self, instr: IrInstruction) -> None:
        """Register any immediate values that need a slot in the constants table.

        ``LOAD_IMM`` always puts its immediate in the table (constants are loaded
        via ``LDA const_addr``).  ``ADD_IMM`` only needs the table for immediates
        other than 0, +1, and −1 (those three use the copy, ADO, or SBO paths).
        """
        if instr.opcode == IrOp.LOAD_IMM:
            # Operands: [IrRegister(dst), IrImmediate(value)]
            for operand in instr.operands:
                if isinstance(operand, IrImmediate):
                    self._intern_const(operand.value)
        elif instr.opcode == IrOp.ADD_IMM:
            # Operands: [IrRegister(dst), IrRegister(src), IrImmediate(imm)]
            for operand in instr.operands:
                if isinstance(operand, IrImmediate) and operand.value not in (0, 1, -1):
                    self._intern_const(operand.value)

    def _intern_const(self, value: int) -> None:
        """Add ``value`` to the constants table if not already present."""
        if value not in self._const_map:
            self._const_map[value] = len(self._const_map)

    # ------------------------------------------------------------------
    # Derived layout helpers (depend on pass 0 results + pass 1 code_end)
    # ------------------------------------------------------------------

    def _n_regs(self) -> int:
        """Number of spill slots = max_reg + 1 (covers v0 through v_max_reg)."""
        return self._max_reg + 1

    def _spill(self, reg_index: int) -> int:
        """Absolute word address of the spill slot for virtual register vN."""
        return self._data_base + reg_index

    def _const_addr(self, value: int) -> int:
        """Absolute word address of ``value`` in the constants table."""
        idx = self._const_map[value]
        return self._data_base + self._n_regs() + idx

    # ------------------------------------------------------------------
    # Instruction encoding helpers
    # ------------------------------------------------------------------

    def _lda(self, addr: int) -> int:
        return encode_instruction(_OP_LDA, 0, addr)

    def _add(self, addr: int) -> int:
        return encode_instruction(_OP_ADD, 0, addr)

    def _sub(self, addr: int) -> int:
        return encode_instruction(_OP_SUB, 0, addr)

    def _sta(self, addr: int) -> int:
        return encode_instruction(_OP_STA, 0, addr)

    def _mpy(self, addr: int) -> int:
        return encode_instruction(_OP_MPY, 0, addr)

    def _dvd(self, addr: int) -> int:
        return encode_instruction(_OP_DVD, 0, addr)

    def _bru(self, addr: int) -> int:
        return encode_instruction(_OP_BRU, 0, addr)

    # ------------------------------------------------------------------
    # Pass 1: label assignment and code-size calculation
    # ------------------------------------------------------------------

    def _word_count(self, instr: IrInstruction) -> int:
        """Return the number of GE-225 words this IR instruction occupies.

        Used in pass 1 to assign consecutive addresses to all labels and
        instructions before any words are actually emitted.

        Raises:
            CodeGenError: for unsupported IR opcodes in V1.
        """
        op = instr.opcode

        if op in (IrOp.LABEL, IrOp.COMMENT):
            return 0
        if op in (IrOp.NOP, IrOp.HALT, IrOp.JUMP):
            return 1
        if op == IrOp.LOAD_IMM:
            # LDA const_addr; STA spill(dst)
            return 2
        if op == IrOp.ADD_IMM:
            imm = self._get_imm(instr)
            if imm == 0:
                # Copy: LDA spill(src); STA spill(dst)
                return 2
            # +1: ADO, -1: SBO, other: ADD const — all 3 words
            return 3
        if op in (IrOp.ADD, IrOp.SUB):
            # LDA; ADD/SUB; STA
            return 3
        if op == IrOp.AND_IMM:
            # BOD-branch pattern for bit-0 extraction (see _emit_and_imm)
            return 7
        if op == IrOp.MUL:
            # LDA; LQA; LDZ; MPY; LAQ; STA
            return 6
        if op == IrOp.DIV:
            # LDA; LQA; LDZ; DVD; STA
            return 5
        if op in (IrOp.CMP_EQ, IrOp.CMP_NE, IrOp.CMP_LT, IrOp.CMP_GT):
            # LDA; SUB; skip; BRU; result0; BRU; result1; STA
            return 8
        if op in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
            # LDA; BZE/BNZ; BRU target
            return 3
        if op == IrOp.SYSCALL:
            self._check_syscall_num(instr)
            # LDA spill(v0); SAN 6; TYP
            return 3
        raise CodeGenError(f"unsupported IR opcode in V1 GE-225 backend: {op!r}")

    def _pass1(self) -> None:
        """Assign code addresses to all labels.

        The prologue (TON) occupies word 0. All IR instructions follow from
        word 1 onwards. After the loop, ``_code_end`` is the address immediately
        past all code words; the halt stub lives there, and the data segment
        starts at ``_code_end + 1``.
        """
        word_addr = 1  # word 0 = TON prologue
        for instr in self._program.instructions:
            if instr.opcode == IrOp.LABEL:
                name = str(instr.operands[0])
                self._label_map[name] = word_addr
            word_addr += self._word_count(instr)

        self._code_end = word_addr
        self._data_base = self._code_end + 1  # halt stub is one word at code_end

    # ------------------------------------------------------------------
    # Pass 2: word emission
    # ------------------------------------------------------------------

    def _pass2(self) -> list[int]:
        """Emit the complete GE-225 word list.

        Layout of the returned list:
          [TON, <IR code words>, <halt stub>, <spill slots>, <constants>]
        """
        words: list[int] = [_W_TON]  # prologue at address 0
        emit_addr = 1  # address of the next word to be appended

        for instr in self._program.instructions:
            new_words = self._emit(instr, emit_addr)
            words.extend(new_words)
            emit_addr += len(new_words)

        # Halt stub: self-referencing branch at code_end
        words.append(self._bru(self._code_end))

        # Data section: n_regs zero-initialised spill slots
        words.extend([0] * self._n_regs())

        # Constants table (in insertion order from pass 0).
        # Values are guaranteed to be in the 20-bit signed range by
        # validate_for_ge225(), so the mask safely encodes negative values
        # as 20-bit two's-complement without silent data corruption.
        for value, _ in sorted(self._const_map.items(), key=lambda kv: kv[1]):
            assert _GE225_WORD_MIN <= value <= _GE225_WORD_MAX, (
                f"constant {value} slipped past pre-flight validation"
            )
            words.append(value & 0xFFFFF)  # encode as 20-bit two's-complement

        return words

    def _emit(self, instr: IrInstruction, start_addr: int) -> list[int]:
        """Emit GE-225 words for a single IR instruction.

        ``start_addr`` is the code-word address at which the first emitted word
        will reside. Inline jump targets within a multi-word sequence (e.g.
        the compare and AND_IMM patterns) are computed relative to ``start_addr``.

        Args:
            instr:      The IR instruction to translate.
            start_addr: The code address of the first emitted word.

        Returns:
            A list of 20-bit integers (GE-225 machine words).
        """
        op = instr.opcode

        if op in (IrOp.LABEL, IrOp.COMMENT):
            return []
        if op == IrOp.NOP:
            return [_W_NOP]
        if op == IrOp.HALT:
            # Jump to the halt stub (BRU code_end)
            return [self._bru(self._code_end)]
        if op == IrOp.LOAD_IMM:
            return self._emit_load_imm(instr)
        if op == IrOp.ADD_IMM:
            return self._emit_add_imm(instr)
        if op == IrOp.ADD:
            return self._emit_binop(_OP_ADD, instr)
        if op == IrOp.SUB:
            return self._emit_binop(_OP_SUB, instr)
        if op == IrOp.AND_IMM:
            return self._emit_and_imm(instr, start_addr)
        if op == IrOp.MUL:
            return self._emit_mul(instr)
        if op == IrOp.DIV:
            return self._emit_div(instr)
        if op == IrOp.CMP_EQ:
            return self._emit_cmp(instr, start_addr, eq=True, negate=False)
        if op == IrOp.CMP_NE:
            return self._emit_cmp(instr, start_addr, eq=True, negate=True)
        if op == IrOp.CMP_LT:
            return self._emit_cmp_signed(instr, start_addr, gt_mode=False)
        if op == IrOp.CMP_GT:
            return self._emit_cmp_signed(instr, start_addr, gt_mode=True)
        if op == IrOp.JUMP:
            return self._emit_jump(instr)
        if op == IrOp.BRANCH_Z:
            return self._emit_branch(instr, zero=True)
        if op == IrOp.BRANCH_NZ:
            return self._emit_branch(instr, zero=False)
        if op == IrOp.SYSCALL:
            return self._emit_syscall(instr)

        raise CodeGenError(f"unsupported IR opcode in V1 GE-225 backend: {op!r}")

    # ------------------------------------------------------------------
    # Per-opcode emitters
    # ------------------------------------------------------------------

    def _emit_load_imm(self, instr: IrInstruction) -> list[int]:
        """LOAD_IMM vDst, imm → LDA const_addr; STA spill(vDst).

        The constant was pre-stored in the data segment during pass 0.
        Two words.
        """
        dst = self._reg(instr, 0)
        imm = self._get_imm(instr)
        return [
            self._lda(self._const_addr(imm)),
            self._sta(self._spill(dst)),
        ]

    def _emit_add_imm(self, instr: IrInstruction) -> list[int]:
        """ADD_IMM vDst, vSrc, imm — three specialisations.

        imm == 0:   register copy  → LDA; STA  (2 words)
        imm == +1:  increment      → LDA; ADO; STA  (3 words)
        imm == -1:  decrement      → LDA; SBO; STA  (3 words)
        other:      add constant   → LDA; ADD const_addr; STA  (3 words)

        The ADD-constant path requires the immediate to have been interned in
        the constants table during pass 0.
        """
        dst = self._reg(instr, 0)
        src = self._reg(instr, 1)
        imm = self._get_imm(instr)

        lda = self._lda(self._spill(src))
        sta = self._sta(self._spill(dst))

        if imm == 0:
            return [lda, sta]
        if imm == 1:
            return [lda, _W_ADO, sta]
        if imm == -1:
            return [lda, _W_SBO, sta]
        return [lda, self._add(self._const_addr(imm)), sta]

    def _emit_binop(self, ge225_op: int, instr: IrInstruction) -> list[int]:
        """ADD or SUB vDst, vA, vB → LDA spill(vA); OP spill(vB); STA spill(vDst).

        Three words.
        """
        dst = self._reg(instr, 0)
        reg_a = self._reg(instr, 1)
        reg_b = self._reg(instr, 2)
        return [
            self._lda(self._spill(reg_a)),
            encode_instruction(ge225_op, 0, self._spill(reg_b)),
            self._sta(self._spill(dst)),
        ]

    def _emit_and_imm(self, instr: IrInstruction, start_addr: int) -> list[int]:
        """AND_IMM vDst, vSrc, 1 — extract parity bit using BOD.

        Only ``imm == 1`` is supported in V1.  Other values raise ``CodeGenError``.

        GE-225 branch-test semantics: ``BOD`` skips the next word when the
        condition is **TRUE** (A is odd).  Therefore when A is odd the BRU at
        +2 is skipped and execution falls to LDO at +3; when A is even the
        BRU executes and jumps to LDZ at +5.

        Seven words::

            addr+0:  LDA  spill(vSrc)
            addr+1:  BOD                         ; skip next (BRU) if A is ODD
            addr+2:  BRU  addr+5 (__zero)        ; A is EVEN → jump to LDZ
            addr+3:  LDO                          ; A is ODD (BOD skipped BRU) → A=1
            addr+4:  BRU  addr+6 (__done)
            addr+5:  LDZ                          ; A is EVEN → A=0  [__zero]
            addr+6:  STA  spill(vDst)             [__done]
        """
        dst = self._reg(instr, 0)
        src = self._reg(instr, 1)
        imm = self._get_imm(instr)
        if imm != 1:
            raise CodeGenError(
                f"AND_IMM with immediate {imm!r} is not supported in V1; only imm=1 is allowed"
            )
        zero_addr = start_addr + 5  # LDZ (result=0) for EVEN inputs
        done_addr = start_addr + 6  # STA
        return [
            self._lda(self._spill(src)),   # +0: load source value
            _W_BOD,                         # +1: skip +2 if A is ODD (TRUE)
            self._bru(zero_addr),           # +2: A is EVEN (BOD didn't skip) → jump to LDZ
            _W_LDO,                         # +3: A is ODD (BOD skipped +2) → A=1
            self._bru(done_addr),           # +4: jump to STA
            _W_LDZ,                         # +5: A is EVEN → A=0  [zero_addr]
            self._sta(self._spill(dst)),    # +6: store result  [done_addr]
        ]

    def _emit_mul(self, instr: IrInstruction) -> list[int]:
        """MUL vDst, vA, vB → MPY sequence.

        The GE-225 MPY instruction computes ``A,Q = Q × mem + A``. To multiply
        vA × vB::

            LDA  spill(vA)    ; A = vA
            LQA               ; Q = A = vA  (seed the Q register)
            LDZ               ; A = 0  (accumulator part of 40-bit state)
            MPY  spill(vB)    ; A,Q = Q*vB + A = vA*vB + 0
            LAQ               ; A = Q  (take the low 20 bits of the product)
            STA  spill(vDst)

        Six words.  Note: for products larger than 2^19 the high 20 bits spill
        into A after MPY. V1 ignores overflow.
        """
        dst = self._reg(instr, 0)
        reg_a = self._reg(instr, 1)
        reg_b = self._reg(instr, 2)
        return [
            self._lda(self._spill(reg_a)),
            _W_LQA,
            _W_LDZ,
            self._mpy(self._spill(reg_b)),
            _W_LAQ,
            self._sta(self._spill(dst)),
        ]

    def _emit_div(self, instr: IrInstruction) -> list[int]:
        """DIV vDst, vA, vB → DVD sequence.

        The GE-225 DVD instruction takes a 40-bit dividend from (A,Q) and
        divides by a memory word.  To compute vA ÷ vB (integer quotient)::

            LDA  spill(vA)    ; A = vA  (low dividend word)
            LQA               ; Q = A = vA
            LDZ               ; A = 0  (high dividend word — zero for small values)
            DVD  spill(vB)    ; A = quotient; Q = remainder
            STA  spill(vDst)

        Five words.  Division by zero propagates a ``ZeroDivisionError`` from
        the GE-225 simulator's Python implementation.
        """
        dst = self._reg(instr, 0)
        reg_a = self._reg(instr, 1)
        reg_b = self._reg(instr, 2)
        return [
            self._lda(self._spill(reg_a)),
            _W_LQA,
            _W_LDZ,
            self._dvd(self._spill(reg_b)),
            self._sta(self._spill(dst)),
        ]

    def _emit_cmp(
        self, instr: IrInstruction, start_addr: int, *, eq: bool, negate: bool
    ) -> list[int]:
        """CMP_EQ or CMP_NE — compare two registers for equality.

        The GE-225 has no compare instruction.  We subtract and test the result::

            LDA  spill(vA)
            SUB  spill(vB)         ; A = vA - vB  (zero iff equal)
            BNZ                    ; if A≠0 → skip next  (skip true branch for EQ)
            BRU  addr+6 (__true)
            LDZ                    ; not equal → result 0
            BRU  addr+7 (__done)
            LDO                    ; equal → result 1    [__true]
            STA  spill(vDst)                             [__done]

        For CMP_NE, swap the skip sense (BZE instead of BNZ) and the result labels.

        Eight words.
        """
        dst = self._reg(instr, 0)
        reg_a = self._reg(instr, 1)
        reg_b = self._reg(instr, 2)

        true_addr = start_addr + 6
        done_addr = start_addr + 7

        # BNZ skips when A≠0; BZE skips when A=0.
        # For EQ: skip to BRU-true when A≠0 → use BNZ (skip past the BRU when not equal)
        # For NE: skip to BRU-true when A=0  → use BZE (skip past the BRU when equal)
        skip_word = _W_BNZ if eq else _W_BZE

        # Result words: if negate (CMP_NE) swap 0 and 1.
        zero_word = _W_LDO if negate else _W_LDZ  # result when A==0 after SUB
        one_word  = _W_LDZ if negate else _W_LDO  # result when A!=0 after SUB

        return [
            self._lda(self._spill(reg_a)),    # +0
            self._sub(self._spill(reg_b)),     # +1
            skip_word,                          # +2: conditional skip
            self._bru(true_addr),              # +3: jump to non-zero branch
            zero_word,                          # +4: result for A==0
            self._bru(done_addr),              # +5: jump past non-zero branch
            one_word,                           # +6: result for A!=0  [__true]
            self._sta(self._spill(dst)),       # +7  [__done]
        ]

    def _emit_cmp_signed(
        self, instr: IrInstruction, start_addr: int, *, gt_mode: bool
    ) -> list[int]:
        """CMP_LT or CMP_GT — signed integer comparison.

        For CMP_LT (vA < vB): compute vA - vB; negative result means vA < vB::

            LDA  spill(vA)
            SUB  spill(vB)         ; A = vA - vB
            BPL                    ; if A>=0 → skip next (not less than)
            BRU  addr+6 (__true)
            LDZ                    ; >=0 → result 0
            BRU  addr+7 (__done)
            LDO                    ; <0  → result 1   [__true]
            STA  spill(vDst)                          [__done]

        For CMP_GT (vA > vB): swap vA and vB (vA > vB iff vB < vA).

        Eight words.
        """
        dst = self._reg(instr, 0)
        reg_a = self._reg(instr, 1)
        reg_b = self._reg(instr, 2)

        true_addr = start_addr + 6
        done_addr = start_addr + 7

        # CMP_GT: swap: compute vB - vA (negative iff vB < vA, i.e., vA > vB)
        lhs = reg_b if gt_mode else reg_a
        rhs = reg_a if gt_mode else reg_b

        # BPL skips when A >= 0 (non-negative). When the difference is negative
        # (lhs < rhs) we do NOT skip, so BRU jumps to __true → LDO.
        return [
            self._lda(self._spill(lhs)),    # +0
            self._sub(self._spill(rhs)),     # +1
            _W_BPL,                          # +2: skip next if A>=0 (not less)
            self._bru(true_addr),            # +3: jump to true (A<0)
            _W_LDZ,                          # +4: A>=0 → result 0
            self._bru(done_addr),            # +5
            _W_LDO,                          # +6: A<0 → result 1  [__true]
            self._sta(self._spill(dst)),     # +7  [__done]
        ]

    def _emit_jump(self, instr: IrInstruction) -> list[int]:
        """JUMP label → BRU label_addr. One word."""
        label = self._resolve_label(instr, 0)
        return [self._bru(label)]

    def _emit_branch(self, instr: IrInstruction, *, zero: bool) -> list[int]:
        """BRANCH_Z or BRANCH_NZ — conditional far branch.

        The GE-225 conditional branches are skip-next-if-true, not jump-if-true.
        A far conditional branch requires a two-word pair after the load::

            LDA  spill(vN)
            BNZ               ; BRANCH_Z:  skip BRU when A≠0 (fall through); execute BRU when A==0
            BRU  target       ; (executed only when A==0)
            ; fall through when A≠0

        For BRANCH_NZ, swap BNZ → BZE.  Three words.
        """
        reg_n = self._reg(instr, 0)
        target = self._resolve_label(instr, 1)
        # To jump when zero: BZE skips BRU when zero... wait, we need to jump
        # when zero, so when A==0 we should NOT skip the BRU.
        # BZE skips next IF A==0 → opposite of what we want for BRANCH_Z.
        # For BRANCH_Z (jump when A==0): use BNZ (skip when A!=0, so don't skip when A==0)
        # For BRANCH_NZ (jump when A!=0): use BZE (skip when A==0, so don't skip when A!=0)
        skip_word = _W_BNZ if zero else _W_BZE
        return [
            self._lda(self._spill(reg_n)),
            skip_word,
            self._bru(target),
        ]

    def _emit_syscall(self, instr: IrInstruction) -> list[int]:
        """SYSCALL 1 — print the character whose code is in spill_v0.

        The GE-225 typewriter subsystem requires the character code in the N
        register (6 bits).  The SAN 6 instruction loads N from the low 6 bits
        of A::

            LDA  spill(v0)    ; load 6-bit typewriter code
            SAN  6            ; shift low 6 bits of A into N register
            TYP               ; print the character whose code is in N

        Three words.
        """
        self._check_syscall_num(instr)
        return [
            self._lda(self._spill(0)),  # spill(v0) = the syscall argument register
            _W_SAN6,
            _W_TYP,
        ]

    # ------------------------------------------------------------------
    # Operand extraction helpers
    # ------------------------------------------------------------------

    def _reg(self, instr: IrInstruction, idx: int) -> int:
        """Extract the register index from the idx-th operand."""
        operand = instr.operands[idx]
        if not isinstance(operand, IrRegister):
            raise CodeGenError(
                f"expected IrRegister at operand {idx} of {instr.opcode!r}, "
                f"got {type(operand).__name__}"
            )
        return operand.index

    def _get_imm(self, instr: IrInstruction) -> int:
        """Extract the integer value from the last IrImmediate operand."""
        for operand in reversed(instr.operands):
            if isinstance(operand, IrImmediate):
                return operand.value
        raise CodeGenError(f"no IrImmediate operand in {instr.opcode!r} instruction")

    def _resolve_label(self, instr: IrInstruction, idx: int) -> int:
        """Resolve a label operand to its absolute code address.

        Raises:
            CodeGenError: if the label has no entry in the label map.
        """
        operand = instr.operands[idx]
        if not isinstance(operand, IrLabel):
            raise CodeGenError(
                f"expected IrLabel at operand {idx} of {instr.opcode!r}, "
                f"got {type(operand).__name__}"
            )
        name = operand.name
        if name not in self._label_map:
            raise CodeGenError(f"undefined label: {name!r}")
        return self._label_map[name]

    def _check_syscall_num(self, instr: IrInstruction) -> None:
        """Verify the SYSCALL number is 1.

        Only SYSCALL 1 (print char from v0) is supported in V1.
        """
        for operand in instr.operands:
            if isinstance(operand, IrImmediate):
                if operand.value != 1:
                    raise CodeGenError(
                        f"only SYSCALL 1 is supported in V1; got SYSCALL {operand.value}"
                    )
                return
        raise CodeGenError("SYSCALL instruction has no numeric operand")
