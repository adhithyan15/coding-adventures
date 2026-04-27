"""Oct IR Compiler — lowers a type-annotated Oct AST into general-purpose IR.

Overview
--------

This module is the Oct-specific frontend of the AOT compiler pipeline.
It walks the typed AST produced by ``oct-type-checker`` and emits IR
instructions (``IrInstruction``) for each construct.

Stage in the pipeline::

    Source text
        → oct-lexer         (characters → tokens)
        → oct-parser        (tokens → untyped ASTNode tree)
        → oct-type-checker  (untyped AST → typed AST with ._oct_type)
        → oct-ir-compiler   (typed AST → IrProgram)   ← this module
        → intel-8008-ir-validator   (pre-flight IR check)
        → ir-to-intel-8008-compiler (IrProgram → 8008 assembly)
        → intel-8008-assembler      (two-pass assemble to binary)
        → intel-8008-packager       (Intel HEX output)

The compiler does NOT know about the Intel 8008 instruction set. Its job
is to translate Oct semantics into target-independent IR.  The 8008-specific
lowering happens in the backend packages.

Virtual Register Allocation (Fixed for Oct v1)
-----------------------------------------------

Oct v1 uses a simple fixed allocation scheme::

    v0  = zero constant (always 0 — preloaded at program start)
    v1  = scratch / expression result temporary / return value
    v2+ = one virtual register per named variable (locals, params)

Registers are allocated in declaration order within each function scope.
A fresh allocation is created for every function (static scoping — no
register sharing between functions at the IR level).

For ``static`` variables: emit ``IrDataDecl(label=name, size=1, init=0)``
and access them with ``LOAD_ADDR`` + ``LOAD_BYTE``/``STORE_BYTE``.

Hardware Register Mapping (done by the backend, not here)
----------------------------------------------------------

The Intel 8008 has registers A (accumulator), B, C, D, E, H, L.
H:L is the 16-bit memory address pair.  The backend maps::

    v0 (zero constant) → zeroed register or inline 0 in instructions
    v1 (scratch/return) → A (accumulator)
    v2  → B
    v3  → C
    v4  → D
    v5  → E

At most 4 named locals (v2–v5) are available, matching the 4 general-purpose
registers B, C, D, E.  The IR validator enforces this limit.

Calling Convention
-------------------

The calling convention for Oct v1 is::

    - Arguments passed in v2, v3, v4, v5 (matching B, C, D, E)
    - Return value in v1 (the accumulator A)
    - Callee uses its own fresh registers starting from v2

Key IR Emission Rules
----------------------

Declarations
~~~~~~~~~~~~

  ``static NAME: u8 = VALUE``
      ``IrDataDecl(label=NAME, size=1, init=VALUE)``
      Static reads  → ``LOAD_ADDR v_addr, NAME; LOAD_BYTE v_dst, v_addr, v0``
      Static writes → ``LOAD_ADDR v_addr, NAME; STORE_BYTE v_src, v_addr, v0``

  ``fn NAME(params) -> type { body }``
      ``LABEL _fn_NAME``
      Compile body.
      Always emit ``RET`` at end (callers pop the return address).

Statements
~~~~~~~~~~

  ``let NAME: type = expr``
      Compile expr into v1, then ``ADD_IMM vN, v1, 0`` to copy to vN.

  ``NAME = expr``
      Local:  compile expr into v1, ``ADD_IMM vN, v1, 0``.
      Static: compile expr into v1, ``LOAD_ADDR v_addr, NAME``,
              ``STORE_BYTE v1, v_addr, v0``.

  ``return expr``
      Compile expr into v1; emit ``RET``.

  ``return`` (void)
      Emit ``RET`` immediately.

  ``if cond { then } [else { else }]``
      Compile cond → vC.
      ``BRANCH_Z  vC, else_K``
      then body
      ``JUMP end_K``
      ``LABEL else_K``
      else body (if present)
      ``LABEL end_K``

  ``while cond { body }``
      ``LABEL while_K_start``
      Compile cond → vC.
      ``BRANCH_Z  vC, while_K_end``
      body
      ``JUMP while_K_start``
      ``LABEL while_K_end``

  ``loop { body }``
      ``LABEL loop_K_start``
      body
      ``JUMP loop_K_start``
      ``LABEL loop_K_end``  (always emitted; needed if body has ``break``)

  ``break``
      ``JUMP loop_K_end``  (jumps to the innermost loop/while end label)

Expressions
~~~~~~~~~~~

See ``_compile_expr()`` for the full dispatch table.  Key entries:

  INT_LIT / HEX_LIT / BIN_LIT
      ``LOAD_IMM v1, n``
  true
      ``LOAD_IMM v1, 1``
  false
      ``LOAD_IMM v1, 0``
  NAME (local var)
      return the variable's register vN (no instruction emitted)
  NAME (static var)
      ``LOAD_ADDR v1, NAME; LOAD_BYTE v1, v1, v0``
  NAME(args) (user fn call)
      compile args into v2, v3, ...; ``CALL _fn_NAME``
  a + b  →  ``ADD v1, vA, vB``
  a - b  →  ``SUB v1, vA, vB``
  a & b  →  ``AND v1, vA, vB``
  a | b  →  ``OR  v1, vA, vB``
  a ^ b  →  ``XOR v1, vA, vB``
  ~a     →  ``NOT v1, vA``
  a == b →  ``CMP_EQ  v1, vA, vB``
  a != b →  ``CMP_NE  v1, vA, vB``
  a < b  →  ``CMP_LT  v1, vA, vB``
  a > b  →  ``CMP_GT  v1, vA, vB``
  a <= b →  ``CMP_GT  v1, vB, vA``  (LE(a,b) = GT(b,a))
  a >= b →  ``CMP_LT  v1, vB, vA``  (GE(a,b) = LT(b,a))
  !a     →  ``CMP_EQ  v1, vA, v0``  (true iff a == 0)
  a && b →  ``AND     v1, vA, vB``  (both are bool 0/1)
  a || b →  ``ADD v1, vA, vB``; ``CMP_NE v1, v1, v0``

Intrinsic Emission
~~~~~~~~~~~~~~~~~~

Oct exposes the Intel 8008's special instructions as built-in functions.
Each maps to a SYSCALL instruction with a platform-specific number.
The SYSCALL ABI uses v2, v3, ... for arguments and v1 for the return value,
matching the general calling convention::

    in(PORT)       → SYSCALL 20+PORT          (PORT literal → syscall number)
    out(PORT, val) → copy val to v2; SYSCALL 40+PORT
    adc(a, b)      → copy a→v2, b→v3; SYSCALL 3   → result in v1
    sbb(a, b)      → copy a→v2, b→v3; SYSCALL 4   → result in v1
    rlc(a)         → copy a→v2; SYSCALL 11         → result in v1
    rrc(a)         → copy a→v2; SYSCALL 12         → result in v1
    ral(a)         → copy a→v2; SYSCALL 13         → result in v1
    rar(a)         → copy a→v2; SYSCALL 14         → result in v1
    carry()        → SYSCALL 15                    → result in v1
    parity(a)      → copy a→v2; SYSCALL 16         → result in v1

PORT is a compile-time literal (enforced by the type checker), so it is
baked into the SYSCALL number rather than passed as a runtime argument.

Main Function Entry Point
~~~~~~~~~~~~~~~~~~~~~~~~~

  ``LABEL _start``
  ``LOAD_IMM v0, 0``  ← constant-zero register, used in comparisons
  ``CALL _fn_main``
  ``HALT``
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
from lang_parser import ASTNode
from lexer import Token

# ---------------------------------------------------------------------------
# Virtual register indices — fixed for Oct v1
# ---------------------------------------------------------------------------
#
# These indices are the IR-level register numbers.  The 8008 backend maps:
#   v0  → constant zero (loaded once at _start, reused in comparisons)
#   v1  → accumulator A (scratch, expression temp, return value)
#   v2  → register B (first argument / first local)
#   v3  → register C (second argument / second local)
#   v4  → register D (third argument / third local)
#   v5  → register E (fourth argument / fourth local)
#
# The IR validator enforces the 4-local limit (v2–v5) as a backend constraint.
# The IR compiler is intentionally ignorant of physical register limits.
# ---------------------------------------------------------------------------

_REG_ZERO = 0     # v0: constant 0, preloaded at _start
_REG_SCRATCH = 1  # v1: scratch/expression temp / return value
_REG_VAR_BASE = 2  # v2+: named variables (locals, params)

# SYSCALL numbers for Oct intrinsics.
# These match the numbers in the OCT00 spec's IR Mapping Summary table.
# The 8008 IR validator enforces that only these syscall numbers appear
# in programs targeting the Intel 8008.
_SYSCALL_ADC = 3   # add with carry:        adc(a, b)
_SYSCALL_SBB = 4   # subtract with borrow:  sbb(a, b)
_SYSCALL_RLC = 11  # rotate left circular:  rlc(a)
_SYSCALL_RRC = 12  # rotate right circular: rrc(a)
_SYSCALL_RAL = 13  # rotate left through carry:  ral(a)
_SYSCALL_RAR = 14  # rotate right through carry: rar(a)
_SYSCALL_CARRY = 15   # read carry flag:  carry()
_SYSCALL_PARITY = 16  # read parity flag: parity(a)
_SYSCALL_IN_BASE = 20   # in(PORT)  → SYSCALL 20+PORT (PORT ∈ 0–7)
_SYSCALL_OUT_BASE = 40  # out(PORT, val) → SYSCALL 40+PORT (PORT ∈ 0–23)

# Names of all Oct hardware intrinsics.  Used when scanning intrinsic_call
# children to find the intrinsic name token.
_INTRINSIC_NAMES = frozenset({
    "in", "out", "adc", "sbb",
    "rlc", "rrc", "ral", "rar",
    "carry", "parity",
})

# Oct expression-level grammar rules.  Used in _is_expr_node().
_EXPR_RULES = frozenset({
    "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
    "add_expr", "bitwise_expr", "unary_expr", "primary",
    "call_expr", "intrinsic_call",
})


# ---------------------------------------------------------------------------
# OctCompileConfig — I/O target configuration
# ---------------------------------------------------------------------------
#
# By default Oct compiles for the Intel 8008: ``in(PORT)`` maps to
# ``SYSCALL 20+PORT`` and ``out(PORT, val)`` maps to ``SYSCALL 40+PORT``.
# These are 8008-specific port numbers; they are not meaningful on WASM, JVM,
# or CLR.
#
# OctCompileConfig allows callers to redirect I/O to a cross-platform ABI
# instead.  Setting ``write_byte_syscall`` to a non-None value makes
# ``out(PORT, val)`` emit ``SYSCALL [write_byte_syscall, v2]`` (the byte is
# in v2; PORT is ignored — cross-platform targets expose a single byte-write
# channel rather than 24 port-specific channels).  Similarly for
# ``read_byte_syscall`` and ``in(PORT)``.
#
# Pre-defined configurations cover the three cross-platform backends:
#
#   INTEL_8008_IO  — default; 8008 port-based SYSCALLs (port encoded in number)
#   WASM_IO        — SYSCALL 1 (fd_write) / SYSCALL 2 (fd_read)
#   JVM_IO         — SYSCALL 1 (System.out.write) / SYSCALL 4 (System.in.read)
#   CLR_IO         — SYSCALL 1 (Console.Write) / SYSCALL 2 (Console.Read)
#
# The ``write_byte_syscall`` and ``read_byte_syscall`` values of None mean
# "use the 8008 port-based encoding" (existing behaviour, unchanged).
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class OctCompileConfig:
    """Configuration controlling how Oct's I/O intrinsics are lowered to IR.

    Attributes:
        write_byte_syscall: The SYSCALL number to use for ``out()`` when
            targeting a cross-platform backend.  ``None`` (the default) means
            ``out(PORT, val)`` → ``SYSCALL 40+PORT`` (Intel 8008 convention).
            When non-None, ``out(PORT, val)`` → ``SYSCALL write_byte_syscall``
            with ``val`` in the standard arg register (v2); PORT is ignored
            because cross-platform targets have a single byte-write channel.

        read_byte_syscall: The SYSCALL number to use for ``in()`` when
            targeting a cross-platform backend.  ``None`` (the default) means
            ``in(PORT)`` → ``SYSCALL 20+PORT`` (Intel 8008 convention).
            When non-None, ``in(PORT)`` → ``SYSCALL read_byte_syscall`` with
            the result in the scratch register (v1); PORT is ignored for the
            same reason.

    Example — target WASM::

        from oct_ir_compiler import compile_oct, WASM_IO

        result = compile_oct(typed_ast, config=WASM_IO)

    Example — target JVM::

        from oct_ir_compiler import compile_oct, JVM_IO

        result = compile_oct(typed_ast, config=JVM_IO)

    Example — default (Intel 8008)::

        result = compile_oct(typed_ast)  # uses INTEL_8008_IO implicitly
    """

    write_byte_syscall: int | None = None
    read_byte_syscall: int | None = None


# Pre-defined I/O configurations.

INTEL_8008_IO: OctCompileConfig = OctCompileConfig(
    write_byte_syscall=None,  # out(PORT, val) → SYSCALL 40+PORT
    read_byte_syscall=None,   # in(PORT) → SYSCALL 20+PORT
)
"""Intel 8008 I/O config (default).

``out(PORT, val)`` lowers to ``SYSCALL 40+PORT`` and ``in(PORT)`` to
``SYSCALL 20+PORT``, matching the 8008 INP/OUT opcode encoding where the port
number is baked into the instruction opcode field.
"""

WASM_IO: OctCompileConfig = OctCompileConfig(
    write_byte_syscall=1,   # WASI fd_write
    read_byte_syscall=2,    # WASI fd_read
)
"""WASM / WASI I/O config.

``out()`` → ``SYSCALL 1`` (fd_write), ``in()`` → ``SYSCALL 2`` (fd_read).
Matches the WASI preview-1 ABI used by ``ir-to-wasm-compiler``.
"""

JVM_IO: OctCompileConfig = OctCompileConfig(
    write_byte_syscall=1,   # System.out.write(byte)
    read_byte_syscall=4,    # System.in.read()
)
"""JVM I/O config.

``out()`` → ``SYSCALL 1`` (System.out.write), ``in()`` → ``SYSCALL 4``
(System.in.read).  Matches the SYSCALL numbers wired in ``ir-to-jvm-class-file``
(which uses SYSCALL 1 and 4 to match Dartmouth BASIC convention).
"""

CLR_IO: OctCompileConfig = OctCompileConfig(
    write_byte_syscall=1,   # Console.Write(char)
    read_byte_syscall=2,    # Console.Read()
)
"""CLR (.NET) I/O config.

``out()`` → ``SYSCALL 1`` (Console.Write), ``in()`` → ``SYSCALL 2``
(Console.Read).  Matches the SYSCALL numbers wired in the ``ir-to-cil-bytecode``
CLR host.
"""


# ---------------------------------------------------------------------------
# OctCompileResult — the output of compilation
# ---------------------------------------------------------------------------


@dataclass
class OctCompileResult:
    """The output of a successful Oct IR compilation.

    Attributes:
        program: The compiled ``IrProgram`` containing all IR instructions,
                 data declarations, and the entry label ``_start``.

    Example::

        from oct_parser import parse_oct
        from oct_type_checker import check_oct
        from oct_ir_compiler import compile_oct

        ast = parse_oct("fn main() { let x: u8 = 42; }")
        result = check_oct(ast)
        compiled = compile_oct(result.typed_ast)
        # compiled.program has the IR instructions
    """

    program: IrProgram


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def compile_oct(
    typed_ast: ASTNode,
    config: OctCompileConfig = INTEL_8008_IO,
) -> OctCompileResult:
    """Compile a typed Oct AST into IR and return the result.

    This is the main entry point for the Oct IR compiler.  It accepts the
    type-annotated AST produced by ``oct_type_checker.check_oct()`` and
    returns an ``OctCompileResult`` wrapping the compiled ``IrProgram``.

    The input AST must be the root ``"program"`` node returned by
    ``oct-parser``.  All expression nodes must already have ``._oct_type``
    set (i.e. the type checker must have run successfully).

    Args:
        typed_ast: Root ``ASTNode`` with ``rule_name == "program"``,
                   already annotated with ``._oct_type`` on every
                   expression node.
        config: I/O target configuration that controls how ``in()`` and
                ``out()`` intrinsics are lowered to SYSCALL IR instructions.
                Defaults to ``INTEL_8008_IO`` (port-based 8008 encoding).
                Pass ``WASM_IO``, ``JVM_IO``, or ``CLR_IO`` to target those
                backends directly from Oct source without any SYSCALL ABI
                mismatch at compile time.

    Returns:
        An ``OctCompileResult`` with the compiled ``IrProgram``.

    Raises:
        ValueError: If the AST root is not a ``"program"`` node.

    Example — default (Intel 8008 target)::

        from oct_parser import parse_oct
        from oct_type_checker import check_oct
        from oct_ir_compiler import compile_oct

        ast = parse_oct("fn main() { out(1, in(0)); }")
        tc_result = check_oct(ast)
        compiled = compile_oct(tc_result.typed_ast)
        # in(0)  → SYSCALL 20   (8008 INP port 0)
        # out(1, …) → SYSCALL 41  (8008 OUT port 1)

    Example — WASM target::

        from oct_ir_compiler import compile_oct, WASM_IO

        compiled = compile_oct(tc_result.typed_ast, config=WASM_IO)
        # in(0)  → SYSCALL 2  (WASI fd_read)
        # out(1, …) → SYSCALL 1  (WASI fd_write)

    Example — JVM target::

        from oct_ir_compiler import compile_oct, JVM_IO

        compiled = compile_oct(tc_result.typed_ast, config=JVM_IO)
        # in(0)  → SYSCALL 4  (System.in.read)
        # out(1, …) → SYSCALL 1  (System.out.write)
    """
    if typed_ast.rule_name != "program":
        raise ValueError(
            f"expected 'program' AST node, got {typed_ast.rule_name!r}"
        )

    compiler = _Compiler(config=config)
    return compiler.compile(typed_ast)


# ---------------------------------------------------------------------------
# Internal compiler state
# ---------------------------------------------------------------------------


@dataclass
class _Compiler:
    """Internal Oct IR compiler state — not part of the public API.

    Holds all mutable state needed during compilation.  The public function
    ``compile_oct()`` creates one instance and calls ``compile()``.

    Attributes:
        config:           I/O target configuration.  Controls how ``in()`` and
                          ``out()`` intrinsics are lowered to SYSCALL IR.
                          Defaults to ``INTEL_8008_IO`` (8008 port encoding).
        _id_gen:          Produces unique instruction IDs.
        _program:         The IR program being built (initialised in
                          ``__post_init__``).
        _statics:         Names of all ``static`` variables declared at
                          the top level.  Used to distinguish static vs.
                          local lookups during expression compilation.
        _if_count:        Counter for generating unique if/else label names.
        _loop_count:      Counter for generating unique loop label names.
        _loop_end_stack:  Stack of loop-end label names.  Each ``loop`` or
                          ``while`` pushes its end label before compiling
                          the body and pops it after.  ``break`` jumps to
                          the top of this stack.
        _next_free_reg:   The next unallocated virtual register index for
                          the current function.  Resets to _REG_VAR_BASE
                          at the start of each function.
    """

    config: OctCompileConfig = field(default_factory=lambda: INTEL_8008_IO)
    _id_gen: IDGenerator = field(default_factory=IDGenerator)
    _program: IrProgram = field(init=False)
    _statics: set[str] = field(default_factory=set)
    _if_count: int = field(default=0)
    _loop_count: int = field(default=0)
    _loop_end_stack: list[str] = field(default_factory=list)
    _next_free_reg: int = field(default=_REG_VAR_BASE)

    def __post_init__(self) -> None:
        """Initialise the IR program after dataclass field initialisation."""
        self._program = IrProgram(entry_label="_start")

    # -----------------------------------------------------------------------
    # Top-level compilation
    # -----------------------------------------------------------------------

    def compile(self, ast: ASTNode) -> OctCompileResult:
        """Run compilation.  Called once per ``_Compiler`` instance.

        Three-phase compilation:
          Phase 1 — collect ``static`` declarations into the .data segment.
          Phase 2 — emit the ``_start`` entry point.
          Phase 3 — compile each ``fn`` declaration into the instruction stream.

        The three-phase design is necessary because ``static`` declarations
        can appear anywhere at the top level (Oct has no declaration-order
        restriction), and the entry point must precede all function labels
        so the 8008 begins executing at address 0.
        """
        # Phase 1: collect static data declarations.
        for child in ast.children:
            inner = _unwrap_top_decl(child)
            if inner is None:
                continue
            if inner.rule_name == "static_decl":
                self._emit_static_data(inner)

        # Phase 2: emit the program entry point.
        self._emit_entry_point(ast)

        # Phase 3: compile all function bodies.
        for child in ast.children:
            inner = _unwrap_top_decl(child)
            if inner is None:
                continue
            if inner.rule_name == "fn_decl":
                self._compile_fn_decl(inner)

        return OctCompileResult(program=self._program)

    # -----------------------------------------------------------------------
    # Low-level instruction emission helpers
    # -----------------------------------------------------------------------

    def _emit(
        self,
        opcode: IrOp,
        *operands: IrRegister | IrImmediate | IrLabel,
    ) -> int:
        """Add one instruction to the program and return its unique ID.

        Args:
            opcode:   The IR opcode.
            operands: Any number of register, immediate, or label operands.

        Returns:
            The unique instruction ID (from the ID generator).
        """
        instr_id = self._id_gen.next()
        self._program.add_instruction(
            IrInstruction(opcode=opcode, operands=list(operands), id=instr_id)
        )
        return instr_id

    def _emit_label(self, name: str) -> None:
        """Add a LABEL pseudo-instruction to the program.

        Labels have ``id = -1`` because they produce no machine code and
        do not participate in source mapping.

        Args:
            name: The label name (e.g., ``"_fn_main"``, ``"if_3_else"``).
        """
        self._program.add_instruction(
            IrInstruction(
                opcode=IrOp.LABEL,
                operands=[IrLabel(name=name)],
                id=-1,
            )
        )

    def _alloc_reg(self) -> int:
        """Allocate and return the next free virtual register index.

        Bumps ``_next_free_reg`` by 1.  Used for temporaries needed during
        static address materialisation (LOAD_ADDR) and intrinsic argument
        staging.
        """
        idx = self._next_free_reg
        self._next_free_reg += 1
        return idx

    # -----------------------------------------------------------------------
    # Static data declarations
    # -----------------------------------------------------------------------

    def _emit_static_data(self, node: ASTNode) -> None:
        """Emit an ``IrDataDecl`` for a ``static`` declaration.

        Oct statics are single bytes (all Oct values fit in one byte).
        The initial value is extracted from the literal initialiser.

        Structure (static_decl)::

            Token("static")  Token(NAME)  Token(COLON)
            ASTNode(type)    Token(EQ)    ASTNode(expr)  Token(SEMICOLON)

        Args:
            node: The ``static_decl`` ASTNode.
        """
        name: str | None = None
        init_val = 0

        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind == "NAME" and name is None:
                    name = child.value
                elif kind in ("INT_LIT", "HEX_LIT", "BIN_LIT"):
                    init_val = _parse_literal(child.value, kind)
            elif isinstance(child, ASTNode) and _is_expr_node(child):
                init_val = _extract_literal_int(child)

        if name is None:
            return

        # Record name so expression compilation can distinguish
        # static reads from local-variable reads.
        self._statics.add(name)

        self._program.add_data(
            IrDataDecl(label=name, size=1, init=init_val)
        )

    # -----------------------------------------------------------------------
    # Entry point
    # -----------------------------------------------------------------------

    def _emit_entry_point(self, ast: ASTNode) -> None:
        """Emit the ``_start`` label, v0 initialisation, CALL main, HALT.

        The Oct program entry point::

            LABEL     _start
            LOAD_IMM  v0, 0      ← constant-zero register
            CALL      _fn_main   ← invoke the user's main function
            HALT                 ← terminate (8008 HLT instruction)

        ``v0`` is preloaded with 0 so that comparison instructions
        (``CMP_EQ`` for ``!``, ``CMP_NE`` for ``||``) can reference the
        zero constant without materialising it inside every expression.

        Args:
            ast: The root ``"program"`` ASTNode.
        """
        self._emit_label("_start")

        # v0 = 0: the constant-zero register.  Preloaded once and never
        # written again.  Every backend pass can assume v0 == 0.
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(index=_REG_ZERO),
            IrImmediate(value=0),
        )

        # Check if a main function is declared.  If not, we still emit the
        # skeleton so the IR is structurally well-formed; the IR validator
        # will report the missing main.
        if _has_fn_named(ast, "main"):
            self._emit(IrOp.CALL, IrLabel(name="_fn_main"))

        self._emit(IrOp.HALT)

    # -----------------------------------------------------------------------
    # Function compilation
    # -----------------------------------------------------------------------

    def _compile_fn_decl(self, node: ASTNode) -> None:
        """Compile a function declaration into a sequence of IR instructions.

        Structure (fn_decl)::

            Token("fn")  Token(NAME)  Token(LPAREN)
            [ASTNode(param_list)]  Token(RPAREN)
            [Token(ARROW)  ASTNode(type)]
            ASTNode(block)

        Each function compiles to::

            LABEL     _fn_NAME
            ... body IR ...
            RET        ← always emitted at the end

        The trailing ``RET`` is always emitted even if the source has an
        explicit ``return`` statement in every path.  The backend can remove
        unreachable instructions after control-flow analysis; for IR
        correctness, the trailing RET ensures every execution path terminates.

        Register allocation is fresh for each function.  Parameters are
        assigned registers v2, v3, ... in left-to-right order.  Let-declared
        locals are assigned the next available register as they are
        encountered during body compilation.

        Args:
            node: The ``fn_decl`` ASTNode.
        """
        fn_name: str | None = None
        params: list[str] = []  # parameter names in order
        block_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind == "NAME" and fn_name is None:
                    fn_name = child.value
            elif isinstance(child, ASTNode):
                if child.rule_name == "param_list":
                    params = _extract_param_names(child)
                elif child.rule_name == "block":
                    block_node = child

        if fn_name is None or block_node is None:
            return

        # Emit the function label.
        self._emit_label(f"_fn_{fn_name}")

        # Build a fresh register allocation for this function.
        # Parameters occupy v2, v3, ... in declaration order.
        # The virtual register map: variable name → IrRegister.
        regs: dict[str, IrRegister] = {}
        next_reg = _REG_VAR_BASE

        for param_name in params:
            regs[param_name] = IrRegister(index=next_reg)
            next_reg += 1

        # Reset the free-register counter for the function scope.
        self._next_free_reg = next_reg

        # Compile the function body (block).
        self._compile_block(block_node, regs)

        # Always emit a trailing RET so that every code path has an exit.
        self._emit(IrOp.RET)

    # -----------------------------------------------------------------------
    # Block and statement compilation
    # -----------------------------------------------------------------------

    def _compile_block(
        self,
        block: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile all statements in a ``{ ... }`` block.

        Walks the block's children looking for ``stmt`` nodes and dispatches
        each to ``_compile_stmt``.  The ``regs`` mapping is mutated in-place
        as ``let`` declarations add new variable bindings.

        Args:
            block: The ``block`` ASTNode.
            regs:  Current variable → register mapping (mutated).
        """
        for child in block.children:
            if isinstance(child, ASTNode) and child.rule_name == "stmt":
                self._compile_stmt(child, regs)

    def _compile_stmt(
        self,
        stmt: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Dispatch a ``stmt`` node to the appropriate statement compiler.

        The ``stmt`` rule is a wrapper: its single ASTNode child is the
        actual statement.  This method unwraps the wrapper and dispatches
        on ``rule_name``.

        Args:
            stmt: A ``stmt`` ASTNode.
            regs: Variable → register mapping.
        """
        if not stmt.children:
            return

        inner = stmt.children[0]
        if not isinstance(inner, ASTNode):
            return

        rule = inner.rule_name

        if rule == "let_stmt":
            self._compile_let_stmt(inner, regs)
        elif rule == "assign_stmt":
            self._compile_assign_stmt(inner, regs)
        elif rule == "return_stmt":
            self._compile_return_stmt(inner, regs)
        elif rule == "if_stmt":
            self._compile_if_stmt(inner, regs)
        elif rule == "while_stmt":
            self._compile_while_stmt(inner, regs)
        elif rule == "loop_stmt":
            self._compile_loop_stmt(inner, regs)
        elif rule == "break_stmt":
            self._compile_break_stmt()
        elif rule == "expr_stmt":
            # Expression used as a statement (e.g., a bare function call).
            # Compile for side effects; discard the result register.
            self._compile_expr_stmt(inner, regs)

    # -----------------------------------------------------------------------
    # Statement compilers
    # -----------------------------------------------------------------------

    def _compile_let_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile a ``let`` declaration.

        Structure::

            Token("let")  Token(NAME)  Token(COLON)
            ASTNode(type)  Token(EQ)  ASTNode(expr)  Token(SEMICOLON)

        Allocates a fresh virtual register vN for the new variable.
        Compiles the initialiser expression (result lands in v1), then
        copies v1 → vN via ``ADD_IMM vN, v1, 0``.

        After this call, ``regs[name] = IrRegister(N)``.

        Args:
            node: The ``let_stmt`` ASTNode.
            regs: Variable → register mapping (mutated to add new binding).
        """
        name_tok: Token | None = None
        expr_node: ASTNode | None = None
        found_type = False

        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind == "NAME" and name_tok is None:
                    name_tok = child
            elif isinstance(child, ASTNode):
                if child.rule_name == "type" and not found_type:
                    found_type = True
                elif _is_expr_node(child) and found_type:
                    expr_node = child

        if name_tok is None or expr_node is None:
            return

        # Allocate the variable's dedicated register.
        var_reg = IrRegister(index=self._next_free_reg)
        self._next_free_reg += 1
        regs[name_tok.value] = var_reg

        # Compile the initialiser.  Result lands in some register.
        result_reg = self._compile_expr(expr_node, regs)

        # Copy result into the variable's dedicated register if needed.
        if result_reg.index != var_reg.index:
            self._emit(
                IrOp.ADD_IMM,
                var_reg,
                result_reg,
                IrImmediate(value=0),
            )

    def _compile_assign_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile an assignment statement.

        Structure::

            Token(NAME)  Token(EQ)  ASTNode(expr)  Token(SEMICOLON)

        Two cases:
          - Local variable: compile expr → result_reg, then copy into vN.
          - Static variable: compile expr → result_reg, then LOAD_ADDR +
            STORE_BYTE to write back to the data segment.

        Args:
            node: The ``assign_stmt`` ASTNode.
            regs: Variable → register mapping.
        """
        name_tok: Token | None = None
        expr_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind == "NAME" and name_tok is None:
                    name_tok = child
            elif isinstance(child, ASTNode) and _is_expr_node(child):
                expr_node = child

        if name_tok is None or expr_node is None:
            return

        name = name_tok.value
        result_reg = self._compile_expr(expr_node, regs)

        if name in self._statics:
            # Static write: LOAD_ADDR v_addr, label; STORE_BYTE val, v_addr, v0
            # We need an address register that is NOT the same as result_reg,
            # otherwise LOAD_ADDR would clobber the value before STORE_BYTE.
            if result_reg.index == _REG_SCRATCH:
                # Value is in v1 (scratch) — use a fresh register for the address.
                addr_reg = IrRegister(index=self._alloc_reg())
            else:
                # Value is in a named variable register — safe to use v1 for addr.
                addr_reg = IrRegister(index=_REG_SCRATCH)
            self._emit(IrOp.LOAD_ADDR, addr_reg, IrLabel(name=name))
            self._emit(
                IrOp.STORE_BYTE,
                result_reg,
                addr_reg,
                IrRegister(index=_REG_ZERO),
            )
        elif name in regs:
            # Local write: copy result into the variable's dedicated register.
            var_reg = regs[name]
            if result_reg.index != var_reg.index:
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

        Structure::

            Token("return")  [ASTNode(expr)]  Token(SEMICOLON)

        If an expression is present, compile it into v1 (the return value
        register per the calling convention), then emit ``RET``.  For a
        void return, just emit ``RET``.

        Args:
            node: The ``return_stmt`` ASTNode.
            regs: Variable → register mapping.
        """
        expr_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode) and _is_expr_node(child):
                expr_node = child
                break

        if expr_node is not None:
            result_reg = self._compile_expr(expr_node, regs)
            # Ensure return value is in v1 (accumulator / return-value register).
            if result_reg.index != _REG_SCRATCH:
                self._emit(
                    IrOp.ADD_IMM,
                    IrRegister(index=_REG_SCRATCH),
                    result_reg,
                    IrImmediate(value=0),
                )

        self._emit(IrOp.RET)

    def _compile_if_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile an ``if`` statement.

        Structure::

            Token("if")  ASTNode(expr)  ASTNode(block)
            [Token("else")  ASTNode(block)]

        Emitted IR::

            ... compile cond → vC ...
            BRANCH_Z  vC, if_K_else
            ... then block ...
            JUMP      if_K_end
            LABEL     if_K_else
            ... else block (if present) ...
            LABEL     if_K_end

        The ``JUMP if_K_end`` after the then-block is always emitted.  If
        there is no else-block, ``if_K_else`` and ``if_K_end`` share the
        same position (two consecutive labels with no instructions between
        them).  The backend's dead-code pass removes the redundant label.

        Args:
            node: The ``if_stmt`` ASTNode.
            regs: Variable → register mapping.
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

        # Compile condition into some register.
        if cond_expr is not None:
            cond_reg = self._compile_expr(cond_expr, regs)
        else:
            cond_reg = IrRegister(index=_REG_SCRATCH)

        # Branch to else/end if condition is false (== 0).
        self._emit(
            IrOp.BRANCH_Z,
            cond_reg,
            IrLabel(name=else_label),
        )

        # Compile then-block.
        if blocks:
            self._compile_block(blocks[0], regs)

        # Unconditional jump past else-block.
        self._emit(IrOp.JUMP, IrLabel(name=end_label))

        self._emit_label(else_label)

        # Compile else-block (if present).
        if len(blocks) >= 2:
            self._compile_block(blocks[1], regs)

        self._emit_label(end_label)

    def _compile_while_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile a ``while`` loop.

        Structure::

            Token("while")  ASTNode(expr)  ASTNode(block)

        Emitted IR::

            LABEL     while_K_start
            ... compile cond → vC ...
            BRANCH_Z  vC, while_K_end
            ... body ...
            JUMP      while_K_start
            LABEL     while_K_end

        Oct while loops check the condition at the **top** of each iteration,
        not the bottom.  The ``BRANCH_Z`` exits the loop when the condition
        is false (zero).  The trailing ``JUMP`` sends control back to the
        condition check.

        ``break`` inside the body jumps to ``while_K_end`` via the
        ``_loop_end_stack``.

        Args:
            node: The ``while_stmt`` ASTNode.
            regs: Variable → register mapping.
        """
        cond_expr: ASTNode | None = None
        block_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, ASTNode):
                if _is_expr_node(child) and cond_expr is None:
                    cond_expr = child
                elif child.rule_name == "block":
                    block_node = child

        loop_num = self._loop_count
        self._loop_count += 1
        start_label = f"while_{loop_num}_start"
        end_label = f"while_{loop_num}_end"

        # Push end label so break statements can target it.
        self._loop_end_stack.append(end_label)

        self._emit_label(start_label)

        # Compile condition.
        if cond_expr is not None:
            cond_reg = self._compile_expr(cond_expr, regs)
        else:
            cond_reg = IrRegister(index=_REG_SCRATCH)

        # Exit loop if condition is false.
        self._emit(
            IrOp.BRANCH_Z,
            cond_reg,
            IrLabel(name=end_label),
        )

        # Compile the loop body.
        if block_node is not None:
            self._compile_block(block_node, regs)

        # Jump back to condition check.
        self._emit(IrOp.JUMP, IrLabel(name=start_label))

        self._emit_label(end_label)
        self._loop_end_stack.pop()

    def _compile_loop_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile an unbounded ``loop`` statement.

        Structure::

            Token("loop")  ASTNode(block)

        Emitted IR::

            LABEL     loop_K_start
            ... body ...
            JUMP      loop_K_start
            LABEL     loop_K_end

        The end label ``loop_K_end`` is always emitted.  If the body
        contains no ``break``, it is unreachable — the backend's dead-code
        pass can remove it.  Emitting it unconditionally keeps the label
        generation logic simple and predictable.

        ``break`` inside the body emits ``JUMP loop_K_end`` via the
        ``_loop_end_stack``.

        Args:
            node: The ``loop_stmt`` ASTNode.
            regs: Variable → register mapping.
        """
        block_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "block":
                block_node = child

        loop_num = self._loop_count
        self._loop_count += 1
        start_label = f"loop_{loop_num}_start"
        end_label = f"loop_{loop_num}_end"

        self._loop_end_stack.append(end_label)

        self._emit_label(start_label)

        if block_node is not None:
            self._compile_block(block_node, regs)

        # Unconditional jump back to start — this is the defining
        # characteristic of an unbounded loop.
        self._emit(IrOp.JUMP, IrLabel(name=start_label))

        self._emit_label(end_label)
        self._loop_end_stack.pop()

    def _compile_break_stmt(self) -> None:
        """Compile a ``break`` statement.

        Emits ``JUMP loop_K_end`` targeting the innermost enclosing
        ``while`` or ``loop`` end label (the top of ``_loop_end_stack``).

        If somehow called outside any loop (which the type checker
        rejects), this is a no-op — no instruction is emitted.
        """
        if self._loop_end_stack:
            self._emit(IrOp.JUMP, IrLabel(name=self._loop_end_stack[-1]))

    def _compile_expr_stmt(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> None:
        """Compile an expression-statement (expression used for side effects).

        Structure::

            ASTNode(expr)  Token(SEMICOLON)

        Common examples: bare function calls (``tick();``) and intrinsic
        calls (``out(1, n);``).  The expression result is computed but the
        result register is discarded.

        Args:
            node: The ``expr_stmt`` ASTNode.
            regs: Variable → register mapping.
        """
        if not node.children:
            return
        expr_child = node.children[0]
        if isinstance(expr_child, ASTNode) and _is_expr_node(expr_child):
            self._compile_expr(expr_child, regs)

    # -----------------------------------------------------------------------
    # Expression compiler
    # -----------------------------------------------------------------------

    def _compile_expr(
        self,
        node: ASTNode | Token,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile an expression node.  Returns the register holding the result.

        The result register is one of:
          - ``v1`` (the scratch register) for literals, binary ops, calls,
            intrinsics, and static variable reads.
          - A variable's dedicated register (vN) for local variable
            name references — no instruction is emitted in this case.

        This recursive function dispatches on ``rule_name`` for ASTNodes and
        on the token type name for bare Token leaves.

        Args:
            node: An expression ASTNode or a bare Token leaf.
            regs: Variable → register mapping for the current scope.

        Returns:
            The ``IrRegister`` holding the expression result.
        """
        scratch = IrRegister(index=_REG_SCRATCH)

        if isinstance(node, Token):
            return self._compile_token(node, regs)

        rule = node.rule_name

        # ── Single-child pass-through nodes ──────────────────────────────
        #
        # Most expression grammar rules are single-child wrappers when no
        # operator is present (they just delegate to the next precedence
        # level).  Detect this and recurse rather than falling through to
        # the binary-operator path.
        if rule in (
            "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
            "add_expr", "bitwise_expr",
        ):
            if len(node.children) == 1:
                return self._compile_expr(node.children[0], regs)
            # Three or more children: binary operator chain.
            return self._compile_binary_chain(node, regs)

        if rule == "unary_expr":
            if len(node.children) == 1:
                return self._compile_expr(node.children[0], regs)
            # Two children: unary operator + operand.
            return self._compile_unary(node, regs)

        if rule == "primary":
            return self._compile_primary(node, regs)

        if rule == "call_expr":
            return self._compile_call_expr(node, regs)

        if rule == "intrinsic_call":
            return self._compile_intrinsic_call(node, regs)

        # Fallback: try to unwrap a single child.
        if node.children:
            return self._compile_expr(node.children[0], regs)

        return scratch

    def _compile_token(
        self,
        tok: Token,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a bare token (literal or name).

        Handles INT_LIT, HEX_LIT, BIN_LIT, ``true``, ``false``, and NAME
        (variable references — both local and static).

        Args:
            tok:  The Token to compile.
            regs: Variable → register mapping.

        Returns:
            The register holding the result.
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        kind = _tok_type(tok)

        if kind in ("INT_LIT", "HEX_LIT", "BIN_LIT"):
            val = _parse_literal(tok.value, kind)
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=val))
            return scratch

        # The oct-lexer promotes keyword tokens (like "true" and "false")
        # to plain string token types.  Handle both the keyword-as-type
        # form (kind == "true") and the NAME-with-value form (value == "true").
        if kind == "true" or (kind == "NAME" and tok.value == "true"):
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=1))
            return scratch

        if kind == "false" or (kind == "NAME" and tok.value == "false"):
            self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(value=0))
            return scratch

        if kind == "NAME":
            name = tok.value
            # Local variable: return its dedicated register (no instruction).
            if name in regs:
                return regs[name]
            # Static variable: materialise address then load.
            if name in self._statics:
                return self._compile_static_read(name)
            # Unknown (shouldn't reach here after type checking).
            return scratch

        return scratch

    def _compile_primary(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a ``primary`` expression.

        Primary expressions are the leaves of the expression tree:
          - Integer/hex/binary literals
          - Boolean literals (true/false)
          - Variable names (local or static)
          - Hardware intrinsic calls (``intrinsic_call``)
          - User-defined function calls (``call_expr``)
          - Parenthesised expressions: ``(`` expr ``)``

        Args:
            node: The ``primary`` ASTNode.
            regs: Variable → register mapping.

        Returns:
            The register holding the primary expression's value.
        """
        if not node.children:
            return IrRegister(index=_REG_SCRATCH)

        first = node.children[0]

        if isinstance(first, Token):
            # Token primary: literal or name.
            return self._compile_token(first, regs)

        if isinstance(first, ASTNode):
            if first.rule_name == "intrinsic_call":
                return self._compile_intrinsic_call(first, regs)
            if first.rule_name == "call_expr":
                return self._compile_call_expr(first, regs)
            # Parenthesised expression: first child is LPAREN, unwrap.
            # But if first child is an expression node, recurse directly.
            return self._compile_expr(first, regs)

        # Parenthesised: (expr) — structure: LPAREN ASTNode(expr) RPAREN.
        if len(node.children) >= 2:
            return self._compile_expr(node.children[1], regs)

        return IrRegister(index=_REG_SCRATCH)

    def _compile_binary_chain(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a binary operator chain (left-associative).

        Grammar rules with binary operators follow the pattern::

            left  op  right  [op  right …]

        Children alternate: operand, operator, operand, operator, operand.
        We compile left-to-right, folding each operator into the accumulating
        left result.

        Args:
            node: An expression ASTNode with 3 or more children.
            regs: Variable → register mapping.

        Returns:
            The register holding the final result.
        """
        children = node.children
        if len(children) < 3:
            if children:
                return self._compile_expr(children[0], regs)
            return IrRegister(index=_REG_SCRATCH)

        # Compile left operand.
        left_reg = self._compile_expr(children[0], regs)

        i = 1
        while i < len(children) - 1:
            op_child = children[i]
            right_node = children[i + 1]

            # Extract the operator string from token or nested node.
            op_val = ""
            if isinstance(op_child, Token):
                op_val = op_child.value
            elif isinstance(op_child, ASTNode):
                op_val = _first_token_value(op_child)

            right_reg = self._compile_expr(right_node, regs)
            left_reg = self._emit_binary_op(op_val, left_reg, right_reg)
            i += 2

        return left_reg

    def _compile_unary(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a unary expression.

        Structure::

            Token(BANG | TILDE)  ASTNode(unary_expr)

        Supported unary operators in Oct:

          ``!`` (logical NOT)
              Emits ``CMP_EQ v1, vA, v0``.
              Result is 1 (true) if vA == 0, else 0 (false).
              This is the Boolean complement: ``!true == false``,
              ``!false == true``.

          ``~`` (bitwise NOT)
              Emits ``NOT v1, vA``.
              Flips all 8 bits of vA.  The new ``IrOp.NOT`` opcode was
              added to compiler-ir specifically for Oct.  The 8008 backend
              lowers it to ``XRI 0xFF`` (XOR-immediate 255).

        Args:
            node: The ``unary_expr`` ASTNode with 2 children.
            regs: Variable → register mapping.

        Returns:
            The register holding the result (v1 scratch).
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        zero = IrRegister(index=_REG_ZERO)

        if len(node.children) < 2:
            return scratch

        op_child = node.children[0]
        operand_node = node.children[1]

        if not isinstance(op_child, Token):
            return scratch

        op_val = op_child.value
        operand_reg = self._compile_expr(operand_node, regs)

        if op_val == "!":
            # Logical NOT: result = (operand == 0) ? 1 : 0.
            # CMP_EQ sets the result register to 1 if equal, 0 otherwise.
            # Since true = 1 and false = 0, CMP_EQ with zero is exactly
            # the Boolean complement.
            self._emit(IrOp.CMP_EQ, scratch, operand_reg, zero)
            return scratch

        if op_val == "~":
            # Bitwise NOT: flip all 8 bits.
            # IrOp.NOT was added to compiler-ir as Phase 0 of the Oct
            # implementation roadmap — precisely for this instruction.
            self._emit(IrOp.NOT, scratch, operand_reg)
            return scratch

        # Unknown unary operator — return operand unchanged.
        return operand_reg

    # -----------------------------------------------------------------------
    # Binary operator emission
    # -----------------------------------------------------------------------

    def _emit_binary_op(
        self,
        op_val: str,
        left_reg: IrRegister,
        right_reg: IrRegister,
    ) -> IrRegister:
        """Emit IR for a binary operator.  Returns the result register (v1).

        Covers all Oct infix operators:

          Arithmetic:    ``+``  ``-``
          Bitwise:       ``&``  ``|``  ``^``
          Comparison:    ``==``  ``!=``  ``<``  ``>``  ``<=``  ``>=``
          Logical:       ``&&``  ``||``

        LE and GE use operand-swap identities to avoid introducing new IR
        opcodes — the same CMP_LT / CMP_GT are sufficient::

            LE(a, b) ≡ GT(b, a)   (a ≤ b iff b > a)
            GE(a, b) ≡ LT(b, a)   (a ≥ b iff b < a)

        Logical OR is slightly more expensive::

            a || b ≡ (a + b) != 0

        This is correct because Oct ``bool`` values are always 0 or 1.
        If either is 1, their sum is 1 or 2, both != 0.  If both are 0,
        the sum is 0, and CMP_NE returns 0 (false).

        Args:
            op_val:    The operator token value (e.g., ``"=="``, ``"&&"``).
            left_reg:  Register holding the left operand.
            right_reg: Register holding the right operand.

        Returns:
            The register holding the result (always v1 scratch).
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        zero = IrRegister(index=_REG_ZERO)

        if op_val == "+":
            self._emit(IrOp.ADD, scratch, left_reg, right_reg)
        elif op_val == "-":
            self._emit(IrOp.SUB, scratch, left_reg, right_reg)
        elif op_val == "&":
            self._emit(IrOp.AND, scratch, left_reg, right_reg)
        elif op_val == "|":
            self._emit(IrOp.OR, scratch, left_reg, right_reg)
        elif op_val == "^":
            self._emit(IrOp.XOR, scratch, left_reg, right_reg)
        elif op_val == "==":
            self._emit(IrOp.CMP_EQ, scratch, left_reg, right_reg)
        elif op_val == "!=":
            self._emit(IrOp.CMP_NE, scratch, left_reg, right_reg)
        elif op_val == "<":
            self._emit(IrOp.CMP_LT, scratch, left_reg, right_reg)
        elif op_val == ">":
            self._emit(IrOp.CMP_GT, scratch, left_reg, right_reg)
        elif op_val == "<=":
            # LE(a, b) = GT(b, a): swap operands.
            self._emit(IrOp.CMP_GT, scratch, right_reg, left_reg)
        elif op_val == ">=":
            # GE(a, b) = LT(b, a): swap operands.
            self._emit(IrOp.CMP_LT, scratch, right_reg, left_reg)
        elif op_val == "&&":
            # Both operands are bool (0 or 1); AND is exact.
            self._emit(IrOp.AND, scratch, left_reg, right_reg)
        elif op_val == "||":
            # OR via ADD + CMP_NE: (a + b) != 0.
            self._emit(IrOp.ADD, scratch, left_reg, right_reg)
            self._emit(IrOp.CMP_NE, scratch, scratch, zero)
        else:
            # Unknown operator — return left unchanged (safe fallback).
            return left_reg

        return scratch

    # -----------------------------------------------------------------------
    # Function call compilation
    # -----------------------------------------------------------------------

    def _compile_call_expr(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a user-defined function call.

        Structure::

            Token(NAME)  Token(LPAREN)  [ASTNode(arg_list)]  Token(RPAREN)

        Calling convention:
          - Arguments are placed in v2, v3, v4, v5 (one per argument).
          - ``CALL _fn_NAME`` invokes the function.
          - Return value is in v1 after the call.

        To avoid clobbering live locals that happen to reside in the
        argument registers (v2+), we compile arguments into temporary
        registers first, then move them into their final call slots.
        The temporaries live above all currently allocated registers.

        Args:
            node: The ``call_expr`` ASTNode.
            regs: Current variable → register mapping.

        Returns:
            ``v1`` (the return value register).
        """
        fn_tok: Token | None = None
        arg_exprs: list[ASTNode | Token] = []

        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind == "NAME" and fn_tok is None:
                    fn_tok = child
            elif isinstance(child, ASTNode) and child.rule_name == "arg_list":
                for ac in child.children:
                    is_arg = isinstance(ac, ASTNode) and _is_expr_node(ac)
                    is_arg = is_arg or (
                        isinstance(ac, Token)
                        and _tok_type(ac) not in ("COMMA", "LPAREN", "RPAREN")
                    )
                    if is_arg:
                        arg_exprs.append(ac)

        if fn_tok is None:
            return IrRegister(index=_REG_SCRATCH)

        # Step 1: Save live caller registers into temps above current
        # allocation to prevent the argument setup from clobbering them.
        live_indices = sorted({r.index for r in regs.values()})
        next_temp = self._next_free_reg
        saved: list[tuple[IrRegister, IrRegister]] = []
        for idx in live_indices:
            original = IrRegister(index=idx)
            temp = IrRegister(index=next_temp)
            next_temp += 1
            self._emit(
                IrOp.ADD_IMM,
                temp,
                original,
                IrImmediate(value=0),
            )
            saved.append((original, temp))

        # Step 2: Compile each argument into its own temp register to
        # avoid aliasing issues when one argument reads a local that
        # another argument is overwriting.
        arg_temps: list[IrRegister] = []
        for arg in arg_exprs:
            temp = IrRegister(index=next_temp)
            next_temp += 1
            result = self._compile_expr(arg, regs)
            if result.index != temp.index:
                self._emit(
                    IrOp.ADD_IMM,
                    temp,
                    result,
                    IrImmediate(value=0),
                )
            arg_temps.append(temp)

        self._next_free_reg = max(self._next_free_reg, next_temp)

        # Step 3: Move temps into the ABI argument slots v2, v3, ...
        for i, temp_reg in enumerate(arg_temps):
            arg_reg = IrRegister(index=_REG_VAR_BASE + i)
            if temp_reg.index != arg_reg.index:
                self._emit(
                    IrOp.ADD_IMM,
                    arg_reg,
                    temp_reg,
                    IrImmediate(value=0),
                )

        # Step 4: Issue the CALL.
        self._emit(IrOp.CALL, IrLabel(name=f"_fn_{fn_tok.value}"))

        # Step 5: Restore saved caller registers.
        for original, temp in saved:
            self._emit(
                IrOp.ADD_IMM,
                original,
                temp,
                IrImmediate(value=0),
            )

        return IrRegister(index=_REG_SCRATCH)

    # -----------------------------------------------------------------------
    # Intrinsic call compilation
    # -----------------------------------------------------------------------

    def _compile_intrinsic_call(
        self,
        node: ASTNode,
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Compile a hardware intrinsic call.

        Structure::

            Token(intrinsic_name)  Token(LPAREN)  [args ...]  Token(RPAREN)

        Each intrinsic maps to a SYSCALL instruction with a fixed number
        from the OCT00 spec.  PORT-based intrinsics (in/out) bake the
        port literal into the SYSCALL number itself.  All other intrinsics
        pass their arguments in the calling-convention registers v2, v3.

        Return values land in v1 (the accumulator / scratch register).

        Args:
            node: The ``intrinsic_call`` ASTNode.
            regs: Variable → register mapping.

        Returns:
            ``v1`` (scratch) — the return value register.
        """
        scratch = IrRegister(index=_REG_SCRATCH)

        # Find the intrinsic name token (first keyword token child).
        name_tok: Token | None = None
        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind in _INTRINSIC_NAMES or child.value in _INTRINSIC_NAMES:
                    name_tok = child
                    break

        if name_tok is None:
            return scratch

        intrinsic = name_tok.value  # "in", "out", "carry", …

        # Collect argument expression nodes (skip LPAREN, RPAREN, COMMA,
        # and the intrinsic name token itself).
        args: list[ASTNode | Token] = []
        seen_lparen = False
        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type(child)
                if kind == "LPAREN":
                    seen_lparen = True
                    continue
                if kind in ("RPAREN", "COMMA"):
                    continue
                if seen_lparen and child.value not in _INTRINSIC_NAMES:
                    args.append(child)
            elif isinstance(child, ASTNode) and seen_lparen:
                args.append(child)

        return self._emit_intrinsic(intrinsic, args, regs)

    def _emit_intrinsic(
        self,
        name: str,
        args: list[ASTNode | Token],
        regs: dict[str, IrRegister],
    ) -> IrRegister:
        """Emit IR for a specific hardware intrinsic.

        Dispatches on the intrinsic name and emits the correct argument
        setup + SYSCALL sequence.

        SYSCALL number table (from OCT00 spec IR Mapping Summary):

          ┌──────────────┬─────────────────────────────────────────┐
          │ Intrinsic    │ SYSCALL number                          │
          ├──────────────┼─────────────────────────────────────────┤
          │ adc(a, b)    │ 3  (add with carry)                     │
          │ sbb(a, b)    │ 4  (subtract with borrow)               │
          │ rlc(a)       │ 11 (rotate left circular)               │
          │ rrc(a)       │ 12 (rotate right circular)              │
          │ ral(a)       │ 13 (rotate left through carry)          │
          │ rar(a)       │ 14 (rotate right through carry)         │
          │ carry()      │ 15 (read carry flag)                    │
          │ parity(a)    │ 16 (read parity flag)                   │
          │ in(PORT)     │ 20 + PORT  (PORT ∈ 0–7)                 │
          │ out(PORT, v) │ 40 + PORT  (PORT ∈ 0–23)                │
          └──────────────┴─────────────────────────────────────────┘

        Args:
            name: Intrinsic name string ("in", "out", "adc", …).
            args: List of argument expression nodes (already stripped of
                  LPAREN, RPAREN, COMMA tokens).
            regs: Variable → register mapping.

        Returns:
            ``v1`` (scratch) — the return-value register.
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        v2 = IrRegister(index=_REG_VAR_BASE)      # first arg register
        v3 = IrRegister(index=_REG_VAR_BASE + 1)  # second arg register

        if name == "in":
            port = _extract_literal_int(args[0]) if args else 0
            if self.config.read_byte_syscall is not None:
                # Cross-platform target (WASM / JVM / CLR): emit
                #   SYSCALL [read_syscall, v1]
                # The backend stores the read byte into the arg register (v1 =
                # scratch), which is also Oct's return-value register.  PORT is
                # ignored — cross-platform backends expose a single read channel.
                self._emit(
                    IrOp.SYSCALL,
                    IrImmediate(value=self.config.read_byte_syscall),
                    scratch,
                )
            else:
                # Intel 8008 target: SYSCALL (20 + PORT).
                # PORT is encoded in the SYSCALL number to match the 8008's INP
                # instruction where the port is part of the opcode field.
                self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_IN_BASE + port))
            return scratch

        if name == "out":
            # Stage the value into v2 (the first argument register) so the
            # backend knows where to find it — this is common to all targets.
            port = _extract_literal_int(args[0]) if args else 0
            if len(args) >= 2:
                val_reg = self._compile_expr(args[1], regs)
                if val_reg.index != v2.index:
                    self._emit(IrOp.ADD_IMM, v2, val_reg, IrImmediate(value=0))
            if self.config.write_byte_syscall is not None:
                # Cross-platform target (WASM / JVM / CLR): emit
                #   SYSCALL [write_syscall, v2]
                # The backend reads the byte-to-write from the arg register v2.
                # PORT is ignored — cross-platform backends expose a single
                # write channel rather than 24 port-specific output channels.
                self._emit(
                    IrOp.SYSCALL,
                    IrImmediate(value=self.config.write_byte_syscall),
                    v2,
                )
            else:
                # Intel 8008 target: SYSCALL (40 + PORT).
                # PORT is baked into the SYSCALL number (8008 OUT opcode field).
                self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_OUT_BASE + port))
            return scratch  # out() is void; return scratch as a safe no-op

        if name in ("adc", "sbb"):
            # adc(a, b) → copy a→v2, b→v3; SYSCALL 3 (or 4 for sbb).
            # Two u8-compatible arguments; result in v1.
            syscall_num = _SYSCALL_ADC if name == "adc" else _SYSCALL_SBB
            if args:
                a_reg = self._compile_expr(args[0], regs)
                if a_reg.index != v2.index:
                    self._emit(IrOp.ADD_IMM, v2, a_reg, IrImmediate(value=0))
            if len(args) >= 2:
                b_reg = self._compile_expr(args[1], regs)
                if b_reg.index != v3.index:
                    self._emit(IrOp.ADD_IMM, v3, b_reg, IrImmediate(value=0))
            self._emit(IrOp.SYSCALL, IrImmediate(value=syscall_num))
            return scratch

        if name in ("rlc", "rrc", "ral", "rar"):
            # rlc/rrc/ral/rar(a) → copy a→v2; SYSCALL n.
            # Single u8-compatible argument; result in v1.
            syscall_nums = {
                "rlc": _SYSCALL_RLC,
                "rrc": _SYSCALL_RRC,
                "ral": _SYSCALL_RAL,
                "rar": _SYSCALL_RAR,
            }
            if args:
                a_reg = self._compile_expr(args[0], regs)
                if a_reg.index != v2.index:
                    self._emit(IrOp.ADD_IMM, v2, a_reg, IrImmediate(value=0))
            self._emit(IrOp.SYSCALL, IrImmediate(value=syscall_nums[name]))
            return scratch

        if name == "carry":
            # carry() → SYSCALL 15.  No arguments; result in v1.
            self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_CARRY))
            return scratch

        if name == "parity":
            # parity(a) → copy a→v2; SYSCALL 16.  Result in v1.
            if args:
                a_reg = self._compile_expr(args[0], regs)
                if a_reg.index != v2.index:
                    self._emit(IrOp.ADD_IMM, v2, a_reg, IrImmediate(value=0))
            self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_PARITY))
            return scratch

        # Unknown intrinsic — should never reach here after type checking.
        return scratch

    # -----------------------------------------------------------------------
    # Static variable read helper
    # -----------------------------------------------------------------------

    def _compile_static_read(self, name: str) -> IrRegister:
        """Load a static variable's value into the scratch register.

        Static variables live in the .data segment.  To read one:

        1. ``LOAD_ADDR v1, label_name``  — load the memory address into v1.
        2. ``LOAD_BYTE v1, v1, v0``      — read the byte at that address.

        Both steps use v1 (scratch) and v0 (zero constant for the offset).
        After this sequence, v1 holds the current value of the static.

        Args:
            name: The static variable's label (same as its source name).

        Returns:
            The scratch register v1 (now holding the static's value).
        """
        scratch = IrRegister(index=_REG_SCRATCH)
        zero = IrRegister(index=_REG_ZERO)

        # Load the address of the static into v1.
        self._emit(IrOp.LOAD_ADDR, scratch, IrLabel(name=name))
        # Load the byte at that address (offset 0) into v1.
        self._emit(IrOp.LOAD_BYTE, scratch, scratch, zero)
        return scratch


# ---------------------------------------------------------------------------
# Private AST traversal helpers
# ---------------------------------------------------------------------------


def _tok_type(tok: Token) -> str:
    """Return the canonical type name string for a Token.

    Oct tokens may have either a string type (for keyword tokens that the
    oct-lexer promotes to plain strings, e.g. ``"fn"``, ``"carry"``) or a
    ``TokenType`` enum value (for punctuation, literals, identifiers).

    This helper normalises both into a plain string so the compiler can
    use simple equality comparisons throughout.

    Args:
        tok: The Token to inspect.

    Returns:
        The token type as a string (``"NAME"``, ``"INT_LIT"``, ``"fn"``, …).
    """
    t = tok.type
    return t if isinstance(t, str) else t.name


def _unwrap_top_decl(child: ASTNode | object) -> ASTNode | None:
    """Unwrap a ``top_decl`` node to its inner declaration ASTNode.

    The Oct grammar wraps every top-level item in a ``top_decl`` node::

        program   → { top_decl }
        top_decl  → static_decl | fn_decl

    We peel off the ``top_decl`` wrapper to access the actual declaration.

    Args:
        child: A direct child of the ``program`` node.

    Returns:
        The inner declaration ASTNode, or ``None`` if not found.
    """
    if not isinstance(child, ASTNode):
        return None
    # The top_decl wrapper's first ASTNode child is the actual declaration.
    for grandchild in child.children:
        if isinstance(grandchild, ASTNode):
            return grandchild
    return None


def _is_expr_node(node: ASTNode | Token) -> bool:
    """Return True if ``node`` is an expression-level grammar rule.

    Used to identify expression children inside statement nodes, skipping
    over keyword tokens, punctuation, and type annotation nodes.

    Args:
        node: Any child node.

    Returns:
        True if this is an expression ASTNode.
    """
    if not isinstance(node, ASTNode):
        return False
    return node.rule_name in _EXPR_RULES


def _extract_param_names(param_list: ASTNode) -> list[str]:
    """Extract parameter names (in order) from a ``param_list`` node.

    Structure::

        param_list → param { COMMA param }
        param      → NAME COLON type

    Args:
        param_list: The ``param_list`` ASTNode.

    Returns:
        List of parameter name strings in declaration order.
    """
    names: list[str] = []
    for child in param_list.children:
        if isinstance(child, ASTNode) and child.rule_name == "param":
            for pc in child.children:
                if isinstance(pc, Token) and _tok_type(pc) == "NAME":
                    names.append(pc.value)
                    break
    return names


def _extract_literal_int(node: ASTNode | Token) -> int:
    """Recursively extract an integer value from a literal expression.

    Walks the expression tree looking for INT_LIT, HEX_LIT, or BIN_LIT
    tokens.  Returns 0 if no literal token is found (safe fallback).

    Used primarily to extract compile-time port numbers from ``in``/``out``
    calls and initial values from ``static`` declarations.

    Args:
        node: An expression node or token.

    Returns:
        The integer value of the literal, or 0 if none found.
    """
    if isinstance(node, Token):
        kind = _tok_type(node)
        if kind in ("INT_LIT", "HEX_LIT", "BIN_LIT"):
            return _parse_literal(node.value, kind)
        # Handle true/false as integer values.
        if node.value == "true":
            return 1
        if node.value == "false":
            return 0
        return 0

    # ASTNode: search children recursively.
    for child in node.children:
        val = _extract_literal_int(child)
        if val != 0:
            return val

    return 0


def _parse_literal(value: str, kind: str) -> int:
    """Parse an integer literal token value.

    Handles:
      - INT_LIT: plain decimal (``"42"``, ``"255"``)
      - HEX_LIT: ``0x``-prefixed hex (``"0xFF"``, ``"0x3A"``)
      - BIN_LIT: ``0b``-prefixed binary (``"0b10110011"``)

    Args:
        value: The raw token string.
        kind:  The token type name (``"INT_LIT"``, ``"HEX_LIT"``,
               ``"BIN_LIT"``).

    Returns:
        The integer value, or 0 on parse failure.
    """
    try:
        if kind == "HEX_LIT":
            return int(value, 16)
        if kind == "BIN_LIT":
            return int(value, 2)
        return int(value)
    except ValueError:
        return 0


def _has_fn_named(ast: ASTNode, name: str) -> bool:
    """Return True if the program contains a function named ``name``.

    Scans the top-level declarations for a ``fn_decl`` whose NAME token
    matches the given name.

    Args:
        ast:  The root ``"program"`` ASTNode.
        name: The function name to look for (e.g., ``"main"``).

    Returns:
        True if a matching function declaration exists.
    """
    for child in ast.children:
        inner = _unwrap_top_decl(child)
        if inner is None:
            continue
        if inner.rule_name == "fn_decl":
            for c in inner.children:
                if (
                    isinstance(c, Token)
                    and _tok_type(c) == "NAME"
                    and c.value == name
                ):
                    return True
    return False


def _first_token_value(node: ASTNode | Token) -> str:
    """Return the value of the first token found in ``node`` (depth-first).

    Used to extract operator strings from operator sub-nodes when the
    grammar wraps them in a named rule rather than leaving them as bare
    tokens.

    Args:
        node: Any node or token.

    Returns:
        The first token's value string, or ``""`` if none found.
    """
    if isinstance(node, Token):
        return node.value
    for child in node.children:
        val = _first_token_value(child)
        if val:
            return val
    return ""
