"""Tests for the GenericCompiler — the pluggable AST-to-bytecode framework.

These tests verify that the GenericCompiler correctly:
1. Dispatches to registered rule handlers
2. Passes through single-child nodes without handlers
3. Manages constant and name pools with deduplication
4. Handles jump emission and patching
5. Manages scopes for local variables
6. Raises errors for unhandled multi-child rules
"""

from __future__ import annotations

import pytest

from lang_parser import ASTNode
from lexer import Token

from bytecode_compiler import GenericCompiler, CompilerError, UnhandledRuleError, CompilerScope


# =========================================================================
# Helpers
# =========================================================================


def make_token(type_name: str, value: str) -> Token:
    """Create a token for testing."""
    return Token(type=type_name, value=value, line=1, column=1)


def make_node(rule_name: str, children: list) -> ASTNode:
    """Create an AST node for testing."""
    return ASTNode(rule_name=rule_name, children=children)


# =========================================================================
# Test: Instruction Emission
# =========================================================================


class TestEmission:
    """Test instruction emission basics."""

    def test_emit_instruction(self):
        """Emit adds instruction to list."""
        compiler = GenericCompiler()
        idx = compiler.emit(0x01, 0)
        assert idx == 0
        assert len(compiler.instructions) == 1
        assert compiler.instructions[0].opcode == 0x01
        assert compiler.instructions[0].operand == 0

    def test_emit_returns_index(self):
        """Emit returns the index of the emitted instruction."""
        compiler = GenericCompiler()
        assert compiler.emit(0x01) == 0
        assert compiler.emit(0x02) == 1
        assert compiler.emit(0x03) == 2

    def test_current_offset(self):
        """Current offset tracks the next instruction index."""
        compiler = GenericCompiler()
        assert compiler.current_offset == 0
        compiler.emit(0x01)
        assert compiler.current_offset == 1
        compiler.emit(0x02)
        assert compiler.current_offset == 2


# =========================================================================
# Test: Constant Pool
# =========================================================================


class TestConstantPool:
    """Test constant pool management."""

    def test_add_constant(self):
        compiler = GenericCompiler()
        idx = compiler.add_constant(42)
        assert idx == 0
        assert compiler.constants == [42]

    def test_deduplicate_constant(self):
        compiler = GenericCompiler()
        idx1 = compiler.add_constant(42)
        idx2 = compiler.add_constant(42)
        assert idx1 == idx2
        assert len(compiler.constants) == 1

    def test_different_constants_get_different_indices(self):
        compiler = GenericCompiler()
        idx1 = compiler.add_constant(42)
        idx2 = compiler.add_constant(99)
        assert idx1 == 0
        assert idx2 == 1
        assert compiler.constants == [42, 99]

    def test_deduplicate_by_type(self):
        """True (bool) and 1 (int) should be separate constants."""
        compiler = GenericCompiler()
        idx_true = compiler.add_constant(True)
        idx_one = compiler.add_constant(1)
        # These should be separate because bool is not int in Starlark
        assert idx_true != idx_one


# =========================================================================
# Test: Name Pool
# =========================================================================


class TestNamePool:
    """Test name pool management."""

    def test_add_name(self):
        compiler = GenericCompiler()
        idx = compiler.add_name("x")
        assert idx == 0
        assert compiler.names == ["x"]

    def test_deduplicate_name(self):
        compiler = GenericCompiler()
        idx1 = compiler.add_name("x")
        idx2 = compiler.add_name("x")
        assert idx1 == idx2
        assert len(compiler.names) == 1

    def test_different_names(self):
        compiler = GenericCompiler()
        idx1 = compiler.add_name("x")
        idx2 = compiler.add_name("y")
        assert idx1 == 0
        assert idx2 == 1


# =========================================================================
# Test: Jump Patching
# =========================================================================


class TestJumpPatching:
    """Test jump emission and backpatching."""

    def test_emit_jump_placeholder(self):
        """emit_jump creates a placeholder with operand 0."""
        compiler = GenericCompiler()
        idx = compiler.emit_jump(0x40)
        assert compiler.instructions[idx].operand == 0

    def test_patch_jump_explicit_target(self):
        """patch_jump with explicit target."""
        compiler = GenericCompiler()
        idx = compiler.emit_jump(0x40)
        compiler.emit(0x01)  # some filler
        compiler.emit(0x02)  # some filler
        compiler.patch_jump(idx, 5)
        assert compiler.instructions[idx].operand == 5

    def test_patch_jump_current_offset(self):
        """patch_jump without target uses current offset."""
        compiler = GenericCompiler()
        idx = compiler.emit_jump(0x40)
        compiler.emit(0x01)
        compiler.emit(0x02)
        compiler.patch_jump(idx)  # Should patch to offset 3
        assert compiler.instructions[idx].operand == 3


# =========================================================================
# Test: Rule Dispatch
# =========================================================================


class TestRuleDispatch:
    """Test AST node dispatch to registered handlers."""

    def test_dispatch_to_handler(self):
        """Registered handler is called for matching rule_name."""
        compiler = GenericCompiler()
        called_with = []

        def my_handler(c, node):
            called_with.append(node.rule_name)
            c.emit(0x42)

        compiler.register_rule("my_rule", my_handler)
        node = make_node("my_rule", [make_token("NAME", "x")])
        compiler.compile_node(node)

        assert called_with == ["my_rule"]
        assert len(compiler.instructions) == 1
        assert compiler.instructions[0].opcode == 0x42

    def test_pass_through_single_child(self):
        """Single-child node without handler passes through."""
        compiler = GenericCompiler()

        def atom_handler(c, node):
            c.emit(0x01, 0)

        compiler.register_rule("atom", atom_handler)

        # Create a chain: expression -> or_expr -> atom
        atom = make_node("atom", [make_token("INT", "42")])
        or_expr = make_node("or_expr", [atom])
        expression = make_node("expression", [or_expr])

        compiler.compile_node(expression)
        assert len(compiler.instructions) == 1
        assert compiler.instructions[0].opcode == 0x01

    def test_unhandled_multi_child_raises(self):
        """Multi-child node without handler raises UnhandledRuleError."""
        compiler = GenericCompiler()
        node = make_node("unknown_rule", [
            make_token("NAME", "a"),
            make_token("PLUS", "+"),
            make_token("NAME", "b"),
        ])
        with pytest.raises(UnhandledRuleError, match="unknown_rule"):
            compiler.compile_node(node)

    def test_bare_token_ignored(self):
        """Bare tokens (NEWLINE, etc.) are ignored by default."""
        compiler = GenericCompiler()
        token = make_token("NEWLINE", "\n")
        compiler.compile_node(token)
        assert len(compiler.instructions) == 0


# =========================================================================
# Test: Scope Management
# =========================================================================


class TestScopeManagement:
    """Test scope tracking for local variables."""

    def test_enter_and_exit_scope(self):
        compiler = GenericCompiler()
        assert compiler.scope is None
        scope = compiler.enter_scope(["x", "y"])
        assert compiler.scope is scope
        assert scope.get_local("x") == 0
        assert scope.get_local("y") == 1
        exited = compiler.exit_scope()
        assert exited is scope
        assert compiler.scope is None

    def test_nested_scopes(self):
        compiler = GenericCompiler()
        outer = compiler.enter_scope(["a"])
        inner = compiler.enter_scope(["b"])
        assert inner.parent is outer
        assert inner.get_local("b") == 0
        assert inner.get_local("a") is None  # Not in inner scope
        compiler.exit_scope()
        assert compiler.scope is outer
        compiler.exit_scope()
        assert compiler.scope is None

    def test_exit_scope_when_none_raises(self):
        compiler = GenericCompiler()
        with pytest.raises(CompilerError):
            compiler.exit_scope()


# =========================================================================
# Test: CompilerScope
# =========================================================================


class TestCompilerScope:
    """Test CompilerScope directly."""

    def test_add_local(self):
        scope = CompilerScope()
        assert scope.add_local("x") == 0
        assert scope.add_local("y") == 1
        assert scope.num_locals == 2

    def test_add_local_dedup(self):
        scope = CompilerScope()
        assert scope.add_local("x") == 0
        assert scope.add_local("x") == 0  # Same index
        assert scope.num_locals == 1

    def test_get_local_missing(self):
        scope = CompilerScope()
        assert scope.get_local("x") is None


# =========================================================================
# Test: Compile (top-level API)
# =========================================================================


class TestCompile:
    """Test the compile() method."""

    def test_compile_adds_halt(self):
        """compile() appends HALT at the end."""
        compiler = GenericCompiler()

        def file_handler(c, node):
            c.emit(0x01, 0)

        compiler.register_rule("file", file_handler)
        node = make_node("file", [])
        code = compiler.compile(node, halt_opcode=0xFF)

        assert len(code.instructions) == 2
        assert code.instructions[-1].opcode == 0xFF

    def test_compile_returns_code_object(self):
        compiler = GenericCompiler()
        compiler.register_rule("file", lambda c, n: None)
        code = compiler.compile(make_node("file", []))
        assert code.instructions is not None
        assert code.constants is not None
        assert code.names is not None
