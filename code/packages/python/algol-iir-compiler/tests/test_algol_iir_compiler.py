from __future__ import annotations

import pytest
from algol_parser import parse_algol
from interpreter_ir import FunctionTypeStatus, IIRModule

from algol_iir_compiler import (
    AlgolIIRUnsupportedError,
    AlgolVM,
    __version__,
    compile_source,
    compile_to_iir,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_literal_result_compiles_to_typed_iir() -> None:
    module = compile_source("begin integer result; result := 7 end")
    assert isinstance(module, IIRModule)
    assert module.language == "algol60"
    assert module.entry_point == "main"
    (fn,) = module.functions
    assert fn.name == "main"
    assert fn.return_type == "i32"
    assert fn.type_status == FunctionTypeStatus.FULLY_TYPED
    assert fn.instructions[-1].op == "ret"
    assert fn.instructions[-1].srcs == ["result"]


def test_arithmetic_precedence_emits_mul_before_add() -> None:
    module = compile_source("begin integer result; result := 1 + 2 * 3 end")
    ops = [instr.op for instr in module.functions[0].instructions]
    assert ops.index("mul") < ops.index("add")


def test_conditional_expression_uses_branch_labels() -> None:
    module = compile_source(
        "begin integer result; result := if true then 1 else 2 end"
    )
    ops = [instr.op for instr in module.functions[0].instructions]
    assert "jmp_if_false" in ops
    assert "jmp" in ops
    assert ops.count("label") == 2


def test_compile_to_iir_accepts_preparsed_ast() -> None:
    ast = parse_algol("begin integer result; result := 7 end")
    module = compile_to_iir(ast, module_name="preparsed.alg")
    assert module.name == "preparsed.alg"


def test_vm_runs_integer_arithmetic() -> None:
    vm = AlgolVM()
    assert vm.run("begin integer result; result := 1 + 2 * 3 end") == 7


def test_vm_runs_if_else() -> None:
    vm = AlgolVM()
    assert (
        vm.run("begin integer result; if 1 < 2 then result := 7 else result := 8 end")
        == 7
    )


def test_vm_runs_for_step_until_loop() -> None:
    vm = AlgolVM()
    source = (
        "begin integer result, i; "
        "result := 0; "
        "for i := 1 step 1 until 3 do result := result + i "
        "end"
    )
    assert vm.run(source) == 6


def test_vm_runs_negative_for_step_until_loop() -> None:
    vm = AlgolVM()
    source = (
        "begin integer result, i; "
        "result := 0; "
        "for i := 3 step -1 until 1 do result := result + i "
        "end"
    )
    assert vm.run(source) == 6


def test_vm_runs_boolean_not() -> None:
    vm = AlgolVM()
    source = (
        "begin integer result; "
        "if not false then result := 1 else result := 0 "
        "end"
    )
    assert vm.run(source) == 1


def test_vm_runs_unary_minus() -> None:
    vm = AlgolVM()
    assert vm.run("begin integer result; result := -1 + 2 end") == 1


def test_vm_runs_compound_statement() -> None:
    vm = AlgolVM()
    source = (
        "begin integer result; "
        "begin result := 1; result := result + 1 end "
        "end"
    )
    assert vm.run(source) == 2


def test_program_without_result_returns_none() -> None:
    assert AlgolVM().run("begin integer x; x := 7 end") is None


def test_vm_records_last_metrics() -> None:
    vm = AlgolVM()
    assert vm.run("begin integer result; result := 7 end") == 7
    assert vm.last_metrics is not None
    assert vm.last_metrics.total_instructions_executed > 0


def test_vm_runs_local_goto() -> None:
    vm = AlgolVM()
    source = (
        "begin integer result; "
        "goto done; "
        "result := 1; "
        "done: result := 7 "
        "end"
    )
    assert vm.run(source) == 7


def test_real_result_uses_f64() -> None:
    module = compile_source("begin real result; result := 1.5 end")
    (fn,) = module.functions
    assert fn.return_type == "f64"
    assert fn.instructions[-1].type_hint == "f64"
    assert AlgolVM().execute_module(module) == 1.5


def test_array_declarations_are_explicitly_unsupported() -> None:
    with pytest.raises(AlgolIIRUnsupportedError, match="array_decl"):
        compile_source("begin integer array a[1:3]; a[1] := 7 end")


def test_nested_declarations_are_explicitly_unsupported() -> None:
    with pytest.raises(AlgolIIRUnsupportedError, match="nested block declarations"):
        compile_source("begin integer x; begin integer y; y := 1 end end")


def test_dynamic_for_step_is_explicitly_unsupported() -> None:
    with pytest.raises(AlgolIIRUnsupportedError, match="dynamic step sign"):
        compile_source(
            "begin integer result, i, s; "
            "s := 1; "
            "for i := 1 step s until 3 do result := i "
            "end"
        )


def test_real_division_is_explicitly_unsupported_for_vm_slice() -> None:
    with pytest.raises(AlgolIIRUnsupportedError, match="real division"):
        compile_source("begin real result; result := 4.0 / 2.0 end")


def test_logical_and_is_explicitly_unsupported_for_common_backend_slice() -> None:
    with pytest.raises(AlgolIIRUnsupportedError, match="logical operators"):
        compile_source(
            "begin integer result; "
            "if (1 < 2) and true then result := 1 else result := 0 "
            "end"
        )
