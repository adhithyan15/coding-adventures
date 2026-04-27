"""test_assembler.py — Integration tests for the Intel4004Assembler.

These tests exercise the full two-pass pipeline: lexing → symbol collection
→ code emission.  They test complete programs rather than individual
instruction encodings (which are covered by test_encoder.py).
"""

from __future__ import annotations

import pytest

from intel_4004_assembler import assemble, Intel4004Assembler, AssemblerError


class TestSimplePrograms:
    """Short programs with known output byte sequences."""

    def test_nop_only(self) -> None:
        """Single NOP → 0x00."""
        binary = assemble("    NOP")
        assert binary == bytes([0x00])

    def test_hlt_only(self) -> None:
        """HLT (simulator extension) → 0x01."""
        binary = assemble("    HLT")
        assert binary == bytes([0x01])

    def test_ldm_xch_hlt(self) -> None:
        """ORG 0x000, LDM 5, XCH R2, HLT → D5 B2 01."""
        src = """\
    ORG 0x000
_start:
    LDM 5
    XCH R2
    HLT
"""
        binary = assemble(src)
        assert binary == bytes([0xD5, 0xB2, 0x01])

    def test_org_sets_pc_but_nop_still_at_offset(self) -> None:
        """ORG 0x000 followed by NOP still emits at byte offset 0."""
        binary = assemble("    ORG 0x000\n    NOP")
        assert binary == bytes([0x00])

    def test_multiple_no_operand_instructions(self) -> None:
        """Three no-op style instructions encode correctly in sequence."""
        src = "    CLB\n    CLC\n    IAC"
        binary = assemble(src)
        assert binary == bytes([0xF0, 0xF1, 0xF2])

    def test_fim_p0_0x42(self) -> None:
        """FIM P0, 0x42 → 0x20 0x42."""
        binary = assemble("    FIM P0, 0x42")
        assert binary == bytes([0x20, 0x42])

    def test_ldm_jun_self_loop(self) -> None:
        """Self-loop JUN $ correctly encodes to JUN PC."""
        # LDM 0 is at offset 0, JUN $ is at offset 1.
        # JUN $ → addr=1 → 0x40 0x01
        binary = assemble("    LDM 0\n    JUN $")
        assert binary == bytes([0xD0, 0x40, 0x01])

    def test_jcn_condition_nibble(self) -> None:
        """JCN with condition 0x4 emits 0x14 + addr8."""
        # NOP at 0, JCN at 1, NOP at 3
        src = "    NOP\n    JCN 0x4, 0x00\n    NOP"
        binary = assemble(src)
        # JCN → 0x14 0x00; NOP before = 0x00; NOP after = 0x00
        assert binary == bytes([0x00, 0x14, 0x00, 0x00])


class TestLabelResolution:
    """Labels must resolve correctly — including forward references."""

    def test_backward_reference(self) -> None:
        """Jumping back to an earlier label resolves correctly."""
        src = """\
    ORG 0x000
loop:
    NOP
    JUN loop
"""
        binary = assemble(src)
        # NOP at 0, JUN at 1 → addr=0 → 0x40 0x00
        assert binary == bytes([0x00, 0x40, 0x00])

    def test_forward_reference(self) -> None:
        """Jumping forward to a label that appears later in the file."""
        src = """\
    ORG 0x000
    JUN end
    NOP
end:
    HLT
"""
        binary = assemble(src)
        # JUN at 0 → addr=3 → 0x40 0x03; NOP at 2 → 0x00; HLT at 3 → 0x01
        assert binary == bytes([0x40, 0x03, 0x00, 0x01])

    def test_self_loop_dollar(self) -> None:
        """JUN $ at PC=2 resolves to addr=2."""
        src = "    NOP\n    NOP\n    JUN $"
        binary = assemble(src)
        # NOP(0), NOP(1), JUN $(2) → 0x40 0x02
        assert binary == bytes([0x00, 0x00, 0x40, 0x02])

    def test_multiple_labels(self) -> None:
        """Multiple labels in one program resolve independently."""
        src = """\
    ORG 0x000
a:
    NOP
b:
    JUN a
c:
    JUN b
"""
        binary = assemble(src)
        # a=0: NOP(0)
        # b=1: JUN a → addr=0 → 0x40 0x00 (bytes 1,2)
        # c=3: JUN b → addr=1 → 0x40 0x01 (bytes 3,4)
        assert binary == bytes([0x00, 0x40, 0x00, 0x40, 0x01])

    def test_jcn_with_label(self) -> None:
        """JCN referencing a label resolves to the correct address."""
        src = """\
    ORG 0x000
start:
    LD R2
    JCN 0x4, done
    NOP
done:
    HLT
"""
        binary = assemble(src)
        # LD R2 at 0 → 0xA2
        # JCN 0x4, done at 1 (2 bytes) → done is at 4 → 0x14 0x04
        # NOP at 3 → 0x00
        # HLT at 4 → 0x01
        assert binary == bytes([0xA2, 0x14, 0x04, 0x00, 0x01])


class TestFullProgram:
    """Larger programs that mirror real code-generator output."""

    def test_counter_loop(self) -> None:
        """Counter loop program from the task description."""
        src = """\
    ORG 0x000
_start:
    LDM 5
    XCH R2
    NOP
loop_0_start:
    LD R2
    JCN 0x4, loop_0_end
    ADD_IMM R2, R2, 1
    JUN loop_0_start
loop_0_end:
    JUN $
"""
        binary = assemble(src)
        # Compute expected bytes:
        # Offset 0: LDM 5      → 0xD5
        # Offset 1: XCH R2     → 0xB2
        # Offset 2: NOP        → 0x00
        # Offset 3: LD R2      → 0xA2          (loop_0_start = 3)
        # Offset 4: JCN 0x4, loop_0_end → 0x14 0x0B (loop_0_end = 11)
        # Offset 6: ADD_IMM R2,R2,1 → LDM 1=0xD1, ADD R2=0x82
        # Offset 8: JUN loop_0_start → addr=3 → 0x40 0x03
        # Offset 10: JUN $ → addr=10 → 0x40 0x0A  (loop_0_end = 10)
        expected = bytes([
            0xD5,        # LDM 5
            0xB2,        # XCH R2
            0x00,        # NOP
            0xA2,        # LD R2
            0x14, 0x0A,  # JCN 0x4, loop_0_end (addr=10)
            0xD1, 0x82,  # ADD_IMM R2, R2, 1 (LDM 1 + ADD R2)
            0x40, 0x03,  # JUN loop_0_start (addr=3)
            0x40, 0x0A,  # JUN $ (addr=10)
        ])
        assert binary == expected

    def test_subroutine_call(self) -> None:
        """JMS + BBL pattern (subroutine call and return)."""
        src = """\
    ORG 0x000
main:
    LDM 3
    JMS sub
    HLT
sub:
    INC R0
    BBL 0
"""
        binary = assemble(src)
        # main=0: LDM 3 → 0xD3 (offset 0)
        # JMS sub at 1 → sub=4 → 0x50 0x04 (offsets 1,2)
        # HLT at 3 → 0x01
        # sub=4: INC R0 → 0x60 (offset 4)
        # BBL 0 → 0xC0 (offset 5)
        # BBL 0 = 0xC0 | 0 = 0xC0
        assert binary == bytes([0xD3, 0x50, 0x04, 0x01, 0x60, 0xC0])

    def test_fim_and_src(self) -> None:
        """FIM + SRC to set RAM address."""
        src = """\
    ORG 0x000
    FIM P1, 0x10
    SRC P1
    WRM
"""
        binary = assemble(src)
        # FIM P1, 0x10 → 0x22 0x10 (2 bytes)
        # SRC P1 → 0x23 (1 byte)
        # WRM → 0xE0 (1 byte)
        assert binary == bytes([0x22, 0x10, 0x23, 0xE0])


class TestErrorCases:
    """Assembly must raise AssemblerError for bad input."""

    def test_undefined_label(self) -> None:
        with pytest.raises(AssemblerError, match="Undefined label"):
            assemble("    JUN ghost_label")

    def test_unknown_mnemonic(self) -> None:
        with pytest.raises(AssemblerError, match="Unknown mnemonic"):
            assemble("    FOOBAR")

    def test_ldm_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            assemble("    LDM 16")

    def test_jcn_bad_condition(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            assemble("    JCN 16, 0x00")

    def test_org_address_too_large(self) -> None:
        """ORG with address > 0xFFF (12-bit limit) raises AssemblerError."""
        with pytest.raises(AssemblerError, match="exceeds Intel 4004"):
            assemble("    ORG 0x1000\n    NOP")

    def test_org_address_at_limit(self) -> None:
        """ORG 0xFFF is the maximum valid address."""
        # Should not raise -- 0xFFF is within the 12-bit space.
        binary = assemble("    ORG 0xFFF\n    NOP")
        assert binary[-1] == 0x00  # NOP at the last position


class TestReusableInstance:
    """The assembler instance is stateless and can be reused."""

    def test_two_programs_in_sequence(self) -> None:
        asm = Intel4004Assembler()
        b1 = asm.assemble("    NOP")
        b2 = asm.assemble("    HLT")
        assert b1 == bytes([0x00])
        assert b2 == bytes([0x01])

    def test_second_call_independent_symbol_table(self) -> None:
        """Symbol table from first call must not bleed into second call."""
        asm = Intel4004Assembler()
        asm.assemble("loop:\n    JUN loop")
        # Second program does NOT define 'loop' — should raise.
        with pytest.raises(AssemblerError, match="Undefined label"):
            asm.assemble("    JUN loop")


class TestPublicAPI:
    """Tests for the module-level convenience function and __init__ exports."""

    def test_assemble_function(self) -> None:
        from intel_4004_assembler import assemble as fn
        assert fn("    NOP") == bytes([0x00])

    def test_assembler_error_exported(self) -> None:
        from intel_4004_assembler import AssemblerError as E
        assert issubclass(E, Exception)

    def test_intel4004_assembler_exported(self) -> None:
        from intel_4004_assembler import Intel4004Assembler as A
        assert callable(A)

    def test_assemble_returns_bytes(self) -> None:
        result = assemble("    NOP")
        assert isinstance(result, bytes)


class TestOrgPaddingAndOffset:
    """ORG with padding fills output with NOP (0x00) bytes."""

    def test_org_at_zero_no_padding(self) -> None:
        """ORG 0 at the start doesn't add any bytes before the first instruction."""
        binary = assemble("    ORG 0x000\n    HLT")
        assert binary == bytes([0x01])

    def test_org_advances_pc_with_padding(self) -> None:
        """ORG 3 inserts 3 NOP bytes before first instruction."""
        binary = assemble("    ORG 0x000\n    ORG 0x003\n    HLT")
        assert binary == bytes([0x00, 0x00, 0x00, 0x01])

    def test_label_after_org_has_correct_address(self) -> None:
        """Label defined after ORG 0x010 should have address 0x10."""
        src = """\
    ORG 0x000
    ORG 0x010
target:
    NOP
    JUN target
"""
        binary = assemble(src)
        # target = 0x10; JUN target at 0x11 (2 bytes)
        # JUN 0x010: high nibble = (0x10 >> 8) & 0xF = 0, low byte = 0x10
        # So JUN encodes as 0x40 0x10
        # After padding: 16 bytes of 0x00, then NOP(0x00), JUN(0x40, 0x10)
        assert len(binary) == 19  # 16 padding + NOP + JUN(2)
        assert binary[16] == 0x00   # NOP
        assert binary[17] == 0x40   # JUN high nibble = 0 (0x10 fits in low byte)
        assert binary[18] == 0x10   # JUN low byte = 0x10
