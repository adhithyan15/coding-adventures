"""Tests for the Starlark Interpreter — full pipeline integration tests.

These tests exercise the complete chain:
    source → lexer → parser → compiler → VM → result

They verify that the interpreter correctly handles:
- Basic expressions and assignments
- Function definitions and calls
- Keyword arguments (the key BUILD-file feature)
- The load() builtin for importing from other files
- Caching of loaded files
- Error handling for load() edge cases
"""

from __future__ import annotations

import os
import tempfile

import pytest

from starlark_interpreter import StarlarkInterpreter, interpret, interpret_file


# =========================================================================
# Basic Interpretation
# =========================================================================


class TestBasicInterpretation:
    """Test that basic Starlark programs run correctly through the interpreter."""

    def test_simple_assignment(self):
        """Verify a simple variable assignment."""
        result = interpret("x = 42\n")
        assert result.variables["x"] == 42

    def test_arithmetic(self):
        """Verify arithmetic expressions."""
        result = interpret("x = 1 + 2 * 3\n")
        assert result.variables["x"] == 7

    def test_string_concatenation(self):
        """Verify string operations."""
        result = interpret('greeting = "hello" + " " + "world"\n')
        assert result.variables["greeting"] == "hello world"

    def test_list_creation(self):
        """Verify list literal creation."""
        result = interpret("items = [1, 2, 3]\n")
        assert result.variables["items"] == [1, 2, 3]

    def test_dict_creation(self):
        """Verify dict literal creation."""
        result = interpret('d = {"a": 1, "b": 2}\n')
        assert result.variables["d"] == {"a": 1, "b": 2}

    def test_print_output(self):
        """Verify that print() captures output."""
        result = interpret('print("hello")\n')
        assert result.output == ["hello"]

    def test_multiple_prints(self):
        """Verify multiple print() calls."""
        result = interpret('print("a")\nprint("b")\nprint("c")\n')
        assert result.output == ["a", "b", "c"]

    def test_boolean_expressions(self):
        """Verify boolean logic."""
        result = interpret("x = True and False\ny = True or False\n")
        assert result.variables["x"] is False
        assert result.variables["y"] is True


# =========================================================================
# Functions
# =========================================================================


class TestFunctions:
    """Test function definitions and calls."""

    def test_simple_function(self):
        """Define and call a simple function."""
        result = interpret(
            "def add(a, b):\n    return a + b\nresult = add(2, 3)\n"
        )
        assert result.variables["result"] == 5

    def test_function_with_keyword_args(self):
        """Call a function using keyword arguments."""
        result = interpret(
            "def greet(name, prefix):\n"
            "    return prefix + \" \" + name\n"
            'result = greet(name="world", prefix="hello")\n'
        )
        assert result.variables["result"] == "hello world"

    def test_mixed_positional_and_keyword(self):
        """Mix positional and keyword arguments."""
        result = interpret(
            "def f(a, b, c):\n    return a * 100 + b * 10 + c\n"
            "result = f(1, c=3, b=2)\n"
        )
        assert result.variables["result"] == 123

    def test_recursive_function(self):
        """Test recursive function calls."""
        result = interpret(
            "def factorial(n):\n"
            "    if n <= 1:\n"
            "        return 1\n"
            "    return n * factorial(n - 1)\n"
            "result = factorial(5)\n"
        )
        assert result.variables["result"] == 120

    def test_nested_function_calls(self):
        """Test functions calling other functions."""
        result = interpret(
            "def double(x):\n    return x * 2\n"
            "def quadruple(x):\n    return double(double(x))\n"
            "result = quadruple(3)\n"
        )
        assert result.variables["result"] == 12


# =========================================================================
# Control Flow
# =========================================================================


class TestControlFlow:
    """Test if/elif/else and for loops."""

    def test_if_else(self):
        """Test if/else branching."""
        result = interpret(
            "x = 10\n"
            "if x > 5:\n"
            "    result = \"big\"\n"
            "else:\n"
            "    result = \"small\"\n"
        )
        assert result.variables["result"] == "big"

    def test_for_loop(self):
        """Test for loop with range()."""
        result = interpret(
            "total = 0\nfor i in range(5):\n    total = total + i\n"
        )
        assert result.variables["total"] == 10

    def test_for_loop_over_list(self):
        """Test for loop over a list."""
        result = interpret(
            "total = 0\nfor x in [10, 20, 30]:\n    total = total + x\n"
        )
        assert result.variables["total"] == 60


# =========================================================================
# Builtins
# =========================================================================


class TestBuiltins:
    """Test built-in functions are available."""

    def test_len(self):
        result = interpret("x = len([1, 2, 3])\n")
        assert result.variables["x"] == 3

    def test_range(self):
        result = interpret("x = range(5)\n")
        assert result.variables["x"] == [0, 1, 2, 3, 4]

    def test_type(self):
        result = interpret('x = type(42)\n')
        assert result.variables["x"] == "int"

    def test_str(self):
        result = interpret("x = str(42)\n")
        assert result.variables["x"] == "42"

    def test_sorted(self):
        result = interpret("x = sorted([3, 1, 2])\n")
        assert result.variables["x"] == [1, 2, 3]

    def test_min_max(self):
        result = interpret("a = min(3, 1, 2)\nb = max(3, 1, 2)\n")
        assert result.variables["a"] == 1
        assert result.variables["b"] == 3


# =========================================================================
# load() Function
# =========================================================================


class TestLoad:
    """Test the load() builtin for importing symbols from other files.

    This is the key feature that enables BUILD files to work. The load()
    function evaluates another Starlark file and injects requested symbols
    into the current scope.
    """

    def test_load_simple_function(self):
        """load() a file and use a function from it."""
        files = {
            "//math.star": "def double(n):\n    return n * 2\n",
        }
        result = interpret(
            'load("//math.star", "double")\n'
            "result = double(21)\n",
            file_resolver=files,
        )
        assert result.variables["result"] == 42

    def test_load_multiple_symbols(self):
        """load() multiple symbols from one file."""
        files = {
            "//utils.star": (
                "def add(a, b):\n    return a + b\n"
                "def mul(a, b):\n    return a * b\n"
            ),
        }
        result = interpret(
            'load("//utils.star", "add", "mul")\n'
            "x = add(2, 3)\ny = mul(4, 5)\n",
            file_resolver=files,
        )
        assert result.variables["x"] == 5
        assert result.variables["y"] == 20

    def test_load_variable(self):
        """load() a variable (not just functions)."""
        files = {
            "//config.star": "VERSION = 42\nNAME = \"mylib\"\n",
        }
        result = interpret(
            'load("//config.star", "VERSION", "NAME")\n',
            file_resolver=files,
        )
        assert result.variables["VERSION"] == 42
        assert result.variables["NAME"] == "mylib"

    def test_load_caching(self):
        """Verify that load() caches results — file is executed only once.

        If the file were executed twice, the counter would be 2.
        Since it's cached, subsequent loads reuse the first result.
        """
        files = {
            "//counter.star": "count = 1\ndef get_count():\n    return count\n",
        }
        result = interpret(
            'load("//counter.star", "get_count")\n'
            'load("//counter.star", "count")\n'
            "result = get_count()\n",
            file_resolver=files,
        )
        assert result.variables["result"] == 1
        assert result.variables["count"] == 1

    def test_load_build_file_style(self):
        """Simulate a BUILD file that loads rule definitions.

        This is the primary use case: a BUILD file loads py_library()
        from a rules file, then calls it with keyword arguments.
        """
        files = {
            "//rules/python.star": (
                "def py_library(name, deps):\n"
                "    return name + \":\" + str(len(deps))\n"
            ),
        }
        result = interpret(
            'load("//rules/python.star", "py_library")\n'
            'result = py_library(name="mylib", deps=["dep1", "dep2"])\n',
            file_resolver=files,
        )
        assert result.variables["result"] == "mylib:2"

    def test_load_chained(self):
        """Loaded files can themselves use load() (recursive loading)."""
        files = {
            "//base.star": "BASE = 10\n",
            "//derived.star": (
                'load("//base.star", "BASE")\n'
                "DERIVED = BASE * 2\n"
            ),
        }
        result = interpret(
            'load("//derived.star", "DERIVED")\n',
            file_resolver=files,
        )
        assert result.variables["DERIVED"] == 20

    def test_load_callable_resolver(self):
        """Test load() with a callable file resolver (not just dict)."""
        def resolver(label: str) -> str:
            if label == "//greet.star":
                return "def hello():\n    return \"hi\"\n"
            raise FileNotFoundError(f"Unknown: {label}")

        result = interpret(
            'load("//greet.star", "hello")\nresult = hello()\n',
            file_resolver=resolver,
        )
        assert result.variables["result"] == "hi"

    def test_load_missing_file_raises(self):
        """load() with unknown file raises FileNotFoundError."""
        with pytest.raises(FileNotFoundError, match="not found"):
            interpret(
                'load("//nonexistent.star", "foo")\n',
                file_resolver={},
            )

    def test_load_missing_symbol_raises(self):
        """load() with unknown symbol raises an error."""
        files = {"//lib.star": "x = 1\n"}
        with pytest.raises(Exception, match="Cannot import 'nonexistent'"):
            interpret(
                'load("//lib.star", "nonexistent")\n',
                file_resolver=files,
            )

    def test_load_no_resolver_raises(self):
        """load() without a file_resolver raises a clear error."""
        with pytest.raises(FileNotFoundError, match="no file_resolver"):
            interpret('load("//foo.star", "bar")\n')


# =========================================================================
# File Interpretation
# =========================================================================


class TestInterpretFile:
    """Test interpret_file() which reads from the filesystem."""

    def test_interpret_file(self):
        """Execute a Starlark file from the filesystem."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".star", delete=False
        ) as f:
            f.write("result = 1 + 2\n")
            f.flush()
            try:
                result = interpret_file(f.name)
                assert result.variables["result"] == 3
            finally:
                os.unlink(f.name)

    def test_interpret_file_with_load(self):
        """Execute a file that uses load()."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".star", delete=False
        ) as f:
            f.write(
                'load("//lib.star", "double")\n'
                "result = double(5)\n"
            )
            f.flush()
            try:
                files = {
                    "//lib.star": "def double(n):\n    return n * 2\n",
                }
                result = interpret_file(f.name, file_resolver=files)
                assert result.variables["result"] == 10
            finally:
                os.unlink(f.name)


# =========================================================================
# StarlarkInterpreter Class
# =========================================================================


class TestStarlarkInterpreter:
    """Test the StarlarkInterpreter class directly."""

    def test_shared_cache(self):
        """Multiple interpret() calls on the same interpreter share the cache."""
        files = {"//lib.star": "X = 42\n"}
        interp = StarlarkInterpreter(file_resolver=files)

        # First call loads the file
        result1 = interp.interpret('load("//lib.star", "X")\nresult = X\n')
        assert result1.variables["result"] == 42

        # Second call should use cached result
        assert "//lib.star" in interp._load_cache
        result2 = interp.interpret('load("//lib.star", "X")\nresult = X + 1\n')
        assert result2.variables["result"] == 43

    def test_separate_interpreters_separate_caches(self):
        """Different interpreter instances have independent caches."""
        files = {"//lib.star": "X = 1\n"}
        interp1 = StarlarkInterpreter(file_resolver=files)
        interp2 = StarlarkInterpreter(file_resolver=files)

        interp1.interpret('load("//lib.star", "X")\n')
        assert "//lib.star" in interp1._load_cache
        assert "//lib.star" not in interp2._load_cache


# =========================================================================
# Keyword Arguments with load() (BUILD file simulation)
# =========================================================================


class TestStarlarkInterpreterFile:
    """Test interpret_file on the StarlarkInterpreter class."""

    def test_interpret_file_class_method(self):
        """Use the class method for file interpretation."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".star", delete=False
        ) as f:
            f.write("x = 100\n")
            f.flush()
            try:
                interp = StarlarkInterpreter()
                result = interp.interpret_file(f.name)
                assert result.variables["x"] == 100
            finally:
                os.unlink(f.name)

    def test_interpret_file_no_trailing_newline(self):
        """File without trailing newline should still work."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".star", delete=False
        ) as f:
            f.write("x = 42")  # No trailing newline
            f.flush()
            try:
                interp = StarlarkInterpreter()
                result = interp.interpret_file(f.name)
                assert result.variables["x"] == 42
            finally:
                os.unlink(f.name)

    def test_interpret_file_with_load_class_method(self):
        """Use the class method with load() support."""
        files = {
            "//lib.star": "Y = 99\n",
        }
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".star", delete=False
        ) as f:
            f.write('load("//lib.star", "Y")\nresult = Y\n')
            f.flush()
            try:
                interp = StarlarkInterpreter(file_resolver=files)
                result = interp.interpret_file(f.name)
                assert result.variables["result"] == 99
            finally:
                os.unlink(f.name)


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_program(self):
        """An empty program should execute without error."""
        result = interpret("\n")
        assert result.variables == {} or len(result.variables) == 0

    def test_comments_only(self):
        """A program with only comments."""
        result = interpret("# this is a comment\n")
        assert len(result.output) == 0

    def test_comparison_operators(self):
        """Test comparison expressions."""
        result = interpret("a = 3 > 2\nb = 1 == 1\nc = 5 != 3\n")
        assert result.variables["a"] is True
        assert result.variables["b"] is True
        assert result.variables["c"] is True

    def test_string_multiplication(self):
        """Test string repetition."""
        result = interpret('x = "ab" * 3\n')
        assert result.variables["x"] == "ababab"

    def test_negative_number(self):
        """Test unary negation."""
        result = interpret("x = -42\n")
        assert result.variables["x"] == -42

    def test_nested_lists(self):
        """Test nested list structures."""
        result = interpret("x = [[1, 2], [3, 4]]\n")
        assert result.variables["x"] == [[1, 2], [3, 4]]


class TestBuildFileSimulation:
    """End-to-end tests simulating real BUILD file patterns.

    These tests verify the complete workflow:
    1. Define rules in .star files
    2. load() them in BUILD-style code
    3. Call rules with keyword arguments
    """

    def test_single_rule_call(self):
        """Simple BUILD file with one rule call."""
        rules = {
            "//rules/python.star": (
                "def py_library(name, deps):\n"
                "    return {\"rule\": \"py_library\", "
                "\"name\": name, \"deps\": deps}\n"
            ),
        }
        result = interpret(
            'load("//rules/python.star", "py_library")\n'
            'target = py_library(name="mylib", deps=["//other:lib"])\n',
            file_resolver=rules,
        )
        target = result.variables["target"]
        assert target["rule"] == "py_library"
        assert target["name"] == "mylib"
        assert target["deps"] == ["//other:lib"]

    def test_multiple_rule_calls(self):
        """BUILD file with multiple rule invocations."""
        rules = {
            "//rules/python.star": (
                "def py_library(name, deps):\n"
                "    return {\"name\": name, \"type\": \"lib\"}\n"
                "def py_test(name, deps):\n"
                "    return {\"name\": name, \"type\": \"test\"}\n"
            ),
        }
        result = interpret(
            'load("//rules/python.star", "py_library", "py_test")\n'
            'lib = py_library(name="core", deps=[])\n'
            'test = py_test(name="core_test", deps=["core"])\n',
            file_resolver=rules,
        )
        assert result.variables["lib"]["type"] == "lib"
        assert result.variables["test"]["type"] == "test"

    def test_multi_language_rules(self):
        """BUILD file loading rules from multiple language rule files."""
        rules = {
            "//rules/python.star": (
                "def py_library(name, deps):\n"
                "    return \"py:\" + name\n"
            ),
            "//rules/go.star": (
                "def go_library(name, deps):\n"
                "    return \"go:\" + name\n"
            ),
        }
        result = interpret(
            'load("//rules/python.star", "py_library")\n'
            'load("//rules/go.star", "go_library")\n'
            'py = py_library(name="mypy", deps=[])\n'
            'go = go_library(name="mygo", deps=[])\n',
            file_resolver=rules,
        )
        assert result.variables["py"] == "py:mypy"
        assert result.variables["go"] == "go:mygo"
