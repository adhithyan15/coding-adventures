"""Additional tests to cover untested handlers and builtins.

These tests target specific coverage gaps in handlers.py, builtins.py,
and vm.py to bring overall coverage above 80%.
"""

from __future__ import annotations

import pytest

from virtual_machine import (
    CodeObject,
    GenericVM,
    Instruction,
    VMTypeError,
)
from virtual_machine.vm import (
    DivisionByZeroError,
    UndefinedNameError,
    VMError,
)

from starlark_compiler.opcodes import Op
from starlark_compiler import compile_starlark
from starlark_vm import create_starlark_vm, execute_starlark, StarlarkResult
from starlark_vm.builtins import (
    builtin_abs,
    builtin_all,
    builtin_any,
    builtin_bool,
    builtin_dict,
    builtin_enumerate,
    builtin_float,
    builtin_getattr,
    builtin_hasattr,
    builtin_int,
    builtin_len,
    builtin_list,
    builtin_max,
    builtin_min,
    builtin_print,
    builtin_range,
    builtin_repr,
    builtin_reversed,
    builtin_sorted,
    builtin_str,
    builtin_tuple,
    builtin_type,
    builtin_zip,
    get_all_builtins,
)
from starlark_vm.handlers import (
    StarlarkFunction,
    StarlarkIterator,
    _is_truthy,
    _starlark_repr,
)


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


def run(source: str) -> StarlarkResult:
    """Compile and execute Starlark source."""
    return execute_starlark(source)


# =========================================================================
# Test: StarlarkIterator
# =========================================================================


class TestStarlarkIterator:
    """Test StarlarkIterator wrapper."""

    def test_iter_list(self):
        it = StarlarkIterator([1, 2, 3])
        assert next(it) == 1
        assert next(it) == 2
        assert next(it) == 3
        with pytest.raises(StopIteration):
            next(it)

    def test_repr(self):
        it = StarlarkIterator([])
        assert repr(it) == "<starlark_iterator>"


# =========================================================================
# Test: StarlarkFunction
# =========================================================================


class TestStarlarkFunction:
    """Test StarlarkFunction object."""

    def test_repr(self):
        code = CodeObject(instructions=[], constants=[], names=[])
        f = StarlarkFunction(code=code, name="greet")
        assert repr(f) == "<function greet>"

    def test_default_name(self):
        code = CodeObject(instructions=[], constants=[], names=[])
        f = StarlarkFunction(code=code)
        assert f.name == "<lambda>"

    def test_defaults(self):
        code = CodeObject(instructions=[], constants=[], names=[])
        f = StarlarkFunction(code=code, defaults=[10, 20])
        assert f.defaults == [10, 20]


# =========================================================================
# Test: _starlark_repr
# =========================================================================


class TestStarlarkRepr:
    """Test _starlark_repr formatting."""

    def test_none(self):
        assert _starlark_repr(None) == "None"

    def test_true(self):
        assert _starlark_repr(True) == "True"

    def test_false(self):
        assert _starlark_repr(False) == "False"

    def test_string(self):
        assert _starlark_repr("hello") == "hello"

    def test_int(self):
        assert _starlark_repr(42) == "42"

    def test_list(self):
        assert _starlark_repr([1, 2]) == "[1, 2]"


# =========================================================================
# Test: Local Variables
# =========================================================================


class TestLocalVariables:
    """Test STORE_LOCAL and LOAD_LOCAL handlers."""

    def test_store_and_load_local(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.STORE_LOCAL, 0),
            Instruction(Op.LOAD_LOCAL, 0),
            Instruction(Op.HALT),
        ], constants=[42])
        assert vm.stack == [42]

    def test_store_local_extends_slots(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.STORE_LOCAL, 5),  # Slot 5, forces extension
            Instruction(Op.LOAD_LOCAL, 5),
            Instruction(Op.HALT),
        ], constants=[99])
        assert vm.stack == [99]

    def test_load_local_undefined_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_LOCAL, 10),
                Instruction(Op.HALT),
            ],
            constants=[],
            names=[],
        )
        with pytest.raises(UndefinedNameError):
            vm.execute(code)


# =========================================================================
# Test: Closure Variables
# =========================================================================


class TestClosureVariables:
    """Test STORE_CLOSURE and LOAD_CLOSURE handlers."""

    def test_store_and_load_closure(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.STORE_CLOSURE, 0),
            Instruction(Op.LOAD_CLOSURE, 0),
            Instruction(Op.HALT),
        ], constants=[77])
        assert vm.stack == [77]


# =========================================================================
# Test: Multiplication Branches
# =========================================================================


class TestMultiplicationBranches:
    """Test uncovered multiplication branches."""

    def test_int_times_string(self):
        result = run('x = 3 * "ab"\n')
        assert result.variables["x"] == "ababab"

    def test_int_times_list(self):
        """int * list should work too."""
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),  # 3
            Instruction(Op.LOAD_CONST, 1),  # [1, 2]
            Instruction(Op.MUL),
            Instruction(Op.HALT),
        ], constants=[3, [1, 2]])
        assert vm.stack == [[1, 2, 1, 2, 1, 2]]

    def test_mul_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.MUL),
                Instruction(Op.HALT),
            ],
            constants=["a", [1]],
        )
        with pytest.raises(VMTypeError, match="Cannot multiply"):
            vm.execute(code)


# =========================================================================
# Test: String Formatting (MOD)
# =========================================================================


class TestStringFormatting:
    """Test string % formatting."""

    def test_string_format_single(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LOAD_CONST, 1),
            Instruction(Op.MOD),
            Instruction(Op.HALT),
        ], constants=["Hello, %s!", "world"])
        assert vm.stack == ["Hello, world!"]

    def test_string_format_tuple(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LOAD_CONST, 1),
            Instruction(Op.MOD),
            Instruction(Op.HALT),
        ], constants=["%s=%d", ("x", 42)])
        assert vm.stack == ["x=42"]

    def test_mod_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.MOD),
                Instruction(Op.HALT),
            ],
            constants=[[1], [2]],
        )
        with pytest.raises(VMTypeError, match="Cannot compute"):
            vm.execute(code)

    def test_mod_by_zero(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.MOD),
                Instruction(Op.HALT),
            ],
            constants=[10, 0],
        )
        with pytest.raises(DivisionByZeroError):
            vm.execute(code)


# =========================================================================
# Test: Membership Tests (in, not in)
# =========================================================================


class TestMembership:
    """Test CMP_IN and CMP_NOT_IN handlers."""

    def test_in_list(self):
        result = run("x = 2 in [1, 2, 3]\n")
        assert result.variables["x"] is True

    def test_not_in_list(self):
        result = run("x = 5 not in [1, 2, 3]\n")
        assert result.variables["x"] is True

    def test_in_string(self):
        result = run('x = "ll" in "hello"\n')
        assert result.variables["x"] is True

    def test_in_dict(self):
        result = run('x = "a" in {"a": 1}\n')
        assert result.variables["x"] is True


# =========================================================================
# Test: JUMP_IF_TRUE handler
# =========================================================================


class TestJumpIfTrue:
    """Test JUMP_IF_TRUE handler (used less commonly)."""

    def test_jump_if_true_taken(self):
        vm = exec_code([
            Instruction(Op.LOAD_TRUE),
            Instruction(Op.JUMP_IF_TRUE, 3),
            Instruction(Op.LOAD_CONST, 0),   # skipped
            Instruction(Op.HALT),
        ], constants=[99])
        # The True was popped by JUMP_IF_TRUE, stack should be empty
        assert vm.stack == []

    def test_jump_if_true_not_taken(self):
        vm = exec_code([
            Instruction(Op.LOAD_FALSE),
            Instruction(Op.JUMP_IF_TRUE, 3),
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.HALT),
        ], constants=[42])
        assert vm.stack == [42]


# =========================================================================
# Test: Functions (end-to-end)
# =========================================================================


class TestFunctionExecution:
    """Test function definition and calling."""

    def test_simple_function(self):
        result = run("def f():\n    return 42\nx = f()\n")
        assert result.variables["x"] == 42

    def test_function_with_args(self):
        result = run("def add(a, b):\n    return a + b\nx = add(3, 4)\n")
        assert result.variables["x"] == 7

    def test_function_no_return(self):
        result = run("def f():\n    pass\nx = f()\n")
        assert result.variables["x"] is None

    def test_nested_function_calls(self):
        result = run("def double(x):\n    return x * 2\nx = double(double(3))\n")
        assert result.variables["x"] == 12


# =========================================================================
# Test: PRINT handler
# =========================================================================


class TestPrintHandler:
    """Test the PRINT opcode handler."""

    def test_print_int(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.PRINT),
            Instruction(Op.HALT),
        ], constants=[42])
        assert vm.output == ["42"]

    def test_print_string(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.PRINT),
            Instruction(Op.HALT),
        ], constants=["hello"])
        assert vm.output == ["hello"]

    def test_print_none(self):
        vm = exec_code([
            Instruction(Op.LOAD_NONE),
            Instruction(Op.PRINT),
            Instruction(Op.HALT),
        ])
        assert vm.output == ["None"]

    def test_print_bool(self):
        vm = exec_code([
            Instruction(Op.LOAD_TRUE),
            Instruction(Op.PRINT),
            Instruction(Op.HALT),
        ])
        assert vm.output == ["True"]


# =========================================================================
# Test: LOAD_ATTR handler
# =========================================================================


class TestLoadAttr:
    """Test LOAD_ATTR handler."""

    def test_list_method_exists(self):
        """Accessing .copy on a list should return a callable method."""
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LOAD_ATTR, 0),
            Instruction(Op.HALT),
        ], constants=[[1, 2, 3]], names=["copy"])
        assert callable(vm.stack[0])

    def test_load_attr_missing_raises(self):
        """Accessing a nonexistent attribute raises VMError."""
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_ATTR, 0),
                Instruction(Op.HALT),
            ],
            constants=[42],
            names=["nonexistent"],
        )
        with pytest.raises(VMError, match="has no attribute"):
            vm.execute(code)


# =========================================================================
# Test: STORE_ATTR handler
# =========================================================================


class TestStoreAttr:
    """Test STORE_ATTR handler."""

    def test_store_attr_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.STORE_ATTR, 0),
                Instruction(Op.HALT),
            ],
            constants=[42],
            names=["x"],
        )
        with pytest.raises(VMError, match="does not support attribute assignment"):
            vm.execute(code)


# =========================================================================
# Test: STORE_SUBSCRIPT handler
# =========================================================================


class TestStoreSubscript:
    """Test STORE_SUBSCRIPT handler."""

    def test_store_subscript_list(self):
        # STORE_SUBSCRIPT handler: value = pop(), key = pop(), obj = pop()
        # So push order is: obj, key, value
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),   # [10, 20, 30]
            Instruction(Op.STORE_NAME, 0),   # x = [10, 20, 30]
            Instruction(Op.LOAD_NAME, 0),    # x (obj)
            Instruction(Op.LOAD_CONST, 2),   # 1 (key)
            Instruction(Op.LOAD_CONST, 1),   # 99 (value)
            Instruction(Op.STORE_SUBSCRIPT),  # x[1] = 99
            Instruction(Op.HALT),
        ], constants=[[10, 20, 30], 99, 1], names=["x"])
        assert vm.variables["x"] == [10, 99, 30]

    def test_store_subscript_dict(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),   # {}
            Instruction(Op.STORE_NAME, 0),   # d = {}
            Instruction(Op.LOAD_NAME, 0),    # d (obj)
            Instruction(Op.LOAD_CONST, 2),   # "key" (key)
            Instruction(Op.LOAD_CONST, 1),   # 42 (value)
            Instruction(Op.STORE_SUBSCRIPT),  # d["key"] = 42
            Instruction(Op.HALT),
        ], constants=[{}, 42, "key"], names=["d"])
        assert vm.variables["d"] == {"key": 42}


# =========================================================================
# Test: LOAD_SLICE handler
# =========================================================================


class TestLoadSlice:
    """Test LOAD_SLICE handler."""

    def test_slice_all(self):
        """[:] — flags=0 (no start, no stop, no step)
        Handler pops: nothing for step/stop/start (flags=0), then obj.
        """
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),  # obj = [1, 2, 3]
            Instruction(Op.LOAD_SLICE, 0),
            Instruction(Op.HALT),
        ], constants=[[1, 2, 3]])
        assert vm.stack == [[1, 2, 3]]

    def test_slice_with_start_stop(self):
        """[1:3] — flags=0x03 (start=bit0, stop=bit1)
        Handler pops: stop (bit1), start (bit0), then obj.
        Push order: obj, start, stop.
        """
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),  # obj = [10, 20, 30, 40]
            Instruction(Op.LOAD_CONST, 1),  # start=1
            Instruction(Op.LOAD_CONST, 2),  # stop=3
            Instruction(Op.LOAD_SLICE, 0x03),
            Instruction(Op.HALT),
        ], constants=[[10, 20, 30, 40], 1, 3])
        assert vm.stack == [[20, 30]]


# =========================================================================
# Test: LIST_APPEND and DICT_SET handlers
# =========================================================================


class TestComprehensionHandlers:
    """Test LIST_APPEND and DICT_SET handlers."""

    def test_list_append(self):
        vm = exec_code([
            Instruction(Op.BUILD_LIST, 0),
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LIST_APPEND),
            Instruction(Op.LOAD_CONST, 1),
            Instruction(Op.LIST_APPEND),
            Instruction(Op.HALT),
        ], constants=[10, 20])
        assert vm.stack == [[10, 20]]

    def test_dict_set(self):
        vm = exec_code([
            Instruction(Op.BUILD_DICT, 0),
            Instruction(Op.LOAD_CONST, 0),  # key
            Instruction(Op.LOAD_CONST, 1),  # value
            Instruction(Op.DICT_SET),
            Instruction(Op.HALT),
        ], constants=["a", 1])
        # DICT_SET pops value then key: stack order is key, value (pushed L-R)
        # handler does: value = pop(), key = pop() → d[key] = value
        assert vm.stack == [{"a": 1}]

    def test_list_append_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),  # not a list
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.LIST_APPEND),
                Instruction(Op.HALT),
            ],
            constants=[42, 99],
        )
        with pytest.raises(VMTypeError, match="LIST_APPEND requires a list"):
            vm.execute(code)

    def test_dict_set_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),  # not a dict
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.LOAD_CONST, 2),
                Instruction(Op.DICT_SET),
                Instruction(Op.HALT),
            ],
            constants=[42, "key", "val"],
        )
        with pytest.raises(VMTypeError, match="DICT_SET requires a dict"):
            vm.execute(code)


# =========================================================================
# Test: LOAD_MODULE and IMPORT_FROM
# =========================================================================


class TestModuleHandlers:
    """Test LOAD_MODULE and IMPORT_FROM handlers."""

    def test_load_module(self):
        vm = exec_code([
            Instruction(Op.LOAD_MODULE, 0),
            Instruction(Op.HALT),
        ], names=["test.star"])
        assert vm.stack == [{"__name__": "test.star"}]

    def test_import_from_success(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.DUP),
            Instruction(Op.IMPORT_FROM, 0),
            Instruction(Op.HALT),
        ], constants=[{"__name__": "mod", "sym": 42}], names=["sym"])
        # Stack: module, 42
        assert vm.stack[-1] == 42

    def test_import_from_missing_raises(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.IMPORT_FROM, 0),
                Instruction(Op.HALT),
            ],
            constants=[{"__name__": "mod"}],
            names=["missing"],
        )
        with pytest.raises(VMError, match="Cannot import"):
            vm.execute(code)


# =========================================================================
# Test: STORE_NAME frozen
# =========================================================================


class TestFrozenMode:
    """Test that frozen mode blocks mutations."""

    def test_frozen_store_name_raises(self):
        vm = create_starlark_vm(frozen=True)
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.STORE_NAME, 0),
                Instruction(Op.HALT),
            ],
            constants=[42],
            names=["x"],
        )
        with pytest.raises(VMError, match="frozen"):
            vm.execute(code)

    def test_frozen_store_subscript_raises(self):
        vm = create_starlark_vm(frozen=True)
        vm.variables["x"] = [1, 2, 3]
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.LOAD_NAME, 0),
                Instruction(Op.STORE_SUBSCRIPT),
                Instruction(Op.HALT),
            ],
            constants=[99, 0],
            names=["x"],
        )
        with pytest.raises(VMError, match="frozen"):
            vm.execute(code)


# =========================================================================
# Test: RETURN at top level
# =========================================================================


class TestReturnTopLevel:
    """Test RETURN at top level halts execution."""

    def test_return_halts(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.RETURN),
            Instruction(Op.LOAD_CONST, 1),  # Should not be reached
            Instruction(Op.HALT),
        ], constants=[42, 99])
        assert vm.halted


# =========================================================================
# Test: Error Paths in Arithmetic
# =========================================================================


class TestArithmeticErrors:
    """Test type error paths in arithmetic handlers."""

    def test_floor_div_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.FLOOR_DIV),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="floor-divide"):
            vm.execute(code)

    def test_floor_div_by_zero(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.FLOOR_DIV),
                Instruction(Op.HALT),
            ],
            constants=[10, 0],
        )
        with pytest.raises(DivisionByZeroError):
            vm.execute(code)

    def test_power_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.POWER),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="Cannot compute"):
            vm.execute(code)

    def test_negate_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.NEGATE),
                Instruction(Op.HALT),
            ],
            constants=["hello"],
        )
        with pytest.raises(VMTypeError, match="Cannot negate"):
            vm.execute(code)

    def test_bit_and_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.BIT_AND),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="bitwise AND"):
            vm.execute(code)

    def test_bit_or_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.BIT_OR),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="bitwise OR"):
            vm.execute(code)

    def test_bit_xor_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.BIT_XOR),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="bitwise XOR"):
            vm.execute(code)

    def test_bit_not_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.BIT_NOT),
                Instruction(Op.HALT),
            ],
            constants=["hello"],
        )
        with pytest.raises(VMTypeError, match="bitwise NOT"):
            vm.execute(code)

    def test_lshift_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.LSHIFT),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="left-shift"):
            vm.execute(code)

    def test_rshift_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.RSHIFT),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="right-shift"):
            vm.execute(code)

    def test_div_type_error(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.LOAD_CONST, 1),
                Instruction(Op.DIV),
                Instruction(Op.HALT),
            ],
            constants=["a", "b"],
        )
        with pytest.raises(VMTypeError, match="Cannot divide"):
            vm.execute(code)


# =========================================================================
# Test: Tuple Concatenation
# =========================================================================


class TestTupleOps:
    """Test tuple-specific operations."""

    def test_tuple_concat(self):
        vm = exec_code([
            Instruction(Op.LOAD_CONST, 0),
            Instruction(Op.LOAD_CONST, 1),
            Instruction(Op.ADD),
            Instruction(Op.HALT),
        ], constants=[(1, 2), (3, 4)])
        assert vm.stack == [(1, 2, 3, 4)]


# =========================================================================
# Test: Builtin Error Paths
# =========================================================================


class TestBuiltinErrors:
    """Test error paths in built-in functions."""

    def test_type_wrong_args(self):
        with pytest.raises(VMTypeError, match="type.*takes"):
            builtin_type([1, 2])

    def test_bool_wrong_args(self):
        with pytest.raises(VMTypeError, match="bool.*takes"):
            builtin_bool([])

    def test_int_wrong_args(self):
        with pytest.raises(VMTypeError, match="int.*takes"):
            builtin_int([])

    def test_int_bad_type(self):
        with pytest.raises(VMTypeError, match="int.*argument"):
            builtin_int([[1, 2]])

    def test_int_with_base(self):
        assert builtin_int(["ff", 16]) == 255

    def test_int_with_base_non_string(self):
        with pytest.raises(VMTypeError, match="non-string"):
            builtin_int([42, 10])

    def test_int_from_bool(self):
        assert builtin_int([True]) == 1
        assert builtin_int([False]) == 0

    def test_float_wrong_args(self):
        with pytest.raises(VMTypeError, match="float.*takes"):
            builtin_float([1, 2])

    def test_float_from_string(self):
        assert builtin_float(["3.14"]) == 3.14

    def test_float_bad_type(self):
        with pytest.raises(VMTypeError, match="float.*argument"):
            builtin_float([[1]])

    def test_str_wrong_args(self):
        with pytest.raises(VMTypeError, match="str.*takes"):
            builtin_str([1, 2])

    def test_str_none(self):
        assert builtin_str([None]) == "None"

    def test_str_bool(self):
        assert builtin_str([True]) == "True"
        assert builtin_str([False]) == "False"

    def test_str_string(self):
        assert builtin_str(["hello"]) == "hello"

    def test_len_wrong_args(self):
        with pytest.raises(VMTypeError, match="len.*takes"):
            builtin_len([1, 2])

    def test_len_tuple(self):
        assert builtin_len([(1, 2, 3)]) == 3

    def test_list_wrong_args(self):
        with pytest.raises(VMTypeError, match="list.*takes"):
            builtin_list([1, 2])

    def test_list_empty(self):
        assert builtin_list([]) == []

    def test_dict_from_pairs(self):
        assert builtin_dict([[("a", 1), ("b", 2)]]) == {"a": 1, "b": 2}

    def test_dict_wrong_args(self):
        with pytest.raises(VMTypeError, match="dict.*takes"):
            builtin_dict([1, 2])

    def test_tuple_empty(self):
        assert builtin_tuple([]) == ()

    def test_tuple_wrong_args(self):
        with pytest.raises(VMTypeError, match="tuple.*takes"):
            builtin_tuple([1, 2])

    def test_range_wrong_args(self):
        with pytest.raises(VMTypeError, match="range.*takes"):
            builtin_range([1, 2, 3, 4])

    def test_sorted_wrong_args(self):
        with pytest.raises(VMTypeError, match="sorted.*takes"):
            builtin_sorted([])

    def test_sorted_reverse(self):
        assert builtin_sorted([[3, 1, 2], True]) == [3, 2, 1]

    def test_reversed_wrong_args(self):
        with pytest.raises(VMTypeError, match="reversed.*takes"):
            builtin_reversed([1, 2])

    def test_enumerate_wrong_args(self):
        with pytest.raises(VMTypeError, match="enumerate.*takes"):
            builtin_enumerate([])

    def test_enumerate_with_start(self):
        assert builtin_enumerate([[10, 20], 5]) == [(5, 10), (6, 20)]

    def test_min_iterable(self):
        assert builtin_min([[3, 1, 2]]) == 1

    def test_max_iterable(self):
        assert builtin_max([[3, 1, 2]]) == 3

    def test_abs_wrong_args(self):
        with pytest.raises(VMTypeError, match="abs.*takes"):
            builtin_abs([1, 2])

    def test_all_wrong_args(self):
        with pytest.raises(VMTypeError, match="all.*takes"):
            builtin_all([1, 2])

    def test_any_wrong_args(self):
        with pytest.raises(VMTypeError, match="any.*takes"):
            builtin_any([1, 2])

    def test_repr_wrong_args(self):
        with pytest.raises(VMTypeError, match="repr.*takes"):
            builtin_repr([1, 2])

    def test_repr_int(self):
        assert builtin_repr([42]) == "42"

    def test_hasattr_wrong_args(self):
        with pytest.raises(VMTypeError, match="hasattr.*takes"):
            builtin_hasattr([1])

    def test_hasattr_true(self):
        assert builtin_hasattr(["hello", "upper"]) is True

    def test_hasattr_false(self):
        assert builtin_hasattr([42, "nonexistent"]) is False

    def test_getattr_wrong_args(self):
        with pytest.raises(VMTypeError, match="getattr.*takes"):
            builtin_getattr([1])

    def test_getattr_basic(self):
        result = builtin_getattr(["hello", "upper"])
        assert callable(result)

    def test_getattr_with_default(self):
        result = builtin_getattr([42, "nonexistent", "default"])
        assert result == "default"

    def test_print_returns_none(self):
        assert builtin_print([1, 2, 3]) is None

    def test_type_tuple(self):
        assert builtin_type([(1, 2)]) == "tuple"

    def test_type_float(self):
        assert builtin_type([3.14]) == "float"

    def test_type_function(self):
        """type() of an unknown type uses __name__."""
        assert builtin_type([object()]) == "object"

    def test_bool_none(self):
        assert builtin_bool([None]) is False

    def test_bool_empty_tuple(self):
        assert builtin_bool([()]) is False

    def test_bool_nonempty_tuple(self):
        assert builtin_bool([(1,)]) is True

    def test_bool_object(self):
        assert builtin_bool([object()]) is True


# =========================================================================
# Test: execute_starlark convenience
# =========================================================================


class TestExecuteStarlark:
    """Test execute_starlark convenience function."""

    def test_returns_starlark_result(self):
        result = execute_starlark("x = 42\n")
        assert isinstance(result, StarlarkResult)
        assert result.variables["x"] == 42

    def test_output_captured(self):
        # print() builtin doesn't emit PRINT opcode directly
        # but we can test via the helper
        result = execute_starlark("x = 1\n")
        assert result.output == []

    def test_traces_returned(self):
        result = execute_starlark("x = 1\n")
        assert isinstance(result.traces, list)


# =========================================================================
# Test: get_all_builtins
# =========================================================================


class TestGetAllBuiltins:
    """Test get_all_builtins registry."""

    def test_returns_dict(self):
        builtins = get_all_builtins()
        assert isinstance(builtins, dict)

    def test_contains_expected_functions(self):
        builtins = get_all_builtins()
        expected = [
            "type", "bool", "int", "float", "str", "len",
            "list", "dict", "tuple", "range", "sorted",
            "reversed", "enumerate", "zip", "min", "max",
            "abs", "all", "any", "repr", "hasattr", "getattr",
            "print",
        ]
        for name in expected:
            assert name in builtins, f"Missing built-in: {name}"

    def test_all_callable(self):
        for name, impl in get_all_builtins().items():
            assert callable(impl), f"Built-in {name} is not callable"


# =========================================================================
# Test: LOAD_CONST error path
# =========================================================================


class TestLoadConstError:
    """Test LOAD_CONST with invalid operand."""

    def test_invalid_operand(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 99),  # Out of range
                Instruction(Op.HALT),
            ],
            constants=[42],
        )
        from virtual_machine.vm import InvalidOperandError
        with pytest.raises(InvalidOperandError):
            vm.execute(code)


# =========================================================================
# Test: STORE_NAME / LOAD_NAME error paths
# =========================================================================


class TestNameErrors:
    """Test name-related error paths."""

    def test_store_name_invalid_operand(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.STORE_NAME, 99),  # Out of range
                Instruction(Op.HALT),
            ],
            constants=[42],
            names=["x"],
        )
        from virtual_machine.vm import InvalidOperandError
        with pytest.raises(InvalidOperandError):
            vm.execute(code)

    def test_load_name_invalid_operand(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_NAME, 99),  # Out of range
                Instruction(Op.HALT),
            ],
            constants=[],
            names=["x"],
        )
        from virtual_machine.vm import InvalidOperandError
        with pytest.raises(InvalidOperandError):
            vm.execute(code)

    def test_load_name_builtin(self):
        """Loading a builtin name should push the builtin function."""
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_NAME, 0),
                Instruction(Op.HALT),
            ],
            constants=[],
            names=["len"],
        )
        vm.execute(code)
        # The builtin should be on the stack
        assert vm.stack[0] is not None


# =========================================================================
# Test: Calling builtins
# =========================================================================


class TestCallingBuiltins:
    """Test calling built-in functions via CALL_FUNCTION."""

    def test_call_builtin_not_callable(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),  # 42 (not callable)
                Instruction(Op.CALL_FUNCTION, 0),
                Instruction(Op.HALT),
            ],
            constants=[42],
        )
        with pytest.raises(VMTypeError, match="not callable"):
            vm.execute(code)


# =========================================================================
# Test: UNPACK_SEQUENCE error
# =========================================================================


class TestUnpackError:
    """Test UNPACK_SEQUENCE with wrong count."""

    def test_unpack_wrong_count(self):
        vm = create_starlark_vm()
        code = CodeObject(
            instructions=[
                Instruction(Op.LOAD_CONST, 0),
                Instruction(Op.UNPACK_SEQUENCE, 5),  # Wrong count
                Instruction(Op.HALT),
            ],
            constants=[(1, 2, 3)],
        )
        with pytest.raises(VMError, match="Cannot unpack"):
            vm.execute(code)
