"""Twig → JVM compiler — TW02 v1.

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
        ↓ lower_ir_to_jvm_class_file
    .class bytes
        ↓ java -cp <dir> <ClassName>
    program output (a single byte on stdout)

Why "first non-Python target"
=============================
JVM gets the first real-runtime target slot because the in-house
``ir-to-jvm-class-file`` already produces real-``java``-compatible
class files (see ``test_oct_8bit_e2e.py`` in that package — it
runs the compiler's output through real ``java`` and asserts on
JVM stdout).  CLR is blocked behind ``cli-assembly-writer``
conformance work (CLR01).

Calling convention notes (see TW02 spec)
========================================
The JVM backend emits one ``invokestatic`` per ``IrOp.CALL``
with descriptor ``()I`` — no arguments, returns int.  Caller and
callee communicate through a shared register array provided by
the runtime.  Param ``i`` of every function lives at register
``2 + i`` (registers 0 and 1 are reserved scratch / HALT-result).

Nested calls work because the compiler evaluates each argument
into a *holding* register (index ≥ 10) before moving values into
the param-slot registers right at the call site — that way
``(f (g x) y)`` doesn't have ``g``'s arg setup stomp on ``f``'s
arg-0 slot.
"""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

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
from ir_to_jvm_class_file import (
    TWIG_RUNTIME_BINARY_NAME,
    JvmBackendConfig,
    JVMClassArtifact,
    JVMMultiClassArtifact,
    build_runtime_class_artifact,
    lower_ir_to_jvm_class_file,
    lower_ir_to_jvm_classes,
    write_class_file,
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
from twig.module_resolver import HOST_MODULE_NAME, ResolvedModule

# ---------------------------------------------------------------------------
# Result types
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class PackageResult:
    """Aggregated artefacts from one compile run.

    JVM02 Phase 2d: ``multi_class_artifact`` is populated when the
    program contains closures (so it ships the auto-generated
    ``Closure`` interface + per-lambda ``Closure_<name>``
    subclasses).  Non-closure programs leave it ``None`` and
    ``artifact`` carries the single main class as before.
    """

    source: str
    class_name: str
    ast: Program
    raw_ir: IrProgram
    optimization: OptimizationResult
    optimized_ir: IrProgram
    artifact: JVMClassArtifact
    class_bytes: bytes
    multi_class_artifact: JVMMultiClassArtifact | None = None


@dataclass(frozen=True)
class ExecutionResult:
    """Compilation result + real-``java`` invocation output."""

    compilation: PackageResult
    stdout: bytes
    stderr: bytes
    returncode: int


class PackageError(TwigError):
    """Stage-tagged failure during compilation or execution."""

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


# TW04 Phase 4d — multi-module result types
# -----------------------------------------
#
# ``compile_modules`` accepts a topologically ordered list of
# ``ResolvedModule`` objects (as produced by ``twig.resolve_modules``)
# and compiles every non-``host`` module to a JVM class.  All module
# classes share one ``TwigRuntime`` register file so that cross-module
# function calls see a consistent register state.


@dataclass(frozen=True)
class ModuleCompileResult:
    """Compilation result for one module in a multi-module build.

    Attributes
    ----------
    module_name:
        The Twig module name (e.g. ``"a/math"``).
    jvm_class_name:
        The JVM internal class name (e.g. ``"a/math"``).  For Phase 4d
        this is the identity of ``module_name`` — forward slashes already
        serve as JVM package separators.
    ir_program:
        The optimised ``IrProgram`` that was lowered to ``artifact``.
    artifact:
        The main JVM class artifact for this module.  If the module
        uses closures or heap primitives the full set of supporting
        classes lives in ``multi_class_artifact``.
    is_entry:
        ``True`` only for the entry module.  The entry module's artifact
        includes a ``main([Ljava/lang/String;)V`` wrapper.  All other
        modules have ``emit_main_wrapper=False``.
    multi_class_artifact:
        Populated when the module's IR requires closure or heap-primitive
        support classes (Closure interface, per-lambda subclasses, Cons,
        Symbol, Nil).  ``None`` for modules that emit a single plain class.
    """

    module_name: str
    jvm_class_name: str
    ir_program: IrProgram
    artifact: JVMClassArtifact
    is_entry: bool
    multi_class_artifact: JVMMultiClassArtifact | None = None


@dataclass(frozen=True)
class MultiModuleResult:
    """Aggregate result of compiling a set of Twig modules together.

    Attributes
    ----------
    runtime_artifact:
        The shared ``TwigRuntime`` class that owns ``int[] __ca_regs``
        and ``Object[] __ca_objregs``.  Must be included in the JAR so
        every generated module class can ``getstatic`` its register file.
    modules:
        One ``ModuleCompileResult`` per compiled module (excluding
        ``host``), in topological order — dependencies before importers.
    entry_class_name:
        The JVM class name of the entry module, suitable for use as
        ``Main-Class`` in a JAR manifest.
    """

    runtime_artifact: JVMClassArtifact
    modules: list[ModuleCompileResult]
    entry_class_name: str


@dataclass(frozen=True)
class MultiModuleExecutionResult:
    """Compilation result + real-``java`` invocation output for a multi-module
    program."""

    multi: MultiModuleResult
    stdout: bytes
    stderr: bytes
    exit_code: int


# ---------------------------------------------------------------------------
# Builtins recognised in v1
# ---------------------------------------------------------------------------


# Each entry maps a Twig builtin name to the binary ``IrOp`` for its
# single-instruction emission.  v1 only supports the binary ones.
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
# remaining rejected set is shrunk to opcodes still without a JVM
# lowering: ``number?`` (would need a tagged-int discriminator the
# Phase 3b heap pool doesn't have yet) and ``print`` (TW04
# territory).
_V1_REJECTED_BUILTINS: frozenset[str] = frozenset(
    {"number?", "print"}
)

# TW03 Phase 3e — Lisp heap-primitive builtins.  Each entry maps a
# builtin name to its IR opcode + arity so the apply-site can
# generate a uniform call sequence.  ``cons`` is the only 2-arg
# heap op; the rest are 1-arg.  ``IS_NULL`` / ``IS_PAIR`` /
# ``IS_SYMBOL`` write 0/1 into an int register so the result feeds
# straight into BRANCH_Z (no boxing needed).
_HEAP_BUILTINS: dict[str, tuple[IrOp, int]] = {
    "cons":    (IrOp.MAKE_CONS, 2),
    "car":     (IrOp.CAR, 1),
    "cdr":     (IrOp.CDR, 1),
    "null?":   (IrOp.IS_NULL, 1),
    "pair?":   (IrOp.IS_PAIR, 1),
    "symbol?": (IrOp.IS_SYMBOL, 1),
}

# Convention shared with the JVM backend's helper register array:
#   register 0  — scratch (also the SYSCALL-arg slot for host syscalls)
#   register 1  — HALT result / function return value
#   register 2..N — function parameter slots (param i → register 2 + i)
#   register 10..  — compiler-allocated holding registers for intermediate
#                    values; using a high base keeps them clear of
#                    parameter slots, so nested calls don't clobber.
_REG_SCRATCH = 0
_REG_HALT_RESULT = 1
_REG_PARAM_BASE = 2
_REG_HOLDING_BASE = 10

# Platform-independent syscall numbers shared by all backends.
# These numbers are the de-facto convention established by the CLR
# backend; the JVM backend's ``__ca_syscall`` helper now also handles
# them.  When a new host export is added, a new entry goes here AND
# in the backend's syscall handler, AND in ``twig/compiler.py``.
_HOST_SYSCALLS: dict[str, int] = {
    "host/write-byte": 1,  # write one byte to stdout
    "host/read-byte":  2,  # read one byte from stdin; return -1 on EOF
    "host/exit":       10, # exit the process with the given code
}


# ---------------------------------------------------------------------------
# Per-function compilation context
# ---------------------------------------------------------------------------


@dataclass
class _FnCtx:
    """Mutable state during compilation of one IR function region.

    A ``_FnCtx`` is created either for the synthetic ``_start``
    region (top-level expressions of the program) or for each
    user-defined function discovered in the AST.
    """

    # Map of *local* names to the register they live in.  Populated
    # from function parameters at function entry, extended as
    # ``let`` bindings introduce new names.
    locals_: dict[str, IrRegister]

    # Next holding-register index to allocate for intermediate values.
    next_holding: int = _REG_HOLDING_BASE


# ---------------------------------------------------------------------------
# Compiler
# ---------------------------------------------------------------------------


class _Compiler:
    """Walks the typed Twig AST and emits one ``IrProgram``.

    A program is one or more *callable regions*: the synthesised
    ``_start`` region (top-level expressions) and one region per
    top-level ``(define (f ...) ...)``.  The JVM backend emits each
    region as a separate ``invokestatic`` target.
    """

    def __init__(self) -> None:
        self._program = IrProgram(entry_label="_start")
        self._gen = IDGenerator()

        # Top-level value defines folded to compile-time constants.
        # ``(define x 42)`` → ``_value_consts["x"] = 42``; references
        # to ``x`` anywhere in the program emit ``LOAD_IMM`` of 42.
        # (v1 only supports literal RHS for value defines.)
        self._value_consts: dict[str, int] = {}

        # Names of top-level function defines and their parameter
        # names.  Used to (a) decide direct-call dispatch at apply
        # sites and (b) lay out parameters into the shared register
        # convention.
        self._fn_params: dict[str, list[str]] = {}

        # JVM02 Phase 2d: lifted lambdas.  Each anonymous Lambda
        # encountered during expression compilation gets a fresh
        # name like ``_lambda_0`` and is appended here as
        # (lifted_name, captures, lambda_node) so we can emit its
        # body region after the user functions.  The closure
        # value-side machinery (MAKE_CLOSURE / APPLY_CLOSURE) is
        # emitted at the use site.
        self._lifted_lambdas: list[tuple[str, list[str], Lambda]] = []
        self._lambda_counter = 0
        # Program-wide fresh-label counter.  Per-region counters
        # collide between functions (``evp``'s ``_else_0`` would
        # clash with ``odp``'s ``_else_0`` and the JVM backend
        # rejects duplicate labels).  Bumping it once per call
        # keeps every emitted label name unique across the entire
        # IR program — same convention twig-beam-compiler uses.
        self._next_label_id = 0

    # ------------------------------------------------------------------
    # Top-level driver
    # ------------------------------------------------------------------

    def compile(self, program: Program) -> IrProgram:
        # ── Pre-pass 1: classify and validate every top-level form ─────
        # We need the function table built before any function body
        # compiles, so calls can find the callee even with mutual
        # recursion.
        top_level_exprs: list[Expr] = []
        function_defs: list[tuple[str, Lambda]] = []
        for form in program.forms:
            if isinstance(form, Define):
                self._classify_define(form)
                if isinstance(form.expr, Lambda):
                    function_defs.append((form.name, form.expr))
            else:
                # Bare top-level expression — accumulates into _start.
                top_level_exprs.append(form)

        # ── Emit each user function as its own callable region ────────
        for name, lam in function_defs:
            self._emit_function(name, lam)

        # ── Emit the synthesised _start region ────────────────────────
        self._emit_start(top_level_exprs)

        # JVM02 Phase 2d: lifted-lambda regions go AFTER ``_start`` so
        # the entry point stays the program's _start.
        # ``_compile_expr`` may have appended to ``self._lifted_lambdas``
        # while compiling earlier functions / _start; lifted lambdas
        # can also be nested (lambda inside lambda) so the list grows
        # during this final pass — iterate by index.
        i = 0
        while i < len(self._lifted_lambdas):
            lifted_name, captures, lam = self._lifted_lambdas[i]
            self._emit_lifted_lambda(lifted_name, captures, lam)
            i += 1

        return self._program

    # ------------------------------------------------------------------
    # Top-level form classification
    # ------------------------------------------------------------------

    def _classify_define(self, form: Define) -> None:
        """Validate and record one top-level ``(define ...)``."""
        if isinstance(form.expr, Lambda):
            # Function define — record its parameter list so apply
            # sites know how many args to plumb through.
            self._fn_params[form.name] = list(form.expr.params)
            return

        # Value define — must have a literal RHS in v1.
        if isinstance(form.expr, IntLit):
            self._value_consts[form.name] = int(form.expr.value)
            return
        if isinstance(form.expr, BoolLit):
            self._value_consts[form.name] = 1 if form.expr.value else 0
            return

        raise TwigCompileError(
            f"(define {form.name} ...) — TW02 v1 only supports literal "
            "RHS for value defines.  Use a top-level function for "
            "computed values."
        )

    # ------------------------------------------------------------------
    # Function emission
    # ------------------------------------------------------------------

    def _emit_function(self, name: str, lam: Lambda) -> None:
        """Compile one ``(define (name params) body...)`` into a
        labeled region of the IR program.

        Parameters live at registers ``_REG_PARAM_BASE + i``.  The
        body's value is moved into ``_REG_HALT_RESULT`` and the
        region ends with ``IrOp.RET``.
        """
        # Open the labelled region.
        self._emit(IrOp.LABEL, IrLabel(name=name), id=-1)

        # Copy each parameter out of its *arrival* register
        # (``_REG_PARAM_BASE + i`` — the slot the caller wrote to)
        # into a fresh body-local *holding* register (index ≥ 10).
        # Why: subsequent ``CALL`` sites in the body marshal arguments
        # into the same arrival slots, which would clobber the param
        # value before recursive use.  Holding registers sit above the
        # param window, so call-site arg setup never touches them.
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

        # Move result into the HALT-result register and return.
        self._emit_move(IrRegister(_REG_HALT_RESULT), last)
        self._emit(IrOp.RET)

    def _emit_start(self, exprs: list[Expr]) -> None:
        """Compile the synthesised ``_start`` region.

        Top-level expressions evaluate in order; the last one's
        value is written as a single byte to stdout via
        ``SYSCALL 1``, then ``HALT`` exits the program.
        """
        self._emit(IrOp.LABEL, IrLabel(name="_start"), id=-1)
        ctx = _FnCtx(locals_={})

        last: IrRegister | None = None
        for expr in exprs:
            last = self._compile_expr(expr, ctx)

        if last is None:
            # Empty program → output 0.
            zero = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
            last = zero

        # Write the final value as a byte to stdout via SYSCALL 1.
        # The JVM backend expects (imm number, register arg).
        self._emit_move(IrRegister(_REG_HALT_RESULT), last)
        self._emit(IrOp.SYSCALL, IrImmediate(1), IrRegister(_REG_HALT_RESULT))
        self._emit(IrOp.HALT)

    # ------------------------------------------------------------------
    # Expression compilation
    # ------------------------------------------------------------------

    def _compile_expr(self, expr: Expr, ctx: _FnCtx) -> IrRegister:
        """Compile ``expr``; return the register holding its value."""
        if isinstance(expr, IntLit):
            return self._compile_int(expr.value, ctx)
        if isinstance(expr, BoolLit):
            return self._compile_int(1 if expr.value else 0, ctx)
        if isinstance(expr, NilLit):
            # TW03 Phase 3e: lower nil to LOAD_NIL.
            dest = self._fresh_holding(ctx)
            self._emit(IrOp.LOAD_NIL, dest)
            return dest
        if isinstance(expr, SymLit):
            # TW03 Phase 3e: lower 'foo / (quote foo) to MAKE_SYMBOL.
            dest = self._fresh_holding(ctx)
            self._emit(IrOp.MAKE_SYMBOL, dest, IrLabel(expr.name))
            return dest
        if isinstance(expr, VarRef):
            return self._compile_var_ref(expr, ctx)
        if isinstance(expr, If):
            return self._compile_if(expr, ctx)
        if isinstance(expr, Let):
            return self._compile_let(expr, ctx)
        if isinstance(expr, Begin):
            last: IrRegister | None = None
            for e in expr.exprs:
                last = self._compile_expr(e, ctx)
            assert last is not None
            return last
        if isinstance(expr, Lambda):
            return self._compile_anonymous_lambda(expr, ctx)
        if isinstance(expr, Apply):
            return self._compile_apply(expr, ctx)
        raise TwigCompileError(
            f"unhandled expression type: {type(expr).__name__}"
        )

    # ------------------------------------------------------------------
    # Atoms
    # ------------------------------------------------------------------

    def _compile_int(self, value: int, ctx: _FnCtx) -> IrRegister:
        reg = self._fresh_holding(ctx)
        self._emit(IrOp.LOAD_IMM, reg, IrImmediate(value))
        return reg

    def _compile_var_ref(self, expr: VarRef, ctx: _FnCtx) -> IrRegister:
        # Local (parameter or let-binding)?
        if expr.name in ctx.locals_:
            return ctx.locals_[expr.name]

        # Top-level value-constant?  Inline it.
        if expr.name in self._value_consts:
            return self._compile_int(self._value_consts[expr.name], ctx)

        # Top-level function reference (not a call) is not yet a thing
        # in v1 — first-class function values need closures.
        if expr.name in self._fn_params:
            raise TwigCompileError(
                f"top-level function {expr.name!r} can't be passed as a "
                "value yet — closures land in TW02.5"
            )

        raise TwigCompileError(f"unbound name {expr.name!r}")

    # ------------------------------------------------------------------
    # Compound forms
    # ------------------------------------------------------------------

    def _compile_if(self, expr: If, ctx: _FnCtx) -> IrRegister:
        """``if`` lowers to ``BRANCH_Z`` + ``JUMP`` over labelled blocks.

        The ``then`` and ``else`` branches each produce a value;
        we move both into the same ``result`` register so the
        post-``if`` code can use it uniformly.
        """
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
        """Mutually-independent (Scheme ``let``) bindings.

        Each RHS is compiled in the *outer* scope.  After all RHS
        regs are computed, we extend ``ctx.locals_`` so the body
        sees them — and restore the prior bindings after the body
        in case the let was nested inside something else that uses
        the same names.
        """
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

    def _compile_apply(self, expr: Apply, ctx: _FnCtx) -> IrRegister:
        """Function application, dispatched at compile time:

        * ``(host/write-byte b)`` etc. → move arg into scratch reg 0,
          emit ``IrOp.SYSCALL IrImmediate(num) IrRegister(0)``.
          The JVM backend's ``__ca_syscall`` helper dispatches on
          the platform-independent syscall number.
        * ``(+ a b)`` and other binary builtins → emit the
          corresponding ``IrOp`` directly (one IR instruction).
        * ``(f a b)`` where ``f`` is a top-level function → emit
          the move-args-into-param-slots dance, then ``IrOp.CALL``.
        * ``((make-adder 7) 35)`` — the function position is itself
          an Apply (or a let-bound name) holding a closure value;
          falls through to ``APPLY_CLOSURE`` (JVM02 Phase 2d).
        """
        if isinstance(expr.fn, VarRef):
            name = expr.fn.name

            # Module-qualified call: any name containing an interior ``/``
            # (not at position 0 or the last position) is module-scoped.
            # Two sub-categories:
            #
            # 1. ``host/*`` — the synthetic host module whose exports
            #    lower to platform-independent syscall numbers.  Handled
            #    by ``_HOST_SYSCALLS`` lookup.
            #
            # 2. ``user/module/fn`` (TW04 Phase 4d) — a call to a
            #    function exported by another Twig module in the same
            #    build.  The compiler cannot see that module's parameter
            #    list here (multi-module compilation doesn't share a
            #    global function table across modules), so we trust the
            #    programmer's call site arity and emit a ``IrOp.CALL``
            #    with the qualified label.  The JVM backend decomposes
            #    ``"a/math/add"`` → ``invokestatic a/math.add()I``.
            _slash = name.find("/")
            if _slash > 0 and _slash < len(name) - 1:
                # Module-qualified host call (``host/write-byte`` etc.)
                num = _HOST_SYSCALLS.get(name)
                if num is not None:
                    # Host syscall path (unchanged from Phase 4c).
                    if num == 2:  # read-byte — no user arg, result into fresh reg
                        dest = self._fresh_holding(ctx)
                        self._emit(IrOp.SYSCALL, IrImmediate(num), dest)
                        return dest
                    # write-byte (1) or exit (10) — one user arg
                    arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
                    self._emit_move(IrRegister(_REG_SCRATCH), arg_regs[0])
                    self._emit(IrOp.SYSCALL, IrImmediate(num), IrRegister(_REG_SCRATCH))
                    # Return the scratch reg; callers treat host calls as
                    # returning NIL/0, and the value there is irrelevant.
                    return IrRegister(_REG_SCRATCH)

                # TW04 Phase 4d: user-module cross-module call.
                # Emit each arg into a holding reg, move to param slots,
                # then IrOp.CALL with the fully-qualified label.  The JVM
                # backend lowers this to ``invokestatic`` on the foreign
                # class (see ``_discover_callable_regions`` + CALL emission
                # in ``ir-to-jvm-class-file``).
                arg_regs_xm = [self._compile_expr(a, ctx) for a in expr.args]
                for i, src in enumerate(arg_regs_xm):
                    self._emit_move(IrRegister(_REG_PARAM_BASE + i), src)
                self._emit(IrOp.CALL, IrLabel(name))
                dest = self._fresh_holding(ctx)
                self._emit_move(dest, IrRegister(_REG_HALT_RESULT))
                return dest

            if name in _V1_REJECTED_BUILTINS:
                raise TwigCompileError(
                    f"builtin {name!r} is not yet supported by the JVM "
                    "backend — see TW02.5"
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
                arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
                dest = self._fresh_holding(ctx)
                self._emit(op, dest, *arg_regs)
                return dest

            # Direct binary builtin
            if name in _BINARY_OPS:
                if len(expr.args) != 2:
                    raise TwigCompileError(
                        f"{name!r} expects 2 arguments in TW02 v1, got "
                        f"{len(expr.args)}"
                    )
                left = self._compile_expr(expr.args[0], ctx)
                right = self._compile_expr(expr.args[1], ctx)
                dest = self._fresh_holding(ctx)
                self._emit(_BINARY_OPS[name], dest, left, right)
                return dest

            # Direct user-function call
            if name in self._fn_params:
                params = self._fn_params[name]
                if len(expr.args) != len(params):
                    raise TwigCompileError(
                        f"function {name!r} takes {len(params)} arguments, "
                        f"got {len(expr.args)}"
                    )

                # Step 1: evaluate every argument into its own holding
                # register.  We must NOT eval directly into r2/r3 —
                # nested calls (g x) would clobber f's r2 with x.
                arg_regs: list[IrRegister] = [
                    self._compile_expr(a, ctx) for a in expr.args
                ]

                # Step 2: copy holding regs into the function's param
                # slots (registers 2, 3, ...).
                for i, src in enumerate(arg_regs):
                    self._emit_move(IrRegister(_REG_PARAM_BASE + i), src)

                # Step 3: invoke.
                self._emit(IrOp.CALL, IrLabel(name))

                # Step 4: result is in register 1.  Copy out so the
                # caller can use it without worrying about a future
                # call clobbering it.
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
    # Closure compilation (JVM02 Phase 2d)
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

        Captures-first parameter layout (matches ir-to-jvm-class-file's
        Phase 2c.5 lowering convention): the lifted region's IR
        register layout is

            r2..r{1+num_free}                    — captures
            r{2+num_free}..r{1+num_free+arity}   — explicit params
            r{10+}                               — body holding regs

        ir-to-jvm-class-file's ``_build_lifted_lambda_method``
        prepends a JVM-args → __ca_regs prologue so the body can
        use the existing IR-body emitter unchanged.
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

    def _emit(self, opcode: IrOp, *operands: object, id: int | None = None) -> None:
        """Append one ``IrInstruction`` to the program.

        ``id=-1`` is used for ``LABEL`` instructions (matches the
        rest of the repo); regular instructions take a fresh ID
        from ``_gen``.
        """
        self._program.add_instruction(
            IrInstruction(
                opcode=opcode,
                operands=list(operands),  # type: ignore[arg-type]
                id=self._gen.next() if id is None else id,
            )
        )

    def _emit_move(self, dst: IrRegister, src: IrRegister) -> None:
        """Move ``src`` into ``dst`` via ``ADD_IMM dst, src, 0``.

        The IR has no dedicated MOV opcode; ``ADD_IMM`` with a
        zero immediate is the canonical equivalent — used
        throughout brainfuck-ir-compiler and friends.  When ``dst``
        already equals ``src`` the move is a no-op (still emitted
        for clarity; the optimiser cleans it up).
        """
        if dst == src:
            return
        self._emit(IrOp.ADD_IMM, dst, src, IrImmediate(0))

    def _fresh_holding(self, ctx: _FnCtx) -> IrRegister:
        idx = ctx.next_holding
        ctx.next_holding += 1
        return IrRegister(index=idx)

    def _fresh_label(self, _ctx: _FnCtx, prefix: str) -> str:
        idx = self._next_label_id
        self._next_label_id += 1
        return f"_{prefix}_{idx}"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def compile_to_ir(source: str) -> IrProgram:
    """Compile Twig source to an unoptimised :class:`IrProgram`."""
    ast = parse_twig(source)
    program = extract_program(ast)
    return _Compiler().compile(program)


def compile_source(
    source: str,
    *,
    class_name: str = "TwigProgram",
    optimize: bool = True,
) -> PackageResult:
    """Run the full Twig → JVM class file pipeline."""
    try:
        ast = parse_twig(source)
        twig_program = extract_program(ast)
    except Exception as exc:
        raise PackageError("parse", str(exc), exc) from exc

    try:
        compiler = _Compiler()
        raw_ir = compiler.compile(twig_program)
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

    # JVM02 Phase 2d: build closure_free_var_counts from the
    # lifted-lambda table the compiler discovered during IR emission.
    # ir-to-jvm-class-file uses this to detect closure regions and
    # auto-generate the ``Closure`` interface + per-lambda
    # ``Closure_<name>`` subclasses.
    #
    # Multi-arity follow-up: also record each lambda's explicit
    # arity (its source-level param count, NOT counting captures)
    # so the backend's lifted-lambda emission and the closure
    # subclass's ``Apply`` forwarder reserve the right number of
    # int-arg slots.  Without this every lambda would be treated
    # as arity-1 — multi-arg lambdas like ``(lambda (x y) (+ x y))``
    # would silently drop the second arg.
    closure_free_var_counts: dict[str, int] = {}
    closure_explicit_arities: dict[str, int] = {}
    for lifted_name, captures, lam in compiler._lifted_lambdas:  # noqa: SLF001
        closure_free_var_counts[lifted_name] = len(captures)
        closure_explicit_arities[lifted_name] = len(lam.params)

    config = JvmBackendConfig(
        class_name=class_name,
        emit_main_wrapper=True,
        closure_free_var_counts=closure_free_var_counts,
        closure_explicit_arities=closure_explicit_arities,
    )

    # TW03 Phase 3e: heap-primitive programs also need the multi-class
    # output (Cons / Symbol / Nil runtime classes).  Detect either
    # closures or heap ops in the IR and route to lower_ir_to_jvm_classes.
    _HEAP_IR_OPS = frozenset({
        IrOp.MAKE_CONS, IrOp.CAR, IrOp.CDR, IrOp.IS_NULL, IrOp.IS_PAIR,
        IrOp.MAKE_SYMBOL, IrOp.IS_SYMBOL, IrOp.LOAD_NIL,
    })
    uses_heap_ir = any(
        i.opcode in _HEAP_IR_OPS for i in optimized_ir.instructions
    )

    try:
        if closure_free_var_counts or uses_heap_ir:
            # Multi-class output: main class + Closure interface +
            # per-lambda Closure_<name> subclasses + Cons/Symbol/Nil
            # runtime classes (auto-included by lower_ir_to_jvm_classes
            # when heap ops are detected).
            multi = lower_ir_to_jvm_classes(optimized_ir, config)
            artifact = multi.main
        else:
            multi = None
            artifact = lower_ir_to_jvm_class_file(optimized_ir, config)
    except Exception as exc:
        raise PackageError("lower-jvm", str(exc), exc) from exc

    return PackageResult(
        source=source,
        class_name=class_name,
        ast=twig_program,
        raw_ir=raw_ir,
        optimization=optimization,
        optimized_ir=optimized_ir,
        artifact=artifact,
        class_bytes=artifact.class_bytes,
        multi_class_artifact=multi,
    )


def java_available() -> bool:
    """Return ``True`` iff a working ``java`` binary is on PATH."""
    if shutil.which("java") is None:
        return False
    try:
        result = subprocess.run(
            ["java", "-version"],
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
    class_name: str = "TwigProgram",
    optimize: bool = True,
    timeout_seconds: int = 30,
) -> ExecutionResult:
    """Compile and execute on the **real** ``java`` runtime.

    Writes the class file to a fresh temp dir and invokes
    ``java -cp <tmp> <class_name>`` as a subprocess.  Returns the
    captured stdout / stderr / exit code so tests can assert on
    real JVM output.
    """
    compilation = compile_source(
        source, class_name=class_name, optimize=optimize
    )
    with tempfile.TemporaryDirectory() as tmp:
        # JVM02 Phase 2d: closure programs need the full multi-class
        # bundle (main + Closure interface + per-lambda subclasses)
        # in a JAR, then ``java -jar`` finds them all.  Non-closure
        # programs use the existing single-class + ``java -cp`` flow.
        if compilation.multi_class_artifact is not None:
            from jvm_jar_writer import JarManifest, write_jar
            classes = tuple(
                (c.class_filename, c.class_bytes)
                for c in compilation.multi_class_artifact.classes
            )
            jar_bytes = write_jar(classes, JarManifest(main_class=class_name))
            jar_path = Path(tmp) / f"{class_name}.jar"
            jar_path.write_bytes(jar_bytes)
            proc = subprocess.run(
                ["java", "-jar", str(jar_path)],
                capture_output=True,
                timeout=timeout_seconds,
                check=False,
            )
        else:
            write_class_file(compilation.artifact, tmp)
            proc = subprocess.run(
                ["java", "-cp", tmp, class_name],
                capture_output=True,
                timeout=timeout_seconds,
                check=False,
            )
    return ExecutionResult(
        compilation=compilation,
        stdout=proc.stdout,
        stderr=proc.stderr,
        returncode=proc.returncode,
    )


# ---------------------------------------------------------------------------
# TW04 Phase 4d — multi-module compilation
# ---------------------------------------------------------------------------


def module_name_to_jvm_class(name: str) -> str:
    """Map a Twig module name to its JVM internal class name.

    JVM internal class names already use ``/`` as the package separator,
    so the mapping is the identity transformation for Phase 4d.

    Examples
    --------
    >>> module_name_to_jvm_class("user/hello")
    'user/hello'
    >>> module_name_to_jvm_class("stdlib/io")
    'stdlib/io'
    >>> module_name_to_jvm_class("a/math")
    'a/math'

    The synthetic ``host`` module has no JVM class — callers should
    filter it out before calling this function.
    """
    # Identity: forward slashes are the JVM package separator already.
    return name


def _compile_one_module(
    module: ResolvedModule,
    *,
    is_entry: bool,
    optimize: bool,
) -> ModuleCompileResult:
    """Compile one ``ResolvedModule`` to a ``JVMClassArtifact``.

    Every module class in a multi-module build uses the shared
    ``TwigRuntime`` register file (``external_runtime_class`` set to
    ``TWIG_RUNTIME_BINARY_NAME``).  The entry module gets a ``main()``
    wrapper; all other modules do not.

    Raises :class:`PackageError` on any compilation failure, tagging it
    with the module name and the stage that failed.
    """
    jvm_class_name = module_name_to_jvm_class(module.name)
    stage_prefix = f"module:{module.name}"

    try:
        compiler = _Compiler()
        raw_ir = compiler.compile(module.program)
    except TwigCompileError:
        raise
    except Exception as exc:  # pragma: no cover
        raise PackageError(f"{stage_prefix}:ir-emit", str(exc), exc) from exc

    if optimize:
        optimization = IrOptimizer.default_passes().optimize(raw_ir)
        optimized_ir = optimization.program
    else:
        optimized_ir = raw_ir

    # Build closure_free_var_counts and closure_explicit_arities from the
    # lifted-lambda table (mirrors compile_source).
    closure_free_var_counts: dict[str, int] = {}
    closure_explicit_arities: dict[str, int] = {}
    for lifted_name, captures, lam in compiler._lifted_lambdas:  # noqa: SLF001
        closure_free_var_counts[lifted_name] = len(captures)
        closure_explicit_arities[lifted_name] = len(lam.params)

    # TW04 Phase 4d: exported functions are only called from OTHER modules
    # via cross-module invokestatic, so _discover_callable_regions would
    # not include them (no local CALL targets them).  Pass exports as
    # extra_callable_labels so the backend emits them as public methods.
    module_decl = module.program.module
    extra_callable = tuple(module_decl.exports) if module_decl else ()

    config = JvmBackendConfig(
        class_name=jvm_class_name,
        emit_main_wrapper=is_entry,
        closure_free_var_counts=closure_free_var_counts,
        closure_explicit_arities=closure_explicit_arities,
        # TW04 Phase 4d: all module classes share one register file via
        # TwigRuntime.  Each module's class accesses __ca_regs and
        # __ca_objregs there rather than defining its own static fields.
        external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        extra_callable_labels=extra_callable,
    )

    _HEAP_IR_OPS = frozenset({
        IrOp.MAKE_CONS, IrOp.CAR, IrOp.CDR, IrOp.IS_NULL, IrOp.IS_PAIR,
        IrOp.MAKE_SYMBOL, IrOp.IS_SYMBOL, IrOp.LOAD_NIL,
    })
    uses_heap_ir = any(
        i.opcode in _HEAP_IR_OPS for i in optimized_ir.instructions
    )

    try:
        if closure_free_var_counts or uses_heap_ir:
            multi = lower_ir_to_jvm_classes(optimized_ir, config)
            artifact = multi.main
        else:
            multi = None
            artifact = lower_ir_to_jvm_class_file(optimized_ir, config)
    except Exception as exc:
        raise PackageError(f"{stage_prefix}:lower-jvm", str(exc), exc) from exc

    return ModuleCompileResult(
        module_name=module.name,
        jvm_class_name=jvm_class_name,
        ir_program=optimized_ir,
        artifact=artifact,
        is_entry=is_entry,
        multi_class_artifact=multi,
    )


def compile_modules(
    modules: list[ResolvedModule],
    *,
    entry_module: str,
    optimize: bool = True,
) -> MultiModuleResult:
    """Compile a set of Twig modules to JVM class files.

    ``modules`` must be in topological order (deps before importers),
    as returned by :func:`twig.resolve_modules`.  The ``host``
    synthetic module is automatically skipped — its exports lower to
    ``IrOp.SYSCALL`` instructions in each module's IR, not to a
    separate ``.class`` file.

    Parameters
    ----------
    modules:
        Topologically ordered list of resolved modules.  The entry
        module must appear last (or at least once) in the list.
    entry_module:
        The name of the module whose ``_start`` region gets a
        ``main([Ljava/lang/String;)V`` wrapper, making it the JVM
        program's entry point.
    optimize:
        Pass ``True`` (the default) to run the IR optimizer before
        lowering.

    Returns
    -------
    MultiModuleResult
        Contains the shared ``TwigRuntime`` artifact and one
        ``ModuleCompileResult`` per non-``host`` module.

    Raises
    ------
    PackageError
        If any module fails to compile.
    ValueError
        If ``entry_module`` does not appear in ``modules``.
    """
    entry_names = {m.name for m in modules}
    if entry_module not in entry_names:
        raise ValueError(
            f"entry_module {entry_module!r} not found in the provided "
            f"module list.  Available: {sorted(entry_names)}"
        )

    runtime_artifact = build_runtime_class_artifact()

    results: list[ModuleCompileResult] = []
    for module in modules:
        if module.name == HOST_MODULE_NAME:
            # host is synthetic — no .class emitted; its exports become
            # SYSCALL instructions in the IR of importing modules.
            continue
        is_entry = (module.name == entry_module)
        result = _compile_one_module(module, is_entry=is_entry, optimize=optimize)
        results.append(result)

    return MultiModuleResult(
        runtime_artifact=runtime_artifact,
        modules=results,
        entry_class_name=module_name_to_jvm_class(entry_module),
    )


def run_modules(
    modules: list[ResolvedModule],
    *,
    entry_module: str,
    optimize: bool = True,
    timeout_seconds: int = 30,
) -> MultiModuleExecutionResult:
    """Compile a multi-module Twig program and execute it on real ``java``.

    Bundles the compiled classes (TwigRuntime + all module classes +
    closure/heap runtime helpers) into a JAR and invokes
    ``java -jar <jar>`` as a subprocess.  The JAR manifest sets
    ``Main-Class`` to the entry module's JVM class name.

    Classes are deduplicated by ``class_filename`` when multiple modules
    request the same runtime helper (e.g. the ``Closure`` interface or
    the ``Nil`` singleton).

    Parameters
    ----------
    modules, entry_module, optimize:
        Forwarded to :func:`compile_modules`.
    timeout_seconds:
        Maximum wall-clock time for the ``java`` invocation.

    Returns
    -------
    MultiModuleExecutionResult
        The compile result plus the subprocess stdout/stderr/exit code.

    Raises
    ------
    PackageError
        If any module fails to compile.
    RuntimeError
        If no ``java`` binary is on PATH.
    """
    if not java_available():
        raise RuntimeError(
            "No java binary found on PATH.  "
            "Install a JRE/JDK (e.g. `brew install openjdk`) to run "
            "multi-module Twig programs on the JVM."
        )

    multi = compile_modules(modules, entry_module=entry_module, optimize=optimize)

    from jvm_jar_writer import JarManifest, write_jar

    # Collect all class bytes, deduplicating by class_filename.
    # Ordering: TwigRuntime first, then module classes in topological
    # order, then shared runtime helpers (Closure, Cons, Symbol, Nil)
    # deduplicated across all modules.
    seen: dict[str, bytes] = {}

    def _add_artifact(art: JVMClassArtifact) -> None:
        key = art.class_filename
        if key not in seen:
            seen[key] = art.class_bytes

    _add_artifact(multi.runtime_artifact)

    for m in multi.modules:
        if m.multi_class_artifact is not None:
            for cls_artifact in m.multi_class_artifact.classes:
                _add_artifact(cls_artifact)
        else:
            _add_artifact(m.artifact)

    jar_classes = tuple(seen.items())
    jar_bytes = write_jar(
        jar_classes,
        JarManifest(main_class=multi.entry_class_name),
    )

    with tempfile.TemporaryDirectory() as tmp:
        jar_path = Path(tmp) / "program.jar"
        jar_path.write_bytes(jar_bytes)
        proc = subprocess.run(
            ["java", "-jar", str(jar_path)],
            capture_output=True,
            timeout=timeout_seconds,
            check=False,
        )

    return MultiModuleExecutionResult(
        multi=multi,
        stdout=proc.stdout,
        stderr=proc.stderr,
        exit_code=proc.returncode,
    )
