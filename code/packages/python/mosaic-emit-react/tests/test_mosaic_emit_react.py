"""Tests for the Mosaic React Backend.

These tests verify that the ReactRenderer and emit_react() function produce
correct TypeScript React component (.tsx) output from Mosaic source.

Test Organisation
-----------------

1. Output file name (ComponentName.tsx)
2. React import statement
3. Props interface generation
4. Required vs optional props
5. TypeScript type mapping for slot types
6. Functional component export
7. Primitive node → JSX element mapping
8. Node properties in inline styles
9. Color rendering as rgba()
10. Slot ref in JSX (content: @slotName)
11. When block → conditional rendering
12. Each block → .map() rendering
13. Slot ref as child → {slotVar}
14. emit_react() convenience function
15. ReactRenderer can be run multiple times
"""

from __future__ import annotations

import pytest

from mosaic_analyzer import analyze
from mosaic_vm import MosaicVM
from mosaic_emit_react import ReactRenderer, emit_react


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def run(source: str) -> str:
    """Run emit_react and return the file content."""
    files = emit_react(source)
    assert len(files) == 1
    return files[0]["content"]


def component(name: str, slots: str = "", tree: str = "Text {}") -> str:
    """Build a minimal component source."""
    return f"component {name} {{ {slots} {tree} }}"


# ---------------------------------------------------------------------------
# 1. Output file name
# ---------------------------------------------------------------------------

class TestOutputFileName:
    """Verify the generated file has the correct name."""

    def test_filename_is_component_tsx(self) -> None:
        files = emit_react(component("Label"))
        assert files[0]["filename"] == "Label.tsx"

    def test_filename_uses_component_name(self) -> None:
        files = emit_react(component("ProfileCard"))
        assert files[0]["filename"] == "ProfileCard.tsx"

    def test_returns_list_with_one_file(self) -> None:
        files = emit_react(component("X"))
        assert isinstance(files, list)
        assert len(files) == 1


# ---------------------------------------------------------------------------
# 2. React import statement
# ---------------------------------------------------------------------------

class TestReactImport:
    """Verify the React import is present."""

    def test_react_import(self) -> None:
        content = run(component("X"))
        assert 'import React from "react";' in content


# ---------------------------------------------------------------------------
# 3. Props interface generation
# ---------------------------------------------------------------------------

class TestPropsInterface:
    """Verify the TypeScript props interface is generated."""

    def test_interface_name(self) -> None:
        content = run(component("Label", "slot title: text;"))
        assert "interface LabelProps" in content

    def test_interface_has_slot_name(self) -> None:
        content = run(component("Label", "slot title: text;"))
        assert "title" in content

    def test_interface_has_ts_type(self) -> None:
        content = run(component("Label", "slot title: text;"))
        assert "string" in content

    def test_no_interface_when_no_slots(self) -> None:
        content = run(component("X"))
        assert "interface" not in content


# ---------------------------------------------------------------------------
# 4. Required vs optional props
# ---------------------------------------------------------------------------

class TestRequiredVsOptional:
    """Verify required slots have no ? and optional slots have ?."""

    def test_required_slot_no_optional_marker(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "title: string;" in content

    def test_optional_slot_has_question_mark(self) -> None:
        content = run(component("X", "slot cnt: number = 0;"))
        assert "cnt?" in content


# ---------------------------------------------------------------------------
# 5. TypeScript type mapping
# ---------------------------------------------------------------------------

class TestTypeScriptTypeMapping:
    """Verify Mosaic types map to correct TypeScript types."""

    def test_text_to_string(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "string" in content

    def test_number_to_number(self) -> None:
        content = run(component("X", "slot cnt: number;"))
        assert "number" in content

    def test_bool_to_boolean(self) -> None:
        content = run(component("X", "slot flag: bool;"))
        assert "boolean" in content

    def test_list_text_to_string_array(self) -> None:
        content = run(component("X", "slot items: list<text>;"))
        assert "string[]" in content

    def test_node_to_react_node(self) -> None:
        content = run(component("X", "slot content: node;"))
        assert "React.ReactNode" in content


# ---------------------------------------------------------------------------
# 6. Functional component export
# ---------------------------------------------------------------------------

class TestFunctionalComponentExport:
    """Verify the exported function component is generated."""

    def test_export_default_function(self) -> None:
        content = run(component("Label", "slot title: text;"))
        assert "export default function Label" in content

    def test_props_destructuring(self) -> None:
        content = run(component("Label", "slot title: text;"))
        assert "{ title }" in content or "title" in content

    def test_function_has_return(self) -> None:
        content = run(component("X"))
        assert "return (" in content


# ---------------------------------------------------------------------------
# 7. Primitive node → JSX mapping
# ---------------------------------------------------------------------------

class TestPrimitiveNodeMapping:
    """Verify primitive nodes produce the correct HTML elements."""

    def test_text_produces_span(self) -> None:
        content = run(component("X", tree="Text {}"))
        assert "<span" in content

    def test_column_produces_div(self) -> None:
        content = run(component("X", tree="Column {}"))
        assert "<div" in content

    def test_row_produces_div(self) -> None:
        content = run(component("X", tree="Row {}"))
        assert "<div" in content


# ---------------------------------------------------------------------------
# 8. Node properties in inline styles
# ---------------------------------------------------------------------------

class TestNodePropertiesInStyles:
    """Verify node properties appear in the JSX output."""

    def test_dimension_in_style(self) -> None:
        content = run(component("X", tree="Text { padding: 16dp; }"))
        assert "padding" in content

    def test_color_property(self) -> None:
        content = run(component("X", tree="Text { background: #fff; }"))
        assert "background" in content


# ---------------------------------------------------------------------------
# 9. Color rendering
# ---------------------------------------------------------------------------

class TestColorRendering:
    """Verify colors are rendered as rgba() strings."""

    def test_color_renders_as_rgba(self) -> None:
        content = run(component("X", tree="Text { background: #2563eb; }"))
        assert "rgba(" in content

    def test_3digit_color(self) -> None:
        content = run(component("X", tree="Text { color: #fff; }"))
        assert "rgba(255,255,255" in content


# ---------------------------------------------------------------------------
# 10. Slot ref in JSX
# ---------------------------------------------------------------------------

class TestSlotRefInJSX:
    """Verify slot refs in content properties become {varName} in JSX."""

    def test_slot_ref_in_content(self) -> None:
        content = run(component("X", "slot title: text;", "Text { content: @title; }"))
        assert "{title}" in content


# ---------------------------------------------------------------------------
# 11. When block → conditional rendering
# ---------------------------------------------------------------------------

class TestWhenBlockRendering:
    """Verify when blocks produce conditional JSX."""

    def test_when_block_produces_conditional(self) -> None:
        src = component("X", "slot show: bool;", "Column { when @show { Text {} } }")
        content = run(src)
        # Should contain either && or some conditional form
        assert "show" in content
        assert "&&" in content


# ---------------------------------------------------------------------------
# 12. Each block → .map() rendering
# ---------------------------------------------------------------------------

class TestEachBlockRendering:
    """Verify each blocks produce .map() calls."""

    def test_each_block_produces_map(self) -> None:
        src = component("X", "slot items: list<text>;",
                         "Column { each @items as item { Text {} } }")
        content = run(src)
        assert ".map(" in content

    def test_loop_variable_in_map(self) -> None:
        src = component("X", "slot items: list<text>;",
                         "Column { each @items as item { Text {} } }")
        content = run(src)
        assert "item" in content


# ---------------------------------------------------------------------------
# 13. Slot ref as child
# ---------------------------------------------------------------------------

class TestSlotRefAsChild:
    """Verify @slot; children produce {slotVar} in JSX."""

    def test_slot_ref_child_renders(self) -> None:
        src = component("X", "slot action: node;", "Column { @action; }")
        content = run(src)
        assert "action" in content


# ---------------------------------------------------------------------------
# 14. emit_react() convenience function
# ---------------------------------------------------------------------------

class TestEmitReactFunction:
    """Verify emit_react() returns correct structure."""

    def test_returns_list(self) -> None:
        files = emit_react(component("X"))
        assert isinstance(files, list)

    def test_has_filename_and_content(self) -> None:
        files = emit_react(component("X"))
        assert "filename" in files[0]
        assert "content" in files[0]

    def test_content_is_string(self) -> None:
        files = emit_react(component("X"))
        assert isinstance(files[0]["content"], str)


# ---------------------------------------------------------------------------
# 15. ReactRenderer can be run multiple times
# ---------------------------------------------------------------------------

class TestRendererReuse:
    """Verify the VM/Renderer pipeline can be run with fresh renderer each time."""

    def test_two_components_independent(self) -> None:
        src1 = component("Alpha")
        src2 = component("Beta")
        f1 = emit_react(src1)
        f2 = emit_react(src2)
        assert f1[0]["filename"] == "Alpha.tsx"
        assert f2[0]["filename"] == "Beta.tsx"

    def test_renderer_class_accessible(self) -> None:
        r = ReactRenderer()
        assert r is not None
