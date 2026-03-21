"""Tests for Starlark VM opcode handlers and built-in functions."""

from __future__ import annotations

import pytest

from virtual_machine import (
    CodeObject,
    GenericVM,
    Instruction,
    VMTypeError,
)
from virtual_machine.vm import DivisionByZeroError, UndefinedNameError

from starlark_compiler.opcodes import Op
from starlark_vm import create_starlark_vm
from starlark_vm.builtins import (
    builtin_abs,
    builtin_all,
    builtin_any,
    builtin_bool,
    builtin_dict,
    builtin_enumerate,
    builtin_float,
    builtin_int,
    builtin_len,
    builtin_list,
    builtin_max,
    builtin_min,
    builtin_range,
    builtin_repr,
    builtin_reversed,
    builtin_sorted,
    builtin_str,
    builtin_tuple,
    builtin_type,
    builtin_zip,
)
from starlark_vm.handlers import _is_truthy


# =========================================================================
# Helpers
# =========================================================================


def exec_code(instructions, constants=None, names=None):
    """Execute instructions on a fresh Starlark VM."""
    vm = create_starlark_vm()
    code = CodeObject(
        instructions=instructions,
        constants=constants or [],
        names=names or [],
    )
    vm.execute(code)
    return vm


# =========================================================================
# Test: Truthiness
# =========================================================================


class TestTruthiness:
    """Test Starlark truthiness rules."""

    def test_none_is_falsy(self):
        assert _is_truthy(None) is False

    def test_false_is_falsy(self):
        assert _is_truthy(False) is False

    def test_true_is_truthy(self):
        assert _is_truthy(True) is True

    def test_zero_is_falsy(self):
        assert _is_truthy(0) is False

    def test_nonzero_is_truthy(self):
        assert _is_truthy(42) is True

    def test_empty_string_is_falsy(self):
        assert _is_truthy("") is False

    def test_nonempty_string_is_truthy(self):
        assert _is_truthy("x") is True

    def test_empty_list_is_falsy(self):
        assert _is_truthy([]) is False

    def test_nonempty_list_is_truthy(self):
        assert _is_truthy([1]) is True

    def test_empty_dict_is_falsy(self):
        assert _is_truthy({}) is False

    def test_empty_tuple_is_falsy(self):
        assert _is_truthy(()) is False


# =========================================================================
# Test: Type Errors in Arithmetic
# =========================================================================


class TestTypeErrors:
    """Test that type errors are raised for invalid operations."""

    def test_add_string_and_int_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.ADD),
                Instruction(Op.HALT),
            ],
            constants=["hello", 42],
        )
        with pytest.raises(VMTypeError, match="Cannot add"):
            vm.execute(code)

    def test_subtract_strings_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.SUB),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="Cannot subtract"):
            vm.execute(code)

    def test_divide_by_zero_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.DIV),
                Instruction(Op.HALT),
            ],
            constants=[10, 0],
        )
        with pytest.raises(DivisionByZeroError):
            vm.execute(code)


# =========================================================================
# Test: Built-in Functions
# =========================================================================


class TestBuiltins:
    """Test Starlark built-in functions."""

    def test_type_int(self):
        assert builtin_type([42]) == "int"

    def test_type_string(self):
        assert builtin_type(["hello"]) == "string"

    def test_type_list(self):
        assert builtin_type([[1, 2]]) == "list"

    def test_type_dict(self):
        assert builtin_type([{}]) == "dict"

    def test_type_bool(self):
        assert builtin_type([True]) == "bool"

    def test_type_none(self):
        assert builtin_type([None]) == "NoneType"

    def test_len_list(self):
        assert builtin_len([[1, 2, 3]]) == 3

    def test_len_string(self):
        assert builtin_len(["hello"]) == 5

    def test_len_dict(self):
        assert builtin_len([{"a": 1}]) == 1

    def test_len_wrong_type(self):
        with pytest.raises(VMTypeError):
            builtin_len([42])

    def test_range_one_arg(self):
        assert builtin_range([5]) == [0, 1, 2, 3, 4]

    def test_range_two_args(self):
        assert builtin_range([2, 5]) == [2, 3, 4]

    def test_range_three_args(self):
        assert builtin_range([0, 10, 3]) == [0, 3, 6, 9]

    def test_sorted_list(self):
        assert builtin_sorted([[3, 1, 2]]) == [1, 2, 3]

    def test_reversed_list(self):
        assert builtin_reversed([[1, 2, 3]]) == [3, 2, 1]

    def test_bool_truthy(self):
        assert builtin_bool([42]) is True

    def test_bool_falsy(self):
        assert builtin_bool([0]) is False

    def test_int_from_float(self):
        assert builtin_int([3.7]) == 3

    def test_int_from_string(self):
        assert builtin_int(["42"]) == 42

    def test_float_from_int(self):
        assert builtin_float([42]) == 42.0

    def test_str_from_int(self):
        assert builtin_str([42]) == "42"

    def test_list_from_tuple(self):
        assert builtin_list([(1, 2, 3)]) == [1, 2, 3]

    def test_tuple_from_list(self):
        assert builtin_tuple([[1, 2, 3]]) == (1, 2, 3)

    def test_dict_empty(self):
        assert builtin_dict([]) == {}

    def test_min_args(self):
        assert builtin_min([3, 1, 2]) == 1

    def test_max_args(self):
        assert builtin_max([3, 1, 2]) == 3

    def test_abs_negative(self):
        assert builtin_abs([-5]) == 5

    def test_all_true(self):
        assert builtin_all([[True, True]]) is True

    def test_all_false(self):
        assert builtin_all([[True, False]]) is False

    def test_any_true(self):
        assert builtin_any([[False, True]]) is True

    def test_any_false(self):
        assert builtin_any([[False, False]]) is False

    def test_repr_string(self):
        assert builtin_repr(["hello"]) == "'hello'"

    def test_enumerate_list(self):
        assert builtin_enumerate([[10, 20, 30]]) == [(0, 10), (1, 20), (2, 30)]

    def test_zip_lists(self):
        assert builtin_zip([[1, 2], [3, 4]]) == [(1, 3), (2, 4)]


# =========================================================================
# Test: Undefined Variable
# =========================================================================


class TestUndefinedVariable:
    """Test undefined variable error."""

    def test_load_undefined_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_NAME, 0),
                Instruction(Op.HALT),
            ],
            names=["undefined_var"],
        )
        with pytest.raises(UndefinedNameError):
            vm.execute(code)


# =========================================================================
# Test: Collection Handlers
# =========================================================================


class TestCollectionHandlers:
    """Test BUILD_LIST, BUILD_DICT, BUILD_TUPLE handlers."""

    def test_build_list(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LOAD_CONST, 1),
            Instruction(Op.LOAD_CONST, 2),
            Instruction(Op.BUILD_LIST, 3),
            Instruction(Op.HALT),
        ], constants=[1, 2, 3])
        assert vm.stack == [[1, 2, 3]]

    def test_build_dict(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),  # key "a"
            Instruction(Op.LOAD_CONST, 1),  # value 1
            Instruction(Op.BUILD_DICT, 1),
            Instruction(Op.HALT),
        ], constants=["a", 1])
        assert vm.stack == [{"a": 1}]

    def test_build_tuple(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LOAD_CONST, 1),
            Instruction(Op.BUILD_TUPLE, 2),
            Instruction(Op.HALT),
        ], constants=[1, 2])
        assert vm.stack == [(1, 2)]

    def test_build_empty_list(self):
        vm = exec_code([
            Instruction(Op.BUILD_LIST, 0),
            Instruction(Op.HALT),
        ])
        assert vm.stack == [[]]


# =========================================================================
# Test: Iteration Handlers
# =========================================================================


class TestIterationHandlers:
    """Test GET_ITER, FOR_ITER, UNPACK_SEQUENCE handlers."""

    def test_unpack_sequence(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),  # (1, 2, 3)
            Instruction(Op.UNPACK_SEQUENCE, 3),
            Instruction(Op.HALT),
        ], constants=[(1, 2, 3)])
        # Values pushed in reverse: 3, 2, 1 → stack = [3, 2, 1]
        assert vm.stack == [3, 2, 1]

    def test_get_iter_and_for_iter(self):
        """Test the iterator protocol: iterate over [10, 20]."""
        vm = exec_code([
            # Build the accumulator
            Instruction(Op.LOAD_CONST, 0),   # 0: 0 (accumulator)
            Instruction(Op.STORE_NAME, 0),   # 1: total = 0
            # Build the list and iterate
            Instruction(Op.LOAD_CONST, 1),   # 2: [10, 20]
            Instruction(Op.GET_ITER),         # 3: get iterator
            Instruction(Op.FOR_ITER, 11),     # 4: next or jump to 11 (HALT)
            Instruction(Op.STORE_NAME, 1),   # 5: item = next
            # total = total + item
            Instruction(Op.LOAD_NAME, 0),    # 6: total
            Instruction(Op.LOAD_NAME, 1),    # 7: item
            Instruction(Op.ADD),              # 8: total + item
            Instruction(Op.STORE_NAME, 0),   # 9: total = result
            Instruction(Op.JUMP, 4),          # 10: back to FOR_ITER
            Instruction(Op.HALT),             # 11: done
        ], constants=[0, [10, 20]], names=["total", "item"])
        assert vm.variables["total"] == 30
