"""Tests for ir_to_intel_8008_compiler.

This test suite verifies that the CodeGenerator correctly lowers every IR
opcode into the expected Intel 8008 assembly sequence, and that the
IrToIntel8008Compiler facade correctly orchestrates validation + generation.

Test organisation:
  1. TestCodeGeneratorHeader       — ORG directive, program header
  2. TestEmitLabel                 — LABEL opcode
  3. TestEmitLoadImm               — LOAD_IMM opcode → MVI
  4. TestEmitLoadAddr              — LOAD_ADDR opcode → MVI H / MVI L
  5. TestEmitLoadByte              — LOAD_BYTE opcode → MVI A, 0; ADD M (safe group-10 path)
  6. TestEmitStoreByte             — STORE_BYTE opcode → MOV M, A
  7. TestEmitAdd                   — ADD opcode → MOV/ADD/MOV
  8. TestEmitAddImm                — ADD_IMM opcode (including imm=0 copy)
  9. TestEmitSub                   — SUB opcode
  10. TestEmitAnd                  — AND opcode → ANA
  11. TestEmitOr                   — OR opcode → ORA
  12. TestEmitXor                  — XOR opcode → XRA
  13. TestEmitNot                  — NOT opcode → XRI 0xFF
  14. TestEmitCmpEq                — CMP_EQ → label-based materialisation
  15. TestEmitCmpNe                — CMP_NE → label-based materialisation
  16. TestEmitCmpLt                — CMP_LT → CY flag test
  17. TestEmitCmpGt                — CMP_GT → operand-swap trick
  18. TestEmitBranchZ              — BRANCH_Z → JTZ
  19. TestEmitBranchNz             — BRANCH_NZ → JFZ
  20. TestEmitJump                 — JUMP → JMP
  21. TestEmitCall                 — CALL → CAL
  22. TestEmitRet                  — RET → MVI A, 0; ADD C; RFC
  23. TestEmitHalt                 — HALT → HLT
  24. TestEmitNop                  — NOP → comment
  25. TestEmitComment              — COMMENT → ; text
  26. TestEmitSyscall              — SYSCALL expansions (all 10)
  27. TestLabelCounter             — unique label generation across calls
  28. TestUnknownOpcode            — graceful fallback for unknown opcodes
  29. TestFullProgram              — end-to-end multi-instruction programs
  30. TestIrToIntel8008Compiler    — facade: validate+generate, error handling
  31. TestPublicApi                — module-level validate() and generate_asm()
"""

from __future__ import annotations

import pytest
from compiler_ir import (
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from ir_to_intel_8008_compiler import (
    CodeGenerator,
    Intel8008Backend,
    IrToIntel8008Compiler,
    IrValidationError,
    generate_asm,
    validate,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_INDENT = "    "


def _reg(i: int) -> IrRegister:
    """Shorthand: create IrRegister(index=i)."""
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    """Shorthand: create IrImmediate(value=v)."""
    return IrImmediate(value=v)


def _lbl(name: str) -> IrLabel:
    """Shorthand: create IrLabel(name=name)."""
    return IrLabel(name=name)


def _instr(op: IrOp, *operands: object) -> IrInstruction:
    """Shorthand: create IrInstruction(opcode=op, operands=[...])."""
    return IrInstruction(opcode=op, operands=list(operands))


def _prog(*instrs: IrInstruction) -> IrProgram:
    """Build a minimal IrProgram with the given instructions."""
    prog = IrProgram(entry_label="_start", version=1)
    for instr in instrs:
        prog.add_instruction(instr)
    return prog


def _gen_lines(*instrs: IrInstruction) -> list[str]:
    """Generate assembly for given instructions and return lines (skip ORG line)."""
    prog = _prog(*instrs)
    asm = CodeGenerator().generate(prog)
    # First line is the ORG directive; return the rest
    return asm.strip().splitlines()[1:]


def _gen(*instrs: IrInstruction) -> str:
    """Generate full assembly for given instructions."""
    return CodeGenerator().generate(_prog(*instrs))


# Physical register mapping
_PREG = {0: "B", 1: "C", 2: "D", 3: "E", 4: "H", 5: "L"}


# ===========================================================================
# 1. TestCodeGeneratorHeader
# ===========================================================================


class TestCodeGeneratorHeader:
    """The generated assembly always starts with ORG 0x0000."""

    def test_empty_program_starts_with_org(self) -> None:
        """An empty program emits only the ORG directive."""
        asm = CodeGenerator().generate(_prog())
        assert asm.startswith(f"{_INDENT}ORG 0x0000\n")

    def test_org_is_first_line(self) -> None:
        """ORG is the very first line even when instructions follow."""
        asm = _gen(_instr(IrOp.HALT))
        lines = asm.splitlines()
        assert lines[0] == f"{_INDENT}ORG 0x0000"

    def test_output_ends_with_newline(self) -> None:
        """Output must end with a newline (POSIX-compliant file)."""
        asm = _gen(_instr(IrOp.HALT))
        assert asm.endswith("\n")

    def test_empty_program_ends_with_newline(self) -> None:
        """Empty program ends with a newline."""
        asm = CodeGenerator().generate(_prog())
        assert asm.endswith("\n")


# ===========================================================================
# 2. TestEmitLabel
# ===========================================================================


class TestEmitLabel:
    """LABEL opcode emits a column-0 label definition."""

    def test_label_at_column_zero(self) -> None:
        """Label definitions start at column 0 (no indentation)."""
        lines = _gen_lines(_instr(IrOp.LABEL, _lbl("loop_start")))
        assert lines == ["loop_start:"]

    def test_label_name_preserved(self) -> None:
        """The label name is emitted verbatim."""
        lines = _gen_lines(_instr(IrOp.LABEL, _lbl("_fn_main")))
        assert lines == ["_fn_main:"]

    def test_label_with_underscores_and_digits(self) -> None:
        """Labels with underscores and digits are passed through unchanged."""
        lines = _gen_lines(_instr(IrOp.LABEL, _lbl("loop_0_end_42")))
        assert lines == ["loop_0_end_42:"]

    def test_label_missing_operand(self) -> None:
        """LABEL with no operand emits nothing (defensive)."""
        lines = _gen_lines(_instr(IrOp.LABEL))
        assert lines == []

    def test_label_wrong_operand_type(self) -> None:
        """LABEL with non-label operand emits nothing."""
        lines = _gen_lines(_instr(IrOp.LABEL, _imm(42)))
        assert lines == []


# ===========================================================================
# 3. TestEmitLoadImm
# ===========================================================================


class TestEmitLoadImm:
    """LOAD_IMM emits MVI Rdst, imm."""

    @pytest.mark.parametrize("vreg,preg", _PREG.items())
    def test_all_virtual_registers(self, vreg: int, preg: str) -> None:
        """Each virtual register maps to the correct physical register."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _reg(vreg), _imm(0)))
        assert lines == [f"{_INDENT}MVI  {preg}, 0"]

    def test_immediate_zero(self) -> None:
        """MVI with value 0 is emitted correctly."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _reg(2), _imm(0)))
        assert lines == [f"{_INDENT}MVI  D, 0"]

    def test_immediate_255(self) -> None:
        """MVI with value 255 (max unsigned byte)."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _reg(1), _imm(255)))
        assert lines == [f"{_INDENT}MVI  C, 255"]

    def test_immediate_42(self) -> None:
        """MVI with a typical value."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _reg(3), _imm(42)))
        assert lines == [f"{_INDENT}MVI  E, 42"]

    def test_missing_operands(self) -> None:
        """LOAD_IMM with no operands emits a comment (defensive)."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM))
        assert len(lines) == 1
        assert "LOAD_IMM" in lines[0]

    def test_wrong_operand_types(self) -> None:
        """LOAD_IMM with wrong types emits a comment (defensive)."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _lbl("x"), _imm(1)))
        assert len(lines) == 1
        assert "LOAD_IMM" in lines[0]


# ===========================================================================
# 4. TestEmitLoadAddr
# ===========================================================================


class TestEmitLoadAddr:
    """LOAD_ADDR emits MVI H, hi(sym); MVI L, lo(sym)."""

    def test_basic_label(self) -> None:
        """LOAD_ADDR expands to two MVI instructions."""
        lines = _gen_lines(_instr(IrOp.LOAD_ADDR, _reg(1), _lbl("tape")))
        assert lines == [
            f"{_INDENT}MVI  H, hi(tape)",
            f"{_INDENT}MVI  L, lo(tape)",
        ]

    def test_destination_register_ignored(self) -> None:
        """The destination virtual register is ignored (H:L is always used)."""
        lines_v1 = _gen_lines(_instr(IrOp.LOAD_ADDR, _reg(1), _lbl("buf")))
        lines_v3 = _gen_lines(_instr(IrOp.LOAD_ADDR, _reg(3), _lbl("buf")))
        assert lines_v1 == lines_v3

    def test_label_name_preserved(self) -> None:
        """The symbol name appears verbatim in hi() and lo()."""
        lines = _gen_lines(_instr(IrOp.LOAD_ADDR, _reg(0), _lbl("my_symbol")))
        assert "hi(my_symbol)" in lines[0]
        assert "lo(my_symbol)" in lines[1]

    def test_missing_operands(self) -> None:
        """LOAD_ADDR with no operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.LOAD_ADDR))
        assert len(lines) == 1
        assert "LOAD_ADDR" in lines[0]

    def test_non_label_operand(self) -> None:
        """LOAD_ADDR with a register instead of label emits a comment."""
        lines = _gen_lines(_instr(IrOp.LOAD_ADDR, _reg(1), _reg(2)))
        assert len(lines) == 1
        assert "LOAD_ADDR" in lines[0]


# ===========================================================================
# 5. TestEmitLoadByte
# ===========================================================================


class TestEmitLoadByte:
    """LOAD_BYTE emits MVI A, 0; ADD M; MOV Rdst, A (safe group-10 path).

    ``MOV A, M`` encodes as 0x7E = 01_111_110 — group=01, sss=110.  On the
    Intel 8008, group=01 with sss=110 is decoded as ``CAL`` (unconditional
    subroutine call), not a memory read.  This is catastrophic: the CPU pushes
    the PC and jumps to an arbitrary address read from the next 2 bytes.

    The safe workaround uses the group-10 ALU path via ``_load_a("M")``:
        MVI  A, 0     — prime accumulator
        ADD  M        — A = 0 + RAM[H:L]  (group-10 sss=110 = M, not CAL)
        MOV  Rdst, A  — store in destination register
    This always produces exactly 3 lines.
    """

    @pytest.mark.parametrize("vreg,preg", _PREG.items())
    def test_all_destination_registers(self, vreg: int, preg: str) -> None:
        """Each destination virtual register receives the byte via safe load."""
        lines = _gen_lines(
            _instr(IrOp.LOAD_BYTE, _reg(vreg), _reg(0), _reg(0))
        )
        assert lines[0] == f"{_INDENT}MVI  A, 0"
        assert lines[1] == f"{_INDENT}ADD  M"
        assert lines[2] == f"{_INDENT}MOV  {preg}, A"

    def test_three_line_sequence(self) -> None:
        """LOAD_BYTE always emits exactly three lines (safe load + MOV)."""
        lines = _gen_lines(_instr(IrOp.LOAD_BYTE, _reg(1), _reg(1), _reg(0)))
        assert len(lines) == 3

    def test_base_and_offset_operands_ignored(self) -> None:
        """The base and offset operands are ignored (H:L is implicit)."""
        lines_a = _gen_lines(_instr(IrOp.LOAD_BYTE, _reg(1), _reg(2), _reg(3)))
        lines_b = _gen_lines(_instr(IrOp.LOAD_BYTE, _reg(1), _reg(4), _reg(5)))
        assert lines_a == lines_b

    def test_uses_add_m_not_mov_a_m(self) -> None:
        """LOAD_BYTE must NOT emit MOV A, M (encodes as CAL on 8008)."""
        lines = _gen_lines(_instr(IrOp.LOAD_BYTE, _reg(2), _reg(0), _reg(0)))
        assert f"{_INDENT}MOV  A, M" not in lines, (
            "MOV A, M = 0x7E = CAL — must use MVI A, 0; ADD M instead"
        )

    def test_missing_operands(self) -> None:
        """LOAD_BYTE with no operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.LOAD_BYTE))
        assert len(lines) == 1
        assert "LOAD_BYTE" in lines[0]


# ===========================================================================
# 6. TestEmitStoreByte
# ===========================================================================


class TestEmitStoreByte:
    """STORE_BYTE emits load-Rsrc-safely; MOV M, A.

    Dangerous source registers (C=v1, H=v4) use ``_load_a`` which emits
    ``MVI A, 0; ADD {reg}`` (3 lines total including MOV M, A) to avoid
    hardware conflicts in Group-01:
      - ``MOV A, C`` = 0x79 → IN 7 (reads input port 7)
      - ``MOV A, H`` = 0x7C → JMP (unconditional jump — catastrophic!)

    Safe source registers (B=v0, D=v2, E=v3, L=v5) use the standard 2-line
    ``MOV A, {reg}; MOV M, A`` sequence.
    """

    @pytest.mark.parametrize("vreg,preg", {k: v for k, v in _PREG.items() if v not in ("C", "H")}.items())
    def test_all_safe_source_registers(self, vreg: int, preg: str) -> None:
        """Safe source registers (B, D, E, L) use standard MOV A, {reg}."""
        lines = _gen_lines(
            _instr(IrOp.STORE_BYTE, _reg(vreg), _reg(0), _reg(0))
        )
        assert lines == [
            f"{_INDENT}MOV  A, {preg}",
            f"{_INDENT}MOV  M, A",
        ]

    def test_c_source_uses_safe_load(self) -> None:
        """STORE_BYTE with C (v1) as source: MVI A, 0; ADD C; MOV M, A.

        MOV A, C = 0x79 = IN 7 — hardware conflict in Group-01.
        """
        lines = _gen_lines(_instr(IrOp.STORE_BYTE, _reg(1), _reg(0), _reg(0)))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}MOV  M, A",
        ]

    def test_h_source_uses_safe_load(self) -> None:
        """STORE_BYTE with H (v4) as source: MVI A, 0; ADD H; MOV M, A.

        MOV A, H = 0x7C = JMP (unconditional jump) — hardware conflict in Group-01.
        """
        lines = _gen_lines(_instr(IrOp.STORE_BYTE, _reg(4), _reg(0), _reg(0)))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  H",
            f"{_INDENT}MOV  M, A",
        ]

    def test_two_line_sequence_for_safe_reg(self) -> None:
        """STORE_BYTE emits 2 lines when source is a safe register (v2=D)."""
        lines = _gen_lines(_instr(IrOp.STORE_BYTE, _reg(2), _reg(0), _reg(0)))
        assert len(lines) == 2

    def test_missing_operands(self) -> None:
        """STORE_BYTE with no operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.STORE_BYTE))
        assert len(lines) == 1
        assert "STORE_BYTE" in lines[0]


# ===========================================================================
# 7. TestEmitAdd
# ===========================================================================


class TestEmitAdd:
    """ADD emits load-Ra-safely; ADD Rb; MOV Rdst, A.

    When Ra is the C register (v1), the standard ``MOV A, C`` instruction
    encodes as 0x79 = IN 7 on the Intel 8008 hardware.  ``_load_a("C")``
    substitutes ``MVI A, 0; ADD C`` (group-10 ALU path) which safely reads C.
    This adds one instruction but is always correct.
    """

    def test_basic_add(self) -> None:
        """ADD v2, v1, v0 — Ra=C uses safe 2-instruction load."""
        lines = _gen_lines(_instr(IrOp.ADD, _reg(2), _reg(1), _reg(0)))
        # MVI A, 0 + ADD C loads C into A (avoids IN-7 trap)
        # then ADD B = A + B = C + B; result stored in D
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}ADD  B",
            f"{_INDENT}MOV  D, A",
        ]

    def test_three_lines_when_ra_not_c(self) -> None:
        """ADD v1, v2, v3 — Ra=D (not C): standard 3-line sequence."""
        lines = _gen_lines(_instr(IrOp.ADD, _reg(1), _reg(2), _reg(3)))
        assert len(lines) == 3

    def test_add_same_registers(self) -> None:
        """ADD v1, v1, v1 (self-add) — Ra=C: uses safe 4-line load."""
        lines = _gen_lines(_instr(IrOp.ADD, _reg(1), _reg(1), _reg(1)))
        # _load_a("C") = [MVI A, 0; ADD C] → A = C
        # then ADD C → A = C + C = 2C; stored back in C
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}ADD  C",
            f"{_INDENT}MOV  C, A",
        ]

    def test_missing_operands(self) -> None:
        """ADD with fewer than 3 operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.ADD, _reg(1), _reg(2)))
        assert len(lines) == 1
        assert "ADD" in lines[0]

    def test_wrong_operand_types(self) -> None:
        """ADD with non-register operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.ADD, _reg(1), _imm(5), _reg(2)))
        assert len(lines) == 1
        assert "ADD" in lines[0]


# ===========================================================================
# 8. TestEmitAddImm
# ===========================================================================


class TestEmitAddImm:
    """ADD_IMM emits MOV A, Ra; [ADI imm;] MOV Rdst, A.

    Special case: imm == 0 is a pure register copy (no ADI).
    """

    def test_nonzero_immediate(self) -> None:
        """ADD_IMM with imm != 0 emits 3 lines including ADI."""
        lines = _gen_lines(_instr(IrOp.ADD_IMM, _reg(1), _reg(2), _imm(5)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}ADI  5",
            f"{_INDENT}MOV  C, A",
        ]

    def test_immediate_zero_is_copy(self) -> None:
        """ADD_IMM with imm==0 copies Ra to Rdst via safe _load_a sequence.

        When Ra is C (v1), _load_a emits ``MVI A, 0; ADD C`` (2 lines) then
        ``MOV D, A`` — 3 lines total instead of the usual 2.  This avoids the
        ``MOV A, C = IN 7`` hardware trap.
        """
        lines = _gen_lines(_instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(0)))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}MOV  D, A",
        ]

    def test_immediate_255(self) -> None:
        """ADD_IMM with max immediate value (255)."""
        lines = _gen_lines(_instr(IrOp.ADD_IMM, _reg(1), _reg(1), _imm(255)))
        assert f"{_INDENT}ADI  255" in lines

    def test_copy_two_lines_when_src_not_c(self) -> None:
        """Register copy (imm=0) emits 2 lines when source is not C (v2=D here)."""
        lines = _gen_lines(_instr(IrOp.ADD_IMM, _reg(3), _reg(2), _imm(0)))
        assert len(lines) == 2

    def test_nonzero_three_lines(self) -> None:
        """Non-zero ADD_IMM always emits exactly 3 lines."""
        lines = _gen_lines(_instr(IrOp.ADD_IMM, _reg(1), _reg(2), _imm(1)))
        assert len(lines) == 3

    def test_missing_operands(self) -> None:
        """ADD_IMM with fewer than 3 operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.ADD_IMM, _reg(1)))
        assert len(lines) == 1
        assert "ADD_IMM" in lines[0]


# ===========================================================================
# 9. TestEmitSub
# ===========================================================================


class TestEmitSub:
    """SUB emits MOV A, Ra; SUB Rb; MOV Rdst, A."""

    def test_basic_sub(self) -> None:
        """SUB v1, v2, v3 → MOV A, D; SUB E; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SUB, _reg(1), _reg(2), _reg(3)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}SUB  E",
            f"{_INDENT}MOV  C, A",
        ]

    def test_three_lines_when_ra_not_c(self) -> None:
        """SUB emits 3 lines when Ra is not C (v2=D here)."""
        lines = _gen_lines(_instr(IrOp.SUB, _reg(0), _reg(2), _reg(3)))
        assert len(lines) == 3

    def test_missing_operands(self) -> None:
        """SUB with fewer than 3 operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.SUB))
        assert len(lines) == 1
        assert "SUB" in lines[0]


# ===========================================================================
# 10. TestEmitAnd
# ===========================================================================


class TestEmitAnd:
    """AND emits load-Ra-safely; ANA Rb; MOV Rdst, A."""

    def test_basic_and(self) -> None:
        """AND v3, v1, v2 — Ra=C uses safe 2-instruction load (4 lines total)."""
        lines = _gen_lines(_instr(IrOp.AND, _reg(3), _reg(1), _reg(2)))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}ANA  D",
            f"{_INDENT}MOV  E, A",
        ]

    def test_mnemonic_is_ana(self) -> None:
        """The 8008 AND instruction is ANA (not AND)."""
        lines = _gen_lines(_instr(IrOp.AND, _reg(1), _reg(2), _reg(3)))
        assert any("ANA" in line for line in lines)

    def test_missing_operands(self) -> None:
        """AND with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.AND))
        assert len(lines) == 1
        assert "AND" in lines[0]


# ===========================================================================
# 11. TestEmitOr
# ===========================================================================


class TestEmitOr:
    """OR emits MOV A, Ra; ORA Rb; MOV Rdst, A."""

    def test_basic_or(self) -> None:
        """OR v2, v0, v1 → MOV A, B; ORA C; MOV D, A."""
        lines = _gen_lines(_instr(IrOp.OR, _reg(2), _reg(0), _reg(1)))
        assert lines == [
            f"{_INDENT}MOV  A, B",
            f"{_INDENT}ORA  C",
            f"{_INDENT}MOV  D, A",
        ]

    def test_mnemonic_is_ora(self) -> None:
        """The 8008 OR instruction is ORA (not OR)."""
        lines = _gen_lines(_instr(IrOp.OR, _reg(1), _reg(2), _reg(3)))
        assert any("ORA" in line for line in lines)

    def test_missing_operands(self) -> None:
        """OR with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.OR, _reg(1)))
        assert len(lines) == 1
        assert "OR" in lines[0]


# ===========================================================================
# 12. TestEmitXor
# ===========================================================================


class TestEmitXor:
    """XOR emits MOV A, Ra; XRA Rb; MOV Rdst, A."""

    def test_basic_xor(self) -> None:
        """XOR v1, v2, v3 → MOV A, D; XRA E; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.XOR, _reg(1), _reg(2), _reg(3)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}XRA  E",
            f"{_INDENT}MOV  C, A",
        ]

    def test_mnemonic_is_xra(self) -> None:
        """The 8008 XOR instruction is XRA (not XOR)."""
        lines = _gen_lines(_instr(IrOp.XOR, _reg(0), _reg(1), _reg(2)))
        assert any("XRA" in line for line in lines)

    def test_missing_operands(self) -> None:
        """XOR with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.XOR, _reg(1), _reg(2)))
        assert len(lines) == 1
        assert "XOR" in lines[0]


# ===========================================================================
# 13. TestEmitNot
# ===========================================================================


class TestEmitNot:
    """NOT emits load-Ra-safely; XRI 0xFF; MOV Rdst, A.

    The 8008 has no bitwise NOT — XOR with 0xFF flips all bits.
    When Ra is C (v1), _load_a substitutes ``MVI A, 0; ADD C`` to avoid
    the ``MOV A, C = IN 7`` hardware trap, adding one extra line.
    """

    def test_basic_not(self) -> None:
        """NOT v2, v1 — Ra=C uses safe 2-instruction load (4 lines total)."""
        lines = _gen_lines(_instr(IrOp.NOT, _reg(2), _reg(1)))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}XRI  0xFF",
            f"{_INDENT}MOV  D, A",
        ]

    def test_xri_0xff(self) -> None:
        """XRI 0xFF is used for bitwise complement (flip all 8 bits)."""
        lines = _gen_lines(_instr(IrOp.NOT, _reg(1), _reg(2)))
        assert any("XRI  0xFF" in line for line in lines)

    def test_three_lines_when_ra_not_c(self) -> None:
        """NOT v1, v0 — Ra=B (not C): standard 3-line sequence."""
        lines = _gen_lines(_instr(IrOp.NOT, _reg(1), _reg(0)))
        assert len(lines) == 3

    def test_all_destination_registers(self) -> None:
        """NOT writes to the correct destination for each virtual register.

        Source is always v0=B here (not C), so each emits exactly 3 lines
        and the MOV dst instruction is always at index 2.
        """
        for vreg, preg in _PREG.items():
            lines = _gen_lines(_instr(IrOp.NOT, _reg(vreg), _reg(0)))
            assert lines[2] == f"{_INDENT}MOV  {preg}, A"

    def test_missing_operands(self) -> None:
        """NOT with fewer than 2 operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.NOT))
        assert len(lines) == 1
        assert "NOT" in lines[0]


# ===========================================================================
# 14. TestEmitCmpEq
# ===========================================================================


class TestEmitCmpEq:
    """CMP_EQ materialises Ra==Rb into Rdst using a 6-or-7-line sequence.

    When Ra is C (v1), the safe _load_a expansion adds one line, giving 7 total.
    """

    def test_exact_sequence(self) -> None:
        """CMP_EQ v2, v1, v0 — Ra=C: 7-line sequence with safe load."""
        gen = CodeGenerator()
        prog = _prog(_instr(IrOp.CMP_EQ, _reg(2), _reg(1), _reg(0)))
        lines = gen.generate(prog).strip().splitlines()[1:]  # skip ORG
        # _load_a("C") = [MVI A, 0; ADD C] → 2 lines; rest is the 5-line pattern
        assert len(lines) == 7
        assert lines[0] == f"{_INDENT}MVI  A, 0"
        assert lines[1] == f"{_INDENT}ADD  C"
        assert lines[2] == f"{_INDENT}CMP  B"
        assert lines[3] == f"{_INDENT}MVI  D, 1"
        assert lines[4].startswith(f"{_INDENT}JTZ  ")
        assert lines[5] == f"{_INDENT}MVI  D, 0"
        # Label definition at column 0
        assert not lines[6].startswith(" ")
        assert lines[6].endswith(":")

    def test_label_on_jtz_matches_definition(self) -> None:
        """The JTZ target label matches the label definition that follows."""
        lines = _gen_lines(_instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)))
        jtz_line = next(line for line in lines if "JTZ" in line)
        label_name = jtz_line.split("JTZ  ")[1].strip()
        label_def = f"{label_name}:"
        assert label_def in lines

    def test_missing_operands(self) -> None:
        """CMP_EQ with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.CMP_EQ, _reg(1), _reg(2)))
        assert len(lines) == 1
        assert "CMP_EQ" in lines[0]


# ===========================================================================
# 15. TestEmitCmpNe
# ===========================================================================


class TestEmitCmpNe:
    """CMP_NE materialises Ra!=Rb into Rdst (inverted logic from CMP_EQ)."""

    def test_exact_sequence(self) -> None:
        """CMP_NE v2, v1, v0 — Ra=C: 7-line sequence with safe load."""
        lines = _gen_lines(_instr(IrOp.CMP_NE, _reg(2), _reg(1), _reg(0)))
        # _load_a("C") = [MVI A, 0; ADD C] inserts before CMP B
        assert lines[0] == f"{_INDENT}MVI  A, 0"
        assert lines[1] == f"{_INDENT}ADD  C"
        assert lines[2] == f"{_INDENT}CMP  B"
        assert lines[3] == f"{_INDENT}MVI  D, 0"
        assert "JTZ" in lines[4]
        assert lines[5] == f"{_INDENT}MVI  D, 1"

    def test_six_lines_when_ra_not_c(self) -> None:
        """CMP_NE emits 6 lines when Ra is not C (v2=D here)."""
        lines = _gen_lines(_instr(IrOp.CMP_NE, _reg(1), _reg(2), _reg(3)))
        assert len(lines) == 6

    def test_label_matches(self) -> None:
        """JTZ target in CMP_NE matches the trailing label definition."""
        lines = _gen_lines(_instr(IrOp.CMP_NE, _reg(1), _reg(2), _reg(3)))
        jtz_line = next(line for line in lines if "JTZ" in line)
        label_name = jtz_line.split("JTZ  ")[1].strip()
        assert f"{label_name}:" in lines

    def test_missing_operands(self) -> None:
        """CMP_NE with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.CMP_NE))
        assert len(lines) == 1
        assert "CMP_NE" in lines[0]


# ===========================================================================
# 16. TestEmitCmpLt
# ===========================================================================


class TestEmitCmpLt:
    """CMP_LT materialises Ra<Rb (unsigned) using the CY flag."""

    def test_exact_sequence(self) -> None:
        """CMP_LT v2, v1, v0 — Ra=C: 7-line sequence with safe load."""
        lines = _gen_lines(_instr(IrOp.CMP_LT, _reg(2), _reg(1), _reg(0)))
        # _load_a("C") = [MVI A, 0; ADD C] inserts before CMP B
        assert lines[0] == f"{_INDENT}MVI  A, 0"
        assert lines[1] == f"{_INDENT}ADD  C"
        assert lines[2] == f"{_INDENT}CMP  B"
        assert lines[3] == f"{_INDENT}MVI  D, 1"
        assert "JTC" in lines[4]  # JTC = Jump if Carry True (CY=1 = borrow)
        assert lines[5] == f"{_INDENT}MVI  D, 0"

    def test_six_lines_when_ra_not_c(self) -> None:
        """CMP_LT emits 6 lines when Ra is not C (v2=D here)."""
        lines = _gen_lines(_instr(IrOp.CMP_LT, _reg(1), _reg(2), _reg(3)))
        assert len(lines) == 6

    def test_uses_jtc_not_jtz(self) -> None:
        """CMP_LT uses JTC (carry flag), not JTZ (zero flag)."""
        lines = _gen_lines(_instr(IrOp.CMP_LT, _reg(1), _reg(2), _reg(3)))
        branch_lines = [line for line in lines if "JTZ" in line or "JTC" in line]
        assert all("JTC" in line for line in branch_lines)

    def test_missing_operands(self) -> None:
        """CMP_LT with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.CMP_LT, _reg(1)))
        assert len(lines) == 1
        assert "CMP_LT" in lines[0]


# ===========================================================================
# 17. TestEmitCmpGt
# ===========================================================================


class TestEmitCmpGt:
    """CMP_GT uses operand-swap: Ra > Rb ⟺ Rb < Ra (Rb in A, CMP Ra)."""

    def test_operand_swap(self) -> None:
        """CMP_GT v2, v1, v3 loads Rb (v3=E) into A, not Ra (v1=C)."""
        lines = _gen_lines(_instr(IrOp.CMP_GT, _reg(2), _reg(1), _reg(3)))
        # First MOV should load Rb (v3=E), not Ra (v1=C)
        assert lines[0] == f"{_INDENT}MOV  A, E"   # Rb in accumulator
        assert lines[1] == f"{_INDENT}CMP  C"       # CMP Ra

    def test_six_lines(self) -> None:
        """CMP_GT emits exactly 6 lines."""
        lines = _gen_lines(_instr(IrOp.CMP_GT, _reg(1), _reg(2), _reg(3)))
        assert len(lines) == 6

    def test_uses_jtc(self) -> None:
        """CMP_GT uses JTC (carry flag) for unsigned greater-than."""
        lines = _gen_lines(_instr(IrOp.CMP_GT, _reg(1), _reg(2), _reg(3)))
        branch_lines = [line for line in lines if "JTC" in line]
        assert len(branch_lines) == 1

    def test_different_from_cmp_lt(self) -> None:
        """CMP_GT and CMP_LT produce different first two instructions."""
        lt_lines = _gen_lines(_instr(IrOp.CMP_LT, _reg(2), _reg(1), _reg(3)))
        gt_lines = _gen_lines(_instr(IrOp.CMP_GT, _reg(2), _reg(1), _reg(3)))
        # LT: MOV A, Ra; CMP Rb  vs  GT: MOV A, Rb; CMP Ra (swapped)
        assert lt_lines[0] != gt_lines[0]

    def test_missing_operands(self) -> None:
        """CMP_GT with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.CMP_GT))
        assert len(lines) == 1
        assert "CMP_GT" in lines[0]


# ===========================================================================
# 18. TestEmitBranchZ
# ===========================================================================


class TestEmitBranchZ:
    """BRANCH_Z emits load-Rcond-safely; CPI 0; JTZ lbl."""

    def test_exact_sequence(self) -> None:
        """BRANCH_Z v1, loop_end — Rcond=C: 4-line sequence with safe load."""
        lines = _gen_lines(_instr(IrOp.BRANCH_Z, _reg(1), _lbl("loop_end")))
        # _load_a("C") = [MVI A, 0; ADD C] then CPI 0; JTZ loop_end
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}CPI  0",
            f"{_INDENT}JTZ  loop_end",
        ]

    def test_three_lines_when_rcond_not_c(self) -> None:
        """BRANCH_Z emits 3 lines when Rcond is not C (v2=D here)."""
        lines = _gen_lines(_instr(IrOp.BRANCH_Z, _reg(2), _lbl("done")))
        assert len(lines) == 3

    def test_uses_jtz(self) -> None:
        """BRANCH_Z uses JTZ (Jump if Zero True), not JFZ."""
        lines = _gen_lines(_instr(IrOp.BRANCH_Z, _reg(1), _lbl("x")))
        assert any("JTZ" in line for line in lines)
        assert not any("JFZ" in line for line in lines)

    def test_missing_operands(self) -> None:
        """BRANCH_Z with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.BRANCH_Z))
        assert len(lines) == 1
        assert "BRANCH_Z" in lines[0]

    def test_wrong_operand_types(self) -> None:
        """BRANCH_Z with wrong types emits a comment."""
        lines = _gen_lines(_instr(IrOp.BRANCH_Z, _imm(0), _reg(1)))
        assert len(lines) == 1


# ===========================================================================
# 19. TestEmitBranchNz
# ===========================================================================


class TestEmitBranchNz:
    """BRANCH_NZ emits MOV A, Rcond; CPI 0; JFZ lbl."""

    def test_exact_sequence(self) -> None:
        """BRANCH_NZ v2, loop_body → 3-instruction sequence."""
        lines = _gen_lines(_instr(IrOp.BRANCH_NZ, _reg(2), _lbl("loop_body")))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}CPI  0",
            f"{_INDENT}JFZ  loop_body",
        ]

    def test_uses_jfz(self) -> None:
        """BRANCH_NZ uses JFZ (Jump if Zero False), not JTZ."""
        lines = _gen_lines(_instr(IrOp.BRANCH_NZ, _reg(1), _lbl("x")))
        assert any("JFZ" in line for line in lines)
        assert not any("JTZ" in line for line in lines)

    def test_missing_operands(self) -> None:
        """BRANCH_NZ with missing operands emits a comment."""
        lines = _gen_lines(_instr(IrOp.BRANCH_NZ))
        assert len(lines) == 1
        assert "BRANCH_NZ" in lines[0]


# ===========================================================================
# 20. TestEmitJump
# ===========================================================================


class TestEmitJump:
    """JUMP emits JMP label (unconditional branch)."""

    def test_basic_jump(self) -> None:
        """JUMP loop_start → JMP loop_start."""
        lines = _gen_lines(_instr(IrOp.JUMP, _lbl("loop_start")))
        assert lines == [f"{_INDENT}JMP  loop_start"]

    def test_one_line(self) -> None:
        """JUMP always emits exactly 1 line."""
        lines = _gen_lines(_instr(IrOp.JUMP, _lbl("somewhere")))
        assert len(lines) == 1

    def test_mnemonic_is_jmp(self) -> None:
        """The 8008 unconditional branch is JMP (not JUMP or JA)."""
        lines = _gen_lines(_instr(IrOp.JUMP, _lbl("x")))
        assert lines[0].startswith(f"{_INDENT}JMP  ")

    def test_missing_operand(self) -> None:
        """JUMP with no label operand emits a comment."""
        lines = _gen_lines(_instr(IrOp.JUMP))
        assert len(lines) == 1
        assert "JUMP" in lines[0]


# ===========================================================================
# 21. TestEmitCall
# ===========================================================================


class TestEmitCall:
    """CALL emits CAL label (subroutine call)."""

    def test_basic_call(self) -> None:
        """CALL _fn_main → CAL _fn_main."""
        lines = _gen_lines(_instr(IrOp.CALL, _lbl("_fn_main")))
        assert lines == [f"{_INDENT}CAL  _fn_main"]

    def test_one_line(self) -> None:
        """CALL always emits exactly 1 line."""
        lines = _gen_lines(_instr(IrOp.CALL, _lbl("sub")))
        assert len(lines) == 1

    def test_mnemonic_is_cal(self) -> None:
        """The 8008 subroutine call is CAL (not CALL or JSR)."""
        lines = _gen_lines(_instr(IrOp.CALL, _lbl("f")))
        assert lines[0].startswith(f"{_INDENT}CAL  ")

    def test_missing_operand(self) -> None:
        """CALL with no label operand emits a comment."""
        lines = _gen_lines(_instr(IrOp.CALL))
        assert len(lines) == 1
        assert "CALL" in lines[0]


# ===========================================================================
# 22. TestEmitRet
# ===========================================================================


class TestEmitRet:
    """RET emits MVI A, 0; ADD C; RFC (copy return value via ALU, then return).

    Why not MOV A, C?
    -----------------
    ``MOV A, C`` encodes as 0x79 = ``01_111_001`` — group=01, sss=001.  On the
    Intel 8008, group=01 with sss=001 is *always* decoded as ``IN 7`` (read
    from input port 7), NOT a register-to-register move.  The C register
    cannot be read in group=01 because its code (001) collides with the IN
    instruction marker.

    The safe workaround uses the group=10 ALU path where sss=001 correctly
    reads the C register:

        MVI  A, 0       ; A ← 0 (clears CY)
        ADD  C          ; A ← 0 + C = C  (CY=0 since C ≤ 127 in practice)
        RFC             ; Return if Carry False — always fires because CY=0
    """

    def test_exact_sequence(self) -> None:
        """RET → MVI A, 0; ADD C; RFC."""
        lines = _gen_lines(_instr(IrOp.RET))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ADD  C",
            f"{_INDENT}RFC",
        ]

    def test_three_lines(self) -> None:
        """RET always emits exactly 3 lines (MVI, ADD, RFC)."""
        lines = _gen_lines(_instr(IrOp.RET))
        assert len(lines) == 3

    def test_return_value_moved_via_add(self) -> None:
        """C register (v1) is copied to A via ADD C in group-10 ALU path."""
        lines = _gen_lines(_instr(IrOp.RET))
        # MVI A, 0 clears A so ADD C gives A = C exactly
        assert lines[0] == f"{_INDENT}MVI  A, 0"
        assert lines[1] == f"{_INDENT}ADD  C"

    def test_rfc_is_unconditional_return(self) -> None:
        """RFC (Return if Carry False) fires unconditionally: CY=0 after MVI A,0+ADD C."""
        lines = _gen_lines(_instr(IrOp.RET))
        assert lines[2] == f"{_INDENT}RFC"


# ===========================================================================
# 23. TestEmitHalt
# ===========================================================================


class TestEmitHalt:
    """HALT emits HLT (halt the processor)."""

    def test_exact_instruction(self) -> None:
        """HALT → HLT."""
        lines = _gen_lines(_instr(IrOp.HALT))
        assert lines == [f"{_INDENT}HLT"]

    def test_one_line(self) -> None:
        """HALT always emits exactly 1 line."""
        lines = _gen_lines(_instr(IrOp.HALT))
        assert len(lines) == 1


# ===========================================================================
# 24. TestEmitNop
# ===========================================================================


class TestEmitNop:
    """NOP emits a comment (8008 has no dedicated NOP instruction)."""

    def test_emits_comment(self) -> None:
        """NOP produces a comment line, not a real instruction."""
        lines = _gen_lines(_instr(IrOp.NOP))
        assert len(lines) == 1
        assert lines[0].startswith(f"{_INDENT};")

    def test_mentions_nop(self) -> None:
        """The NOP comment mentions 'NOP' so it's identifiable in disassembly."""
        lines = _gen_lines(_instr(IrOp.NOP))
        assert "NOP" in lines[0]


# ===========================================================================
# 25. TestEmitComment
# ===========================================================================


class TestEmitComment:
    """COMMENT emits a semicolon comment line."""

    def test_comment_with_label_operand(self) -> None:
        """COMMENT with an IrLabel operand emits '; name'."""
        lines = _gen_lines(_instr(IrOp.COMMENT, _lbl("load tape")))
        assert lines == [f"{_INDENT}; load tape"]

    def test_comment_with_immediate_operand(self) -> None:
        """COMMENT with an IrImmediate operand emits '; value'."""
        lines = _gen_lines(_instr(IrOp.COMMENT, _imm(42)))
        assert lines == [f"{_INDENT}; 42"]

    def test_empty_comment(self) -> None:
        """COMMENT with no operands emits a bare semicolon line."""
        lines = _gen_lines(_instr(IrOp.COMMENT))
        assert lines == [f"{_INDENT};"]

    def test_starts_with_semicolon(self) -> None:
        """Every COMMENT line starts with '; '."""
        lines = _gen_lines(_instr(IrOp.COMMENT, _lbl("test")))
        assert lines[0].startswith(f"{_INDENT}; ")


# ===========================================================================
# 26. TestEmitSyscall
# ===========================================================================


class TestEmitSyscall:
    """SYSCALL expands to the appropriate 8008 inline sequence."""

    def test_syscall_3_adc(self) -> None:
        """SYSCALL 3 (adc): MOV A, D; ADC E; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(3)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}ADC  E",
            f"{_INDENT}MOV  C, A",
        ]

    def test_syscall_4_sbb(self) -> None:
        """SYSCALL 4 (sbb): MOV A, D; SBB E; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(4)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}SBB  E",
            f"{_INDENT}MOV  C, A",
        ]

    def test_syscall_11_rlc(self) -> None:
        """SYSCALL 11 (rlc): MOV A, D; RLC; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(11)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}RLC",
            f"{_INDENT}MOV  C, A",
        ]

    def test_syscall_12_rrc(self) -> None:
        """SYSCALL 12 (rrc): MOV A, D; RRC; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(12)))
        assert f"{_INDENT}RRC" in lines

    def test_syscall_13_ral(self) -> None:
        """SYSCALL 13 (ral): MOV A, D; RAL; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(13)))
        assert f"{_INDENT}RAL" in lines

    def test_syscall_14_rar(self) -> None:
        """SYSCALL 14 (rar): MOV A, D; RAR; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(14)))
        assert f"{_INDENT}RAR" in lines

    def test_syscall_15_carry(self) -> None:
        """SYSCALL 15 (carry): MVI A, 0; ACI 0; MOV C, A."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(15)))
        assert lines == [
            f"{_INDENT}MVI  A, 0",
            f"{_INDENT}ACI  0",
            f"{_INDENT}MOV  C, A",
        ]

    def test_syscall_16_parity(self) -> None:
        """SYSCALL 16 (parity): ORA A + conditional branch sequence."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(16)))
        assert lines[0] == f"{_INDENT}MOV  A, D"
        assert lines[1] == f"{_INDENT}ORA  A"
        assert lines[2] == f"{_INDENT}MVI  C, 0"
        assert "JFP" in lines[3]  # Jump if Parity False (odd parity → skip)
        assert lines[4] == f"{_INDENT}MVI  C, 1"

    def test_syscall_parity_six_lines(self) -> None:
        """SYSCALL 16 (parity) emits exactly 6 lines."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(16)))
        assert len(lines) == 6

    def test_syscall_parity_label_matches(self) -> None:
        """JFP target in parity sequence matches the trailing label definition."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(16)))
        jfp_line = next(line for line in lines if "JFP" in line)
        label_name = jfp_line.split("JFP  ")[1].strip()
        assert f"{label_name}:" in lines

    @pytest.mark.parametrize("port", range(8))
    def test_syscall_in_ports(self, port: int) -> None:
        """SYSCALL 20+p (in): IN p; MOV C, A — for each port 0–7."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(20 + port)))
        assert lines == [
            f"{_INDENT}IN   {port}",
            f"{_INDENT}MOV  C, A",
        ]

    @pytest.mark.parametrize("port", range(24))
    def test_syscall_out_ports(self, port: int) -> None:
        """SYSCALL 40+p (out): MOV A, D; OUT p — for each port 0–23."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(40 + port)))
        assert lines == [
            f"{_INDENT}MOV  A, D",
            f"{_INDENT}OUT  {port}",
        ]

    def test_syscall_unknown_number(self) -> None:
        """Unrecognised SYSCALL number emits a comment (defensive)."""
        lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(99)))
        assert len(lines) == 1
        assert "SYSCALL" in lines[0]
        assert "99" in lines[0]

    def test_syscall_missing_operand(self) -> None:
        """SYSCALL with no operand emits a comment."""
        lines = _gen_lines(_instr(IrOp.SYSCALL))
        assert len(lines) == 1
        assert "SYSCALL" in lines[0]

    def test_rotation_three_lines(self) -> None:
        """All rotation SYSCALLs emit exactly 3 lines."""
        for num in (11, 12, 13, 14):
            lines = _gen_lines(_instr(IrOp.SYSCALL, _imm(num)))
            assert len(lines) == 3, f"SYSCALL {num} emitted {len(lines)} lines"


# ===========================================================================
# 27. TestLabelCounter
# ===========================================================================


class TestLabelCounter:
    """The label counter generates unique labels across all calls.

    Labels like cmp_0, cmp_1, ... must not collide when multiple comparison
    or parity instructions appear in the same program.
    """

    def test_counter_starts_at_zero(self) -> None:
        """The first generated label uses suffix 0."""
        gen = CodeGenerator()
        assert gen._label_count == 0

    def test_counter_increments_per_comparison(self) -> None:
        """Each comparison instruction advances the counter by one."""
        gen = CodeGenerator()
        prog = _prog(
            _instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)),
        )
        gen.generate(prog)
        assert gen._label_count == 2

    def test_no_label_collisions(self) -> None:
        """Multiple comparison instructions produce distinct labels."""
        prog = _prog(
            _instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.CMP_NE, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.CMP_LT, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.CMP_GT, _reg(1), _reg(2), _reg(3)),
        )
        asm = CodeGenerator().generate(prog)
        # Extract all label definitions (lines that end with ':' and no indent)
        label_defs = [
            ln.rstrip(":")
            for ln in asm.splitlines()
            if ln.endswith(":") and not ln.startswith(" ")
        ]
        # All label definitions must be unique
        assert len(label_defs) == len(set(label_defs))

    def test_parity_label_distinct_from_cmp_label(self) -> None:
        """Parity syscall and comparison labels are distinct."""
        prog = _prog(
            _instr(IrOp.SYSCALL, _imm(16)),   # parity — uses _next_label()
            _instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)),
        )
        asm = CodeGenerator().generate(prog)
        label_defs = [
            ln.rstrip(":")
            for ln in asm.splitlines()
            if ln.endswith(":") and not ln.startswith(" ")
        ]
        assert len(label_defs) == len(set(label_defs))

    def test_generator_stateful_across_generate_calls(self) -> None:
        """Calling generate() twice on the same CodeGenerator object
        increments the counter across calls (no reset between programs).
        """
        gen = CodeGenerator()
        prog1 = _prog(_instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)))
        prog2 = _prog(_instr(IrOp.CMP_EQ, _reg(1), _reg(2), _reg(3)))
        gen.generate(prog1)
        gen.generate(prog2)
        # Counter should be at 2 after two comparisons
        assert gen._label_count == 2


# ===========================================================================
# 28. TestUnknownOpcode
# ===========================================================================


class TestUnknownOpcode:
    """Unknown opcodes produce a safe comment fallback rather than crashing."""

    def test_known_fallback(self) -> None:
        """The catch-all case in _emit produces a comment with the opcode name."""
        # We patch a known opcode's dispatch by directly calling _emit with
        # a non-matching enum.  Because IrOp is an IntEnum we can construct
        # a custom value that won't match any case.
        gen = CodeGenerator()
        # Use IrOp.LOAD_WORD (value=4) which is unsupported by the 8008 backend
        fake_instr = IrInstruction(opcode=IrOp.LOAD_WORD, operands=[])
        lines = gen._emit(fake_instr)
        # Should be a comment mentioning the opcode
        assert len(lines) == 1
        assert lines[0].startswith(f"{_INDENT};")


# ===========================================================================
# 29. TestFullProgram
# ===========================================================================


class TestFullProgram:
    """End-to-end tests with realistic multi-instruction programs."""

    def test_halt_only_program(self) -> None:
        """Minimal program: ORG + HLT."""
        asm = _gen(_instr(IrOp.HALT))
        assert f"{_INDENT}ORG 0x0000" in asm
        assert f"{_INDENT}HLT" in asm

    def test_load_and_return(self) -> None:
        """Program that loads a constant and returns it.

        RET emits MVI A, 0; ADD C; RFC (NOT MOV A, C, which encodes as IN 7).
        """
        asm = _gen(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(42)),
            _instr(IrOp.RET),
        )
        assert f"{_INDENT}MVI  C, 42" in asm
        # MOV A, C is forbidden: encodes as 0x79 → IN 7 (not register copy)
        assert f"{_INDENT}MOV  A, C" not in asm
        assert f"{_INDENT}MVI  A, 0" in asm
        assert f"{_INDENT}ADD  C" in asm
        assert f"{_INDENT}RFC" in asm

    def test_function_call_sequence(self) -> None:
        """Main + function call sequence."""
        asm = _gen(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.CALL, _lbl("_fn_add")),
            _instr(IrOp.HALT),
            _instr(IrOp.LABEL, _lbl("_fn_add")),
            _instr(IrOp.ADD, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.RET),
        )
        assert "_start:" in asm
        assert f"{_INDENT}CAL  _fn_add" in asm
        assert f"{_INDENT}HLT" in asm
        assert "_fn_add:" in asm
        assert f"{_INDENT}RFC" in asm

    def test_memory_access_sequence(self) -> None:
        """LOAD_ADDR + LOAD_BYTE sequence for reading a static.

        LOAD_BYTE must NOT emit ``MOV A, M`` (= 0x7E = CAL on the 8008).
        Instead it emits ``MVI A, 0; ADD M`` (safe group-10 path).
        """
        asm = _gen(
            _instr(IrOp.LOAD_ADDR, _reg(1), _lbl("tape")),
            _instr(IrOp.LOAD_BYTE, _reg(1), _reg(1), _reg(0)),
        )
        assert f"{_INDENT}MVI  H, hi(tape)" in asm
        assert f"{_INDENT}MVI  L, lo(tape)" in asm
        # Safe load: MVI A, 0 + ADD M (not MOV A, M which encodes as CAL!)
        assert f"{_INDENT}MVI  A, 0" in asm
        assert f"{_INDENT}ADD  M" in asm
        assert f"{_INDENT}MOV  A, M" not in asm   # this would be CAL
        assert f"{_INDENT}MOV  C, A" in asm

    def test_store_sequence(self) -> None:
        """LOAD_ADDR + STORE_BYTE sequence for writing a static."""
        asm = _gen(
            _instr(IrOp.LOAD_ADDR, _reg(1), _lbl("counter")),
            _instr(IrOp.STORE_BYTE, _reg(2), _reg(1), _reg(0)),
        )
        assert f"{_INDENT}MVI  H, hi(counter)" in asm
        assert f"{_INDENT}MVI  L, lo(counter)" in asm
        assert f"{_INDENT}MOV  A, D" in asm
        assert f"{_INDENT}MOV  M, A" in asm

    def test_conditional_branch_loop(self) -> None:
        """While-loop pattern: LABEL + BRANCH_Z + body + JUMP."""
        asm = _gen(
            _instr(IrOp.LABEL, _lbl("loop_top")),
            _instr(IrOp.BRANCH_Z, _reg(1), _lbl("loop_end")),
            _instr(IrOp.ADD_IMM, _reg(1), _reg(1), _imm(255)),  # decrement
            _instr(IrOp.JUMP, _lbl("loop_top")),
            _instr(IrOp.LABEL, _lbl("loop_end")),
        )
        assert "loop_top:" in asm
        assert f"{_INDENT}JTZ  loop_end" in asm
        assert f"{_INDENT}JMP  loop_top" in asm
        assert "loop_end:" in asm

    def test_syscall_in_out_sequence(self) -> None:
        """Read from port 0, write to port 0 sequence."""
        asm = _gen(
            _instr(IrOp.SYSCALL, _imm(20)),   # in(0) → C
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(0)),  # copy C → D
            _instr(IrOp.SYSCALL, _imm(40)),   # out(0, D)
        )
        assert f"{_INDENT}IN   0" in asm
        assert f"{_INDENT}MOV  C, A" in asm
        assert f"{_INDENT}OUT  0" in asm

    def test_instruction_ordering_preserved(self) -> None:
        """Instructions appear in the program order in the output."""
        asm = _gen(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(1)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(2)),
            _instr(IrOp.LOAD_IMM, _reg(3), _imm(3)),
        )
        idx_c = asm.index("MVI  C, 1")
        idx_d = asm.index("MVI  D, 2")
        idx_e = asm.index("MVI  E, 3")
        assert idx_c < idx_d < idx_e

    def test_arithmetic_chain(self) -> None:
        """a + b - c using ADD and SUB."""
        asm = _gen(
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(10)),
            _instr(IrOp.LOAD_IMM, _reg(3), _imm(5)),
            _instr(IrOp.ADD, _reg(1), _reg(2), _reg(3)),   # v1 = 10+5=15
            _instr(IrOp.LOAD_IMM, _reg(3), _imm(3)),
            _instr(IrOp.SUB, _reg(1), _reg(1), _reg(3)),   # v1 = 15-3=12
            _instr(IrOp.RET),
        )
        assert f"{_INDENT}ADD  E" in asm
        assert f"{_INDENT}SUB  E" in asm

    def test_bitwise_operations(self) -> None:
        """AND, OR, XOR, NOT all appear in the correct sequence."""
        asm = _gen(
            _instr(IrOp.AND, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.OR, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.XOR, _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.NOT, _reg(1), _reg(2)),
        )
        assert "ANA  E" in asm
        assert "ORA  E" in asm
        assert "XRA  E" in asm
        assert "XRI  0xFF" in asm


# ===========================================================================
# 30. TestIrToIntel8008Compiler
# ===========================================================================


class TestIrToIntel8008Compiler:
    """IrToIntel8008Compiler orchestrates validation then code generation."""

    def _make_valid_prog(self) -> IrProgram:
        """Build a simple valid program that passes all 8008 constraints."""
        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)])
        )
        prog.add_instruction(IrInstruction(IrOp.HALT, []))
        return prog

    def test_compile_valid_program(self) -> None:
        """compile() returns assembly text for a valid program."""
        compiler = IrToIntel8008Compiler()
        asm = compiler.compile(self._make_valid_prog())
        assert isinstance(asm, str)
        assert "ORG 0x0000" in asm
        assert "MVI  C, 42" in asm

    def test_compile_raises_on_invalid(self) -> None:
        """compile() raises IrValidationError for programs that fail validation."""
        prog = IrProgram(entry_label="_start", version=1)
        # LOAD_WORD is rejected by the validator (no word ops on 8008)
        prog.add_instruction(IrInstruction(IrOp.LOAD_WORD, []))
        compiler = IrToIntel8008Compiler()
        with pytest.raises(IrValidationError):
            compiler.compile(prog)

    def test_compile_error_message_contains_rule(self) -> None:
        """IrValidationError includes the violated rule name."""
        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(IrInstruction(IrOp.LOAD_WORD, []))
        compiler = IrToIntel8008Compiler()
        with pytest.raises(IrValidationError) as exc_info:
            compiler.compile(prog)
        assert exc_info.value.rule  # rule must not be empty

    def test_compile_multiple_errors_combined(self) -> None:
        """When multiple rules fail, errors are concatenated in the message."""
        prog = IrProgram(entry_label="_start", version=1)
        # LOAD_WORD (no_word_ops) + STORE_WORD (no_word_ops) — possibly 1 rule
        # Let's use LOAD_WORD and also add too many virtual registers
        prog.add_instruction(IrInstruction(IrOp.LOAD_WORD, []))
        # Add 7 distinct virtual registers (v0–v6) to hit register_count
        for i in range(7):
            prog.add_instruction(
                IrInstruction(IrOp.LOAD_IMM, [IrRegister(i), IrImmediate(0)])
            )
        compiler = IrToIntel8008Compiler()
        with pytest.raises(IrValidationError) as exc_info:
            compiler.compile(prog)
        # The error is raised — message contains details
        assert str(exc_info.value)

    def test_intel8008_backend_alias(self) -> None:
        """Intel8008Backend is an alias for IrToIntel8008Compiler."""
        assert Intel8008Backend is IrToIntel8008Compiler

    def test_compiler_has_validator_and_codegen(self) -> None:
        """The compiler exposes validator and codegen attributes."""
        compiler = IrToIntel8008Compiler()
        assert hasattr(compiler, "validator")
        assert hasattr(compiler, "codegen")

    def test_compile_returns_string(self) -> None:
        """compile() always returns a str (not bytes or None)."""
        asm = IrToIntel8008Compiler().compile(self._make_valid_prog())
        assert type(asm) is str

    def test_compile_ends_with_newline(self) -> None:
        """compile() output ends with a newline (POSIX convention)."""
        asm = IrToIntel8008Compiler().compile(self._make_valid_prog())
        assert asm.endswith("\n")

    def test_multiple_compiles_from_same_instance(self) -> None:
        """The same compiler can compile multiple programs sequentially."""
        compiler = IrToIntel8008Compiler()
        prog1 = self._make_valid_prog()
        prog2 = self._make_valid_prog()
        asm1 = compiler.compile(prog1)
        asm2 = compiler.compile(prog2)
        assert "ORG 0x0000" in asm1
        assert "ORG 0x0000" in asm2


# ===========================================================================
# 31. TestPublicApi
# ===========================================================================


class TestPublicApi:
    """Module-level validate() and generate_asm() convenience functions."""

    def _valid_prog(self) -> IrProgram:
        """Minimal valid program."""
        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(IrInstruction(IrOp.HALT, []))
        return prog

    def test_validate_valid_program(self) -> None:
        """validate() returns an empty list for a valid program."""
        errors = validate(self._valid_prog())
        assert errors == []

    def test_validate_invalid_program(self) -> None:
        """validate() returns non-empty list for an invalid program."""
        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(IrInstruction(IrOp.LOAD_WORD, []))
        errors = validate(prog)
        assert len(errors) > 0

    def test_generate_asm_valid_program(self) -> None:
        """generate_asm() returns assembly text for a valid program."""
        asm = generate_asm(self._valid_prog())
        assert isinstance(asm, str)
        assert "ORG 0x0000" in asm
        assert "HLT" in asm

    def test_generate_asm_does_not_validate(self) -> None:
        """generate_asm() skips validation (may produce asm for invalid IR)."""
        # A LOAD_WORD would fail validation but generate_asm skips it
        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(IrInstruction(IrOp.LOAD_WORD, []))
        # Should not raise IrValidationError — fallback comment instead
        asm = generate_asm(prog)
        assert isinstance(asm, str)

    def test_validate_returns_list_of_errors(self) -> None:
        """validate() returns a list of IrValidationError objects."""
        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(IrInstruction(IrOp.LOAD_WORD, []))
        errors = validate(prog)
        assert all(isinstance(e, IrValidationError) for e in errors)

    def test_generate_asm_ends_with_newline(self) -> None:
        """generate_asm() output ends with a newline."""
        asm = generate_asm(self._valid_prog())
        assert asm.endswith("\n")


# ===========================================================================
# 32. TestRegisterMapping
# ===========================================================================


class TestRegisterMapping:
    """Verify the complete virtual → physical register mapping."""

    @pytest.mark.parametrize("vreg,expected_preg", [
        (0, "B"),
        (1, "C"),
        (2, "D"),
        (3, "E"),
        (4, "H"),
        (5, "L"),
    ])
    def test_vreg_mapping(self, vreg: int, expected_preg: str) -> None:
        """Each virtual register maps to the documented physical register."""
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _reg(vreg), _imm(0)))
        assert f"MVI  {expected_preg}, 0" in lines[0]

    def test_unknown_register_falls_back(self) -> None:
        """Virtual registers beyond v5 fall back gracefully (no crash)."""
        # v99 is not in the table; _preg() returns "B" as fallback
        lines = _gen_lines(_instr(IrOp.LOAD_IMM, _reg(99), _imm(7)))
        assert len(lines) == 1
        assert "MVI  B, 7" in lines[0]
