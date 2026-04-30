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
    JvmBackendConfig,
    JVMClassArtifact,
    JVMMultiClassArtifact,
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
#   register 0  — scratch zero (also the SYSCALL-arg slot for SYSCALL 1)
#   register 1  — HALT result / function return value
#   register 2..N — function parameter slots (param i → register 2 + i)
#   register 10..  — compiler-allocated holding registers for intermediate
#                    values; using a high base keeps them clear of
#                    parameter slots, so nested calls don't clobber.
_REG_HALT_RESULT = 1
_REG_PARAM_BASE = 2
_REG_HOLDING_BASE = 10


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
    closure_free_var_counts: dict[str, int] = {}
    for lifted_name, captures, _lam in compiler._lifted_lambdas:  # noqa: SLF001
        closure_free_var_counts[lifted_name] = len(captures)

    config = JvmBackendConfig(
        class_name=class_name,
        emit_main_wrapper=True,
        closure_free_var_counts=closure_free_var_counts,
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
