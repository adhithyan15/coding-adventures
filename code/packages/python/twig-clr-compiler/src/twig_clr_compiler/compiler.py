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
    CILProgramArtifact,
    CILTypeArtifact,
    lower_ir_to_cil_bytecode,
)
from ir_to_cil_bytecode.backend import SequentialCILTokenProvider
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
from twig.module_resolver import HOST_MODULE_NAME, ResolvedModule

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

# TW03 Phase 3e — heap-primitive builtins now compile (was rejected
# in v1).  Each maps to its corresponding IR opcode below.  The
# remaining rejected set is shrunk to opcodes still without a CLR
# lowering.
_V1_REJECTED_BUILTINS: frozenset[str] = frozenset(
    {"number?", "print"}
)

# TW03 Phase 3e — Lisp heap-primitive builtins.  Each entry maps a
# builtin name to its IR opcode + arity so the apply-site can
# generate a uniform call sequence.  Phase 3c's CLR backend
# lowering will turn these into ``newobj`` / ``ldfld`` / ``isinst``
# CIL bytecode against the auto-generated Cons / Symbol / Nil
# TypeDefs.
_HEAP_BUILTINS: dict[str, tuple[IrOp, int]] = {
    "cons":    (IrOp.MAKE_CONS, 2),
    "car":     (IrOp.CAR, 1),
    "cdr":     (IrOp.CDR, 1),
    "null?":   (IrOp.IS_NULL, 1),
    "pair?":   (IrOp.IS_PAIR, 1),
    "symbol?": (IrOp.IS_SYMBOL, 1),
}


# Register convention — see module docstring.
_REG_SCRATCH: Final = 0
_REG_HALT_RESULT: Final = 1
_REG_PARAM_BASE: Final = 2
_REG_HOLDING_BASE: Final = 10

# Platform-independent syscall numbers — shared with twig-jvm-compiler
# and the interpreter (``twig/compiler.py``).  The CIL backend already
# supports all three: SYSCALL 1 (write-byte), 2 (read-byte), 10 (exit).
_HOST_SYSCALLS: dict[str, int] = {
    "host/write-byte": 1,  # write one byte to stdout
    "host/read-byte":  2,  # read one byte from stdin; return -1 on EOF
    "host/exit":       10, # exit the process with the given code
}

# Register 0 is used as the SYSCALL argument slot for the CLR backend.
# This matches the CILBackendConfig(syscall_arg_reg=_REG_SCRATCH) below
# so that ``emit_ldloc(config.syscall_arg_reg)`` loads the right value.
_CLR_SYSCALL_ARG_REG: Final = _REG_SCRATCH


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
            # TW03 Phase 3e: lower nil to LOAD_NIL.
            reg = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_NIL, reg)
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
            # TW03 Phase 3e: lower 'foo / (quote foo) to MAKE_SYMBOL.
            reg = self._fresh_holding(ctx)
            self._emit(IrOp.MAKE_SYMBOL, reg, IrLabel(expr.name))
            return reg

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

            # Module-qualified host syscall: ``host/write-byte``,
            # ``host/read-byte``, ``host/exit``.  The slash in the
            # name identifies a host-module reference.  We look up the
            # platform-independent syscall number and emit
            # ``IrOp.SYSCALL IrImmediate(num) IrRegister(result_reg)``.
            #
            # The CIL backend loads the arg from
            # ``config.syscall_arg_reg`` (= ``_CLR_SYSCALL_ARG_REG`` =
            # register 0 = scratch), so for write-byte/exit we move the
            # arg into register 0 first.  For read-byte there's no
            # input arg; register 0 is loaded by the backend but ignored
            # by ``__ca_syscall`` for num=2.
            _slash = name.find("/")
            if _slash > 0 and _slash < len(name) - 1:
                # Module-qualified call — ``host/write-byte`` or a user-module
                # cross-call like ``a/math/add``.
                num = _HOST_SYSCALLS.get(name)
                if num is None:
                    # TW04 Phase 4e: user-module cross-call.  Emit
                    # ``IrOp.CALL IrLabel(name)`` with args marshalled
                    # into param slots first.  The IR label contains
                    # ``/`` so the CIL backend recognises it as a
                    # cross-module call and resolves it via
                    # ``config.external_method_tokens`` at lowering time.
                    xm_arg_regs = [
                        self._compile_expr(a, ctx) for a in expr.args
                    ]
                    for i, src in enumerate(xm_arg_regs):
                        self._emit_move(IrRegister(_REG_PARAM_BASE + i), src)
                    self._emit(IrOp.CALL, IrLabel(name))
                    dest = self._fresh_holding(ctx)
                    self._emit_move(dest, IrRegister(_REG_HALT_RESULT))
                    return dest
                scratch = IrRegister(_CLR_SYSCALL_ARG_REG)
                if num == 2:  # read-byte — no user arg, result into fresh reg
                    dest = self._fresh_holding(ctx)
                    self._emit(IrOp.SYSCALL, IrImmediate(num), dest)
                    return dest
                # write-byte (1) or exit (10) — one user arg
                arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
                self._emit_move(scratch, arg_regs[0])
                self._emit(IrOp.SYSCALL, IrImmediate(num), scratch)
                return scratch

            if name in _V1_REJECTED_BUILTINS:
                raise TwigCompileError(
                    f"builtin {name!r} is not yet supported by the CLR "
                    "backend — see TW03 Phase 3 for heap primitives."
                )

            # TW03 Phase 3e — heap-primitive builtins.  Single-shape
            # lowering: evaluate args, fresh holding reg for the
            # result, emit the IR opcode.
            if name in _HEAP_BUILTINS:
                op, expected_arity = _HEAP_BUILTINS[name]
                if len(expr.args) != expected_arity:
                    raise TwigCompileError(
                        f"{name!r} expects {expected_arity} arguments, "
                        f"got {len(expr.args)}"
                    )
                arg_regs = [
                    self._compile_expr(a, ctx) for a in expr.args
                ]
                dest = self._fresh_holding(ctx)
                self._emit(op, dest, *arg_regs)
                return dest

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
            | set(_HEAP_BUILTINS)
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
    #
    # Multi-arity follow-up: also record each lambda's explicit
    # arity (its source-level param count, NOT counting captures)
    # so the backend's IClosure.Apply prologue extracts the right
    # number of int args from the int32[] parameter.  Without
    # this, every closure body would be treated as arity-1 and
    # multi-arg lambdas like ``(lambda (x y) (+ x y))`` would
    # silently drop the second arg (arity-1 hard limit removed
    # in the same change).
    closure_free_var_counts: dict[str, int] = {}
    closure_explicit_arities: dict[str, int] = {}
    for lifted_name, captures, lam in compiler._lifted_lambdas:  # noqa: SLF001
        closure_free_var_counts[lifted_name] = len(captures)
        closure_explicit_arities[lifted_name] = len(lam.params)

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
                closure_explicit_arities=closure_explicit_arities,
                # The compiler emits ``IrOp.SYSCALL`` with the arg value
                # already in register 0 (scratch).  Configure the CLR
                # backend to read the syscall arg from that register so
                # it matches the JVM convention and the compiler's intent.
                syscall_arg_reg=_CLR_SYSCALL_ARG_REG,
                # TW04 Phase 4c: use inline .NET API calls for SYSCALL 1/2/10
                # (Console.Write / Console.Read / Environment.Exit) instead of
                # the brainfuck-specific __ca_syscall external helper.
                inline_host_syscalls=True,
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


# ---------------------------------------------------------------------------
# TW04 Phase 4e — multi-module CLR compilation
# ---------------------------------------------------------------------------
#
# Each Twig module (excluding the synthetic ``host`` module) becomes one
# TypeDef row in a **single** PE/CLI assembly.  Cross-module function calls
# lower to ordinary CIL ``call`` instructions addressed by pre-assigned
# MethodDef tokens in the combined assembly.
#
# Why one assembly (not one PE per module)?
# -----------------------------------------
# CLR cross-assembly calls require ``AssemblyRef`` + ``MemberRef`` tokens
# which ``cli-assembly-writer`` does not yet support.  Packing all modules
# into one PE lets every ``call`` instruction use a simple ``MethodDef``
# token (``0x06xxxxxx``) — the same kind used for same-TypeDef calls.
#
# CLR type naming
# ---------------
# CLR type names may not contain ``/``.  We replace every ``/`` in the
# Twig module name with ``_`` to produce a valid CLR identifier:
# ``"a/math"`` → ``"a_math"``, ``"user/hello"`` → ``"user_hello"``.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ModuleClrCompileResult:
    """Per-module artefact from a multi-module CLR compile run.

    ``type_name`` is the CLR TypeDef name for this module's methods in the
    combined assembly (``module_name_to_clr_type(module_name)``).
    ``callable_names`` lists the emitted method names in IR label-position
    order — used by callers to reconstruct the MethodDef token layout.
    """

    module_name: str
    type_name: str
    ir: IrProgram
    artifact: CILProgramArtifact
    callable_names: tuple[str, ...]


@dataclass(frozen=True)
class MultiModuleClrResult:
    """Combined artefact from compiling a complete Twig module graph to CLR.

    ``assembly_bytes`` is a single PE/CLI ``.exe`` containing one TypeDef
    per Twig module.  ``entry_type_name`` is the TypeDef that owns the
    ``.entrypoint`` method.
    """

    entry_module: str
    module_results: tuple[ModuleClrCompileResult, ...]
    assembly_bytes: bytes
    entry_type_name: str


@dataclass(frozen=True)
class MultiModuleClrExecutionResult:
    """Compile + real-``dotnet`` execution result for a multi-module program."""

    compilation: MultiModuleClrResult
    stdout: bytes
    stderr: bytes
    returncode: int


def module_name_to_clr_type(name: str) -> str:
    """Map a Twig module name to a CLR type name.

    CLR type names may not contain ``/``.  We replace every occurrence with
    ``_`` so ``"a/math"`` becomes ``"a_math"`` and ``"user/hello"`` becomes
    ``"user_hello"``.  The mapping is injective for all legal Twig module
    names.

    Example::

        >>> module_name_to_clr_type("user/hello")
        'user_hello'
        >>> module_name_to_clr_type("a/math")
        'a_math'
    """
    return name.replace("/", "_")


def _compile_module_to_ir(
    module: ResolvedModule,
) -> tuple[IrProgram, list[tuple[str, list[str], Lambda]], IrProgram]:
    """Compile a resolved Twig module to raw IR, lifted lambdas, optimised IR.

    Returns ``(raw_ir, lifted_lambdas, optimized_ir)``.  The lifted-lambdas
    list mirrors what ``_Compiler._lifted_lambdas`` accumulates and is used
    to build ``closure_free_var_counts`` for the CIL backend.
    """
    compiler = _Compiler()
    raw_ir = compiler.compile(module.program)
    optimization = IrOptimizer.default_passes().optimize(raw_ir)
    return raw_ir, list(compiler._lifted_lambdas), optimization.program  # noqa: SLF001


def _discover_module_callable_names(
    optimized_ir: IrProgram,
    lifted_lambdas: list[tuple[str, list[str], Lambda]],
    exports: list[str],
) -> tuple[str, ...]:
    """Return the main-type callable method names for a module's IR program.

    The "main-type" callables are all callable regions *minus* lifted-lambda
    (closure) regions.  They are ordered by their label position in the IR
    instruction stream, which matches the MethodDef table row order that the
    assembly writer emits.

    ``exports`` are included even when no local CALL targets them (they are
    only called from OTHER modules).

    Phase 4e restriction: dep modules must not contain closures.  The entry
    module's closures are handled separately during token assignment.
    """
    # Names of lifted lambda regions (closure bodies).
    closure_region_names: set[str] = {name for name, _, _ in lifted_lambdas}

    # Scan label positions in IR instruction order.
    label_positions: dict[str, int] = {}
    for i, instr in enumerate(optimized_ir.instructions):
        if instr.opcode == IrOp.LABEL:
            label_positions[instr.operands[0].name] = i  # type: ignore[union-attr]

    # Seed with the entry label and any local CALL targets.
    callable_names: set[str] = {optimized_ir.entry_label}
    for instr in optimized_ir.instructions:
        if instr.opcode == IrOp.CALL:
            target_name = instr.operands[0].name  # type: ignore[union-attr]
            if "/" not in target_name:
                callable_names.add(target_name)
        elif instr.opcode == IrOp.MAKE_CLOSURE:
            target_name = instr.operands[1].name  # type: ignore[union-attr]
            callable_names.add(target_name)

    # Include exports — they are only called cross-module so they have no
    # local CALL callers.
    for exp in exports:
        if exp in label_positions:
            callable_names.add(exp)

    # Filter out closure regions; sort by IR position.
    main_callables = callable_names - closure_region_names
    return tuple(
        sorted(main_callables, key=lambda n: label_positions.get(n, 0))
    )


def compile_modules(
    modules: list[ResolvedModule],
    *,
    entry_module: str,
    assembly_name: str = "TwigProgram",
) -> MultiModuleClrResult:
    """Compile a topologically ordered list of resolved Twig modules to a
    single CLR PE/CLI assembly.

    ``modules`` should be the list returned by ``twig.resolve_modules`` (in
    dependency-first topological order).  ``entry_module`` names the module
    whose ``main`` function becomes the assembly's entry point.  The synthetic
    ``host`` module is silently skipped.

    **Phase 4e restriction**: dependency modules must not contain closures or
    heap primitives.  These extensions are reserved for a future phase.

    The function uses a two-pass approach:

    **Pass 1** — compile each module's Twig source to IR, discover each
    module's main-type callable names, and compute cumulative MethodDef token
    offsets.

    **Pass 2** — recompile each module to a ``CILProgramArtifact`` using a
    ``SequentialCILTokenProvider`` with the correct ``method_token_offset``
    and ``CILBackendConfig.external_method_tokens`` populated from Pass 1.

    The entry module becomes the main TypeDef (row 2) in the combined
    assembly; each dep module becomes an extra TypeDef row.
    """
    _validate_assembly_name(assembly_name)

    # Skip the synthetic host module — it has no IR to compile.
    real_modules = [m for m in modules if m.name != HOST_MODULE_NAME]
    if not real_modules:
        raise ClrPackageError(
            "compile-modules",
            "no real modules found (only host module present)",
        )

    # Reorder: entry module first, then deps in the order they appear.
    try:
        entry_mod = next(m for m in real_modules if m.name == entry_module)
    except StopIteration:
        raise ClrPackageError(
            "compile-modules",
            f"entry module {entry_module!r} not found in modules list",
        ) from None
    dep_mods = [m for m in real_modules if m.name != entry_module]
    # Topological order: deps before entry (from resolve_modules), but we want
    # entry first so its methods get the lowest MethodDef token numbers.
    ordered: list[ResolvedModule] = [entry_mod] + dep_mods

    # ── Pass 1: compile each module to IR, discover callable names ────────

    # Per-module: (raw_ir, lifted_lambdas, optimized_ir)
    _IrTriple = tuple[IrProgram, list[tuple[str, list[str], Lambda]], IrProgram]
    module_irs: dict[str, _IrTriple] = {}
    # Ordered callable names for the main TypeDef of each module.
    module_callables: dict[str, tuple[str, ...]] = {}

    for mod in ordered:
        raw_ir, lifted, opt_ir = _compile_module_to_ir(mod)
        module_irs[mod.name] = (raw_ir, lifted, opt_ir)
        mod_decl = mod.program.module
        exports = list(mod_decl.exports) if mod_decl else []
        callables = _discover_module_callable_names(opt_ir, lifted, exports)
        module_callables[mod.name] = callables

    # ── Shared call_register_count ────────────────────────────────────────
    #
    # CIL methods declare their parameter count in the method signature.
    # A cross-module ``call`` pushes exactly ``call_register_count`` arguments.
    # If each module used its own local_count (derived from its own max
    # register index), the entry module (which has many more holding registers
    # than dep modules) would push MORE arguments than the dep method declares
    # — causing a ``System.InvalidProgramException`` at runtime.
    #
    # The fix: compute the global maximum local_count across ALL modules and
    # use it as ``call_register_count`` for every module.  Dep module methods
    # end up declaring more parameters than they strictly need, but the extra
    # parameter slots are simply never read — this is harmless.
    #
    # local_count = max(max_reg_index + 1, syscall_arg_reg + 1, 2)
    # We use ``_CLR_SYSCALL_ARG_REG = 0``, so the floor is 2.
    global_local_count = 2
    for mod in ordered:
        _, _, opt_ir = module_irs[mod.name]
        max_reg = -1
        for instr in opt_ir.instructions:
            for operand in instr.operands:
                if isinstance(operand, IrRegister):
                    max_reg = max(max_reg, operand.index)
        mod_local_count = max(max_reg + 1, _CLR_SYSCALL_ARG_REG + 1, 2)
        global_local_count = max(global_local_count, mod_local_count)

    # ── Assign cumulative MethodDef token offsets ─────────────────────────
    #
    # Entry module callables: rows 1..M_entry          (offset 0)
    # Entry module closure rows: rows M_entry+1..      (1 + 2*K)
    # Dep 1 callables: rows after entry module total   (offset = entry total)
    # Dep 2 callables: rows after entry + dep 1 total
    #
    # Helper / MemberRef tokens (0x0Axxxxxx) are unaffected — they always
    # start at 0x0A000001 in every module's token provider and map to the
    # SAME five MemberRef rows in the combined assembly.

    module_token_offset: dict[str, int] = {}
    running_offset = 0
    for mod in ordered:
        module_token_offset[mod.name] = running_offset
        running_offset += len(module_callables[mod.name])
        # Entry module closure rows (Phase 4e: only entry module may have closures).
        if mod.name == entry_module:
            _, lifted, _ = module_irs[mod.name]
            num_closures = len(lifted)
            if num_closures:
                # IClosure::Apply (1 row) + per-closure (.ctor + Apply = 2 rows each)
                running_offset += 1 + 2 * num_closures

    # ── Build all_external_tokens: for each module, the qualified label →
    # pre-assigned MethodDef token of every OTHER module's callable. ───────
    #
    # A dep-module callable ``add`` with module name ``a/math`` is
    # referenced in a CALL instruction as ``IrLabel("a/math/add")`` — the
    # module name, a ``/`` separator, then the function name.  The token is
    # ``0x06000000 | (offset + row_within_module)``.

    all_external_tokens: dict[str, dict[str, int]] = {
        mod.name: {} for mod in ordered
    }
    for dep in ordered:
        dep_offset = module_token_offset[dep.name]
        for i, callable_name in enumerate(module_callables[dep.name]):
            qualified = f"{dep.name}/{callable_name}"
            token = 0x06000001 + dep_offset + i - 1  # 1-indexed row
            # Correct: 0x06000001 + dep_offset means row (dep_offset+1).
            # But row indexing: entry offset=0 → row 1 = 0x06000001 + 0.
            # dep_offset=M_entry → row M_entry+1 = 0x06000001 + M_entry.
            token = 0x06000000 | (dep_offset + 1 + i)
            for other in ordered:
                if other.name != dep.name:
                    all_external_tokens[other.name][qualified] = token

    # ── Pass 2: compile each module to CILProgramArtifact ────────────────

    module_results: list[ModuleClrCompileResult] = []

    for mod in ordered:
        raw_ir, lifted, opt_ir = module_irs[mod.name]
        is_entry = mod.name == entry_module
        offset = module_token_offset[mod.name]
        callables = module_callables[mod.name]
        ext_tokens = all_external_tokens[mod.name]

        closure_free_var_counts: dict[str, int] = {}
        closure_explicit_arities: dict[str, int] = {}
        if is_entry:
            for lifted_name, captures, lam in lifted:
                closure_free_var_counts[lifted_name] = len(captures)
                closure_explicit_arities[lifted_name] = len(lam.params)

        mod_decl = mod.program.module
        exports = list(mod_decl.exports) if mod_decl else []

        # Build a provider that starts at the right MethodDef row for this
        # module.  The entry module uses offset=0 (rows start at 1); dep
        # modules use their cumulative offset.
        closure_names: tuple[str, ...] = tuple(
            n for n, _, _ in lifted
        ) if is_entry else ()
        provider = SequentialCILTokenProvider(
            callables,
            method_token_offset=offset,
            closure_names=closure_names,
            closure_free_var_counts=closure_free_var_counts,
        )

        try:
            artifact = lower_ir_to_cil_bytecode(
                opt_ir,
                CILBackendConfig(
                    # TW04 Phase 4e: use the global shared call_register_count
                    # so all modules declare the same parameter count.  See
                    # the "Shared call_register_count" comment above.
                    call_register_count=global_local_count,
                    closure_free_var_counts=closure_free_var_counts,
                    closure_explicit_arities=closure_explicit_arities,
                    syscall_arg_reg=_CLR_SYSCALL_ARG_REG,
                    extra_callable_labels=tuple(exports),
                    external_method_tokens=ext_tokens,
                    # TW04 Phase 4c: use inline .NET API calls for host syscalls
                    # so SYSCALL 1/2/10 work on real dotnet without a helper lib.
                    inline_host_syscalls=True,
                ),
                token_provider=provider,
            )
        except Exception as exc:
            raise ClrPackageError(
                "lower-cil",
                f"module {mod.name!r}: {exc}",
                exc,
            ) from exc

        module_results.append(
            ModuleClrCompileResult(
                module_name=mod.name,
                type_name=module_name_to_clr_type(mod.name),
                ir=opt_ir,
                artifact=artifact,
                callable_names=callables,
            )
        )

    # ── Merge into one CILProgramArtifact ────────────────────────────────
    #
    # Entry module's artifact becomes the "main" artifact (methods → main
    # TypeDef).  Each dep module's methods are packaged as a CILTypeArtifact
    # in ``extra_types``.  The assembly writer emits them in declaration order,
    # which matches our cumulative MethodDef token assignment.

    entry_result = module_results[0]
    dep_results = module_results[1:]

    # Start with the entry module's extra_types (closures, heap types).
    merged_extra_types = list(entry_result.artifact.extra_types)

    for dep_result in dep_results:
        type_name = dep_result.type_name
        dep_methods = dep_result.artifact.methods
        merged_extra_types.append(
            CILTypeArtifact(
                name=type_name,
                namespace="",
                is_interface=False,
                extends="System.Object",
                implements=(),
                fields=(),
                methods=dep_methods,
            )
        )

    # Build the combined artifact by replacing extra_types on the entry artifact.
    from dataclasses import replace as _dc_replace

    combined_artifact = _dc_replace(
        entry_result.artifact,
        extra_types=tuple(merged_extra_types),
    )

    try:
        cli_config = CLIAssemblyConfig(
            assembly_name=assembly_name,
            module_name=f"{assembly_name}.exe",
            type_name=entry_result.type_name,
        )
        written = write_cli_assembly(combined_artifact, cli_config)
    except Exception as exc:
        raise ClrPackageError("write-pe", str(exc), exc) from exc

    return MultiModuleClrResult(
        entry_module=entry_module,
        module_results=tuple(module_results),
        assembly_bytes=written.assembly_bytes,
        entry_type_name=entry_result.type_name,
    )


def run_modules(
    modules: list[ResolvedModule],
    *,
    entry_module: str,
    assembly_name: str = "TwigProgram",
    timeout_seconds: int = 30,
) -> MultiModuleClrExecutionResult:
    """Compile and execute a multi-module Twig program on the real ``dotnet``
    runtime.

    Calls ``compile_modules``, writes the single ``.exe`` to a temp directory
    alongside its ``runtimeconfig.json``, runs ``dotnet <name>.exe``, and
    returns the result.

    The process **exit code** is the program's return value (same convention
    as single-module ``run_source``).
    """
    compilation = compile_modules(
        modules,
        entry_module=entry_module,
        assembly_name=assembly_name,
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
    return MultiModuleClrExecutionResult(
        compilation=compilation,
        stdout=proc.stdout,
        stderr=proc.stderr,
        returncode=proc.returncode,
    )
