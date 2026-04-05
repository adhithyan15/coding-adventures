"""Tests for the Mosaic Parser.

These tests verify that the grammar-driven parser correctly builds an AST from
Mosaic source code. The AST mirrors the grammar: each ``ASTNode`` has a
``rule_name`` that corresponds to a grammar rule, and ``Token`` objects are
leaf nodes with ``type_name`` and ``value`` attributes.

Test Organisation
-----------------

1. Root rule — file has rule_name "file"
2. Component declaration — component_decl node
3. Slot declarations — slot_decl with various types
4. Import declarations — import_decl node
5. Node tree structure — node_tree + node_element
6. Property assignments — property_assignment nodes
7. Slot references as properties — slot_ref nodes
8. Nested nodes — child_node recursion
9. When blocks — when_block nodes
10. Each blocks — each_block nodes
11. List types — list_type nodes
12. Enum values — enum_value (NAME.NAME) nodes
13. create_parser factory
14. Default values in slots
15. Full component with all features
"""

from __future__ import annotations

import pytest

from lang_parser import ASTNode
from mosaic_parser import PARSER_GRAMMAR, create_parser, parse


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def find_child(node: ASTNode, rule_name: str) -> ASTNode | None:
    """Find first direct child ASTNode with given rule_name."""
    for c in node.children:
        if isinstance(c, ASTNode) and c.rule_name == rule_name:
            return c
    return None


def find_tokens(node: ASTNode, type_name: str) -> list:
    """Find all direct-child tokens with given type_name."""
    return [c for c in node.children if not isinstance(c, ASTNode) and c.type_name == type_name]


def token_values(node: ASTNode, type_name: str) -> list[str]:
    """Return values of all direct-child tokens with given type_name."""
    return [c.value for c in node.children if not isinstance(c, ASTNode) and c.type_name == type_name]


# ---------------------------------------------------------------------------
# 1. Root rule
# ---------------------------------------------------------------------------

class TestRootRule:
    """The root of the AST must be the 'file' rule."""

    def test_root_is_file(self) -> None:
        ast = parse("component X { Text {} }")
        assert ast.rule_name == "file"

    def test_file_has_component_decl(self) -> None:
        ast = parse("component X { Text {} }")
        comp = find_child(ast, "component_decl")
        assert comp is not None


# ---------------------------------------------------------------------------
# 2. Component declaration
# ---------------------------------------------------------------------------

class TestComponentDeclaration:
    """Verify component_decl structure."""

    def test_component_name(self) -> None:
        ast = parse("component ProfileCard { Text {} }")
        comp = find_child(ast, "component_decl")
        assert comp is not None
        names = token_values(comp, "NAME")
        assert "ProfileCard" in names

    def test_component_keyword(self) -> None:
        ast = parse("component X { Text {} }")
        comp = find_child(ast, "component_decl")
        assert comp is not None
        keywords = token_values(comp, "KEYWORD")
        assert "component" in keywords

    def test_component_has_node_tree(self) -> None:
        ast = parse("component X { Text {} }")
        comp = find_child(ast, "component_decl")
        assert comp is not None
        tree = find_child(comp, "node_tree")
        assert tree is not None


# ---------------------------------------------------------------------------
# 3. Slot declarations
# ---------------------------------------------------------------------------

class TestSlotDeclarations:
    """Verify slot_decl structure for various types."""

    def test_slot_text_type(self) -> None:
        src = "component X { slot title: text; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        assert slot is not None
        names = token_values(slot, "NAME")
        assert "title" in names

    def test_slot_type_has_keyword(self) -> None:
        src = "component X { slot count: number; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        slot_type = find_child(slot, "slot_type")
        assert slot_type is not None

    def test_slot_component_type_is_name(self) -> None:
        src = "component X { slot action: Button; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        slot_type = find_child(slot, "slot_type")
        assert slot_type is not None
        names = token_values(slot_type, "NAME")
        assert "Button" in names

    def test_multiple_slots(self) -> None:
        src = "component X { slot a: text; slot b: number; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slots = [c for c in comp.children if isinstance(c, ASTNode) and c.rule_name == "slot_decl"]
        assert len(slots) == 2


# ---------------------------------------------------------------------------
# 4. Import declarations
# ---------------------------------------------------------------------------

class TestImportDeclarations:
    """Verify import_decl structure."""

    def test_simple_import(self) -> None:
        src = 'import Button from "./button.mosaic"; component X { Text {} }'
        ast = parse(src)
        imp = find_child(ast, "import_decl")
        assert imp is not None
        names = token_values(imp, "NAME")
        assert "Button" in names

    def test_import_with_alias(self) -> None:
        src = 'import Card as InfoCard from "./card.mosaic"; component X { Text {} }'
        ast = parse(src)
        imp = find_child(ast, "import_decl")
        assert imp is not None
        names = token_values(imp, "NAME")
        assert "Card" in names
        assert "InfoCard" in names


# ---------------------------------------------------------------------------
# 5. Node tree structure
# ---------------------------------------------------------------------------

class TestNodeTreeStructure:
    """Verify node_tree and node_element structure."""

    def test_node_tree_has_node_element(self) -> None:
        ast = parse("component X { Text {} }")
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        assert elem is not None

    def test_node_element_tag(self) -> None:
        ast = parse("component X { Column {} }")
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        names = token_values(elem, "NAME")
        assert "Column" in names


# ---------------------------------------------------------------------------
# 6. Property assignments
# ---------------------------------------------------------------------------

class TestPropertyAssignments:
    """Verify property_assignment nodes."""

    def test_property_with_string(self) -> None:
        src = 'component X { Text { content: "hello"; } }'
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        content_node = find_child(elem, "node_content")
        prop = find_child(content_node, "property_assignment")
        assert prop is not None

    def test_property_with_dimension(self) -> None:
        src = "component X { Text { padding: 16dp; } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        content_node = find_child(elem, "node_content")
        prop = find_child(content_node, "property_assignment")
        assert prop is not None
        prop_val = find_child(prop, "property_value")
        dims = token_values(prop_val, "DIMENSION")
        assert "16dp" in dims

    def test_property_with_color(self) -> None:
        src = "component X { Text { background: #2563eb; } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        content = find_child(elem, "node_content")
        prop = find_child(content, "property_assignment")
        prop_val = find_child(prop, "property_value")
        colors = token_values(prop_val, "COLOR_HEX")
        assert "#2563eb" in colors


# ---------------------------------------------------------------------------
# 7. Slot references as properties
# ---------------------------------------------------------------------------

class TestSlotRefAsProperty:
    """Verify slot_ref nodes inside property_value."""

    def test_slot_ref_at_name(self) -> None:
        src = "component X { Text { content: @title; } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        content = find_child(elem, "node_content")
        prop = find_child(content, "property_assignment")
        prop_val = find_child(prop, "property_value")
        slot_ref = find_child(prop_val, "slot_ref")
        assert slot_ref is not None
        names = token_values(slot_ref, "NAME")
        assert "title" in names


# ---------------------------------------------------------------------------
# 8. Nested child nodes
# ---------------------------------------------------------------------------

class TestNestedNodes:
    """Verify nested child_node structure."""

    def test_nested_child_node(self) -> None:
        src = "component X { Column { Text {} } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        col = find_child(tree, "node_element")
        content = find_child(col, "node_content")
        child = find_child(content, "child_node")
        assert child is not None

    def test_deeply_nested(self) -> None:
        src = "component X { Column { Row { Text {} } } }"
        ast = parse(src)
        assert ast.rule_name == "file"


# ---------------------------------------------------------------------------
# 9. When blocks
# ---------------------------------------------------------------------------

class TestWhenBlocks:
    """Verify when_block structure."""

    def test_when_block_exists(self) -> None:
        src = "component X { Column { when @show { Text {} } } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        col = find_child(tree, "node_element")
        content = find_child(col, "node_content")
        when = find_child(content, "when_block")
        assert when is not None

    def test_when_has_slot_ref(self) -> None:
        src = "component X { Column { when @visible { Text {} } } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        col = find_child(tree, "node_element")
        content = find_child(col, "node_content")
        when = find_child(content, "when_block")
        slot_ref = find_child(when, "slot_ref")
        assert slot_ref is not None
        names = token_values(slot_ref, "NAME")
        assert "visible" in names


# ---------------------------------------------------------------------------
# 10. Each blocks
# ---------------------------------------------------------------------------

class TestEachBlocks:
    """Verify each_block structure."""

    def test_each_block_exists(self) -> None:
        src = "component X { Column { each @items as item { Text {} } } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        col = find_child(tree, "node_element")
        content = find_child(col, "node_content")
        each = find_child(content, "each_block")
        assert each is not None

    def test_each_has_slot_ref_and_loop_var(self) -> None:
        src = "component X { Column { each @items as item { Text {} } } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        col = find_child(tree, "node_element")
        content = find_child(col, "node_content")
        each = find_child(content, "each_block")
        slot_ref = find_child(each, "slot_ref")
        assert slot_ref is not None
        # "item" should appear as a NAME token directly on each_block
        names = token_values(each, "NAME")
        assert "item" in names


# ---------------------------------------------------------------------------
# 11. List types
# ---------------------------------------------------------------------------

class TestListTypes:
    """Verify list_type nodes for list<T> slots."""

    def test_list_text_slot(self) -> None:
        src = "component X { slot items: list<text>; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        slot_type = find_child(slot, "slot_type")
        list_type = find_child(slot_type, "list_type")
        assert list_type is not None

    def test_list_component_slot(self) -> None:
        src = "component X { slot actions: list<Button>; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        slot_type = find_child(slot, "slot_type")
        list_type = find_child(slot_type, "list_type")
        assert list_type is not None


# ---------------------------------------------------------------------------
# 12. Enum values
# ---------------------------------------------------------------------------

class TestEnumValues:
    """Verify enum_value (NAME.NAME) nodes."""

    def test_enum_value_in_property(self) -> None:
        src = "component X { Text { style: heading.large; } }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        tree = find_child(comp, "node_tree")
        elem = find_child(tree, "node_element")
        content = find_child(elem, "node_content")
        prop = find_child(content, "property_assignment")
        prop_val = find_child(prop, "property_value")
        enum_val = find_child(prop_val, "enum_value")
        assert enum_val is not None
        names = token_values(enum_val, "NAME")
        assert "heading" in names
        assert "large" in names


# ---------------------------------------------------------------------------
# 13. create_parser factory
# ---------------------------------------------------------------------------

class TestCreateParserFactory:
    """Verify create_parser() factory function."""

    def test_create_parser_returns_grammar_parser(self) -> None:
        p = create_parser("component X { Text {} }")
        ast = p.parse()
        assert ast.rule_name == "file"

    def test_create_parser_matches_parse(self) -> None:
        src = "component X { slot y: number; Text {} }"
        ast1 = parse(src)
        ast2 = create_parser(src).parse()
        assert ast1.rule_name == ast2.rule_name


# ---------------------------------------------------------------------------
# 14. Default values
# ---------------------------------------------------------------------------

class TestDefaultValues:
    """Verify default_value nodes for optional slots."""

    def test_default_number(self) -> None:
        src = "component X { slot count: number = 0; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        default = find_child(slot, "default_value")
        assert default is not None
        nums = token_values(default, "NUMBER")
        assert "0" in nums

    def test_default_bool_true(self) -> None:
        src = "component X { slot visible: bool = true; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        default = find_child(slot, "default_value")
        assert default is not None
        kws = token_values(default, "KEYWORD")
        assert "true" in kws

    def test_default_color(self) -> None:
        src = "component X { slot bg: color = #fff; Text {} }"
        ast = parse(src)
        comp = find_child(ast, "component_decl")
        slot = find_child(comp, "slot_decl")
        default = find_child(slot, "default_value")
        colors = token_values(default, "COLOR_HEX")
        assert "#fff" in colors


# ---------------------------------------------------------------------------
# 15. Full component
# ---------------------------------------------------------------------------

class TestFullComponent:
    """Verify a full-featured component parses without errors."""

    def test_full_component(self) -> None:
        src = """
        component ProfileCard {
            slot avatar-url: image;
            slot display-name: text;
            slot items: list<text>;

            Column {
                Image { source: @avatar-url; }
                Text { content: @display-name; font-size: 18sp; }
                each @items as item {
                    Text { content: @item; }
                }
            }
        }
        """
        ast = parse(src)
        assert ast.rule_name == "file"

    def test_parser_grammar_accessible(self) -> None:
        assert PARSER_GRAMMAR is not None
        rule_names = [r.name for r in PARSER_GRAMMAR.rules]
        assert "file" in rule_names
        assert "component_decl" in rule_names
        assert "slot_decl" in rule_names
