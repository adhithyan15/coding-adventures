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
    """``identity()`` returns the constant 42.

    TW03 Phase 1 convention: the result lands in ``r1``
    (``_REG_HALT_RESULT``) before ``RET``.
    """
    gen = IDGenerator()
    program = IrProgram(entry_label="identity")
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("identity")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(42)], id=gen.next())
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
        IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(17)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(25)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD, [_reg(1), _reg(2), _reg(3)], id=gen.next())
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
        IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(6)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(7)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.MUL, [_reg(1), _reg(2), _reg(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    result = _drop_and_run(
        tmp_path, module="muller42", entry="mul", program=program
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "42"


@requires_erl
def test_recursive_factorial_returns_120(tmp_path: Path) -> None:
    """The headline TW03 Phase 1 BEAM test: ``fact(5) → 120`` on real ``erl``.

    Hand-built IR mirroring what twig-beam-compiler will eventually
    emit, exercising:
    - allocate / deallocate y-register frame
    - x-register arg passing at CALL sites
    - BRANCH_Z + JUMP for the if/else of the base case
    - CMP_EQ + arithmetic across recursive call boundary
    """
    gen = IDGenerator()
    program = IrProgram(entry_label="main")

    # fact(n):
    #   if n == 0 -> return 1
    #   else      -> return n * fact(n - 1)
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("fact")], id=-1))
    # r2 = arg n (placed by entry-shuffle).  Compare with 0 → r10.
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(11), _imm(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.CMP_EQ, [_reg(10), _reg(2), _reg(11)], id=gen.next()
        )
    )
    # if (r10 == 0): jump to else_label  (BRANCH_Z r10 → jump if zero)
    program.add_instruction(
        IrInstruction(
            IrOp.BRANCH_Z, [_reg(10), _label("_else")], id=gen.next()
        )
    )
    # then-branch: r1 = 1; jump end
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(1)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.JUMP, [_label("_end")], id=gen.next())
    )
    # else-branch: compute n * fact(n-1)
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("_else")], id=-1))
    # r12 = n - 1
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(13), _imm(1)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.SUB, [_reg(12), _reg(2), _reg(13)], id=gen.next())
    )
    # Stage arg in r2 for the recursive call (Twig calling convention).
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [_reg(14), _reg(2), _imm(0)], id=gen.next())
    )  # save n
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [_reg(2), _reg(12), _imm(0)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.CALL, [_label("fact")], id=gen.next()))
    # Result of fact(n-1) is in r1.  r1 = saved_n * r1.
    program.add_instruction(
        IrInstruction(IrOp.MUL, [_reg(1), _reg(14), _reg(1)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("_end")], id=-1))
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    # main(): r2 = 5; call fact; result lands in r1.
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(5)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.CALL, [_label("fact")], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    config = BEAMBackendConfig(
        module_name="rec_fact",
        arity_overrides={"fact": 1, "main": 0},
    )
    beam = encode_beam(lower_ir_to_beam(program, config))
    (tmp_path / "rec_fact.beam").write_bytes(beam)
    eval_expr = textwrap.dedent("""
        {module, _} = code:load_file(rec_fact),
        Result = rec_fact:main(),
        io:format("~p~n", [Result]),
        init:stop().
    """).strip()
    result = subprocess.run(
        ["erl", "-noshell", "-pa", str(tmp_path), "-eval", eval_expr],
        capture_output=True,
        text=True,
        timeout=15,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "120", (
        f"factorial result mismatch — stdout={result.stdout!r}, "
        f"stderr={result.stderr!r}"
    )


@requires_erl
def test_closure_make_adder_returns_42(tmp_path: Path) -> None:
    """The headline BEAM02 Phase 2 test: ``((make-adder 7) 35) → 42``.

    Hand-built IR mirroring what twig-beam-compiler will emit for
    ``(define (make-adder n) (lambda (x) (+ x n))) ((make-adder 7) 35)``,
    exercising the full closure pipeline end-to-end on real ``erl``:

    - ``MAKE_CLOSURE`` lowering → cons-cell ``[FnAtom | Caps]``
    - ``APPLY_CLOSURE`` lowering → ``erlang:'++'/2`` + ``erlang:apply/3``
    - Captures-first register layout in the lifted lambda body
    """
    gen = IDGenerator()
    program = IrProgram(entry_label="main")

    # _lambda_0(N, X) — exported with full arity 2 (1 capture + 1 explicit).
    # Captures-first layout: y2 = N (captured), y3 = X (explicit).
    # Body: r1 = N + X.
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [_label("_lambda_0")], id=-1)
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD, [_reg(1), _reg(2), _reg(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    # make_adder(n).  Body: r1 = MAKE_CLOSURE(_lambda_0, [n]).
    # Layout: y2 = arg n, y10 = closure ref.
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [_label("make_adder")], id=-1)
    )
    program.add_instruction(
        IrInstruction(
            IrOp.MAKE_CLOSURE,
            [_reg(1), _label("_lambda_0"), _imm(1), _reg(2)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    # main().  Body: r1 = APPLY_CLOSURE(make_adder(7), [35]).
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
    # call make_adder(7) — stage arg in r2
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(7)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.CALL, [_label("make_adder")], id=gen.next())
    )
    # The closure ref is in r1 after CALL.  Save it in r10 so the
    # arg-staging dance (r2 = 35) doesn't clobber it.
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [_reg(10), _reg(1), _imm(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(11), _imm(35)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.APPLY_CLOSURE,
            [_reg(1), _reg(10), _imm(1), _reg(11)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    # arity_overrides for _lambda_0 declares the EXPLICIT arity (1 — the
    # ``(lambda (x) ...)`` parameter). The backend widens it to 2 (1
    # capture + 1 explicit) for func_info / exports / apply.
    config = BEAMBackendConfig(
        module_name="closure_adder",
        arity_overrides={"_lambda_0": 1, "make_adder": 1, "main": 0},
        closure_free_var_counts={"_lambda_0": 1},
    )
    beam = encode_beam(lower_ir_to_beam(program, config))
    (tmp_path / "closure_adder.beam").write_bytes(beam)
    eval_expr = textwrap.dedent("""
        {module, _} = code:load_file(closure_adder),
        Result = closure_adder:main(),
        io:format("~p~n", [Result]),
        init:stop().
    """).strip()
    result = subprocess.run(
        ["erl", "-noshell", "-pa", str(tmp_path), "-eval", eval_expr],
        capture_output=True,
        text=True,
        timeout=15,
    )
    assert result.returncode == 0, (
        f"erl rejected the closure module:\n  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout.strip() == "42", (
        f"closure result mismatch — stdout={result.stdout!r}, "
        f"stderr={result.stderr!r}"
    )
