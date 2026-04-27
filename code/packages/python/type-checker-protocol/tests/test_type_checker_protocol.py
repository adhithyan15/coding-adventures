"""
Tests for type_checker_protocol.

We test:
  1. TypeErrorDiagnostic construction and immutability
  2. TypeCheckResult construction, .ok property, and immutability
  3. That a concrete mock class structurally satisfies the Protocol
  4. Duck typing (no explicit inheritance required)
  5. Edge cases: empty errors list, multiple errors, partial AST on failure
"""

from __future__ import annotations

import dataclasses
from dataclasses import FrozenInstanceError
from typing import runtime_checkable

import pytest

from type_checker_protocol import (
    GenericTypeChecker,
    TypeCheckResult,
    TypeChecker,
    TypeErrorDiagnostic,
)
from type_checker_protocol.protocol import ASTIn, ASTOut  # noqa: F401 — imported for annotation use


# ---------------------------------------------------------------------------
# Shared fixtures / helpers
# ---------------------------------------------------------------------------


@dataclasses.dataclass
class SimpleNode:
    """A minimal untyped AST node used in tests."""

    kind: str
    value: str = ""


@dataclasses.dataclass
class TypedNode:
    """A minimal typed AST node used in tests."""

    kind: str
    value: str = ""
    resolved_type: str = "unknown"


class GoodTypeChecker:
    """Concrete implementation that always succeeds.

    This class does NOT inherit from TypeChecker — it relies entirely on
    structural subtyping (duck typing).
    """

    def check(self, ast: SimpleNode) -> TypeCheckResult[TypedNode]:
        """Annotate the node with a trivial type."""
        typed = TypedNode(kind=ast.kind, value=ast.value, resolved_type="int")
        return TypeCheckResult(typed_ast=typed, errors=[])


class BadTypeChecker:
    """Concrete implementation that always reports one type error."""

    def check(self, ast: SimpleNode) -> TypeCheckResult[TypedNode]:
        """Return an error for every node."""
        err = TypeErrorDiagnostic(
            message=f"Unknown kind: {ast.kind}",
            line=1,
            column=1,
        )
        typed = TypedNode(kind=ast.kind, value=ast.value, resolved_type="error")
        return TypeCheckResult(typed_ast=typed, errors=[err])


class MultiErrorTypeChecker:
    """Concrete implementation that reports multiple errors."""

    def check(self, ast: SimpleNode) -> TypeCheckResult[TypedNode]:
        """Return two errors."""
        errors = [
            TypeErrorDiagnostic(message="First error", line=1, column=1),
            TypeErrorDiagnostic(message="Second error", line=2, column=5),
        ]
        typed = TypedNode(kind=ast.kind, value=ast.value, resolved_type="error")
        return TypeCheckResult(typed_ast=typed, errors=errors)


class RuleDrivenTypeChecker(GenericTypeChecker[SimpleNode]):
    """Concrete checker exercising the generic dispatch framework."""

    def __init__(self) -> None:
        super().__init__()
        self.register_hook("node", "literal", self._node_literal)
        self.register_hook("node", "broken", self._node_broken)

    def run(self, ast: SimpleNode) -> None:
        self.dispatch("node", ast, default=None)

    def node_kind(self, node: SimpleNode) -> str | None:
        return node.kind

    def locate(self, subject: object) -> tuple[int, int]:
        del subject
        return (7, 9)

    def _node_literal(self, node: SimpleNode) -> None:
        node.value = "checked"

    def _node_broken(self, node: SimpleNode) -> None:
        self._error(f"bad node: {node.kind}", node)


# ---------------------------------------------------------------------------
# TypeErrorDiagnostic tests
# ---------------------------------------------------------------------------


class TestTypeErrorDiagnostic:
    """Tests for the TypeErrorDiagnostic frozen dataclass."""

    def test_construction_with_all_fields(self) -> None:
        """Can construct a diagnostic with message, line, and column."""
        diag = TypeErrorDiagnostic(message="Type mismatch", line=3, column=7)
        assert diag.message == "Type mismatch"
        assert diag.line == 3
        assert diag.column == 7

    def test_construction_with_line_one(self) -> None:
        """Line 1 column 1 is a valid location (first token in source)."""
        diag = TypeErrorDiagnostic(message="Error at start", line=1, column=1)
        assert diag.line == 1
        assert diag.column == 1

    def test_frozen_message(self) -> None:
        """Mutating message raises FrozenInstanceError."""
        diag = TypeErrorDiagnostic(message="Immutable", line=1, column=1)
        with pytest.raises(FrozenInstanceError):
            diag.message = "Changed"  # type: ignore[misc]

    def test_frozen_line(self) -> None:
        """Mutating line raises FrozenInstanceError."""
        diag = TypeErrorDiagnostic(message="Immutable", line=5, column=2)
        with pytest.raises(FrozenInstanceError):
            diag.line = 99  # type: ignore[misc]

    def test_frozen_column(self) -> None:
        """Mutating column raises FrozenInstanceError."""
        diag = TypeErrorDiagnostic(message="Immutable", line=5, column=2)
        with pytest.raises(FrozenInstanceError):
            diag.column = 99  # type: ignore[misc]

    def test_equality(self) -> None:
        """Two diagnostics with the same fields are equal (dataclass __eq__)."""
        a = TypeErrorDiagnostic(message="msg", line=1, column=1)
        b = TypeErrorDiagnostic(message="msg", line=1, column=1)
        assert a == b

    def test_inequality_different_line(self) -> None:
        """Diagnostics with different line numbers are not equal."""
        a = TypeErrorDiagnostic(message="msg", line=1, column=1)
        b = TypeErrorDiagnostic(message="msg", line=2, column=1)
        assert a != b

    def test_inequality_different_message(self) -> None:
        """Diagnostics with different messages are not equal."""
        a = TypeErrorDiagnostic(message="error A", line=1, column=1)
        b = TypeErrorDiagnostic(message="error B", line=1, column=1)
        assert a != b

    def test_hashable(self) -> None:
        """Frozen dataclasses are hashable — can be used in sets and as dict keys."""
        diag = TypeErrorDiagnostic(message="msg", line=1, column=1)
        s = {diag}
        assert diag in s

    def test_repr_contains_message(self) -> None:
        """repr() should mention the message so it shows up in test failure output."""
        diag = TypeErrorDiagnostic(message="Something broke", line=3, column=4)
        assert "Something broke" in repr(diag)


# ---------------------------------------------------------------------------
# TypeCheckResult tests
# ---------------------------------------------------------------------------


class TestTypeCheckResult:
    """Tests for the TypeCheckResult frozen dataclass."""

    def test_ok_when_no_errors(self) -> None:
        """ok is True when errors list is empty."""
        node = TypedNode(kind="int")
        result = TypeCheckResult(typed_ast=node, errors=[])
        assert result.ok is True

    def test_not_ok_when_errors_present(self) -> None:
        """ok is False when there is at least one error."""
        node = TypedNode(kind="bad")
        err = TypeErrorDiagnostic(message="error", line=1, column=1)
        result = TypeCheckResult(typed_ast=node, errors=[err])
        assert result.ok is False

    def test_ok_with_default_empty_errors(self) -> None:
        """errors defaults to empty list; ok is True."""
        node = TypedNode(kind="int")
        result = TypeCheckResult(typed_ast=node)
        assert result.ok is True

    def test_typed_ast_accessible(self) -> None:
        """typed_ast is accessible after construction."""
        node = TypedNode(kind="string", resolved_type="str")
        result = TypeCheckResult(typed_ast=node, errors=[])
        assert result.typed_ast.resolved_type == "str"

    def test_partial_ast_with_errors(self) -> None:
        """typed_ast is accessible even when errors are present (partial annotation)."""
        node = TypedNode(kind="broken", resolved_type="error")
        err = TypeErrorDiagnostic(message="bad type", line=2, column=3)
        result = TypeCheckResult(typed_ast=node, errors=[err])
        assert result.ok is False
        assert result.typed_ast.resolved_type == "error"
        assert result.errors[0].line == 2

    def test_multiple_errors(self) -> None:
        """errors list can hold multiple diagnostics."""
        node = TypedNode(kind="broken")
        errors = [
            TypeErrorDiagnostic(message="error 1", line=1, column=1),
            TypeErrorDiagnostic(message="error 2", line=5, column=10),
            TypeErrorDiagnostic(message="error 3", line=10, column=3),
        ]
        result = TypeCheckResult(typed_ast=node, errors=errors)
        assert result.ok is False
        assert len(result.errors) == 3

    def test_frozen_typed_ast(self) -> None:
        """Mutating typed_ast raises FrozenInstanceError."""
        node = TypedNode(kind="int")
        result = TypeCheckResult(typed_ast=node, errors=[])
        with pytest.raises(FrozenInstanceError):
            result.typed_ast = TypedNode(kind="str")  # type: ignore[misc]

    def test_frozen_errors(self) -> None:
        """Mutating errors raises FrozenInstanceError."""
        node = TypedNode(kind="int")
        result = TypeCheckResult(typed_ast=node, errors=[])
        with pytest.raises(FrozenInstanceError):
            result.errors = []  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Protocol structural subtyping tests
# ---------------------------------------------------------------------------


class TestProtocolStructuralSubtyping:
    """Verify that concrete implementations satisfy the TypeChecker Protocol."""

    def test_good_checker_satisfies_protocol(self) -> None:
        """GoodTypeChecker satisfies TypeChecker without inheriting from it."""
        # If GoodTypeChecker didn't satisfy the protocol, this type-annotation
        # would be wrong (mypy would catch it).  At runtime we verify the
        # duck-typed call works correctly.
        checker: TypeChecker[SimpleNode, TypedNode] = GoodTypeChecker()
        node = SimpleNode(kind="literal", value="42")
        result = checker.check(node)
        assert result.ok is True
        assert result.typed_ast.resolved_type == "int"

    def test_bad_checker_satisfies_protocol(self) -> None:
        """BadTypeChecker satisfies TypeChecker and returns errors correctly."""
        checker: TypeChecker[SimpleNode, TypedNode] = BadTypeChecker()
        node = SimpleNode(kind="??")
        result = checker.check(node)
        assert result.ok is False
        assert len(result.errors) == 1
        assert "??" in result.errors[0].message

    def test_multi_error_checker_satisfies_protocol(self) -> None:
        """MultiErrorTypeChecker satisfies TypeChecker with multiple errors."""
        checker: TypeChecker[SimpleNode, TypedNode] = MultiErrorTypeChecker()
        node = SimpleNode(kind="bad")
        result = checker.check(node)
        assert result.ok is False
        assert len(result.errors) == 2

    def test_no_inheritance_required(self) -> None:
        """The concrete checker does not inherit from TypeChecker at all."""
        assert TypeChecker not in GoodTypeChecker.__mro__
        assert TypeChecker not in BadTypeChecker.__mro__

    def test_duck_typing_with_inline_class(self) -> None:
        """An anonymous inline class satisfies the protocol (duck typing)."""

        class InlineChecker:
            def check(self, ast: SimpleNode) -> TypeCheckResult[TypedNode]:
                return TypeCheckResult(
                    typed_ast=TypedNode(kind=ast.kind, resolved_type="bool"),
                    errors=[],
                )

        checker: TypeChecker[SimpleNode, TypedNode] = InlineChecker()
        result = checker.check(SimpleNode(kind="bool_expr"))
        assert result.ok is True
        assert result.typed_ast.resolved_type == "bool"

    def test_pipeline_function_accepts_any_conforming_checker(self) -> None:
        """A function typed with TypeChecker[In, Out] accepts any conforming impl."""

        def run_check(
            checker: TypeChecker[SimpleNode, TypedNode],
            ast: SimpleNode,
        ) -> TypeCheckResult[TypedNode]:
            return checker.check(ast)

        good_result = run_check(GoodTypeChecker(), SimpleNode(kind="x"))
        bad_result = run_check(BadTypeChecker(), SimpleNode(kind="x"))
        assert good_result.ok is True
        assert bad_result.ok is False

    def test_result_errors_are_type_error_diagnostics(self) -> None:
        """Every item in result.errors is a TypeErrorDiagnostic instance."""
        checker = MultiErrorTypeChecker()
        result = checker.check(SimpleNode(kind="x"))
        for err in result.errors:
            assert isinstance(err, TypeErrorDiagnostic)

    def test_result_errors_order_preserved(self) -> None:
        """Errors are returned in the order the checker added them."""
        checker = MultiErrorTypeChecker()
        result = checker.check(SimpleNode(kind="x"))
        assert result.errors[0].message == "First error"
        assert result.errors[1].message == "Second error"
        assert result.errors[0].line < result.errors[1].line


# ---------------------------------------------------------------------------
# GenericTypeChecker framework tests
# ---------------------------------------------------------------------------


class TestGenericTypeChecker:
    """Verify the reusable dispatch-based checker framework."""

    def test_registered_hook_can_mutate_typed_ast(self) -> None:
        node = SimpleNode(kind="literal", value="before")
        result = RuleDrivenTypeChecker().check(node)
        assert result.ok is True
        assert result.typed_ast.value == "checked"

    def test_hook_can_report_errors_via_shared_diagnostic_api(self) -> None:
        node = SimpleNode(kind="broken")
        result = RuleDrivenTypeChecker().check(node)
        assert result.ok is False
        assert result.errors[0] == TypeErrorDiagnostic(
            message="bad node: broken",
            line=7,
            column=9,
        )

    def test_unhandled_node_kind_falls_through_cleanly(self) -> None:
        node = SimpleNode(kind="unknown", value="unchanged")
        result = RuleDrivenTypeChecker().check(node)
        assert result.ok is True
        assert result.typed_ast.value == "unchanged"


# ---------------------------------------------------------------------------
# Import / public API tests
# ---------------------------------------------------------------------------


class TestPublicAPI:
    """Verify the public API exported from __init__.py."""

    def test_type_checker_importable_from_package(self) -> None:
        """TypeChecker is importable from the top-level package."""
        from type_checker_protocol import TypeChecker as TC  # noqa: F401

        assert TC is TypeChecker

    def test_generic_type_checker_importable_from_package(self) -> None:
        """GenericTypeChecker is importable from the top-level package."""
        from type_checker_protocol import GenericTypeChecker as GTC  # noqa: F401

        assert GTC is GenericTypeChecker

    def test_type_check_result_importable_from_package(self) -> None:
        """TypeCheckResult is importable from the top-level package."""
        from type_checker_protocol import TypeCheckResult as TCR  # noqa: F401

        assert TCR is TypeCheckResult

    def test_type_error_diagnostic_importable_from_package(self) -> None:
        """TypeErrorDiagnostic is importable from the top-level package."""
        from type_checker_protocol import TypeErrorDiagnostic as TED  # noqa: F401

        assert TED is TypeErrorDiagnostic

    def test_all_exports_present(self) -> None:
        """__all__ contains the public protocol and framework names."""
        import type_checker_protocol as pkg

        assert set(pkg.__all__) == {
            "GenericTypeChecker",
            "TypeChecker",
            "TypeCheckResult",
            "TypeErrorDiagnostic",
        }
