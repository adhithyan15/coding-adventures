"""Tests for IrValidator — Intel 8008 hardware constraint checking.

Each test class focuses on one of the six validation rules:

  1. no_word_ops    — LOAD_WORD / STORE_WORD are forbidden
  2. static_ram     — total static data must not exceed 8 191 bytes
  3. call_depth     — call graph depth must not exceed 7
  4. register_count — at most 6 distinct virtual register indices
  5. imm_range      — LOAD_IMM and ADD_IMM immediates must be in [0, 255]
  6. syscall_whitelist — SYSCALL numbers must be 8008-valid

Each class tests:
  - Passing case (should return empty list or no errors for that rule)
  - Failing case (should produce the correct error)
  - Edge / boundary cases

The validator accumulates ALL errors in a single pass; separate tests
verify this accumulation property and the error type itself.
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

from intel_8008_ir_validator import IrValidationError, IrValidator

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
    """Return a fresh IDGenerator for unique instruction IDs."""
    return IDGenerator()


def make_call_chain(names: list[str]) -> IrProgram:
    """Build a program with a linear call chain.

    ``names = ["main", "foo", "bar"]`` means main calls foo, foo calls bar.
    Each function has a LABEL and a RET; intermediate functions also have a CALL.
    """
    g = IDGenerator()
    prog = IrProgram(entry_label=names[0])
    for idx, name in enumerate(names):
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel(name)], id=-1))
        if idx + 1 < len(names):
            prog.add_instruction(
                IrInstruction(IrOp.CALL, [IrLabel(names[idx + 1])], id=g.next())
            )
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
    return prog


# ---------------------------------------------------------------------------
# Rule 1: no_word_ops
# ---------------------------------------------------------------------------


class TestNoWordOps:
    """Rule: LOAD_WORD and STORE_WORD are not supported on Intel 8008."""

    def test_load_word_produces_error(self) -> None:
        """LOAD_WORD should produce a 'no_word_ops' error."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        assert any(e.rule == "no_word_ops" for e in errors)

    def test_load_word_error_mentions_load_word(self) -> None:
        """The error message for LOAD_WORD should mention 'LOAD_WORD'."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert len(word_errors) >= 1
        assert "LOAD_WORD" in word_errors[0].message

    def test_store_word_produces_error(self) -> None:
        """STORE_WORD should produce a 'no_word_ops' error."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.STORE_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert len(word_errors) == 1
        assert "STORE_WORD" in word_errors[0].message

    def test_both_load_and_store_word_reported(self) -> None:
        """Both LOAD_WORD and STORE_WORD in the same program produce two errors."""
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
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert len(word_errors) == 2

    def test_multiple_load_word_reported_only_once(self) -> None:
        """Multiple LOAD_WORD instructions produce only one no_word_ops error."""
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
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert len(word_errors) == 1

    def test_load_byte_is_allowed(self) -> None:
        """LOAD_BYTE is valid on the 8008 — only LOAD_WORD is forbidden."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_BYTE,
                [IrRegister(2), IrRegister(3), IrRegister(0)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert word_errors == []

    def test_store_byte_is_allowed(self) -> None:
        """STORE_BYTE is valid on the 8008 — only STORE_WORD is forbidden."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.STORE_BYTE,
                [IrRegister(2), IrRegister(3), IrRegister(0)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert word_errors == []

    def test_empty_program_has_no_word_ops_errors(self) -> None:
        """An empty program trivially passes the no_word_ops check."""
        errors = IrValidator().validate(IrProgram(entry_label="_start"))
        word_errors = [e for e in errors if e.rule == "no_word_ops"]
        assert word_errors == []


# ---------------------------------------------------------------------------
# Rule 2: static_ram
# ---------------------------------------------------------------------------


class TestStaticRam:
    """Rule: total IrDataDecl sizes must not exceed 8 191 bytes."""

    def test_exactly_8191_bytes_passes(self) -> None:
        """8 191 bytes is exactly the 8008 limit — should pass."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="buf", size=8191, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert ram_errors == []

    def test_8192_bytes_fails(self) -> None:
        """8 192 bytes exceeds the 8 191-byte limit."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="buf", size=8192, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert len(ram_errors) == 1
        assert "8192" in ram_errors[0].message

    def test_sum_of_multiple_decls_too_large(self) -> None:
        """Sum of declarations exceeding 8 191 bytes should fail."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="a", size=5000, init=0))
        prog.add_data(IrDataDecl(label="b", size=4000, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert len(ram_errors) == 1
        assert "9000" in ram_errors[0].message

    def test_sum_exactly_at_limit_passes(self) -> None:
        """Two declarations summing to exactly 8 191 bytes should pass."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="a", size=4000, init=0))
        prog.add_data(IrDataDecl(label="b", size=4191, init=0))
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
        """A single 1-byte static declaration should always pass."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="x", size=1, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert ram_errors == []

    def test_error_message_mentions_limit(self) -> None:
        """The error message should mention the 8 191-byte limit."""
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="big", size=10000, init=0))
        errors = IrValidator().validate(prog)
        ram_errors = [e for e in errors if e.rule == "static_ram"]
        assert len(ram_errors) == 1
        assert "8191" in ram_errors[0].message


# ---------------------------------------------------------------------------
# Rule 3: call_depth
# ---------------------------------------------------------------------------


class TestCallDepth:
    """Rule: static call graph depth must not exceed 7."""

    def test_depth_0_passes(self) -> None:
        """A single function with no calls has depth 0 — passes."""
        prog = make_call_chain(["main"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_1_passes(self) -> None:
        """main → foo is depth 1 — passes."""
        prog = make_call_chain(["main", "foo"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_7_passes(self) -> None:
        """A chain of depth 7 is exactly at the limit — should pass."""
        # main → f1 → f2 → f3 → f4 → f5 → f6 → f7 (7 call edges)
        chain = ["main", "f1", "f2", "f3", "f4", "f5", "f6", "f7"]
        prog = make_call_chain(chain)
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_8_fails(self) -> None:
        """A chain of depth 8 exceeds the 8008 hardware stack limit."""
        # main → f1 → … → f8 (8 call edges)
        chain = ["main", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8"]
        prog = make_call_chain(chain)
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert len(depth_errors) == 1
        assert depth_errors[0].rule == "call_depth"

    def test_depth_8_error_mentions_limit(self) -> None:
        """The depth-exceeded error should mention the 7-level limit."""
        chain = ["main", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8"]
        prog = make_call_chain(chain)
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert "7" in depth_errors[0].message

    def test_no_calls_passes(self) -> None:
        """A program with only labels and HALT but no CALL instructions passes."""
        g = gen_id()
        prog = make_program(
            IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1),
            IrInstruction(IrOp.HALT, [], id=g.next()),
        )
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_self_recursive_function_rejected(self) -> None:
        """A self-recursive function is rejected as unsupported."""
        g = gen_id()
        prog = IrProgram(entry_label="recurse")
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("recurse")], id=-1))
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("recurse")], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert len(depth_errors) == 1
        assert "recursive" in depth_errors[0].message.lower()

    def test_mutual_recursion_rejected(self) -> None:
        """A cyclic call graph (mutual recursion) is rejected."""
        g = gen_id()
        prog = IrProgram(entry_label="a")
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("a")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("b")], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("b")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("a")], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert len(depth_errors) == 1
        assert "cycle" in depth_errors[0].message.lower()

    def test_branching_call_graph_depth_measured_correctly(self) -> None:
        """Call depth is the longest path, not the total number of functions."""
        # main calls both foo and bar; bar calls baz.
        # Depth: main→foo = 1, main→bar→baz = 2.  Max = 2. Should pass.
        g = gen_id()
        prog = IrProgram(entry_label="main")
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("main")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("foo")], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("bar")], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("foo")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("bar")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("baz")], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("baz")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []

    def test_depth_3_passes(self) -> None:
        """main → foo → bar → baz (depth 3) is well within the 8008 limit of 7."""
        prog = make_call_chain(["main", "foo", "bar", "baz"])
        errors = IrValidator().validate(prog)
        depth_errors = [e for e in errors if e.rule == "call_depth"]
        assert depth_errors == []


# ---------------------------------------------------------------------------
# Rule 4: register_count
# ---------------------------------------------------------------------------


class TestRegisterCount:
    """Rule: at most 6 distinct virtual register indices (v0–v5)."""

    def _make_prog_with_regs(self, n_regs: int) -> IrProgram:
        """Build a program using v0..v(n_regs-1) each in a LOAD_IMM."""
        prog = IrProgram(entry_label="_start")
        g = IDGenerator()
        for reg_idx in range(n_regs):
            prog.add_instruction(
                IrInstruction(
                    IrOp.LOAD_IMM,
                    [IrRegister(reg_idx), IrImmediate(0)],
                    id=g.next(),
                )
            )
        return prog

    def test_6_registers_passes(self) -> None:
        """Exactly 6 distinct registers (v0–v5) is at the limit — passes."""
        prog = self._make_prog_with_regs(6)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_7_registers_fails(self) -> None:
        """7 distinct registers (v0–v6) exceeds the limit."""
        prog = self._make_prog_with_regs(7)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert len(reg_errors) == 1
        assert "7" in reg_errors[0].message

    def test_1_register_passes(self) -> None:
        """A single register (v0) is well within the limit."""
        prog = self._make_prog_with_regs(1)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_duplicate_register_indices_count_once(self) -> None:
        """The same register used multiple times is counted only once."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=g.next()
            ),
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)], id=g.next()
            ),
            IrInstruction(
                IrOp.ADD, [IrRegister(2), IrRegister(2), IrRegister(2)], id=g.next()
            ),
        )
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_no_registers_passes(self) -> None:
        """A program with no register operands (HALT only) passes."""
        g = gen_id()
        prog = make_program(IrInstruction(IrOp.HALT, [], id=g.next()))
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_5_registers_passes(self) -> None:
        """5 distinct registers (v0–v4) is within the limit."""
        prog = self._make_prog_with_regs(5)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []

    def test_error_message_mentions_count(self) -> None:
        """The register_count error message should mention the actual count."""
        prog = self._make_prog_with_regs(10)
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert len(reg_errors) == 1
        assert "10" in reg_errors[0].message

    def test_registers_in_different_operand_positions(self) -> None:
        """Registers appearing in any operand position are all counted."""
        g = gen_id()
        prog = make_program(
            # v0 + v1 → v2
            IrInstruction(
                IrOp.ADD, [IrRegister(2), IrRegister(0), IrRegister(1)], id=g.next()
            ),
            # v3 in second operand
            IrInstruction(
                IrOp.ADD, [IrRegister(2), IrRegister(3), IrRegister(1)], id=g.next()
            ),
        )
        # v0, v1, v2, v3 = 4 distinct registers — within limit
        errors = IrValidator().validate(prog)
        reg_errors = [e for e in errors if e.rule == "register_count"]
        assert reg_errors == []


# ---------------------------------------------------------------------------
# Rule 5: imm_range
# ---------------------------------------------------------------------------


class TestImmRange:
    """Rule: LOAD_IMM and ADD_IMM immediates must be in [0, 255]."""

    # --- LOAD_IMM tests ---

    def test_load_imm_0_passes(self) -> None:
        """0 is the minimum valid value for LOAD_IMM."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert imm_errors == []

    def test_load_imm_255_passes(self) -> None:
        """255 is the maximum valid value for LOAD_IMM."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(255)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert imm_errors == []

    def test_load_imm_256_fails(self) -> None:
        """256 exceeds the 8-bit range for LOAD_IMM."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(256)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert len(imm_errors) == 1
        assert "256" in imm_errors[0].message

    def test_load_imm_negative_fails(self) -> None:
        """Negative values are out of range for LOAD_IMM."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(-1)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert len(imm_errors) == 1
        assert "-1" in imm_errors[0].message

    # --- ADD_IMM tests ---

    def test_add_imm_0_passes(self) -> None:
        """0 is valid for ADD_IMM (used for register copy: ADI 0)."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.ADD_IMM,
                [IrRegister(2), IrRegister(1), IrImmediate(0)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert imm_errors == []

    def test_add_imm_255_passes(self) -> None:
        """255 is the maximum valid value for ADD_IMM."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.ADD_IMM,
                [IrRegister(2), IrRegister(1), IrImmediate(255)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert imm_errors == []

    def test_add_imm_256_fails(self) -> None:
        """256 exceeds the 8-bit range for ADD_IMM."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.ADD_IMM,
                [IrRegister(2), IrRegister(1), IrImmediate(256)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert len(imm_errors) == 1
        assert "ADD_IMM" in imm_errors[0].message

    def test_add_imm_negative_fails(self) -> None:
        """Negative ADD_IMM values are out of range."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.ADD_IMM,
                [IrRegister(2), IrRegister(1), IrImmediate(-5)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert len(imm_errors) == 1

    # --- Multiple violations ---

    def test_multiple_out_of_range_all_reported(self) -> None:
        """Multiple out-of-range immediates should all produce errors."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(300)], id=g.next()
            ),
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(500)], id=g.next()
            ),
            IrInstruction(
                IrOp.ADD_IMM,
                [IrRegister(4), IrRegister(2), IrImmediate(1000)],
                id=g.next(),
            ),
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert len(imm_errors) == 3

    def test_sub_not_checked_for_imm_range(self) -> None:
        """SUB does not have an immediate operand — imm_range does not apply."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.SUB,
                [IrRegister(2), IrRegister(3), IrRegister(4)],
                id=g.next(),
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert imm_errors == []

    def test_load_imm_128_passes(self) -> None:
        """128 is a common 8-bit value — should pass."""
        g = gen_id()
        prog = make_program(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(128)], id=g.next()
            )
        )
        errors = IrValidator().validate(prog)
        imm_errors = [e for e in errors if e.rule == "imm_range"]
        assert imm_errors == []


# ---------------------------------------------------------------------------
# Rule 6: syscall_whitelist
# ---------------------------------------------------------------------------


class TestSyscallWhitelist:
    """Rule: SYSCALL numbers must be in the 8008 hardware whitelist."""

    def _syscall_prog(self, num: int) -> IrProgram:
        """Build a program containing a single SYSCALL instruction."""
        g = IDGenerator()
        return make_program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(num)], id=g.next())
        )

    # --- Valid syscall numbers ---

    def test_syscall_3_passes(self) -> None:
        """SYSCALL 3 (adc) is valid on the 8008."""
        errors = IrValidator().validate(self._syscall_prog(3))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_4_passes(self) -> None:
        """SYSCALL 4 (sbb) is valid on the 8008."""
        errors = IrValidator().validate(self._syscall_prog(4))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_11_passes(self) -> None:
        """SYSCALL 11 (rlc) is valid."""
        errors = IrValidator().validate(self._syscall_prog(11))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_12_passes(self) -> None:
        """SYSCALL 12 (rrc) is valid."""
        errors = IrValidator().validate(self._syscall_prog(12))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_13_passes(self) -> None:
        """SYSCALL 13 (ral) is valid."""
        errors = IrValidator().validate(self._syscall_prog(13))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_14_passes(self) -> None:
        """SYSCALL 14 (rar) is valid."""
        errors = IrValidator().validate(self._syscall_prog(14))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_15_passes(self) -> None:
        """SYSCALL 15 (carry) is valid."""
        errors = IrValidator().validate(self._syscall_prog(15))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_syscall_16_passes(self) -> None:
        """SYSCALL 16 (parity) is valid."""
        errors = IrValidator().validate(self._syscall_prog(16))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []

    def test_all_in_ports_pass(self) -> None:
        """SYSCALL 20–27 (in ports 0–7) are all valid."""
        for num in range(20, 28):
            errors = IrValidator().validate(self._syscall_prog(num))
            sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
            assert sc_errors == [], f"SYSCALL {num} should pass"

    def test_all_out_ports_pass(self) -> None:
        """SYSCALL 40–63 (out ports 0–23) are all valid."""
        for num in range(40, 64):
            errors = IrValidator().validate(self._syscall_prog(num))
            sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
            assert sc_errors == [], f"SYSCALL {num} should pass"

    # --- Invalid syscall numbers ---

    def test_syscall_0_rejected(self) -> None:
        """SYSCALL 0 is not a valid 8008 intrinsic."""
        errors = IrValidator().validate(self._syscall_prog(0))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_1_rejected(self) -> None:
        """SYSCALL 1 has no 8008 hardware mapping."""
        errors = IrValidator().validate(self._syscall_prog(1))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_2_rejected(self) -> None:
        """SYSCALL 2 is not in the whitelist."""
        errors = IrValidator().validate(self._syscall_prog(2))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_5_rejected(self) -> None:
        """SYSCALL 5 (between adc/sbb and rotations) is not a valid 8008 intrinsic."""
        errors = IrValidator().validate(self._syscall_prog(5))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_17_rejected(self) -> None:
        """SYSCALL 17 (gap between parity and in-ports) is not valid."""
        errors = IrValidator().validate(self._syscall_prog(17))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_28_rejected(self) -> None:
        """SYSCALL 28 (in port 8 — above the 8 input-port limit) is invalid."""
        errors = IrValidator().validate(self._syscall_prog(28))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_39_rejected(self) -> None:
        """SYSCALL 39 (gap before out-ports) is not valid."""
        errors = IrValidator().validate(self._syscall_prog(39))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_64_rejected(self) -> None:
        """SYSCALL 64 (out port 24 — above the 24 output-port limit) is invalid."""
        errors = IrValidator().validate(self._syscall_prog(64))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_syscall_99_rejected(self) -> None:
        """SYSCALL 99 is not a valid 8008 intrinsic."""
        errors = IrValidator().validate(self._syscall_prog(99))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_error_message_mentions_syscall_number(self) -> None:
        """The syscall_whitelist error message should mention the bad SYSCALL number."""
        errors = IrValidator().validate(self._syscall_prog(99))
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert "99" in sc_errors[0].message

    def test_repeated_invalid_syscall_reported_once(self) -> None:
        """The same invalid SYSCALL number appearing twice is reported only once."""
        g = IDGenerator()
        prog = make_program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=g.next()),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=g.next()),
        )
        errors = IrValidator().validate(prog)
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 1

    def test_two_different_invalid_syscalls_both_reported(self) -> None:
        """Two different invalid SYSCALL numbers each produce their own error."""
        g = IDGenerator()
        prog = make_program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(0)], id=g.next()),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=g.next()),
        )
        errors = IrValidator().validate(prog)
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert len(sc_errors) == 2

    def test_no_syscalls_passes(self) -> None:
        """A program with no SYSCALL instructions trivially passes."""
        g = gen_id()
        prog = make_program(IrInstruction(IrOp.HALT, [], id=g.next()))
        errors = IrValidator().validate(prog)
        sc_errors = [e for e in errors if e.rule == "syscall_whitelist"]
        assert sc_errors == []


# ---------------------------------------------------------------------------
# Error accumulation
# ---------------------------------------------------------------------------


class TestErrorAccumulation:
    """Verify multiple rules can fail simultaneously and all errors are reported."""

    def test_word_ops_and_static_ram_both_reported(self) -> None:
        """A program with LOAD_WORD and too much RAM should produce both errors."""
        g = gen_id()
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="huge", size=10000, init=0))
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

    def test_all_six_rules_can_fail_at_once(self) -> None:
        """A maximally broken program should trigger all six validation rules."""
        # Construct a pathological program that violates every rule:
        #  1. no_word_ops     — include LOAD_WORD and STORE_WORD
        #  2. static_ram      — declare 10 000 bytes of static data
        #  3. call_depth      — build a call chain of depth 8
        #  4. register_count  — use virtual registers v0–v7 (8 distinct)
        #  5. imm_range       — LOAD_IMM with value 300
        #  6. syscall_whitelist — SYSCALL 99
        g = IDGenerator()
        prog = IrProgram(entry_label="_start")

        # --- Rule 2: static_ram ---
        prog.add_data(IrDataDecl(label="big", size=10000, init=0))

        # --- Rule 1: no_word_ops ---
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

        # --- Rule 3: call_depth (depth 8) ---
        chain = ["a", "b", "c", "d", "e", "f", "g", "h", "i"]  # 8 edges
        for idx, name in enumerate(chain):
            prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel(name)], id=-1))
            if idx + 1 < len(chain):
                prog.add_instruction(
                    IrInstruction(IrOp.CALL, [IrLabel(chain[idx + 1])], id=g.next())
                )
            prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))

        # --- Rule 4: register_count (8 distinct: v0–v7) ---
        for reg_idx in range(8):
            prog.add_instruction(
                IrInstruction(
                    IrOp.LOAD_IMM,
                    [IrRegister(reg_idx), IrImmediate(0)],
                    id=g.next(),
                )
            )

        # --- Rule 5: imm_range ---
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(300)], id=g.next()
            )
        )

        # --- Rule 6: syscall_whitelist ---
        prog.add_instruction(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=g.next())
        )

        errors = IrValidator().validate(prog)
        rules = {e.rule for e in errors}
        assert "no_word_ops" in rules
        assert "static_ram" in rules
        assert "call_depth" in rules
        assert "register_count" in rules
        assert "imm_range" in rules
        assert "syscall_whitelist" in rules

    def test_clean_program_has_no_errors(self) -> None:
        """A well-formed program with no violations should have zero errors."""
        g = IDGenerator()
        prog = IrProgram(entry_label="_start")

        # 1 static byte
        prog.add_data(IrDataDecl(label="x", size=1, init=0))

        # Simple entry stub
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0)], id=g.next()
            )
        )
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("_fn_main")], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))

        # Simple main function using 2 locals and a valid syscall
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_main")], id=-1))
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(100)], id=g.next()
            )
        )
        prog.add_instruction(
            IrInstruction(
                IrOp.ADD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )
        # SYSCALL 20 = in(0)
        prog.add_instruction(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(20)], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))

        errors = IrValidator().validate(prog)
        assert errors == []


# ---------------------------------------------------------------------------
# IrValidationError type tests
# ---------------------------------------------------------------------------


class TestIrValidationError:
    """Tests for the IrValidationError class itself."""

    def test_str_representation(self) -> None:
        """__str__ should format as '[rule] message'."""
        e = IrValidationError(rule="static_ram", message="too many statics")
        assert str(e) == "[static_ram] too many statics"

    def test_is_exception_subclass(self) -> None:
        """IrValidationError must be an Exception so the backend can raise it."""
        e = IrValidationError(rule="no_word_ops", message="forbidden")
        assert isinstance(e, Exception)

    def test_can_be_raised(self) -> None:
        """IrValidationError should be raise-able like any Exception."""
        with pytest.raises(IrValidationError):
            raise IrValidationError(rule="syscall_whitelist", message="bad syscall")

    def test_equality(self) -> None:
        """Two IrValidationError objects with same rule+message are equal."""
        a = IrValidationError(rule="imm_range", message="value 300 out of range")
        b = IrValidationError(rule="imm_range", message="value 300 out of range")
        assert a == b

    def test_inequality_by_rule(self) -> None:
        """Different rules make errors unequal even with same message."""
        a = IrValidationError(rule="imm_range", message="value 300")
        b = IrValidationError(rule="syscall_whitelist", message="value 300")
        assert a != b

    def test_inequality_by_message(self) -> None:
        """Same rule but different message means errors are unequal."""
        a = IrValidationError(rule="call_depth", message="depth 8")
        b = IrValidationError(rule="call_depth", message="depth 9")
        assert a != b

    def test_hash_equal_objects_have_equal_hash(self) -> None:
        """Equal IrValidationError objects must have the same hash."""
        a = IrValidationError(rule="register_count", message="too many regs")
        b = IrValidationError(rule="register_count", message="too many regs")
        assert hash(a) == hash(b)

    def test_rule_attribute(self) -> None:
        """The rule attribute should be readable."""
        e = IrValidationError(rule="call_depth", message="too deep")
        assert e.rule == "call_depth"

    def test_message_attribute(self) -> None:
        """The message attribute should be readable."""
        e = IrValidationError(rule="call_depth", message="too deep")
        assert e.message == "too deep"

    def test_equality_with_non_error(self) -> None:
        """Comparing with a non-IrValidationError returns NotImplemented."""
        e = IrValidationError(rule="static_ram", message="big")
        result = e.__eq__("not an error")
        assert result is NotImplemented
