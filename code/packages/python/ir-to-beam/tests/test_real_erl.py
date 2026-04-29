"""End-to-end smoke tests: lower an IR program, encode it, ask
real ``erl`` to load it and call the entry point.

These tests skip cleanly when ``erl`` is not on PATH, but locally
they prove the entire BEAM01 Phase 2+3 pipeline works.

Why the call-back-into-erl pattern?
====================================

We can't ``-s mod fn`` an ``erl`` script because that requires the
function to take its arguments as a *list of atoms* (Erlang's
``-s`` calling convention).  Instead, the test's ``-eval`` payload
does:

  code:load_file(M),       %% load the .beam from -pa <dir>
  V = M:fn(),              %% call the entry directly
  io:format("~p~n", [V]),  %% print the return value
  init:stop().

Then the test parses the printed value and asserts on it.
"""

from __future__ import annotations

import shutil
import subprocess
import textwrap
from pathlib import Path

import pytest
from beam_bytecode_encoder import encode_beam
from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from ir_to_beam import BEAMBackendConfig, lower_ir_to_beam


def _has_erl() -> bool:
    return shutil.which("erl") is not None


requires_erl = pytest.mark.skipif(not _has_erl(), reason="erl not on PATH")


def _label(name: str) -> IrLabel:
    return IrLabel(name=name)


def _reg(i: int) -> IrRegister:
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _drop_and_run(
    tmp_path: Path,
    *,
    module: str,
    entry: str,
    program: IrProgram,
) -> subprocess.CompletedProcess[str]:
    """Helper: lower → encode → write to disk → ``erl -eval``."""
    config = BEAMBackendConfig(module_name=module)
    beam = encode_beam(lower_ir_to_beam(program, config))
    (tmp_path / f"{module}.beam").write_bytes(beam)

    eval_expr = textwrap.dedent(f"""
        {{module, _}} = code:load_file({module}),
        Result = {module}:{entry}(),
        io:format("~p~n", [Result]),
        init:stop().
    """).strip()
    return subprocess.run(
        ["erl", "-noshell", "-pa", str(tmp_path), "-eval", eval_expr],
        capture_output=True,
        text=True,
        timeout=15,
    )


@requires_erl
def test_identity_returns_42(tmp_path: Path) -> None:
    """``identity()`` returns the constant 42."""
    gen = IDGenerator()
    program = IrProgram(entry_label="identity")
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("identity")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(42)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    result = _drop_and_run(
        tmp_path, module="answer42", entry="identity", program=program
    )
    assert result.returncode == 0, (
        f"erl rejected the module:\n  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout.strip() == "42"


@requires_erl
def test_add_returns_42(tmp_path: Path) -> None:
    """``add()`` returns ``17 + 25 = 42`` via gc_bif2."""
    gen = IDGenerator()
    program = IrProgram(entry_label="add")
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("add")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(17)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(25)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD, [_reg(0), _reg(0), _reg(1)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    result = _drop_and_run(
        tmp_path, module="adder42", entry="add", program=program
    )
    assert result.returncode == 0, (
        f"erl rejected the module:\n  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout.strip() == "42"


@requires_erl
def test_multiplication_returns_42(tmp_path: Path) -> None:
    """``mul()`` returns ``6 * 7 = 42``."""
    gen = IDGenerator()
    program = IrProgram(entry_label="mul")
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("mul")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(6)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(7)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.MUL, [_reg(0), _reg(0), _reg(1)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    result = _drop_and_run(
        tmp_path, module="muller42", entry="mul", program=program
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "42"
