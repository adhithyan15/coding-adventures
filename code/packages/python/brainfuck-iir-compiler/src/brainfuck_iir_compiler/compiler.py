"""Brainfuck → InterpreterIR compiler.

Background
==========
Brainfuck has eight commands that operate on a tape of byte cells and a
single data pointer.  ``InterpreterIR`` (LANG01) is the language-agnostic
bytecode that the generic ``vm-core`` interpreter (LANG02) executes.  This
module bridges the two: AST in, ``IIRModule`` out.

The mapping is mechanical.  See ``code/specs/BF04-brainfuck-iir-compiler.md``
for the canonical command → IIR table.  This file is the reference
implementation of that table.

Why one giant ``main`` function?
================================
Brainfuck has no functions, no scopes, no parameters — just a sequence of
commands and (possibly nested) loops.  So everything compiles into a
single ``IIRFunction`` named ``main`` with no parameters.  Loops are
implemented as ``label`` + ``jmp_if_*`` pairs within that one function;
the IIR's named-label control flow handles arbitrary nesting without any
extra bookkeeping on our side.

Why fixed register names instead of SSA?
========================================
The IIR's textual operand grammar looks SSA-shaped (``dest`` per
instruction), but ``vm-core``'s frame model is plain mutable registers:
``frame.assign(name, value)`` reuses the same slot on every assignment to
the same name.  Critically, an SSA renaming would *break* programs with
control flow that may skip a block — names defined only inside the
skipped block would be undefined when the post-block code tries to read
them, because the loop body never assigned them.

The IIR has no phi-nodes that would let us paper over this in the
front-end.  So we simply use a small fixed set of register names —
``ptr``, ``v``, ``c``, ``k`` — and overwrite them as needed.  This
matches the natural register allocation a hand-written interpreter
would use, and matches how ``brainfuck-ir-compiler`` allocates
registers in the static AOT path (v0..v6 fixed roles).

When BF05 wires the JIT, ``jit-core``'s specialiser builds its own SSA
form internally; the front-end's job is just to give vm-core a runnable
program.

Type hints
==========
Every emitted instruction carries a concrete ``type_hint`` (``u8`` for
cell values and immediates that participate in cell arithmetic, ``u32``
for pointer values).  No instruction is left as ``"any"``.  This makes
the resulting ``IIRFunction`` ``FULLY_TYPED``, which lets the future
JIT (BF05) tier up immediately on first call without waiting for the
profiler to fill in observed types.
"""

from __future__ import annotations

from typing import Any

from brainfuck import parse_brainfuck
from interpreter_ir import (
    FunctionTypeStatus,
    IIRFunction,
    IIRInstr,
    IIRModule,
)
from lang_parser import ASTNode

# ---------------------------------------------------------------------------
# Public entry points
# ---------------------------------------------------------------------------


def compile_source(
    source: str,
    *,
    module_name: str = "brainfuck",
) -> IIRModule:
    """Lex, parse, and compile Brainfuck ``source`` into an ``IIRModule``.

    Convenience wrapper around :func:`compile_to_iir` that hides the
    parser invocation.  Raises whatever the lexer/parser raise on
    malformed input (e.g. unmatched brackets).
    """
    ast = parse_brainfuck(source)
    return compile_to_iir(ast, module_name=module_name)


def compile_to_iir(
    ast: ASTNode,
    *,
    module_name: str = "brainfuck",
) -> IIRModule:
    """Compile a parsed Brainfuck AST into a single-function ``IIRModule``.

    The returned module has exactly one function named ``main`` with no
    parameters and ``return_type="void"``.  Its ``type_status`` is
    :attr:`FunctionTypeStatus.FULLY_TYPED` because every emitted
    instruction carries a concrete type hint.
    """
    compiler = _Compiler()
    compiler.emit_program(ast)
    return IIRModule(
        name=module_name,
        functions=[compiler.finish()],
        entry_point="main",
        language="brainfuck",
    )


# ---------------------------------------------------------------------------
# Internal compiler state
# ---------------------------------------------------------------------------


_PTR = "ptr"   # data-pointer register
_VAL = "v"     # cell-value scratch register
_COND = "c"    # loop-condition scratch register
_IMM = "k"     # immediate-1 scratch register

# Registers the function uses, in declaration order.  Used to size the
# per-frame register file in :meth:`_Compiler.finish`.
_REGISTERS: tuple[str, ...] = (_PTR, _VAL, _COND, _IMM)


class _Compiler:
    """Stateful AST walker that accumulates IIR instructions.

    The compiler keeps two things alive:

    - ``_instrs`` — the ordered list of ``IIRInstr`` we're building.
    - ``_loop_counter`` — depth-first index used to label nested loops
      uniquely (``bf_loop_0_start``, ``bf_loop_1_start``, …).

    All Brainfuck values live in four fixed registers (see ``_PTR``,
    ``_VAL``, ``_COND``, ``_IMM``).  Reusing the same names across
    instructions is fine: ``frame.assign`` overwrites the same register
    slot, and reads always pick up the most recent value.
    """

    def __init__(self) -> None:
        self._instrs: list[IIRInstr] = []
        self._loop_counter: int = 0

    # ------------------------------------------------------------------
    # Instruction-emit helpers
    # ------------------------------------------------------------------

    def _emit(
        self,
        op: str,
        dest: str | None,
        srcs: list[str | int | float | bool],
        type_hint: str,
    ) -> None:
        self._instrs.append(
            IIRInstr(op=op, dest=dest, srcs=srcs, type_hint=type_hint)
        )

    # ------------------------------------------------------------------
    # Program entry — prologue, body, epilogue
    # ------------------------------------------------------------------

    def emit_program(self, ast: ASTNode) -> None:
        """Walk the program AST and emit IIR for every instruction."""
        # Prologue: initialise the data pointer to cell 0.  Subsequent
        # ``>`` / ``<`` commands overwrite this register in place.
        self._emit("const", _PTR, [0], type_hint="u32")

        # The grammar guarantees the root is ``program`` with ``instruction``
        # children, but we accept any iterable of children to keep the
        # walker robust to grammar tweaks.
        for child in ast.children:
            if isinstance(child, ASTNode):
                self._emit_node(child)

        # Epilogue.  ``ret_void`` makes the dispatch loop pop the root
        # frame and terminate cleanly.  Without it, the interpreter would
        # implicitly fall off the end of the instruction list — which
        # also halts, but ``ret_void`` is the documented contract.
        self._emit("ret_void", None, [], type_hint="void")

    def finish(self) -> IIRFunction:
        # Four fixed registers (one per name in ``_REGISTERS``) plus a
        # generous headroom slot for vm-core internals — small constant.
        return IIRFunction(
            name="main",
            params=[],
            return_type="void",
            instructions=self._instrs,
            register_count=len(_REGISTERS) + 4,
            type_status=FunctionTypeStatus.FULLY_TYPED,
        )

    # ------------------------------------------------------------------
    # AST dispatch
    # ------------------------------------------------------------------

    def _emit_node(self, node: ASTNode) -> None:
        """Dispatch on the grammar rule that produced this node.

        The Brainfuck grammar (BF01) defines:

        - ``instruction`` — a single child that is either ``loop`` or
          ``command``.
        - ``loop`` — a ``[`` token, zero or more ``instruction`` children,
          and a ``]`` token.
        - ``command`` — a single token whose ``value`` is one of the eight
          Brainfuck command characters.
        """
        if node.rule_name == "instruction":
            for child in node.children:
                if isinstance(child, ASTNode):
                    self._emit_node(child)
        elif node.rule_name == "loop":
            self._emit_loop(node)
        elif node.rule_name == "command":
            self._emit_command(node)
        else:
            # Defensive: an unfamiliar rule means the grammar drifted.
            # Skipping silently would mask bugs; raise loudly instead.
            raise ValueError(f"unexpected AST rule: {node.rule_name!r}")

    # ------------------------------------------------------------------
    # Per-command emitters
    # ------------------------------------------------------------------

    def _emit_command(self, node: ASTNode) -> None:
        tok = _first_token(node)
        if tok is None:
            raise ValueError("command node has no token")
        char = tok.value

        if char == ">":
            self._emit_ptr_shift(delta=+1)
        elif char == "<":
            self._emit_ptr_shift(delta=-1)
        elif char == "+":
            self._emit_cell_mutation(delta=+1)
        elif char == "-":
            self._emit_cell_mutation(delta=-1)
        elif char == ".":
            self._emit_output()
        elif char == ",":
            self._emit_input()
        else:
            raise ValueError(f"unknown brainfuck command: {char!r}")

    def _emit_ptr_shift(self, *, delta: int) -> None:
        """Compile ``>`` (delta=+1) or ``<`` (delta=-1).

        We emit a ``const`` for the immediate and an ``add`` / ``sub`` that
        overwrites the pointer register in place.  We do NOT use ``add``
        with a negative immediate for ``<`` — the IIR's ``sub`` is the
        documented mnemonic for subtraction, and keeping the two cases
        symmetric makes the emitted IR easier to read.
        """
        self._emit("const", _IMM, [1], type_hint="u32")
        op = "add" if delta > 0 else "sub"
        self._emit(op, _PTR, [_PTR, _IMM], type_hint="u32")

    def _emit_cell_mutation(self, *, delta: int) -> None:
        """Compile ``+`` (delta=+1) or ``-`` (delta=-1).

        Sequence: load the cell, compute the new value, store back.  The
        ``u8_wrap=True`` knob on ``VMCore`` handles the 0↔255 wraparound
        automatically, so we don't need an explicit AND mask here.
        """
        self._emit("load_mem", _VAL, [_PTR], type_hint="u8")
        self._emit("const", _IMM, [1], type_hint="u8")
        op = "add" if delta > 0 else "sub"
        self._emit(op, _VAL, [_VAL, _IMM], type_hint="u8")
        self._emit("store_mem", None, [_PTR, _VAL], type_hint="u8")

    def _emit_output(self) -> None:
        """Compile ``.`` — write the current cell to stdout via ``putchar``.

        ``call_builtin`` dispatches into the host's :class:`BuiltinRegistry`,
        which the :class:`BrainfuckVM` wrapper populates with a closure that
        appends the byte to the run's output buffer.  Routing via a builtin
        (rather than the IIR's ``io_out`` instruction) keeps the wiring
        explicit at the host boundary and matches how Tetrad will eventually
        expose its own host interfaces.
        """
        self._emit("load_mem", _VAL, [_PTR], type_hint="u8")
        self._emit("call_builtin", None, ["putchar", _VAL], type_hint="void")

    def _emit_input(self) -> None:
        """Compile ``,`` — read one byte from stdin via ``getchar``."""
        self._emit("call_builtin", _VAL, ["getchar"], type_hint="u8")
        self._emit("store_mem", None, [_PTR, _VAL], type_hint="u8")

    # ------------------------------------------------------------------
    # Loop emitter
    # ------------------------------------------------------------------

    def _emit_loop(self, node: ASTNode) -> None:
        """Compile ``[ body ]`` into a structured WASM-friendly loop.

        IIR layout::

            label   loop_N_start
            load    c, ptr           ; read the current cell
            jmp_if_false c, loop_N_end
            ... body ...
            jmp     loop_N_start     ; unconditional back-edge
            label   loop_N_end

        Why this exact shape?
        ``ir-to-wasm-compiler`` recognises structured loops via the
        regex ``^loop_\\d+_start$`` plus a fixed pattern: a single
        forward conditional branch (``BRANCH_Z`` / ``BRANCH_NZ``) to the
        ``_end`` label, followed by an unconditional ``JUMP`` back to
        ``_start``.  Conditional back-edges are not supported.  Emitting
        the loop in this canonical shape is what makes Brainfuck JIT to
        WASM today (BF05) — see
        ``ir_to_wasm_compiler.compiler._emit_loop``.

        Brainfuck semantics ("loop while cell is non-zero") are
        preserved by the top-of-loop test: each iteration begins by
        re-loading and testing the cell, so any body that walks the
        pointer and re-points it before the back-edge runs (e.g. the
        canonical ``[->+<]``) reads the *new* current cell on the
        next iteration.

        The interpreter path does not care about label format or branch
        shape — it executes whatever IIR the compiler emits.  This shape
        works equally well there.
        """
        loop_id = self._loop_counter
        self._loop_counter += 1
        start_label = f"loop_{loop_id}_start"
        end_label = f"loop_{loop_id}_end"

        # Loop entry + per-iteration guard.
        self._emit("label", None, [start_label], type_hint="void")
        self._emit("load_mem", _COND, [_PTR], type_hint="u8")
        self._emit("jmp_if_false", None, [_COND, end_label], type_hint="void")

        # Body.  Skip the bracket tokens — emit only the ``instruction`` /
        # ``loop`` / ``command`` children.
        for child in node.children:
            if isinstance(child, ASTNode):
                self._emit_node(child)

        # Unconditional back-edge — the next iteration's top-of-loop
        # ``load_mem`` + ``jmp_if_false`` is what actually decides
        # whether to keep going.
        self._emit("jmp", None, [start_label], type_hint="void")
        self._emit("label", None, [end_label], type_hint="void")


# ---------------------------------------------------------------------------
# Token helpers
# ---------------------------------------------------------------------------


def _first_token(node: ASTNode) -> Any:
    """Return the first ``Token`` descendant of ``node``, or None.

    ``ASTNode.is_leaf`` and ``.token`` cover the simple case; we recurse
    into nested children for command-style nodes whose token sits one
    level deep.
    """
    if node.is_leaf and node.token is not None:
        return node.token
    for child in node.children:
        if isinstance(child, ASTNode):
            tok = _first_token(child)
            if tok is not None:
                return tok
        else:
            return child
    return None
