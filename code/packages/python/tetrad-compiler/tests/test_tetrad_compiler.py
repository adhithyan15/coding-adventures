"""Tests for tetrad_compiler.compile_program() and compile_checked().

Coverage target: ≥95%.

Test strategy mirrors spec TET03:
  - Instruction emission for each expression/statement form
  - Two-path compilation (typed vs untyped feedback slots)
  - Control flow: if/else, while, short-circuit &&/||
  - Function calls and parameter passing
  - Compile-time error conditions
  - Source map population
"""

from __future__ import annotations

import pytest

from tetrad_compiler import CompilerError, Op, compile_program
from tetrad_compiler.bytecode import CodeObject, Instruction

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def cp(src: str) -> CodeObject:
    return compile_program(src)


def opcodes(code: CodeObject) -> list[int]:
    return [i.opcode for i in code.instructions]


def operands(code: CodeObject, idx: int) -> list[int]:
    return code.instructions[idx].operands


def fn(code: CodeObject, name: str) -> CodeObject:
    """Return the sub-CodeObject for a named function."""
    for f in code.functions:
        if f.name == name:
            return f
    raise KeyError(name)


# ---------------------------------------------------------------------------
# 1. CodeObject and Instruction dataclasses
# ---------------------------------------------------------------------------


def test_instruction_default_operands() -> None:
    instr = Instruction(Op.HALT)
    assert instr.operands == []


def test_instruction_with_operands() -> None:
    instr = Instruction(Op.ADD, [2, 5])
    assert instr.opcode == Op.ADD
    assert instr.operands == [2, 5]


def test_code_object_defaults() -> None:
    code = CodeObject(name="<main>", params=[])
    assert code.instructions == []
    assert code.functions == []
    assert code.var_names == []
    assert code.feedback_slot_count == 0
    assert not code.immediate_jit_eligible


# ---------------------------------------------------------------------------
# 2. Integer literal emission
# ---------------------------------------------------------------------------


def test_literal_zero_uses_lda_zero() -> None:
    code = cp("let x = 0;")
    assert code.instructions[0].opcode == Op.LDA_ZERO


def test_literal_nonzero_uses_lda_imm() -> None:
    code = cp("let x = 42;")
    assert code.instructions[0].opcode == Op.LDA_IMM
    assert code.instructions[0].operands == [42]


def test_literal_255_is_valid() -> None:
    code = cp("let x = 255;")
    assert code.instructions[0].opcode == Op.LDA_IMM
    assert code.instructions[0].operands == [255]


def test_literal_overflow_raises() -> None:
    with pytest.raises(CompilerError, match="out of u8 range"):
        cp("let x = 256;")


def test_literal_hex_is_valid() -> None:
    code = cp("let x = 0xFF;")
    assert code.instructions[0].operands[0] == 255


# ---------------------------------------------------------------------------
# 3. Variable declaration and access
# ---------------------------------------------------------------------------


def test_let_stores_to_var() -> None:
    code = cp("let x = 10;")
    # LDA_IMM 10; STA_VAR 0; HALT
    assert code.instructions[1].opcode == Op.STA_VAR
    assert code.instructions[1].operands == [0]
    assert code.var_names[0] == "x"


def test_let_zero_uses_lda_zero() -> None:
    code = cp("let x = 0;")
    assert opcodes(code)[0] == Op.LDA_ZERO


def test_name_expr_emits_lda_var() -> None:
    code = cp("fn f() { let x = 5; let y = x; }")
    f = fn(code, "f")
    # After let x (STA_VAR 0): let y = x → LDA_VAR 0; STA_VAR 1
    lda_idx = next(
        i for i, instr in enumerate(f.instructions) if instr.opcode == Op.LDA_VAR
    )
    assert f.instructions[lda_idx].operands == [0]


def test_undefined_variable_raises() -> None:
    with pytest.raises(CompilerError, match="undefined variable"):
        cp("fn f() { let x = y; }")


def test_assign_updates_existing_var() -> None:
    code = cp("fn f() { let x = 1; x = 2; }")
    f = fn(code, "f")
    # Second STA_VAR should also be index 0 (for x)
    sta_ops = [i for i in f.instructions if i.opcode == Op.STA_VAR]
    assert all(s.operands[0] == 0 for s in sta_ops)


def test_assign_undefined_raises() -> None:
    with pytest.raises(CompilerError, match="undefined variable"):
        cp("fn f() { x = 1; }")


# ---------------------------------------------------------------------------
# 4. Arithmetic two-path compilation
# ---------------------------------------------------------------------------


def test_typed_add_no_slot() -> None:
    # Both params u8 → typed → no slot
    code = cp("fn f(a: u8, b: u8) -> u8 { return a + b; }")
    f = fn(code, "f")
    add = next(i for i in f.instructions if i.opcode == Op.ADD)
    assert len(add.operands) == 1   # typed: [r], no slot


def test_untyped_add_has_slot() -> None:
    code = cp("fn f(a, b) { return a + b; }")
    f = fn(code, "f")
    add = next(i for i in f.instructions if i.opcode == Op.ADD)
    assert len(add.operands) == 2   # untyped: [r, slot]


def test_typed_add_feedback_slot_count_zero() -> None:
    code = cp("fn f(a: u8, b: u8) -> u8 { return a + b; }")
    f = fn(code, "f")
    assert f.feedback_slot_count == 0


def test_untyped_add_feedback_slot_count_nonzero() -> None:
    code = cp("fn f(a, b) { return a + b; }")
    f = fn(code, "f")
    assert f.feedback_slot_count > 0


def test_two_binary_ops_get_separate_slots() -> None:
    code = cp("fn f(a, b, c) { return a + b + c; }")
    f = fn(code, "f")
    adds = [i for i in f.instructions if i.opcode == Op.ADD]
    slots = [i.operands[1] for i in adds if len(i.operands) == 2]
    assert len(slots) == 2
    assert slots[0] != slots[1]


# ---------------------------------------------------------------------------
# 5. ADD_IMM / SUB_IMM optimisation
# ---------------------------------------------------------------------------


def test_add_literal_uses_add_imm() -> None:
    code = cp("fn f(a: u8) -> u8 { return a + 1; }")
    f = fn(code, "f")
    imm = next((i for i in f.instructions if i.opcode == Op.ADD_IMM), None)
    assert imm is not None
    assert imm.operands[0] == 1


def test_sub_literal_uses_sub_imm() -> None:
    code = cp("fn f(a: u8) -> u8 { return a - 1; }")
    f = fn(code, "f")
    imm = next((i for i in f.instructions if i.opcode == Op.SUB_IMM), None)
    assert imm is not None
    assert imm.operands[0] == 1


def test_typed_add_imm_no_slot() -> None:
    code = cp("fn f(a: u8) -> u8 { return a + 5; }")
    f = fn(code, "f")
    imm = next(i for i in f.instructions if i.opcode == Op.ADD_IMM)
    assert len(imm.operands) == 1  # typed: [n], no slot


def test_untyped_add_imm_has_slot() -> None:
    code = cp("fn f(a) { return a + 5; }")
    f = fn(code, "f")
    imm = next(i for i in f.instructions if i.opcode == Op.ADD_IMM)
    assert len(imm.operands) == 2  # untyped: [n, slot]


# ---------------------------------------------------------------------------
# 6. Bitwise operations (never slotted)
# ---------------------------------------------------------------------------


def test_bitwise_and_no_slot() -> None:
    code = cp("fn f(a, b) { return a & b; }")
    f = fn(code, "f")
    a = next(i for i in f.instructions if i.opcode == Op.AND)
    assert len(a.operands) == 1


def test_bitwise_or_no_slot() -> None:
    code = cp("fn f(a, b) { return a | b; }")
    f = fn(code, "f")
    assert any(i.opcode == Op.OR for i in f.instructions)
    o = next(i for i in f.instructions if i.opcode == Op.OR)
    assert len(o.operands) == 1


def test_bitwise_xor_no_slot() -> None:
    code = cp("fn f(a, b) { return a ^ b; }")
    f = fn(code, "f")
    x = next(i for i in f.instructions if i.opcode == Op.XOR)
    assert len(x.operands) == 1


def test_shl_no_slot() -> None:
    code = cp("fn f(a, b) { return a << b; }")
    f = fn(code, "f")
    s = next(i for i in f.instructions if i.opcode == Op.SHL)
    assert len(s.operands) == 1


def test_shr_no_slot() -> None:
    code = cp("fn f(a, b) { return a >> b; }")
    f = fn(code, "f")
    s = next(i for i in f.instructions if i.opcode == Op.SHR)
    assert len(s.operands) == 1


# ---------------------------------------------------------------------------
# 7. Comparison operations
# ---------------------------------------------------------------------------


def test_typed_eq_no_slot() -> None:
    code = cp("fn f(a: u8, b: u8) -> u8 { return a == b; }")
    f = fn(code, "f")
    eq = next(i for i in f.instructions if i.opcode == Op.EQ)
    assert len(eq.operands) == 1


def test_untyped_lt_has_slot() -> None:
    code = cp("fn f(a, b) { return a < b; }")
    f = fn(code, "f")
    lt = next(i for i in f.instructions if i.opcode == Op.LT)
    assert len(lt.operands) == 2


def test_all_comparison_opcodes() -> None:
    ops = [("==", Op.EQ), ("!=", Op.NEQ), ("<", Op.LT),
           ("<=", Op.LTE), (">", Op.GT), (">=", Op.GTE)]
    for sym, opcode in ops:
        code = cp(f"fn f(a, b) {{ return a {sym} b; }}")
        f = fn(code, "f")
        assert any(i.opcode == opcode for i in f.instructions), sym


# ---------------------------------------------------------------------------
# 8. Unary operations
# ---------------------------------------------------------------------------


def test_bitwise_not_emits_not() -> None:
    code = cp("fn f(a) { return ~a; }")
    f = fn(code, "f")
    assert any(i.opcode == Op.NOT for i in f.instructions)


def test_logical_not_emits_logical_not() -> None:
    code = cp("fn f(a) { return !a; }")
    f = fn(code, "f")
    assert any(i.opcode == Op.LOGICAL_NOT for i in f.instructions)


def test_unary_negate_emits_lda_zero_sub() -> None:
    code = cp("fn f(a) { return -a; }")
    f = fn(code, "f")
    lda_zero_found = any(i.opcode == Op.LDA_ZERO for i in f.instructions)
    sub_found = any(i.opcode == Op.SUB for i in f.instructions)
    assert lda_zero_found
    assert sub_found


def test_unary_negate_typed_no_slot() -> None:
    code = cp("fn f(a: u8) -> u8 { return -a; }")
    f = fn(code, "f")
    sub = next(i for i in f.instructions if i.opcode == Op.SUB)
    assert len(sub.operands) == 1


def test_unary_negate_untyped_has_slot() -> None:
    code = cp("fn f(a) { return -a; }")
    f = fn(code, "f")
    sub = next(i for i in f.instructions if i.opcode == Op.SUB)
    assert len(sub.operands) == 2


# ---------------------------------------------------------------------------
# 9. Short-circuit &&, ||
# ---------------------------------------------------------------------------


def test_short_circuit_and_uses_jz() -> None:
    code = cp("fn f(a, b) { return a && b; }")
    f = fn(code, "f")
    assert any(i.opcode == Op.JZ for i in f.instructions)


def test_short_circuit_or_uses_jnz() -> None:
    code = cp("fn f(a, b) { return a || b; }")
    f = fn(code, "f")
    assert any(i.opcode == Op.JNZ for i in f.instructions)


def test_short_circuit_and_false_path_has_lda_zero() -> None:
    code = cp("fn f(a, b) { return a && b; }")
    f = fn(code, "f")
    # The false path emits LDA_IMM 0
    lda_zeros = [i for i in f.instructions if i.opcode == Op.LDA_IMM
                 and i.operands == [0]]
    assert len(lda_zeros) >= 1


def test_short_circuit_or_true_path_has_lda_one() -> None:
    code = cp("fn f(a, b) { return a || b; }")
    f = fn(code, "f")
    lda_ones = [i for i in f.instructions if i.opcode == Op.LDA_IMM
                and i.operands == [1]]
    assert len(lda_ones) >= 1


# ---------------------------------------------------------------------------
# 10. in() and out()
# ---------------------------------------------------------------------------


def test_in_emits_io_in() -> None:
    code = cp("fn f() { let x = in(); }")
    f = fn(code, "f")
    assert any(i.opcode == Op.IO_IN for i in f.instructions)


def test_out_emits_io_out() -> None:
    code = cp("fn f() { out(42); }")
    f = fn(code, "f")
    assert any(i.opcode == Op.IO_OUT for i in f.instructions)


# ---------------------------------------------------------------------------
# 11. Control flow
# ---------------------------------------------------------------------------


def test_if_emits_jz() -> None:
    code = cp("fn f(a: u8) { if a > 0 { let x = 1; } }")
    f = fn(code, "f")
    assert any(i.opcode == Op.JZ for i in f.instructions)


def test_if_jz_offset_skips_body() -> None:
    code = cp("fn f(a: u8) { if a > 0 { let x = 1; } }")
    f = fn(code, "f")
    jz = next(i for i in f.instructions if i.opcode == Op.JZ)
    assert jz.operands[0] > 0  # forward offset (skips body)


def test_if_else_has_jmp_after_then() -> None:
    code = cp("fn f(a: u8) { if a > 0 { let x = 1; } else { let y = 2; } }")
    f = fn(code, "f")
    assert any(i.opcode == Op.JMP for i in f.instructions)


def test_while_emits_jz_and_jmp_loop() -> None:
    code = cp("fn f(a: u8) { while a > 0 { a = a - 1; } }")
    f = fn(code, "f")
    assert any(i.opcode == Op.JZ for i in f.instructions)
    assert any(i.opcode == Op.JMP_LOOP for i in f.instructions)


def test_while_jmp_loop_offset_is_negative() -> None:
    code = cp("fn f(a: u8) { while a > 0 { a = a - 1; } }")
    f = fn(code, "f")
    loop = next(i for i in f.instructions if i.opcode == Op.JMP_LOOP)
    assert loop.operands[0] < 0  # backward jump


# ---------------------------------------------------------------------------
# 12. Return statement
# ---------------------------------------------------------------------------


def test_return_expr_emits_ret() -> None:
    code = cp("fn f(a: u8) -> u8 { return a; }")
    f = fn(code, "f")
    assert any(i.opcode == Op.RET for i in f.instructions)


def test_bare_return_emits_lda_zero_ret() -> None:
    code = cp("fn f() { return; }")
    f = fn(code, "f")
    ops = opcodes(f)
    assert Op.LDA_ZERO in ops
    assert Op.RET in ops


def test_implicit_return_appended() -> None:
    # Function with no explicit return gets LDA_ZERO + RET appended
    code = cp("fn f() { let x = 1; }")
    f = fn(code, "f")
    assert f.instructions[-1].opcode == Op.RET


# ---------------------------------------------------------------------------
# 13. Function declaration and call
# ---------------------------------------------------------------------------


def test_function_in_main_functions_list() -> None:
    code = cp("fn add(a, b) { return a + b; }")
    assert len(code.functions) == 1
    assert code.functions[0].name == "add"


def test_function_has_params() -> None:
    code = cp("fn add(a, b) { return a + b; }")
    assert code.functions[0].params == ["a", "b"]


def test_call_emits_call_opcode() -> None:
    code = cp("fn add(a, b) { return a + b; }\nfn main() { let x = add(1, 2); }")
    f = fn(code, "main")
    assert any(i.opcode == Op.CALL for i in f.instructions)


def test_call_has_func_idx_argc_slot() -> None:
    code = cp("fn add(a, b) { return a + b; }\nfn main() { let x = add(1, 2); }")
    f = fn(code, "main")
    call = next(i for i in f.instructions if i.opcode == Op.CALL)
    func_idx, argc, slot = call.operands
    assert func_idx == 0     # add is function index 0
    assert argc == 2


def test_call_stores_args_in_registers() -> None:
    code = cp("fn add(a, b) { return a + b; }\nfn main() { let x = add(1, 2); }")
    f = fn(code, "main")
    sta_regs = [i for i in f.instructions if i.opcode == Op.STA_REG]
    # R0 and R1 should be used for arg passing
    assert any(i.operands[0] == 0 for i in sta_regs)
    assert any(i.operands[0] == 1 for i in sta_regs)


def test_call_undefined_function_raises() -> None:
    with pytest.raises(CompilerError, match="undefined function"):
        cp("fn f() { let x = ghost(1); }")


def test_call_wrong_argc_raises() -> None:
    with pytest.raises(CompilerError, match="expects 2 args, got 3"):
        cp("fn add(a, b) { return a + b; }\nfn f() { let x = add(1, 2, 3); }")


def test_param_loading_from_registers() -> None:
    # The function preamble must copy params from R0.. into var_names
    code = cp("fn f(a, b) { return a; }")
    f = fn(code, "f")
    # First instruction should be LDA_REG 0 (load first param from R0)
    assert f.instructions[0].opcode == Op.LDA_REG
    assert f.instructions[0].operands == [0]


# ---------------------------------------------------------------------------
# 14. Immediate JIT eligibility
# ---------------------------------------------------------------------------


def test_fully_typed_is_jit_eligible() -> None:
    code = cp("fn f(a: u8, b: u8) -> u8 { return a + b; }")
    f = fn(code, "f")
    assert f.immediate_jit_eligible


def test_untyped_not_jit_eligible() -> None:
    code = cp("fn f(a, b) { return a + b; }")
    f = fn(code, "f")
    assert not f.immediate_jit_eligible


# ---------------------------------------------------------------------------
# 15. Global declarations
# ---------------------------------------------------------------------------


def test_global_stored_in_var_names() -> None:
    code = cp("let COUNT = 10;")
    assert "COUNT" in code.var_names


def test_global_in_main_instructions() -> None:
    code = cp("let COUNT = 10;")
    # LDA_IMM 10; STA_VAR 0; HALT
    assert code.instructions[0].opcode == Op.LDA_IMM
    assert code.instructions[1].opcode == Op.STA_VAR
    assert code.instructions[-1].opcode == Op.HALT


def test_main_ends_with_halt() -> None:
    code = cp("")
    assert code.instructions[-1].opcode == Op.HALT


# ---------------------------------------------------------------------------
# 16. Source map
# ---------------------------------------------------------------------------


def test_source_map_populated() -> None:
    code = cp("let x = 42;")
    assert len(code.source_map) > 0


def test_source_map_entry_is_triple() -> None:
    code = cp("let x = 42;")
    entry = code.source_map[0]
    assert len(entry) == 3
    instr_idx, line, col = entry
    assert isinstance(instr_idx, int)


# ---------------------------------------------------------------------------
# 17. compile_checked errors
# ---------------------------------------------------------------------------


def test_compile_checked_raises_on_type_errors() -> None:
    from tetrad_parser import parse
    from tetrad_type_checker import check

    prog = parse("fn f() -> u8 { return in(); }")
    result = check(prog)
    assert result.errors  # type error from type checker
    with pytest.raises(CompilerError):
        from tetrad_compiler import compile_checked
        compile_checked(result)


# ---------------------------------------------------------------------------
# 18. End-to-end programs
# ---------------------------------------------------------------------------


TYPED_ADD = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
MULTIPLY = """
fn multiply(a, b) {
    let result = 0;
    while b > 0 {
        result = result + a;
        b = b - 1;
    }
    return result;
}
"""
ECHO = """
fn main() {
    let x = in();
    out(x);
}
"""


def test_compile_typed_add() -> None:
    code = cp(TYPED_ADD)
    f = fn(code, "add")
    assert f.feedback_slot_count == 0
    assert f.immediate_jit_eligible


def test_compile_multiply() -> None:
    code = cp(MULTIPLY)
    f = fn(code, "multiply")
    assert not f.immediate_jit_eligible
    # while loop must have JZ and JMP_LOOP
    assert any(i.opcode == Op.JZ for i in f.instructions)
    assert any(i.opcode == Op.JMP_LOOP for i in f.instructions)


def test_compile_echo() -> None:
    code = cp(ECHO)
    f = fn(code, "main")
    assert any(i.opcode == Op.IO_IN for i in f.instructions)
    assert any(i.opcode == Op.IO_OUT for i in f.instructions)


def test_compile_mixed_program() -> None:
    code = cp(TYPED_ADD + MULTIPLY)
    assert len(code.functions) == 2
    assert fn(code, "add").immediate_jit_eligible
    assert not fn(code, "multiply").immediate_jit_eligible


def test_compile_program_raises_on_lex_error() -> None:
    from tetrad_lexer import LexError

    with pytest.raises(LexError):
        cp("fn f() { let x = @; }")


def test_compile_program_raises_on_parse_error() -> None:
    from tetrad_parser import ParseError

    with pytest.raises(ParseError):
        cp("fn f( { }")


def test_group_expr_transparent() -> None:
    code = cp("fn f(a: u8) -> u8 { return (a); }")
    f = fn(code, "f")
    # (a) → LDA_VAR; just a load, no extra instructions
    assert any(i.opcode == Op.LDA_VAR for i in f.instructions)


# ---------------------------------------------------------------------------
# 19. Coverage gaps
# ---------------------------------------------------------------------------


def test_register_spill_raises() -> None:
    # Eight levels of binary nesting exhaust all 8 registers (line 169)
    # Each binary op uses 2 temp registers; deeply nested expressions spill

    import tetrad_compiler as _cm
    from tetrad_compiler.bytecode import CodeObject

    state = _cm._CompilerState(  # type: ignore[attr-defined]
        code=CodeObject(name="f", params=[]),
        locals={},
        type_map={},
        function_index={},
        all_functions=[],
    )
    # Exhaust all 8 registers by allocating without freeing
    for _ in range(8):
        _cm._alloc_reg(state)  # type: ignore[attr-defined]
    with pytest.raises(CompilerError, match="8 virtual registers"):
        _cm._alloc_reg(state)  # type: ignore[attr-defined]


def test_compile_expr_unknown_node_raises() -> None:
    # Unknown expression type hits the else-raise in _compile_expr (line 283)
    import tetrad_compiler as _cm
    from tetrad_compiler.bytecode import CodeObject

    state = _cm._CompilerState(  # type: ignore[attr-defined]
        code=CodeObject(name="f", params=[]),
        locals={},
        type_map={},
        function_index={},
        all_functions=[],
    )
    with pytest.raises(CompilerError, match="unknown expression type"):
        _cm._compile_expr("not_an_ast_node", state)  # type: ignore[attr-defined]


def test_add_imm_right_literal_overflow_raises() -> None:
    # ADD_IMM path with right literal > 255 (line 340)
    with pytest.raises(CompilerError, match="out of u8 range"):
        cp("fn f(a) { return a + 300; }")


def test_compile_stmt_bare_block() -> None:
    # Block as a direct statement in _compile_stmt (lines 493-494)
    from tetrad_parser.ast import Block, IntLiteral, LetStmt

    import tetrad_compiler as _cm
    from tetrad_compiler.bytecode import CodeObject

    state = _cm._CompilerState(  # type: ignore[attr-defined]
        code=CodeObject(name="f", params=[]),
        locals={},
        type_map={},
        function_index={},
        all_functions=[],
    )
    inner_block = Block(
        stmts=[
            LetStmt(
                name="z",
                declared_type=None,
                value=IntLiteral(value=7, line=1, column=1),
                line=1,
                column=1,
            )
        ],
        line=1,
        column=1,
    )
    _cm._compile_stmt(inner_block, state)  # type: ignore[attr-defined]
    assert state.code.var_names == ["z"]
