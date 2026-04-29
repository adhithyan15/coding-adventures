"""Twig source → real ``dotnet`` end-to-end compiler.

Pipeline
========
::

    Twig source
        ↓ parse + extract     (twig package)
    typed AST
        ↓ Compiler.compile()  (this module — Twig → compiler-IR)
    IrProgram
        ↓ ir-optimizer        (constant folding, DCE)
    IrProgram (optimised)
        ↓ ir-to-cil-bytecode  (CIL method bodies)
    CILProgramArtifact
        ↓ cli-assembly-writer (CLR01-conformant PE/CLI .exe)
    .exe bytes
        ↓ dotnet <name>.exe   (real .NET runtime, net9.0)
    process exit code

Why this is intentionally narrow (matches twig-beam-compiler v1)
================================================================

The v1 surface is the intersection of what ``ir-to-cil-bytecode``
already lowers and what makes for a useful smoke test through real
``dotnet``: integer literals, binary arithmetic (+, -, *, /),
single-binding ``let``, and ``begin`` for sequencing.

The contract: the value of the program's last top-level expression
becomes the **process exit code** — matching how a C#
``static int Main()`` returns to the shell.  ``run_source`` reports
that exit code to the caller as ``ClrRunResult.returncode``.

Register / local convention
===========================

``ir-to-cil-bytecode``'s ``RET``/``HALT`` lowering reads
``local 1`` and emits ``ret``.  We mirror that — the last
expression's value lands in ``IrRegister(1)`` before ``RET``.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from cil_bytecode_builder import CILBytecodeBuilder
from cli_assembly_writer import CLIAssemblyConfig, write_cli_assembly
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
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILMethodArtifact,
    CILProgramArtifact,
    SequentialCILTokenProvider,
    lower_ir_to_cil_bytecode,
)
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
class ClrPackageResult:
    """Aggregated artefacts from one compile run."""

    source: str
    assembly_name: str
    ast: Program
    raw_ir: IrProgram
    optimization: OptimizationResult
    optimized_ir: IrProgram
    assembly_bytes: bytes


@dataclass(frozen=True)
class ClrRunResult:
    """Compilation result + real-``dotnet`` invocation output."""

    compilation: ClrPackageResult
    stdout: bytes
    stderr: bytes
    returncode: int


class ClrPackageError(TwigError):
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


_BINARY_OPS: dict[str, IrOp] = {
    "+": IrOp.ADD,
    "-": IrOp.SUB,
    "*": IrOp.MUL,
    "/": IrOp.DIV,
}

_V1_REJECTED_BUILTINS: frozenset[str] = frozenset(
    {
        "=",
        "<",
        ">",
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


# Register convention.  ``ir-to-cil-bytecode``'s ``RET`` lowering
# reads ``local 1`` and emits ``ret``, so we land the final result
# in ``IrRegister(1)``.  Holding registers start above the
# return-value slot.
_REG_RETURN: Final = 1
_REG_HOLDING_BASE: Final = 2

# Strict allowlist for ``assembly_name`` — used as the on-disk
# ``.exe`` filename and as the CLR module name.  Same defense
# pattern ``twig-beam-compiler`` uses for ``module_name``.
_SAFE_ASSEMBLY_NAME_RE: Final = re.compile(r"^[A-Za-z][A-Za-z0-9_]{0,63}$")


def _validate_assembly_name(name: str) -> None:
    if not _SAFE_ASSEMBLY_NAME_RE.match(name):
        msg = (
            f"assembly_name must match {_SAFE_ASSEMBLY_NAME_RE.pattern!r} "
            f"(letter-start, alnum + underscore, ≤64 chars); got {name!r}.  "
            "Strict rules block path traversal in the on-disk filename and "
            "follow CLR module-naming conventions."
        )
        raise ClrPackageError("validate", msg)


# ---------------------------------------------------------------------------
# Per-region compilation context
# ---------------------------------------------------------------------------


@dataclass
class _FnCtx:
    """Mutable state during compilation of one IR region.

    ``locals_`` maps Twig names from ``let`` bindings to the IR
    register holding the bound value.  ``next_holding`` is the
    next intermediate-value register to allocate.
    """

    locals_: dict[str, IrRegister]
    next_holding: int = _REG_HOLDING_BASE


# ---------------------------------------------------------------------------
# Compiler
# ---------------------------------------------------------------------------


class _Compiler:
    """Walk the Twig AST → ``IrProgram`` with a single ``main`` region."""

    def __init__(self) -> None:
        self._program = IrProgram(entry_label="main")
        self._gen = IDGenerator()

    # ------------------------------------------------------------------

    def compile(self, program: Program) -> IrProgram:
        for form in program.forms:
            if isinstance(form, Define):
                raise TwigCompileError(
                    "(define ...) is not yet supported by the CLR backend "
                    "v1 — only top-level expressions.  See README for the "
                    "v1 surface and roadmap."
                )

        self._emit(IrOp.LABEL, IrLabel(name="main"), id=-1)
        ctx = _FnCtx(locals_={})

        last: IrRegister | None = None
        for expr in program.forms:
            last = self._compile_expr(expr, ctx)

        if last is None:
            zero = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
            last = zero

        # Move the final result into the return register and close
        # the region with RET.  The CIL backend's RET lowering
        # reads ``local 1`` and emits ``ret``.
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
                "lambdas are not yet supported by the CLR backend v1 — "
                "needs the multi-method / closure-class lowering planned "
                "for TW02.5."
            )

        if isinstance(expr, SymLit):
            raise TwigCompileError(
                "quoted symbols are not yet supported by the CLR backend "
                "v1 — atoms-as-values needs heap support across backends."
            )

        msg = f"unsupported Twig form for CLR v1: {type(expr).__name__}"
        raise TwigCompileError(msg)

    def _compile_let(self, expr: Let, ctx: _FnCtx) -> IrRegister:
        if len(expr.bindings) != 1:
            raise TwigCompileError(
                "(let ((name expr) ...) body) with multiple bindings is "
                "not yet supported — v1 only handles a single binding"
            )
        name, value_expr = expr.bindings[0]
        bound_reg = self._compile_expr(value_expr, ctx)

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
                f"builtin {name!r} is not yet supported by the CLR backend "
                "v1 — needs branching / heap / I/O support"
            )
        if name not in _BINARY_OPS:
            raise TwigCompileError(f"unknown builtin: {name!r}")
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

    def _emit(
        self,
        op: IrOp,
        *operands: object,
        id: int | None = None,
    ) -> None:
        self._program.add_instruction(
            IrInstruction(
                op,
                list(operands),
                id=self._gen.next() if id is None else id,
            )
        )

    def _emit_move(self, dst: IrRegister, src: IrRegister) -> None:
        if dst.index == src.index:
            return
        # No MOVE op in compiler-ir; lower as ADD with a freshly
        # zeroed register.  Same idiom ``twig-beam-compiler`` uses.
        zero = IrRegister(index=1000)
        self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
        self._emit(IrOp.ADD, dst, src, zero)

    def _fresh_holding(self, ctx: _FnCtx) -> IrRegister:
        reg = IrRegister(index=ctx.next_holding)
        ctx.next_holding += 1
        return reg


# ---------------------------------------------------------------------------
# Public entry points
# ---------------------------------------------------------------------------


def compile_to_ir(source: str) -> IrProgram:
    """Compile Twig source into an unoptimised :class:`IrProgram`."""
    ast = parse_twig(source)
    program = extract_program(ast)
    return _Compiler().compile(program)


def compile_source(
    source: str,
    *,
    assembly_name: str = "TwigProgram",
    optimize: bool = True,
) -> ClrPackageResult:
    """Run the full Twig → ``.exe`` pipeline end-to-end."""
    _validate_assembly_name(assembly_name)
    try:
        ast = parse_twig(source)
        twig_program = extract_program(ast)
    except Exception as exc:
        raise ClrPackageError("parse", str(exc), exc) from exc

    try:
        raw_ir = _Compiler().compile(twig_program)
    except TwigCompileError:
        raise
    except Exception as exc:  # pragma: no cover
        raise ClrPackageError("ir-emit", str(exc), exc) from exc

    if optimize:
        optimization = IrOptimizer.default_passes().optimize(raw_ir)
        optimized_ir = optimization.program
    else:
        optimization = OptimizationResult(program=raw_ir)
        optimized_ir = raw_ir

    try:
        cil_program = lower_ir_to_cil_bytecode(
            optimized_ir, CILBackendConfig()
        )
    except Exception as exc:
        raise ClrPackageError("lower-cil", str(exc), exc) from exc

    try:
        artifact = write_cli_assembly(
            cil_program,
            CLIAssemblyConfig(
                assembly_name=assembly_name,
                module_name=f"{assembly_name}.exe",
                type_name=assembly_name,
            ),
        )
    except Exception as exc:
        raise ClrPackageError("write-pe", str(exc), exc) from exc

    return ClrPackageResult(
        source=source,
        assembly_name=assembly_name,
        ast=twig_program,
        raw_ir=raw_ir,
        optimization=optimization,
        optimized_ir=optimized_ir,
        assembly_bytes=artifact.assembly_bytes,
    )


def dotnet_available() -> bool:
    """Return ``True`` iff a working ``dotnet`` binary is on PATH."""
    if shutil.which("dotnet") is None:
        return False
    try:
        result = subprocess.run(
            ["dotnet", "--version"],
            capture_output=True,
            timeout=10,
            check=False,
        )
        return result.returncode == 0
    except (subprocess.SubprocessError, OSError):
        return False


def _runtimeconfig_for_net9() -> str:
    """Minimal ``<name>.runtimeconfig.json`` real .NET expects.

    Without this file alongside the assembly, ``dotnet <name>.exe``
    fails before even loading the PE because it can't pick a runtime.
    """
    return json.dumps(
        {
            "runtimeOptions": {
                "tfm": "net9.0",
                "framework": {
                    "name": "Microsoft.NETCore.App",
                    "version": "9.0.0",
                },
            },
        }
    )


def run_source(
    source: str,
    *,
    assembly_name: str = "TwigProgram",
    optimize: bool = True,
    timeout_seconds: int = 30,
) -> ClrRunResult:
    """Compile and execute on the **real** ``dotnet`` runtime.

    Drops the ``.exe`` plus a ``runtimeconfig.json`` to a temp dir
    and invokes ``dotnet <name>.exe``.  The process exit code IS
    the program's result for v1 — same convention as a C#
    ``static int Main()``.
    """
    compilation = compile_source(
        source, assembly_name=assembly_name, optimize=optimize
    )
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        exe_path = tmp_path / f"{assembly_name}.exe"
        cfg_path = tmp_path / f"{assembly_name}.runtimeconfig.json"
        exe_path.write_bytes(compilation.assembly_bytes)
        cfg_path.write_text(_runtimeconfig_for_net9())

        proc = subprocess.run(
            ["dotnet", str(exe_path)],
            capture_output=True,
            timeout=timeout_seconds,
            check=False,
        )
    return ClrRunResult(
        compilation=compilation,
        stdout=proc.stdout,
        stderr=proc.stderr,
        returncode=proc.returncode,
    )
