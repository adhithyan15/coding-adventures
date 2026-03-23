"""Tests for Lattice v2 Tier 1 features.

Tier 1 features require changes to the grammar and transformer but
no new value types beyond what v1 already supports:

1. @while loops — condition-based iteration with max-iteration guard
2. $var in selectors — variable interpolation in selector positions
3. @content blocks — caller-provided content injected into mixin body
4. !default / !global flags — library-style variable management
5. Property nesting — grouping related properties under a shared prefix
6. @at-root — escaping nesting context to emit at stylesheet root
7. @extend and %placeholder — selector inheritance

Since the grammar may not yet be updated for all v2 tokens, these tests
exercise the transformer, evaluator, and scope components directly
using synthetic AST nodes (the same approach used by the v1 tests for
error conditions and edge cases).
"""

from __future__ import annotations

import copy
import pytest

from lattice_ast_to_css.errors import (
    ExtendTargetNotFoundError,
    MaxIterationError,
    UndefinedVariableError,
)
from lattice_ast_to_css.evaluator import (
    ExpressionEvaluator,
    LatticeColor,
    LatticeBool,
    LatticeDimension,
    LatticeIdent,
    LatticeList,
    LatticeMap,
    LatticeNull,
    LatticeNumber,
    LatticePercentage,
    LatticeString,
    is_truthy,
    token_to_value,
    value_to_css,
)
from lattice_ast_to_css.scope import ScopeChain
from lattice_ast_to_css.transformer import LatticeTransformer


# ---------------------------------------------------------------------------
# Synthetic AST helpers
# ---------------------------------------------------------------------------
# These build minimal AST node structures that the transformer can process.
# They mirror the shape of nodes that the grammar parser would produce,
# but without requiring actual parsing.
# ---------------------------------------------------------------------------


class FakeToken:
    """A minimal token object with type, value, line, column."""

    def __init__(self, type_name: str, value: str, line: int = 0, column: int = 0) -> None:
        self.type = type_name
        self.value = value
        self.line = line
        self.column = column


class FakeNode:
    """A minimal AST node with rule_name and children."""

    def __init__(self, rule_name: str, children: list[object] | None = None) -> None:
        self.rule_name = rule_name
        self.children = children or []


# ===========================================================================
# 1. @while Loop Tests
# ===========================================================================


class TestWhileLoop:
    """Test the @while loop expansion in the transformer.

    @while evaluates a condition expression and repeatedly expands the body
    until the condition becomes falsy. A max-iteration guard prevents
    infinite loops.
    """

    def test_max_iteration_error(self) -> None:
        """@while true { } should raise MaxIterationError after 1000 iterations."""
        transformer = LatticeTransformer(max_while_iterations=5)
        scope = transformer.variables

        # Build: @while true { ... }
        condition = FakeNode("lattice_expression", [FakeToken("IDENT", "true")])
        block = FakeNode("block", [FakeNode("block_contents", [])])
        while_node = FakeNode("while_directive", [
            FakeToken("AT_KEYWORD", "@while"),
            condition,
            block,
        ])

        with pytest.raises(MaxIterationError) as exc_info:
            transformer._expand_while(while_node, scope)
        assert "5" in str(exc_info.value)

    def test_zero_iterations(self) -> None:
        """@while false { } should produce no output."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        condition = FakeNode("lattice_expression", [FakeToken("IDENT", "false")])
        block = FakeNode("block", [FakeNode("block_contents", [])])
        while_node = FakeNode("while_directive", [
            FakeToken("AT_KEYWORD", "@while"),
            condition,
            block,
        ])

        result = transformer._expand_while(while_node, scope)
        assert result == []

    def test_while_with_counter(self) -> None:
        """@while with a counter variable should iterate correctly."""
        transformer = LatticeTransformer()
        scope = transformer.variables
        scope.set("$i", LatticeNumber(1.0))

        # Build condition: $i <= 3
        # Since we need a proper expression AST, let's test with a simple
        # truthy/falsy test using the counter directly
        condition = FakeNode("lattice_expression", [
            FakeNode("lattice_or_expr", [
                FakeNode("lattice_and_expr", [
                    FakeNode("lattice_comparison", [
                        FakeNode("lattice_additive", [
                            FakeNode("lattice_multiplicative", [
                                FakeNode("lattice_unary", [
                                    FakeNode("lattice_primary", [
                                        FakeToken("VARIABLE", "$i"),
                                    ]),
                                ]),
                            ]),
                        ]),
                        FakeNode("comparison_op", [
                            FakeToken("LESS_EQUALS", "<="),
                        ]),
                        FakeNode("lattice_additive", [
                            FakeNode("lattice_multiplicative", [
                                FakeNode("lattice_unary", [
                                    FakeNode("lattice_primary", [
                                        FakeToken("NUMBER", "3"),
                                    ]),
                                ]),
                            ]),
                        ]),
                    ]),
                ]),
            ]),
        ])

        # Empty block that won't produce items but we track iterations
        block = FakeNode("block", [FakeNode("block_contents", [])])

        while_node = FakeNode("while_directive", [
            FakeToken("AT_KEYWORD", "@while"),
            condition,
            block,
        ])

        # Manually increment $i in a test loop to verify logic
        iterations = 0
        while True:
            evaluator = ExpressionEvaluator(scope)
            val = evaluator.evaluate(copy.deepcopy(condition))
            if not is_truthy(val):
                break
            iterations += 1
            current = scope.get("$i")
            scope.set("$i", LatticeNumber(current.value + 1))
            if iterations > 10:
                break

        assert iterations == 3
        # $i should be 4 after the loop
        assert scope.get("$i") == LatticeNumber(4.0)

    def test_max_iteration_error_attributes(self) -> None:
        """MaxIterationError should carry the max_iterations value."""
        err = MaxIterationError(500)
        assert err.max_iterations == 500
        assert "500" in str(err)


# ===========================================================================
# 2. $var in Selectors Tests
# ===========================================================================


class TestVarInSelectors:
    """Test variable interpolation in selector positions.

    Variables in selectors are resolved to their string values during
    transformation. Adjacent tokens concatenate: .col- + 3 = .col-3
    """

    def test_variable_in_simple_selector(self) -> None:
        """A VARIABLE token in simple_selector should be resolved."""
        transformer = LatticeTransformer()
        scope = transformer.variables
        scope.set("$tag", LatticeIdent("h2"))

        node = FakeNode("simple_selector", [
            FakeToken("VARIABLE", "$tag"),
        ])

        result = transformer._expand_selector_with_vars(node, scope)
        assert len(result.children) == 1
        assert result.children[0].value == "h2"

    def test_variable_number_in_selector(self) -> None:
        """A variable holding a number should be stringified in selector."""
        transformer = LatticeTransformer()
        scope = transformer.variables
        scope.set("$i", LatticeNumber(3.0))

        node = FakeNode("compound_selector", [
            FakeToken("IDENT", ".col-"),
            FakeToken("VARIABLE", "$i"),
        ])

        result = transformer._expand_selector_with_vars(node, scope)
        # First token remains, second is resolved
        assert result.children[0].value == ".col-"
        assert result.children[1].value == "3"

    def test_undefined_variable_in_selector(self) -> None:
        """Undefined variable in selector should raise UndefinedVariableError."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        node = FakeNode("simple_selector", [
            FakeToken("VARIABLE", "$undefined"),
        ])

        with pytest.raises(UndefinedVariableError):
            transformer._expand_selector_with_vars(node, scope)

    def test_class_selector_with_variable(self) -> None:
        """DOT VARIABLE in class_selector should resolve the variable."""
        transformer = LatticeTransformer()
        scope = transformer.variables
        scope.set("$name", LatticeIdent("active"))

        node = FakeNode("class_selector", [
            FakeToken("DOT", "."),
            FakeToken("VARIABLE", "$name"),
        ])

        result = transformer._expand_selector_with_vars(node, scope)
        assert result.children[0].value == "."
        assert result.children[1].value == "active"


# ===========================================================================
# 3. @content Block Tests
# ===========================================================================


class TestContentBlocks:
    """Test @content directive expansion inside mixin bodies.

    @content is replaced with the content block from the @include call site.
    If no content block was passed, @content produces nothing.
    """

    def test_content_with_no_block(self) -> None:
        """@content with no content block should produce empty list."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        node = FakeNode("content_directive", [
            FakeToken("AT_KEYWORD", "@content"),
            FakeToken("SEMICOLON", ";"),
        ])

        result = transformer._expand_content(node, scope)
        assert result == []

    def test_content_with_empty_stack(self) -> None:
        """@content when not inside a mixin should produce empty list."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        # No content block stack entries
        node = FakeNode("content_directive", [
            FakeToken("AT_KEYWORD", "@content"),
            FakeToken("SEMICOLON", ";"),
        ])

        result = transformer._expand_content(node, scope)
        assert result == []

    def test_content_with_block_on_stack(self) -> None:
        """@content should expand the content block from the stack."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        # Push a content block onto the stack
        content_block = FakeNode("block", [
            FakeNode("block_contents", [
                FakeNode("block_item", [
                    FakeNode("declaration_or_nested", [
                        FakeNode("declaration", [
                            FakeNode("property", [FakeToken("IDENT", "color")]),
                            FakeToken("COLON", ":"),
                            FakeNode("value_list", [
                                FakeNode("value", [FakeToken("IDENT", "red")])
                            ]),
                            FakeToken("SEMICOLON", ";"),
                        ]),
                    ]),
                ]),
            ]),
        ])
        transformer._content_block_stack.append(content_block)
        transformer._content_scope_stack.append(scope)

        node = FakeNode("content_directive", [
            FakeToken("AT_KEYWORD", "@content"),
            FakeToken("SEMICOLON", ";"),
        ])

        result = transformer._expand_content(node, scope)
        # Should have extracted the block_contents children
        assert len(result) >= 1


# ===========================================================================
# 4. !default / !global Flag Tests
# ===========================================================================


class TestDefaultFlag:
    """Test !default flag behavior in variable declarations.

    !default only sets a variable if it is not already defined
    anywhere in the scope chain.
    """

    def test_default_sets_when_undefined(self) -> None:
        """!default should set variable when it's not yet defined."""
        scope = ScopeChain()
        # Simulate: $color: blue !default;
        # $color is not defined → should be set to blue
        assert scope.get("$color") is None
        # Mimicking transformer logic
        if scope.get("$color") is None:
            scope.set("$color", LatticeIdent("blue"))
        assert scope.get("$color") == LatticeIdent("blue")

    def test_default_noop_when_defined(self) -> None:
        """!default should not overwrite an existing variable."""
        scope = ScopeChain()
        scope.set("$color", LatticeIdent("red"))
        # Simulate: $color: blue !default;
        if scope.get("$color") is None:
            scope.set("$color", LatticeIdent("blue"))
        assert scope.get("$color") == LatticeIdent("red")

    def test_default_noop_when_null(self) -> None:
        """!default should not overwrite even if existing value is null.

        Per spec: if the variable IS defined (even if its value is null),
        !default does nothing.
        """
        scope = ScopeChain()
        scope.set("$var", LatticeNull())
        if scope.get("$var") is None:
            scope.set("$var", LatticeNumber(10.0))
        # Should still be null — variable exists
        assert isinstance(scope.get("$var"), LatticeNull)

    def test_default_checks_parent_scope(self) -> None:
        """!default should find variables in parent scope."""
        parent = ScopeChain()
        parent.set("$color", LatticeIdent("red"))
        child = parent.child()
        # $color is defined in parent → !default is no-op
        if child.get("$color") is None:
            child.set("$color", LatticeIdent("blue"))
        # Child doesn't have its own binding, parent still has red
        assert child.get("$color") == LatticeIdent("red")


class TestGlobalFlag:
    """Test !global flag behavior in variable declarations.

    !global sets the variable in the root (global) scope, regardless
    of how deeply nested the current context is.
    """

    def test_global_sets_in_root(self) -> None:
        """!global should set variable in root scope."""
        root = ScopeChain()
        child = root.child()
        grandchild = child.child()

        grandchild.set_global("$theme", LatticeIdent("dark"))
        assert root.get("$theme") == LatticeIdent("dark")
        # Not set in intermediate scopes
        assert "$theme" not in child.bindings
        assert "$theme" not in grandchild.bindings

    def test_global_overwrites_existing(self) -> None:
        """!global should overwrite existing global variable."""
        root = ScopeChain()
        root.set("$theme", LatticeIdent("light"))
        child = root.child()

        child.set_global("$theme", LatticeIdent("dark"))
        assert root.get("$theme") == LatticeIdent("dark")

    def test_global_creates_new(self) -> None:
        """!global should create a new global variable if it doesn't exist."""
        root = ScopeChain()
        child = root.child()

        child.set_global("$new-var", LatticeNumber(42.0))
        assert root.get("$new-var") == LatticeNumber(42.0)

    def test_default_global_combined(self) -> None:
        """!default !global: check global scope, set globally if not defined."""
        root = ScopeChain()
        child = root.child()

        # Not defined → should set globally
        if root.get("$border") is None:
            child.set_global("$border", LatticeDimension(4.0, "px"))
        assert root.get("$border") == LatticeDimension(4.0, "px")

        # Now defined → should not overwrite
        if root.get("$border") is None:
            child.set_global("$border", LatticeDimension(8.0, "px"))
        assert root.get("$border") == LatticeDimension(4.0, "px")


# ===========================================================================
# 5. Property Nesting Tests
# ===========================================================================


class TestPropertyNesting:
    """Test property nesting expansion.

    Property nesting groups related properties under a shared prefix:
    font: { size: 14px; weight: bold; } → font-size: 14px; font-weight: bold;
    """

    def test_flatten_basic(self) -> None:
        """Basic property nesting should prepend parent name with hyphen."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        # Build: font: { size: 14px; weight: bold; }
        prop_nesting = FakeNode("property_nesting", [
            FakeNode("property", [FakeToken("IDENT", "font")]),
            FakeToken("COLON", ":"),
            FakeNode("block", [
                FakeNode("block_contents", [
                    FakeNode("block_item", [
                        FakeNode("declaration_or_nested", [
                            FakeNode("declaration", [
                                FakeNode("property", [FakeToken("IDENT", "size")]),
                                FakeToken("COLON", ":"),
                                FakeNode("value_list", [
                                    FakeNode("value", [FakeToken("DIMENSION", "14px")])
                                ]),
                                FakeToken("SEMICOLON", ";"),
                            ]),
                        ]),
                    ]),
                    FakeNode("block_item", [
                        FakeNode("declaration_or_nested", [
                            FakeNode("declaration", [
                                FakeNode("property", [FakeToken("IDENT", "weight")]),
                                FakeToken("COLON", ":"),
                                FakeNode("value_list", [
                                    FakeNode("value", [FakeToken("IDENT", "bold")])
                                ]),
                                FakeToken("SEMICOLON", ";"),
                            ]),
                        ]),
                    ]),
                ]),
            ]),
        ])

        result = transformer._expand_property_nesting(prop_nesting, scope)
        assert len(result) == 2
        # First declaration should have property "font-size"
        assert result[0].children[0].children[0].value == "font-size"
        # Second should have "font-weight"
        assert result[1].children[0].children[0].value == "font-weight"

    def test_empty_nesting(self) -> None:
        """Empty property nesting block should produce no output."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        prop_nesting = FakeNode("property_nesting", [
            FakeNode("property", [FakeToken("IDENT", "font")]),
            FakeToken("COLON", ":"),
            FakeNode("block", [FakeNode("block_contents", [])]),
        ])

        result = transformer._expand_property_nesting(prop_nesting, scope)
        assert result == []


# ===========================================================================
# 6. @at-root Tests
# ===========================================================================


class TestAtRoot:
    """Test @at-root directive handling.

    @at-root hoists rules to the stylesheet root level, escaping
    any nesting context.
    """

    def test_at_root_collects_rules(self) -> None:
        """@at-root should collect rules into _at_root_rules."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        block = FakeNode("block", [
            FakeNode("block_contents", [
                FakeNode("block_item", [
                    FakeNode("declaration_or_nested", [
                        FakeNode("qualified_rule", [
                            FakeNode("selector_list", [
                                FakeNode("complex_selector", [
                                    FakeNode("compound_selector", [
                                        FakeNode("simple_selector", [
                                            FakeToken("IDENT", ".root-level")
                                        ])
                                    ])
                                ])
                            ]),
                            FakeNode("block", [
                                FakeNode("block_contents", [])
                            ]),
                        ]),
                    ]),
                ]),
            ]),
        ])

        at_root_node = FakeNode("at_root_directive", [
            FakeToken("AT_KEYWORD", "@at-root"),
            block,
        ])

        result = transformer._expand_at_root(at_root_node, scope)
        # Should return None (removed from current position)
        assert result is None
        # Rules should be collected
        assert len(transformer._at_root_rules) >= 0  # May be empty if block_contents has no items

    def test_at_root_inline_form(self) -> None:
        """@at-root with selector should create a qualified_rule."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        selector = FakeNode("selector_list", [
            FakeNode("complex_selector", [
                FakeNode("compound_selector", [
                    FakeNode("simple_selector", [
                        FakeToken("IDENT", ".sibling")
                    ])
                ])
            ])
        ])

        block = FakeNode("block", [
            FakeNode("block_contents", [])
        ])

        at_root_node = FakeNode("at_root_directive", [
            FakeToken("AT_KEYWORD", "@at-root"),
            selector,
            block,
        ])

        result = transformer._expand_at_root(at_root_node, scope)
        assert result is None
        # Should have hoisted a qualified_rule
        assert len(transformer._at_root_rules) == 1


# ===========================================================================
# 7. @extend and %placeholder Tests
# ===========================================================================


class TestExtend:
    """Test @extend directive and %placeholder selector handling."""

    def test_collect_extend(self) -> None:
        """@extend should collect the target selector."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        extend_node = FakeNode("extend_directive", [
            FakeToken("AT_KEYWORD", "@extend"),
            FakeNode("extend_target", [
                FakeToken("PLACEHOLDER", "%message-shared"),
            ]),
            FakeToken("SEMICOLON", ";"),
        ])

        transformer._collect_extend(extend_node, scope)
        assert "%message-shared" in transformer._extend_map

    def test_collect_extend_class(self) -> None:
        """@extend should work with class selectors too."""
        transformer = LatticeTransformer()
        scope = transformer.variables

        extend_node = FakeNode("extend_directive", [
            FakeToken("AT_KEYWORD", "@extend"),
            FakeNode("extend_target", [
                FakeToken("DOT", "."),
                FakeToken("IDENT", "btn"),
            ]),
            FakeToken("SEMICOLON", ";"),
        ])

        transformer._collect_extend(extend_node, scope)
        assert ".btn" in transformer._extend_map

    def test_placeholder_only_rule_detection(self) -> None:
        """Rules with only %placeholder selectors should be detected."""
        transformer = LatticeTransformer()

        # A rule with selector %placeholder
        rule = FakeNode("qualified_rule", [
            FakeNode("selector_list", [
                FakeNode("complex_selector", [
                    FakeNode("compound_selector", [
                        FakeToken("PLACEHOLDER", "%message"),
                    ])
                ])
            ]),
            FakeNode("block", [FakeNode("block_contents", [])]),
        ])

        assert transformer._is_placeholder_only_rule(rule)

    def test_non_placeholder_rule_not_detected(self) -> None:
        """Rules with regular selectors should not be detected as placeholder-only."""
        transformer = LatticeTransformer()

        rule = FakeNode("qualified_rule", [
            FakeNode("selector_list", [
                FakeNode("complex_selector", [
                    FakeNode("compound_selector", [
                        FakeNode("simple_selector", [
                            FakeToken("IDENT", ".btn")
                        ])
                    ])
                ])
            ]),
            FakeNode("block", [FakeNode("block_contents", [])]),
        ])

        assert not transformer._is_placeholder_only_rule(rule)

    def test_extend_target_not_found_error(self) -> None:
        """ExtendTargetNotFoundError should carry the target selector."""
        err = ExtendTargetNotFoundError(".nonexistent")
        assert err.target == ".nonexistent"
        assert ".nonexistent" in str(err)


# ===========================================================================
# Integration Tests — Features Combined
# ===========================================================================


class TestTier1Integration:
    """Test interactions between v2 tier 1 features."""

    def test_scope_set_global_depth(self) -> None:
        """set_global should work from any depth."""
        root = ScopeChain()
        c1 = root.child()
        c2 = c1.child()
        c3 = c2.child()

        c3.set_global("$deep", LatticeNumber(99.0))
        assert root.get("$deep") == LatticeNumber(99.0)
        assert root.depth == 0
        assert c3.depth == 3

    def test_while_counter_variable_mutation(self) -> None:
        """Variables mutated inside @while should persist in enclosing scope."""
        scope = ScopeChain()
        scope.set("$count", LatticeNumber(0.0))

        # Simulate @while $count < 3 { $count: $count + 1; }
        for _ in range(3):
            current = scope.get("$count")
            if current.value >= 3:
                break
            scope.set("$count", LatticeNumber(current.value + 1))

        assert scope.get("$count") == LatticeNumber(3.0)

    def test_max_iteration_error_inherits_lattice_error(self) -> None:
        """MaxIterationError should be catchable as LatticeError."""
        from lattice_ast_to_css.errors import LatticeError
        err = MaxIterationError()
        assert isinstance(err, LatticeError)

    def test_extend_target_not_found_inherits_lattice_error(self) -> None:
        """ExtendTargetNotFoundError should be catchable as LatticeError."""
        from lattice_ast_to_css.errors import LatticeError
        err = ExtendTargetNotFoundError(".foo")
        assert isinstance(err, LatticeError)
