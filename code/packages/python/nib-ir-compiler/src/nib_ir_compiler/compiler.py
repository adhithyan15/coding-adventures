"""Nib IR Compiler — translates a typed Nib AST into general-purpose IR.

Overview
--------

This module is the Nib-specific frontend of the AOT compiler pipeline.
It walks the typed AST produced by ``nib-type-checker`` and emits IR
instructions (``IrInstruction``) for each node.

Stage in the pipeline::

    Source text
        → Nib Lexer          (characters → tokens)
        → Nib Parser         (tokens → untyped ASTNode tree)
        → Nib Type Checker   (untyped AST → typed AST)
        → Nib IR Compiler    (typed AST → IrProgram)   ← this module
        → Backend Validator  (IrProgram → validated IR)
        → Code Generator     (validated IR → machine code)

The compiler does NOT know about RISC-V, ARM, the Intel 4004, or any
specific machine target. Its only job is to translate Nib semantics into
target-independent IR.

Virtual Register Allocation (Fixed for Nib v1)
-----------------------------------------------

Nib v1 uses a simple fixed allocation scheme::

  v0  = zero constant (always 0 — preloaded at program start)
  v1  = scratch / expression result temporary
  v2+ = one virtual register per named variable (locals, params, statics
        accessed via LOAD_ADDR)

Registers are allocated in declaration order within each function scope.
A fresh allocation is created for every function (static scoping — no
register sharing between functions at the IR level).

For ``static`` variables: emit ``IrDataDecl(label=name, size=N, init=0)``
and access them with ``LOAD_ADDR`` + ``LOAD_BYTE``/``STORE_BYTE``.

Calling Convention
-------------------

The calling convention for Nib v1 is::

  - Arguments passed in v2, v3, v4, ... (caller-save)
  - Return value in v1
  - Callee uses its own fresh registers starting from v2

Key IR Emission Rules
----------------------

Declarations
~~~~~~~~~~~~

  ``const NAME: type = expr``
      No IR emitted. Constants are inlined at use sites by the type checker.

  ``static NAME: type = init``
      ``IrDataDecl(label=NAME, size=type.size_bytes, init=init_val)``

  ``fn NAME(params) -> type { body }``
      ``LABEL _fn_NAME``
      Compile body.
      Ensure ``RET`` at end.

Statements
~~~~~~~~~~

  ``let NAME: type = expr``
      Compile expr directly into the variable's register vN.

  ``NAME = expr``
      Compile expr into v1, then ``ADD_IMM vN, v1, 0`` (copy).

  ``return expr``
      Compile expr into v1; emit ``RET``.

  ``for i: type in start..end { body }``
      ``LOAD_IMM vI, start``
      ``LABEL loop_K_start``
      body
      ``ADD_IMM vI, vI, 1``
      ``CMP_LT vTmp, end_val, vI``   — note: end_val > vI means continue
      ``BRANCH_Z vTmp, loop_K_start`` — branch back while end_val > i
      ``LABEL loop_K_end``

  ``if cond { then } else { else }``
      Compile cond into vC.
      ``BRANCH_Z vC, else_K``
      then body
      ``JUMP end_K``
      ``LABEL else_K``
      else body (if present)
      ``LABEL end_K``

Expressions
~~~~~~~~~~~

See ``_compile_expr()`` for the full dispatch table. Key entries:

  INT_LIT / HEX_LIT  →  ``LOAD_IMM v1, n``
  true               →  ``LOAD_IMM v1, 1``
  false              →  ``LOAD_IMM v1, 0``
  NAME (variable)    →  return the variable's register
  NAME(args)         →  compile args into v2, v3, ...; ``CALL _fn_NAME``
  a +% b  (u4)       →  ``ADD vT, vA, vB``; ``AND_IMM vT, vT, 15``
  a +% b  (u8/bcd)   →  ``ADD vT, vA, vB``; ``AND_IMM vT, vT, 255``
  a - b              →  ``SUB vT, vA, vB``
  a == b             →  ``CMP_EQ vT, vA, vB``
  a != b             →  ``CMP_NE vT, vA, vB``
  a < b              →  ``CMP_LT vT, vA, vB``
  a > b              →  ``CMP_GT vT, vA, vB``
  a <= b             →  ``CMP_GT vT, vB, vA``  (LE(a,b) = GT(b,a))
  a >= b             →  ``CMP_LT vT, vB, vA``  (GE(a,b) = LT(b,a))
  !a                 →  ``CMP_EQ vT, vA, v0``  (true iff a == 0)
  a && b             →  ``AND vT, vA, vB``
  a || b             →  ``ADD vT, vA, vB``; ``CMP_NE vT, vT, v0``
  a & b              →  ``AND vT, vA, vB``
  ~a (u4)            →  ``XOR_IMM vT, vA, 0xF`` (complement nibble)
  ~a (u8)            →  ``XOR_IMM vT, vA, 0xFF``

(XOR is not in the current IrOp set; bitwise complement via ~a is emitted
as SUB-from-mask: AND_IMM + ADD_IMM chain, or stored as a COMMENT for v1.
In Nib v1, ``~`` is rare — we use a two-instruction sequence.)

Main Function
~~~~~~~~~~~~~

  ``_start`` label → ``CALL _fn_main`` → ``HALT``
"""

from __future__ import annotations

from dataclasses import dataclass, field

from compiler_ir import (
    IDGenerator,
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from compiler_source_map import SourceMapChain
from lang_parser import ASTNode
from lexer import Token
from nib_type_checker.types import NibType

from nib_ir_compiler.build_config import BuildConfig

# ---------------------------------------------------------------------------
# Virtual register indices — fixed for Nib v1
# ---------------------------------------------------------------------------
#
# v0 = zero constant (always 0, preloaded in program prologue)
# v1 = scratch / expression result temporary (also the return value register)
# v2+ = named variables, allocated in declaration order within each function
#
# The calling convention:
#   Arguments:   v2, v3, v4, ... (caller passes args in these registers)
#   Return value: v1 (callee writes result here before RET)
#   Callee-local: v2+ (the callee allocates its own fresh reg set)
# ---------------------------------------------------------------------------

_REG_ZERO = 0    # v0: constant 0 (preloaded at _start)
_REG_SCRATCH = 1 # v1: scratch/expression temp / return value

# The first register available for named variables.
_REG_VAR_BASE = 2


# ---------------------------------------------------------------------------
# CompileResult — the outputs of compilation
# ---------------------------------------------------------------------------


@dataclass
class CompileResult:
    """The outputs of a successful Nib compilation.

    Attributes:
        program:    The compiled ``IrProgram`` containing all IR instructions,
                    data declarations, and the entry label ``_start``.
        source_map: The source map chain (currently only the IR segment is
                    populated; full source-to-IR mapping is a future feature).

    Example::

        from nib_parser import parse_nib
        from nib_type_checker import check
        from nib_ir_compiler import compile_nib

        ast = parse_nib("fn main() { let x: u4 = 5; }")
        result = check(ast)
        compiled = compile_nib(result.typed_ast)
        # compiled.program has the IR instructions
    """

    program: IrProgram
    source_map: SourceMapChain | None


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def compile_nib(
    typed_ast: ASTNode,
    config: BuildConfig | None = None,
) -> CompileResult:
    """Compile a typed Nib AST into IR and return the result.

    This is the main entry point for the compiler. It takes the annotated
    AST produced by ``nib_type_checker.check()``, and an optional
    ``BuildConfig`` that controls what the compiler emits.

    Args:
        typed_ast: The root ``ASTNode`` with ``rule_name == "program"``,
                   already annotated with ``._nib_type`` on every expression
                   node by the type checker.
        config:    Build configuration. Defaults to ``debug_config()``
                   (debug comments enabled).

    Returns:
        A ``CompileResult`` with the compiled ``IrProgram`` and source map.

    Raises:
        ValueError: If the AST root is not a ``"program"`` node.

    Example::

        from nib_parser import parse_nib
        from nib_type_checker import check
        from nib_ir_compiler import compile_nib, release_config

        ast = parse_nib("fn main() { let x: u4 = 5; }")
        result = check(ast)
        compiled = compile_nib(result.typed_ast, release_config())
    """
    if config is None:
        config = BuildConfig()

    if typed_ast.rule_name != "program":
        raise ValueError(
            f"expected 'program' AST node, got {typed_ast.rule_name!r}"
        )

    c = _Compiler(config=config)
    return c.compile(typed_ast)


# ---------------------------------------------------------------------------
# Internal compiler state
# ---------------------------------------------------------------------------


@dataclass
class _Compiler:
    """Internal compiler state — not part of the public API.

    Holds all mutable state needed during compilation. The public entry
    point ``compile_nib()`` creates one of these and calls ``compile()``.

    Attributes:
        config:      Build configuration.
        _id_gen:     Produces unique instruction IDs.
        _program:    The IR program being built.
        _loop_count: Counter for generating unique loop label names.
        _if_count:   Counter for generating unique if/else label names.
    """

    config: BuildConfig
    _id_gen: IDGenerator = field(default_factory=IDGenerator)
    _program: IrProgram = field(init=False)
    _loop_count: int = field(default=0)
    _if_count: int = field(default=0)
    _const_values: dict[str, int] = field(default_factory=dict)
    _next_free_reg: int = field(default=_REG_VAR_BASE)

    def __post_init__(self) -> None:
        """Initialize the IR program after field init."""
        self._program = IrProgram(entry_label="_start")

    def compile(self, ast: ASTNode) -> CompileResult:
        """Run compilation. Called once per compiler instance.

        Two phases:
          1. Collect all ``static`` declarations into ``.data`` segment.
          2. Compile all function declarations.
          3. Emit the ``_start`` entry point that calls ``main`` and halts.
        """
        # Phase 1: collect static data declarations (emitted into .data).
        for child in ast.children:
            inner = _unwrap_top_decl(child)
            if inner is None:
                continue
            if inner.rule_name == "const_decl":
                name, _nib_type, init_val = _extract_decl_info(inner)
                if name is not None:
                    self._const_values[name] = init_val
            if inner.rule_name == "static_decl":
                self._emit_static_data(inner)

        # Phase 2: emit the entry point.
        self._emit_entry_point(ast)

        # Phase 3: compile function bodies.
        for child in ast.children:
            inner = _unwrap_top_decl(child)
            if inner is None:
                continue
            if inner.rule_name == "fn_decl":
                self._compile_fn_decl(inner)

        return CompileResult(
            program=self._program,
            source_map=None,
        )

    # -----------------------------------------------------------------------
    # ID helpers
    # -----------------------------------------------------------------------

    def _emit(self, opcode: IrOp, *operands: IrRegister | IrImmediate | IrLabel) -> int:
        """Add one instruction to the program and return its unique ID."""
        instr_id = self._id_gen.next()
        self._program.add_instruction(
            IrInstruction(opcode=opcode, operands=list(operands), id=instr_id)
        )
        return instr_id

    def _emit_label(self, name: str) -> None:
        """Add a LABEL pseudo-instruction (labels have ID = -1)."""
        self._program.add_instruction(
            IrInstruction(
                opcode=IrOp.LABEL,
                operands=[IrLabel(name=name)],
                id=-1,
            )
        )

    def _emit_comment(self, text: str) -> None:
        """Emit a COMMENT pseudo-instruction (only in debug builds)."""
        if self.config.insert_debug_comments:
            self._program.add_instruction(
                IrInstruction(
                    opcode=IrOp.COMMENT,
                    operands=[IrLabel(name=text)],
                    id=-1,
                )
            )

    # -----------------------------------------------------------------------
    # Static data declarations
    # -----------------------------------------------------------------------

    def _emit_static_data(self, node: ASTNode) -> None:
        """Emit an IrDataDecl for a ``static`` declaration.

        Structure (static_decl):
            STATIC_kw  NAME  COLON  type  EQ  expr  SEMICOLON

        The ``size`` of the data region is determined by ``NibType.size_bytes``:
          - u4   → 1 byte  (nibble, byte-aligned)
          - u8   → 2 bytes (register pair)
          - bcd  → 1 byte  (nibble, byte-aligned)
          - bool → 1 byte  (nibble, byte-aligned)

        The initial value is extracted from the expression node. Only integer
        and hex literals are supported as static initializers in Nib v1 (the
        type checker enforces this).
        """
        name, nib_type, init_val = _extract_decl_info(node)
        if name is None or nib_type is None:
            return

        size = nib_type.size_bytes
        self._emit_comment(f"static {name}: {nib_type.value} = {init_val}")
        self._program.add_data(
            IrDataDecl(label=name, size=size, init=init_val)
        )

    # -----------------------------------------------------------------------
    # Entry point
    # -----------------------------------------------------------------------

    def _emit_entry_point(self, ast: ASTNode) -> None:
        """Emit the _start label, v0 = 0 setup, CALL _fn_main, HALT.

        The program entry point:

        .. code-block:: text

          LABEL     _start
          LOAD_IMM  v0, 0        ← v0 is the constant-zero register
          CALL      _fn_main     ← invoke the user's main function
          HALT                   ← terminate the program

        We check if a ``main`` function is declared. If not, we still emit
        the entry point so the IR is well-formed (the backend validator will
        catch the missing main).
        """
        self._emit_label("_start")
        self._emit_comment("program entry point: initialize v0=0, call main, halt")

        # v0 = 0: the constant-zero register, used in comparisons and as a
        # safe default operand. Every IR program in this pipeline preloads it.
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(index=_REG_ZERO),
            IrImmediate(value=0),
        )

        # Check if a main function is declared.
        has_main = _has_fn_named(ast, "main")
        if has_main:
            self._emit(IrOp.CALL, IrLabel(name="_fn_main"))

        self._emit(IrOp.HALT)

    # -----------------------------------------------------------------------
    # Function compilation
    # -----------------------------------------------------------------------

    def _compile_fn_decl(self, node: ASTNode) -> None:
        """Compile a function declaration into a sequence of IR instructions.

        Structure (fn_decl):
            FN_kw  NAME  LPAREN  [param_list]  RPAREN  [ARROW type]  block

        Each function compiles to:

        .. code-block:: text

          LABEL     _fn_NAME
          ... (prologue: nothing needed for Nib v1 — no frame pointer)
          ... body IR ...
          RET        ← always emitted at end (even void functions)

        Register allocation is fresh for each function: each local variable
        and parameter gets its own virtual register starting from v2.
        """
        fn_name: str | None = None
        params: list[tuple[str, NibType]] = []
        block_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token):
                t_name = _tok_type(child)
                if t_name == "NAME" and fn_name is None:
                    fn_name = child.value
            elif isinstance(child, ASTNode):
                if child.rule_name == "param_list":
                    params = _extract_params(child)
                elif child.rule_name == "block":
                    block_node = child

        if fn_name is None or block_node is None:
            return

        # Emit the function label.
        self._emit_comment(f"function: {fn_name}({', '.join(f'{n}: {t.value}' for n, t in params)})")
        self._emit_label(f"_fn_{fn_name}")

        # Build fresh register allocation for this function scope.
        # Parameters occupy v2, v3, ... in order.
        regs: dict[str, IrRegister] = {}
        next_reg = _REG_VAR_BASE
        for param_name, _ in params:
            regs[param_name] = IrRegister(index=next_reg)
            next_reg += 1
        self._next_free_reg = next_reg

        # Compile the function body (block).
        next_reg = self._compile_block(block_node, regs, next_reg)
        self._next_free_reg = next_reg

        # Always emit RET at end of function to ensure well-formed IR.
        # (If the source has an explicit return, we'll have already emitted one,
        # but a trailing RET is harmless — the backend can remove dead code.)
        self._emit(IrOp.RET)

    # -----------------------------------------------------------------------
    # Block and statement compilation
    # -----------------------------------------------------------------------

    def _compile_block(
        self,
        block: ASTNode,
        regs: dict[str, IrRegister],
        next_reg: int,
    ) -> int:
        """Compile all statements in a ``{ ... }`` block.

        Args:
            block:    The ``block`` ASTNode.
            regs:     Current variable → register mapping (mutated in place
                      as ``let`` declarations are processed).
            next_reg: The next free virtual register index.

        Returns:
            The updated ``next_reg`` after all statements in the block.
        """
        self._next_free_reg = max(self._next_free_reg, next_reg)
        for child in block.children:
            if isinstance(child, ASTNode) and child.rule_name == "stmt":
                next_reg = self._compile_stmt(child, regs, next_reg)
                self._next_free_reg = max(self._next_free_reg, next_reg)
        return next_reg

    def _compile_stmt(
        self,
        stmt: ASTNode,
        regs: dict[str, IrRegister],
        next_reg: int,
    ) -> int:
        """Dispatch a statement node to the appropriate handler.

        Args:
            stmt:     A ``stmt`` ASTNode (wraps the inner statement).
            regs:     Variable → register mapping (mutated by let_stmt).
            next_reg: Next free register index.

        Returns:
            Updated ``next_reg``.
        """
        if not stmt.children:
            return next_reg

        inner = stmt.children[0]
        if not isinstance(inner, ASTNode):
            return next_reg

        rule = inner.rule_name

        if rule == "let_stmt":
            next_reg = self._compile_let_stmt(inner, regs, next_reg)
        elif rule == "assign_stmt":
            self._compile_assign_stmt(inner, regs)
        elif rule == "return_stmt":
            self._compile_return_stmt(inner, regs)
        elif rule == "for_stmt":
            next_reg = self._compile_for_stmt(inner, regs, next_reg)
        elif rule == "if_stmt":
            next_reg = self._compile_if_stmt(inner, regs, next_reg)
        elif rule == "expr_stmt":
            # Expression used as a statement — compile for side effects.
            if inner.children:
                expr_child = inner.children[0]
                if isinstance(expr_child, ASTNode):
                    self._compile_expr(expr_child, regs)

        return next_reg

    # -----------------------------------------------------------------------
    # Statement compilers
    # -----------------------------------------------------------------------

    def _compile_let_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
        next_reg: int,
    ) -> int:
        """Compile a ``let`` declaration.

        Structure: LET_kw  NAME  COLON  type  EQ  expr  SEMICOLON

        Allocates a fresh virtual register vN for the variable, then compiles
        the initializer expression into a scratch register, and copies the
        result to vN via ADD_IMM vN, v1, 0.

        Simpler path: compile the expression directly into a target register.
        We use v1 as the scratch and then bind the variable to a fresh vN.
        After compilation, ``regs[name] = IrRegister(next_reg)`` and the
        value in v1 is moved to vN.

        Args:
            node:     The ``let_stmt`` ASTNode.
            regs:     Variable → register mapping (mutated to add new binding).
            next_reg: Next free register index.

        Returns:
            Updated ``next_reg`` (incremented by 1 for the new variable).
        """
        name_tok: Token | None = None
        nib_type: NibType | None = None
        expr_node: ASTNode | None = None

        found_type = False
        for child in node.children:
            if isinstance(child, Token):
                t_name = _tok_type(child)
                if t_name == "NAME" and name_tok is None:
                    name_tok = child
            elif isinstance(child, ASTNode):
                if child.rule_name == "type" and not found_type:
                    nib_type = _resolve_type_node(child)
                    found_type = True
                elif _is_expr_node(child) and found_type:
                    expr_node = child

        if name_tok is None or expr_node is None:
            return next_reg

        type_str = nib_type.value if nib_type else "?"
        self._emit_comment(f"let {name_tok.value}: {type_str}")

        # Allocate a fresh register for this variable.
        var_reg = IrRegister(index=next_reg)
        regs[name_tok.value] = var_reg
        next_reg += 1

        # Propagate the declared type down to expression sub-nodes so that
        # _compile_add_expr picks the correct AND_IMM mask (15 for u4, 255 for
        # u8/bcd).  The type checker annotates numeric literals with U4 as an
        # untyped sentinel; without propagation, "100 +% 200" in a u8 context
        # would incorrectly emit AND_IMM 15 instead of AND_IMM 255.
        if nib_type is not None:
            from nib_type_checker.types import NibType as _NT
            if nib_type != _NT.U4:
                _propagate_context_type(expr_node, nib_type)

        # Compile the initializer expression. The result lands in the scratch
        # register (v1). We then copy v1 → vN via ADD_IMM vN, v1, 0.
        result_reg = self._compile_expr(expr_node, regs)

        if result_reg.index != var_reg.index:
            # Copy result into the variable's dedicated register.
            self._emit(
                IrOp.ADD_IMM,
                var_reg,
                result_reg,
                IrImmediate(value=0),
            )

        return next_reg

    def _compile_assign_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile a variable assignment.

        Structure: NAME  EQ  expr  SEMICOLON

        Compiles the expression into v1, then copies v1 to the variable's
        register with ``ADD_IMM vN, v1, 0``.
        """
        name_tok: Token | None = None
        expr_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token):
                t_name = _tok_type(child)
                if t_name == "NAME" and name_tok is None:
                    name_tok = child
            elif isinstance(child, ASTNode) and _is_expr_node(child) and name_tok is not None:
                expr_node = child

        if name_tok is None or expr_node is None:
            return

        self._emit_comment(f"assign {name_tok.value}")

        result_reg = self._compile_expr(expr_node, regs)
        var_reg = regs.get(name_tok.value)

        if var_reg is not None and result_reg.index != var_reg.index:
            self._emit(
                IrOp.ADD_IMM,
                var_reg,
                result_reg,
                IrImmediate(value=0),
            )

    def _compile_return_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile a ``return`` statement.

        Structure: RETURN_kw  expr  SEMICOLON

        Compiles the expression into v1 (the return value register), then
        emits ``RET``. The calling convention requires the return value in v1.
        """
        expr_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode) and _is_expr_node(child):
                expr_node = child
                break

        self._emit_comment("return")

        if expr_node is not None:
            result_reg = self._compile_expr(expr_node, regs)
            # Ensure return value is in v1 (the calling-convention return reg).
            if result_reg.index != _REG_SCRATCH:
                self._emit(
                    IrOp.ADD_IMM,
                    IrRegister(index=_REG_SCRATCH),
                    result_reg,
                    IrImmediate(value=0),
                )

        self._emit(IrOp.RET)

    def _compile_for_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
        next_reg: int,
    ) -> int:
        """Compile a ``for i: type in start..end { body }`` loop.

        Structure: FOR_kw  NAME  COLON  type  IN_kw  expr  RANGE  expr  block

        Compiled IR sequence:

        .. code-block:: text

          LOAD_IMM  vI, start      ← initialise loop variable
          LABEL     loop_K_start
          CMP_LT    vTmp, vI, end  ← test: is i < end?
          BRANCH_Z  vTmp, loop_K_end  ← exit if NOT less
          ... body ...
          ADD_IMM   vI, vI, 1      ← increment
          JUMP      loop_K_start   ← repeat
          LABEL     loop_K_end

        Args:
            node:     The ``for_stmt`` ASTNode.
            regs:     Variable → register mapping (mutated to add loop var).
            next_reg: Next free register index.

        Returns:
            Updated ``next_reg``.
        """
        loop_var_tok: Token | None = None
        loop_type: NibType | None = None
        bound_exprs: list[ASTNode] = []
        block_node: ASTNode | None = None

        found_type = False
        for child in node.children:
            if isinstance(child, Token):
                t_name = _tok_type(child)
                if t_name == "NAME" and loop_var_tok is None:
                    loop_var_tok = child
            elif isinstance(child, ASTNode):
                if child.rule_name == "type" and not found_type:
                    loop_type = _resolve_type_node(child)
                    found_type = True
                elif child.rule_name == "block":
                    block_node = child
                elif _is_expr_node(child) and found_type:
                    bound_exprs.append(child)

        if loop_var_tok is None or block_node is None:
            return next_reg

        type_str = loop_type.value if loop_type else "?"
        self._emit_comment(f"for {loop_var_tok.value}: {type_str}")

        default_start = Token(type="INT_LIT", value="0", line=1, column=1)
        default_end = Token(type="INT_LIT", value="1", line=1, column=1)

        loop_reg = IrRegister(index=next_reg)
        next_reg += 1
        end_reg = IrRegister(index=next_reg)
        next_reg += 1
        cmp_reg = IrRegister(index=next_reg)
        next_reg += 1
        self._next_free_reg = max(self._next_free_reg, next_reg)

        # Evaluate the bounds once when entering the loop, then keep the
        # cached end bound in a dedicated register for the duration.
        start_source_reg = self._compile_expr(
            bound_exprs[0] if len(bound_exprs) >= 1 else default_start,
            regs,
        )
        if start_source_reg.index != loop_reg.index:
            self._emit(IrOp.ADD_IMM, loop_reg, start_source_reg, IrImmediate(value=0))

        end_source_reg = self._compile_expr(
            bound_exprs[1] if len(bound_exprs) >= 2 else default_end,
            regs,
        )
        if end_source_reg.index != end_reg.index:
            self._emit(IrOp.ADD_IMM, end_reg, end_source_reg, IrImmediate(value=0))

        regs[loop_var_tok.value] = loop_reg

        loop_num = self._loop_count
        self._loop_count += 1
        start_label = f"loop_{loop_num}_start"
        end_label = f"loop_{loop_num}_end"

        self._emit_label(start_label)

        # Test condition: cmp_reg = (loop_reg < end_reg)
        self._emit(
            IrOp.CMP_LT,
            cmp_reg,
            loop_reg,
            end_reg,
        )
        # Branch to end if condition is false (i.e., i >= end_val).
        self._emit(
            IrOp.BRANCH_Z,
            cmp_reg,
            IrLabel(name=end_label),
        )

        # Compile the loop body.
        next_reg = self._compile_block(block_node, regs, next_reg)

        # Increment the loop variable.
        self._emit(
            IrOp.ADD_IMM,
            loop_reg,
            loop_reg,
            IrImmediate(value=1),
        )

        # Jump back to the loop start.
        self._emit(IrOp.JUMP, IrLabel(name=start_label))

        self._emit_label(end_label)

        return next_reg

    def _compile_if_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
        next_reg: int,
    ) -> int:
        """Compile an ``if cond { then } [else { else }]`` statement.

        Structure: IF_kw  expr  block  [ELSE_kw  block]

        Compiled IR sequence:

        .. code-block:: text

          ... compile cond into vC ...
          BRANCH_Z  vC, else_K    ← skip then-block if cond == 0
          ... then block ...
          JUMP      end_K         ← skip else-block
          LABEL     else_K
          ... else block (if present) ...
          LABEL     end_K

        Args:
            node:     The ``if_stmt`` ASTNode.
            regs:     Variable → register mapping.
            next_reg: Next free register index.

        Returns:
            Updated ``next_reg``.
        """
        cond_expr: ASTNode | None = None
        blocks: list[ASTNode] = []

        for child in node.children:
            if isinstance(child, ASTNode):
                if _is_expr_node(child) and cond_expr is None:
                    cond_expr = child
                elif child.rule_name == "block":
                    blocks.append(child)

        if_num = self._if_count
        self._if_count += 1
        else_label = f"if_{if_num}_else"
        end_label = f"if_{if_num}_end"

        self._emit_comment("if statement")

        # Compile condition.
        if cond_expr is not None:
            cond_reg = self._compile_expr(cond_expr, regs)
        else:
            # Degenerate case: no condition expression found. Default to v1.
            cond_reg = IrRegister(index=_REG_SCRATCH)

        # Branch to else/end if condition is false (== 0).
        self._emit(
            IrOp.BRANCH_Z,
            cond_reg,
            IrLabel(name=else_label),
        )

        # Compile then-block.
        if blocks:
            next_reg = self._compile_block(blocks[0], regs, next_reg)

        # Jump past else-block.
        self._emit(IrOp.JUMP, IrLabel(name=end_label))

        self._emit_label(else_label)

        # Compile else-block (if present).
        if len(blocks) >= 2:
            next_reg = self._compile_block(blocks[1], regs, next_reg)

        self._emit_label(end_label)

        return next_reg

    # -----------------------------------------------------------------------
    # Expression compiler
    # -----------------------------------------------------------------------

    def _compile_expr(
        self,
        node: ASTNode | Token,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile an expression node. Returns the register holding the result.

        The result register is either:
          - ``v1`` (the scratch register) for literals, binary ops, and calls.
          - A variable's dedicated register (vN) for name references.

        This recursive function dispatches on ``rule_name`` for ASTNodes, and
        on ``token.type`` for leaf tokens.

        Args:
            node: An expression ``ASTNode`` or ``Token``.
            regs: Variable → register mapping for the current scope.

        Returns:
            The ``IrRegister`` that holds the expression result.
        """
        if isinstance(node, Token):
            return self._compile_token_expr(node, regs)

        rule = node.rule_name

        # Single-child passthrough nodes — unwrap and recurse.
        if rule in (
            "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
            "bitwise_expr", "unary_expr",
        ):
            # Check for binary operator (3+ children) vs. single-child passthrough.
            children = node.children
            if len(children) == 1:
                return self._compile_expr(children[0], regs)
            # Binary operator node.
            return self._compile_compound_expr(node, regs)

        if rule == "add_expr":
            children = node.children
            if len(children) == 1:
                return self._compile_expr(children[0], regs)
            return self._compile_add_expr(node, regs)

        if rule == "primary":
            return self._compile_primary(node, regs)

        if rule == "call_expr":
            return self._compile_call_expr(node, regs)

        # Fallback: try single-child unwrap.
        if node.children:
            return self._compile_expr(node.children[0], regs)

        # Should not reach here — return scratch as a safe default.
        return IrRegister(index=_REG_SCRATCH)

    def _compile_token_expr(
        self,
        tok: Token,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a bare token (INT_LIT, HEX_LIT, true, false, or NAME)."""
        t_name = _tok_type(tok)
        scratch = IrRegister(index=_REG_SCRATCH)

        if t_name == "INT_LIT":
            # Integer literal: load the decimal value into v1.
            val = int(tok.value)
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=val))
            return scratch

        if t_name == "HEX_LIT":
            # Hexadecimal literal: parse and load into v1.
            val = int(tok.value, 16)
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=val))
            return scratch

        # After the nib-lexer keyword reclassification, `true` and `false`
        # tokens have type="true"/"false" (not type="NAME").  Handle both.
        if t_name in ("true", "false"):
            val = 1 if t_name == "true" else 0
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=val))
            return scratch

        if t_name == "NAME":
            val_str = tok.value
            if val_str == "true":
                # Boolean true → 1.
                self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=1))
                return scratch
            if val_str == "false":
                # Boolean false → 0.
                self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=0))
                return scratch
            if val_str in self._const_values:
                self._emit(
                    IrOp.LOAD_IMM,
                    scratch,
                    IrImmediate(value=self._const_values[val_str]),
                )
                return scratch
            # Variable reference: return the variable's dedicated register.
            if val_str in regs:
                return regs[val_str]
            # Unknown name (static variable or const) — fall back to scratch.
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=0))
            return scratch

        return scratch

    def _compile_primary(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a ``primary`` expression node.

        Primary expressions are the leaves of the expression tree:
          - Integer literals (INT_LIT, HEX_LIT)
          - Boolean literals (true, false)
          - Variable names (NAME)
          - Function calls (call_expr)
          - Parenthesized expressions: ``(`` expr ``)``
        """
        if not node.children:
            return IrRegister(index=_REG_SCRATCH)

        first = node.children[0]

        # Token primary: literal or name.
        if isinstance(first, Token):
            return self._compile_token_expr(first, regs)

        # ASTNode primary: call_expr or parenthesized expr.
        if isinstance(first, ASTNode):
            if first.rule_name == "call_expr":
                return self._compile_call_expr(first, regs)
            return self._compile_expr(first, regs)

        # Parenthesized: (expr) — first child is LPAREN token, second is expr.
        if len(node.children) >= 2:
            return self._compile_expr(node.children[1], regs)

        return IrRegister(index=_REG_SCRATCH)

    def _compile_call_expr(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a function call expression.

        Structure: NAME  LPAREN  [arg_list]  RPAREN

        Calling convention:
          - Arguments are compiled into v2, v3, v4, ... (caller-save).
          - ``CALL _fn_NAME`` invokes the function.
          - Return value is in v1 after the call.

        Args:
            node: The ``call_expr`` ASTNode.
            regs: Current variable → register mapping.

        Returns:
            ``v1`` (the return value register).
        """
        fn_tok: Token | None = None
        arg_exprs: list[ASTNode] = []

        for child in node.children:
            if isinstance(child, Token):
                t_name = _tok_type(child)
                if t_name == "NAME" and fn_tok is None:
                    fn_tok = child
            elif isinstance(child, ASTNode) and child.rule_name == "arg_list":
                for ac in child.children:
                    if isinstance(ac, ASTNode) and _is_expr_node(ac):
                        arg_exprs.append(ac)

        if fn_tok is None:
            return IrRegister(index=_REG_SCRATCH)

        self._emit_comment(f"call {fn_tok.value}({len(arg_exprs)} args)")

        # Preserve caller locals before repurposing v2, v3, ... for outbound
        # arguments. This keeps calls composable even when the caller has live
        # locals already resident in the argument registers.
        live_regs = sorted({reg.index for reg in regs.values()})
        next_temp_reg = self._next_free_reg
        saved_regs: list[tuple[IrRegister, IrRegister]] = []
        for reg_index in live_regs:
            original = IrRegister(index=reg_index)
            saved = IrRegister(index=next_temp_reg)
            next_temp_reg += 1
            self._emit(
                IrOp.ADD_IMM,
                saved,
                original,
                IrImmediate(value=0),
            )
            saved_regs.append((original, saved))

        # Compile arguments into temporary registers first so later argument
        # setup cannot clobber earlier results before the CALL is emitted.
        arg_temps: list[IrRegister] = []
        for arg_expr in arg_exprs:
            temp_reg = IrRegister(index=next_temp_reg)
            next_temp_reg += 1
            result_reg = self._compile_expr(arg_expr, regs)
            if result_reg.index != temp_reg.index:
                self._emit(
                    IrOp.ADD_IMM,
                    temp_reg,
                    result_reg,
                    IrImmediate(value=0),
                )
            arg_temps.append(temp_reg)

        self._next_free_reg = max(self._next_free_reg, next_temp_reg)

        # Move the temporary argument values into the ABI-defined call slots.
        for i, temp_reg in enumerate(arg_temps):
            arg_reg = IrRegister(index=_REG_VAR_BASE + i)
            if temp_reg.index == arg_reg.index:
                continue
            self._emit(
                IrOp.ADD_IMM,
                arg_reg,
                temp_reg,
                IrImmediate(value=0),
            )

        self._emit(IrOp.CALL, IrLabel(name=f"_fn_{fn_tok.value}"))

        for original, saved in saved_regs:
            self._emit(
                IrOp.ADD_IMM,
                original,
                saved,
                IrImmediate(value=0),
            )

        return IrRegister(index=_REG_SCRATCH)

    def _compile_compound_expr(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a compound expression (binary or unary operator).

        Handles: or_expr, and_expr, eq_expr, cmp_expr, bitwise_expr, unary_expr.

        For binary operators: children alternate operand/op/operand. For
        unary operators: first child is the operator token, second is the
        operand.

        Returns the register holding the result (usually v1 scratch).
        """
        rule = node.rule_name
        children = node.children
        scratch = IrRegister(index=_REG_SCRATCH)

        # Unary operator: BANG (!) or TILDE (~).
        if rule == "unary_expr" and len(children) == 2:
            op_child = children[0]
            operand_node = children[1]
            if isinstance(op_child, Token):
                op_val = op_child.value
                operand_reg = self._compile_expr(operand_node, regs)
                return self._emit_unary_op(op_val, operand_reg, node)

        # Binary operator: left op right (and possibly more: a op b op c ...).
        # We process the chain left-to-right, folding into a single result.
        if len(children) < 3:
            if children:
                return self._compile_expr(children[0], regs)
            return scratch

        # Compile left operand.
        left_reg = self._compile_expr(children[0], regs)

        i = 1
        while i < len(children) - 1:
            op_child = children[i]
            right_node = children[i + 1]

            op_val = ""
            if isinstance(op_child, Token):
                op_val = op_child.value
            elif isinstance(op_child, ASTNode):
                op_val = _first_token_value(op_child)

            right_reg = self._compile_expr(right_node, regs)
            result_reg = self._emit_binary_op(op_val, left_reg, right_reg, node)
            left_reg = result_reg
            i += 2

        return left_reg

    def _compile_add_expr(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile an add_expr node (arithmetic operators +, +%, -, etc.).

        Structure: bitwise_expr { (PLUS | MINUS | WRAP_ADD | SAT_ADD) bitwise_expr }

        Nib-specific: the ``+%`` operator (WRAP_ADD) emits a mask after ADD:
          u4:  ``AND_IMM vT, vT, 15``
          u8/bcd: ``AND_IMM vT, vT, 255``

        The type is read from the ``_nib_type`` attribute set by the type checker.
        """
        children = node.children
        scratch = IrRegister(index=_REG_SCRATCH)

        if len(children) < 3:
            if children:
                return self._compile_expr(children[0], regs)
            return scratch

        # Compile left operand.
        left_reg = self._compile_expr(children[0], regs)

        i = 1
        while i < len(children) - 1:
            op_child = children[i]
            right_node = children[i + 1]

            op_val = ""
            if isinstance(op_child, Token):
                op_val = op_child.value
            elif isinstance(op_child, ASTNode):
                op_val = _first_token_value(op_child)

            right_reg = self._compile_expr(right_node, regs)

            # Determine the expression's NibType from annotation (if available).
            nib_type: NibType | None = getattr(node, "_nib_type", None)

            result_reg = self._emit_add_op(op_val, left_reg, right_reg, nib_type)
            left_reg = result_reg
            i += 2

        return left_reg

    # -----------------------------------------------------------------------
    # IR emission helpers for individual operators
    # -----------------------------------------------------------------------

    def _fresh_reg(self, regs: dict[str, IrRegister]) -> IrRegister:
        """Allocate a fresh scratch register above all known variable regs.

        This avoids clobbering a named variable's register when computing
        a temporary sub-expression result. We use v1 as the canonical scratch;
        for nested temporaries, we use a register above all currently allocated
        named registers.

        In Nib v1, most expressions are simple enough that v1 suffices. This
        helper returns v1 as the universal scratch register for simplicity.
        """
        return IrRegister(index=_REG_SCRATCH)

    def _emit_unary_op(
        self,
        op_val: str,
        operand_reg: IrRegister,
        node: ASTNode,
    ) -> IrRegister:
        """Emit IR for a unary operator.

        Supported operators:

          ``!``  — logical NOT: ``CMP_EQ vT, vA, v0``
                   (true iff operand == 0 — the definition of logical NOT for booleans)

          ``~``  — bitwise complement:
                   u4: ``AND_IMM vT, 0xF, vA`` is impossible (no such opcode),
                   so we use ``SUB vT, vMaxMask, vA``:
                     LOAD_IMM  v1, mask
                     SUB       v1, v1, vA
                   where mask = 15 for u4, 255 for u8.

        Args:
            op_val:      The operator string (``"!"`` or ``"~"``).
            operand_reg: The register holding the operand.
            node:        The AST node (used to read ``_nib_type`` annotation).

        Returns:
            The register holding the result (v1 scratch).
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        zero = IrRegister(index=_REG_ZERO)

        if op_val == "!":
            # Logical NOT: result = (operand == 0) ? 1 : 0
            self._emit(
                IrOp.CMP_EQ,
                scratch,
                operand_reg,
                zero,
            )
            return scratch

        if op_val == "~":
            # Bitwise complement: result = mask XOR operand.
            # IrOp has no XOR, so we emulate: result = mask - operand.
            # (Valid because mask = all-1s in the type's bit width.)
            nib_type: NibType | None = getattr(node, "_nib_type", None)
            mask = 0xFF if (nib_type == NibType.U8) else 0xF

            # Load mask into scratch, then subtract operand.
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=mask))
            self._emit(IrOp.SUB, scratch, scratch, operand_reg)
            return scratch

        # Unknown unary op — return operand unchanged.
        return operand_reg

    def _emit_binary_op(
        self,
        op_val: str,
        left_reg: IrRegister,
        right_reg: IrRegister,
        node: ASTNode,
    ) -> IrRegister:
        """Emit IR for a generic binary operator (non-arithmetic).

        Handles: ``==``, ``!=``, ``<``, ``>``, ``<=``, ``>=``, ``&&``, ``||``, ``&``.

        Args:
            op_val:    The operator string (e.g., ``"=="``, ``"&&"``).
            left_reg:  Register holding the left operand.
            right_reg: Register holding the right operand.
            node:      The AST node (for ``_nib_type`` annotation if needed).

        Returns:
            The register holding the result.
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        zero = IrRegister(index=_REG_ZERO)

        if op_val == "==":
            self._emit(IrOp.CMP_EQ, scratch, left_reg, right_reg)
            return scratch

        if op_val == "!=":
            self._emit(IrOp.CMP_NE, scratch, left_reg, right_reg)
            return scratch

        if op_val == "<":
            self._emit(IrOp.CMP_LT, scratch, left_reg, right_reg)
            return scratch

        if op_val == ">":
            self._emit(IrOp.CMP_GT, scratch, left_reg, right_reg)
            return scratch

        if op_val == "<=":
            # LE(a, b) ≡ GT(b, a): swap operands to turn ≤ into >.
            self._emit(IrOp.CMP_GT, scratch, right_reg, left_reg)
            return scratch

        if op_val == ">=":
            # GE(a, b) ≡ LT(b, a): swap operands to turn ≥ into <.
            self._emit(IrOp.CMP_LT, scratch, right_reg, left_reg)
            return scratch

        if op_val == "&&":
            # Logical AND: result = left & right (both are booleans 0/1).
            self._emit(IrOp.AND, scratch, left_reg, right_reg)
            return scratch

        if op_val == "||":
            # Logical OR: result = (left + right) != 0.
            # ADD then CMP_NE with 0 (the zero-constant register).
            self._emit(IrOp.ADD, scratch, left_reg, right_reg)
            self._emit(IrOp.CMP_NE, scratch, scratch, zero)
            return scratch

        if op_val == "&":
            # Bitwise AND.
            self._emit(IrOp.AND, scratch, left_reg, right_reg)
            return scratch

        # Unknown operator: return left unchanged as a safe default.
        return left_reg

    def _emit_add_op(
        self,
        op_val: str,
        left_reg: IrRegister,
        right_reg: IrRegister,
        nib_type: NibType | None,
    ) -> IrRegister:
        """Emit IR for an arithmetic operator (+, +%, -, +?).

        The key Nib-specific behaviour is the wrapping addition ``+%``:

          u4  → ``ADD vT, vA, vB``; ``AND_IMM vT, vT, 15``
                (keep result in [0, 15])

          u8  → ``ADD vT, vA, vB``; ``AND_IMM vT, vT, 255``
                (keep result in [0, 255])

          bcd → ``ADD vT, vA, vB``; ``AND_IMM vT, vT, 255``
                (the backend emits DAA after ADD; we tag with a COMMENT)

        For ``-``: straightforward ``SUB vT, vA, vB``.

        Args:
            op_val:    The operator token value (``"+%"``, ``"-"``, ``"+"``).
            left_reg:  Register holding the left operand.
            right_reg: Register holding the right operand.
            nib_type:  The type annotation of the expression (or ``None``).

        Returns:
            The register holding the result (v1 scratch).
        """
        scratch = IrRegister(index=_REG_SCRATCH)

        if op_val == "+%":
            # Wrapping addition: ADD + AND mask.
            self._emit(IrOp.ADD, scratch, left_reg, right_reg)

            if nib_type == NibType.BCD:
                # BCD: backend needs to emit DAA after ADD.
                # We emit a COMMENT to signal this intent.
                self._emit_comment("bcd +%: backend should emit DAA after ADD")
                # Still AND to keep within byte range.
                self._emit(IrOp.AND_IMM, scratch, scratch, IrImmediate(value=255))
            elif nib_type == NibType.U4:
                # u4: keep result in [0, 15] with a 4-bit mask.
                self._emit(IrOp.AND_IMM, scratch, scratch, IrImmediate(value=15))
            else:
                # u8 (or unknown): keep result in [0, 255] with an 8-bit mask.
                self._emit(IrOp.AND_IMM, scratch, scratch, IrImmediate(value=255))

            return scratch

        if op_val == "-":
            self._emit(IrOp.SUB, scratch, left_reg, right_reg)
            return scratch

        if op_val == "+":
            # Plain addition (not wrapping). Emit ADD without masking.
            # The type checker forbids bare + on bcd, so this is only for u4/u8.
            self._emit(IrOp.ADD, scratch, left_reg, right_reg)
            return scratch

        if op_val == "+?":
            # Saturating addition is a future feature. Emit plain ADD for now.
            # The backend validator will flag this if saturation is required.
            self._emit(IrOp.ADD, scratch, left_reg, right_reg)
            return scratch

        # Unknown operator: return left unchanged.
        return left_reg


# ---------------------------------------------------------------------------
# Module-level convenience function
# ---------------------------------------------------------------------------


_default_config = BuildConfig()


def compile_nib_simple(typed_ast: ASTNode) -> CompileResult:
    """Compile a typed Nib AST with the default (debug) configuration.

    This is a convenience alias for ``compile_nib(typed_ast, BuildConfig())``.

    Args:
        typed_ast: The root ``ASTNode`` annotated by the type checker.

    Returns:
        A ``CompileResult`` with the compiled program.

    Example::

        from nib_parser import parse_nib
        from nib_type_checker import check
        from nib_ir_compiler.compiler import compile_nib_simple

        ast = parse_nib("fn main() { }")
        result = check(ast)
        compiled = compile_nib_simple(result.typed_ast)
    """
    return compile_nib(typed_ast, _default_config)


# ---------------------------------------------------------------------------
# Private AST traversal helpers
# ---------------------------------------------------------------------------


def _tok_type(tok: Token) -> str:
    """Return the canonical type name string for a Token."""
    t = tok.type
    return t if isinstance(t, str) else t.name


def _unwrap_top_decl(child: ASTNode | object) -> ASTNode | None:
    """Unwrap a top_decl node to find its inner declaration ASTNode.

    The grammar structure is::

        program   → { top_decl }
        top_decl  → const_decl | static_decl | fn_decl

    We need to unwrap the ``top_decl`` wrapper to get to the actual
    declaration node.
    """
    if not isinstance(child, ASTNode):
        return None
    # child is a top_decl — its first ASTNode child is the actual declaration.
    for grandchild in child.children:
        if isinstance(grandchild, ASTNode):
            return grandchild
    return None


def _extract_decl_info(
    node: ASTNode,
) -> tuple[str | None, NibType | None, int]:
    """Extract (name, nib_type, init_val) from a const_decl or static_decl.

    Structure: [KEYWORD, NAME, COLON, type, EQ, expr, SEMICOLON]

    Walks the children to find:
      - The NAME token (index 1 among tokens).
      - The ``type`` ASTNode.
      - The initializer expression (an INT_LIT or HEX_LIT, or 0 as fallback).
    """
    name_tok: Token | None = None
    type_node: ASTNode | None = None
    init_val = 0

    token_idx = 0
    found_type = False

    for child in node.children:
        if isinstance(child, Token):
            t_name = _tok_type(child)
            if t_name == "NAME" and token_idx == 0:
                # First NAME token is the variable name.
                name_tok = child
                token_idx += 1
            elif t_name in ("INT_LIT", "HEX_LIT"):
                # Literal in initializer position.
                init_val = _parse_literal(child.value, t_name)
        elif isinstance(child, ASTNode):
            if child.rule_name == "type" and not found_type:
                type_node = child
                found_type = True
            elif _is_expr_node(child):
                # Extract literal value from expression.
                init_val = _extract_const_int(child)

    nib_type = _resolve_type_node(type_node) if type_node else None
    name = name_tok.value if name_tok else None
    return name, nib_type, init_val


def _extract_params(param_list_node: ASTNode) -> list[tuple[str, NibType]]:
    """Extract (name, NibType) pairs from a param_list node.

    Structure (param_list): [param, {COMMA, param}]
    Structure (param):       [NAME, COLON, type]
    """
    params: list[tuple[str, NibType]] = []
    for child in param_list_node.children:
        if isinstance(child, ASTNode) and child.rule_name == "param":
            name_tok: Token | None = None
            type_node: ASTNode | None = None
            for pc in child.children:
                if isinstance(pc, Token):
                    t_name = _tok_type(pc)
                    if t_name == "NAME" and name_tok is None:
                        name_tok = pc
                elif isinstance(pc, ASTNode) and pc.rule_name == "type":
                    type_node = pc
            if name_tok is not None and type_node is not None:
                nib_type = _resolve_type_node(type_node)
                if nib_type is not None:
                    params.append((name_tok.value, nib_type))
    return params


def _resolve_type_node(type_node: ASTNode | None) -> NibType | None:
    """Convert a ``type`` AST node to a NibType enum value.

    The ``type`` node is a leaf: its single token has value one of
    ``"u4"``, ``"u8"``, ``"bcd"``, ``"bool"``.
    """
    if type_node is None:
        return None
    mapping: dict[str, NibType] = {
        "u4": NibType.U4,
        "u8": NibType.U8,
        "bcd": NibType.BCD,
        "bool": NibType.BOOL,
    }
    for child in type_node.children:
        if isinstance(child, Token):
            return mapping.get(child.value)
        if isinstance(child, ASTNode) and child.is_leaf and child.token:
            return mapping.get(child.token.value)
    return None


def _is_expr_node(node: ASTNode) -> bool:
    """Return True if node is an expression-level grammar rule."""
    return node.rule_name in (
        "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
        "add_expr", "bitwise_expr", "unary_expr", "primary", "call_expr",
    )


def _propagate_context_type(node: ASTNode | Token, declared_type: "NibType") -> None:
    """Override numeric-literal type annotations with the declared context type.

    The type checker annotates numeric literal expressions with NibType.U4 as
    a "representative" untyped sentinel.  When those literals appear inside a
    ``let`` or ``assign`` statement whose declared type is U8 or BCD, the IR
    compiler must use a 255 mask for ``AND_IMM`` (not 15).

    This helper walks the expression tree and replaces every NibType.U4 (or
    None) annotation with ``declared_type``, so ``_compile_add_expr`` reads
    the correct type.  We only override when ``declared_type != U4`` to avoid
    unnecessary mutation for purely u4 contexts.

    Nodes that already have a non-U4 type (e.g., a u8 variable reference) are
    left untouched — their type was set by the type checker from the symbol
    table and is authoritative.

    Args:
        node:          The root expression AST node to walk.
        declared_type: The type from the enclosing let/assign declaration.
    """
    if isinstance(node, Token):
        return
    from nib_type_checker.types import NibType  # local import to avoid circularity
    existing: NibType | None = getattr(node, "_nib_type", None)
    if existing is None or existing == NibType.U4:
        node._nib_type = declared_type  # type: ignore[attr-defined]
    for child in node.children:
        if isinstance(child, ASTNode):
            _propagate_context_type(child, declared_type)


def _extract_const_int(expr: ASTNode | Token) -> int:
    """Recursively extract an integer constant from a literal expression.

    Walks the expression tree looking for INT_LIT or HEX_LIT tokens.
    Returns 0 if no literal is found (safe fallback).
    """
    if isinstance(expr, Token):
        t_name = _tok_type(expr)
        if t_name in ("INT_LIT", "HEX_LIT"):
            return _parse_literal(expr.value, t_name)
        return 0
    # ASTNode: search children.
    for child in expr.children:
        val = _extract_const_int(child)
        if val != 0:
            return val
        # Also check if child is a leaf token.
        if isinstance(child, Token):
            t_name = _tok_type(child)
            if t_name in ("INT_LIT", "HEX_LIT"):
                return _parse_literal(child.value, t_name)
    return 0


def _parse_literal(value: str, t_name: str) -> int:
    """Parse a literal token value into an integer."""
    try:
        if t_name == "HEX_LIT":
            return int(value, 16)
        return int(value)
    except ValueError:
        return 0


def _has_fn_named(ast: ASTNode, name: str) -> bool:
    """Return True if the program AST contains a function named ``name``."""
    for child in ast.children:
        inner = _unwrap_top_decl(child)
        if inner is None:
            continue
        if inner.rule_name == "fn_decl":
            for c in inner.children:
                if isinstance(c, Token) and _tok_type(c) == "NAME" and c.value == name:
                    return True
    return False


def _first_token_value(node: ASTNode | Token) -> str:
    """Return the value of the first token found in ``node``."""
    if isinstance(node, Token):
        return node.value
    for child in node.children:
        val = _first_token_value(child)
        if val:
            return val
    return ""
