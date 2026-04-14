"""Tests for IrValidator — Intel 4004 hardware constraint checking.

Each test function focuses on one of the five validation rules.  We test:
  - The failing case (should produce errors)
  - The passing case (should return empty list)
  - Edge cases (boundary values, accumulation of multiple errors)

The validator is designed to accumulate ALL errors in a single pass so
programmers can fix everything at once.  The tests verify this property.
"""

from __future__ import annotations

import pytest
from compiler_ir import (
    IDGenerator,
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from intel_4004_ir_validator import IrValidationError, IrValidator

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_program(
    *instructions: IrInstruction,
    data: list[IrDataDecl] | None = None,
) -> IrProgram:
    """Build a minimal IrProgram with the given instructions."""
    prog = IrProgram(entry_label="_start")
    for instr in instructions:
        prog.add_instruction(instr)
    if data:
        for decl in data:
            prog.add_data(decl)
    return prog


def gen_id() -> IDGenerator:
    """Return a fresh IDGenerator."""
    return IDGenerator()


# ---------------------------------------------------------------------------
# Rule 1: no_word_ops
# ---------------------------------------------------------------------------


class TestNoWordOps:
    """Rule: LOAD_WORD and STORE_WORD are not supported on Intel 4004."""

    def test_load_word_raises_error(self) -> None:
        """LOAD_WORD should produce a 'no_word_ops' validation error."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        assert len(errors) == 1
        assert errors[0].rule == "no_word_ops"
        assert "LOAD_WORD" in errors[0].message

    def test_store_word_raises_error(self) -> None:
        """STORE_WORD should produce a 'no_word_ops' validation error."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.STORE_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        assert len(errors) == 1
        assert errors[0].rule == "no_word_ops"
        assert "STORE_WORD" in errors[0].message

    def test_both_load_word_and_store_word_both_reported(self) -> None:
        """Both LOAD_WORD and STORE_WORD should produce two errors."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            ),
            IrInstruction(
                IrOp.STORE_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            ),
        )
        errors = IrValidator().validate(prog)
        rules = [e.rule for e in errors]
        assert rules.count("no_word_ops") == 2

    def test_multiple_load_word_reports_only_once(self) -> None:
        """Multiple LOAD_WORD instructions produce only one error per opcode type."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            ),
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(3), IrRegister(0), IrRegister(1)],
                id=g.next(),
            ),
        )
        errors = IrValidator().validate(prog)
        word_op_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert len(word_op_errors) == 1

    def test_load_byte_store_byte_are_allowed(self) -> None:
        """LOAD_BYTE and STORE_BYTE are valid — only WORD variants are forbidden."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_BYTE,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            ),
            IrInstruction(
                IrOp.STORE_BYTE,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            ),
        )
        errors = IrValidator().validate(prog)
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert word_errors == []

    def test_empty_program_passes(self) -> None:
        """An empty program has no LOAD_WORD/STORE_WORD instructions."""
        errors = IrValidator().validate(IrProgram(entry_label="_start"))
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert word_errors == []


# ---------------------------------------------------------------------------
# Rule 2: static_ram
# ---------------------------------------------------------------------------


class TestStaticRam:
    """Rule: total IrDataDecl sizes must not exceed 160 bytes."""

    def test_exactly_160_bytes_passes(self) -> None:
        """160 bytes is exactly the 4004 limit — should pass."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="buf", size=160, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert ram_errors == []

    def test_161_bytes_fails(self) -> None:
        """161 bytes exceeds the 160-byte limit."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="buf", size=161, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert len(ram_errors) == 1
        assert "161" in ram_errors[0].message

    def test_sum_of_multiple_decls_too_large(self) -> None:
        """Sum of multiple data declarations exceeding 160 bytes should fail."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="a", size=100, init=0))
        prog.add_data(IrDataDecl(label="b", size=80, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert len(ram_errors) == 1
        assert "180" in ram_errors[0].message

    def test_sum_of_decls_exactly_at_limit(self) -> None:
        """Two declarations summing to exactly 160 bytes should pass."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="a", size=80, init=0))
        prog.add_data(IrDataDecl(label="b", size=80, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert ram_errors == []

    def test_no_data_declarations_passes(self) -> None:
        """No data declarations means 0 bytes used — trivially passes."""
        prog = IrProgram(entry_label="_start")
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert ram_errors == []

    def test_single_byte_passes(self) -> None:
        """A single 1-byte declaration should pass."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="cell", size=1, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert ram_errors == []


# ---------------------------------------------------------------------------
# Rule 3: call_depth
# ---------------------------------------------------------------------------


class TestCallDepth:
    """Rule: static call graph depth must not exceed 2."""

    def _make_prog_with_calls(self, call_chain: list[str]) -> IrProgram:
        """Build a program with a linear call chain.

        call_chain = ["main", "foo", "bar"] means:
          main calls foo, foo calls bar.
        """
        prog = IrProgram(entry_label=call_chain[0])
        for idx, fname in enumerate(call_chain):
            prog.add_instruction(
                IrInstruction(IrOp.LABEL, [IrLabel(fname)], id=-1)
            )
            if idx + 1 < len(call_chain):
                prog.add_instruction(
                    IrInstruction(
                        IrOp.CALL, [IrLabel(call_chain[idx + 1])], id=idx
                    )
                )
            prog.add_instruction(IrInstruction(IrOp.RET, [], id=idx + 100))
        return prog

    def test_depth_0_passes(self) -> None:
        """A single function with no calls has depth 0 — passes."""
        prog = self._make_prog_with_calls(["main"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_1_passes(self) -> None:
        """main → foo is depth 1 — passes."""
        prog = self._make_prog_with_calls(["main", "foo"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_2_passes(self) -> None:
        """main → foo → bar is depth 2 — exactly at the limit, passes."""
        prog = self._make_prog_with_calls(["main", "foo", "bar"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_3_fails(self) -> None:
        """main → foo → bar → baz is depth 3 — exceeds the limit."""
        prog = self._make_prog_with_calls(["main", "foo", "bar", "baz"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert len(depth_errors) == 1
        assert depth_errors[0].rule == "call_depth"

    def test_no_calls_passes(self) -> None:
        """A program with no CALL instructions at all passes."""
        g = gen_id()
        prog = make_program(
            IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1),
            IrInstruction(IrOp.HALT, [], id=g.next()),
        )
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_recursive_call_reports_error(self) -> None:
        """A self-recursive function is rejected as unsupported."""
        prog = IrProgram(entry_label="recurse")
        prog.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("recurse")], id=-1)
        )
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("recurse")], id=0)
        )
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=1))
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert len(depth_errors) == 1
        assert "recursive" in depth_errors[0].message.lower()

    def test_mutual_recursion_reports_error(self) -> None:
        """A cyclic call graph is rejected before code generation."""
        prog = IrProgram(entry_label="a")
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("a")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("b")], id=0))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=1))
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("b")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("a")], id=2))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=3))
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert len(depth_errors) == 1
        assert "cycle" in depth_errors[0].message.lower()


# ---------------------------------------------------------------------------
# Rule 4: register_count
# ---------------------------------------------------------------------------


class TestRegisterCount:
    """Rule: at most 12 distinct virtual register indices."""

    def _make_prog_with_regs(self, n_regs: int) -> IrProgram:
        """Build a program using v0..v(n_regs-1) each in a LOAD_IMM."""
        prog = IrProgram(entry_label="_start")
        g = IDGenerator()
        for reg_idx in range(n_regs):
            prog.add_instruction(
                IrInstruction(
                    IrOp.LOAD_IMM,
                    [IrRegister(reg_idx), IrImmediate(reg_idx % 16)],
                    id=g.next(),
                )
            )
        return prog

    def test_12_registers_passes(self) -> None:
        """Exactly 12 distinct registers is at the limit — passes."""
        prog = self._make_prog_with_regs(12)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_13_registers_fails(self) -> None:
        """13 distinct registers exceeds the limit of 12."""
        prog = self._make_prog_with_regs(13)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert len(reg_errors) == 1
        assert "13" in reg_errors[0].message

    def test_1_register_passes(self) -> None:
        """A single register is well within the limit."""
        prog = self._make_prog_with_regs(1)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_duplicate_register_indices_count_once(self) -> None:
        """The same register used multiple times should count only once."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=g.next()
            ),
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)], id=g.next()
            ),
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=g.next()
            ),
        )
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_no_registers_passes(self) -> None:
        """A program with no register operands (e.g., just HALT) passes."""
        g = gen_id()
        prog = make_program(IrInstruction(IrOp.HALT, [], id=g.next()))
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []


# ---------------------------------------------------------------------------
# Rule 5: operand_range
# ---------------------------------------------------------------------------


class TestOperandRange:
    """Rule: LOAD_IMM immediate must be in [0, 255]."""

    def test_value_0_passes(self) -> None:
        """0 is the minimum valid value."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert range_errors == []

    def test_value_255_passes(self) -> None:
        """255 is the maximum valid value (u8 max)."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(255)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert range_errors == []

    def test_value_256_fails(self) -> None:
        """256 exceeds u8 range — should fail."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(256)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert len(range_errors) == 1
        assert "256" in range_errors[0].message

    def test_negative_value_fails(self) -> None:
        """Negative values are out of u8 range."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(-1)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert len(range_errors) == 1
        assert "-1" in range_errors[0].message

    def test_multiple_out_of_range_all_reported(self) -> None:
        """Multiple out-of-range LOAD_IMM immediates should all produce errors."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(300)], id=g.next()
            ),
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(500)], id=g.next()
            ),
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert len(range_errors) == 2

    def test_add_imm_not_checked(self) -> None:
        """The range check only applies to LOAD_IMM, not ADD_IMM or other ops."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.ADD_IMM,
                [IrRegister(2), IrRegister(2), IrImmediate(300)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert range_errors == []

    def test_value_15_passes(self) -> None:
        """15 is a common 4-bit immediate — passes."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(15)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        range_errors = [e for e in errors if e.rule == "operand_range"]
        assert range_errors == []


# ---------------------------------------------------------------------------
# Error accumulation: multiple rules can fail simultaneously
# ---------------------------------------------------------------------------


class TestErrorAccumulation:
    """Verify that multiple failures are reported together, not one at a time."""

    def test_word_ops_and_static_ram_both_reported(self) -> None:
        """A program with LOAD_WORD and too much RAM should get both errors."""
        g = gen_id()
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="big", size=200, init=0))
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        rules = {e.rule for e in errors}
        assert "no_word_ops" in rules
        assert "static_ram" in rules

    def test_all_five_rules_can_fail_at_once(self) -> None:
        """A pathologically bad program should trigger all five rules."""
        # Build a program that violates every rule:
        # - LOAD_WORD and STORE_WORD (no_word_ops)
        # - 200 bytes of data (static_ram)
        # - call chain of depth 3 (call_depth)
        # - 13 distinct registers (register_count)
        # - LOAD_IMM with value 300 (operand_range)
        g = IDGenerator()
        prog = IrProgram(entry_label="_start")

        # static_ram violation
        prog.add_data(IrDataDecl(label="big", size=200, init=0))

        # no_word_ops violations
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        prog.add_instruction(
            IrInstruction(
                IrOp.STORE_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )

        # call_depth violation: depth 3
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("a")], id=-1))
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("b")], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("b")], id=-1))
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("c")], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("c")], id=-1))
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("d")], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("d")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))

        # register_count violation: 13 distinct registers (v0..v12)
        for reg_idx in range(13):
            prog.add_instruction(
                IrInstruction(
                    IrOp.LOAD_IMM,
                    [IrRegister(reg_idx), IrImmediate(0)],
                    id=g.next(),
                )
            )

        # operand_range violation
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(300)], id=g.next()
            )
        )

        errors = IrValidator().validate(prog)
        rules = {e.rule for e in errors}
        assert "no_word_ops" in rules
        assert "static_ram" in rules
        assert "call_depth" in rules
        assert "register_count" in rules
        assert "operand_range" in rules


# ---------------------------------------------------------------------------
# IrValidationError type tests
# ---------------------------------------------------------------------------


class TestIrValidationError:
    """Tests for the IrValidationError class itself."""

    def test_str_representation(self) -> None:
        """__str__ should format as '[rule] message'."""
        e = IrValidationError(rule="static_ram", message="too big")
        assert str(e) == "[static_ram] too big"

    def test_is_exception_subclass(self) -> None:
        """IrValidationError must be an Exception so the backend can raise it."""
        e = IrValidationError(rule="no_word_ops", message="forbidden op")
        assert isinstance(e, Exception)

    def test_can_be_raised(self) -> None:
        """IrValidationError should be raise-able like any Exception."""
        with pytest.raises(IrValidationError):
            raise IrValidationError(rule="static_ram", message="too much RAM")

    def test_equality(self) -> None:
        """Two IrValidationError objects with same rule+message are equal."""
        a = IrValidationError(rule="operand_range", message="value 300")
        b = IrValidationError(rule="operand_range", message="value 300")
        assert a == b

    def test_inequality(self) -> None:
        """Different rule means not equal."""
        a = IrValidationError(rule="operand_range", message="value 300")
        b = IrValidationError(rule="register_count", message="value 300")
        assert a != b

    def test_rule_attribute_accessible(self) -> None:
        """The rule attribute should be readable."""
        e = IrValidationError(rule="call_depth", message="too deep")
        assert e.rule == "call_depth"

    def test_message_attribute_accessible(self) -> None:
        """The message attribute should be readable."""
        e = IrValidationError(rule="call_depth", message="too deep")
        assert e.message == "too deep"
