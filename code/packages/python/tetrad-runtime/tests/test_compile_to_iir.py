"""Tests for the Tetrad-bytecode → IIR translator.

These tests assert on the *shape* of the translated IIR rather than its
execution result — the runtime-level tests in ``test_runtime.py`` cover
behaviour.  Splitting them this way keeps regressions in the translator
diagnosable separately from regressions in vm-core.
"""

from __future__ import annotations

import pytest
from interpreter_ir import IIRFunction, IIRModule
from interpreter_ir.function import FunctionTypeStatus

from tetrad_runtime import code_object_to_iir, compile_to_iir

# ---------------------------------------------------------------------------
# Shape assertions
# ---------------------------------------------------------------------------


def test_module_has_synthetic_entry_point() -> None:
    """The module entry is a synthetic ``__entry__`` wrapper that runs
    global initialisers and (if present) calls user-defined ``main``."""
    from tetrad_runtime.iir_translator import ENTRY_FN_NAME

    module = compile_to_iir("fn main() -> u8 { return 0; }")
    assert isinstance(module, IIRModule)
    assert module.entry_point == ENTRY_FN_NAME
    assert module.language == "tetrad"
    # User's ``main`` is still in the module — the entry wrapper calls it.
    assert module.get_function("main") is not None
    assert module.get_function(ENTRY_FN_NAME) is not None


def test_user_functions_appear_alongside_main() -> None:
    module = compile_to_iir(
        "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
        "fn main() -> u8 { return add(1, 2); }"
    )
    names = module.function_names()
    assert "main" in names
    assert "add" in names


def test_iir_module_validates_clean() -> None:
    module = compile_to_iir(
        "fn loop_to(n: u8) -> u8 {\n"
        "  let i = 0;\n"
        "  while i < n { i = i + 1; }\n"
        "  return i;\n"
        "}\n"
        "fn main() -> u8 { return loop_to(5); }"
    )
    assert module.validate() == []


def test_fully_typed_status_propagates() -> None:
    module = compile_to_iir(
        "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
        "fn main() -> u8 { return add(3, 4); }"
    )
    add_fn = module.get_function("add")
    assert add_fn is not None
    assert add_fn.type_status == FunctionTypeStatus.FULLY_TYPED


def test_branch_targets_become_labels() -> None:
    """Every branch destination becomes a ``label`` instruction in the IIR."""
    module = compile_to_iir(
        "fn loop_to(n: u8) -> u8 {\n"
        "  let i = 0;\n"
        "  while i < n { i = i + 1; }\n"
        "  return i;\n"
        "}\n"
        "fn main() -> u8 { return loop_to(3); }"
    )
    loop_fn = module.get_function("loop_to")
    assert loop_fn is not None
    label_ops = [i for i in loop_fn.instructions if i.op == "label"]
    # While-loop: back-edge target + exit target = 2 labels.
    assert len(label_ops) >= 2
    # Every jump references a label that exists.
    label_names = {str(i.srcs[0]) for i in label_ops}
    for instr in loop_fn.instructions:
        if instr.op in ("jmp", "jmp_if_true", "jmp_if_false"):
            assert str(instr.srcs[-1]) in label_names


def test_io_translates_to_builtins() -> None:
    """``in()`` and ``out(v)`` lower to ``call_builtin`` of ``__io_in``/``__io_out``."""
    module = compile_to_iir(
        "fn echo(v: u8) -> u8 { return v; }\n"
        "fn main() -> u8 {\n"
        "  out(7);\n"
        "  return echo(in());\n"
        "}"
    )
    main = module.get_function("main")
    assert main is not None
    builtin_ops = [i for i in main.instructions if i.op == "call_builtin"]
    builtin_names = {str(i.srcs[0]) for i in builtin_ops}
    assert "__io_in" in builtin_names
    assert "__io_out" in builtin_names


def test_globals_translate_to_store_mem() -> None:
    """Top-level globals lower to ``store_mem`` at addresses 0..N-1."""
    from tetrad_runtime.iir_translator import ENTRY_FN_NAME
    module = compile_to_iir(
        "let counter: u8 = 5;\n"
        "fn main() -> u8 { return 0; }"
    )
    entry = module.get_function(ENTRY_FN_NAME)
    assert entry is not None
    store_ops = [i for i in entry.instructions if i.op == "store_mem"]
    assert len(store_ops) == 1
    # The first global gets address 0.
    assert store_ops[0].srcs[0] == 0
    # The module's tetrad_globals attribute names what's at each address.
    assert getattr(module, "tetrad_globals", []) == ["counter"]


def test_locals_translate_to_tetrad_move() -> None:
    module = compile_to_iir(
        "fn doubler(x: u8) -> u8 {\n"
        "  let y = x;\n"
        "  return y + y;\n"
        "}\n"
        "fn main() -> u8 { return doubler(7); }"
    )
    doubler = module.get_function("doubler")
    assert doubler is not None
    move_ops = [i for i in doubler.instructions if i.op == "tetrad.move"]
    # At minimum: param load + let store + var read or two = several moves.
    assert len(move_ops) >= 2


def test_call_marshals_args_through_named_register_slots() -> None:
    """The CALL emission lists the callee name then named ``_r{i}`` slots."""
    module = compile_to_iir(
        "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
        "fn main() -> u8 { return add(10, 20); }"
    )
    main = module.get_function("main")
    assert main is not None
    call_ops = [i for i in main.instructions if i.op == "call"]
    assert len(call_ops) == 1
    call = call_ops[0]
    assert call.srcs[0] == "add"
    # Two args → two register-slot names.
    assert call.srcs[1:3] == ["_r0", "_r1"]


def test_translate_existing_code_object() -> None:
    """``code_object_to_iir`` should accept a pre-built CodeObject too."""
    from tetrad_compiler import compile_program

    from tetrad_runtime.iir_translator import ENTRY_FN_NAME
    code = compile_program("fn main() -> u8 { return 42; }")
    module = code_object_to_iir(code, module_name="explicit-name")
    assert module.name == "explicit-name"
    assert module.entry_point == ENTRY_FN_NAME


def test_unknown_opcode_raises() -> None:
    """An unrecognised opcode in the input raises a clear error."""
    from tetrad_compiler.bytecode import CodeObject, Instruction
    bad_code = CodeObject(
        name="<main>",
        params=[],
        instructions=[Instruction(0xCD, [])],
    )
    with pytest.raises(ValueError, match=r"no IIR translation"):
        code_object_to_iir(bad_code)


# ---------------------------------------------------------------------------
# Per-opcode coverage spot checks
# ---------------------------------------------------------------------------


def _function_named(module: IIRModule, name: str) -> IIRFunction:
    fn = module.get_function(name)
    assert fn is not None, f"function {name!r} not in module"
    return fn


def test_arithmetic_opcodes_use_standard_iir_mnemonics() -> None:
    module = compile_to_iir(
        "fn arith(a: u8, b: u8) -> u8 {\n"
        "  return (a + b) * (a - b);\n"
        "}\n"
        "fn main() -> u8 { return arith(7, 3); }"
    )
    arith = _function_named(module, "arith")
    ops = {i.op for i in arith.instructions}
    assert "add" in ops
    assert "sub" in ops
    assert "mul" in ops


def test_comparison_emits_cast_back_to_u8() -> None:
    module = compile_to_iir(
        "fn cmp(a: u8, b: u8) -> u8 {\n"
        "  return a < b;\n"
        "}\n"
        "fn main() -> u8 { return cmp(1, 2); }"
    )
    cmp_fn = _function_named(module, "cmp")
    ops = [i.op for i in cmp_fn.instructions]
    assert "cmp_lt" in ops
    assert "cast" in ops
