"""Tests for CodeGenerator — IR → Intel 4004 assembly translation.

Each test verifies that one or more IR opcodes produce the correct assembly
output.  We test:
  - Instruction indentation (4 spaces)
  - Label format (bare identifier + colon, no indent)
  - Exact assembly mnemonics and operands
  - File header (ORG 0x000)
  - Multi-line instruction sequences (e.g., LOAD_IMM → LDM + XCH)
"""

from __future__ import annotations

from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from intel_4004_backend.codegen import CodeGenerator

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def asm_lines(prog: IrProgram) -> list[str]:
    """Generate assembly and split into non-empty lines."""
    return [ln for ln in CodeGenerator().generate(prog).splitlines() if ln.strip()]


def single_instr(op: IrOp, operands: list) -> IrProgram:  # type: ignore[type-arg]
    """Build a minimal program with a single instruction."""
    g = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(op, operands, id=g.next()))
    return prog


def multi_instr(
    *pairs: tuple[IrOp, list],  # type: ignore[type-arg]
) -> IrProgram:
    """Build a program from (opcode, operands) pairs."""
    g = IDGenerator()
    prog = IrProgram(entry_label="_start")
    for op, operands in pairs:
        id_ = -1 if op == IrOp.LABEL else g.next()
        prog.add_instruction(IrInstruction(op, operands, id=id_))
    return prog


# ---------------------------------------------------------------------------
# Header
# ---------------------------------------------------------------------------


class TestHeader:
    """The generated file must start with ORG 0x000."""

    def test_org_directive_present(self) -> None:
        """Every generated program starts with '    ORG 0x000'."""
        prog = IrProgram(entry_label="_start")
        asm = CodeGenerator().generate(prog)
        first_line = asm.splitlines()[0]
        assert first_line == "    ORG 0x000"

    def test_output_ends_with_newline(self) -> None:
        """The output string should end with a newline."""
        prog = IrProgram(entry_label="_start")
        asm = CodeGenerator().generate(prog)
        assert asm.endswith("\n")


# ---------------------------------------------------------------------------
# LABEL
# ---------------------------------------------------------------------------


class TestLabel:
    """LABEL lbl → lbl: (at column 0, no indent)."""

    def test_label_at_column_zero(self) -> None:
        """Labels should have no leading whitespace."""
        prog = multi_instr((IrOp.LABEL, [IrLabel("_start")]))
        lines = asm_lines(prog)
        label_lines = [ln for ln in lines if ":" in ln and not ln.startswith(" ")]
        assert any("_start:" in ln for ln in label_lines)

    def test_label_has_colon_suffix(self) -> None:
        """Label line should be 'name:' exactly."""
        prog = multi_instr((IrOp.LABEL, [IrLabel("loop_start")]))
        lines = asm_lines(prog)
        assert "loop_start:" in lines

    def test_label_not_indented(self) -> None:
        """Label lines do not start with spaces."""
        prog = multi_instr((IrOp.LABEL, [IrLabel("my_func")]))
        lines = asm_lines(prog)
        for ln in lines:
            if "my_func" in ln:
                assert not ln.startswith(" ")


# ---------------------------------------------------------------------------
# LOAD_IMM
# ---------------------------------------------------------------------------


class TestLoadImm:
    """LOAD_IMM vN, k → LDM k + XCH Rn (small k) or FIM Pn, k (large k)."""

    def test_small_immediate_uses_ldm_xch(self) -> None:
        """k ≤ 15 should produce LDM k followed by XCH Rn."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)])
        lines = asm_lines(prog)
        assert any("LDM 5" in ln for ln in lines)
        assert any("XCH R2" in ln for ln in lines)

    def test_large_immediate_uses_fim(self) -> None:
        """k > 15 should produce FIM Pn, k."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(200)])
        lines = asm_lines(prog)
        assert any("FIM" in ln and "200" in ln for ln in lines)

    def test_zero_immediate(self) -> None:
        """k=0 should use LDM 0 + XCH R2."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)])
        lines = asm_lines(prog)
        assert any("LDM 0" in ln for ln in lines)
        assert any("XCH R2" in ln for ln in lines)

    def test_boundary_15_uses_ldm(self) -> None:
        """k=15 is the boundary — should use LDM 15."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(15)])
        lines = asm_lines(prog)
        assert any("LDM 15" in ln for ln in lines)

    def test_boundary_16_uses_fim(self) -> None:
        """k=16 crosses into FIM territory."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(16)])
        lines = asm_lines(prog)
        assert any("FIM" in ln for ln in lines)
        assert not any("LDM 16" in ln for ln in lines)

    def test_instructions_are_indented(self) -> None:
        """Assembly instructions should be indented with 4 spaces."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)])
        raw_lines = CodeGenerator().generate(prog).splitlines()
        instr_lines = [ln for ln in raw_lines if "LDM" in ln or "XCH" in ln]
        for ln in instr_lines:
            assert ln.startswith("    "), f"Expected 4-space indent, got: {ln!r}"

    def test_register_v3_maps_to_r3(self) -> None:
        """vReg 3 should map to physical register R3."""
        prog = single_instr(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(7)])
        lines = asm_lines(prog)
        assert any("XCH R3" in ln for ln in lines)


# ---------------------------------------------------------------------------
# LOAD_ADDR
# ---------------------------------------------------------------------------


class TestLoadAddr:
    """LOAD_ADDR vN, lbl → FIM Pn, lbl."""

    def test_load_addr_emits_fim_with_label(self) -> None:
        """FIM Pn, label_name should be emitted."""
        prog = single_instr(IrOp.LOAD_ADDR, [IrRegister(4), IrLabel("tape")])
        lines = asm_lines(prog)
        assert any("FIM" in ln and "tape" in ln for ln in lines)

    def test_load_addr_uses_correct_pair(self) -> None:
        """vReg 4 maps to pair P2 (registers R4:R5)."""
        prog = single_instr(IrOp.LOAD_ADDR, [IrRegister(4), IrLabel("buf")])
        lines = asm_lines(prog)
        assert any("FIM P2" in ln for ln in lines)


# ---------------------------------------------------------------------------
# LOAD_BYTE
# ---------------------------------------------------------------------------


class TestLoadByte:
    """LOAD_BYTE dst, base, off → SRC Pbase; RDM; XCH Rdst."""

    def test_load_byte_emits_src_rdm_xch(self) -> None:
        """Should emit SRC, RDM, and XCH in that order."""
        prog = single_instr(
            IrOp.LOAD_BYTE,
            [IrRegister(2), IrRegister(4), IrRegister(5)],
        )
        lines = asm_lines(prog)
        assert any("SRC" in ln for ln in lines)
        assert any("RDM" in ln for ln in lines)
        assert any("XCH R2" in ln for ln in lines)

    def test_load_byte_uses_base_pair(self) -> None:
        """SRC should use the pair register of the base register (v4 → P2)."""
        prog = single_instr(
            IrOp.LOAD_BYTE,
            [IrRegister(2), IrRegister(4), IrRegister(5)],
        )
        lines = asm_lines(prog)
        assert any("SRC P2" in ln for ln in lines)


# ---------------------------------------------------------------------------
# STORE_BYTE
# ---------------------------------------------------------------------------


class TestStoreByte:
    """STORE_BYTE src, base, off → LD Rsrc; SRC Pbase; WRM."""

    def test_store_byte_emits_ld_src_wrm(self) -> None:
        """Should emit LD, SRC, WRM in that order."""
        prog = single_instr(
            IrOp.STORE_BYTE,
            [IrRegister(2), IrRegister(4), IrRegister(5)],
        )
        lines = asm_lines(prog)
        assert any("LD R2" in ln for ln in lines)
        assert any("SRC" in ln for ln in lines)
        assert any("WRM" in ln for ln in lines)

    def test_store_byte_uses_base_pair(self) -> None:
        """SRC should reference the pair of the base register."""
        prog = single_instr(
            IrOp.STORE_BYTE,
            [IrRegister(2), IrRegister(4), IrRegister(5)],
        )
        lines = asm_lines(prog)
        assert any("SRC P2" in ln for ln in lines)


# ---------------------------------------------------------------------------
# ADD
# ---------------------------------------------------------------------------


class TestAdd:
    """ADD vR, vA, vB → LD Ra; ADD Rb; XCH Rr."""

    def test_add_emits_ld_add_xch(self) -> None:
        """Should emit LD Ra, ADD Rb, XCH Rr."""
        prog = single_instr(
            IrOp.ADD, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any("LD R2" in ln for ln in lines)
        assert any("ADD R3" in ln for ln in lines)
        assert any("XCH R4" in ln for ln in lines)


# ---------------------------------------------------------------------------
# ADD_IMM
# ---------------------------------------------------------------------------


class TestAddImm:
    """ADD_IMM vN, vN, k — register += immediate."""

    def test_add_imm_small_k(self) -> None:
        """Small k (≤ 15) should use LDM + scratch register approach."""
        prog = single_instr(
            IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(3)]
        )
        lines = asm_lines(prog)
        assert any("LDM 3" in ln for ln in lines)
        assert any("XCH R2" in ln for ln in lines)

    def test_add_imm_large_k(self) -> None:
        """Large k (> 15) should use FIM P7 scratch pair."""
        prog = single_instr(
            IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(20)]
        )
        lines = asm_lines(prog)
        assert any("FIM P7, 20" in ln for ln in lines)


# ---------------------------------------------------------------------------
# SUB
# ---------------------------------------------------------------------------


class TestSub:
    """SUB vR, vA, vB → LD Ra; SUB Rb; XCH Rr."""

    def test_sub_emits_ld_sub_xch(self) -> None:
        """Should emit LD Ra, SUB Rb, XCH Rr."""
        prog = single_instr(
            IrOp.SUB, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any("LD R2" in ln for ln in lines)
        assert any("SUB R3" in ln for ln in lines)
        assert any("XCH R4" in ln for ln in lines)


# ---------------------------------------------------------------------------
# AND_IMM
# ---------------------------------------------------------------------------


class TestAndImm:
    """AND_IMM vN, vN, mask — bitwise AND with immediate."""

    def test_and_imm_255_is_noop(self) -> None:
        """AND_IMM with 255 is a no-op on 4004 — emits a comment."""
        prog = single_instr(
            IrOp.AND_IMM, [IrRegister(2), IrRegister(2), IrImmediate(255)]
        )
        lines = asm_lines(prog)
        assert any("no-op" in ln.lower() or "255" in ln for ln in lines)

    def test_and_imm_15_is_noop(self) -> None:
        """AND_IMM mask=15 is a no-op on 4004 (registers are 4-bit hardware).

        The 4004 has no AND instruction. Masking to 0xF is hardware-enforced
        since registers can never exceed 15. The codegen emits a comment.
        """
        prog = single_instr(
            IrOp.AND_IMM, [IrRegister(2), IrRegister(2), IrImmediate(15)]
        )
        lines = asm_lines(prog)
        # No real instructions emitted — only the ORG header and a comment.
        def is_real_instr(ln: str) -> bool:
            s = ln.strip()
            return bool(s) and not s.startswith(";") and not s.startswith("ORG")
        assert not any(is_real_instr(ln) for ln in lines)

    def test_and_imm_255_is_noop(self) -> None:
        """AND_IMM mask=255 is a no-op on 4004 (8-bit pairs naturally cap at 255)."""
        prog = single_instr(
            IrOp.AND_IMM, [IrRegister(2), IrRegister(2), IrImmediate(255)]
        )
        lines = asm_lines(prog)
        def is_real_instr(ln: str) -> bool:
            s = ln.strip()
            return bool(s) and not s.startswith(";") and not s.startswith("ORG")
        assert not any(is_real_instr(ln) for ln in lines)

    def test_and_imm_arbitrary_mask_emits_comment(self) -> None:
        """AND_IMM with an unsupported mask emits a comment (no real instruction)."""
        prog = single_instr(
            IrOp.AND_IMM, [IrRegister(2), IrRegister(2), IrImmediate(7)]
        )
        lines = asm_lines(prog)
        assert any("AND_IMM" in ln for ln in lines)


# ---------------------------------------------------------------------------
# AND
# ---------------------------------------------------------------------------


class TestAnd:
    """AND vR, vA, vB → LD Ra; AND Rb; XCH Rr."""

    def test_and_emits_ld_and_xch(self) -> None:
        """Should emit LD Ra, AND Rb, XCH Rr."""
        prog = single_instr(
            IrOp.AND, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any("LD R2" in ln for ln in lines)
        assert any("AND R3" in ln for ln in lines)
        assert any("XCH R4" in ln for ln in lines)


# ---------------------------------------------------------------------------
# CMP_LT
# ---------------------------------------------------------------------------


class TestCmpLt:
    """CMP_LT vR, vA, vB → LD Ra; SUB Rb; TCS; XCH Rr."""

    def test_cmp_lt_emits_tcs(self) -> None:
        """Should include TCS (transfer carry subtract)."""
        prog = single_instr(
            IrOp.CMP_LT, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any("TCS" in ln for ln in lines)
        assert any("LD R2" in ln for ln in lines)
        assert any("SUB R3" in ln for ln in lines)
        assert any("XCH R4" in ln for ln in lines)


# ---------------------------------------------------------------------------
# CMP_EQ
# ---------------------------------------------------------------------------


class TestCmpEq:
    """CMP_EQ vR, vA, vB → LD Ra; SUB Rb; CMA; IAC; XCH Rr."""

    def test_cmp_eq_emits_cma_iac(self) -> None:
        """Should include CMA and IAC after SUB."""
        prog = single_instr(
            IrOp.CMP_EQ, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any("CMA" in ln for ln in lines)
        assert any("IAC" in ln for ln in lines)
        assert any("LD R2" in ln for ln in lines)
        assert any("SUB R3" in ln for ln in lines)
        assert any("XCH R4" in ln for ln in lines)


# ---------------------------------------------------------------------------
# CMP_NE / CMP_GT
# ---------------------------------------------------------------------------


class TestCmpNeGt:
    """CMP_NE and CMP_GT emit a comment (no direct 4004 equivalent)."""

    def test_cmp_ne_emits_comment(self) -> None:
        """CMP_NE should emit a semicolon-prefixed comment."""
        prog = single_instr(
            IrOp.CMP_NE, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any(";" in ln and "CMP_NE" in ln for ln in lines)

    def test_cmp_gt_emits_comment(self) -> None:
        """CMP_GT should emit a semicolon-prefixed comment."""
        prog = single_instr(
            IrOp.CMP_GT, [IrRegister(4), IrRegister(2), IrRegister(3)]
        )
        lines = asm_lines(prog)
        assert any(";" in ln and "CMP_GT" in ln for ln in lines)


# ---------------------------------------------------------------------------
# JUMP
# ---------------------------------------------------------------------------


class TestJump:
    """JUMP lbl → JUN lbl."""

    def test_jump_emits_jun(self) -> None:
        """Should emit JUN label_name."""
        prog = single_instr(IrOp.JUMP, [IrLabel("loop_top")])
        lines = asm_lines(prog)
        assert any("JUN loop_top" in ln for ln in lines)


# ---------------------------------------------------------------------------
# BRANCH_Z
# ---------------------------------------------------------------------------


class TestBranchZ:
    """BRANCH_Z vN, lbl → LD Rn; JCN 0x4, lbl."""

    def test_branch_z_emits_ld_jcn_4(self) -> None:
        """Should emit LD Rn and JCN 0x4, label."""
        prog = single_instr(IrOp.BRANCH_Z, [IrRegister(2), IrLabel("done")])
        lines = asm_lines(prog)
        assert any("LD R2" in ln for ln in lines)
        assert any("JCN 0x4" in ln and "done" in ln for ln in lines)


# ---------------------------------------------------------------------------
# BRANCH_NZ
# ---------------------------------------------------------------------------


class TestBranchNz:
    """BRANCH_NZ vN, lbl → LD Rn; JCN 0xC, lbl."""

    def test_branch_nz_emits_ld_jcn_c(self) -> None:
        """Should emit LD Rn and JCN 0xC, label."""
        prog = single_instr(IrOp.BRANCH_NZ, [IrRegister(2), IrLabel("loop_top")])
        lines = asm_lines(prog)
        assert any("LD R2" in ln for ln in lines)
        assert any("JCN 0xC" in ln and "loop_top" in ln for ln in lines)


# ---------------------------------------------------------------------------
# CALL
# ---------------------------------------------------------------------------


class TestCall:
    """CALL lbl → JMS lbl."""

    def test_call_emits_jms(self) -> None:
        """Should emit JMS label_name."""
        prog = single_instr(IrOp.CALL, [IrLabel("my_func")])
        lines = asm_lines(prog)
        assert any("JMS my_func" in ln for ln in lines)


# ---------------------------------------------------------------------------
# RET
# ---------------------------------------------------------------------------


class TestRet:
    """RET → BBL 0."""

    def test_ret_emits_bbl_0(self) -> None:
        """Should emit BBL 0."""
        prog = single_instr(IrOp.RET, [])
        lines = asm_lines(prog)
        assert any("BBL 0" in ln for ln in lines)


# ---------------------------------------------------------------------------
# HALT
# ---------------------------------------------------------------------------


class TestHalt:
    """HALT → HLT (simulator halt sentinel, opcode 0x01)."""

    def test_halt_emits_hlt(self) -> None:
        """Should emit HLT — the simulator's halt opcode, not JUN $."""
        prog = single_instr(IrOp.HALT, [])
        lines = asm_lines(prog)
        assert any("HLT" in ln for ln in lines)


# ---------------------------------------------------------------------------
# NOP
# ---------------------------------------------------------------------------


class TestNop:
    """NOP → NOP."""

    def test_nop_emits_nop(self) -> None:
        """Should emit NOP."""
        prog = single_instr(IrOp.NOP, [])
        lines = asm_lines(prog)
        assert any(ln.strip() == "NOP" for ln in lines)


# ---------------------------------------------------------------------------
# COMMENT
# ---------------------------------------------------------------------------


class TestComment:
    """COMMENT text → ; text."""

    def test_comment_emits_semicolon_prefix(self) -> None:
        """Comment instruction should produce a line starting with '; '."""
        prog = single_instr(IrOp.COMMENT, [IrLabel("hello world")])
        lines = asm_lines(prog)
        assert any("; hello world" in ln for ln in lines)


# ---------------------------------------------------------------------------
# SYSCALL
# ---------------------------------------------------------------------------


class TestSyscall:
    """SYSCALL n → ; syscall(n) — not natively supported."""

    def test_syscall_1_write_emits_comment(self) -> None:
        """SYSCALL 1 (WRITE) should emit a comment."""
        prog = single_instr(IrOp.SYSCALL, [IrImmediate(1)])
        lines = asm_lines(prog)
        assert any(";" in ln and "1" in ln for ln in lines)

    def test_syscall_2_read_emits_comment(self) -> None:
        """SYSCALL 2 (READ) should emit a comment."""
        prog = single_instr(IrOp.SYSCALL, [IrImmediate(2)])
        lines = asm_lines(prog)
        assert any(";" in ln and "2" in ln for ln in lines)

    def test_syscall_includes_4004_note(self) -> None:
        """Syscall comment should mention 4004 limitation."""
        prog = single_instr(IrOp.SYSCALL, [IrImmediate(1)])
        lines = asm_lines(prog)
        assert any("4004" in ln for ln in lines)


# ---------------------------------------------------------------------------
# Indentation consistency
# ---------------------------------------------------------------------------


class TestIndentation:
    """All instructions (except labels) must be indented with exactly 4 spaces."""

    def test_instructions_have_4_space_indent(self) -> None:
        """Every non-label, non-empty line should start with 4 spaces."""
        prog = multi_instr(
            (IrOp.LABEL, [IrLabel("_start")]),
            (IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)]),
            (IrOp.HALT, []),
        )
        asm = CodeGenerator().generate(prog)
        for ln in asm.splitlines():
            if not ln.strip():
                continue
            if ":" in ln and not ln.startswith(" "):
                # It's a label — should have no indent
                continue
            # Everything else must start with exactly 4 spaces
            assert ln.startswith("    "), f"Expected 4-space indent, got: {ln!r}"


# ---------------------------------------------------------------------------
# Register pair mapping
# ---------------------------------------------------------------------------


class TestRegisterPairMapping:
    """vReg index → physical pair name mapping."""

    def test_vreg_0_1_map_to_p0(self) -> None:
        """vReg 0 and vReg 1 belong to P0."""
        from intel_4004_backend.codegen import _vreg_to_pair

        assert _vreg_to_pair(0) == "P0"
        assert _vreg_to_pair(1) == "P0"

    def test_vreg_2_3_map_to_p1(self) -> None:
        """vReg 2 and vReg 3 belong to P1."""
        from intel_4004_backend.codegen import _vreg_to_pair

        assert _vreg_to_pair(2) == "P1"
        assert _vreg_to_pair(3) == "P1"

    def test_vreg_4_5_map_to_p2(self) -> None:
        """vReg 4 and vReg 5 belong to P2."""
        from intel_4004_backend.codegen import _vreg_to_pair

        assert _vreg_to_pair(4) == "P2"
        assert _vreg_to_pair(5) == "P2"

    def test_vreg_12_maps_to_p6(self) -> None:
        """vReg 12 belongs to P6 (RAM address pair)."""
        from intel_4004_backend.codegen import _vreg_to_pair

        assert _vreg_to_pair(12) == "P6"
