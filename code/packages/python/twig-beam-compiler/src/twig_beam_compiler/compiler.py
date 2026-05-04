"""Twig source → ``.beam`` end-to-end compiler.

Pipeline
========
::

    Twig source
        ↓ parse + extract     (twig package)
    typed AST
        ↓ Compiler.compile()  (this module — Twig → compiler-IR)
    IrProgram
        ↓ ir-optimizer
    IrProgram (optimised)
        ↓ lower_ir_to_beam    (ir-to-beam)
    BEAMModule
        ↓ encode_beam         (beam-bytecode-encoder)
    .beam bytes
        ↓ erl -noshell -eval ...
    program output (printed by io:format)

TW03 Phase 1 surface (this version)
===================================

Mirrors the twig-jvm-compiler / twig-clr-compiler frontends so
all three real-runtime Twig tracks share IR shape:

- Integer literals (positive)
- Binary arithmetic: ``+``, ``-``, ``*``, ``/``
- Comparison: ``=``, ``<``, ``>``
- ``let`` (single + multi binding), ``begin``, ``if``
- Top-level ``define`` for both functions and value constants
- Function calls (incl. nested) and **recursion**

The contract: ``run_source`` invokes
``erl -noshell -eval 'io:format("~p~n", [<module>:main()])'``,
printing the value of the program's final top-level expression
to stdout.  ``BeamRunResult.stdout`` carries that printed value.

Register convention (shared with JVM/CLR compilers)
===================================================

* ``register 0`` — scratch.
* ``register 1`` — function return value / HALT result.
* ``registers 2..N`` — function parameter slots
  (param ``i`` → register ``_REG_PARAM_BASE + i = 2 + i``).
* ``registers 10..`` — compiler-allocated holding registers for
  intermediate values.  Using a high base keeps them clear of
  the param window so nested calls don't clobber.

ir-to-beam translates this convention to the BEAM x/y register
files (see that package's docs).
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
# TW04 Phase 4f — multi-module result types
# ---------------------------------------------------------------------------
#
# Each Twig module compiles to one ``.beam`` file.  At runtime all files are
# loaded into the same ``erl`` session — BEAM modules talk to each other via
# plain remote-call MFAs (``a_math:add/2`` etc.) without any shared memory
# trick.  The execution model is simpler than JVM (no shared TwigRuntime
# register file) because BEAM already has per-process heaps and y-register
# stacks that survive across calls.


@dataclass(frozen=True)
class ModuleBeamCompileResult:
    """Compilation artefact for one module in a multi-module BEAM build.

    Attributes
    ----------
    module_name:
        The Twig module name (e.g. ``"a/math"``).
    beam_module:
        The Erlang module atom name (e.g. ``"a_math"``).  This is what
        the ``.beam`` file declares as ``-module(a_math)`` and what the
        caller uses in remote-call MFAs.
    ir:
        The optimised ``IrProgram`` that was lowered to ``beam_bytes``.
    beam_bytes:
        The raw ``.beam`` file bytes for this module.  Write these to
        ``<beam_module>.beam`` and load with ``erl -pa <dir>``.
    exports:
        The Twig-level function names exported by this module's
        ``(module ... (export ...))`` declaration.  Used by callers to
        validate cross-module call sites and by the BEAM backend to
        force-include those functions as callable regions.
    """

    module_name: str
    beam_module: str
    ir: IrProgram
    beam_bytes: bytes
    exports: tuple[str, ...]


@dataclass(frozen=True)
class MultiModuleBeamResult:
    """Aggregate result of compiling a set of Twig modules to BEAM.

    Attributes
    ----------
    entry_module:
        The Twig entry module name (e.g. ``"user/hello"``).
    module_results:
        One ``ModuleBeamCompileResult`` per compiled module, in the order
        they were lowered.  Excludes the synthetic ``host`` module.
    entry_beam_module:
        The Erlang module atom for the entry module — used as the target
        of the ``erl -eval '<entry>:main()'`` invocation.
    """

    entry_module: str
    module_results: tuple[ModuleBeamCompileResult, ...]
    entry_beam_module: str


@dataclass(frozen=True)
class MultiModuleBeamExecutionResult:
    """Compile + real-``erl`` execution result for a multi-module program.

    Attributes
    ----------
    compilation:
        The full compilation result (all modules, all IR).
    stdout:
        Raw bytes written to stdout by the ``erl`` process.
    stderr:
        Raw bytes written to stderr by the ``erl`` process.
    returncode:
        The ``erl`` process exit code.
    """

    compilation: MultiModuleBeamResult
    stdout: bytes
    stderr: bytes
    returncode: int


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
# remaining rejected set is shrunk to opcodes still without a BEAM
# lowering.
_V1_REJECTED_BUILTINS: frozenset[str] = frozenset(
    {"number?", "print"}
)

# TW03 Phase 3e — Lisp heap-primitive builtins.  Each entry maps a
# builtin name to its IR opcode + arity so the apply-site can
# generate a uniform call sequence.  Phase 3d's BEAM backend
# lowering will turn these into native put_list / get_hd / get_tl /
# is_nil / is_nonempty_list / is_atom / move {atom, idx} opcodes.
_HEAP_BUILTINS: dict[str, tuple[IrOp, int]] = {
    "cons":    (IrOp.MAKE_CONS, 2),
    "car":     (IrOp.CAR, 1),
    "cdr":     (IrOp.CDR, 1),
    "null?":   (IrOp.IS_NULL, 1),
    "pair?":   (IrOp.IS_PAIR, 1),
    "symbol?": (IrOp.IS_SYMBOL, 1),
}


# Register convention — see module docstring.
_REG_HALT_RESULT: Final = 1
_REG_PARAM_BASE: Final = 2
_REG_HOLDING_BASE: Final = 10


# Strict allowlist for ``module_name``.  Used as the on-disk
# ``.beam`` filename AND interpolated into the ``erl -eval``
# Erlang source string in ``run_source`` — so untrusted values
# would let a caller execute arbitrary Erlang.
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
    """Mutable state during compilation of one IR region."""

    locals_: dict[str, IrRegister]
    next_holding: int = _REG_HOLDING_BASE


# ---------------------------------------------------------------------------
# Compiler
# ---------------------------------------------------------------------------


class _Compiler:
    """Walk the typed Twig AST → ``IrProgram`` with one region per
    top-level function plus a synthesised ``main`` for top-level
    expressions.  Mirrors twig-jvm-compiler / twig-clr-compiler.
    """

    def __init__(self) -> None:
        self._program = IrProgram(entry_label="main")
        self._gen = IDGenerator()
        self._value_consts: dict[str, int] = {}
        self._fn_params: dict[str, list[str]] = {}
        # Program-wide label counter.  Per-region counters would
        # produce duplicate ``_else_0`` etc. across functions, and
        # the BEAM backend rejects duplicate label names just like
        # the CIL backend does.
        self._next_label_id = 0
        # TW03 Phase 2: lifted lambdas.  Each anonymous Lambda
        # encountered during expression compilation gets a fresh
        # name like ``_lambda_0`` and is appended here as
        # (lifted_name, captures, lambda_node) so we can emit its
        # body region after the user functions.  The closure
        # value-side machinery (MAKE_CLOSURE / APPLY_CLOSURE) is
        # emitted at the use site.
        self._lifted_lambdas: list[tuple[str, list[str], Lambda]] = []
        self._lambda_counter = 0
        # TW04 Phase 4f: cross-module call arities.  Each time we emit a
        # cross-module IrOp.CALL we record the actual call-site arity here.
        # Unlike CLR/JVM, BEAM ``call_ext`` requires the EXACT arity of the
        # remote function — so we track it at the call site where the caller
        # knows how many arguments they are passing.
        self._cross_module_arities: dict[str, int] = {}

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

        # Lifted-lambda regions go AFTER ``main`` so ``main`` is
        # unambiguously the program's entry point in the IR
        # instruction stream.  ``_compile_expr`` may have appended
        # to ``self._lifted_lambdas`` while compiling earlier
        # functions / main; lifted lambdas can also be nested
        # (lambda inside lambda) so the list grows during this
        # final pass — iterate by index.
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
        # holding register.  Mirrors the JVM01 paired fix +
        # twig-clr-compiler convention.  ir-to-beam handles the
        # x→y register dance internally (see its docstring), so
        # this front-end stays IR-shaped identically across all
        # three Twig backends.
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

        msg = f"unsupported Twig form for BEAM v1: {type(expr).__name__}"
        raise TwigCompileError(msg)

    def _compile_if(self, expr: If, ctx: _FnCtx) -> IrRegister:
        cond = self._compile_expr(expr.cond, ctx)
        else_label = self._fresh_label("else")
        end_label = self._fresh_label("endif")
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
        # ``APPLY_CLOSURE`` and dispatches via ``erlang:apply/3``.
        if isinstance(expr.fn, VarRef):
            name = expr.fn.name

            # TW04 Phase 4f: module-qualified cross-module call.
            # Any name with an interior ``/`` (not at position 0 or
            # the last character) is a cross-module reference, e.g.
            # ``"a/math/add"`` or ``"stdlib/io/write"``.
            # We emit each argument into a holding register, move them
            # to the Twig param-slot registers (r2..rN), then emit
            # ``IrOp.CALL IrLabel("a/math/add")``.  The BEAM backend
            # detects the ``/`` in the label name and lowers this to a
            # ``call_ext N a_math:add/N`` remote call.
            #
            # Note: ``host/*`` calls (``host/write-byte`` etc.) are not
            # handled by the BEAM backend the same way as JVM/CLR — the
            # BEAM frontend does not support host syscalls (they require
            # a separate runtime not present in the BEAM version).
            _slash = name.find("/")
            if _slash > 0 and _slash < len(name) - 1:
                # Cross-module call — emit args into param slots then CALL.
                arg_regs_xm = [self._compile_expr(a, ctx) for a in expr.args]
                for i, src in enumerate(arg_regs_xm):
                    self._emit_move(IrRegister(_REG_PARAM_BASE + i), src)
                self._emit(IrOp.CALL, IrLabel(name))
                # Record the call-site arity for the BEAM backend.  Unlike
                # CLR/JVM, BEAM call_ext must embed the EXACT remote arity
                # in the MFA triple so the loader can find the function.
                self._cross_module_arities[name] = len(expr.args)
                dest = self._fresh_holding(ctx)
                self._emit_move(dest, IrRegister(_REG_HALT_RESULT))
                return dest

            if name in _V1_REJECTED_BUILTINS:
                raise TwigCompileError(
                    f"builtin {name!r} is not yet supported by the BEAM "
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
    # Closure compilation (TW03 Phase 2)
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

        Captures-first parameter layout (matching the
        ``[Caps... | Args...]`` arglist that ir-to-beam stages
        when calling via apply/3): the lifted function's IR
        register layout is

            r2..r{1+num_free}                    — captures
            r{2+num_free}..r{1+num_free+arity}   — explicit params
            r{10+}                               — body holding regs
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

    def _fresh_label(self, prefix: str) -> str:
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
        compiler = _Compiler()
        raw_ir = compiler.compile(twig_program)
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

    # Build arity overrides for every top-level function the
    # compiler discovered.  ``ir-to-beam`` needs these to emit
    # the right ``call N, label`` arity at each call site and to
    # declare the right parameter count per function.  For
    # lifted lambdas, ``arity_overrides`` carries the EXPLICIT
    # arity (the ``(lambda (x) ...)``-level count) — the BEAM
    # backend widens this internally to ``num_free + explicit``
    # using ``closure_free_var_counts``.
    arity_overrides = {
        name: len(params) for name, params in compiler._fn_params.items()  # noqa: SLF001
    }
    closure_free_var_counts: dict[str, int] = {}
    for lifted_name, captures, lam in compiler._lifted_lambdas:  # noqa: SLF001
        arity_overrides[lifted_name] = len(lam.params)
        closure_free_var_counts[lifted_name] = len(captures)

    try:
        beam_module = lower_ir_to_beam(
            optimized_ir,
            BEAMBackendConfig(
                module_name=module_name,
                arity_overrides=arity_overrides,
                closure_free_var_counts=closure_free_var_counts,
            ),
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
    to call ``<module>:main()`` and ``io:format`` its return
    value to stdout, then ``init:stop()``.
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


# ---------------------------------------------------------------------------
# TW04 Phase 4f — multi-module public API
# ---------------------------------------------------------------------------


def module_name_to_beam_module(name: str) -> str:
    """Map a Twig module name to its Erlang module atom name.

    Erlang module names (atoms) may not contain ``/`` — it's not a
    valid atom character without quoting, and BEAM module atoms map
    directly to ``.beam`` filenames.  We replace every ``/`` with
    ``_`` to produce a valid, lowercase Erlang atom.

    The mapping is injective for all legal Twig module names:
    distinct module names always yield distinct beam atoms.

    Examples
    --------
    >>> module_name_to_beam_module("a/math")
    'a_math'
    >>> module_name_to_beam_module("user/hello")
    'user_hello'
    >>> module_name_to_beam_module("stdlib/io")
    'stdlib_io'
    >>> module_name_to_beam_module("mymodule")
    'mymodule'
    """
    return name.replace("/", "_")


def _compile_one_beam_module(
    module: ResolvedModule,
    *,
    optimize: bool,
    external_function_arities: dict[str, int],
) -> tuple[ModuleBeamCompileResult, dict[str, int]]:
    """Compile one ``ResolvedModule`` to a ``ModuleBeamCompileResult``.

    This is the per-module lowering step for ``compile_modules``.  Each
    module compiles independently.  Unlike CLR/JVM (which use a fixed-width
    parameter frame for all calls), BEAM ``call_ext`` requires the exact
    arity of the remote function — supplied via ``external_function_arities``.

    Parameters
    ----------
    module:
        The resolved Twig module to compile.
    optimize:
        Whether to run the IR optimizer before lowering.
    external_function_arities:
        Map of cross-module label → actual remote arity, e.g.
        ``{"a/math/add": 2, "b/utils/double": 1}``.  Passed through to
        ``BEAMBackendConfig`` so the backend emits the right MFA arity in
        each ``call_ext`` instruction.

    Returns
    -------
    tuple[ModuleBeamCompileResult, dict[str, int]]
        The compiled artefact and this module's cross-module call arities
        (from ``_Compiler._cross_module_arities``), so the caller can
        accumulate them for the next module in the build.
    """
    beam_module = module_name_to_beam_module(module.name)

    try:
        compiler = _Compiler()
        raw_ir = compiler.compile(module.program)
    except TwigCompileError:
        raise
    except Exception as exc:  # pragma: no cover
        raise BeamPackageError(
            f"module:{module.name}:ir-emit", str(exc), exc
        ) from exc

    if optimize:
        optimization = IrOptimizer.default_passes().optimize(raw_ir)
        optimized_ir = optimization.program
    else:
        optimized_ir = raw_ir

    # Gather closure data from the compiler (same as compile_source).
    arity_overrides: dict[str, int] = {
        name: len(params) for name, params in compiler._fn_params.items()  # noqa: SLF001
    }
    closure_free_var_counts: dict[str, int] = {}
    for lifted_name, captures, lam in compiler._lifted_lambdas:  # noqa: SLF001
        arity_overrides[lifted_name] = len(lam.params)
        closure_free_var_counts[lifted_name] = len(captures)

    # Exports declared in the (module ...) declaration.  These functions are
    # only called from OTHER modules (no local CALL site), so they must be
    # force-included as callable regions by the BEAM backend.
    mod_decl = module.program.module
    exports: tuple[str, ...] = tuple(mod_decl.exports) if mod_decl else ()

    # Cross-module arities discovered in THIS module's source (from _Compiler).
    # Add these to the external_function_arities dict for future callers.
    this_module_xm_arities = dict(compiler._cross_module_arities)  # noqa: SLF001

    try:
        beam_module_obj = lower_ir_to_beam(
            optimized_ir,
            BEAMBackendConfig(
                module_name=beam_module,
                arity_overrides=arity_overrides,
                closure_free_var_counts=closure_free_var_counts,
                # Force-include exported functions so they become callable
                # regions even when no local CALL targets them.
                extra_callable_labels=exports,
                # Cross-module call arities: maps "a/math/add" → 2 so the
                # backend emits call_ext 2 {a_math,add,2} correctly.
                external_function_arities=external_function_arities,
            ),
        )
        beam_bytes = encode_beam(beam_module_obj)
    except Exception as exc:
        raise BeamPackageError(
            f"module:{module.name}:lower-beam", str(exc), exc
        ) from exc

    result = ModuleBeamCompileResult(
        module_name=module.name,
        beam_module=beam_module,
        ir=optimized_ir,
        beam_bytes=beam_bytes,
        exports=exports,
    )
    return result, this_module_xm_arities


def _compute_global_call_register_count(
    modules: list[ResolvedModule],
    *,
    optimize: bool,
) -> int:
    """Compute the global maximum ``call_register_count`` across all modules.

    Each module's local maximum register index determines the number of
    y-register slots its functions use.  The global maximum is the floor
    for ALL module's call sites — including cross-module remote calls —
    so that every callee's parameter window is wide enough.

    This mirrors CLR's ``global_local_count`` computation.  The value is
    ``max(max_reg_index + 1, _REG_PARAM_BASE)`` across all modules, with
    a hard floor of ``_REG_PARAM_BASE`` (= 2) to guarantee at least the
    return-value and one-parameter slots exist.

    Parameters
    ----------
    modules:
        All resolved modules (including ``host``, which is skipped).
    optimize:
        Whether the IR optimizer will run — must match the actual compile
        so that register indices are consistent.

    Returns
    -------
    int
        The global call_register_count floor.
    """
    global_max = _REG_PARAM_BASE  # floor: at least 2 (halt-result + 1 param)

    for module in modules:
        if module.name == HOST_MODULE_NAME:
            continue
        try:
            compiler = _Compiler()
            raw_ir = compiler.compile(module.program)
        except TwigCompileError:
            raise
        except Exception:  # pragma: no cover
            continue

        if optimize:
            opt = IrOptimizer.default_passes().optimize(raw_ir)
            ir = opt.program
        else:
            ir = raw_ir

        for instr in ir.instructions:
            for operand in instr.operands:
                if isinstance(operand, IrRegister):
                    # Only count param/holding registers; index 0 (scratch)
                    # and 1 (halt-result) are always present.
                    global_max = max(global_max, operand.index + 1)

    return global_max


def compile_modules(
    modules: list[ResolvedModule],
    entry_module: str,
    *,
    optimize: bool = True,
) -> MultiModuleBeamResult:
    """Compile a set of Twig modules to individual BEAM ``.beam`` files.

    ``modules`` must be in topological order (deps before importers), as
    returned by :func:`twig.resolve_modules`.  The synthetic ``host``
    module is automatically skipped — the BEAM front-end does not support
    host syscalls (write-byte / read-byte / exit); include only modules
    with pure Twig logic.

    Each non-``host`` module compiles to one ``.beam`` file.  All files
    must be loaded into the same ``erl`` session for cross-module calls
    to resolve.

    Parameters
    ----------
    modules:
        Topologically ordered list of resolved modules.
    entry_module:
        The Twig module name whose ``main`` function is the program's
        entry point (invoked by ``erl -eval '<beam_module>:main()'``).
    optimize:
        Pass ``True`` (the default) to run the IR optimizer before
        lowering each module.

    Returns
    -------
    MultiModuleBeamResult
        One ``ModuleBeamCompileResult`` per non-``host`` module.

    Raises
    ------
    BeamPackageError
        If any module fails to compile.
    ValueError
        If ``entry_module`` does not appear in ``modules``.
    """
    module_names = {m.name for m in modules}
    if entry_module not in module_names:
        raise ValueError(
            f"entry_module {entry_module!r} not found in the provided "
            f"module list.  Available: {sorted(module_names)}"
        )

    # Build the cross-module arities dict incrementally across modules.
    # Unlike CLR/JVM (which use a fixed-width parameter frame), BEAM's
    # ``call_ext`` requires the exact arity of the remote function.
    #
    # We compile modules in topological order (deps before importers).
    # As each dep module compiles, it exposes its exported functions'
    # arities.  These arities come from the dep module's ``arity_overrides``
    # (which record each declared function's parameter count).  When the
    # entry module compiles, it can look up "a/math/add" → 2 to emit the
    # correct ``call_ext 2 {a_math, add, 2}`` instruction.
    #
    # We seed the dict with arities from dep module exports (computed below)
    # and accumulate cross-module arities reported by each compiled module.
    all_external_arities: dict[str, int] = {}

    # Pre-pass: seed from dep module export arities.  Each dep module
    # declares ``(define (add x y) ...)`` which the _Compiler records in
    # ``_fn_params``.  We need those arities BEFORE compiling the entry
    # module so the entry module's BEAMBackendConfig has them available.
    #
    # Strategy: compile each module in order.  When compiling a dep module,
    # also record its exported function arities into all_external_arities
    # so that subsequent (importer) modules see them.  Since topological
    # order guarantees deps come before importers, this single pass suffices.

    results: list[ModuleBeamCompileResult] = []
    for module in modules:
        if module.name == HOST_MODULE_NAME:
            # The synthetic host module has no Twig source to lower to BEAM.
            # Its exports (write-byte, etc.) are not supported by the BEAM
            # front-end; skip silently.
            continue
        result, xm_arities = _compile_one_beam_module(
            module,
            optimize=optimize,
            external_function_arities=all_external_arities,
        )
        results.append(result)
        # Accumulate this module's cross-module call arities (for modules
        # that import THIS module or call cross-module functions transitively).
        all_external_arities.update(xm_arities)
        # Also record the module's own exported function arities under the
        # qualified key ``"<module_name>/<fn_name>"`` so that subsequent
        # importer modules can find them.  The arity comes from the dep
        # module's own IR labels which were compiled with the correct arity.
        if result.exports:
            # Re-derive arities from what we compiled.  The dep module's
            # ``arity_overrides`` were passed to BEAM; we recover them here
            # by re-scanning the IR for LABEL operands and counting param-slot
            # moves in the function prologue.  Simpler: just re-compile
            # a quick _Compiler pass to get fn_params.
            try:
                _scanner = _Compiler()
                _scanner.compile(module.program)
                for fn_name in result.exports:
                    if fn_name in _scanner._fn_params:  # noqa: SLF001
                        qualified = f"{module.name}/{fn_name}"
                        all_external_arities[qualified] = len(
                            _scanner._fn_params[fn_name]  # noqa: SLF001
                        )
            except TwigCompileError:
                pass  # best-effort; compile step above will raise for real

    return MultiModuleBeamResult(
        entry_module=entry_module,
        module_results=tuple(results),
        entry_beam_module=module_name_to_beam_module(entry_module),
    )


def run_modules(
    modules: list[ResolvedModule],
    entry_module: str,
    *,
    optimize: bool = True,
    tmp_dir: Path | None = None,
    timeout_seconds: int = 30,
) -> MultiModuleBeamExecutionResult:
    """Compile a multi-module Twig program and execute it on real ``erl``.

    Writes each module's ``.beam`` file to a temporary directory, then
    invokes::

        erl -noshell -pa <tmpdir> -eval '<entry_beam_module>:main()' \\
            -eval 'erlang:halt()'

    The entry module's ``main()`` function is expected to return an integer
    which the ``erlang:halt()`` call uses as the process exit code.

    Parameters
    ----------
    modules:
        Topologically ordered list of resolved modules (from
        :func:`twig.resolve_modules`).
    entry_module:
        The Twig module name whose ``main`` function is the entry point.
    optimize:
        Whether to run the IR optimizer before lowering.
    tmp_dir:
        Optional directory to write ``.beam`` files into.  If ``None``
        a fresh ``tempfile.TemporaryDirectory`` is used and cleaned up
        automatically.
    timeout_seconds:
        Maximum wall-clock time for the ``erl`` invocation.

    Returns
    -------
    MultiModuleBeamExecutionResult
        The compile result plus subprocess stdout/stderr/exit code.

    Raises
    ------
    BeamPackageError
        If any module fails to compile.
    RuntimeError
        If no ``erl`` binary is on PATH.
    ValueError
        If ``entry_module`` does not appear in ``modules``.
    """
    if not erl_available():
        raise RuntimeError(
            "No erl binary found on PATH.  "
            "Install Erlang/OTP (e.g. `brew install erlang`) to run "
            "multi-module Twig programs on BEAM."
        )

    compilation = compile_modules(modules, entry_module, optimize=optimize)
    entry_beam = compilation.entry_beam_module

    def _run_in(tmp: str) -> subprocess.CompletedProcess[bytes]:
        """Write all .beam files and invoke erl."""
        for mr in compilation.module_results:
            beam_path = Path(tmp) / f"{mr.beam_module}.beam"
            beam_path.write_bytes(mr.beam_bytes)
        # Explicitly load EVERY compiled module before calling main().
        #
        # BEAM's ``-pa <dir>`` adds the directory to the code path, but
        # lazy auto-loading on first remote-call is not reliable in
        # ``-noshell`` mode when the target function is reached via a
        # ``call_ext`` before any explicit ``code:load_file`` or
        # ``code:ensure_loaded``.  In practice, calling the entry module
        # before its deps are loaded causes a SIGSEGV (-11) or
        # ``{undef,[{a_math,add,2,[]}]}`` crash.  The fix is simple:
        # emit one ``{module,_} = code:load_file(<atom>)`` statement per
        # module, in compilation order (deps first), before the
        # ``main()`` call.  That guarantees all MFA imports are resolved
        # at call_ext dispatch time.
        #
        # Note: ``code:load_file/1`` takes the module name as an atom
        # (without quotes), which in Erlang's eval string means we write
        # the bare atom name — exactly what ``mr.beam_module`` contains.
        load_stmts = ", ".join(
            f"{{module,_}} = code:load_file({mr.beam_module})"
            for mr in compilation.module_results
        )
        eval_expr = (
            f"{load_stmts},"
            f" Result = {entry_beam}:main(),"
            " erlang:halt(Result)."
        )
        return subprocess.run(
            ["erl", "-noshell", "-pa", tmp, "-eval", eval_expr],
            capture_output=True,
            timeout=timeout_seconds,
            check=False,
        )

    if tmp_dir is not None:
        proc = _run_in(str(tmp_dir))
    else:
        with tempfile.TemporaryDirectory() as tmp:
            proc = _run_in(tmp)

    return MultiModuleBeamExecutionResult(
        compilation=compilation,
        stdout=proc.stdout,
        stderr=proc.stderr,
        returncode=proc.returncode,
    )
