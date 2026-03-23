"""Tests for Lattice error types.

Each error type is tested for:
1. Correct message formatting
2. Line/column storage
3. Inheritance from LatticeError
4. Additional attributes specific to each error type
"""

from __future__ import annotations

from lattice_ast_to_css.errors import (
    CircularReferenceError,
    LatticeError,
    MissingReturnError,
    ModuleNotFoundError,
    ReturnOutsideFunctionError,
    TypeErrorInExpression,
    UndefinedFunctionError,
    UndefinedMixinError,
    UndefinedVariableError,
    UnitMismatchError,
    WrongArityError,
)


class TestLatticeError:
    """Test the base LatticeError class."""

    def test_message(self) -> None:
        err = LatticeError("something went wrong")
        assert err.message == "something went wrong"

    def test_str_without_location(self) -> None:
        err = LatticeError("oops")
        assert str(err) == "oops"

    def test_str_with_location(self) -> None:
        err = LatticeError("oops", line=5, column=10)
        assert str(err) == "oops at line 5, column 10"

    def test_line_column(self) -> None:
        err = LatticeError("msg", line=3, column=7)
        assert err.line == 3
        assert err.column == 7

    def test_is_exception(self) -> None:
        assert issubclass(LatticeError, Exception)


class TestModuleNotFoundError:
    """Test module not found errors from @use resolution."""

    def test_message(self) -> None:
        err = ModuleNotFoundError("colors", line=1, column=6)
        assert "colors" in str(err)
        assert "not found" in str(err)

    def test_module_name_attribute(self) -> None:
        err = ModuleNotFoundError("utils/mixins")
        assert err.module_name == "utils/mixins"

    def test_inherits_lattice_error(self) -> None:
        assert issubclass(ModuleNotFoundError, LatticeError)


class TestReturnOutsideFunctionError:
    """Test @return outside function errors."""

    def test_message(self) -> None:
        err = ReturnOutsideFunctionError(line=10, column=5)
        assert "@return" in str(err)
        assert "outside" in str(err)

    def test_inherits_lattice_error(self) -> None:
        assert issubclass(ReturnOutsideFunctionError, LatticeError)


class TestUndefinedVariableError:
    """Test undefined variable errors."""

    def test_message(self) -> None:
        err = UndefinedVariableError("$foo", line=12, column=15)
        assert "$foo" in str(err)
        assert "Undefined variable" in str(err)

    def test_name_attribute(self) -> None:
        err = UndefinedVariableError("$color")
        assert err.name == "$color"

    def test_inherits_lattice_error(self) -> None:
        assert issubclass(UndefinedVariableError, LatticeError)


class TestUndefinedMixinError:
    """Test undefined mixin errors."""

    def test_message(self) -> None:
        err = UndefinedMixinError("button", line=15, column=14)
        assert "button" in str(err)
        assert "Undefined mixin" in str(err)

    def test_name_attribute(self) -> None:
        err = UndefinedMixinError("clearfix")
        assert err.name == "clearfix"


class TestUndefinedFunctionError:
    """Test undefined function errors."""

    def test_message(self) -> None:
        err = UndefinedFunctionError("spacing", line=20, column=12)
        assert "spacing" in str(err)
        assert "Undefined function" in str(err)

    def test_name_attribute(self) -> None:
        err = UndefinedFunctionError("double")
        assert err.name == "double"


class TestWrongArityError:
    """Test wrong argument count errors."""

    def test_message(self) -> None:
        err = WrongArityError("Mixin", "button", expected=2, got=3, line=15)
        assert "button" in str(err)
        assert "2" in str(err)
        assert "3" in str(err)

    def test_attributes(self) -> None:
        err = WrongArityError("Function", "spacing", expected=1, got=0)
        assert err.name == "spacing"
        assert err.expected == 1
        assert err.got == 0


class TestCircularReferenceError:
    """Test circular reference errors."""

    def test_message(self) -> None:
        err = CircularReferenceError("mixin", ["a", "b", "a"], line=8)
        assert "a → b → a" in str(err)
        assert "Circular mixin" in str(err)

    def test_chain_attribute(self) -> None:
        err = CircularReferenceError("function", ["f", "g", "f"])
        assert err.chain == ["f", "g", "f"]


class TestTypeErrorInExpression:
    """Test type error in expression errors."""

    def test_message(self) -> None:
        err = TypeErrorInExpression("add", "10px", "red", line=5)
        assert "10px" in str(err)
        assert "red" in str(err)
        assert "add" in str(err)

    def test_attributes(self) -> None:
        err = TypeErrorInExpression("multiply", "string", "number")
        assert err.op == "multiply"
        assert err.left_type == "string"
        assert err.right_type == "number"


class TestUnitMismatchError:
    """Test unit mismatch errors."""

    def test_message(self) -> None:
        err = UnitMismatchError("px", "s", line=7)
        assert "px" in str(err)
        assert "s" in str(err)

    def test_attributes(self) -> None:
        err = UnitMismatchError("em", "vh")
        assert err.left_unit == "em"
        assert err.right_unit == "vh"


class TestMissingReturnError:
    """Test missing @return errors."""

    def test_message(self) -> None:
        err = MissingReturnError("double", line=25)
        assert "double" in str(err)
        assert "@return" in str(err)

    def test_name_attribute(self) -> None:
        err = MissingReturnError("spacing")
        assert err.name == "spacing"


class TestCatchAll:
    """Test that all errors can be caught with except LatticeError."""

    def test_catch_all_errors(self) -> None:
        """Every error subclass is catchable as LatticeError."""
        errors = [
            ModuleNotFoundError("x"),
            ReturnOutsideFunctionError(),
            UndefinedVariableError("$x"),
            UndefinedMixinError("x"),
            UndefinedFunctionError("x"),
            WrongArityError("Mixin", "x", 1, 2),
            CircularReferenceError("mixin", ["a", "a"]),
            TypeErrorInExpression("add", "a", "b"),
            UnitMismatchError("px", "s"),
            MissingReturnError("x"),
        ]
        for err in errors:
            assert isinstance(err, LatticeError), f"{type(err).__name__} not a LatticeError"
