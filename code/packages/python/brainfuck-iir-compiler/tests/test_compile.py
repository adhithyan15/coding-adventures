"""IIR-shape tests for ``brainfuck-iir-compiler``.

These tests check structural properties of the compiled IR — they do
*not* execute it.  Execution coverage lives in :mod:`test_execute`.
"""

from __future__ import annotations

import pytest
from brainfuck import parse_brainfuck
from interpreter_ir import (
    CONCRETE_TYPES,
    DYNAMIC_TYPE,
    FunctionTypeStatus,
    IIRModule,
)

from brainfuck_iir_compiler import compile_source, compile_to_iir

# ---------------------------------------------------------------------------
# Module shape
# ---------------------------------------------------------------------------


def test_empty_program_is_valid() -> None:
    module = compile_source("")
    assert isinstance(module, IIRModule)
    assert module.entry_point == "main"
    assert module.language == "brainfuck"
    assert len(module.functions) == 1


def test_module_has_single_main_function() -> None:
    module = compile_source("+.")
    (fn,) = module.functions
    assert fn.name == "main"
    assert fn.params == []
    assert fn.return_type == "void"


def test_function_is_fully_typed() -> None:
    module = compile_source("++[>+<-]")
    (fn,) = module.functions
    assert fn.type_status == FunctionTypeStatus.FULLY_TYPED


def test_no_instruction_uses_dynamic_type_hint() -> None:
    module = compile_source("++[>+<-].")
    (fn,) = module.functions
    for instr in fn.instructions:
        assert instr.type_hint != DYNAMIC_TYPE, (
            f"instruction {instr.op!r} left as 'any' — fully-typed contract broken"
        )


def test_only_concrete_or_void_type_hints_used() -> None:
    module = compile_source(",.++>-<[+]")
    (fn,) = module.functions
    allowed = set(CONCRETE_TYPES) | {"void"}
    for instr in fn.instructions:
        assert instr.type_hint in allowed, (
            f"instruction {instr.op!r} carries unexpected type_hint "
            f"{instr.type_hint!r}"
        )


# ---------------------------------------------------------------------------
# Prologue / epilogue
# ---------------------------------------------------------------------------


def test_prologue_initialises_pointer_to_zero() -> None:
    module = compile_source("")
    (fn,) = module.functions
    first = fn.instructions[0]
    assert first.op == "const"
    assert first.dest == "ptr"
    assert first.srcs == [0]
    assert first.type_hint == "u32"


def test_epilogue_is_ret_void() -> None:
    module = compile_source("+")
    (fn,) = module.functions
    last = fn.instructions[-1]
    assert last.op == "ret_void"
    assert last.dest is None


# ---------------------------------------------------------------------------
# Per-command IR shape
# ---------------------------------------------------------------------------


def test_pointer_right_emits_const_then_add() -> None:
    module = compile_source(">")
    (fn,) = module.functions
    # prologue ptr0=0, then const k1=1, add ptr1=ptr0+k1, ret_void
    ops = [i.op for i in fn.instructions]
    assert ops == ["const", "const", "add", "ret_void"]


def test_pointer_left_emits_const_then_sub() -> None:
    module = compile_source("<")
    (fn,) = module.functions
    ops = [i.op for i in fn.instructions]
    assert ops == ["const", "const", "sub", "ret_void"]


def test_inc_emits_load_const_add_store() -> None:
    module = compile_source("+")
    (fn,) = module.functions
    ops = [i.op for i in fn.instructions[1:-1]]  # strip prologue/epilogue
    assert ops == ["load_mem", "const", "add", "store_mem"]


def test_dec_emits_load_const_sub_store() -> None:
    module = compile_source("-")
    (fn,) = module.functions
    ops = [i.op for i in fn.instructions[1:-1]]
    assert ops == ["load_mem", "const", "sub", "store_mem"]


def test_output_emits_load_then_putchar_call() -> None:
    module = compile_source(".")
    (fn,) = module.functions
    body = fn.instructions[1:-1]
    assert [i.op for i in body] == ["load_mem", "call_builtin"]
    call = body[1]
    assert call.srcs[0] == "putchar"


def test_input_emits_getchar_call_then_store() -> None:
    module = compile_source(",")
    (fn,) = module.functions
    body = fn.instructions[1:-1]
    assert [i.op for i in body] == ["call_builtin", "store_mem"]
    call = body[0]
    assert call.srcs == ["getchar"]


# ---------------------------------------------------------------------------
# Loop shape
# ---------------------------------------------------------------------------


def test_loop_emits_label_branch_label() -> None:
    module = compile_source("[+]")
    (fn,) = module.functions
    body = fn.instructions[1:-1]  # strip prologue/epilogue
    ops = [i.op for i in body]
    # label, load_mem, jmp_if_false, [body for +], jmp, label
    # Unconditional back-edge (BF05) — required for ir-to-wasm-compiler
    # to recognise the loop as structured.
    assert ops[0] == "label"
    assert ops[1] == "load_mem"
    assert ops[2] == "jmp_if_false"
    assert ops[-1] == "label"
    assert ops[-2] == "jmp"


def test_nested_loops_get_unique_labels() -> None:
    module = compile_source("[[+]]")
    (fn,) = module.functions
    labels = [i.srcs[0] for i in fn.instructions if i.op == "label"]
    assert len(labels) == len(set(labels)), f"duplicate labels: {labels}"
    # Should be 2 distinct loops × 2 labels each = 4 total
    assert len(labels) == 4


def test_loop_jump_targets_are_consistent() -> None:
    module = compile_source("[+]")
    (fn,) = module.functions
    label_names = {i.srcs[0] for i in fn.instructions if i.op == "label"}
    # Conditional branches store the target label at srcs[1]; unconditional
    # ``jmp`` stores it at srcs[0].  Collect both forms.
    jump_targets: set[str] = set()
    for i in fn.instructions:
        if i.op in {"jmp_if_true", "jmp_if_false"}:
            jump_targets.add(i.srcs[1])
        elif i.op == "jmp":
            jump_targets.add(i.srcs[0])
    assert jump_targets <= label_names, (
        f"jump targets {jump_targets - label_names} have no matching label"
    )


# ---------------------------------------------------------------------------
# AST entry point
# ---------------------------------------------------------------------------


def test_compile_to_iir_accepts_pre_parsed_ast() -> None:
    ast = parse_brainfuck("++.")
    module = compile_to_iir(ast)
    assert isinstance(module, IIRModule)
    assert module.functions[0].name == "main"


def test_compile_to_iir_respects_module_name() -> None:
    ast = parse_brainfuck("+")
    module = compile_to_iir(ast, module_name="hello.bf")
    assert module.name == "hello.bf"


# ---------------------------------------------------------------------------
# Register file usage
# ---------------------------------------------------------------------------


def test_compiler_uses_small_fixed_register_set() -> None:
    """Brainfuck compiles to a fixed handful of named registers.

    SSA-style fresh names would break loops (defined inside a skipped
    body, then read after the body), and Brainfuck doesn't need fresh
    names anyway — the four fixed registers cover the whole language.
    """
    module = compile_source("++[>+<-].")
    (fn,) = module.functions
    dests = {i.dest for i in fn.instructions if i.dest is not None}
    # Should be a tiny set: pointer, value, condition, immediate.
    assert dests <= {"ptr", "v", "c", "k"}, (
        f"unexpected register names in compiled IIR: {dests}"
    )


def test_register_count_is_bounded() -> None:
    # Even a sprawling program should not need more than a handful of
    # registers because every command reuses the same four names.
    module = compile_source("+++[->+++<]>." * 10)
    (fn,) = module.functions
    assert fn.register_count <= 16, (
        f"register file unexpectedly large: {fn.register_count}"
    )


# ---------------------------------------------------------------------------
# Defensive errors
# ---------------------------------------------------------------------------


def test_unmatched_open_bracket_raises_at_parse_time() -> None:
    with pytest.raises(Exception):  # noqa: B017 — parser raises a generic exception on bracket mismatch
        compile_source("[+")


def test_unmatched_close_bracket_raises_at_parse_time() -> None:
    with pytest.raises(Exception):  # noqa: B017 — parser raises a generic exception on bracket mismatch
        compile_source("+]")


# ---------------------------------------------------------------------------
# Compiler-internal defensive paths
# ---------------------------------------------------------------------------


def test_unexpected_ast_rule_raises() -> None:
    """Synthetic ASTNode with an unknown rule name must fail loudly.

    The grammar guarantees only ``program`` / ``instruction`` / ``loop`` /
    ``command`` rules in real input.  Defensive code in the compiler
    rejects anything else so a future grammar tweak doesn't silently
    drop instructions.
    """
    from lang_parser import ASTNode

    from brainfuck_iir_compiler.compiler import _Compiler

    bogus = ASTNode(rule_name="bogus", children=[])
    with pytest.raises(ValueError, match="unexpected AST rule"):
        _Compiler()._emit_node(bogus)


def test_command_with_no_token_raises() -> None:
    """A ``command`` AST with no token children is malformed input.

    The grammar should never produce one, but the compiler validates
    that assumption rather than crashing further down the pipeline.
    """
    from lang_parser import ASTNode

    from brainfuck_iir_compiler.compiler import _Compiler

    empty_command = ASTNode(rule_name="command", children=[])
    with pytest.raises(ValueError, match="no token"):
        _Compiler()._emit_command(empty_command)


def test_unknown_command_token_raises() -> None:
    """A token whose value isn't one of the eight commands must be rejected."""
    from lang_parser import ASTNode
    from lexer import Token

    from brainfuck_iir_compiler.compiler import _Compiler

    bad_token = Token(type="BOGUS", value="@", line=1, column=1)
    bad_command = ASTNode(rule_name="command", children=[bad_token])
    with pytest.raises(ValueError, match="unknown brainfuck command"):
        _Compiler()._emit_command(bad_command)


def test_first_token_walks_through_nested_nodes() -> None:
    """`_first_token` must descend through nested ASTNode wrappers.

    The grammar can wrap a single command token in several layers of
    ``instruction`` / ``command`` nodes.  The helper must dig through
    them rather than returning None for non-leaf nodes.
    """
    from lang_parser import ASTNode
    from lexer import Token

    from brainfuck_iir_compiler.compiler import _first_token

    inner_tok = Token(type="INC", value="+", line=1, column=1)
    inner = ASTNode(rule_name="command", children=[inner_tok])
    outer = ASTNode(rule_name="instruction", children=[inner])
    assert _first_token(outer) is inner_tok


def test_first_token_returns_none_for_empty_node() -> None:
    """Empty AST nodes legitimately have no descendant token."""
    from lang_parser import ASTNode

    from brainfuck_iir_compiler.compiler import _first_token

    empty = ASTNode(rule_name="program", children=[])
    assert _first_token(empty) is None
