"""Brainfuck IR Compiler — translates a Brainfuck AST into general-purpose IR.

Overview
--------

This module is the Brainfuck-specific frontend of the AOT compiler pipeline.
It walks the AST produced by the Brainfuck parser and emits IR instructions
for each node. It also builds the first two segments of the source map chain:

  Segment 1: ``SourceToAst``  — source positions → AST node IDs
  Segment 2: ``AstToIr``      — AST node IDs → IR instruction IDs

The compiler does NOT know about RISC-V, ARM, ELF, or any specific machine
target. Its only job is to translate Brainfuck semantics into target-independent
IR.

Register Allocation
--------------------

Brainfuck needs very few registers:

::

  v0 = tape base address (pointer to the start of the tape)
  v1 = tape pointer offset (current cell index, 0-based)
  v2 = temporary (cell value for loads/stores)
  v3 = temporary (for bounds checks)
  v4 = syscall argument (cell value to output / value read from input)
  v5 = max pointer value (tape_size - 1, for bounds checks)
  v6 = zero constant (for lower bounds checks)

This fixed allocation maps directly to a small set of physical registers
in any ISA. Future languages (BASIC) that need more registers will use a
proper register allocator in the backend.

Command → IR Mapping
---------------------

Each Brainfuck command maps to a specific sequence of IR instructions:

.. code-block:: text

  ┌──────────────┬─────────────────────────────────────────────────────────┐
  │ Command      │ IR Output                                               │
  ├──────────────┼─────────────────────────────────────────────────────────┤
  │ > (RIGHT)    │ ADD_IMM v1, v1, 1                                       │
  │ < (LEFT)     │ ADD_IMM v1, v1, -1                                      │
  │ + (INC)      │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1;               │
  │              │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1             │
  │ - (DEC)      │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1;              │
  │              │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1             │
  │ . (OUTPUT)   │ LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1     │
  │ , (INPUT)    │ SYSCALL 2; STORE_BYTE v4, v0, v1                        │
  └──────────────┴─────────────────────────────────────────────────────────┘

Loop Compilation
-----------------

A Brainfuck loop ``[body]`` compiles to:

.. code-block:: text

  LABEL      loop_N_start
  LOAD_BYTE  v2, v0, v1          ← load current cell
  BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
  ...compile body...
  JUMP       loop_N_start        ← repeat
  LABEL      loop_N_end

Loops nest arbitrarily deep. Each loop gets a unique number N (from
``loop_count``) so labels are unique across the whole program.
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
from compiler_source_map import (
    SourceMapChain,
    SourcePosition,
)
from lang_parser import ASTNode
from lexer import Token

from brainfuck_ir_compiler.build_config import BuildConfig

# ---------------------------------------------------------------------------
# Virtual register indices — fixed allocation for Brainfuck
# ---------------------------------------------------------------------------
#
# Brainfuck needs only 7 virtual registers. We assign them by convention:
#
#   v0 = tape base address  (loaded from .data tape at startup)
#   v1 = tape pointer       (current cell index, 0-based)
#   v2 = cell temp          (used for loads/stores/arithmetic)
#   v3 = bounds temp        (used only in debug mode)
#   v4 = syscall arg        (cell value for output; read result for input)
#   v5 = max pointer        (tape_size - 1, for right bounds check)
#   v6 = zero constant      (0, for left bounds check comparisons)
# ---------------------------------------------------------------------------

_REG_TAPE_BASE = 0  # v0: base address of the tape
_REG_TAPE_PTR = 1   # v1: current cell offset
_REG_TEMP = 2       # v2: scratch register for cell values
_REG_TEMP2 = 3      # v3: scratch register for bounds checks
_REG_SYS_ARG = 4    # v4: syscall argument register
_REG_MAX_PTR = 5    # v5: tape_size - 1 (upper bounds check)
_REG_ZERO = 6       # v6: constant 0 (lower bounds check)

# ---------------------------------------------------------------------------
# Syscall numbers — match the RISC-V simulator's ecall dispatch
# ---------------------------------------------------------------------------

_SYSCALL_WRITE = 1   # write byte in a0 to stdout
_SYSCALL_READ = 2    # read byte from stdin into a0
_SYSCALL_EXIT = 10   # halt with exit code in a0


# ---------------------------------------------------------------------------
# CompileResult — the outputs of compilation
# ---------------------------------------------------------------------------


@dataclass
class CompileResult:
    """The outputs of a successful compilation.

    Attributes:
        program:    The compiled ``IrProgram`` containing all IR instructions.
        source_map: The source map chain with ``SourceToAst`` and ``AstToIr``
                    segments filled in by this compiler.

    Example::

        ast = parse_brainfuck("+.")
        result = compile_brainfuck(ast, "hello.bf", release_config())
        text = print_ir(result.program)
    """

    program: IrProgram
    source_map: SourceMapChain


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def compile_brainfuck(
    ast: ASTNode,
    filename: str,
    config: BuildConfig,
) -> CompileResult:
    """Compile a Brainfuck AST into IR and return the result.

    This is the main entry point for the compiler. It takes the AST produced
    by ``parse_brainfuck()``, a filename (used in source map entries), and a
    ``BuildConfig`` that controls what the compiler emits.

    Args:
        ast:      The root ``ASTNode`` with ``rule_name == "program"``.
        filename: Source file name for source map entries (e.g., ``"hello.bf"``).
        config:   Build configuration (debug or release).

    Returns:
        A ``CompileResult`` with the compiled program and source map.

    Raises:
        ValueError: If the AST root is not a ``"program"`` node, or if the
                    tape size is not positive.

    Example::

        from brainfuck import parse_brainfuck
        from brainfuck_ir_compiler import compile_brainfuck, release_config

        ast = parse_brainfuck("+.")
        result = compile_brainfuck(ast, "hello.bf", release_config())
        # result.program has the IR instructions
        # result.source_map has the source map chain
    """
    if ast.rule_name != "program":
        raise ValueError(
            f"expected 'program' AST node, got {ast.rule_name!r}"
        )
    if config.tape_size <= 0:
        raise ValueError(
            f"invalid tape_size {config.tape_size}: must be positive"
        )

    c = _Compiler(config=config, filename=filename)
    return c.compile(ast)


# ---------------------------------------------------------------------------
# Internal compiler state
# ---------------------------------------------------------------------------


@dataclass
class _Compiler:
    """Internal compiler state — not part of the public API.

    Holds all mutable state needed during compilation. The public entry
    point ``compile_brainfuck()`` creates one of these and calls ``compile()``.

    Attributes:
        config:      Build configuration.
        filename:    Source file name (for source map entries).
        _id_gen:     Produces unique instruction IDs.
        _node_id:    Monotonic AST node ID counter.
        _program:    The IR program being built.
        _source_map: The source map chain being built.
        _loop_count: Counter for generating unique loop label names.
    """

    config: BuildConfig
    filename: str
    _id_gen: IDGenerator = field(default_factory=IDGenerator)
    _node_id: int = field(default=0)
    _program: IrProgram = field(init=False)
    _source_map: SourceMapChain = field(init=False)
    _loop_count: int = field(default=0)

    def __post_init__(self) -> None:
        """Initialize program and source map after field init."""
        self._program = IrProgram(entry_label="_start")
        self._source_map = SourceMapChain.new()

    def compile(self, ast: ASTNode) -> CompileResult:
        """Run compilation. Called once per compiler instance."""
        # Add tape data declaration
        self._program.add_data(
            IrDataDecl(label="tape", size=self.config.tape_size, init=0)
        )

        # Emit prologue
        self._emit_prologue()

        # Compile the program body
        self._compile_program(ast)

        # Emit epilogue
        self._emit_epilogue()

        return CompileResult(
            program=self._program,
            source_map=self._source_map,
        )

    # -----------------------------------------------------------------------
    # ID helpers
    # -----------------------------------------------------------------------

    def _next_node_id(self) -> int:
        """Return the next unique AST node ID."""
        node_id = self._node_id
        self._node_id += 1
        return node_id

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

    # -----------------------------------------------------------------------
    # Prologue and Epilogue
    # -----------------------------------------------------------------------
    #
    # Prologue: sets up execution environment before any Brainfuck command runs.
    #   _start: label
    #   LOAD_ADDR v0, tape       — load base address of tape into v0
    #   LOAD_IMM  v1, 0          — tape pointer starts at cell 0
    #   (debug) LOAD_IMM v5, tape_size-1  — for bounds check upper bound
    #   (debug) LOAD_IMM v6, 0            — for bounds check lower bound
    #
    # Epilogue: terminates the program cleanly.
    #   HALT
    #   (debug) __trap_oob label + exit with code 1
    # -----------------------------------------------------------------------

    def _emit_prologue(self) -> None:
        """Emit the program entry point and register initialization."""
        self._emit_label("_start")

        # v0 = &tape (base address of the tape array)
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(index=_REG_TAPE_BASE),
            IrLabel(name="tape"),
        )

        # v1 = 0 (tape pointer starts at cell index 0)
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(index=_REG_TAPE_PTR),
            IrImmediate(value=0),
        )

        if self.config.insert_bounds_checks:
            # v5 = tape_size - 1 (maximum valid pointer value)
            self._emit(
                IrOp.LOAD_IMM,
                IrRegister(index=_REG_MAX_PTR),
                IrImmediate(value=self.config.tape_size - 1),
            )
            # v6 = 0 (minimum valid pointer value, for CMP_LT check)
            self._emit(
                IrOp.LOAD_IMM,
                IrRegister(index=_REG_ZERO),
                IrImmediate(value=0),
            )

    def _emit_epilogue(self) -> None:
        """Emit HALT and optional out-of-bounds trap handler."""
        self._emit(IrOp.HALT)

        if self.config.insert_bounds_checks:
            # Trap handler: jump here when pointer goes out of bounds.
            # Exits with error code 1.
            self._emit_label("__trap_oob")
            self._emit(
                IrOp.LOAD_IMM,
                IrRegister(index=_REG_SYS_ARG),
                IrImmediate(value=1),
            )
            self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_EXIT), IrRegister(index=_REG_SYS_ARG))

    # -----------------------------------------------------------------------
    # AST Walking
    # -----------------------------------------------------------------------
    #
    # The Brainfuck AST structure (from brainfuck.grammar):
    #
    #   program     → { instruction }
    #   instruction → loop | command
    #   loop        → LOOP_START { instruction } LOOP_END
    #   command     → RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
    #
    # The compiler walks this tree recursively, emitting IR for each node.
    # -----------------------------------------------------------------------

    def _compile_program(self, node: ASTNode) -> None:
        """Compile all top-level instructions in the program."""
        for child in node.children:
            if isinstance(child, ASTNode):
                self._compile_node(child)

    def _compile_node(self, node: ASTNode) -> None:
        """Dispatch compilation to the appropriate handler."""
        if node.rule_name == "instruction":
            # An instruction wraps either a loop or a command
            for child in node.children:
                if isinstance(child, ASTNode):
                    self._compile_node(child)

        elif node.rule_name == "command":
            self._compile_command(node)

        elif node.rule_name == "loop":
            self._compile_loop(node)

        else:
            raise ValueError(f"unexpected AST node type: {node.rule_name!r}")

    # -----------------------------------------------------------------------
    # Command compilation
    # -----------------------------------------------------------------------

    def _compile_command(self, node: ASTNode) -> None:
        """Compile a single Brainfuck command node into IR.

        Each command type maps to a characteristic sequence of IR instructions.
        All IR instructions for a command are recorded in the source map under
        a single AST node ID.

        Args:
            node: An ``ASTNode`` with ``rule_name == "command"``.

        Raises:
            ValueError: If the command token has an unexpected value.
        """
        tok = self._extract_token(node)
        if tok is None:
            raise ValueError("command node has no token")

        # Assign a unique AST node ID for source mapping
        ast_node_id = self._next_node_id()

        # Record source position → AST node ID
        self._source_map.source_to_ast.add(
            SourcePosition(
                file=self.filename,
                line=tok.line,
                column=tok.column,
                length=1,
            ),
            ast_node_id=ast_node_id,
        )

        ir_ids: list[int] = []

        if tok.value == ">":
            # RIGHT: move tape pointer one cell to the right.
            # With bounds checking: first verify pointer < tape_size.
            if self.config.insert_bounds_checks:
                ir_ids.extend(self._emit_bounds_check_right())
            ir_id = self._emit(
                IrOp.ADD_IMM,
                IrRegister(index=_REG_TAPE_PTR),
                IrRegister(index=_REG_TAPE_PTR),
                IrImmediate(value=1),
            )
            ir_ids.append(ir_id)

        elif tok.value == "<":
            # LEFT: move tape pointer one cell to the left.
            # With bounds checking: first verify pointer > 0.
            if self.config.insert_bounds_checks:
                ir_ids.extend(self._emit_bounds_check_left())
            ir_id = self._emit(
                IrOp.ADD_IMM,
                IrRegister(index=_REG_TAPE_PTR),
                IrRegister(index=_REG_TAPE_PTR),
                IrImmediate(value=-1),
            )
            ir_ids.append(ir_id)

        elif tok.value == "+":
            # INC: increment the current cell by 1 (wrapping at 255 → 0).
            ir_ids.extend(self._emit_cell_mutation(delta=1))

        elif tok.value == "-":
            # DEC: decrement the current cell by 1 (wrapping at 0 → 255).
            ir_ids.extend(self._emit_cell_mutation(delta=-1))

        elif tok.value == ".":
            # OUTPUT: write the current cell's value to stdout.
            #
            # We load the cell into v2, then copy to v4 (the syscall arg register)
            # using ADD_IMM v4, v2, 0, then call syscall 1 (write).
            id1 = self._emit(
                IrOp.LOAD_BYTE,
                IrRegister(index=_REG_TEMP),
                IrRegister(index=_REG_TAPE_BASE),
                IrRegister(index=_REG_TAPE_PTR),
            )
            ir_ids.append(id1)
            id2 = self._emit(
                IrOp.ADD_IMM,
                IrRegister(index=_REG_SYS_ARG),
                IrRegister(index=_REG_TEMP),
                IrImmediate(value=0),
            )
            ir_ids.append(id2)
            id3 = self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_WRITE), IrRegister(index=_REG_SYS_ARG))
            ir_ids.append(id3)

        elif tok.value == ",":
            # INPUT: read one byte from stdin into the current cell.
            #
            # syscall 2 (read) places the result in the syscall arg register (v4).
            # We then store it to the current cell.
            id1 = self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_READ), IrRegister(index=_REG_SYS_ARG))
            ir_ids.append(id1)
            id2 = self._emit(
                IrOp.STORE_BYTE,
                IrRegister(index=_REG_SYS_ARG),
                IrRegister(index=_REG_TAPE_BASE),
                IrRegister(index=_REG_TAPE_PTR),
            )
            ir_ids.append(id2)

        else:
            raise ValueError(f"unknown command token: {tok.value!r}")

        # Record AST node → IR instruction IDs
        self._source_map.ast_to_ir.add(ast_node_id=ast_node_id, ir_ids=ir_ids)

    def _emit_cell_mutation(self, delta: int) -> list[int]:
        """Emit IR for incrementing or decrementing the current cell.

        The sequence:

        .. code-block:: text

          LOAD_BYTE  v2, v0, v1         ← load current cell value
          ADD_IMM    v2, v2, delta       ← add +1 or -1
          AND_IMM    v2, v2, 255         ← (if masking) wrap to byte range
          STORE_BYTE v2, v0, v1         ← store back to cell

        Args:
            delta: The amount to add (1 for INC, -1 for DEC).

        Returns:
            The list of instruction IDs emitted.
        """
        ids: list[int] = []

        # Load the current cell value into v2
        ids.append(self._emit(
            IrOp.LOAD_BYTE,
            IrRegister(index=_REG_TEMP),
            IrRegister(index=_REG_TAPE_BASE),
            IrRegister(index=_REG_TAPE_PTR),
        ))

        # Add the delta (1 or -1)
        ids.append(self._emit(
            IrOp.ADD_IMM,
            IrRegister(index=_REG_TEMP),
            IrRegister(index=_REG_TEMP),
            IrImmediate(value=delta),
        ))

        # Mask to byte range [0, 255] if enabled.
        # AND_IMM v2, v2, 255 ensures: if cell was 255 and delta=1,
        # the result 256 becomes 256 & 255 = 0 (wraps to 0).
        if self.config.mask_byte_arithmetic:
            ids.append(self._emit(
                IrOp.AND_IMM,
                IrRegister(index=_REG_TEMP),
                IrRegister(index=_REG_TEMP),
                IrImmediate(value=255),
            ))

        # Store the result back to the current cell
        ids.append(self._emit(
            IrOp.STORE_BYTE,
            IrRegister(index=_REG_TEMP),
            IrRegister(index=_REG_TAPE_BASE),
            IrRegister(index=_REG_TAPE_PTR),
        ))

        return ids

    # -----------------------------------------------------------------------
    # Bounds checking
    # -----------------------------------------------------------------------
    #
    # In debug builds, the compiler inserts range checks before every pointer
    # move. If the pointer goes out of bounds, the program jumps to __trap_oob
    # (which exits with error code 1).
    #
    # RIGHT (>):
    #   CMP_GT  v3, v1, v5        ← is ptr > tape_size-1?
    #   BRANCH_NZ v3, __trap_oob  ← if so, trap
    #
    # LEFT (<):
    #   CMP_LT  v3, v1, v6        ← is ptr < 0?
    #   BRANCH_NZ v3, __trap_oob  ← if so, trap
    #
    # Note: the check happens BEFORE the pointer move, so we're checking
    # whether the pointer is ALREADY at the boundary before moving further.
    # -----------------------------------------------------------------------

    def _emit_bounds_check_right(self) -> list[int]:
        """Emit a right bounds check (before a '>' move).

        Returns:
            The list of instruction IDs emitted.
        """
        ids: list[int] = []
        ids.append(self._emit(
            IrOp.CMP_GT,
            IrRegister(index=_REG_TEMP2),
            IrRegister(index=_REG_TAPE_PTR),
            IrRegister(index=_REG_MAX_PTR),
        ))
        ids.append(self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(index=_REG_TEMP2),
            IrLabel(name="__trap_oob"),
        ))
        return ids

    def _emit_bounds_check_left(self) -> list[int]:
        """Emit a left bounds check (before a '<' move).

        Returns:
            The list of instruction IDs emitted.
        """
        ids: list[int] = []
        ids.append(self._emit(
            IrOp.CMP_LT,
            IrRegister(index=_REG_TAPE_PTR),
            IrRegister(index=_REG_TAPE_PTR),
            IrRegister(index=_REG_ZERO),
        ))
        ids.append(self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(index=_REG_TAPE_PTR),
            IrLabel(name="__trap_oob"),
        ))
        return ids

    # -----------------------------------------------------------------------
    # Loop compilation
    # -----------------------------------------------------------------------

    def _compile_loop(self, node: ASTNode) -> None:
        """Compile a Brainfuck loop node ``[body]`` into IR.

        A loop compiles to:

        .. code-block:: text

          LABEL      loop_N_start
          LOAD_BYTE  v2, v0, v1          ← load current cell
          BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
          ...compile body instructions...
          JUMP       loop_N_start        ← go back to check condition
          LABEL      loop_N_end

        Each loop gets a unique number ``N`` so labels don't collide when
        loops are nested.

        Args:
            node: An ``ASTNode`` with ``rule_name == "loop"``.
        """
        loop_num = self._loop_count
        self._loop_count += 1
        start_label = f"loop_{loop_num}_start"
        end_label = f"loop_{loop_num}_end"

        # Source map: record the loop's source position using its start_line/column
        ast_node_id = self._next_node_id()
        if node.start_line is not None and node.start_column is not None:
            self._source_map.source_to_ast.add(
                SourcePosition(
                    file=self.filename,
                    line=node.start_line,
                    column=node.start_column,
                    length=1,
                ),
                ast_node_id=ast_node_id,
            )

        ir_ids: list[int] = []

        # Emit loop start label (not counted as a real instruction — ID = -1)
        self._emit_label(start_label)

        # Load current cell value to decide whether to enter loop
        ir_ids.append(self._emit(
            IrOp.LOAD_BYTE,
            IrRegister(index=_REG_TEMP),
            IrRegister(index=_REG_TAPE_BASE),
            IrRegister(index=_REG_TAPE_PTR),
        ))

        # Branch to loop end if cell == 0 (skip the body)
        ir_ids.append(self._emit(
            IrOp.BRANCH_Z,
            IrRegister(index=_REG_TEMP),
            IrLabel(name=end_label),
        ))

        # Compile body (all instruction children; skip bracket tokens)
        for child in node.children:
            if isinstance(child, ASTNode):
                self._compile_node(child)

        # Jump back to loop start for next iteration
        ir_ids.append(self._emit(
            IrOp.JUMP,
            IrLabel(name=start_label),
        ))

        # Emit loop end label
        self._emit_label(end_label)

        # Record loop AST node → IR instruction IDs
        self._source_map.ast_to_ir.add(ast_node_id=ast_node_id, ir_ids=ir_ids)

    # -----------------------------------------------------------------------
    # Token extraction
    # -----------------------------------------------------------------------

    def _extract_token(self, node: ASTNode) -> Token | None:
        """Dig through an AST node to find the first leaf token.

        The Brainfuck ``command`` rule is always a leaf that wraps a single
        token (e.g., ``Token(INC, '+', line=1, column=3)``). We walk down
        to find it.

        Args:
            node: The AST node to search.

        Returns:
            The first ``Token`` found, or ``None`` if the node has no tokens.
        """
        # If it's a leaf node, return its token directly
        if node.is_leaf and node.token is not None:
            return node.token

        # Otherwise search children
        for child in node.children:
            if isinstance(child, Token):
                return child
            if isinstance(child, ASTNode):
                tok = self._extract_token(child)
                if tok is not None:
                    return tok
        return None
