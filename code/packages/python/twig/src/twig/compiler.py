"""Twig compiler: typed AST → ``IIRModule``.

Pipeline shape
==============
::

    Twig source
      → parse_twig                  (lexer + grammar parser)
      → extract_program             (typed AST)
      → compile_program (this file) → IIRModule
      → vm-core / TwigVM runtime

What this compiler emits
========================
* One ``IIRFunction`` for the program's synthesised ``main``,
  containing the value defines and top-level expressions in source
  order.  ``main`` returns the value of the final top-level
  expression (or ``nil`` if the program has none).
* One ``IIRFunction`` per top-level ``(define (f ...) ...)``.
* One ``IIRFunction`` per anonymous ``lambda`` expression.  These
  use gensym'd names like ``__lambda_3`` and accept the captured
  free-variable values as their *leading* parameters, followed by
  the user-declared parameters.

Apply-site dispatch
===================
The compiler decides at compile time whether an ``Apply`` is a
direct call or an indirect closure call:

* If ``apply.fn`` is a ``VarRef`` whose name is a top-level *function*
  define **or** a builtin → emit ``IIRInstr("call", dest, [name, ...args])``.
* Otherwise (locals, value-globals, computed expressions) → emit
  ``IIRInstr("call_builtin", dest, ["apply_closure", handle, ...args])``.

The ``apply_closure`` builtin lives in the host (TwigVM) and
re-enters ``vm.execute`` with the closure's captured environment
prepended to the user-supplied arguments.

Globals
=======
Top-level ``(define name expr)`` of a *value* (not a function) is
implemented via host builtins:

* The synthesised ``main`` evaluates the RHS and emits
  ``call_builtin "global_set" "name" value`` to stash it.
* References to value-globals compile to
  ``call_builtin "global_get" "name"``.

Function defines do not go through ``global_*`` — references to them
compile to direct ``call``s, which is faster and matches Scheme's
top-level semantics.

Why all type hints are ``"any"``
================================
Twig is dynamically typed.  Every emitted instruction carries
``type_hint="any"`` so the VM's profiler can later observe runtime
types and feed them back into JIT specialisation (a future spec).
The ``IIRFunction.type_status`` is left at ``UNTYPED`` for the same
reason.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from interpreter_ir import (
    FunctionTypeStatus,
    IIRFunction,
    IIRInstr,
    IIRModule,
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
from twig.errors import TwigCompileError
from twig.free_vars import free_vars

# ---------------------------------------------------------------------------
# Built-in names
# ---------------------------------------------------------------------------
#
# Builtins are host-side callables registered with ``vm-core``'s
# ``BuiltinRegistry`` by ``TwigVM``.  They are invoked through
# ``call_builtin``.  The compiler treats their names as part of the
# global namespace: a reference to ``+`` or ``cons`` resolves to a
# direct builtin call, never a "name not found" error.
BUILTINS: frozenset[str] = frozenset(
    {
        # Arithmetic / comparison
        "+", "-", "*", "/", "=", "<", ">",
        # Cons cells
        "cons", "car", "cdr",
        # Predicates
        "null?", "pair?", "number?", "symbol?",
        # I/O
        "print",
    }
)


# ---------------------------------------------------------------------------
# Per-function compilation context
# ---------------------------------------------------------------------------


@dataclass
class _FnCtx:
    """Mutable state while compiling one IIRFunction body."""

    instrs: list[IIRInstr] = field(default_factory=list)
    locals_: set[str] = field(default_factory=set)
    """Names introduced at this function's top level (params + lets).

    Used by free-var analysis when a nested lambda computes its
    captures: any name that is *currently bound* in this frame is a
    legitimate capture, anything else must be a global or a typo.
    """

    label_counter: int = 0
    var_counter: int = 0


# ---------------------------------------------------------------------------
# Top-level compiler driver
# ---------------------------------------------------------------------------


class _Compiler:
    """Compile a :class:`Program` into an ``IIRModule``."""

    def __init__(self, *, module_name: str = "twig") -> None:
        self._module_name = module_name
        # Top-level function names (define of a lambda).
        self._fn_globals: set[str] = set()
        # Top-level value names (define of a non-lambda).
        self._value_globals: set[str] = set()
        # Cumulative function table — populated as we compile each
        # top-level define and each anonymous lambda.
        self._functions: list[IIRFunction] = []
        # Gensym counter for synthetic top-level function names.
        self._lambda_counter: int = 0

    # ------------------------------------------------------------------
    # Public entry
    # ------------------------------------------------------------------

    def compile(self, program: Program) -> IIRModule:
        # ── Pre-pass: classify every top-level define ─────────────────
        # We need this before walking bodies because free-variable
        # analysis at lambda sites needs to know which names are
        # globals and therefore *don't* count as free.
        for form in program.forms:
            if isinstance(form, Define):
                if isinstance(form.expr, Lambda):
                    self._fn_globals.add(form.name)
                else:
                    self._value_globals.add(form.name)

        # ── Compile every top-level form ──────────────────────────────
        # Function defines become IIRFunctions; value defines and
        # bare top-level expressions accumulate into ``main``.
        main_ctx = _FnCtx()
        last_main_value: str | None = None
        for form in program.forms:
            if isinstance(form, Define) and isinstance(form.expr, Lambda):
                self._compile_top_level_lambda(form.name, form.expr)
            elif isinstance(form, Define):
                # (define x expr) — evaluate at top level, store in globals.
                v = self._compile_expr(form.expr, main_ctx)
                name_reg = self._string_arg(main_ctx, form.name)
                self._emit_call_builtin(
                    main_ctx,
                    builtin="global_set",
                    args=[name_reg, v],
                    dest=None,
                )
                last_main_value = None  # define returns nothing useful
            else:
                # Bare top-level expression.  Its value becomes the
                # candidate return value for ``main``.
                last_main_value = self._compile_expr(form, main_ctx)

        # ── Synthesised ``main`` ──────────────────────────────────────
        if last_main_value is not None:
            main_ctx.instrs.append(
                IIRInstr("ret", None, [last_main_value], type_hint="any")
            )
        else:
            # No final value-producing expression → return nil.
            nil_var = self._fresh(main_ctx, "nil")
            self._emit_call_builtin(
                main_ctx, builtin="make_nil", args=[], dest=nil_var
            )
            main_ctx.instrs.append(
                IIRInstr("ret", None, [nil_var], type_hint="any")
            )

        self._functions.append(
            IIRFunction(
                name="main",
                params=[],
                return_type="any",
                instructions=main_ctx.instrs,
                register_count=self._reg_count(main_ctx),
                type_status=FunctionTypeStatus.UNTYPED,
            )
        )

        return IIRModule(
            name=self._module_name,
            functions=self._functions,
            entry_point="main",
            language="twig",
        )

    # ------------------------------------------------------------------
    # Function compilation
    # ------------------------------------------------------------------

    def _compile_top_level_lambda(self, name: str, lam: Lambda) -> None:
        """Compile ``(define (name args...) body+)``.

        Top-level functions never have free variables (TW00 doesn't
        permit forward references that aren't themselves globals),
        so the IIR signature is just the user-declared parameters.
        Captured-var prefixing is for *anonymous* lambdas only.
        """
        ctx = _FnCtx()
        ctx.locals_.update(lam.params)

        last: str | None = None
        for expr in lam.body:
            last = self._compile_expr(expr, ctx)
        if last is None:
            # Empty body would have been rejected by the parser, but
            # guard defensively.
            raise TwigCompileError(f"function {name!r} has empty body")
        ctx.instrs.append(IIRInstr("ret", None, [last], type_hint="any"))

        self._functions.append(
            IIRFunction(
                name=name,
                params=[(p, "any") for p in lam.params],
                return_type="any",
                instructions=ctx.instrs,
                register_count=self._reg_count(ctx),
                type_status=FunctionTypeStatus.UNTYPED,
            )
        )

    def _compile_anonymous_lambda(self, lam: Lambda, outer: _FnCtx) -> str:
        """Compile a ``lambda`` expression occurring inside another
        function.

        Emits a fresh top-level IIRFunction and returns the name of
        a register in ``outer`` holding a *closure handle* — produced
        by ``call_builtin "make_closure" <fn_name> captured...``.
        The fresh function takes the captured variables as its
        leading parameters, followed by the user-declared params.
        """
        # 1. Compute free variables — names referenced in the body
        #    that aren't params and aren't globals.
        globals_ = self._fn_globals | self._value_globals | BUILTINS
        captures = free_vars(lam, globals_)

        # All captures must be currently bound in ``outer``; if not,
        # the user wrote a name that doesn't resolve to anything.
        for c in captures:
            if c not in outer.locals_:
                raise TwigCompileError(
                    f"unbound name {c!r} captured by lambda — "
                    "did you forget a (define) or a (let ...) binding?"
                )

        # 2. Build the inner function's IIR.
        fn_name = f"__lambda_{self._lambda_counter}"
        self._lambda_counter += 1

        inner = _FnCtx()
        # The inner function's parameter list is captures ++ params.
        inner.locals_.update(captures)
        inner.locals_.update(lam.params)

        last: str | None = None
        for expr in lam.body:
            last = self._compile_expr(expr, inner)
        if last is None:
            raise TwigCompileError("lambda has empty body")
        inner.instrs.append(IIRInstr("ret", None, [last], type_hint="any"))

        all_params = [(c, "any") for c in captures] + [
            (p, "any") for p in lam.params
        ]
        self._functions.append(
            IIRFunction(
                name=fn_name,
                params=all_params,
                return_type="any",
                instructions=inner.instrs,
                register_count=self._reg_count(inner),
                type_status=FunctionTypeStatus.UNTYPED,
            )
        )

        # 3. Emit ``make_closure`` at the call site.  The fn_name
        # itself is a string literal — we must materialise it via
        # ``const`` so it survives ``frame.resolve``.  The captured
        # values are already register names from ``outer``'s scope.
        fn_name_reg = self._string_arg(outer, fn_name)
        dest = self._fresh(outer, "clos")
        srcs: list[str | int | float | bool] = [
            "make_closure", fn_name_reg, *captures,
        ]
        outer.instrs.append(
            IIRInstr("call_builtin", dest, srcs, type_hint="any")
        )
        return dest

    # ------------------------------------------------------------------
    # Expression compilation
    # ------------------------------------------------------------------

    def _compile_expr(self, expr: Expr, ctx: _FnCtx) -> str:
        """Compile ``expr`` into ``ctx``; return the dest register."""
        if isinstance(expr, IntLit):
            v = self._fresh(ctx, "n")
            ctx.instrs.append(
                IIRInstr("const", v, [expr.value], type_hint="any")
            )
            return v

        if isinstance(expr, BoolLit):
            v = self._fresh(ctx, "b")
            ctx.instrs.append(
                IIRInstr("const", v, [expr.value], type_hint="any")
            )
            return v

        if isinstance(expr, NilLit):
            v = self._fresh(ctx, "nil")
            self._emit_call_builtin(
                ctx, builtin="make_nil", args=[], dest=v
            )
            return v

        if isinstance(expr, SymLit):
            name_reg = self._string_arg(ctx, expr.name)
            v = self._fresh(ctx, "sym")
            self._emit_call_builtin(
                ctx, builtin="make_symbol", args=[name_reg], dest=v
            )
            return v

        if isinstance(expr, VarRef):
            return self._compile_var_ref(expr, ctx)

        if isinstance(expr, If):
            return self._compile_if(expr, ctx)

        if isinstance(expr, Begin):
            last: str | None = None
            for e in expr.exprs:
                last = self._compile_expr(e, ctx)
            assert last is not None  # parser rejects empty (begin)
            return last

        if isinstance(expr, Let):
            return self._compile_let(expr, ctx)

        if isinstance(expr, Lambda):
            return self._compile_anonymous_lambda(expr, ctx)

        if isinstance(expr, Apply):
            return self._compile_apply(expr, ctx)

        raise TwigCompileError(
            f"unhandled expression type: {type(expr).__name__}"
        )

    def _compile_var_ref(self, expr: VarRef, ctx: _FnCtx) -> str:
        """Compile a bare-name reference."""
        name = expr.name

        # Local? (parameter or let-binding)
        if name in ctx.locals_:
            # The IIR's register file is keyed by name, so we just
            # return the name directly — the next instruction that
            # reads it will resolve through ``frame.resolve``.
            return name

        # Top-level function?  Functions are first-class — we wrap
        # them in a 0-capture closure so the value can be passed
        # around or applied later.
        if name in self._fn_globals:
            name_reg = self._string_arg(ctx, name)
            v = self._fresh(ctx, "fnref")
            self._emit_call_builtin(
                ctx, builtin="make_closure", args=[name_reg], dest=v
            )
            return v

        # Top-level value?  Look up via host-side global table.
        if name in self._value_globals:
            name_reg = self._string_arg(ctx, name)
            v = self._fresh(ctx, "g")
            self._emit_call_builtin(
                ctx, builtin="global_get", args=[name_reg], dest=v
            )
            return v

        # Builtin?  Like top-level functions, expose as a closure
        # handle so users can pass ``+`` to ``map`` (in a future
        # version) without special-casing.
        if name in BUILTINS:
            name_reg = self._string_arg(ctx, name)
            v = self._fresh(ctx, "bref")
            self._emit_call_builtin(
                ctx,
                builtin="make_builtin_closure",
                args=[name_reg],
                dest=v,
            )
            return v

        raise TwigCompileError(
            f"unbound name {name!r} (no local, define, or builtin matches)"
        )

    def _compile_if(self, expr: If, ctx: _FnCtx) -> str:
        """``if`` lowers to standard ``label`` + ``jmp_if_false`` + ``jmp``."""
        cond = self._compile_expr(expr.cond, ctx)
        else_label = self._fresh_label(ctx, "else")
        end_label = self._fresh_label(ctx, "endif")
        result = self._fresh(ctx, "ifv")

        ctx.instrs.append(
            IIRInstr(
                "jmp_if_false",
                None,
                [cond, else_label],
                type_hint="void",
            )
        )

        # Then branch — compile and move the value into ``result``
        # via the host-side ``_move`` builtin.  ``_move`` is a
        # faithful identity function that preserves the value's
        # *Python type*; the alternative ``add result, then_v, 0``
        # would coerce booleans (``True + 0 == 1``).
        then_v = self._compile_expr(expr.then_branch, ctx)
        ctx.instrs.append(
            IIRInstr(
                "call_builtin", result, ["_move", then_v], type_hint="any"
            )
        )
        ctx.instrs.append(
            IIRInstr("jmp", None, [end_label], type_hint="void")
        )

        # Else branch — same shape.
        ctx.instrs.append(
            IIRInstr("label", None, [else_label], type_hint="void")
        )
        else_v = self._compile_expr(expr.else_branch, ctx)
        ctx.instrs.append(
            IIRInstr(
                "call_builtin", result, ["_move", else_v], type_hint="any"
            )
        )

        ctx.instrs.append(
            IIRInstr("label", None, [end_label], type_hint="void")
        )
        return result

    def _compile_let(self, expr: Let, ctx: _FnCtx) -> str:
        """Mutually-independent bindings — Scheme ``let`` semantics.

        Each RHS is compiled in the *outer* scope (no binding sees
        another's name yet); the body is then compiled with all
        binding names added to ``locals_``.
        """
        # Compile RHSs in outer scope.
        binding_values: list[tuple[str, str]] = []
        for name, rhs in expr.bindings:
            v = self._compile_expr(rhs, ctx)
            binding_values.append((name, v))

        # Bind into ``locals_`` via add-zero copy so the binding
        # names exist as named registers in the frame.
        added_names: list[str] = []
        for name, src in binding_values:
            if name in ctx.locals_:
                # Already a local — overwriting is fine; vm-core
                # reuses the register slot.
                pass
            else:
                ctx.locals_.add(name)
                added_names.append(name)
            # Use a host-side identity builtin so the value moves
            # without being coerced.  Plain ``add x 0`` would turn
            # ``True`` into ``1`` because Python's ``bool`` is a
            # subclass of ``int``.
            ctx.instrs.append(
                IIRInstr("call_builtin", name, ["_move", src], type_hint="any")
            )

        # Compile body.
        last: str | None = None
        for e in expr.body:
            last = self._compile_expr(e, ctx)
        assert last is not None

        # Pop ``let`` names back out of ``locals_`` (so subsequent
        # code in the enclosing scope doesn't think they're bound).
        for name in added_names:
            ctx.locals_.discard(name)

        return last

    def _compile_apply(self, expr: Apply, ctx: _FnCtx) -> str:
        """``Apply`` — direct call vs. closure call decided at compile time."""
        # Direct call: fn is a VarRef whose name is a top-level
        # function or a builtin.
        if isinstance(expr.fn, VarRef):
            name = expr.fn.name

            if name in self._fn_globals:
                # Direct user-defined call.
                arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
                dest = self._fresh(ctx, "r")
                srcs: list[str | int | float | bool] = [name, *arg_regs]
                ctx.instrs.append(
                    IIRInstr("call", dest, srcs, type_hint="any")
                )
                return dest

            if name in BUILTINS:
                # Direct builtin call.
                arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
                dest = self._fresh(ctx, "r")
                srcs = [name, *arg_regs]
                ctx.instrs.append(
                    IIRInstr("call_builtin", dest, srcs, type_hint="any")
                )
                return dest

        # Indirect call: compile the fn expression to get a closure
        # handle, then ``call_builtin "apply_closure" handle args...``.
        fn_handle = self._compile_expr(expr.fn, ctx)
        arg_regs = [self._compile_expr(a, ctx) for a in expr.args]
        dest = self._fresh(ctx, "r")
        srcs = ["apply_closure", fn_handle, *arg_regs]
        ctx.instrs.append(
            IIRInstr("call_builtin", dest, srcs, type_hint="any")
        )
        return dest

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _fresh(self, ctx: _FnCtx, prefix: str) -> str:
        ctx.var_counter += 1
        return f"_{prefix}{ctx.var_counter}"

    def _fresh_label(self, ctx: _FnCtx, prefix: str) -> str:
        ctx.label_counter += 1
        return f"_{prefix}{ctx.label_counter}"

    def _emit_call_builtin(
        self,
        ctx: _FnCtx,
        *,
        builtin: str,
        args: list,
        dest: str | None,
    ) -> None:
        """Emit a ``call_builtin`` instruction.

        ``builtin`` is the literal builtin name; it is stored as
        ``srcs[0]`` and read by ``vm-core``'s ``handle_call_builtin``
        without being resolved through the frame.  ``args`` are the
        runtime argument values — each goes into ``srcs[1:]`` and IS
        resolved through the frame, so any *string* in ``args`` must
        be a register name.

        Raw string literals (e.g. a function name passed to
        ``make_closure`` or a global key passed to ``global_set``)
        therefore need to be materialised into a register first.
        :meth:`_string_arg` does that.
        """
        srcs: list[str | int | float | bool] = [builtin, *args]
        ctx.instrs.append(
            IIRInstr(
                "call_builtin",
                dest,
                srcs,
                type_hint="any" if dest is not None else "void",
            )
        )

    def _string_arg(self, ctx: _FnCtx, value: str) -> str:
        """Materialise a Python string into a fresh register and
        return that register's name.

        Used when a builtin call needs a *literal* string as one of
        its runtime arguments — e.g.
        ``call_builtin "make_closure" <fn_name> ...``.  The IIR's
        ``call_builtin`` resolves every entry in ``srcs[1:]`` through
        the frame's name-to-register map, so a bare string would be
        looked up as a variable and fail.  Wrapping the string in a
        ``const`` works because ``const`` stores its argument
        verbatim, including string values.
        """
        var = self._fresh(ctx, "s")
        ctx.instrs.append(IIRInstr("const", var, [value], type_hint="any"))
        return var

    def _reg_count(self, ctx: _FnCtx) -> int:
        # vm-core's RegisterFile is fixed-size.  Be generous: count
        # every distinct dest name we emitted plus a small buffer.
        names: set[str] = set()
        for instr in ctx.instrs:
            if instr.dest is not None:
                names.add(instr.dest)
            for src in instr.srcs:
                if isinstance(src, str):
                    names.add(src)
        return max(len(names) + 8, 16)


# ---------------------------------------------------------------------------
# Public entry
# ---------------------------------------------------------------------------


def compile_program(
    program: Program, *, module_name: str = "twig"
) -> IIRModule:
    """Compile a typed :class:`Program` into an ``IIRModule``."""
    return _Compiler(module_name=module_name).compile(program)
