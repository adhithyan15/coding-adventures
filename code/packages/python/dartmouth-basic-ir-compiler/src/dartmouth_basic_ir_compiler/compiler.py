"""Dartmouth BASIC IR Compiler — lowers a BASIC AST to target-independent IR.

Overview
--------

This module is the Dartmouth BASIC-specific frontend of the AOT compiler
pipeline. It walks the AST produced by ``dartmouth_basic_parser`` and emits
IR instructions for each BASIC statement and expression.

The compiler knows nothing about machine architecture. It produces a generic
``IrProgram`` that any backend (GE-225, WASM, JVM) can translate to native code.

Historical Note
---------------

Dartmouth BASIC was the world's first general-purpose programming language
designed to be compiled and run on shared, time-sliced hardware. In 1964,
John Kemeny and Thomas Kurtz ran the first BASIC programs on a GE-225 mainframe
at Dartmouth College. Students typed programs on Teletype terminals and received
printed output within seconds — a radical departure from the batch-processing
model where a programmer submitted a deck of punched cards and waited overnight.

The language was intentionally simple: 17 statement types, floating-point only,
1-based arrays, and line numbers as the sole control-flow mechanism. GOTO and
GOSUB were not bugs — they were the entire control structure, and they worked
well at the scale of programs that fit on a single teletype page.

V1 Scope
--------

This first version compiles the subset needed to run meaningful integer programs:

  - REM    — no-op comments
  - LET    — variable assignment (scalar only) with all arithmetic operators
  - PRINT  — string literals and numeric expressions (variables, arithmetic)
  - GOTO   — unconditional jump to a line number
  - IF … THEN — conditional jump; all six relational operators
  - FOR … TO [STEP] — pre-test counted loop with positive integer step
  - NEXT   — ends the innermost FOR loop
  - END    — halt execution
  - STOP   — halt execution (synonym for END in V1)

Virtual Register Layout
-----------------------

Every BASIC variable gets a fixed virtual register. The GE-225 backend will
assign each register a dedicated spill slot (memory word). This fixed layout
means that GOTO targets and loop iterations always read/write the correct register:

  v0       — syscall argument (char code to print for SYSCALL 1)
  v1–v26   — BASIC scalar variables A–Z (v1=A, v2=B, …, v26=Z)
  v27–v286 — BASIC two-character variables A0–Z9 (v27=A0, v28=A1, …)
  v287+    — expression temporaries (fresh register per intermediate value)

Expression temporaries are never recycled — the counter only moves forward.
This keeps the IR simple and avoids liveness analysis, at the cost of a larger
spill area. For the GE-225 simulator this is irrelevant; memory is plentiful.

Label Conventions
-----------------

  _start          — program entry point
  _line_N         — BASIC line number N (GOTO/IF target)
  _for_N_check    — top of FOR loop N's pre-test
  _for_N_end      — label after FOR loop N's body (NEXT target)
"""

from __future__ import annotations

from dataclasses import dataclass, field

from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from lang_parser import ASTNode
from lexer import Token

from dartmouth_basic_ir_compiler.ge225_codes import (
    CARRIAGE_RETURN_CODE,
    ascii_to_ge225,
)


# ---------------------------------------------------------------------------
# Token type comparison helper
# ---------------------------------------------------------------------------
#
# The lexer uses both string token types (e.g., "LINE_NUM", "EQ") and enum
# token types (e.g., TokenType.NAME, TokenType.NUMBER). This helper normalises
# both so the rest of the compiler can compare with plain string names.


def _type_is(token_type: object, name: str) -> bool:
    """Return True if token_type matches the given type name string.

    Handles both string types (``"EQ"``) and enum types (``TokenType.NAME``).

    Args:
        token_type: The ``token.type`` value (str or enum member).
        name: The canonical type name to compare against.

    Returns:
        True if the type's name equals ``name``.

    Example::

        _type_is("NAME", "NAME")          # True
        _type_is(TokenType.NAME, "NAME")  # True
        _type_is(TokenType.NUMBER, "NAME") # False
    """
    if isinstance(token_type, str):
        return token_type == name
    return getattr(token_type, "name", None) == name

# ---------------------------------------------------------------------------
# Fixed virtual register indices
# ---------------------------------------------------------------------------

_REG_SYSCALL_ARG = 0   # v0: argument to SYSCALL 1 (typewriter char code)

# BASIC scalar variable A–Z → v1–v26
_VAR_BASE = 1                       # v1 = A
_VAR_LETTER_DIGIT_BASE = 27         # v27 = A0

# Expression temporaries start at v287 (after all variable registers)
_TEMP_BASE = 287

# Syscall number for "print char in v0"
_SYSCALL_PRINT_CHAR = 1

# GE-225 typewriter code for the minus sign '-' (octal 33 = decimal 27)
_GE225_MINUS_CODE = 0o33


def _scalar_reg(name: str) -> int:
    """Map a BASIC scalar variable name to its fixed virtual register index.

    Single-letter variables (A–Z) map to v1–v26.
    Two-character variables (A0–Z9) map to v27–v286.

    Args:
        name: The BASIC variable name (e.g., ``"A"``, ``"B3"``).

    Returns:
        The virtual register index.

    Example::

        _scalar_reg("A")   # 1
        _scalar_reg("Z")   # 26
        _scalar_reg("A0")  # 27
        _scalar_reg("Z9")  # 286
    """
    if len(name) == 1:
        return _VAR_BASE + (ord(name.upper()) - ord("A"))
    # Two-character: letter (0..25) * 10 + digit (0..9)
    letter_index = ord(name[0].upper()) - ord("A")
    digit_index = int(name[1])
    return _VAR_LETTER_DIGIT_BASE + letter_index * 10 + digit_index


# ---------------------------------------------------------------------------
# FOR stack record
# ---------------------------------------------------------------------------


@dataclass
class _ForRecord:
    """State kept while compiling a FOR loop.

    Pushed onto ``_Compiler._for_stack`` when FOR is encountered.
    Popped when the matching NEXT is encountered.

    Attributes:
        var_name:    The BASIC variable name of the loop counter.
        var_reg:     Virtual register for the loop counter.
        limit_reg:   Virtual register holding the limit (upper bound).
        step_reg:    Virtual register holding the step value.
        check_label: Label at the top of the pre-test (jump back here on NEXT).
        end_label:   Label after the loop body (jump here when counter > limit).
        loop_num:    Unique integer N for this loop (used in label names).
    """

    var_name: str
    var_reg: int
    limit_reg: int
    step_reg: int
    check_label: str
    end_label: str
    loop_num: int


# ---------------------------------------------------------------------------
# CompileResult and CompileError
# ---------------------------------------------------------------------------


@dataclass
class CompileResult:
    """The outputs of a successful BASIC compilation.

    Attributes:
        program:  The compiled ``IrProgram`` containing all IR instructions.
        var_regs: Maps BASIC variable names to their virtual register indices.
                  Useful for debugging: after execution, the value of BASIC
                  variable A is in the memory word for register ``var_regs["A"]``.

    Example::

        result = compile_basic(ast)
        print(result.var_regs["I"])   # register index for variable I
    """

    program: IrProgram
    var_regs: dict[str, int]


class CompileError(ValueError):
    """Raised when the BASIC program contains a feature not supported in V1.

    Examples:
        - GOSUB / RETURN (subroutine calls)
        - DIM (array declarations)
        - INPUT, DATA, READ, RESTORE
        - DEF FN (user-defined functions)
        - PRINT with numeric variables or formatting
        - Exponentiation (^ operator)
        - NEXT without a matching FOR
        - NEXT naming the wrong variable
        - String characters without a GE-225 typewriter code
    """


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def compile_basic(
    ast: ASTNode,
    *,
    char_encoding: str = "ge225",
    int_bits: int = 32,
) -> CompileResult:
    """Compile a Dartmouth BASIC AST to an IrProgram.

    This is the main entry point. It takes the root AST node from
    ``dartmouth_basic_parser.parse_dartmouth_basic()`` and lowers it to a
    target-independent ``IrProgram``.

    Args:
        ast:           Root ``ASTNode`` with ``rule_name == "program"``.
        char_encoding: Character encoding used for PRINT char codes in the IR.
                       ``"ge225"`` (default) emits GE-225 typewriter codes —
                       compatible with the GE-225 backend.
                       ``"ascii"`` emits standard ASCII byte values — compatible
                       with the WASM backend's WASI ``fd_write`` syscall.
        int_bits:      Signed integer width of the target machine in bits
                       (default 32).  Controls how many decimal digit positions
                       ``PRINT`` of a number unrolls.  The rule is:

                         max_value = 2**(int_bits-1) - 1
                         digit_positions = len(str(max_value))

                       Examples:
                         - ``int_bits=32`` → max 2,147,483,647 → 10 digits
                         - ``int_bits=20`` → max     524,287   →  6 digits
                                            (GE-225 20-bit signed words)

                       **Critical**: every power-of-ten constant emitted as a
                       ``LOAD_IMM`` must fit in the target machine's signed word.
                       Passing ``int_bits=32`` for a GE-225 backend would cause
                       1,000,000,000 to overflow a 20-bit register, producing
                       garbled digit extraction.

    Returns:
        A ``CompileResult`` containing the IR program and variable register map.

    Raises:
        CompileError: If the program uses a V1-excluded feature (GOSUB, DIM,
                      INPUT, DEF FN, PRINT with variables, exponentiation,
                      NEXT without FOR, or unsupported string characters).
        ValueError:   If ``ast.rule_name != "program"`` or ``int_bits < 2``.

    Example::

        from dartmouth_basic_parser import parse_dartmouth_basic
        from dartmouth_basic_ir_compiler import compile_basic

        ast = parse_dartmouth_basic("10 LET A = 5\\n20 END\\n")
        result = compile_basic(ast)
        # result.program contains the IrProgram
        # result.var_regs["A"] == 1 (virtual register for variable A)

        # For a 20-bit target (e.g. the GE-225):
        result = compile_basic(ast, int_bits=20)
    """
    if ast.rule_name != "program":
        raise ValueError(
            f"expected 'program' AST node, got {ast.rule_name!r}"
        )
    if char_encoding not in ("ge225", "ascii"):
        raise ValueError(f"char_encoding must be 'ge225' or 'ascii'; got {char_encoding!r}")
    if int_bits < 2:
        raise ValueError(f"int_bits must be >= 2; got {int_bits!r}")
    c = _Compiler(char_encoding=char_encoding, int_bits=int_bits)
    return c.compile(ast)


# ---------------------------------------------------------------------------
# Internal compiler
# ---------------------------------------------------------------------------


@dataclass
class _Compiler:
    """Internal compiler state — not part of the public API.

    Created by ``compile_basic()`` and discarded after compilation.

    Attributes:
        _program:     The IR program being built.
        _id_gen:      Produces unique monotonic instruction IDs.
        _next_reg:    Next expression-temporary register index.
        _loop_count:  Counter for unique FOR loop names.
        _label_count: Counter for unique synthetic label names (print routines).
        _for_stack:   Stack of open FOR loop records (innermost last).
    """

    _program: IrProgram = field(init=False)
    _id_gen: IDGenerator = field(default_factory=IDGenerator)
    _next_reg: int = field(default=_TEMP_BASE)
    _loop_count: int = field(default=0)
    _label_count: int = field(default=0)
    _for_stack: list[_ForRecord] = field(default_factory=list)
    char_encoding: str = field(default="ge225")
    int_bits: int = field(default=32)

    def __post_init__(self) -> None:
        """Initialize the IR program."""
        self._program = IrProgram(entry_label="_start")

    # ------------------------------------------------------------------
    # Top-level compilation
    # ------------------------------------------------------------------

    def compile(self, ast: ASTNode) -> CompileResult:
        """Run the full compilation and return the result."""
        self._emit_label("_start")
        for child in ast.children:
            if isinstance(child, ASTNode) and child.rule_name == "line":
                self._compile_line(child)
        # Epilogue: catch programs that fall through without END
        self._emit(IrOp.HALT)
        var_regs = {
            chr(ord("A") + i): _VAR_BASE + i for i in range(26)
        }
        return CompileResult(program=self._program, var_regs=var_regs)

    # ------------------------------------------------------------------
    # Line compilation
    # ------------------------------------------------------------------

    def _compile_line(self, node: ASTNode) -> None:
        """Compile one numbered BASIC line.

        Emits a LABEL for the line number, then dispatches the statement.

        Args:
            node: An ``ASTNode`` with ``rule_name == "line"``.
        """
        line_num: int | None = None
        stmt_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "LINE_NUM"):
                line_num = int(child.value)
            elif isinstance(child, ASTNode) and child.rule_name == "statement":
                stmt_node = child

        if line_num is None:
            return
        self._emit_label(f"_line_{line_num}")
        if stmt_node is not None:
            self._compile_statement(stmt_node)

    # ------------------------------------------------------------------
    # Statement dispatch
    # ------------------------------------------------------------------

    def _compile_statement(self, node: ASTNode) -> None:
        """Dispatch to the handler for the concrete statement type.

        Args:
            node: An ``ASTNode`` with ``rule_name == "statement"``.
        """
        for child in node.children:
            if isinstance(child, ASTNode):
                rule = child.rule_name
                if rule == "rem_stmt":
                    self._compile_rem(child)
                elif rule == "let_stmt":
                    self._compile_let(child)
                elif rule == "print_stmt":
                    self._compile_print(child)
                elif rule == "goto_stmt":
                    self._compile_goto(child)
                elif rule == "if_stmt":
                    self._compile_if(child)
                elif rule == "for_stmt":
                    self._compile_for(child)
                elif rule == "next_stmt":
                    self._compile_next(child)
                elif rule in ("end_stmt", "stop_stmt"):
                    self._emit(IrOp.HALT)
                elif rule in (
                    "gosub_stmt", "return_stmt", "dim_stmt",
                    "def_stmt", "input_stmt", "read_stmt",
                    "data_stmt", "restore_stmt",
                ):
                    raise CompileError(
                        f"'{rule.replace('_stmt', '').upper()}' is not "
                        f"supported in V1 of the compiled pipeline"
                    )
                return

    # ------------------------------------------------------------------
    # REM
    # ------------------------------------------------------------------

    def _compile_rem(self, node: ASTNode) -> None:
        """Emit a COMMENT instruction for a REM statement.

        The COMMENT opcode produces no machine code on any backend. It is
        emitted purely for human-readable IR output.

        Args:
            node: An ``ASTNode`` with ``rule_name == "rem_stmt"``.
        """
        # Gather the remark text from any token children after REM
        parts: list[str] = []
        for child in node.children:
            if isinstance(child, Token) and child.type != "KEYWORD":
                parts.append(child.value)
        text = " ".join(parts).strip() if parts else ""
        self._emit(IrOp.COMMENT, IrLabel(name=text or "REM"))

    # ------------------------------------------------------------------
    # LET
    # ------------------------------------------------------------------

    def _compile_let(self, node: ASTNode) -> None:
        """Compile a LET assignment: LET var = expr.

        The expression is compiled into a fresh register, then copied into
        the variable's fixed register via ADD_IMM v_var, v_val, 0.

        Args:
            node: An ``ASTNode`` with ``rule_name == "let_stmt"``.
        """
        var_node: ASTNode | None = None
        expr_node: ASTNode | None = None
        seen_eq = False

        for child in node.children:
            if isinstance(child, Token):
                if _type_is(child.type, "EQ"):
                    seen_eq = True
            elif isinstance(child, ASTNode):
                if child.rule_name == "variable" and not seen_eq:
                    var_node = child
                elif seen_eq and expr_node is None:
                    expr_node = child

        if var_node is None or expr_node is None:
            raise CompileError("malformed LET statement")

        var_name = self._extract_var_name(var_node)
        if var_name is None:
            raise CompileError("malformed LET: could not extract variable name")

        v_var = _scalar_reg(var_name)
        v_val = self._compile_expr(expr_node)

        # Copy expression result into the variable's register.
        # ADD_IMM v_var, v_val, 0 means: v_var = v_val + 0 = v_val.
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(index=v_var),
            IrRegister(index=v_val),
            IrImmediate(value=0),
        )

    # ------------------------------------------------------------------
    # PRINT
    # ------------------------------------------------------------------

    def _compile_print(self, node: ASTNode) -> None:
        """Compile a PRINT statement.

        Supports string literals, numeric expressions, and comma-separated
        mixtures.  A carriage return (GE-225 code 0o37) is always appended.

        Print_item dispatch:
          - STRING token  → emit one LOAD_IMM + SYSCALL 1 per character.
          - ASTNode (expr) → compile expression; emit decimal digit sequence
            via ``_emit_print_number``.

        Args:
            node: An ``ASTNode`` with ``rule_name == "print_stmt"``.

        Raises:
            CompileError: If a string literal contains a character with no
                          GE-225 typewriter code.
        """
        # Find the print_list child (contains the actual print items)
        print_list: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "print_list":
                print_list = child
                break

        cr = ord('\n') if self.char_encoding == "ascii" else CARRIAGE_RETURN_CODE

        if print_list is None:
            # Bare PRINT → just a carriage return
            self._emit_print_code(cr)
            return

        # Walk each print_item (print_sep nodes are ignored)
        for item_node in print_list.children:
            if isinstance(item_node, ASTNode) and item_node.rule_name == "print_item":
                self._compile_print_item(item_node)

        self._emit_print_code(cr)

    def _compile_print_item(self, node: ASTNode) -> None:
        """Compile one item from a PRINT argument list.

        A ``print_item`` node contains either a STRING token (string literal)
        or an ``expr`` subtree (numeric expression).

        Args:
            node: An ``ASTNode`` with ``rule_name == "print_item"``.
        """
        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "STRING"):
                text = child.value.strip('"\'')
                for ch in text:
                    ge225_code = ascii_to_ge225(ch)
                    if ge225_code is None:
                        raise CompileError(
                            f"character {ch!r} has no GE-225 typewriter equivalent; "
                            f"V1 supports A-Z, 0-9, space, and basic punctuation"
                        )
                    emit_code = ord(ch) if self.char_encoding == "ascii" else ge225_code
                    self._emit_print_code(emit_code)
                return
            elif isinstance(child, ASTNode):
                # Numeric expression: compile, then emit decimal digit sequence
                v_val = self._compile_expr(child)
                self._emit_print_number(v_val)
                return

    def _emit_print_code(self, code: int) -> None:
        """Emit LOAD_IMM + SYSCALL 1 to print one character.

        Args:
            code: Character code to print.  Interpretation depends on
                  ``char_encoding``: a GE-225 typewriter code (6-bit) when
                  ``"ge225"``, or an ASCII byte value when ``"ascii"``.
        """
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(index=_REG_SYSCALL_ARG),
            IrImmediate(value=code),
        )
        self._emit(IrOp.SYSCALL, IrImmediate(value=_SYSCALL_PRINT_CHAR), IrRegister(index=_REG_SYSCALL_ARG))

    def _emit_print_number(self, v_val: int) -> None:
        """Emit IR that prints the integer in register v_val as a decimal string.

        Algorithm (all arithmetic is done in generated IR, not in Python):

        1. Copy v_val to a scratch register r_work so the original is untouched.
        2. If r_work < 0: print '-', then negate (r_work = 0 − r_work).
        3. Unrolled digit-extraction for positions 1000000000 … 10:
             r_dig   = r_work / power          (integer quotient)
             r_work  = r_work − r_dig * power  (remainder = r_work mod power)
             if r_dig + r_started == 0: skip   (leading-zero suppression)
             else: print r_dig; started = 1
        4. Units digit: always print r_work (handles the value 0 correctly).

        For ``char_encoding="ge225"``: GE-225 typewriter codes for the digits
        0-9 equal the digit values, so the digit can be loaded directly into v0.
        For ``char_encoding="ascii"``: ASCII '0' = 48, so each digit is offset
        by 48 (``ADD_IMM v0, r_dig, 48``) before SYSCALL 1.

        Leading-zero suppression trick:
          r_dig ≥ 0 and r_started ∈ {0, 1}, so r_dig + r_started == 0 iff
          both are zero — a single ADD replaces the costlier AND-of-booleans.

        Args:
            v_val: Virtual register index holding the integer to print.
        """
        # Fresh unique label suffix for this particular print-number emission
        label_id = self._label_count
        self._label_count += 1

        # Scratch registers (all freshly allocated; spill slots are cheap)
        r_work   = self._new_reg()   # working copy of the magnitude
        r_zero   = self._new_reg()   # constant 0
        r_one    = self._new_reg()   # constant 1
        r_started = self._new_reg()  # 1 once the first non-zero digit is printed
        r_dig    = self._new_reg()   # current digit (reused each position)
        r_mttmp  = self._new_reg()   # temp: r_dig * power  (for mod computation)
        r_pow    = self._new_reg()   # power of 10 (reused each position)
        r_is_neg = self._new_reg()   # 1 if the original value was negative

        # Initialise
        self._emit(IrOp.ADD_IMM, IrRegister(r_work),    IrRegister(v_val),  IrImmediate(0))
        self._emit(IrOp.LOAD_IMM, IrRegister(r_zero),   IrImmediate(0))
        self._emit(IrOp.LOAD_IMM, IrRegister(r_one),    IrImmediate(1))
        self._emit(IrOp.LOAD_IMM, IrRegister(r_started), IrImmediate(0))

        # Sign handling: if r_work < 0, print '-' then negate
        minus_code = ord('-') if self.char_encoding == "ascii" else _GE225_MINUS_CODE
        l_pos = f"_pnum_{label_id}_pos"
        self._emit(IrOp.CMP_LT, IrRegister(r_is_neg), IrRegister(r_work), IrRegister(r_zero))
        self._emit(IrOp.BRANCH_Z, IrRegister(r_is_neg), IrLabel(l_pos))
        self._emit_print_code(minus_code)                                    # '-'
        r_neg = self._new_reg()
        self._emit(IrOp.SUB, IrRegister(r_neg),  IrRegister(r_zero), IrRegister(r_work))
        self._emit(IrOp.ADD_IMM, IrRegister(r_work), IrRegister(r_neg), IrImmediate(0))
        self._emit_label(l_pos)

        # For ASCII encoding, digits 0-9 must be offset by 48 to reach '0'-'9'.
        # For GE-225 encoding, typewriter codes 0-9 already map to '0'-'9'.
        digit_offset = 48 if self.char_encoding == "ascii" else 0

        # Unrolled digit extraction for each power of ten above the units place.
        #
        # The number of digit positions is derived from ``self.int_bits``:
        #
        #   max_value    = 2**(int_bits-1) - 1   (largest positive signed integer)
        #   digit_count  = len(str(max_value))    (decimal digits in that number)
        #   powers       = [10**(digit_count-1), …, 10]   (digit_count-1 entries)
        #
        # Examples:
        #   int_bits=32 → max 2,147,483,647 → 10 digits → powers [10^9 … 10]
        #   int_bits=20 → max     524,287   →  6 digits → powers [10^5 … 10]
        #
        # It is critical that every power constant fits in the target's signed
        # word.  For the GE-225 (20-bit), 10^5 = 100,000 < 524,287 ✓; but
        # 10^9 = 1,000,000,000 far exceeds the 20-bit range and would be
        # truncated on LOAD_IMM, producing garbled digit extraction.
        _max_val     = (1 << (self.int_bits - 1)) - 1
        _digit_count = len(str(_max_val))          # e.g. 10 for 32-bit, 6 for 20-bit
        _powers      = [10 ** (_digit_count - 1 - i) for i in range(_digit_count - 1)]

        for pos_idx, power in enumerate(_powers):
            l_skip = f"_pnum_{label_id}_s{pos_idx}"

            # Digit and remainder
            self._emit(IrOp.LOAD_IMM, IrRegister(r_pow),   IrImmediate(power))
            self._emit(IrOp.DIV,      IrRegister(r_dig),   IrRegister(r_work),  IrRegister(r_pow))
            self._emit(IrOp.MUL,      IrRegister(r_mttmp), IrRegister(r_dig),   IrRegister(r_pow))
            self._emit(IrOp.SUB,      IrRegister(r_work),  IrRegister(r_work),  IrRegister(r_mttmp))

            # Leading-zero suppression: skip if digit==0 AND not yet started
            r_sum = self._new_reg()
            self._emit(IrOp.ADD,      IrRegister(r_sum),   IrRegister(r_dig),   IrRegister(r_started))
            self._emit(IrOp.BRANCH_Z, IrRegister(r_sum),   IrLabel(l_skip))

            # Print this digit (add digit_offset to convert to printable code)
            self._emit(IrOp.ADD_IMM, IrRegister(_REG_SYSCALL_ARG), IrRegister(r_dig), IrImmediate(digit_offset))
            self._emit(IrOp.SYSCALL,  IrImmediate(_SYSCALL_PRINT_CHAR), IrRegister(index=_REG_SYSCALL_ARG))
            self._emit(IrOp.ADD_IMM,  IrRegister(r_started), IrRegister(r_one), IrImmediate(0))
            self._emit_label(l_skip)

        # Units digit: always print (this correctly handles the value 0)
        self._emit(IrOp.ADD_IMM, IrRegister(_REG_SYSCALL_ARG), IrRegister(r_work), IrImmediate(digit_offset))
        self._emit(IrOp.SYSCALL, IrImmediate(_SYSCALL_PRINT_CHAR), IrRegister(index=_REG_SYSCALL_ARG))

    # ------------------------------------------------------------------
    # GOTO
    # ------------------------------------------------------------------

    def _compile_goto(self, node: ASTNode) -> None:
        """Compile a GOTO statement: GOTO lineno.

        Emits an unconditional JUMP to the label for the target line.

        Args:
            node: An ``ASTNode`` with ``rule_name == "goto_stmt"``.
        """
        lineno = self._extract_lineno(node)
        self._emit(IrOp.JUMP, IrLabel(name=f"_line_{lineno}"))

    # ------------------------------------------------------------------
    # IF … THEN
    # ------------------------------------------------------------------

    def _compile_if(self, node: ASTNode) -> None:
        """Compile an IF expression relop expression THEN lineno statement.

        The six relational operators map to IR comparison opcodes.
        ``<=`` and ``>=`` are derived by flipping ``>`` and ``<`` with a
        NOT idiom (subtract 1, mask low bit).

        Args:
            node: An ``ASTNode`` with ``rule_name == "if_stmt"``.
        """
        # Collect sub-nodes: [expr1, relop_node, expr2, THEN, NUMBER]
        exprs: list[ASTNode] = []
        relop_token: Token | None = None
        lineno: int | None = None

        for child in node.children:
            if isinstance(child, Token):
                if _type_is(child.type, "NUMBER"):
                    lineno = int(child.value)
            elif isinstance(child, ASTNode):
                if child.rule_name == "relop":
                    relop_token = self._first_token(child)
                elif child.rule_name in (
                    "expr", "term", "power", "unary", "primary"
                ):
                    exprs.append(child)

        if lineno is None or relop_token is None or len(exprs) < 2:
            raise CompileError("malformed IF statement")

        v_lhs = self._compile_expr(exprs[0])
        v_rhs = self._compile_expr(exprs[1])
        relop = relop_token.value

        v_cmp = self._new_reg()
        label = f"_line_{lineno}"

        if relop == "<":
            self._emit(IrOp.CMP_LT, IrRegister(v_cmp), IrRegister(v_lhs), IrRegister(v_rhs))
            self._emit(IrOp.BRANCH_NZ, IrRegister(v_cmp), IrLabel(label))

        elif relop == ">":
            self._emit(IrOp.CMP_GT, IrRegister(v_cmp), IrRegister(v_lhs), IrRegister(v_rhs))
            self._emit(IrOp.BRANCH_NZ, IrRegister(v_cmp), IrLabel(label))

        elif relop == "=":
            self._emit(IrOp.CMP_EQ, IrRegister(v_cmp), IrRegister(v_lhs), IrRegister(v_rhs))
            self._emit(IrOp.BRANCH_NZ, IrRegister(v_cmp), IrLabel(label))

        elif relop == "<>":
            self._emit(IrOp.CMP_NE, IrRegister(v_cmp), IrRegister(v_lhs), IrRegister(v_rhs))
            self._emit(IrOp.BRANCH_NZ, IrRegister(v_cmp), IrLabel(label))

        elif relop == "<=":
            # LE = NOT GT: v_cmp = (lhs > rhs); flip to (lhs <= rhs)
            self._emit(IrOp.CMP_GT, IrRegister(v_cmp), IrRegister(v_lhs), IrRegister(v_rhs))
            v_flipped = self._not_bool(v_cmp)
            self._emit(IrOp.BRANCH_NZ, IrRegister(v_flipped), IrLabel(label))

        elif relop == ">=":
            # GE = NOT LT: v_cmp = (lhs < rhs); flip to (lhs >= rhs)
            self._emit(IrOp.CMP_LT, IrRegister(v_cmp), IrRegister(v_lhs), IrRegister(v_rhs))
            v_flipped = self._not_bool(v_cmp)
            self._emit(IrOp.BRANCH_NZ, IrRegister(v_flipped), IrLabel(label))

        else:
            raise CompileError(f"unknown relational operator: {relop!r}")

    def _not_bool(self, v_in: int) -> int:
        """Flip a boolean register: 0 → 1, 1 → 0.

        Implemented as: v_out = (v_in - 1) & 1.
        When v_in = 1: (1-1) & 1 = 0 & 1 = 0.
        When v_in = 0: (0-1) & 1 = (-1) & 1... but in two's complement,
        -1 in 20 bits = 0xFFFFF, and 0xFFFFF & 1 = 1. ✓

        Args:
            v_in: The register holding 0 or 1.

        Returns:
            A fresh register holding the flipped value.
        """
        v_sub = self._new_reg()
        v_out = self._new_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(v_sub),
            IrRegister(v_in),
            IrImmediate(-1),
        )
        self._emit(
            IrOp.AND_IMM,
            IrRegister(v_out),
            IrRegister(v_sub),
            IrImmediate(1),
        )
        return v_out

    # ------------------------------------------------------------------
    # FOR … TO [STEP]
    # ------------------------------------------------------------------

    def _compile_for(self, node: ASTNode) -> None:
        """Compile a FOR statement.

        Emits:
        1. Evaluate start, limit, step expressions into registers.
        2. Copy start into the variable register.
        3. Emit the pre-test label.
        4. Emit CMP_GT (var > limit) → BRANCH_NZ to end label.

        The body and NEXT are compiled later. NEXT pops this record and emits
        the increment and backward jump.

        Args:
            node: An ``ASTNode`` with ``rule_name == "for_stmt"``.
        """
        # Extract: FOR NAME EQ expr TO expr [STEP expr]
        var_name: str | None = None
        exprs: list[ASTNode] = []
        has_step = False

        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "NAME"):
                var_name = child.value.upper()
            elif isinstance(child, Token) and _type_is(child.type, "KEYWORD") and child.value.upper() == "STEP":
                has_step = True
            elif isinstance(child, ASTNode) and child.rule_name in (
                "expr", "term", "power", "unary", "primary"
            ):
                exprs.append(child)

        if var_name is None or len(exprs) < 2:
            raise CompileError("malformed FOR statement")

        v_var = _scalar_reg(var_name)

        # Compile start and limit expressions
        v_start = self._compile_expr(exprs[0])
        v_limit = self._compile_expr(exprs[1])

        # Compile step expression or default to 1
        if has_step and len(exprs) >= 3:
            v_step = self._compile_expr(exprs[2])
        else:
            v_step = self._new_reg()
            self._emit(
                IrOp.LOAD_IMM,
                IrRegister(v_step),
                IrImmediate(1),
            )

        # Initialize loop variable: var = start
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(v_var),
            IrRegister(v_start),
            IrImmediate(0),
        )

        loop_num = self._loop_count
        self._loop_count += 1
        check_label = f"_for_{loop_num}_check"
        end_label = f"_for_{loop_num}_end"

        self._emit_label(check_label)

        # Pre-test: exit if var > limit
        v_cmp = self._new_reg()
        self._emit(
            IrOp.CMP_GT,
            IrRegister(v_cmp),
            IrRegister(v_var),
            IrRegister(v_limit),
        )
        self._emit(IrOp.BRANCH_NZ, IrRegister(v_cmp), IrLabel(end_label))

        # Push record for NEXT to use
        self._for_stack.append(
            _ForRecord(
                var_name=var_name,
                var_reg=v_var,
                limit_reg=v_limit,
                step_reg=v_step,
                check_label=check_label,
                end_label=end_label,
                loop_num=loop_num,
            )
        )

    # ------------------------------------------------------------------
    # NEXT
    # ------------------------------------------------------------------

    def _compile_next(self, node: ASTNode) -> None:
        """Compile a NEXT statement.

        Pops the innermost FOR record, verifies the variable name matches,
        emits the increment and backward jump, then emits the loop end label.

        Args:
            node: An ``ASTNode`` with ``rule_name == "next_stmt"``.
        """
        if not self._for_stack:
            raise CompileError("NEXT without matching FOR")

        # Extract NEXT variable name
        next_var: str | None = None
        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "NAME"):
                next_var = child.value.upper()

        rec = self._for_stack[-1]

        if next_var is not None and next_var != rec.var_name:
            raise CompileError(
                f"NEXT {next_var} does not match innermost FOR {rec.var_name}"
            )

        self._for_stack.pop()

        # Increment: var += step
        self._emit(
            IrOp.ADD,
            IrRegister(rec.var_reg),
            IrRegister(rec.var_reg),
            IrRegister(rec.step_reg),
        )

        # Jump back to pre-test
        self._emit(IrOp.JUMP, IrLabel(rec.check_label))

        # End label (BRANCH_NZ target when var > limit)
        self._emit_label(rec.end_label)

    # ------------------------------------------------------------------
    # Expression compilation
    # ------------------------------------------------------------------

    def _compile_expr(self, node: ASTNode) -> int:
        """Recursively compile an expression node and return its result register.

        Walks the grammar's precedence tower:
          expr → term { (PLUS | MINUS) term }
          term → power { (STAR | SLASH) power }
          power → unary [CARET unary]
          unary → MINUS unary | primary
          primary → variable | NUMBER | LPAREN expr RPAREN

        Args:
            node: An ``ASTNode`` at any expression level.

        Returns:
            The virtual register index holding the expression result.

        Raises:
            CompileError: If the expression uses ``^`` (power), array indexing,
                          or any other V1-excluded feature.
        """
        rule = node.rule_name

        if rule == "primary":
            return self._compile_primary(node)
        if rule == "unary":
            return self._compile_unary(node)
        if rule in ("expr", "term"):
            return self._compile_binop_chain(node)
        if rule == "power":
            return self._compile_power(node)
        if rule == "variable":
            return self._compile_variable_expr(node)

        # Single-child pass-through for wrapper rules
        ast_children = [c for c in node.children if isinstance(c, ASTNode)]
        if len(ast_children) == 1:
            return self._compile_expr(ast_children[0])

        raise CompileError(f"unexpected expression node: {rule!r}")

    def _compile_primary(self, node: ASTNode) -> int:
        """Compile a primary expression (number literal, variable, or parenthesized expr).

        Args:
            node: An ``ASTNode`` with ``rule_name == "primary"``.

        Returns:
            The virtual register holding the result.
        """
        for child in node.children:
            if isinstance(child, Token):
                if _type_is(child.type, "NUMBER"):
                    v = self._new_reg()
                    val = int(float(child.value))   # truncate float literals
                    self._emit(IrOp.LOAD_IMM, IrRegister(v), IrImmediate(val))
                    return v
            elif isinstance(child, ASTNode):
                if child.rule_name == "variable":
                    return self._compile_variable_expr(child)
                return self._compile_expr(child)

        raise CompileError("empty primary expression")

    def _compile_variable_expr(self, node: ASTNode) -> int:
        """Return the fixed register for a scalar variable reference.

        Array element access (``NAME(expr)``) raises CompileError in V1.

        Args:
            node: An ``ASTNode`` with ``rule_name == "variable"``.

        Returns:
            The fixed virtual register for the scalar variable.
        """
        # Check for array syntax: if there's a LPAREN child, it's an array
        has_paren = any(
            isinstance(c, Token) and _type_is(c.type, "LPAREN")
            for c in node.children
        )
        if has_paren:
            raise CompileError(
                "array element access is not supported in V1"
            )

        name = self._extract_var_name(node)
        if name is None:
            raise CompileError("could not extract variable name from AST")
        return _scalar_reg(name)

    def _compile_unary(self, node: ASTNode) -> int:
        """Compile a unary expression (optional leading minus).

        Args:
            node: An ``ASTNode`` with ``rule_name == "unary"``.

        Returns:
            The virtual register holding the result.
        """
        has_minus = False
        inner_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "MINUS"):
                has_minus = True
            elif isinstance(child, ASTNode):
                inner_node = child

        if inner_node is None:
            raise CompileError("empty unary expression")

        v_inner = self._compile_expr(inner_node)

        if not has_minus:
            return v_inner

        # Negate: v_result = 0 - v_inner
        v_zero = self._new_reg()
        v_result = self._new_reg()
        self._emit(IrOp.LOAD_IMM, IrRegister(v_zero), IrImmediate(0))
        self._emit(
            IrOp.SUB,
            IrRegister(v_result),
            IrRegister(v_zero),
            IrRegister(v_inner),
        )
        return v_result

    def _compile_power(self, node: ASTNode) -> int:
        """Compile a power expression (unary [^ unary]).

        The ``^`` operator is not supported in V1. If detected, raises
        ``CompileError``. Otherwise, passes through to the unary child.

        Args:
            node: An ``ASTNode`` with ``rule_name == "power"``.

        Returns:
            The virtual register holding the result.
        """
        has_caret = any(
            isinstance(c, Token) and _type_is(c.type, "CARET")
            for c in node.children
        )
        if has_caret:
            raise CompileError(
                "the ^ (power) operator is not supported in V1"
            )
        # Single unary child
        for child in node.children:
            if isinstance(child, ASTNode):
                return self._compile_expr(child)
        raise CompileError("empty power expression")

    def _compile_binop_chain(self, node: ASTNode) -> int:
        """Compile a left-associative binary operator chain.

        Used for both ``expr`` (PLUS/MINUS) and ``term`` (STAR/SLASH).
        Processes children left to right:
          [operand, op_token, operand, op_token, operand, ...]

        Args:
            node: An ``ASTNode`` with ``rule_name == "expr"`` or ``"term"``.

        Returns:
            The virtual register holding the final result.
        """
        # Collect alternating (operand_node, op_token) pairs
        operands: list[int] = []
        operators: list[str] = []

        for child in node.children:
            if isinstance(child, Token) and any(
                _type_is(child.type, t) for t in ("PLUS", "MINUS", "STAR", "SLASH")
            ):
                operators.append(child.value)
            elif isinstance(child, ASTNode):
                operands.append(self._compile_expr(child))

        if not operands:
            raise CompileError(f"empty {node.rule_name} expression")

        result = operands[0]
        for i, op in enumerate(operators):
            rhs = operands[i + 1]
            v_out = self._new_reg()
            if op == "+":
                self._emit(
                    IrOp.ADD,
                    IrRegister(v_out),
                    IrRegister(result),
                    IrRegister(rhs),
                )
            elif op == "-":
                self._emit(
                    IrOp.SUB,
                    IrRegister(v_out),
                    IrRegister(result),
                    IrRegister(rhs),
                )
            elif op == "*":
                self._emit(
                    IrOp.MUL,
                    IrRegister(v_out),
                    IrRegister(result),
                    IrRegister(rhs),
                )
            elif op == "/":
                self._emit(
                    IrOp.DIV,
                    IrRegister(v_out),
                    IrRegister(result),
                    IrRegister(rhs),
                )
            else:
                raise CompileError(f"unknown binary operator: {op!r}")
            result = v_out

        return result

    # ------------------------------------------------------------------
    # Helper utilities
    # ------------------------------------------------------------------

    def _new_reg(self) -> int:
        """Allocate and return the next fresh expression-temporary register."""
        reg = self._next_reg
        self._next_reg += 1
        return reg

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

    def _extract_var_name(self, node: ASTNode) -> str | None:
        """Extract the variable name from a ``variable`` AST node.

        Returns the uppercase name string, or ``None`` if not found.

        Args:
            node: An ``ASTNode`` with ``rule_name == "variable"``.

        Returns:
            The variable name (e.g., ``"A"``, ``"B3"``), or ``None``.
        """
        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "NAME"):
                return child.value.upper()
        return None

    def _extract_lineno(self, node: ASTNode) -> int:
        """Extract an integer line number from a statement node.

        Args:
            node: An ``ASTNode`` containing a ``NUMBER`` token.

        Returns:
            The integer line number.

        Raises:
            CompileError: If no NUMBER token is found.
        """
        for child in node.children:
            if isinstance(child, Token) and _type_is(child.type, "NUMBER"):
                return int(child.value)
        raise CompileError(f"could not find line number in {node.rule_name}")

    def _first_token(self, node: ASTNode) -> Token | None:
        """Return the first Token child of an AST node.

        Args:
            node: Any ``ASTNode``.

        Returns:
            The first ``Token`` child, or ``None``.
        """
        for child in node.children:
            if isinstance(child, Token):
                return child
        return None
