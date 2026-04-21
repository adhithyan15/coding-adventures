"""Tetrad bytecode compiler (spec TET03).

Walks the Tetrad AST and emits a CodeObject — a self-contained bundle of
instructions, constants, and metadata consumed by the VM (spec TET04) and
the JIT (spec TET05).

The key property of this compiler is **two-path compilation**:

  Typed path  (both operands statically known to be u8):
      → emit 2-byte instruction  opcode + register (no feedback slot)
      → feedback_slot_count unchanged

  Untyped path (at least one operand Unknown):
      → emit 3-byte instruction  opcode + register + slot_index
      → feedback_slot_count++

This means a FULLY_TYPED function emits ZERO slot bytes throughout its body,
saving ROM bytes on the 4004 and eliminating the feedback-vector RAM
allocation at call time.

Algorithm overview (``compile_checked``):
  1. Register all functions in the function-index table.
  2. Compile each FnDecl into a sub-CodeObject; append to main.functions.
  3. Compile each GlobalDecl into instructions in the main CodeObject.
  4. Emit HALT.

Public API
----------
``compile_program(source) -> CodeObject``
    Full pipeline: lex + parse + type-check + compile.

``compile_checked(result: TypeCheckResult) -> CodeObject``
    Compile from a pre-built TypeCheckResult.  Raises CompilerError if
    result.errors is non-empty.

``CompilerError`` — raised for any compile-time violation.
``CodeObject``, ``Instruction``, ``Op`` — re-exported from bytecode.py.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from tetrad_parser.ast import (
    AssignStmt,
    BinaryExpr,
    Block,
    CallExpr,
    ExprStmt,
    FnDecl,
    GlobalDecl,
    GroupExpr,
    IfStmt,
    InExpr,
    IntLiteral,
    LetStmt,
    NameExpr,
    OutExpr,
    ReturnStmt,
    UnaryExpr,
    WhileStmt,
)
from tetrad_type_checker import check_source
from tetrad_type_checker.types import FunctionTypeStatus, TypeCheckResult, TypeInfo

from tetrad_compiler.bytecode import CodeObject, Instruction, Op

__all__ = [
    "compile_program",
    "compile_checked",
    "CompilerError",
    "CodeObject",
    "Instruction",
    "Op",
]

# ---------------------------------------------------------------------------
# Error type
# ---------------------------------------------------------------------------


class CompilerError(Exception):
    """Raised when the compiler cannot generate valid bytecode.

    Attributes mirror ParseError/TypeError so callers can handle all
    pipeline errors uniformly.
    """

    def __init__(self, message: str, line: int = 0, column: int = 0) -> None:
        super().__init__(message)
        self.message = message
        self.line = line
        self.column = column


# ---------------------------------------------------------------------------
# Opcode tables
# ---------------------------------------------------------------------------

# Binary operator string → opcode for the general (register) form.
_OP_MAP: dict[str, int] = {
    "+": Op.ADD,
    "-": Op.SUB,
    "*": Op.MUL,
    "/": Op.DIV,
    "%": Op.MOD,
    "&": Op.AND,
    "|": Op.OR,
    "^": Op.XOR,
    "<<": Op.SHL,
    ">>": Op.SHR,
    "==": Op.EQ,
    "!=": Op.NEQ,
    "<": Op.LT,
    "<=": Op.LTE,
    ">": Op.GT,
    ">=": Op.GTE,
}

# Bitwise ops never carry a feedback slot (always u8 in v1).
_BITWISE_OPS = frozenset(["&", "|", "^", "<<", ">>"])

# Short-circuit ops use jumps instead of LOGICAL_AND/LOGICAL_OR.
_SHORT_CIRCUIT = frozenset(["&&", "||"])


# ---------------------------------------------------------------------------
# Compiler state
# ---------------------------------------------------------------------------


@dataclass
class _CompilerState:
    """Mutable compilation context for one function or the top level.

    ``code``           — the CodeObject being built.
    ``locals``         — maps variable names to var_names indices for this scope.
    ``type_map``       — from TypeCheckResult; keyed by id(ast_node).
    ``function_index`` — maps function names to their index in all_functions.
    ``all_functions``  — the main CodeObject's functions list (shared reference).
                         Functions always call into the global function pool.
    ``next_register``  — next register index to hand out (0–7).
    ``free_registers`` — registers that have been freed and can be reused.
    ``next_slot``      — next feedback slot index to allocate.
    """

    code: CodeObject
    locals: dict[str, int]
    type_map: dict[int, TypeInfo]
    function_index: dict[str, int]
    all_functions: list[CodeObject]
    next_register: int = 0
    free_registers: list[int] = field(default_factory=list)
    next_slot: int = 0


# ---------------------------------------------------------------------------
# Register and slot allocation
# ---------------------------------------------------------------------------


def _alloc_reg(state: _CompilerState, line: int = 0, col: int = 0) -> int:
    """Return the next free register index, raising CompilerError on spill."""
    if state.free_registers:
        return state.free_registers.pop()
    r = state.next_register
    state.next_register += 1
    if state.next_register > 8:
        raise CompilerError(
            "expression too complex: exceeds 8 virtual registers", line, col
        )
    if state.code.register_count < state.next_register:
        state.code.register_count = state.next_register
    return r


def _free_reg(r: int, state: _CompilerState) -> None:
    """Return a register to the free pool."""
    state.free_registers.append(r)


def _alloc_slot(state: _CompilerState) -> int:
    """Allocate the next feedback slot index and increment the counter."""
    slot = state.next_slot
    state.next_slot += 1
    state.code.feedback_slot_count = state.next_slot
    return slot


# ---------------------------------------------------------------------------
# Instruction emission
# ---------------------------------------------------------------------------


def _emit(
    opcode: int,
    operands: list[int],
    state: _CompilerState,
    line: int = 0,
    col: int = 0,
) -> int:
    """Append an instruction and record it in the source map.

    Returns the index of the newly appended instruction.
    """
    idx = len(state.code.instructions)
    state.code.instructions.append(Instruction(opcode, list(operands)))
    state.code.source_map.append((idx, line, col))
    return idx


def _emit_jump(opcode: int, state: _CompilerState, line: int = 0, col: int = 0) -> int:
    """Emit a jump instruction with a placeholder offset.

    Returns the instruction index so the caller can patch it later.
    """
    return _emit(opcode, [0], state, line, col)


def _patch_jump(idx: int, state: _CompilerState) -> None:
    """Set the offset of instruction at ``idx`` to point to the current position."""
    target = len(state.code.instructions)
    state.code.instructions[idx].operands[0] = target - (idx + 1)


# ---------------------------------------------------------------------------
# Type query helpers
# ---------------------------------------------------------------------------


def _both_u8(
    left: object, right: object, type_map: dict[int, TypeInfo]
) -> bool:
    """Return True if both expression nodes are statically known to be u8."""
    li = type_map.get(id(left))
    ri = type_map.get(id(right))
    return li is not None and li.ty == "u8" and ri is not None and ri.ty == "u8"


def _is_u8(node: object, type_map: dict[int, TypeInfo]) -> bool:
    """Return True if ``node`` is statically known to be u8."""
    info = type_map.get(id(node))
    return info is not None and info.ty == "u8"


# ---------------------------------------------------------------------------
# Expression compilation
# ---------------------------------------------------------------------------


def _compile_expr(expr: object, state: _CompilerState) -> None:
    """Compile an expression; result lands in acc.

    This is the core of the compiler.  Every expression path must leave its
    result in the accumulator when it returns.
    """
    if isinstance(expr, IntLiteral):
        _compile_int_literal(expr, state)

    elif isinstance(expr, NameExpr):
        _compile_name(expr, state)

    elif isinstance(expr, BinaryExpr):
        _compile_binary(expr, state)

    elif isinstance(expr, UnaryExpr):
        _compile_unary(expr, state)

    elif isinstance(expr, CallExpr):
        _compile_call(expr, state)

    elif isinstance(expr, InExpr):
        _emit(Op.IO_IN, [], state, expr.line, expr.column)

    elif isinstance(expr, OutExpr):
        _compile_expr(expr.value, state)
        _emit(Op.IO_OUT, [], state, expr.line, expr.column)

    elif isinstance(expr, GroupExpr):
        _compile_expr(expr.expr, state)

    else:
        raise CompilerError(f"unknown expression type: {type(expr).__name__}")


def _compile_int_literal(expr: IntLiteral, state: _CompilerState) -> None:
    """Emit LDA_ZERO or LDA_IMM N for an integer literal.

    Validates the value is in u8 range [0, 255].
    """
    n = expr.value
    if n < 0 or n > 255:
        raise CompilerError(
            f"integer literal {n} out of u8 range (0–255)", expr.line, expr.column
        )
    if n == 0:
        _emit(Op.LDA_ZERO, [], state, expr.line, expr.column)
    else:
        _emit(Op.LDA_IMM, [n], state, expr.line, expr.column)


def _compile_name(expr: NameExpr, state: _CompilerState) -> None:
    """Emit LDA_VAR for a variable reference."""
    idx = state.locals.get(expr.name)
    if idx is None:
        raise CompilerError(
            f"undefined variable '{expr.name}'", expr.line, expr.column
        )
    _emit(Op.LDA_VAR, [idx], state, expr.line, expr.column)


def _compile_binary(expr: BinaryExpr, state: _CompilerState) -> None:
    """Compile a binary expression.

    Short-circuit &&/|| use jump sequences.
    Arithmetic/comparison use the two-register approach from the spec:
      1. compile L → acc
      2. STA_REG r_left  (save left)
      3. compile R → acc
      4. STA_REG r_right (save right)
      5. LDA_REG r_left  (load left back)
      6. OP r_right      (acc = left OP right)
    The ADD_IMM/SUB_IMM optimisation fires when the right operand is a literal.
    """
    op = expr.op
    ln, col = expr.line, expr.column

    if op == "&&":
        _compile_short_and(expr, state)
        return
    if op == "||":
        _compile_short_or(expr, state)
        return

    # ADD_IMM / SUB_IMM optimisation: right side is a literal
    if op in ("+", "-") and isinstance(expr.right, IntLiteral):
        _compile_expr(expr.left, state)
        n = expr.right.value
        if n < 0 or n > 255:
            raise CompilerError(
                f"integer literal {n} out of u8 range (0–255)",
                expr.right.line,
                expr.right.column,
            )
        imm_op = Op.ADD_IMM if op == "+" else Op.SUB_IMM
        if _is_u8(expr.left, state.type_map):
            _emit(imm_op, [n], state, ln, col)
        else:
            slot = _alloc_slot(state)
            _emit(imm_op, [n, slot], state, ln, col)
        return

    # General two-register path
    opcode = _OP_MAP[op]
    _compile_expr(expr.left, state)
    r_left = _alloc_reg(state, ln, col)
    _emit(Op.STA_REG, [r_left], state, ln, col)
    _compile_expr(expr.right, state)
    r_right = _alloc_reg(state, ln, col)
    _emit(Op.STA_REG, [r_right], state, ln, col)
    _emit(Op.LDA_REG, [r_left], state, ln, col)

    if op in _BITWISE_OPS or _both_u8(expr.left, expr.right, state.type_map):
        _emit(opcode, [r_right], state, ln, col)
    else:
        slot = _alloc_slot(state)
        _emit(opcode, [r_right, slot], state, ln, col)

    _free_reg(r_left, state)
    _free_reg(r_right, state)


def _compile_short_and(expr: BinaryExpr, state: _CompilerState) -> None:
    """Compile ``a && b`` using short-circuit jumps.

    a && b:
      eval a          → acc = a
      JZ  false_lbl   → if a==0, skip b
      eval b          → acc = b
      JMP end_lbl
    false_lbl:
      LDA_IMM 0       → acc = 0
    end_lbl:
    """
    ln, col = expr.line, expr.column
    _compile_expr(expr.left, state)
    jz_idx = _emit_jump(Op.JZ, state, ln, col)
    _compile_expr(expr.right, state)
    jmp_idx = _emit_jump(Op.JMP, state, ln, col)
    _patch_jump(jz_idx, state)      # false_label is here
    _emit(Op.LDA_IMM, [0], state, ln, col)
    _patch_jump(jmp_idx, state)     # end_label is here


def _compile_short_or(expr: BinaryExpr, state: _CompilerState) -> None:
    """Compile ``a || b`` using short-circuit jumps.

    a || b:
      eval a          → acc = a
      JNZ true_lbl    → if a!=0, skip b
      eval b          → acc = b
      JMP end_lbl
    true_lbl:
      LDA_IMM 1       → acc = 1
    end_lbl:
    """
    ln, col = expr.line, expr.column
    _compile_expr(expr.left, state)
    jnz_idx = _emit_jump(Op.JNZ, state, ln, col)
    _compile_expr(expr.right, state)
    jmp_idx = _emit_jump(Op.JMP, state, ln, col)
    _patch_jump(jnz_idx, state)     # true_label is here
    _emit(Op.LDA_IMM, [1], state, ln, col)
    _patch_jump(jmp_idx, state)     # end_label is here


def _compile_unary(expr: UnaryExpr, state: _CompilerState) -> None:
    """Compile a unary expression.

    ~x → NOT         (bitwise complement)
    !x → LOGICAL_NOT (logical not: 0→1, nonzero→0)
    -x → LDA_ZERO; STA_REG r; compile(x); SUB r  (wrapping negation)
    """
    ln, col = expr.line, expr.column
    _compile_expr(expr.operand, state)
    if expr.op == "~":
        _emit(Op.NOT, [], state, ln, col)
    elif expr.op == "!":
        _emit(Op.LOGICAL_NOT, [], state, ln, col)
    elif expr.op == "-":
        # Wrapping negation: 0 - x
        r = _alloc_reg(state, ln, col)
        _emit(Op.STA_REG, [r], state, ln, col)
        _emit(Op.LDA_ZERO, [], state, ln, col)
        if _is_u8(expr.operand, state.type_map):
            _emit(Op.SUB, [r], state, ln, col)
        else:
            slot = _alloc_slot(state)
            _emit(Op.SUB, [r, slot], state, ln, col)
        _free_reg(r, state)


def _compile_call(expr: CallExpr, state: _CompilerState) -> None:
    """Compile a function call.

    Evaluates each argument left-to-right, stores in R0..R(argc-1), then
    emits CALL func_idx argc slot.
    """
    ln, col = expr.line, expr.column
    fn_idx = state.function_index.get(expr.name)
    if fn_idx is None:
        raise CompilerError(f"undefined function '{expr.name}'", ln, col)

    # Verify argument count against the callee's CodeObject.
    callee = state.all_functions[fn_idx]
    expected = len(callee.params)
    if len(expr.args) != expected:
        raise CompilerError(
            f"'{expr.name}' expects {expected} args, got {len(expr.args)}",
            ln,
            col,
        )

    for i, arg in enumerate(expr.args):
        _compile_expr(arg, state)
        _emit(Op.STA_REG, [i], state, ln, col)

    slot = _alloc_slot(state)
    _emit(Op.CALL, [fn_idx, len(expr.args), slot], state, ln, col)


# ---------------------------------------------------------------------------
# Statement compilation
# ---------------------------------------------------------------------------


def _compile_stmt(stmt: object, state: _CompilerState) -> None:
    """Compile one statement, updating ``state.locals`` for new variables."""
    if isinstance(stmt, LetStmt):
        _compile_let(stmt, state)
    elif isinstance(stmt, AssignStmt):
        _compile_assign(stmt, state)
    elif isinstance(stmt, ReturnStmt):
        _compile_return(stmt, state)
    elif isinstance(stmt, IfStmt):
        _compile_if(stmt, state)
    elif isinstance(stmt, WhileStmt):
        _compile_while(stmt, state)
    elif isinstance(stmt, ExprStmt):
        _compile_expr(stmt.expr, state)
    elif isinstance(stmt, Block):
        _compile_block(stmt, state)


def _compile_block(block: Block, state: _CompilerState) -> None:
    """Compile all statements in a block sequentially."""
    for stmt in block.stmts:
        _compile_stmt(stmt, state)


def _compile_let(stmt: LetStmt, state: _CompilerState) -> None:
    """``let x = expr;`` → compile expr, allocate var slot, STA_VAR."""
    _compile_expr(stmt.value, state)
    idx = len(state.code.var_names)
    state.code.var_names.append(stmt.name)
    state.locals[stmt.name] = idx
    _emit(Op.STA_VAR, [idx], state, stmt.line, stmt.column)


def _compile_assign(stmt: AssignStmt, state: _CompilerState) -> None:
    """``x = expr;`` → compile expr, look up existing var slot, STA_VAR."""
    idx = state.locals.get(stmt.name)
    if idx is None:
        raise CompilerError(
            f"undefined variable '{stmt.name}'", stmt.line, stmt.column
        )
    _compile_expr(stmt.value, state)
    _emit(Op.STA_VAR, [idx], state, stmt.line, stmt.column)


def _compile_return(stmt: ReturnStmt, state: _CompilerState) -> None:
    """``return expr;`` or bare ``return;``."""
    if stmt.value is not None:
        _compile_expr(stmt.value, state)
    else:
        _emit(Op.LDA_ZERO, [], state, stmt.line, stmt.column)
    _emit(Op.RET, [], state, stmt.line, stmt.column)


def _compile_if(stmt: IfStmt, state: _CompilerState) -> None:
    """if/else with forward jump patching.

    if cond { then } else { else_block }
      compile(cond)
      JZ  patch_1        ; if false, skip then
      compile(then)
      JMP patch_2        ; skip else
    patch_1:
      compile(else_block) ; may be empty
    patch_2:
    """
    ln, col = stmt.line, stmt.column
    _compile_expr(stmt.condition, state)
    jz_idx = _emit_jump(Op.JZ, state, ln, col)
    _compile_block(stmt.then_block, state)
    if stmt.else_block is not None:
        jmp_idx = _emit_jump(Op.JMP, state, ln, col)
        _patch_jump(jz_idx, state)
        _compile_block(stmt.else_block, state)
        _patch_jump(jmp_idx, state)
    else:
        _patch_jump(jz_idx, state)


def _compile_while(stmt: WhileStmt, state: _CompilerState) -> None:
    """while loop using JMP_LOOP for the back-edge.

    loop_start:
      compile(cond)
      JZ   patch_exit    ; exit if false
      compile(body)
      JMP_LOOP loop_start ; backward jump (marks loop back-edge for VM)
    patch_exit:
    """
    ln, col = stmt.line, stmt.column
    loop_start = len(state.code.instructions)
    _compile_expr(stmt.condition, state)
    jz_idx = _emit_jump(Op.JZ, state, ln, col)
    _compile_block(stmt.body, state)
    # Backward jump: offset is relative to instruction AFTER this JMP_LOOP
    back_idx = len(state.code.instructions)
    offset = loop_start - (back_idx + 1)  # negative
    _emit(Op.JMP_LOOP, [offset], state, ln, col)
    _patch_jump(jz_idx, state)


# ---------------------------------------------------------------------------
# Function compilation
# ---------------------------------------------------------------------------


def _compile_fn(
    fn: FnDecl,
    main_state: _CompilerState,
    type_check_result: TypeCheckResult,
) -> CodeObject:
    """Compile a function declaration into a child CodeObject.

    Parameters are loaded from R0..R(argc-1) at function entry (the call
    convention established by _compile_call) and stored into var_names[0..argc-1].
    The function body then accesses all names via LDA_VAR/STA_VAR.
    """
    fn_status = type_check_result.env.function_status.get(
        fn.name, FunctionTypeStatus.UNTYPED
    )
    code = CodeObject(
        name=fn.name,
        params=list(fn.params),
        type_status=fn_status,
        immediate_jit_eligible=(fn_status is FunctionTypeStatus.FULLY_TYPED),
    )
    fn_locals: dict[str, int] = {}
    state = _CompilerState(
        code=code,
        locals=fn_locals,
        type_map=main_state.type_map,
        function_index=main_state.function_index,
        all_functions=main_state.all_functions,
    )

    # Copy each parameter from its argument register into var_names.
    # The caller placed arg i into R[i] before CALL.
    for i, param in enumerate(fn.params):
        var_idx = len(code.var_names)
        code.var_names.append(param)
        fn_locals[param] = var_idx
        _emit(Op.LDA_REG, [i], state, fn.line, fn.column)
        _emit(Op.STA_VAR, [var_idx], state, fn.line, fn.column)

    _compile_block(fn.body, state)

    # Implicit return 0 if the function falls off the end.
    last = code.instructions[-1] if code.instructions else None
    if last is None or last.opcode != Op.RET:
        _emit(Op.LDA_ZERO, [], state, fn.line, fn.column)
        _emit(Op.RET, [], state, fn.line, fn.column)

    return code


# ---------------------------------------------------------------------------
# Main entry points
# ---------------------------------------------------------------------------


def compile_checked(result: TypeCheckResult) -> CodeObject:
    """Compile a type-checked Tetrad program into a CodeObject.

    Raises ``CompilerError`` if ``result.errors`` is non-empty or if the AST
    contains a compile-time error (undefined variable, register spill, etc.).
    """
    if result.errors:
        first = result.errors[0]
        raise CompilerError(first.message, first.line, first.column)

    program = result.program
    main_code = CodeObject(
        name="<main>",
        params=[],
        type_status=FunctionTypeStatus.FULLY_TYPED,
        immediate_jit_eligible=True,
    )
    global_locals: dict[str, int] = {}
    function_index: dict[str, int] = {}

    # Pass 1: Register all functions in the index (enables forward calls).
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            idx = len(main_code.functions)
            function_index[decl.name] = idx
            # Placeholder — will be replaced in pass 2.
            main_code.functions.append(
                CodeObject(name=decl.name, params=list(decl.params))
            )

    main_state = _CompilerState(
        code=main_code,
        locals=global_locals,
        type_map=result.type_map,
        function_index=function_index,
        all_functions=main_code.functions,
    )

    # Pass 2: Compile each function body into its CodeObject slot.
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            fn_code = _compile_fn(decl, main_state, result)
            function_index_slot = function_index[decl.name]
            main_code.functions[function_index_slot] = fn_code

    # Pass 3: Compile global declarations into main's instruction stream.
    for decl in program.decls:
        if isinstance(decl, GlobalDecl):
            _compile_expr(decl.value, main_state)
            idx = len(main_code.var_names)
            main_code.var_names.append(decl.name)
            global_locals[decl.name] = idx
            _emit(Op.STA_VAR, [idx], main_state, decl.line, decl.column)

    _emit(Op.HALT, [], main_state)
    return main_code


def compile_program(source: str) -> CodeObject:
    """Lex + parse + type-check + compile a Tetrad source string.

    Raises ``LexError``, ``ParseError``, or ``CompilerError`` on failure.
    Type errors are wrapped in ``CompilerError``.
    """
    result = check_source(source)
    return compile_checked(result)
