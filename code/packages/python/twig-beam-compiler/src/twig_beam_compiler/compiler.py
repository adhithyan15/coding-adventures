"""Twig source → ``.beam`` end-to-end compiler.

Pipeline
========
::

    Twig source
        ↓ parse + extract     (twig package)
    typed AST
        ↓ Compiler.compile()  (this module)
    IrProgram
        ↓ ir-optimizer
    IrProgram (optimised)
        ↓ lower_ir_to_beam
    BEAMModule
        ↓ encode_beam
    .beam bytes
        ↓ erl -noshell -eval ...
    program output

Why this is intentionally narrow
================================

The v1 surface is *just* what ``ir-to-beam`` Phase 3 supports plus
the boilerplate to wrap an expression as a ``main/0`` function.
That means: integer literals, binary arithmetic (+, -, *, /),
single-binding ``let``, and a ``begin`` for sequencing.  Top-level
``define`` and ``if`` need branch lowering in ``ir-to-beam`` (a
follow-up PR).

The contract: ``main/0`` returns the value of the program's last
top-level expression.  Real ``erl`` then prints that value via
``-eval 'io:format("~p~n", [<module>:main()])'``, which the
``run_source`` helper does for us.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Final

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
from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_beam import BEAMBackendConfig, lower_ir_to_beam
from twig import (
    TwigCompileError,
    TwigError,
    extract_program,
    parse_twig,
)
from twig.ast_nodes import (
    Apply,
    Begin,
    Define,
    Expr,
    IntLit,
    Lambda,
    Let,
    Program,
    SymLit,
    VarRef,
)


# ---------------------------------------------------------------------------
# Result types
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class BeamPackageResult:
    """Aggregated artefacts from one compile run."""

    source: str
    module_name: str
    ast: Program
    raw_ir: IrProgram
    optimization: OptimizationResult
    optimized_ir: IrProgram
    beam_bytes: bytes


@dataclass(frozen=True)
class BeamRunResult:
    """Compilation result + real-``erl`` invocation output."""

    compilation: BeamPackageResult
    stdout: bytes
    stderr: bytes
    returncode: int


class BeamPackageError(TwigError):
    """Stage-tagged failure during compile or run."""

    def __init__(
        self,
        stage: str,
        message: str,
        cause: Exception | None = None,
    ) -> None:
        super().__init__(f"[{stage}] {message}")
        self.stage = stage
        self.message = message
        self.cause = cause


# ---------------------------------------------------------------------------
# Builtins recognised in v1
# ---------------------------------------------------------------------------


# Map a Twig builtin name to the ``IrOp`` for its single-instruction
# emission.  Only binary numeric ops are in v1.
_BINARY_OPS: dict[str, IrOp] = {
    "+": IrOp.ADD,
    "-": IrOp.SUB,
    "*": IrOp.MUL,
    "/": IrOp.DIV,
}

# Twig builtins we don't yet support on BEAM.  Everything else
# falls through to the generic "unsupported" error.
_V1_REJECTED_BUILTINS: frozenset[str] = frozenset(
    {
        # Comparison/logic — needs branching.
        "=",
        "<",
        ">",
        # Cons/list/symbol — heap-side machinery, not yet wired.
        "cons",
        "car",
        "cdr",
        "null?",
        "pair?",
        "number?",
        "symbol?",
        "print",
    }
)


# Register convention.  We use only x-registers (compiler-IR
# ``IrRegister`` indices map directly to BEAM's ``x{i}``).  Reg 0
# is the ``main/0`` return value (matching BEAM's "x0 holds the
# return" convention).
_REG_RETURN: int = 0
_REG_HOLDING_BASE: int = 1

# A safe Erlang atom name — strict allowlist (lowercase ASCII
# start, alnum + underscore tail, length 1..64).  We use this to
# block code-injection through ``module_name``: ``run_source``
# interpolates ``module_name`` into the ``erl -eval`` Erlang source
# string, so an unvalidated value lets a caller execute arbitrary
# Erlang (e.g. ``os:cmd("...")``).  The same allowlist also
# protects the on-disk ``.beam`` filename from path traversal.
_SAFE_MODULE_NAME_RE: Final = re.compile(r"^[a-z][a-zA-Z0-9_]{0,63}$")


def _validate_module_name(name: str) -> None:
    if not _SAFE_MODULE_NAME_RE.match(name):
        msg = (
            f"module_name must match {_SAFE_MODULE_NAME_RE.pattern!r} "
            f"(lowercase-start atom, alnum + underscore, ≤64 chars); "
            f"got {name!r}.  Stricter than ECMA Erlang atom rules on "
            "purpose — keeps the Erlang ``-eval`` interpolation in "
            "``run_source`` injection-proof."
        )
        raise BeamPackageError("validate", msg)


# ---------------------------------------------------------------------------
# Per-region compilation context
# ---------------------------------------------------------------------------


@dataclass
class _FnCtx:
    """Mutable state during compilation of one IR region.

    ``locals_`` maps Twig names introduced by ``let`` bindings to
    the IR register that holds the bound value.  ``next_holding``
    is the next intermediate-value register to allocate.
    """

    locals_: dict[str, IrRegister]
    next_holding: int = _REG_HOLDING_BASE


# ---------------------------------------------------------------------------
# Compiler
# ---------------------------------------------------------------------------


class _Compiler:
    """Walk the typed Twig AST and emit a single ``IrProgram`` whose
    sole region is ``main``.

    v1 only handles top-level expressions (no ``define``).  The
    last top-level expression's value lands in ``x0`` (the BEAM
    return-value register) and the region closes with ``RET``.
    """

    def __init__(self) -> None:
        self._program = IrProgram(entry_label="main")
        self._gen = IDGenerator()

    # ------------------------------------------------------------------

    def compile(self, program: Program) -> IrProgram:
        # v1 forbids top-level ``define`` (function or value).
        # When we add ``define`` support we'll either inline value
        # defines like the JVM compiler does, or emit them as
        # additional BEAM functions.
        for form in program.forms:
            if isinstance(form, Define):
                raise TwigCompileError(
                    "(define ...) is not yet supported by the BEAM backend "
                    "v1 — only top-level expressions.  See BEAM01 Phase 4 "
                    "for the planned roadmap."
                )

        self._emit(IrOp.LABEL, IrLabel(name="main"), id=-1)
        ctx = _FnCtx(locals_={})

        last: IrRegister | None = None
        for expr in program.forms:
            last = self._compile_expr(expr, ctx)

        if last is None:
            # Empty program → return 0.
            zero = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
            last = zero

        self._emit_move(IrRegister(_REG_RETURN), last)
        self._emit(IrOp.RET)
        return self._program

    # ------------------------------------------------------------------
    # Expression compilation
    # ------------------------------------------------------------------

    def _compile_expr(self, expr: Expr, ctx: _FnCtx) -> IrRegister:
        if isinstance(expr, IntLit):
            reg = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, reg, IrImmediate(int(expr.value)))
            return reg

        if isinstance(expr, VarRef):
            try:
                return ctx.locals_[expr.name]
            except KeyError as exc:
                raise TwigCompileError(
                    f"unbound name: {expr.name!r}"
                ) from exc

        if isinstance(expr, Let):
            return self._compile_let(expr, ctx)

        if isinstance(expr, Begin):
            return self._compile_begin(expr, ctx)

        if isinstance(expr, Apply):
            return self._compile_apply(expr, ctx)

        if isinstance(expr, Lambda):
            raise TwigCompileError(
                "lambdas are not yet supported by the BEAM backend "
                "v1 — see BEAM01 Phase 4+ for closures."
            )

        if isinstance(expr, SymLit):
            raise TwigCompileError(
                "quoted symbols are not yet supported by the BEAM "
                "backend v1 — atoms-as-values needs heap support."
            )

        msg = f"unsupported Twig form for BEAM v1: {type(expr).__name__}"
        raise TwigCompileError(msg)

    def _compile_let(self, expr: Let, ctx: _FnCtx) -> IrRegister:
        # v1: single-binding ``let`` only — no shadowing concerns.
        if len(expr.bindings) != 1:
            raise TwigCompileError(
                "(let ((name expr) ...) body) with multiple bindings "
                "is not yet supported — v1 only handles a single binding"
            )
        name, value_expr = expr.bindings[0]
        bound_reg = self._compile_expr(value_expr, ctx)

        # Save and restore the binding so the let scope is correct.
        previous = ctx.locals_.get(name)
        ctx.locals_[name] = bound_reg
        try:
            last = self._compile_expr(expr.body[0], ctx)
            for body_expr in expr.body[1:]:
                last = self._compile_expr(body_expr, ctx)
            return last
        finally:
            if previous is None:
                del ctx.locals_[name]
            else:
                ctx.locals_[name] = previous

    def _compile_begin(self, expr: Begin, ctx: _FnCtx) -> IrRegister:
        last: IrRegister | None = None
        for sub in expr.exprs:
            last = self._compile_expr(sub, ctx)
        if last is None:
            zero = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
            return zero
        return last

    def _compile_apply(self, expr: Apply, ctx: _FnCtx) -> IrRegister:
        if not isinstance(expr.fn, VarRef):
            raise TwigCompileError(
                "v1 only supports calls to named builtins (no first-class "
                "function values yet)"
            )
        name = expr.fn.name
        if name in _V1_REJECTED_BUILTINS:
            raise TwigCompileError(
                f"builtin {name!r} is not yet supported by the BEAM "
                "backend v1 — needs branching / heap / I/O support"
            )
        if name not in _BINARY_OPS:
            raise TwigCompileError(
                f"unknown builtin: {name!r}"
            )
        if len(expr.args) != 2:
            raise TwigCompileError(
                f"{name!r} expects exactly 2 arguments, got {len(expr.args)}"
            )
        lhs = self._compile_expr(expr.args[0], ctx)
        rhs = self._compile_expr(expr.args[1], ctx)
        result = self._fresh_holding(ctx)
        self._emit(_BINARY_OPS[name], result, lhs, rhs)
        return result

    # ------------------------------------------------------------------
    # Low-level emit helpers
    # ------------------------------------------------------------------

    def _emit(self, op: IrOp, *operands: object, id: int | None = None) -> None:
        self._program.add_instruction(
            IrInstruction(
                op,
                list(operands),
                id=self._gen.next() if id is None else id,
            )
        )

    def _emit_move(self, dst: IrRegister, src: IrRegister) -> None:
        # Lower a "move" as an ADD_IMM 0 — the IR has no MOVE op
        # but ADD_IMM with immediate 0 is the canonical idiom in
        # the existing twig-jvm-compiler.  Here we go even simpler:
        # if dst == src already, no instruction needed.
        if dst.index == src.index:
            return
        # We don't have ADD_IMM in our ir-to-beam v1 op set, so use
        # a load-and-add-zero pattern:
        zero = self._fresh_holding_for_move()
        self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
        self._emit(IrOp.ADD, dst, src, zero)

    def _fresh_holding(self, ctx: _FnCtx) -> IrRegister:
        reg = IrRegister(index=ctx.next_holding)
        ctx.next_holding += 1
        return reg

    def _fresh_holding_for_move(self) -> IrRegister:
        # Use a high-numbered scratch register the rest of the
        # compiler doesn't allocate from.  Picking 1000 is arbitrary
        # but well above anything the tests will exercise.
        return IrRegister(index=1000)


# ---------------------------------------------------------------------------
# Public entry points
# ---------------------------------------------------------------------------


def compile_to_ir(source: str) -> IrProgram:
    """Compile Twig source into an unoptimised ``IrProgram``."""
    ast = parse_twig(source)
    program = extract_program(ast)
    return _Compiler().compile(program)


def compile_source(
    source: str,
    *,
    module_name: str = "twig_main",
    optimize: bool = True,
) -> BeamPackageResult:
    """Run the full Twig → ``.beam`` pipeline end-to-end."""
    _validate_module_name(module_name)
    try:
        ast = parse_twig(source)
        twig_program = extract_program(ast)
    except Exception as exc:
        raise BeamPackageError("parse", str(exc), exc) from exc

    try:
        raw_ir = _Compiler().compile(twig_program)
    except TwigCompileError:
        raise
    except Exception as exc:  # pragma: no cover
        raise BeamPackageError("ir-emit", str(exc), exc) from exc

    if optimize:
        optimization = IrOptimizer.default_passes().optimize(raw_ir)
        optimized_ir = optimization.program
    else:
        optimization = OptimizationResult(program=raw_ir)
        optimized_ir = raw_ir

    try:
        beam_module = lower_ir_to_beam(
            optimized_ir,
            BEAMBackendConfig(module_name=module_name),
        )
        beam_bytes = encode_beam(beam_module)
    except Exception as exc:
        raise BeamPackageError("lower-beam", str(exc), exc) from exc

    return BeamPackageResult(
        source=source,
        module_name=module_name,
        ast=twig_program,
        raw_ir=raw_ir,
        optimization=optimization,
        optimized_ir=optimized_ir,
        beam_bytes=beam_bytes,
    )


def erl_available() -> bool:
    """Return ``True`` iff a working ``erl`` binary is on PATH."""
    if shutil.which("erl") is None:
        return False
    try:
        result = subprocess.run(
            ["erl", "-noshell", "-eval", "init:stop()."],
            capture_output=True,
            timeout=5,
            check=False,
        )
        return result.returncode == 0
    except (subprocess.SubprocessError, OSError):
        return False


def run_source(
    source: str,
    *,
    module_name: str = "twig_main",
    optimize: bool = True,
    timeout_seconds: int = 30,
) -> BeamRunResult:
    """Compile and execute on the **real** ``erl`` runtime.

    Drops the ``.beam`` to a fresh temp dir and uses ``erl -eval``
    to call ``<module>:main()`` and ``io:format`` its return value
    to stdout, then ``init:stop()``.
    """
    compilation = compile_source(
        source, module_name=module_name, optimize=optimize
    )
    with tempfile.TemporaryDirectory() as tmp:
        beam_path = Path(tmp) / f"{module_name}.beam"
        beam_path.write_bytes(compilation.beam_bytes)
        eval_expr = (
            f"{{module, _}} = code:load_file({module_name}),"
            f'io:format("~p~n", [{module_name}:main()]),'
            "init:stop()."
        )
        proc = subprocess.run(
            ["erl", "-noshell", "-pa", tmp, "-eval", eval_expr],
            capture_output=True,
            timeout=timeout_seconds,
            check=False,
        )
    return BeamRunResult(
        compilation=compilation,
        stdout=proc.stdout,
        stderr=proc.stderr,
        returncode=proc.returncode,
    )
