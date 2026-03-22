"""Starlark VM — A complete Starlark bytecode interpreter.

==========================================================================
Chapter 1: The Full Pipeline
==========================================================================

This module ties everything together. The ``create_starlark_vm()`` factory
creates a ``GenericVM`` that's fully configured for Starlark execution:

1. All ~50 opcodes have registered handlers.
2. All ~25 built-in functions are registered.
3. Starlark-specific restrictions are configured (recursion limits, etc.).

The ``execute_starlark()`` convenience function goes even further: it takes
Starlark source code as a string, compiles it, and executes it in one call.

==========================================================================
Chapter 2: How to Use
==========================================================================

**Quick start — one call does everything:**

    result = execute_starlark("x = 1 + 2\\nprint(x)\\n")
    print(result.variables["x"])  # 3
    print(result.output)          # ["3"]

**Step by step — for more control:**

    from starlark_ast_to_bytecode_compiler import compile_starlark
    from starlark_vm import create_starlark_vm

    # Compile
    code = compile_starlark("x = 1 + 2\\n")

    # Execute
    vm = create_starlark_vm()
    traces = vm.execute(code)

    # Inspect
    print(vm.variables["x"])  # 3
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from virtual_machine import GenericVM, VMTrace

from starlark_ast_to_bytecode_compiler.opcodes import Op
from starlark_vm.builtins import get_all_builtins
from starlark_vm.handlers import (
    handle_add,
    handle_bit_and,
    handle_bit_not,
    handle_bit_or,
    handle_bit_xor,
    handle_build_dict,
    handle_build_list,
    handle_build_tuple,
    handle_call_function,
    handle_call_function_kw,
    handle_cmp_eq,
    handle_cmp_ge,
    handle_cmp_gt,
    handle_cmp_in,
    handle_cmp_le,
    handle_cmp_lt,
    handle_cmp_ne,
    handle_cmp_not_in,
    handle_dict_set,
    handle_div,
    handle_dup,
    handle_floor_div,
    handle_for_iter,
    handle_get_iter,
    handle_halt,
    handle_import_from,
    handle_jump,
    handle_jump_if_false,
    handle_jump_if_false_or_pop,
    handle_jump_if_true,
    handle_jump_if_true_or_pop,
    handle_list_append,
    handle_load_attr,
    handle_load_closure,
    handle_load_const,
    handle_load_false,
    handle_load_local,
    handle_load_module,
    handle_load_name,
    handle_load_none,
    handle_load_slice,
    handle_load_subscript,
    handle_load_true,
    handle_lshift,
    handle_make_function,
    handle_mod,
    handle_mul,
    handle_negate,
    handle_not,
    handle_pop,
    handle_power,
    handle_print,
    handle_return,
    handle_rshift,
    handle_store_attr,
    handle_store_closure,
    handle_store_local,
    handle_store_name,
    handle_store_subscript,
    handle_sub,
    handle_unpack_sequence,
)


# =========================================================================
# VM Factory
# =========================================================================


def create_starlark_vm(
    max_recursion_depth: int = 200,
    frozen: bool = False,
) -> GenericVM:
    """Create a ``GenericVM`` fully configured for Starlark execution.

    This is the main factory function. It:
    1. Creates a fresh GenericVM.
    2. Registers all ~50 Starlark opcode handlers.
    3. Registers all ~25 Starlark built-in functions.
    4. Configures Starlark-specific restrictions.

    Parameters
    ----------
    max_recursion_depth : int
        Maximum call stack depth. Default 200. Set to 0 to completely
        forbid function calls (strict Starlark mode).
    frozen : bool
        Whether to start in frozen mode (no mutations allowed).

    Returns
    -------
    GenericVM
        A VM ready to execute Starlark bytecode.

    Example::

        vm = create_starlark_vm()
        code = compile_starlark("x = 42\\n")
        vm.execute(code)
        assert vm.variables["x"] == 42
    """
    vm = GenericVM()

    # -- Register all opcode handlers --

    # Stack operations
    vm.register_opcode(Op.LOAD_CONST, handle_load_const)
    vm.register_opcode(Op.POP, handle_pop)
    vm.register_opcode(Op.DUP, handle_dup)
    vm.register_opcode(Op.LOAD_NONE, handle_load_none)
    vm.register_opcode(Op.LOAD_TRUE, handle_load_true)
    vm.register_opcode(Op.LOAD_FALSE, handle_load_false)

    # Variable operations
    vm.register_opcode(Op.STORE_NAME, handle_store_name)
    vm.register_opcode(Op.LOAD_NAME, handle_load_name)
    vm.register_opcode(Op.STORE_LOCAL, handle_store_local)
    vm.register_opcode(Op.LOAD_LOCAL, handle_load_local)
    vm.register_opcode(Op.STORE_CLOSURE, handle_store_closure)
    vm.register_opcode(Op.LOAD_CLOSURE, handle_load_closure)

    # Arithmetic
    vm.register_opcode(Op.ADD, handle_add)
    vm.register_opcode(Op.SUB, handle_sub)
    vm.register_opcode(Op.MUL, handle_mul)
    vm.register_opcode(Op.DIV, handle_div)
    vm.register_opcode(Op.FLOOR_DIV, handle_floor_div)
    vm.register_opcode(Op.MOD, handle_mod)
    vm.register_opcode(Op.POWER, handle_power)
    vm.register_opcode(Op.NEGATE, handle_negate)
    vm.register_opcode(Op.BIT_AND, handle_bit_and)
    vm.register_opcode(Op.BIT_OR, handle_bit_or)
    vm.register_opcode(Op.BIT_XOR, handle_bit_xor)
    vm.register_opcode(Op.BIT_NOT, handle_bit_not)
    vm.register_opcode(Op.LSHIFT, handle_lshift)
    vm.register_opcode(Op.RSHIFT, handle_rshift)

    # Comparisons
    vm.register_opcode(Op.CMP_EQ, handle_cmp_eq)
    vm.register_opcode(Op.CMP_NE, handle_cmp_ne)
    vm.register_opcode(Op.CMP_LT, handle_cmp_lt)
    vm.register_opcode(Op.CMP_GT, handle_cmp_gt)
    vm.register_opcode(Op.CMP_LE, handle_cmp_le)
    vm.register_opcode(Op.CMP_GE, handle_cmp_ge)
    vm.register_opcode(Op.CMP_IN, handle_cmp_in)
    vm.register_opcode(Op.CMP_NOT_IN, handle_cmp_not_in)

    # Boolean
    vm.register_opcode(Op.NOT, handle_not)

    # Control flow
    vm.register_opcode(Op.JUMP, handle_jump)
    vm.register_opcode(Op.JUMP_IF_FALSE, handle_jump_if_false)
    vm.register_opcode(Op.JUMP_IF_TRUE, handle_jump_if_true)
    vm.register_opcode(Op.JUMP_IF_FALSE_OR_POP, handle_jump_if_false_or_pop)
    vm.register_opcode(Op.JUMP_IF_TRUE_OR_POP, handle_jump_if_true_or_pop)

    # Functions
    vm.register_opcode(Op.MAKE_FUNCTION, handle_make_function)
    vm.register_opcode(Op.CALL_FUNCTION, handle_call_function)
    vm.register_opcode(Op.CALL_FUNCTION_KW, handle_call_function_kw)
    vm.register_opcode(Op.RETURN, handle_return)

    # Collections
    vm.register_opcode(Op.BUILD_LIST, handle_build_list)
    vm.register_opcode(Op.BUILD_DICT, handle_build_dict)
    vm.register_opcode(Op.BUILD_TUPLE, handle_build_tuple)
    vm.register_opcode(Op.LIST_APPEND, handle_list_append)
    vm.register_opcode(Op.DICT_SET, handle_dict_set)

    # Subscript & attribute
    vm.register_opcode(Op.LOAD_SUBSCRIPT, handle_load_subscript)
    vm.register_opcode(Op.STORE_SUBSCRIPT, handle_store_subscript)
    vm.register_opcode(Op.LOAD_ATTR, handle_load_attr)
    vm.register_opcode(Op.STORE_ATTR, handle_store_attr)
    vm.register_opcode(Op.LOAD_SLICE, handle_load_slice)

    # Iteration
    vm.register_opcode(Op.GET_ITER, handle_get_iter)
    vm.register_opcode(Op.FOR_ITER, handle_for_iter)
    vm.register_opcode(Op.UNPACK_SEQUENCE, handle_unpack_sequence)

    # Module
    vm.register_opcode(Op.LOAD_MODULE, handle_load_module)
    vm.register_opcode(Op.IMPORT_FROM, handle_import_from)

    # I/O
    vm.register_opcode(Op.PRINT, handle_print)

    # VM control
    vm.register_opcode(Op.HALT, handle_halt)

    # -- Register built-in functions --
    for name, impl in get_all_builtins().items():
        vm.register_builtin(name, impl)

    # Override print() with a closure that captures output to the VM.
    # The default builtin_print returns None without side effects because
    # builtins don't have access to the VM instance. This closure fixes
    # that by writing to vm.output directly, matching the PRINT opcode's
    # behavior for calls made via ``print("hello")``.
    def _print_with_capture(args: list) -> None:
        output_str = " ".join(str(a) for a in args)
        vm.output.append(output_str)
        return None

    vm.register_builtin("print", _print_with_capture)

    # -- Configure restrictions --
    vm.set_max_recursion_depth(max_recursion_depth)
    if frozen:
        vm.set_frozen(True)

    return vm


# =========================================================================
# Execution Result
# =========================================================================


@dataclass
class StarlarkResult:
    """The result of executing a Starlark program.

    Contains all the information about the execution:
    - variables: the final state of all named variables
    - output: captured print output
    - traces: step-by-step execution trace (for debugging)
    """

    variables: dict[str, Any]
    """Final variable state after execution."""

    output: list[str]
    """Captured print output, one entry per print() call."""

    traces: list[VMTrace]
    """Step-by-step execution trace."""


# =========================================================================
# Convenience Functions
# =========================================================================


def execute_starlark(source: str) -> StarlarkResult:
    """Compile and execute Starlark source code in one call.

    This is the highest-level API. Pass in Starlark source code,
    get back the execution result with variables, output, and traces.

    Parameters
    ----------
    source : str
        Starlark source code. Should end with a newline.

    Returns
    -------
    StarlarkResult
        The execution result.

    Example::

        result = execute_starlark("x = 1 + 2\\nprint(x)\\n")
        assert result.variables["x"] == 3
        assert result.output == ["3"]
    """
    from starlark_ast_to_bytecode_compiler import compile_starlark

    code = compile_starlark(source)
    vm = create_starlark_vm()
    traces = vm.execute(code)

    return StarlarkResult(
        variables=dict(vm.variables),
        output=list(vm.output),
        traces=traces,
    )
