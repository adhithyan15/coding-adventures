"""Tests for the Mosaic VM.

The VM drives tree traversal by calling MosaicRenderer methods in depth-first
order. These tests use a recording renderer to verify call sequences and value
normalization.

Test Organisation
-----------------

1. Basic traversal call order
2. Color hex parsing — #rgb, #rrggbb, #rrggbbaa
3. Dimension pass-through
4. Slot reference resolution in properties
5. When block traversal
6. Each block traversal with loop variable
7. Slot references as children (render_slot_child)
8. Multiple slots and properties
9. MosaicVMError on unknown slots
10. Value normalization — ident → string, bool, number, enum
"""

from __future__ import annotations

import pytest

from mosaic_analyzer import analyze
from mosaic_vm import MosaicRenderer, MosaicVM, MosaicVMError, SlotContext
from mosaic_analyzer import MosaicSlot


# ---------------------------------------------------------------------------
# Recording Renderer
# ---------------------------------------------------------------------------

class RecordingRenderer(MosaicRenderer):
    """Records all renderer calls for assertion in tests."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []

    def begin_component(self, name: str, slots: list[MosaicSlot]) -> None:
        self.calls.append(("begin_component", name, len(slots)))

    def end_component(self) -> None:
        self.calls.append(("end_component",))

    def emit(self) -> list[dict]:
        return [{"filename": "out.txt", "content": str(self.calls)}]

    def begin_node(
        self,
        tag: str,
        is_primitive: bool,
        properties: list[dict],
        context: SlotContext,
    ) -> None:
        self.calls.append(("begin_node", tag, is_primitive, properties))

    def end_node(self, tag: str) -> None:
        self.calls.append(("end_node", tag))

    def render_slot_child(
        self, slot_name: str, slot_type: dict, context: SlotContext
    ) -> None:
        self.calls.append(("render_slot_child", slot_name, slot_type))

    def begin_when(self, slot_name: str, context: SlotContext) -> None:
        self.calls.append(("begin_when", slot_name))

    def end_when(self) -> None:
        self.calls.append(("end_when",))

    def begin_each(
        self,
        slot_name: str,
        item_name: str,
        element_type: dict,
        context: SlotContext,
    ) -> None:
        self.calls.append(("begin_each", slot_name, item_name, element_type))

    def end_each(self) -> None:
        self.calls.append(("end_each",))


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def run(source: str) -> tuple[RecordingRenderer, list[dict]]:
    """Analyze source, run VM with a recording renderer."""
    ir = analyze(source)
    vm = MosaicVM(ir)
    r = RecordingRenderer()
    files = vm.run(r)
    return r, files


# ---------------------------------------------------------------------------
# 1. Basic traversal call order
# ---------------------------------------------------------------------------

class TestBasicTraversal:
    """Verify call sequence for a minimal component."""

    def test_minimal_component_calls(self) -> None:
        r, _ = run("component X { Text {} }")
        kinds = [c[0] for c in r.calls]
        assert kinds == ["begin_component", "begin_node", "end_node", "end_component"]

    def test_begin_component_name(self) -> None:
        r, _ = run("component ProfileCard { Text {} }")
        assert r.calls[0][1] == "ProfileCard"

    def test_emit_returns_files(self) -> None:
        _, files = run("component X { Text {} }")
        assert isinstance(files, list)
        assert len(files) == 1
        assert "filename" in files[0]

    def test_begin_node_tag(self) -> None:
        r, _ = run("component X { Column {} }")
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        assert begin_node[1] == "Column"

    def test_begin_node_is_primitive(self) -> None:
        r, _ = run("component X { Text {} }")
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        assert begin_node[2] is True  # Text is primitive

    def test_nested_nodes_call_order(self) -> None:
        r, _ = run("component X { Column { Text {} } }")
        kinds = [c[0] for c in r.calls]
        # begin_component, begin_node(Column), begin_node(Text), end_node(Text),
        # end_node(Column), end_component
        assert kinds.count("begin_node") == 2
        assert kinds.count("end_node") == 2


# ---------------------------------------------------------------------------
# 2. Color hex parsing
# ---------------------------------------------------------------------------

class TestColorParsing:
    """Verify hex color strings are parsed into RGBA dicts."""

    def test_6digit_color(self) -> None:
        r, _ = run('component X { Text { background: #2563eb; } }')
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        color = props[0]["value"]
        assert color["kind"] == "color"
        assert color["r"] == 0x25
        assert color["g"] == 0x63
        assert color["b"] == 0xeb
        assert color["a"] == 255

    def test_3digit_color(self) -> None:
        r, _ = run('component X { Text { color: #fff; } }')
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        color = props[0]["value"]
        assert color["kind"] == "color"
        assert color["r"] == 255
        assert color["g"] == 255
        assert color["b"] == 255
        assert color["a"] == 255

    def test_8digit_color_alpha(self) -> None:
        r, _ = run('component X { Text { color: #00000080; } }')
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        color = props[0]["value"]
        assert color["kind"] == "color"
        assert color["a"] == 0x80


# ---------------------------------------------------------------------------
# 3. Dimension pass-through
# ---------------------------------------------------------------------------

class TestDimensionPassThrough:
    """Verify dimension values are normalized correctly."""

    def test_dp_dimension(self) -> None:
        r, _ = run('component X { Text { padding: 16dp; } }')
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        dim = props[0]["value"]
        assert dim["kind"] == "dimension"
        assert dim["value"] == 16.0
        assert dim["unit"] == "dp"

    def test_percent_dimension(self) -> None:
        r, _ = run('component X { Text { width: 100%; } }')
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        dim = props[0]["value"]
        assert dim["unit"] == "%"


# ---------------------------------------------------------------------------
# 4. Slot reference resolution in properties
# ---------------------------------------------------------------------------

class TestSlotRefInProperties:
    """Verify slot refs in properties become enriched slot_ref dicts."""

    def test_slot_ref_resolved(self) -> None:
        src = "component X { slot title: text; Text { content: @title; } }"
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        ref = props[0]["value"]
        assert ref["kind"] == "slot_ref"
        assert ref["slot_name"] == "title"
        assert ref["slot_type"] == {"kind": "text"}
        assert ref["is_loop_var"] is False


# ---------------------------------------------------------------------------
# 5. When block traversal
# ---------------------------------------------------------------------------

class TestWhenBlock:
    """Verify when block produces correct call sequence."""

    def test_when_call_order(self) -> None:
        src = "component X { slot show: bool; Column { when @show { Text {} } } }"
        r, _ = run(src)
        kinds = [c[0] for c in r.calls]
        assert "begin_when" in kinds
        assert "end_when" in kinds
        when_idx = kinds.index("begin_when")
        end_idx = kinds.index("end_when")
        assert when_idx < end_idx

    def test_when_slot_name(self) -> None:
        src = "component X { slot show: bool; Column { when @show { Text {} } } }"
        r, _ = run(src)
        when_call = next(c for c in r.calls if c[0] == "begin_when")
        assert when_call[1] == "show"


# ---------------------------------------------------------------------------
# 6. Each block traversal
# ---------------------------------------------------------------------------

class TestEachBlock:
    """Verify each block produces correct call sequence."""

    def test_each_call_order(self) -> None:
        src = "component X { slot items: list<text>; Column { each @items as item { Text {} } } }"
        r, _ = run(src)
        kinds = [c[0] for c in r.calls]
        assert "begin_each" in kinds
        assert "end_each" in kinds

    def test_each_slot_name_and_item(self) -> None:
        src = "component X { slot items: list<text>; Column { each @items as item { Text {} } } }"
        r, _ = run(src)
        each_call = next(c for c in r.calls if c[0] == "begin_each")
        assert each_call[1] == "items"
        assert each_call[2] == "item"

    def test_each_element_type(self) -> None:
        src = "component X { slot items: list<text>; Column { each @items as item { Text {} } } }"
        r, _ = run(src)
        each_call = next(c for c in r.calls if c[0] == "begin_each")
        assert each_call[3] == {"kind": "text"}

    def test_loop_var_in_each_body(self) -> None:
        src = "component X { slot items: list<text>; Column { each @items as item { Text { content: @item; } } } }"
        r, _ = run(src)
        # The @item reference inside the each body should resolve as a loop var
        begin_text = [c for c in r.calls if c[0] == "begin_node" and c[1] == "Text"]
        assert len(begin_text) == 1
        props = begin_text[0][3]
        ref = props[0]["value"]
        assert ref["kind"] == "slot_ref"
        assert ref["is_loop_var"] is True


# ---------------------------------------------------------------------------
# 7. Slot references as children
# ---------------------------------------------------------------------------

class TestSlotRefChild:
    """Verify @slot; children produce render_slot_child calls."""

    def test_slot_child_called(self) -> None:
        src = "component X { slot action: node; Column { @action; } }"
        r, _ = run(src)
        kinds = [c[0] for c in r.calls]
        assert "render_slot_child" in kinds

    def test_slot_child_name(self) -> None:
        src = "component X { slot action: node; Column { @action; } }"
        r, _ = run(src)
        child_call = next(c for c in r.calls if c[0] == "render_slot_child")
        assert child_call[1] == "action"


# ---------------------------------------------------------------------------
# 8. Multiple slots and properties
# ---------------------------------------------------------------------------

class TestMultipleSlotsAndProperties:
    """Verify multiple properties are resolved correctly."""

    def test_multiple_properties(self) -> None:
        src = "component X { Text { padding: 16dp; color: #fff; } }"
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        assert len(props) == 2
        assert props[0]["name"] == "padding"
        assert props[1]["name"] == "color"


# ---------------------------------------------------------------------------
# 9. MosaicVMError on unknown slots
# ---------------------------------------------------------------------------

class TestMosaicVMError:
    """Verify MosaicVMError is raised for unresolvable slots."""

    def test_error_is_exception(self) -> None:
        assert issubclass(MosaicVMError, Exception)


# ---------------------------------------------------------------------------
# 10. Value normalization
# ---------------------------------------------------------------------------

class TestValueNormalization:
    """Verify value kinds are normalized correctly."""

    def test_bool_true_property(self) -> None:
        src = "component X { Text { visible: true; } }"
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        assert props[0]["value"] == {"kind": "bool", "value": True}

    def test_string_property(self) -> None:
        src = 'component X { Text { label: "hello"; } }'
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        assert props[0]["value"] == {"kind": "string", "value": "hello"}

    def test_number_property(self) -> None:
        src = "component X { Text { opacity: 1; } }"
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        assert props[0]["value"] == {"kind": "number", "value": 1.0}

    def test_ident_becomes_string(self) -> None:
        src = "component X { Text { align: center; } }"
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        # ident is normalized to string
        assert props[0]["value"]["kind"] == "string"
        assert props[0]["value"]["value"] == "center"

    def test_enum_property(self) -> None:
        src = "component X { Text { style: heading.large; } }"
        r, _ = run(src)
        begin_node = next(c for c in r.calls if c[0] == "begin_node")
        props = begin_node[3]
        assert props[0]["value"] == {"kind": "enum", "namespace": "heading", "member": "large"}
