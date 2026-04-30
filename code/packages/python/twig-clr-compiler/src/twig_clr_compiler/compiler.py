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

Calling convention notes
========================

The CIL backend's natural calling convention is **per-method
locals**: each callable region (= method) gets a fresh
``call_register_count``-wide local frame.  At a ``CALL`` site,
all caller locals get pushed onto the operand stack as args,
the callee receives them via ``ldarg``, copies them into its own
locals, runs, and returns via ``ldloc 1; ret``.

Critically, this gives us **automatic caller-saves** — the
caller's locals (other than local 1, the return slot) are
inviolate across a call.  The JVM backend needed JVM01's manual
caller-saves only because it stored "registers" in a class-level
static int array shared across method invocations; CIL has no
such issue.

Register layout
===============

Mirroring twig-jvm-compiler so the two compilers feel identical:

* ``register 0`` — scratch (also CIL's syscall-arg slot).
* ``register 1`` — function return value / HALT result
  (the CIL backend's ``RET`` lowering reads ``ldloc 1``).
* ``registers 2..N`` — function parameter slots (param ``i``
  lives at register ``_REG_PARAM_BASE + i = 2 + i``).
* ``registers 10..`` — compiler-allocated holding registers for
  intermediate values; using a high base keeps them clear of
  parameter slots, so nested calls don't clobber.

This layout is shared with twig-jvm-compiler because both
backends drive the same IR; only the lowering differs.
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
from twig.free_vars import free_vars

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
# Builtins
# ---------------------------------------------------------------------------


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
    {"cons", "car", "cdr", "null?", "pair?", "number?", "symbol?", "print"}
)


# Register convention — see module docstring.
_REG_HALT_RESULT: Final = 1
_REG_PARAM_BASE: Final = 2
_REG_HOLDING_BASE: Final = 10


# Strict allowlist for ``assembly_name``.  Used as both an
# on-disk filename component and a CLR module/type name.
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
    """Mutable state during compilation of one IR region."""

    locals_: dict[str, IrRegister]
    next_holding: int = _REG_HOLDING_BASE
    next_label: int = 0


# ---------------------------------------------------------------------------
# Compiler
# ---------------------------------------------------------------------------


class _Compiler:
    """Walk the typed Twig AST → ``IrProgram`` with one region per
    top-level function plus a synthesised ``main`` for top-level
    expressions.

    Mirrors twig-jvm-compiler's structure intentionally — both
    drive the same IR pipeline; only the lowering target differs.
    """

    def __init__(self) -> None:
        self._program = IrProgram(entry_label="main")
        self._gen = IDGenerator()
        self._value_consts: dict[str, int] = {}
        self._fn_params: dict[str, list[str]] = {}
        # Label counter is **program-wide**, not per-region.  The
        # CIL backend validates label uniqueness across the entire
        # IR program (sensible: jumps could in theory cross
        # regions).  Per-region counters would produce duplicate
        # ``_else_0`` labels for any program with two functions
        # that both contain ``if``.
        self._next_label_id = 0
        # TW03 Phase 2 / CLR02 Phase 2d: lifted lambdas.  Each
        # anonymous Lambda encountered during expression compilation
        # gets a fresh name like ``_lambda_0`` and is appended here
        # as (lifted_name, captures, lambda_node) so we can emit its
        # body region after the user functions.  The closure
        # value-side machinery (MAKE_CLOSURE / APPLY_CLOSURE) is
        # emitted at the use site.
        self._lifted_lambdas: list[tuple[str, list[str], Lambda]] = []
        self._lambda_counter = 0

    # ------------------------------------------------------------------

    def compile(self, program: Program) -> IrProgram:
        top_level_exprs: list[Expr] = []
        function_defs: list[tuple[str, Lambda]] = []
        for form in program.forms:
            if isinstance(form, Define):
                self._classify_define(form)
                if isinstance(form.expr, Lambda):
                    function_defs.append((form.name, form.expr))
            else:
                top_level_exprs.append(form)

        for name, lam in function_defs:
            self._emit_function(name, lam)

        self._emit_main(top_level_exprs)

        # CLR02 Phase 2d: lifted-lambda regions go AFTER ``main`` so
        # ``main`` is the entry point of the IR instruction stream.
        # ``_compile_expr`` may have appended to ``self._lifted_lambdas``
        # while compiling earlier functions / main; lifted lambdas
        # can also be nested (lambda inside lambda) so the list grows
        # during this final pass — iterate by index.
        i = 0
        while i < len(self._lifted_lambdas):
            lifted_name, captures, lam = self._lifted_lambdas[i]
            self._emit_lifted_lambda(lifted_name, captures, lam)
            i += 1

        return self._program

    # ------------------------------------------------------------------

    def _classify_define(self, form: Define) -> None:
        if isinstance(form.expr, Lambda):
            self._fn_params[form.name] = list(form.expr.params)
            return

        if isinstance(form.expr, IntLit):
            self._value_consts[form.name] = int(form.expr.value)
            return
        if isinstance(form.expr, BoolLit):
            self._value_consts[form.name] = 1 if form.expr.value else 0
            return

        raise TwigCompileError(
            f"(define {form.name} ...) — v1 only supports literal "
            "RHS for value defines.  Use a top-level function for "
            "computed values."
        )

    # ------------------------------------------------------------------
    # Function emission
    # ------------------------------------------------------------------

    def _emit_function(self, name: str, lam: Lambda) -> None:
        self._emit(IrOp.LABEL, IrLabel(name=name), id=-1)

        # Copy each parameter out of its arrival register
        # (``_REG_PARAM_BASE + i``) into a fresh body-local
        # holding register.  Mirrors the JVM01 paired fix in
        # twig-jvm-compiler — keeps the IR shape uniform across
        # both compilers.  CIL itself has automatic caller-saves
        # so this isn't strictly necessary on CLR, but keeping
        # the IR identical means downstream tooling treats both
        # the same.
        ctx = _FnCtx(locals_={})
        for i, param in enumerate(lam.params):
            arrival = IrRegister(index=_REG_PARAM_BASE + i)
            body_local = self._fresh_holding(ctx)
            self._emit_move(body_local, arrival)
            ctx.locals_[param] = body_local

        last: IrRegister | None = None
        for expr in lam.body:
            last = self._compile_expr(expr, ctx)
        if last is None:
            raise TwigCompileError(f"function {name!r} has empty body")

        self._emit_move(IrRegister(_REG_HALT_RESULT), last)
        self._emit(IrOp.RET)

    def _emit_main(self, exprs: list[Expr]) -> None:
        self._emit(IrOp.LABEL, IrLabel(name="main"), id=-1)
        ctx = _FnCtx(locals_={})

        last: IrRegister | None = None
        for expr in exprs:
            last = self._compile_expr(expr, ctx)

        if last is None:
            zero = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
            last = zero

        self._emit_move(IrRegister(_REG_HALT_RESULT), last)
        self._emit(IrOp.RET)

    # ------------------------------------------------------------------
    # Expression compilation
    # ------------------------------------------------------------------

    def _compile_expr(self, expr: Expr, ctx: _FnCtx) -> IrRegister:
        if isinstance(expr, IntLit):
            reg = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, reg, IrImmediate(int(expr.value)))
            return reg

        if isinstance(expr, BoolLit):
            reg = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, reg, IrImmediate(1 if expr.value else 0))
            return reg

        if isinstance(expr, NilLit):
            reg = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, reg, IrImmediate(0))
            return reg

        if isinstance(expr, VarRef):
            if expr.name in ctx.locals_:
                return ctx.locals_[expr.name]
            if expr.name in self._value_consts:
                reg = self._fresh_holding(ctx)
                self._emit(
                    IrOp.LOAD_IMM,
                    reg,
                    IrImmediate(self._value_consts[expr.name]),
                )
                return reg
            raise TwigCompileError(f"unbound name: {expr.name!r}")

        if isinstance(expr, If):
            return self._compile_if(expr, ctx)

        if isinstance(expr, Let):
            return self._compile_let(expr, ctx)

        if isinstance(expr, Begin):
            return self._compile_begin(expr, ctx)

        if isinstance(expr, Apply):
            return self._compile_apply(expr, ctx)

        if isinstance(expr, Lambda):
            return self._compile_anonymous_lambda(expr, ctx)

        if isinstance(expr, SymLit):
            raise TwigCompileError(
                "quoted symbols are not yet supported by the CLR "
                "backend — see TW03 Phase 3."
            )

        msg = f"unsupported Twig form for CLR v1: {type(expr).__name__}"
        raise TwigCompileError(msg)

    def _compile_if(self, expr: If, ctx: _FnCtx) -> IrRegister:
        cond = self._compile_expr(expr.cond, ctx)
        else_label = self._fresh_label(ctx, "else")
        end_label = self._fresh_label(ctx, "endif")
        result = self._fresh_holding(ctx)

        self._emit(IrOp.BRANCH_Z, cond, IrLabel(else_label))

        then_v = self._compile_expr(expr.then_branch, ctx)
        self._emit_move(result, then_v)
        self._emit(IrOp.JUMP, IrLabel(end_label))

        self._emit(IrOp.LABEL, IrLabel(else_label), id=-1)
        else_v = self._compile_expr(expr.else_branch, ctx)
        self._emit_move(result, else_v)

        self._emit(IrOp.LABEL, IrLabel(end_label), id=-1)
        return result

    def _compile_let(self, expr: Let, ctx: _FnCtx) -> IrRegister:
        binding_regs: list[tuple[str, IrRegister]] = []
        for name, rhs in expr.bindings:
            v = self._compile_expr(rhs, ctx)
            binding_regs.append((name, v))

        saved: dict[str, IrRegister | None] = {}
        for name, reg in binding_regs:
            saved[name] = ctx.locals_.get(name)
            ctx.locals_[name] = reg

        last: IrRegister | None = None
        for e in expr.body:
            last = self._compile_expr(e, ctx)
        assert last is not None

        for name, prior in saved.items():
            if prior is None:
                del ctx.locals_[name]
            else:
                ctx.locals_[name] = prior

        return last

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
        # The fast path: a direct VarRef to a known builtin or
        # top-level function compiles to ``CALL`` (or an arithmetic
        # opcode).  Anything else — a Lambda in expression position,
        # a let-bound closure, the result of a function-returning
        # call — falls through to the closure path which uses
        # ``APPLY_CLOSURE`` and dispatches via the IClosure interface.
        if isinstance(expr.fn, VarRef):
            name = expr.fn.name
            if name in _V1_REJECTED_BUILTINS:
                raise TwigCompileError(
                    f"builtin {name!r} is not yet supported by the CLR "
                    "backend — see TW03 Phase 3 for heap primitives."
                )

            if name in _BINARY_OPS:
                if len(expr.args) != 2:
                    raise TwigCompileError(
                        f"{name!r} expects 2 arguments, got {len(expr.args)}"
                    )
                left = self._compile_expr(expr.args[0], ctx)
                right = self._compile_expr(expr.args[1], ctx)
                dest = self._fresh_holding(ctx)
                self._emit(_BINARY_OPS[name], dest, left, right)
                return dest

            if name in self._fn_params:
                params = self._fn_params[name]
                if len(expr.args) != len(params):
                    raise TwigCompileError(
                        f"function {name!r} takes {len(params)} arguments, "
                        f"got {len(expr.args)}"
                    )

                arg_regs: list[IrRegister] = [
                    self._compile_expr(a, ctx) for a in expr.args
                ]

                for i, src in enumerate(arg_regs):
                    self._emit_move(IrRegister(_REG_PARAM_BASE + i), src)

                self._emit(IrOp.CALL, IrLabel(name))

                dest = self._fresh_holding(ctx)
                self._emit_move(dest, IrRegister(_REG_HALT_RESULT))
                return dest

            # Fall through to closure-apply only if the name is
            # actually bound locally (a let-binding holding a
            # closure value).  Unbound names are user errors.
            if name not in ctx.locals_:
                raise TwigCompileError(
                    f"unknown function {name!r}.  Supported: binary "
                    "builtins (+, -, *, /, =, <, >), top-level "
                    "(define (f ...) ...), and closure values bound "
                    "via let or returned from another function."
                )

        # Closure path: compile fn to a register holding a closure
        # value, then APPLY_CLOSURE with the explicit args.
        closure_reg = self._compile_expr(expr.fn, ctx)
        arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
        dest = self._fresh_holding(ctx)
        self._emit(
            IrOp.APPLY_CLOSURE,
            dest,
            closure_reg,
            IrImmediate(len(arg_regs)),
            *arg_regs,
        )
        return dest

    # ------------------------------------------------------------------
    # Closure compilation (CLR02 Phase 2d)
    # ------------------------------------------------------------------

    def _compile_anonymous_lambda(
        self, lam: Lambda, outer: _FnCtx
    ) -> IrRegister:
        """Lift ``lam`` to a fresh top-level function and emit a
        ``MAKE_CLOSURE`` at the use site.

        Free-variable analysis (``twig.free_vars``) determines what
        the lambda captures from its enclosing scope.  Each
        capture must already be bound in ``outer.locals_`` —
        otherwise we'd be referring to a name that doesn't resolve.
        """
        globals_ = (
            set(self._fn_params)
            | set(self._value_consts)
            | set(_BINARY_OPS)
            | _V1_REJECTED_BUILTINS
        )
        captures = free_vars(lam, globals_)

        for c in captures:
            if c not in outer.locals_:
                raise TwigCompileError(
                    f"unbound name {c!r} captured by lambda — "
                    "did you forget a (define) or a (let ...) binding?"
                )

        lifted_name = f"_lambda_{self._lambda_counter}"
        self._lambda_counter += 1
        self._lifted_lambdas.append((lifted_name, captures, lam))

        # Materialise MAKE_CLOSURE at the use site.  The IR op
        # carries the captured values as their *current* IR
        # registers in ``outer``.
        capture_regs = [outer.locals_[c] for c in captures]
        dest = self._fresh_holding(outer)
        self._emit(
            IrOp.MAKE_CLOSURE,
            dest,
            IrLabel(lifted_name),
            IrImmediate(len(captures)),
            *capture_regs,
        )
        return dest

    def _emit_lifted_lambda(
        self,
        lifted_name: str,
        captures: list[str],
        lam: Lambda,
    ) -> None:
        """Emit a callable region for a lifted lambda body.

        Captures-first parameter layout (matches ir-to-cil-bytecode's
        Phase 2c lowering convention): the lifted region's IR
        register layout is

            r2..r{1+num_free}                    — captures
            r{2+num_free}..r{1+num_free+arity}   — explicit params
            r{10+}                               — body holding regs

        ir-to-cil-bytecode's ``_lower_closure_region`` reads
        captures from ``this.captI`` instance fields and the
        explicit arg from ldarg.1 into those slots, then runs
        the IR body unchanged.
        """
        self._emit(IrOp.LABEL, IrLabel(name=lifted_name), id=-1)
        ctx = _FnCtx(locals_={})

        all_params = captures + list(lam.params)
        for i, param in enumerate(all_params):
            arrival = IrRegister(index=_REG_PARAM_BASE + i)
            body_local = self._fresh_holding(ctx)
            self._emit_move(body_local, arrival)
            ctx.locals_[param] = body_local

        last: IrRegister | None = None
        for expr in lam.body:
            last = self._compile_expr(expr, ctx)
        if last is None:
            raise TwigCompileError(f"lambda {lifted_name!r} has empty body")

        self._emit_move(IrRegister(_REG_HALT_RESULT), last)
        self._emit(IrOp.RET)

    # ------------------------------------------------------------------
    # IR-emission helpers
    # ------------------------------------------------------------------

    def _emit(
        self,
        opcode: IrOp,
        *operands: object,
        id: int | None = None,
    ) -> None:
        self._program.add_instruction(
            IrInstruction(
                opcode=opcode,
                operands=list(operands),
                id=self._gen.next() if id is None else id,
            )
        )

    def _emit_move(self, dst: IrRegister, src: IrRegister) -> None:
        if dst == src:
            return
        self._emit(IrOp.ADD_IMM, dst, src, IrImmediate(0))

    def _fresh_holding(self, ctx: _FnCtx) -> IrRegister:
        idx = ctx.next_holding
        ctx.next_holding += 1
        return IrRegister(index=idx)

    def _fresh_label(self, ctx: _FnCtx, prefix: str) -> str:
        # Program-wide counter (see ``__init__`` doc).
        idx = self._next_label_id
        self._next_label_id += 1
        return f"_{prefix}_{idx}"


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
        compiler = _Compiler()
        raw_ir = compiler.compile(twig_program)
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

    # CLR02 Phase 2d: build closure_free_var_counts from the
    # lifted-lambda table the compiler discovered during IR
    # emission.  ir-to-cil-bytecode uses this to detect closure
    # regions and emit them as ``Apply`` methods on the
    # auto-generated ``Closure_<name>`` TypeDefs.
    closure_free_var_counts: dict[str, int] = {}
    for lifted_name, captures, _lam in compiler._lifted_lambdas:  # noqa: SLF001
        closure_free_var_counts[lifted_name] = len(captures)

    try:
        # ``call_register_count=None`` tells the CIL backend to
        # auto-derive the call-arg count from ``plan.local_count``.
        # The default of ``0`` would emit 0-parameter callable
        # methods — which means arg-passing breaks silently:
        # callee reads ldloc 2 expecting the arg but finds 0
        # because no parameters were declared and no entry-copy
        # ran.  ``None`` is the only correct setting for any
        # program that calls user-defined functions with args.
        cil_program = lower_ir_to_cil_bytecode(
            optimized_ir,
            CILBackendConfig(
                call_register_count=None,
                closure_free_var_counts=closure_free_var_counts,
            ),
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

    The process exit code IS the program's result for v1 — same
    convention as a C# ``static int Main()``.
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
