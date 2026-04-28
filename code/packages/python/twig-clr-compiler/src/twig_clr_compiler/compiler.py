"""Twig → CLR compiler — TW02 v1 arithmetic floor.

This module performs two jobs:

1. Walks Twig's typed AST (from :mod:`twig.ast_nodes`) and emits an
   :class:`compiler_ir.IrProgram` for the v1 surface (integer
   literals, booleans, arithmetic, comparisons, ``if`` / ``let`` /
   ``begin``, and a single top-level expression whose value becomes
   the program's exit code).
2. Runs that program through the existing CLR pipeline:
   ``ir-optimizer`` → ``ir-to-cil-bytecode`` → ``cli-assembly-writer``
   → ``clr-vm-simulator``.

The module deliberately rejects any v1-out-of-scope construct
(``define``, ``lambda``, ``cons``, ``print``, …) at compile time
with a :class:`TwigCompileError`.  These come back online in
TW02.5 / TW03 — see ``code/specs/TW02-twig-clr-compiler.md``.

Why mirror ``brainfuck-clr-compiler`` so closely?
-------------------------------------------------
The brainfuck CLR compiler already exercises the full pipeline
(parse → IR → optimise → CIL → assembly → simulator) and gives us
a battle-tested reference for the package shape, dataclasses, and
error-handling conventions.  Mirroring it now makes the future
TW03 (BEAM) and TW04 (JVM) compilers straightforward — they reuse
the same scaffolding with a different lowering tail.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

from cli_assembly_writer import (
    CLIAssemblyArtifact,
    CLIAssemblyConfig,
    write_cli_assembly,
)
from clr_pe_file import CLRPEFile, decode_clr_pe_file
from clr_vm_simulator import CLRVMResult, CLRVMStdlibHost, run_clr_entry_point
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
    CILProgramArtifact,
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
    BoolLit,
    Define,
    Expr,
    If,
    IntLit,
    Lambda,
    Let,
    NilLit,
    Program,
    SymLit,
    VarRef,
)

# ---------------------------------------------------------------------------
# Result dataclasses
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class PackageResult:
    """Aggregated artefacts from one compile run."""

    source: str
    filename: str
    assembly_name: str
    type_name: str
    ast: Program
    raw_ir: IrProgram
    optimization: OptimizationResult
    optimized_ir: IrProgram
    cil_artifact: CILProgramArtifact
    assembly_artifact: CLIAssemblyArtifact
    decoded_assembly: CLRPEFile
    assembly_bytes: bytes
    assembly_path: Path | None = None


@dataclass(frozen=True)
class ExecutionResult:
    """Compilation result + simulator output."""

    compilation: PackageResult
    vm_result: CLRVMResult


class PackageError(TwigError):
    """Wraps a stage-tagged failure during compilation or execution."""

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
#
# Each entry maps a Twig builtin name to the corresponding ``IrOp`` for
# its single-instruction emission.  Every op here is binary (two
# register operands) — the v1 surface excludes variadic forms.

_BINARY_OPS: dict[str, IrOp] = {
    "+": IrOp.ADD,
    "-": IrOp.SUB,
    "*": IrOp.MUL,
    "/": IrOp.DIV,
    "=": IrOp.CMP_EQ,
    "<": IrOp.CMP_LT,
    ">": IrOp.CMP_GT,
}

_V1_REJECTED_BUILTINS: frozenset[str] = frozenset(
    {
        "cons", "car", "cdr",
        "null?", "pair?", "number?", "symbol?",
        "print",
    }
)


# ---------------------------------------------------------------------------
# AST → IrProgram
# ---------------------------------------------------------------------------


class _Compiler:
    """Walks a typed Twig AST and accumulates ``IrInstruction``s.

    The compiler is a one-shot object: each ``compile`` call builds a
    fresh ``IrProgram`` tagged with ``entry_label="_start"``.  The
    walker tracks one mutable map of let-bound names to register
    indices and one fresh-register / fresh-label counter pair.
    """

    def __init__(self) -> None:
        self._program = IrProgram(entry_label="_start")
        self._gen = IDGenerator()
        self._next_reg = 0
        self._next_label = 0
        self._locals: dict[str, IrRegister] = {}

    # ------------------------------------------------------------------
    # Public entry
    # ------------------------------------------------------------------

    def compile(self, program: Program) -> IrProgram:
        # V1: reject any top-level define.  TW02.5 will lift this.
        for form in program.forms:
            if isinstance(form, Define):
                raise TwigCompileError(
                    "(define ...) is not yet supported by the CLR backend "
                    "— see TW02.5"
                )

        # Emit the entry label so the WASM-style "split functions"
        # logic in downstream consumers recognises this as the entry
        # function (consistent with how brainfuck-ir-compiler emits).
        self._emit_label("_start")

        # Compile each top-level expression in order; the last one's
        # value becomes the HALT result (i.e. the program's exit code
        # in the CLR simulator).
        last: IrRegister | None = None
        for form in program.forms:
            assert isinstance(form, Expr)
            last = self._compile_expr(form)

        if last is None:
            # Empty program → return 0 (CLR simulators expect *some*
            # value at HALT; using 0 matches Brainfuck's convention).
            zero = self._fresh_reg()
            self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
            last = zero

        # The ir-to-cil-bytecode lowering reads ``IrRegister(1)`` at
        # HALT for the function's return value.  Move our last result
        # there so the CLR simulator's exit code matches the program's
        # value.  This mirrors the convention used by
        # ``wasm-backend.compile()``.
        if last.index != 1:
            self._emit(IrOp.ADD_IMM, IrRegister(1), last, IrImmediate(0))

        self._emit(IrOp.HALT)
        return self._program

    # ------------------------------------------------------------------
    # Expression dispatch
    # ------------------------------------------------------------------

    def _compile_expr(self, expr: Expr) -> IrRegister:
        """Compile ``expr`` and return the register holding its value."""
        if isinstance(expr, IntLit):
            return self._compile_int(expr)
        if isinstance(expr, BoolLit):
            return self._compile_bool(expr)
        if isinstance(expr, NilLit):
            raise TwigCompileError(
                "nil is not yet supported by the CLR backend "
                "(needs heap objects — see TW02.5)"
            )
        if isinstance(expr, SymLit):
            raise TwigCompileError(
                "symbols are not yet supported by the CLR backend "
                "(needs heap objects — see TW02.5)"
            )
        if isinstance(expr, VarRef):
            return self._compile_var_ref(expr)
        if isinstance(expr, If):
            return self._compile_if(expr)
        if isinstance(expr, Let):
            return self._compile_let(expr)
        if isinstance(expr, Begin):
            return self._compile_begin(expr)
        if isinstance(expr, Lambda):
            raise TwigCompileError(
                "lambdas are not yet supported by the CLR backend "
                "— see TW02.5"
            )
        if isinstance(expr, Apply):
            return self._compile_apply(expr)
        raise TwigCompileError(
            f"unhandled expression type: {type(expr).__name__}"
        )

    # ------------------------------------------------------------------
    # Atoms
    # ------------------------------------------------------------------

    def _compile_int(self, expr: IntLit) -> IrRegister:
        reg = self._fresh_reg()
        self._emit(IrOp.LOAD_IMM, reg, IrImmediate(expr.value))
        return reg

    def _compile_bool(self, expr: BoolLit) -> IrRegister:
        reg = self._fresh_reg()
        self._emit(IrOp.LOAD_IMM, reg, IrImmediate(1 if expr.value else 0))
        return reg

    def _compile_var_ref(self, expr: VarRef) -> IrRegister:
        reg = self._locals.get(expr.name)
        if reg is None:
            raise TwigCompileError(
                f"unbound name {expr.name!r} — only let-bindings are "
                "supported in TW02 v1 (top-level defines come in TW02.5)"
            )
        return reg

    # ------------------------------------------------------------------
    # Compound forms
    # ------------------------------------------------------------------

    def _compile_if(self, expr: If) -> IrRegister:
        """``if`` lowers to ``BRANCH_Z`` over labeled blocks.

        Layout:

        ::

            <cond into c_reg>
            BRANCH_Z c_reg, else_label
            <then into t_reg>
            ADD_IMM result, t_reg, 0      ; move
            JUMP end_label
        else_label:
            <else into e_reg>
            ADD_IMM result, e_reg, 0
        end_label:
        """
        cond_reg = self._compile_expr(expr.cond)
        else_label = self._fresh_label("else")
        end_label = self._fresh_label("endif")
        result = self._fresh_reg()

        self._emit(IrOp.BRANCH_Z, cond_reg, IrLabel(else_label))

        then_reg = self._compile_expr(expr.then_branch)
        self._emit(IrOp.ADD_IMM, result, then_reg, IrImmediate(0))
        self._emit(IrOp.JUMP, IrLabel(end_label))

        self._emit_label(else_label)
        else_reg = self._compile_expr(expr.else_branch)
        self._emit(IrOp.ADD_IMM, result, else_reg, IrImmediate(0))

        self._emit_label(end_label)
        return result

    def _compile_let(self, expr: Let) -> IrRegister:
        """Mutually-independent (Scheme ``let``) bindings.

        We compile each RHS in the *outer* scope, then extend the
        local map for the body.  After the body compiles we restore
        the outer map so subsequent code in the enclosing scope
        doesn't see the let names.
        """
        binding_regs: list[tuple[str, IrRegister]] = []
        for name, rhs in expr.bindings:
            rhs_reg = self._compile_expr(rhs)
            binding_regs.append((name, rhs_reg))

        saved: dict[str, IrRegister | None] = {}
        for name, reg in binding_regs:
            saved[name] = self._locals.get(name)
            self._locals[name] = reg

        last: IrRegister | None = None
        for e in expr.body:
            last = self._compile_expr(e)
        assert last is not None  # parser rejects empty body

        for name, prior in saved.items():
            if prior is None:
                del self._locals[name]
            else:
                self._locals[name] = prior

        return last

    def _compile_begin(self, expr: Begin) -> IrRegister:
        last: IrRegister | None = None
        for e in expr.exprs:
            last = self._compile_expr(e)
        assert last is not None
        return last

    def _compile_apply(self, expr: Apply) -> IrRegister:
        """v1 supports binary builtins only (``+``, ``-``, ``*``, ``/``,
        ``=``, ``<``, ``>``).  Other applications are rejected.
        """
        if not isinstance(expr.fn, VarRef):
            raise TwigCompileError(
                "only direct builtin calls are supported in TW02 v1"
            )
        name = expr.fn.name

        if name in _V1_REJECTED_BUILTINS:
            raise TwigCompileError(
                f"builtin {name!r} is not yet supported by the CLR backend "
                "— see TW02.5 for cons / symbols / print"
            )

        if name not in _BINARY_OPS:
            raise TwigCompileError(
                f"unknown function {name!r} — TW02 v1 supports only the "
                "binary builtins +, -, *, /, =, <, >"
            )
        if len(expr.args) != 2:
            raise TwigCompileError(
                f"{name!r} expects exactly 2 arguments in TW02 v1, "
                f"got {len(expr.args)}"
            )

        left = self._compile_expr(expr.args[0])
        right = self._compile_expr(expr.args[1])
        dest = self._fresh_reg()
        self._emit(_BINARY_OPS[name], dest, left, right)
        return dest

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _fresh_reg(self) -> IrRegister:
        # Skip register 0 (some pipelines use it as a scratch / tape
        # base) and 1 (HALT-result convention) so we don't clobber
        # them with intermediate values during expression evaluation.
        # Brainfuck-ir-compiler uses similar conventions.
        idx = self._next_reg + 2
        self._next_reg += 1
        return IrRegister(index=idx)

    def _fresh_label(self, prefix: str) -> str:
        self._next_label += 1
        return f"{prefix}_{self._next_label}"

    def _emit(self, opcode: IrOp, *operands: object) -> None:
        self._program.add_instruction(
            IrInstruction(
                opcode=opcode,
                operands=list(operands),  # type: ignore[arg-type]
                id=self._gen.next(),
            )
        )

    def _emit_label(self, name: str) -> None:
        self._program.add_instruction(
            IrInstruction(
                opcode=IrOp.LABEL,
                operands=[IrLabel(name=name)],
                id=-1,  # labels use -1, matching the rest of the repo
            )
        )


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def compile_to_ir(source: str) -> IrProgram:
    """Compile Twig source into an unoptimised :class:`IrProgram`."""
    ast = parse_twig(source)
    program = extract_program(ast)
    return _Compiler().compile(program)


def compile_source(
    source: str,
    *,
    filename: str = "program.twig",
    assembly_name: str = "TwigProgram",
    type_name: str = "TwigProgram",
    optimize: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    """Run the full Twig → CIL → PE assembly pipeline.

    Parameters mirror ``brainfuck_clr_compiler.compile_source`` so
    callers familiar with the BF backend get the same handles.
    """
    try:
        ast = parse_twig(source)
        twig_program = extract_program(ast)
    except Exception as exc:
        raise PackageError("parse", str(exc), exc) from exc

    try:
        raw_ir = _Compiler().compile(twig_program)
    except TwigCompileError:
        raise
    except Exception as exc:  # pragma: no cover
        raise PackageError("ir-emit", str(exc), exc) from exc

    if optimize:
        optimization = IrOptimizer.default_passes().optimize(raw_ir)
        optimized_ir = optimization.program
    else:
        optimization = OptimizationResult(program=raw_ir)
        optimized_ir = raw_ir

    resolved_cil = cil_config or CILBackendConfig(syscall_arg_reg=4)
    try:
        cil_artifact = lower_ir_to_cil_bytecode(optimized_ir, resolved_cil)
    except Exception as exc:
        raise PackageError("cil-emit", str(exc), exc) from exc

    try:
        assembly_artifact = write_cli_assembly(
            cil_artifact,
            CLIAssemblyConfig(
                assembly_name=assembly_name,
                module_name=f"{assembly_name}.exe",
                type_name=type_name,
            ),
        )
    except Exception as exc:
        raise PackageError("assembly", str(exc), exc) from exc

    decoded = decode_clr_pe_file(assembly_artifact.assembly_bytes)

    return PackageResult(
        source=source,
        filename=filename,
        assembly_name=assembly_name,
        type_name=type_name,
        ast=twig_program,
        raw_ir=raw_ir,
        optimization=optimization,
        optimized_ir=optimized_ir,
        cil_artifact=cil_artifact,
        assembly_artifact=assembly_artifact,
        decoded_assembly=decoded,
        assembly_bytes=assembly_artifact.assembly_bytes,
    )


def run_source(source: str, **kwargs: Any) -> ExecutionResult:
    """Compile and execute on the in-house ``clr-vm-simulator``."""
    compilation = compile_source(source, **kwargs)
    host = CLRVMStdlibHost()
    vm_result = run_clr_entry_point(compilation.assembly_bytes, host=host)
    return ExecutionResult(compilation=compilation, vm_result=vm_result)
