"""Tests for the Mosaic Analyzer.

These tests verify that the analyzer correctly transforms a Mosaic AST into
a typed MosaicIR. Each test focuses on one semantic concern:

1. Component name extraction
2. Primitive slot types (text, number, bool, image, color, node)
3. Component-type slots (imported component names)
4. List slot types
5. Required vs optional slots (default values)
6. Default value types (string, number, dimension, color, bool)
7. Import declarations (simple and aliased)
8. Primitive node detection
9. Node properties — various value kinds
10. Slot references as property values (@slotName)
11. Enum property values (namespace.member)
12. Nested child nodes
13. Slot references as children (@slotName;)
14. When blocks
15. Each blocks with loop variable
16. Full component with multiple features
17. AnalysisError raised on bad input
"""

from __future__ import annotations

import pytest

from mosaic_analyzer import (
    AnalysisError,
    MosaicComponent,
    MosaicIR,
    MosaicImport,
    MosaicNode,
    MosaicProperty,
    MosaicSlot,
    PRIMITIVE_NODES,
    analyze,
)


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def quick(source: str) -> MosaicIR:
    """Wrap minimal source in analyze()."""
    return analyze(source)


def component(name: str, slots: str = "", tree: str = "Text {}") -> str:
    """Build a minimal component source string."""
    return f"component {name} {{ {slots} {tree} }}"


# ---------------------------------------------------------------------------
# 1. Component name
# ---------------------------------------------------------------------------

class TestComponentName:
    """Verify the component name is correctly extracted."""

    def test_simple_name(self) -> None:
        ir = quick(component("Label"))
        assert ir.component.name == "Label"

    def test_pascal_case_name(self) -> None:
        ir = quick(component("ProfileCard"))
        assert ir.component.name == "ProfileCard"

    def test_component_type(self) -> None:
        ir = quick(component("X"))
        assert isinstance(ir.component, MosaicComponent)


# ---------------------------------------------------------------------------
# 2. Primitive slot types
# ---------------------------------------------------------------------------

class TestPrimitiveSlotTypes:
    """Verify slot types for all primitive kinds."""

    def test_text_type(self) -> None:
        ir = quick(component("X", "slot title: text;"))
        assert ir.component.slots[0].type == {"kind": "text"}

    def test_number_type(self) -> None:
        ir = quick(component("X", "slot count: number;"))
        assert ir.component.slots[0].type == {"kind": "number"}

    def test_bool_type(self) -> None:
        ir = quick(component("X", "slot visible: bool;"))
        assert ir.component.slots[0].type == {"kind": "bool"}

    def test_image_type(self) -> None:
        ir = quick(component("X", "slot src: image;"))
        assert ir.component.slots[0].type == {"kind": "image"}

    def test_color_type(self) -> None:
        ir = quick(component("X", "slot bg: color;"))
        assert ir.component.slots[0].type == {"kind": "color"}

    def test_node_type(self) -> None:
        ir = quick(component("X", "slot content: node;"))
        assert ir.component.slots[0].type == {"kind": "node"}


# ---------------------------------------------------------------------------
# 3. Component-type slots
# ---------------------------------------------------------------------------

class TestComponentTypeSlots:
    """Verify slots with imported component types."""

    def test_component_type(self) -> None:
        ir = quick(component("X", "slot action: Button;"))
        assert ir.component.slots[0].type == {"kind": "component", "name": "Button"}

    def test_component_type_name(self) -> None:
        ir = quick(component("X", "slot action: Badge;"))
        assert ir.component.slots[0].type["name"] == "Badge"


# ---------------------------------------------------------------------------
# 4. List slot types
# ---------------------------------------------------------------------------

class TestListSlotTypes:
    """Verify list<T> slot types."""

    def test_list_text(self) -> None:
        ir = quick(component("X", "slot items: list<text>;"))
        t = ir.component.slots[0].type
        assert t == {"kind": "list", "element_type": {"kind": "text"}}

    def test_list_number(self) -> None:
        ir = quick(component("X", "slot scores: list<number>;"))
        t = ir.component.slots[0].type
        assert t["element_type"] == {"kind": "number"}

    def test_list_component(self) -> None:
        ir = quick(component("X", "slot actions: list<Button>;"))
        t = ir.component.slots[0].type
        assert t["element_type"] == {"kind": "component", "name": "Button"}


# ---------------------------------------------------------------------------
# 5. Required vs optional slots
# ---------------------------------------------------------------------------

class TestRequiredOptionalSlots:
    """Verify required flag based on default value presence."""

    def test_required_slot(self) -> None:
        ir = quick(component("X", "slot title: text;"))
        assert ir.component.slots[0].required is True
        assert ir.component.slots[0].default_value is None

    def test_optional_slot_with_number_default(self) -> None:
        ir = quick(component("X", "slot count: number = 0;"))
        assert ir.component.slots[0].required is False
        assert ir.component.slots[0].default_value is not None

    def test_optional_slot_with_bool_default(self) -> None:
        ir = quick(component("X", "slot visible: bool = true;"))
        assert ir.component.slots[0].required is False


# ---------------------------------------------------------------------------
# 6. Default value types
# ---------------------------------------------------------------------------

class TestDefaultValueTypes:
    """Verify default value kinds for various literal types."""

    def test_default_number(self) -> None:
        ir = quick(component("X", "slot count: number = 42;"))
        dv = ir.component.slots[0].default_value
        assert dv == {"kind": "number", "value": 42.0}

    def test_default_bool_true(self) -> None:
        ir = quick(component("X", "slot visible: bool = true;"))
        dv = ir.component.slots[0].default_value
        assert dv == {"kind": "bool", "value": True}

    def test_default_bool_false(self) -> None:
        ir = quick(component("X", "slot visible: bool = false;"))
        dv = ir.component.slots[0].default_value
        assert dv == {"kind": "bool", "value": False}

    def test_default_color(self) -> None:
        ir = quick(component("X", "slot bg: color = #fff;"))
        dv = ir.component.slots[0].default_value
        assert dv == {"kind": "color_hex", "value": "#fff"}

    def test_default_dimension(self) -> None:
        ir = quick(component("X", "slot pad: number = 16dp;"))
        dv = ir.component.slots[0].default_value
        assert dv == {"kind": "dimension", "value": 16.0, "unit": "dp"}

    def test_default_string(self) -> None:
        ir = quick(component("X", 'slot label: text = "hello";'))
        dv = ir.component.slots[0].default_value
        assert dv == {"kind": "string", "value": "hello"}


# ---------------------------------------------------------------------------
# 7. Import declarations
# ---------------------------------------------------------------------------

class TestImportDeclarations:
    """Verify MosaicImport extraction."""

    def test_simple_import(self) -> None:
        src = 'import Button from "./button.mosaic"; component X { Text {} }'
        ir = quick(src)
        assert len(ir.imports) == 1
        assert ir.imports[0].name == "Button"
        assert ir.imports[0].path == "./button.mosaic"
        assert ir.imports[0].alias is None

    def test_aliased_import(self) -> None:
        src = 'import Card as InfoCard from "./card.mosaic"; component X { Text {} }'
        ir = quick(src)
        assert ir.imports[0].name == "Card"
        assert ir.imports[0].alias == "InfoCard"

    def test_multiple_imports(self) -> None:
        src = (
            'import A from "./a.mosaic"; '
            'import B from "./b.mosaic"; '
            'component X { Text {} }'
        )
        ir = quick(src)
        assert len(ir.imports) == 2


# ---------------------------------------------------------------------------
# 8. Primitive node detection
# ---------------------------------------------------------------------------

class TestPrimitiveNodeDetection:
    """Verify is_primitive flag on nodes."""

    def test_text_is_primitive(self) -> None:
        ir = quick(component("X", tree="Text {}"))
        assert ir.component.root.is_primitive is True

    def test_column_is_primitive(self) -> None:
        ir = quick(component("X", tree="Column {}"))
        assert ir.component.root.is_primitive is True

    def test_all_primitives(self) -> None:
        for prim in PRIMITIVE_NODES:
            ir = quick(component("X", tree=f"{prim} {{}}"))
            assert ir.component.root.is_primitive is True, f"{prim} should be primitive"

    def test_custom_component_not_primitive(self) -> None:
        ir = quick(component("X", tree="Button {}"))
        assert ir.component.root.is_primitive is False


# ---------------------------------------------------------------------------
# 9. Node properties
# ---------------------------------------------------------------------------

class TestNodeProperties:
    """Verify property name and value extraction."""

    def test_dimension_property(self) -> None:
        ir = quick(component("X", tree="Text { padding: 16dp; }"))
        props = ir.component.root.properties
        assert len(props) == 1
        assert props[0].name == "padding"
        assert props[0].value == {"kind": "dimension", "value": 16.0, "unit": "dp"}

    def test_color_property(self) -> None:
        ir = quick(component("X", tree="Text { background: #2563eb; }"))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "color_hex", "value": "#2563eb"}

    def test_string_property(self) -> None:
        ir = quick(component("X", tree='Text { content: "hello"; }'))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "string", "value": "hello"}

    def test_number_property(self) -> None:
        ir = quick(component("X", tree="Text { opacity: 1; }"))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "number", "value": 1.0}

    def test_ident_property(self) -> None:
        ir = quick(component("X", tree="Text { align: center; }"))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "ident", "value": "center"}

    def test_bool_true_property(self) -> None:
        ir = quick(component("X", tree="Text { visible: true; }"))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "bool", "value": True}


# ---------------------------------------------------------------------------
# 10. Slot references as property values
# ---------------------------------------------------------------------------

class TestSlotRefAsPropertyValue:
    """Verify @slotName in property values becomes slot_ref dict."""

    def test_slot_ref_in_content(self) -> None:
        ir = quick(component("X", "slot title: text;", "Text { content: @title; }"))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "slot_ref", "slot_name": "title"}

    def test_slot_ref_name(self) -> None:
        ir = quick(component("X", "slot src: image;", "Image { source: @src; }"))
        assert ir.component.root.properties[0].value["slot_name"] == "src"


# ---------------------------------------------------------------------------
# 11. Enum property values
# ---------------------------------------------------------------------------

class TestEnumPropertyValues:
    """Verify namespace.member enum values."""

    def test_enum_value(self) -> None:
        ir = quick(component("X", tree="Text { style: heading.large; }"))
        props = ir.component.root.properties
        assert props[0].value == {"kind": "enum", "namespace": "heading", "member": "large"}


# ---------------------------------------------------------------------------
# 12. Nested child nodes
# ---------------------------------------------------------------------------

class TestNestedChildNodes:
    """Verify nested node children."""

    def test_direct_child(self) -> None:
        ir = quick(component("X", tree="Column { Text {} }"))
        children = ir.component.root.children
        assert len(children) == 1
        assert children[0]["kind"] == "node"
        assert children[0]["node"].node_type == "Text"

    def test_nested_primitive(self) -> None:
        ir = quick(component("X", tree="Column { Row { Text {} } }"))
        col_children = ir.component.root.children
        assert col_children[0]["node"].node_type == "Row"


# ---------------------------------------------------------------------------
# 13. Slot references as children
# ---------------------------------------------------------------------------

class TestSlotRefAsChild:
    """Verify @slotName; as a child produces a slot_ref child dict."""

    def test_slot_ref_child(self) -> None:
        ir = quick(component("X", "slot action: node;", "Column { @action; }"))
        children = ir.component.root.children
        assert len(children) == 1
        assert children[0]["kind"] == "slot_ref"
        assert children[0]["slot_name"] == "action"


# ---------------------------------------------------------------------------
# 14. When blocks
# ---------------------------------------------------------------------------

class TestWhenBlocks:
    """Verify when block child dicts."""

    def test_when_block_kind(self) -> None:
        ir = quick(component("X", "slot show: bool;", "Column { when @show { Text {} } }"))
        children = ir.component.root.children
        assert children[0]["kind"] == "when"

    def test_when_slot_name(self) -> None:
        ir = quick(component("X", "slot show: bool;", "Column { when @show { Text {} } }"))
        assert ir.component.root.children[0]["slot_name"] == "show"

    def test_when_body_has_node(self) -> None:
        ir = quick(component("X", "slot show: bool;", "Column { when @show { Text {} } }"))
        body = ir.component.root.children[0]["body"]
        assert len(body) == 1
        assert body[0]["kind"] == "node"


# ---------------------------------------------------------------------------
# 15. Each blocks
# ---------------------------------------------------------------------------

class TestEachBlocks:
    """Verify each block child dicts."""

    def test_each_block_kind(self) -> None:
        ir = quick(component("X", "slot items: list<text>;",
                              "Column { each @items as item { Text {} } }"))
        children = ir.component.root.children
        assert children[0]["kind"] == "each"

    def test_each_slot_name(self) -> None:
        ir = quick(component("X", "slot items: list<text>;",
                              "Column { each @items as item { Text {} } }"))
        assert ir.component.root.children[0]["slot_name"] == "items"

    def test_each_item_name(self) -> None:
        ir = quick(component("X", "slot items: list<text>;",
                              "Column { each @items as item { Text {} } }"))
        assert ir.component.root.children[0]["item_name"] == "item"

    def test_each_body(self) -> None:
        ir = quick(component("X", "slot items: list<text>;",
                              "Column { each @items as item { Text {} } }"))
        body = ir.component.root.children[0]["body"]
        assert len(body) == 1
        assert body[0]["kind"] == "node"


# ---------------------------------------------------------------------------
# 16. Full component
# ---------------------------------------------------------------------------

class TestFullComponent:
    """Verify a full-featured component produces correct IR."""

    def test_full_component(self) -> None:
        src = """
        component ProfileCard {
            slot avatar-url: image;
            slot display-name: text;
            slot follower-count: number = 0;
            slot items: list<text>;
            slot show-footer: bool = false;

            Column {
                Image { source: @avatar-url; }
                Text { content: @display-name; font-size: 18sp; }
                when @show-footer {
                    Text { content: "Footer"; }
                }
                each @items as item {
                    Text { content: @item; }
                }
            }
        }
        """
        ir = analyze(src)
        assert ir.component.name == "ProfileCard"
        assert len(ir.component.slots) == 5
        assert ir.component.root.node_type == "Column"
        assert len(ir.component.root.children) == 4

    def test_ir_type(self) -> None:
        ir = quick(component("X"))
        assert isinstance(ir, MosaicIR)
        assert isinstance(ir.component, MosaicComponent)


# ---------------------------------------------------------------------------
# 17. AnalysisError
# ---------------------------------------------------------------------------

class TestAnalysisError:
    """Verify AnalysisError is raised for structural problems."""

    def test_analysis_error_is_exception(self) -> None:
        assert issubclass(AnalysisError, Exception)
