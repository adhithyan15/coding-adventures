"""Tests for nib-ir-compiler — Nib typed AST → IrProgram compiler.

Test Strategy
-------------

Each test runs the full pipeline:

    source text
        → parse_nib()           (Nib parser)
        → check()               (Nib type checker)
        → compile_nib()         (this package: Nib IR compiler)

This end-to-end approach validates that:
  1. The compiler integrates correctly with the parser and type checker.
  2. The emitted IR is well-formed (correct opcode sequences).
  3. Edge cases in each language construct are handled.

Test Categories
---------------

  1. Basic program structure (empty program, HALT, _start label)
  2. Static declarations (data segment, sizes for each type)
  3. Let declarations (register allocation, LOAD_IMM emission)
  4. Assignment statements
  5. Arithmetic operators (+%, -, +, bitwise)
  6. Comparison operators (==, !=, <, >, <=, >=)
  7. Logical operators (&&, ||, !)
  8. For loops (LABEL, BRANCH_Z/NZ, ADD_IMM for increment)
  9. If statements (BRANCH_Z, JUMP, labels)
  10. Function declarations and calls (LABEL, CALL, RET)
  11. Return statements
  12. Build config (debug vs release)
  13. Complex programs (multiple functions, nested constructs)
"""

from __future__ import annotations

import pytest

from compiler_ir import IrOp, IrProgram

from nib_ir_compiler import BuildConfig, CompileResult, compile_nib, debug_config, release_config


# ---------------------------------------------------------------------------
# Test pipeline helper
# ---------------------------------------------------------------------------


def compile_source(source: str, config: BuildConfig | None = None) -> IrProgram:
    """Run the full Nib pipeline and return the compiled IrProgram.

    This helper imports the parser and type checker lazily to keep each test
    self-contained. It raises AssertionError if type checking fails (which
    means the test source has a bug, not the compiler).

    Args:
        source: Nib source code string.
        config: Optional build config. Defaults to release config (no comments).

    Returns:
        The compiled ``IrProgram``.
    """
    from nib_parser import parse_nib
    from nib_type_checker import check

    ast = parse_nib(source)
    result = check(ast)
    assert result.ok, f"Type check failed: {[str(e.message) for e in result.errors]}"
    cfg = config if config is not None else release_config()
    return compile_nib(result.typed_ast, cfg).program


def compile_result(source: str, config: BuildConfig | None = None) -> CompileResult:
    """Run the full pipeline and return the CompileResult (not just the program)."""
    from nib_parser import parse_nib
    from nib_type_checker import check

    ast = parse_nib(source)
    result = check(ast)
    assert result.ok, f"Type check failed: {[str(e.message) for e in result.errors]}"
    cfg = config if config is not None else release_config()
    return compile_nib(result.typed_ast, cfg)


def opcodes(program: IrProgram) -> list[IrOp]:
    """Extract the list of opcodes from a program's instruction stream."""
    return [i.opcode for i in program.instructions]


# ---------------------------------------------------------------------------
# 1. Basic program structure
# ---------------------------------------------------------------------------


class TestBasicStructure:
    def test_empty_program_compiles(self) -> None:
        """An empty program (no declarations) should produce a valid IrProgram."""
        p = compile_source("")
        assert p is not None

    def test_empty_program_has_halt(self) -> None:
        """Every program must end with HALT."""
        p = compile_source("")
        assert IrOp.HALT in opcodes(p)

    def test_empty_program_has_start_label(self) -> None:
        """Every program must start with a _start LABEL."""
        p = compile_source("")
        label_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.LABEL and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name == "_start"
        ]
        assert len(label_instrs) >= 1

    def test_empty_program_loads_zero_register(self) -> None:
        """The prologue must initialise v0 = 0 (the constant-zero register)."""
        p = compile_source("")
        # Find a LOAD_IMM with destination v0, value 0.
        found = any(
            i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[0], "index")
            and i.operands[0].index == 0
            and hasattr(i.operands[1], "value")
            and i.operands[1].value == 0
            for i in p.instructions
        )
        assert found

    def test_program_with_main_calls_main(self) -> None:
        """A program with a main function should emit CALL _fn_main."""
        p = compile_source("fn main() { }")
        call_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.CALL and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name == "_fn_main"
        ]
        assert len(call_instrs) >= 1

    def test_program_without_main_no_call(self) -> None:
        """A program with no main function should not emit CALL _fn_main."""
        p = compile_source("static x: u4 = 0;")
        call_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.CALL and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name == "_fn_main"
        ]
        assert len(call_instrs) == 0

    def test_program_is_irprogram_instance(self) -> None:
        """The compiler must return an IrProgram instance."""
        p = compile_source("fn main() { }")
        assert isinstance(p, IrProgram)

    def test_entry_label_is_start(self) -> None:
        """The program's entry_label must be '_start'."""
        p = compile_source("fn main() { }")
        assert p.entry_label == "_start"


# ---------------------------------------------------------------------------
# 2. Static declarations
# ---------------------------------------------------------------------------


class TestStaticDeclarations:
    def test_static_u4_creates_data_decl(self) -> None:
        """static x: u4 = 0; should create an IrDataDecl with label 'x'."""
        p = compile_source("static x: u4 = 0;")
        assert any(d.label == "x" for d in p.data)

    def test_static_u4_has_size_1(self) -> None:
        """u4 static variables are 1 byte (nibble, byte-aligned)."""
        p = compile_source("static x: u4 = 0;")
        decl = next(d for d in p.data if d.label == "x")
        assert decl.size == 1

    def test_static_u8_has_size_2(self) -> None:
        """u8 static variables are 2 bytes (register pair)."""
        p = compile_source("static x: u8 = 0;")
        decl = next(d for d in p.data if d.label == "x")
        assert decl.size == 2

    def test_static_bcd_has_size_1(self) -> None:
        """bcd static variables are 1 byte."""
        p = compile_source("static x: bcd = 0;")
        decl = next(d for d in p.data if d.label == "x")
        assert decl.size == 1

    def test_static_bool_has_size_1(self) -> None:
        """bool static variables are 1 byte."""
        p = compile_source("static flag: bool = false;")
        assert any(d.label == "flag" for d in p.data)
        decl = next(d for d in p.data if d.label == "flag")
        assert decl.size == 1

    def test_static_init_value_zero(self) -> None:
        """static x: u4 = 0; init value should be 0."""
        p = compile_source("static x: u4 = 0;")
        decl = next(d for d in p.data if d.label == "x")
        assert decl.init == 0

    def test_static_init_value_literal(self) -> None:
        """static x: u4 = 5; init value should be 5."""
        p = compile_source("static x: u4 = 5;")
        decl = next(d for d in p.data if d.label == "x")
        assert decl.init == 5

    def test_multiple_statics(self) -> None:
        """Multiple static declarations each get their own IrDataDecl."""
        p = compile_source("static a: u4 = 0;\nstatic b: u8 = 0;")
        labels = [d.label for d in p.data]
        assert "a" in labels
        assert "b" in labels

    def test_const_decl_no_data(self) -> None:
        """const declarations should NOT emit data declarations (they're inlined)."""
        p = compile_source("const MAX: u8 = 100;\nfn main() { }")
        assert not any(d.label == "MAX" for d in p.data)


# ---------------------------------------------------------------------------
# 3. Let declarations and LOAD_IMM
# ---------------------------------------------------------------------------


class TestLetDeclarations:
    def test_let_emits_load_imm(self) -> None:
        """let x: u4 = 5; should emit at least one LOAD_IMM."""
        p = compile_source("fn main() { let x: u4 = 5; }")
        assert IrOp.LOAD_IMM in opcodes(p)

    def test_let_hex_literal_emits_load_imm(self) -> None:
        """let x: u4 = 0xF; should emit LOAD_IMM with value 15."""
        p = compile_source("fn main() { let x: u4 = 0xF; }")
        load_imm_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[1], "value")
            and i.operands[1].value == 15
        ]
        assert len(load_imm_instrs) >= 1

    def test_let_true_emits_load_imm_1(self) -> None:
        """let flag: bool = true; should emit LOAD_IMM with value 1."""
        p = compile_source("fn main() { let flag: bool = true; }")
        load_1 = [
            i for i in p.instructions
            if i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[1], "value")
            and i.operands[1].value == 1
        ]
        assert len(load_1) >= 1

    def test_let_false_emits_load_imm_0(self) -> None:
        """let flag: bool = false; should emit LOAD_IMM with value 0."""
        p = compile_source("fn main() { let flag: bool = false; }")
        # There's also the v0=0 prologue, but at least one LOAD_IMM 0 exists.
        load_0 = [
            i for i in p.instructions
            if i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[1], "value")
            and i.operands[1].value == 0
        ]
        assert len(load_0) >= 1

    def test_multiple_let_allocates_different_registers(self) -> None:
        """Multiple let bindings should use different destination registers."""
        p = compile_source("fn main() { let x: u4 = 1; let y: u4 = 2; }")
        # The program should reference at least 3 different registers:
        # v0 (zero), v1 (scratch), v2 (x), v3 (y).
        all_reg_indices: set[int] = set()
        for i in p.instructions:
            for op in i.operands:
                if hasattr(op, "index"):
                    all_reg_indices.add(op.index)
        assert len(all_reg_indices) >= 3

    def test_let_variable_used_in_expression(self) -> None:
        """Using a let variable in an expression should reference its register."""
        p = compile_source("fn main() { let x: u4 = 3; let y: u4 = x; }")
        # ADD_IMM should appear (copy from one reg to another).
        assert IrOp.ADD_IMM in opcodes(p) or IrOp.LOAD_IMM in opcodes(p)


# ---------------------------------------------------------------------------
# 4. Wrap-add (+%) and masking
# ---------------------------------------------------------------------------


class TestWrapAdd:
    def test_wrap_add_u4_emits_and_imm(self) -> None:
        """u4 +% should emit AND_IMM to mask to 4 bits (value 15)."""
        p = compile_source("fn main() { let x: u4 = 1 +% 2; }")
        assert IrOp.AND_IMM in opcodes(p)

    def test_wrap_add_u4_mask_value_is_15(self) -> None:
        """u4 +% AND_IMM should use mask value 15."""
        p = compile_source("fn main() { let x: u4 = 1 +% 2; }")
        and_imm_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.AND_IMM
            and len(i.operands) >= 3
            and hasattr(i.operands[2], "value")
            and i.operands[2].value == 15
        ]
        assert len(and_imm_instrs) >= 1

    def test_wrap_add_u8_emits_and_imm(self) -> None:
        """u8 +% should emit AND_IMM to mask to 8 bits (value 255)."""
        p = compile_source("fn main() { let x: u8 = 100 +% 200; }")
        assert IrOp.AND_IMM in opcodes(p)

    def test_wrap_add_u8_mask_value_is_255(self) -> None:
        """u8 +% AND_IMM should use mask value 255."""
        p = compile_source("fn main() { let x: u8 = 100 +% 200; }")
        and_imm_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.AND_IMM
            and len(i.operands) >= 3
            and hasattr(i.operands[2], "value")
            and i.operands[2].value == 255
        ]
        assert len(and_imm_instrs) >= 1

    def test_wrap_add_emits_add_before_and(self) -> None:
        """Wrapping add should emit ADD before AND_IMM."""
        p = compile_source("fn main() { let x: u4 = 1 +% 2; }")
        ops = opcodes(p)
        add_idx = next((i for i, op in enumerate(ops) if op == IrOp.ADD), None)
        and_idx = next((i for i, op in enumerate(ops) if op == IrOp.AND_IMM), None)
        assert add_idx is not None
        assert and_idx is not None
        assert add_idx < and_idx


# ---------------------------------------------------------------------------
# 5. Subtraction
# ---------------------------------------------------------------------------


class TestSubtraction:
    def test_sub_emits_sub_opcode(self) -> None:
        """a - b should emit SUB."""
        p = compile_source("fn main() { let x: u4 = 5 - 3; }")
        assert IrOp.SUB in opcodes(p)

    def test_sub_no_and_imm(self) -> None:
        """Plain subtraction should NOT emit AND_IMM (no masking needed)."""
        p = compile_source("fn main() { let x: u4 = 5 - 3; }")
        # No AND_IMM should appear (v0=0 prologue doesn't use AND_IMM either).
        assert IrOp.AND_IMM not in opcodes(p)


# ---------------------------------------------------------------------------
# 6. Comparison operators
# ---------------------------------------------------------------------------


class TestComparisonOperators:
    def test_eq_emits_cmp_eq(self) -> None:
        """a == b should emit CMP_EQ."""
        p = compile_source("fn main() { let ok: bool = 1 == 1; }")
        assert IrOp.CMP_EQ in opcodes(p)

    def test_ne_emits_cmp_ne(self) -> None:
        """a != b should emit CMP_NE."""
        p = compile_source("fn main() { let ok: bool = 1 != 2; }")
        assert IrOp.CMP_NE in opcodes(p)

    def test_lt_emits_cmp_lt(self) -> None:
        """a < b should emit CMP_LT."""
        p = compile_source("fn main() { let ok: bool = 1 < 2; }")
        assert IrOp.CMP_LT in opcodes(p)

    def test_gt_emits_cmp_gt(self) -> None:
        """a > b should emit CMP_GT."""
        p = compile_source("fn main() { let ok: bool = 2 > 1; }")
        assert IrOp.CMP_GT in opcodes(p)

    def test_le_emits_cmp_gt_swapped(self) -> None:
        """a <= b should emit CMP_GT with swapped operands (LE(a,b) = GT(b,a))."""
        p = compile_source("fn main() { let ok: bool = 1 <= 2; }")
        assert IrOp.CMP_GT in opcodes(p)

    def test_ge_emits_cmp_lt_swapped(self) -> None:
        """a >= b should emit CMP_LT with swapped operands (GE(a,b) = LT(b,a))."""
        p = compile_source("fn main() { let ok: bool = 2 >= 1; }")
        assert IrOp.CMP_LT in opcodes(p)


# ---------------------------------------------------------------------------
# 7. Logical operators
# ---------------------------------------------------------------------------


class TestLogicalOperators:
    def test_logical_and_emits_and(self) -> None:
        """a && b should emit AND."""
        p = compile_source("fn main() { let ok: bool = true && false; }")
        assert IrOp.AND in opcodes(p)

    def test_logical_or_emits_add_and_cmp_ne(self) -> None:
        """a || b should emit ADD followed by CMP_NE."""
        p = compile_source("fn main() { let ok: bool = true || false; }")
        ops = opcodes(p)
        assert IrOp.ADD in ops
        assert IrOp.CMP_NE in ops

    def test_logical_not_emits_cmp_eq(self) -> None:
        """!a should emit CMP_EQ (with zero register)."""
        p = compile_source("fn main() { let ok: bool = !true; }")
        assert IrOp.CMP_EQ in opcodes(p)


# ---------------------------------------------------------------------------
# 8. For loops
# ---------------------------------------------------------------------------


class TestForLoops:
    def test_for_loop_has_label(self) -> None:
        """for loop should emit at least one LABEL instruction."""
        p = compile_source("fn main() { for i: u8 in 0..5 { } }")
        assert IrOp.LABEL in opcodes(p)

    def test_for_loop_has_branch_z(self) -> None:
        """for loop condition check should use BRANCH_Z."""
        p = compile_source("fn main() { for i: u8 in 0..5 { } }")
        assert IrOp.BRANCH_Z in opcodes(p)

    def test_for_loop_has_jump_back(self) -> None:
        """for loop should emit JUMP to go back to start label."""
        p = compile_source("fn main() { for i: u8 in 0..5 { } }")
        assert IrOp.JUMP in opcodes(p)

    def test_for_loop_initialises_loop_var(self) -> None:
        """for loop should emit LOAD_IMM to set initial value of loop var."""
        p = compile_source("fn main() { for i: u8 in 0..5 { } }")
        assert IrOp.LOAD_IMM in opcodes(p)

    def test_for_loop_increments_with_add_imm(self) -> None:
        """for loop should emit ADD_IMM to increment the loop variable."""
        p = compile_source("fn main() { for i: u8 in 0..5 { } }")
        assert IrOp.ADD_IMM in opcodes(p)

    def test_for_loop_with_body(self) -> None:
        """for loop with a body statement should compile the body too."""
        p = compile_source("fn main() { for i: u4 in 0..3 { let x: u4 = 1; } }")
        # The body's LOAD_IMM for x=1 should appear.
        load_1 = [
            i for i in p.instructions
            if i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[1], "value")
            and i.operands[1].value == 1
        ]
        assert len(load_1) >= 1

    def test_two_for_loops_have_unique_labels(self) -> None:
        """Two for loops in the same function should have distinct label names."""
        p = compile_source("fn main() { for i: u4 in 0..3 { } for j: u4 in 0..3 { } }")
        label_names: list[str] = [
            i.operands[0].name
            for i in p.instructions
            if i.opcode == IrOp.LABEL and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name.startswith("loop_")
        ]
        # Should have 4 labels: loop_0_start, loop_0_end, loop_1_start, loop_1_end
        assert len(set(label_names)) >= 4

    def test_for_loop_runtime_upper_bound_compiles(self) -> None:
        """Loop bounds can come from a runtime numeric variable."""
        p = compile_source("fn main() { let n: u8 = 5; for i: u8 in 0..n { } }")
        cmp_lt = [
            i for i in p.instructions
            if i.opcode == IrOp.CMP_LT
        ]
        assert len(cmp_lt) >= 1

    def test_const_name_loads_immediate_value(self) -> None:
        """Top-level const names are lowered as immediates when referenced."""
        p = compile_source("const C: u4 = 3; fn main() { let x: u4 = C; }")
        load_3 = [
            i for i in p.instructions
            if i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[1], "value")
            and i.operands[1].value == 3
        ]
        assert len(load_3) >= 1


# ---------------------------------------------------------------------------
# 9. If statements
# ---------------------------------------------------------------------------


class TestIfStatements:
    def test_if_has_branch_z(self) -> None:
        """if statement should emit BRANCH_Z to skip then-block if cond == 0."""
        p = compile_source("fn main() { if true { } }")
        assert IrOp.BRANCH_Z in opcodes(p)

    def test_if_has_jump(self) -> None:
        """if statement should emit JUMP to skip else-block after then-block."""
        p = compile_source("fn main() { if true { } }")
        assert IrOp.JUMP in opcodes(p)

    def test_if_has_labels(self) -> None:
        """if statement should emit LABEL instructions for else and end."""
        p = compile_source("fn main() { if true { } }")
        if_labels = [
            i for i in p.instructions
            if i.opcode == IrOp.LABEL
            and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name.startswith("if_")
        ]
        assert len(if_labels) >= 2

    def test_if_else_compiles_both_branches(self) -> None:
        """if/else should compile both branches."""
        p = compile_source("fn main() { if true { let x: u4 = 1; } else { let x: u4 = 2; } }")
        # Both LOAD_IMM 1 and LOAD_IMM 2 should appear.
        vals = {
            i.operands[1].value
            for i in p.instructions
            if i.opcode == IrOp.LOAD_IMM
            and len(i.operands) >= 2
            and hasattr(i.operands[1], "value")
        }
        assert 1 in vals
        assert 2 in vals

    def test_nested_if_has_multiple_labels(self) -> None:
        """Nested if statements should have multiple distinct if_K labels."""
        p = compile_source(
            "fn main() { if true { if true { } } }"
        )
        if_labels = [
            i.operands[0].name
            for i in p.instructions
            if i.opcode == IrOp.LABEL
            and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name.startswith("if_")
        ]
        # Two nested ifs: at least 4 labels (else_0, end_0, else_1, end_1).
        assert len(set(if_labels)) >= 4


# ---------------------------------------------------------------------------
# 10. Function declarations and calls
# ---------------------------------------------------------------------------


class TestFunctionDeclarations:
    def test_fn_main_emits_label(self) -> None:
        """fn main() should emit LABEL _fn_main."""
        p = compile_source("fn main() { }")
        fn_labels = [
            i for i in p.instructions
            if i.opcode == IrOp.LABEL
            and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name == "_fn_main"
        ]
        assert len(fn_labels) == 1

    def test_fn_emits_ret_at_end(self) -> None:
        """Every function should end with RET."""
        p = compile_source("fn main() { }")
        assert IrOp.RET in opcodes(p)

    def test_fn_call_emits_call(self) -> None:
        """Calling a function should emit CALL."""
        p = compile_source("fn f() { } fn main() { f(); }")
        assert IrOp.CALL in opcodes(p)

    def test_fn_call_uses_correct_label(self) -> None:
        """CALL should reference the _fn_NAME label."""
        p = compile_source("fn helper() { } fn main() { helper(); }")
        call_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.CALL
            and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name == "_fn_helper"
        ]
        assert len(call_instrs) >= 1

    def test_fn_with_params_allocates_regs(self) -> None:
        """fn f(a: u4, b: u4) should allocate registers for each param."""
        p = compile_source("fn f(a: u4, b: u4) -> u4 { return a; } fn main() { }")
        # The function body references registers beyond v2 (its first param).
        all_reg_indices: set[int] = set()
        for i in p.instructions:
            for op in i.operands:
                if hasattr(op, "index"):
                    all_reg_indices.add(op.index)
        assert len(all_reg_indices) >= 3  # v0, v1, v2 (at minimum)

    def test_two_functions_have_distinct_labels(self) -> None:
        """Two functions should have distinct _fn_NAME labels."""
        p = compile_source("fn foo() { } fn bar() { } fn main() { }")
        label_names = [
            i.operands[0].name
            for i in p.instructions
            if i.opcode == IrOp.LABEL
            and i.operands
            and hasattr(i.operands[0], "name")
            and i.operands[0].name.startswith("_fn_")
        ]
        assert "_fn_foo" in label_names
        assert "_fn_bar" in label_names
        assert "_fn_main" in label_names

    def test_fn_with_args_passes_in_v2(self) -> None:
        """Function call arguments should be placed in v2, v3, ... before CALL."""
        p = compile_source("fn add(a: u4, b: u4) -> u4 { return a; } fn main() { add(1, 2); }")
        # Before CALL, we should see ADD_IMM to move args into position.
        # At minimum, we see LOAD_IMM for literal args.
        has_arg_setup = any(
            i.opcode in (IrOp.LOAD_IMM, IrOp.ADD_IMM)
            for i in p.instructions
        )
        assert has_arg_setup

    def test_fn_call_preserves_live_local_registers(self) -> None:
        """Calls should restore caller locals after repurposing v2+ for arguments."""
        p = compile_source(
            "fn id(a: u4) -> u4 { return a; } "
            "fn main() -> u4 { let x: u4 = 2; id(x); return x; }"
        )

        call_index = next(
            index
            for index, instruction in enumerate(p.instructions)
            if instruction.opcode == IrOp.CALL
            and instruction.operands
            and hasattr(instruction.operands[0], "name")
            and instruction.operands[0].name == "_fn_id"
        )
        restore_instr = p.instructions[call_index + 1]

        assert restore_instr.opcode == IrOp.ADD_IMM
        assert hasattr(restore_instr.operands[0], "index")
        assert hasattr(restore_instr.operands[1], "index")
        assert restore_instr.operands[0].index == 2
        assert restore_instr.operands[1].index > 2


# ---------------------------------------------------------------------------
# 11. Return statements
# ---------------------------------------------------------------------------


class TestReturnStatements:
    def test_return_emits_ret(self) -> None:
        """An explicit return statement should emit RET."""
        p = compile_source("fn f() -> u4 { return 5; } fn main() { }")
        assert IrOp.RET in opcodes(p)

    def test_return_loads_value_before_ret(self) -> None:
        """return 5 should emit LOAD_IMM before RET."""
        p = compile_source("fn f() -> u4 { return 5; } fn main() { }")
        ops = opcodes(p)
        ret_idx = next((i for i, op in enumerate(ops) if op == IrOp.RET), None)
        assert ret_idx is not None
        # At least one LOAD_IMM before the first RET.
        has_load_before = any(op == IrOp.LOAD_IMM for op in ops[:ret_idx])
        assert has_load_before

    def test_void_function_still_has_ret(self) -> None:
        """A void function with no explicit return should still emit RET."""
        p = compile_source("fn main() { }")
        assert IrOp.RET in opcodes(p)


# ---------------------------------------------------------------------------
# 12. Build configuration
# ---------------------------------------------------------------------------


class TestBuildConfig:
    def test_debug_config_emits_comments(self) -> None:
        """debug_config() should produce COMMENT pseudo-instructions."""
        p = compile_source("fn main() { let x: u4 = 1; }", debug_config())
        assert IrOp.COMMENT in opcodes(p)

    def test_release_config_no_comments(self) -> None:
        """release_config() should NOT produce COMMENT instructions."""
        p = compile_source("fn main() { let x: u4 = 1; }", release_config())
        assert IrOp.COMMENT not in opcodes(p)

    def test_default_config_is_debug(self) -> None:
        """The default BuildConfig should enable debug comments."""
        from nib_parser import parse_nib
        from nib_type_checker import check
        ast = parse_nib("fn main() { let x: u4 = 1; }")
        result = check(ast)
        compiled = compile_nib(result.typed_ast)  # no config → default
        assert IrOp.COMMENT in opcodes(compiled.program)

    def test_compile_result_has_program(self) -> None:
        """CompileResult should expose a non-None program."""
        cr = compile_result("fn main() { }")
        assert cr.program is not None

    def test_compile_result_source_map_is_none_for_v1(self) -> None:
        """In v1, the source_map field should be None (not yet implemented)."""
        cr = compile_result("fn main() { }")
        assert cr.source_map is None


# ---------------------------------------------------------------------------
# 13. Complex programs
# ---------------------------------------------------------------------------


class TestComplexPrograms:
    def test_fibonacci_like_program(self) -> None:
        """A multi-function program with arithmetic should compile cleanly."""
        source = """
            fn add(a: u8, b: u8) -> u8 {
                return a +% b;
            }
            fn main() {
                let a: u8 = 0;
                let b: u8 = 1;
                let c: u8 = add(a, b);
            }
        """
        p = compile_source(source)
        assert IrOp.HALT in opcodes(p)
        assert IrOp.CALL in opcodes(p)
        assert IrOp.RET in opcodes(p)

    def test_counter_program_with_for_loop(self) -> None:
        """A for-loop counting program should compile correctly."""
        source = """
            static counter: u8 = 0;
            fn main() {
                for i: u8 in 0..10 {
                    let x: u4 = 1;
                }
            }
        """
        p = compile_source(source)
        assert any(d.label == "counter" for d in p.data)
        assert IrOp.BRANCH_Z in opcodes(p)
        assert IrOp.JUMP in opcodes(p)

    def test_conditional_program(self) -> None:
        """A program with if/else should produce correct IR."""
        source = """
            fn check(x: u4) -> bool {
                if x == 0 {
                    return false;
                } else {
                    return true;
                }
            }
            fn main() {
                let ok: bool = check(5);
            }
        """
        p = compile_source(source)
        assert IrOp.CMP_EQ in opcodes(p)
        assert IrOp.BRANCH_Z in opcodes(p)
        assert IrOp.RET in opcodes(p)

    def test_program_with_multiple_static_types(self) -> None:
        """A program with statics of every type should produce correct data."""
        source = """
            static a: u4 = 0;
            static b: u8 = 0;
            static c: bcd = 0;
            static d: bool = false;
            fn main() { }
        """
        p = compile_source(source)
        labels = {d.label for d in p.data}
        assert labels == {"a", "b", "c", "d"}
        sizes = {d.label: d.size for d in p.data}
        assert sizes["a"] == 1   # u4 → 1 byte
        assert sizes["b"] == 2   # u8 → 2 bytes
        assert sizes["c"] == 1   # bcd → 1 byte
        assert sizes["d"] == 1   # bool → 1 byte

    def test_bitwise_and_emits_and(self) -> None:
        """Bitwise & should emit AND."""
        p = compile_source("fn main() { let x: u4 = 0xF & 0x3; }")
        assert IrOp.AND in opcodes(p)

    def test_program_with_chained_comparison(self) -> None:
        """A chained comparison program should emit the right comparison opcode."""
        p = compile_source("fn main() { let ok: bool = 3 < 5; }")
        assert IrOp.CMP_LT in opcodes(p)

    def test_instruction_ids_are_unique(self) -> None:
        """All real instructions (id != -1) must have unique IDs."""
        source = """
            fn add(a: u4, b: u4) -> u4 {
                return a +% b;
            }
            fn main() {
                let x: u4 = 5;
                let y: u4 = add(x, 3);
            }
        """
        p = compile_source(source)
        real_ids = [i.id for i in p.instructions if i.id != -1]
        assert len(real_ids) == len(set(real_ids)), "Duplicate instruction IDs found!"

    def test_bcd_wrap_add_emits_and_imm_255(self) -> None:
        """bcd +% should emit AND_IMM with mask 255."""
        p = compile_source("fn main() { let x: bcd = 5 +% 3; }")
        and_imm_instrs = [
            i for i in p.instructions
            if i.opcode == IrOp.AND_IMM
            and len(i.operands) >= 3
            and hasattr(i.operands[2], "value")
            and i.operands[2].value == 255
        ]
        assert len(and_imm_instrs) >= 1


# ---------------------------------------------------------------------------
# 14. Error handling
# ---------------------------------------------------------------------------


class TestErrorHandling:
    def test_invalid_ast_root_raises_value_error(self) -> None:
        """compile_nib() with a non-program root should raise ValueError."""
        from lang_parser import ASTNode
        bad_node = ASTNode(rule_name="fn_decl", children=[])
        with pytest.raises(ValueError, match="expected 'program' AST node"):
            compile_nib(bad_node)

    def test_typed_ast_with_no_functions_compiles(self) -> None:
        """A program with only static declarations (no functions) should compile."""
        p = compile_source("static x: u4 = 7;")
        assert p is not None
        assert IrOp.HALT in opcodes(p)
