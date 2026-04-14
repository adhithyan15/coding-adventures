"""Integration tests for Intel4004Backend — complete pipeline tests.

These tests verify that the backend's compile() method:
  1. Runs validation first
  2. Raises IrValidationError on constraint violations
  3. Produces correct assembly for valid programs
  4. Handles a variety of complete programs end-to-end

We also test the module-level convenience functions: validate() and generate_asm().
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

from intel_4004_backend import (
    Intel4004Backend,
    IrValidationError,
    generate_asm,
    validate,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_simple_prog() -> IrProgram:
    """Build a minimal 'hello world equivalent': load 5 into R2, then halt."""
    g = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(
            IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=g.next()
        )
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))
    return prog


def make_loop_prog() -> IrProgram:
    """Build a countdown loop: R2 = 10; while R2 != 0: R2 -= 1."""
    g = IDGenerator()
    prog = IrProgram(entry_label="_start")

    # R2 = 10
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(
            IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(10)], id=g.next()
        )
    )

    # loop_top: branch to done if R2 == 0
    prog.add_instruction(
        IrInstruction(IrOp.LABEL, [IrLabel("loop_top")], id=-1)
    )
    prog.add_instruction(
        IrInstruction(
            IrOp.BRANCH_Z, [IrRegister(2), IrLabel("done")], id=g.next()
        )
    )

    # R2 -= 1 (load 1 into R3 and subtract)
    prog.add_instruction(
        IrInstruction(
            IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(1)], id=g.next()
        )
    )
    prog.add_instruction(
        IrInstruction(
            IrOp.SUB,
            [IrRegister(2), IrRegister(2), IrRegister(3)],
            id=g.next(),
        )
    )
    prog.add_instruction(
        IrInstruction(IrOp.JUMP, [IrLabel("loop_top")], id=g.next())
    )

    # done:
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("done")], id=-1))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))
    return prog


def make_data_prog() -> IrProgram:
    """Build a program that declares static data and uses it."""
    g = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_data(IrDataDecl(label="scratch", size=4, init=0))

    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(
            IrOp.LOAD_ADDR, [IrRegister(4), IrLabel("scratch")], id=g.next()
        )
    )
    prog.add_instruction(
        IrInstruction(
            IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(42)], id=g.next()
        )
    )
    prog.add_instruction(
        IrInstruction(
            IrOp.STORE_BYTE,
            [IrRegister(2), IrRegister(4), IrRegister(0)],
            id=g.next(),
        )
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))
    return prog


def make_invalid_prog() -> IrProgram:
    """Build a program that violates the no_word_ops rule."""
    g = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(
            IrOp.LOAD_WORD,
            [IrRegister(2), IrRegister(0), IrRegister(1)],
            id=g.next(),
        )
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))
    return prog


# ---------------------------------------------------------------------------
# Intel4004Backend.compile() — happy path
# ---------------------------------------------------------------------------


class TestBackendHappyPath:
    """Valid programs should produce well-formed assembly."""

    def test_minimal_program_produces_asm(self) -> None:
        """The simplest valid program should produce non-empty assembly."""
        backend = Intel4004Backend()
        asm = backend.compile(make_simple_prog())
        assert isinstance(asm, str)
        assert len(asm) > 0

    def test_minimal_program_starts_with_org(self) -> None:
        """Output must start with ORG 0x000."""
        backend = Intel4004Backend()
        asm = backend.compile(make_simple_prog())
        assert asm.startswith("    ORG 0x000")

    def test_minimal_program_contains_ldm_and_halt(self) -> None:
        """The 'load 5 + halt' program should contain LDM 5 and HLT."""
        backend = Intel4004Backend()
        asm = backend.compile(make_simple_prog())
        assert "LDM 5" in asm
        assert "HLT" in asm

    def test_minimal_program_contains_start_label(self) -> None:
        """The _start label should appear in the output."""
        backend = Intel4004Backend()
        asm = backend.compile(make_simple_prog())
        assert "_start:" in asm

    def test_loop_program_produces_asm(self) -> None:
        """A loop program should produce assembly with JCN and JUN."""
        backend = Intel4004Backend()
        asm = backend.compile(make_loop_prog())
        assert "JCN" in asm
        assert "JUN loop_top" in asm

    def test_data_program_produces_asm(self) -> None:
        """A program with data declarations should produce assembly with FIM and WRM."""
        backend = Intel4004Backend()
        asm = backend.compile(make_data_prog())
        assert "FIM" in asm
        assert "WRM" in asm

    def test_output_ends_with_newline(self) -> None:
        """Generated assembly should end with a newline."""
        backend = Intel4004Backend()
        asm = backend.compile(make_simple_prog())
        assert asm.endswith("\n")


# ---------------------------------------------------------------------------
# Intel4004Backend.compile() — validation error propagation
# ---------------------------------------------------------------------------


class TestBackendValidationErrors:
    """Invalid programs must raise IrValidationError, never silently produce asm."""

    def test_load_word_raises_validation_error(self) -> None:
        """A program with LOAD_WORD should raise IrValidationError."""
        backend = Intel4004Backend()
        with pytest.raises(IrValidationError):
            backend.compile(make_invalid_prog())

    def test_validation_error_message_is_informative(self) -> None:
        """The raised error's message should mention the violated rule."""
        backend = Intel4004Backend()
        with pytest.raises(IrValidationError) as exc_info:
            backend.compile(make_invalid_prog())
        msg = exc_info.value.message
        assert "no_word_ops" in exc_info.value.rule or "no_word_ops" in msg

    def test_ram_overflow_raises_error(self) -> None:
        """A program with too much static RAM raises IrValidationError."""
        backend = Intel4004Backend()
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="big", size=300, init=0))
        prog.add_instruction(IrInstruction(IrOp.HALT, [], id=0))
        with pytest.raises(IrValidationError):
            backend.compile(prog)

    def test_multiple_errors_combined_in_one_exception(self) -> None:
        """Multiple constraint violations should be combined in a single exception."""
        backend = Intel4004Backend()
        g = IDGenerator()
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl(label="big", size=300, init=0))  # static_ram
        prog.add_instruction(
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
                id=g.next(),
            )
        )  # no_word_ops
        with pytest.raises(IrValidationError) as exc_info:
            backend.compile(prog)
        msg = exc_info.value.message
        assert "no_word_ops" in msg or "LOAD_WORD" in msg
        assert "static_ram" in msg or "160" in msg


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------


class TestModuleFunctions:
    """The module-level validate() and generate_asm() functions."""

    def test_validate_returns_empty_for_valid_program(self) -> None:
        """validate() on a clean program returns an empty list."""
        errors = validate(make_simple_prog())
        assert errors == []

    def test_validate_returns_errors_for_invalid_program(self) -> None:
        """validate() on a bad program returns at least one error."""
        errors = validate(make_invalid_prog())
        assert len(errors) > 0
        assert all(isinstance(e, IrValidationError) for e in errors)

    def test_generate_asm_produces_string(self) -> None:
        """generate_asm() on a valid program returns a non-empty string."""
        asm = generate_asm(make_simple_prog())
        assert isinstance(asm, str)
        assert len(asm) > 0

    def test_generate_asm_contains_org(self) -> None:
        """generate_asm() output starts with ORG 0x000."""
        asm = generate_asm(make_simple_prog())
        assert "ORG 0x000" in asm


# ---------------------------------------------------------------------------
# Exact assembly snapshot tests
# ---------------------------------------------------------------------------


class TestExactAssembly:
    """Known programs should produce exact assembly output.

    These tests lock down the exact output format so regressions are
    immediately visible.
    """

    def test_load_and_halt_exact_output(self) -> None:
        """Minimal program: LABEL _start, LOAD_IMM v2 5, HALT.

        Expected output::

                ORG 0x000
            _start:
                LDM 5
                XCH R2
                JUN $
        """
        backend = Intel4004Backend()
        asm = backend.compile(make_simple_prog())
        lines = [ln for ln in asm.splitlines() if ln.strip()]
        assert lines[0] == "    ORG 0x000"
        assert lines[1] == "_start:"
        assert lines[2] == "    LDM 5"
        assert lines[3] == "    XCH R2"
        assert lines[4] == "    HLT"

    def test_loop_program_has_correct_structure(self) -> None:
        """The countdown loop program should have the expected structural elements."""
        backend = Intel4004Backend()
        asm = backend.compile(make_loop_prog())
        assert "_start:" in asm
        assert "loop_top:" in asm
        assert "done:" in asm
        assert "LDM 10" in asm
        assert "JCN 0x4, done" in asm
        assert "JUN loop_top" in asm

    def test_syscall_program_does_not_crash(self) -> None:
        """A program using SYSCALL should not raise during codegen."""
        g = IDGenerator()
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        prog.add_instruction(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(1)], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))
        backend = Intel4004Backend()
        asm = backend.compile(prog)
        assert "syscall" in asm.lower() or "WRITE" in asm

    def test_subroutine_call_and_return(self) -> None:
        """A program with CALL+RET should produce JMS and BBL 0."""
        g = IDGenerator()
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        prog.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("my_sub")], id=g.next())
        )
        prog.add_instruction(IrInstruction(IrOp.HALT, [], id=g.next()))
        prog.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("my_sub")], id=-1)
        )
        prog.add_instruction(IrInstruction(IrOp.RET, [], id=g.next()))
        backend = Intel4004Backend()
        asm = backend.compile(prog)
        assert "JMS my_sub" in asm
        assert "BBL 0" in asm
