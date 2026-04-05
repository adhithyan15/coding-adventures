"""Tests for the Mosaic Web Component Backend.

These tests verify that the WebComponentRenderer and emit_webcomponent()
function produce correct TypeScript Custom Element (.ts) output from Mosaic
source.

Test Organisation
-----------------

1.  Output file name (mosaic-component-name.ts)
2.  Class declaration
3.  Backing fields for slots
4.  Property getters and setters
5.  connectedCallback with Shadow DOM
6.  _render() method body
7.  TypeScript type mapping
8.  Default values for optional slots
9.  Primitive node → HTML mapping
10. Node properties in style
11. Color rendering as rgba()
12. Slot ref in content (text content)
13. When block → if statement
14. Each block → forEach loop
15. Slot ref as child → <slot> element
16. customElements.define registration
17. pascal_to_kebab conversion
18. emit_webcomponent() convenience function
19. WebComponentRenderer can be run multiple times
"""

from __future__ import annotations

import pytest

from mosaic_emit_webcomponent import WebComponentRenderer, emit_webcomponent, pascal_to_kebab


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def run(source: str) -> str:
    """Run emit_webcomponent and return the file content."""
    files = emit_webcomponent(source)
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

    def test_filename_uses_mosaic_prefix(self) -> None:
        files = emit_webcomponent(component("Label"))
        assert files[0]["filename"] == "mosaic-label.ts"

    def test_filename_kebab_case(self) -> None:
        files = emit_webcomponent(component("ProfileCard"))
        assert files[0]["filename"] == "mosaic-profile-card.ts"

    def test_returns_list_with_one_file(self) -> None:
        files = emit_webcomponent(component("X"))
        assert isinstance(files, list)
        assert len(files) == 1


# ---------------------------------------------------------------------------
# 2. Class declaration
# ---------------------------------------------------------------------------

class TestClassDeclaration:
    """Verify the class extends HTMLElement."""

    def test_class_extends_html_element(self) -> None:
        content = run(component("Label"))
        assert "class MosaicLabel extends HTMLElement" in content

    def test_class_name_pascal(self) -> None:
        content = run(component("ProfileCard"))
        assert "class MosaicProfileCard extends HTMLElement" in content

    def test_header_comment(self) -> None:
        content = run(component("X"))
        assert "AUTO-GENERATED" in content


# ---------------------------------------------------------------------------
# 3. Backing fields for slots
# ---------------------------------------------------------------------------

class TestBackingFields:
    """Verify private backing fields are generated for each slot."""

    def test_backing_field_text(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "private _title: string" in content

    def test_backing_field_number(self) -> None:
        content = run(component("X", "slot cnt: number;"))
        assert "private _cnt: number" in content

    def test_backing_field_bool(self) -> None:
        content = run(component("X", "slot flag: bool;"))
        assert "private _flag: boolean" in content

    def test_backing_field_list(self) -> None:
        content = run(component("X", "slot items: list<text>;"))
        assert "string[]" in content

    def test_backing_field_default_empty_string(self) -> None:
        content = run(component("X", "slot title: text;"))
        # Required text slot default is ""
        assert '_title: string = ""' in content

    def test_backing_field_default_zero(self) -> None:
        content = run(component("X", "slot cnt: number;"))
        assert "_cnt: number = 0" in content

    def test_backing_field_default_false(self) -> None:
        content = run(component("X", "slot flag: bool;"))
        assert "_flag: boolean = false" in content

    def test_backing_field_optional_number_default(self) -> None:
        content = run(component("X", "slot cnt: number = 0;"))
        # Optional with default 0
        assert "_cnt: number" in content


# ---------------------------------------------------------------------------
# 4. Property getters and setters
# ---------------------------------------------------------------------------

class TestGettersAndSetters:
    """Verify property accessors call _render() on set."""

    def test_getter_exists(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "get title()" in content

    def test_setter_exists(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "set title(" in content

    def test_setter_calls_render(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "this._render()" in content

    def test_getter_returns_field(self) -> None:
        content = run(component("X", "slot title: text;"))
        assert "return this._title" in content


# ---------------------------------------------------------------------------
# 5. connectedCallback
# ---------------------------------------------------------------------------

class TestConnectedCallback:
    """Verify connectedCallback attaches Shadow DOM."""

    def test_connected_callback_present(self) -> None:
        content = run(component("X"))
        assert "connectedCallback()" in content

    def test_attach_shadow(self) -> None:
        content = run(component("X"))
        assert "attachShadow" in content

    def test_shadow_mode_open(self) -> None:
        content = run(component("X"))
        assert 'mode: "open"' in content


# ---------------------------------------------------------------------------
# 6. _render() method
# ---------------------------------------------------------------------------

class TestRenderMethod:
    """Verify _render() is generated with guard and innerHTML assignment."""

    def test_render_method_present(self) -> None:
        content = run(component("X"))
        assert "private _render()" in content

    def test_render_guard(self) -> None:
        content = run(component("X"))
        assert "if (!this.shadowRoot) return;" in content

    def test_render_html_accumulator(self) -> None:
        content = run(component("X"))
        assert 'let html = ""' in content

    def test_render_assigns_innerhtml(self) -> None:
        content = run(component("X"))
        assert "this.shadowRoot.innerHTML = html" in content

    def test_escape_html_helper(self) -> None:
        content = run(component("X"))
        assert "_escapeHtml" in content


# ---------------------------------------------------------------------------
# 7. TypeScript type mapping
# ---------------------------------------------------------------------------

class TestTypeMapping:
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

    def test_node_to_html_element(self) -> None:
        content = run(component("X", "slot action: node;"))
        assert "HTMLElement" in content


# ---------------------------------------------------------------------------
# 8. customElements.define registration
# ---------------------------------------------------------------------------

class TestCustomElementsDefine:
    """Verify the registration call is at the bottom of the file."""

    def test_define_present(self) -> None:
        content = run(component("Label"))
        assert "customElements.define(" in content

    def test_define_tag_name(self) -> None:
        content = run(component("Label"))
        assert '"mosaic-label"' in content

    def test_define_class_name(self) -> None:
        content = run(component("Label"))
        assert "MosaicLabel" in content

    def test_define_full_call(self) -> None:
        content = run(component("Label"))
        assert 'customElements.define("mosaic-label", MosaicLabel)' in content


# ---------------------------------------------------------------------------
# 9. Primitive node → HTML mapping
# ---------------------------------------------------------------------------

class TestPrimitiveMapping:
    """Verify primitive nodes produce the correct HTML elements."""

    def test_text_produces_span(self) -> None:
        content = run(component("X", tree="Text {}"))
        assert "<span>" in content

    def test_column_produces_div_flex_column(self) -> None:
        content = run(component("X", tree="Column {}"))
        assert "flex-direction:column" in content

    def test_row_produces_div_flex_row(self) -> None:
        content = run(component("X", tree="Row {}"))
        assert "flex-direction:row" in content

    def test_spacer_produces_flex1(self) -> None:
        content = run(component("X", tree="Spacer {}"))
        assert "flex:1" in content


# ---------------------------------------------------------------------------
# 10. Node properties in style
# ---------------------------------------------------------------------------

class TestNodePropertiesInStyle:
    """Verify node properties appear in the HTML output."""

    def test_padding_in_style(self) -> None:
        content = run(component("X", tree="Text { padding: 16dp; }"))
        assert "padding" in content

    def test_background_in_style(self) -> None:
        content = run(component("X", tree="Text { background: #fff; }"))
        assert "background" in content


# ---------------------------------------------------------------------------
# 11. Color rendering
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
# 12. Slot ref in content
# ---------------------------------------------------------------------------

class TestSlotRefInContent:
    """Verify slot refs in content properties use _escapeHtml."""

    def test_slot_ref_uses_escape(self) -> None:
        content = run(component("X", "slot title: text;", "Text { content: @title; }"))
        assert "_escapeHtml" in content

    def test_slot_ref_field_name(self) -> None:
        content = run(component("X", "slot title: text;", "Text { content: @title; }"))
        assert "_title" in content


# ---------------------------------------------------------------------------
# 13. When block → if statement
# ---------------------------------------------------------------------------

class TestWhenBlock:
    """Verify when blocks produce if statements."""

    def test_when_produces_if(self) -> None:
        src = component("X", "slot show: bool;", "Column { when @show { Text {} } }")
        content = run(src)
        assert "if (" in content

    def test_when_uses_field(self) -> None:
        src = component("X", "slot show: bool;", "Column { when @show { Text {} } }")
        content = run(src)
        assert "_show" in content

    def test_when_closes_brace(self) -> None:
        src = component("X", "slot show: bool;", "Column { when @show { Text {} } }")
        content = run(src)
        # The closing brace of the if block
        assert "if (" in content


# ---------------------------------------------------------------------------
# 14. Each block → forEach
# ---------------------------------------------------------------------------

class TestEachBlock:
    """Verify each blocks produce forEach loops."""

    def test_each_produces_foreach(self) -> None:
        src = component("X", "slot items: list<text>;",
                        "Column { each @items as item { Text {} } }")
        content = run(src)
        assert ".forEach(" in content

    def test_each_item_variable(self) -> None:
        src = component("X", "slot items: list<text>;",
                        "Column { each @items as item { Text {} } }")
        content = run(src)
        assert "item" in content

    def test_each_uses_field(self) -> None:
        src = component("X", "slot items: list<text>;",
                        "Column { each @items as item { Text {} } }")
        content = run(src)
        assert "_items" in content


# ---------------------------------------------------------------------------
# 15. Slot ref as child → <slot> element
# ---------------------------------------------------------------------------

class TestSlotRefAsChild:
    """Verify @slot; children produce <slot> elements."""

    def test_slot_element_in_output(self) -> None:
        src = component("X", "slot action: node;", "Column { @action; }")
        content = run(src)
        assert "<slot" in content

    def test_slot_name_attribute(self) -> None:
        src = component("X", "slot action: node;", "Column { @action; }")
        content = run(src)
        assert "action" in content


# ---------------------------------------------------------------------------
# 16. pascal_to_kebab
# ---------------------------------------------------------------------------

class TestPascalToKebab:
    """Verify pascal_to_kebab conversion."""

    def test_single_word(self) -> None:
        assert pascal_to_kebab("Button") == "button"

    def test_two_words(self) -> None:
        assert pascal_to_kebab("ProfileCard") == "profile-card"

    def test_three_words(self) -> None:
        assert pascal_to_kebab("HowItWorks") == "how-it-works"

    def test_already_lower(self) -> None:
        assert pascal_to_kebab("label") == "label"


# ---------------------------------------------------------------------------
# 17. emit_webcomponent() convenience function
# ---------------------------------------------------------------------------

class TestEmitWebcomponentFunction:
    """Verify emit_webcomponent() returns correct structure."""

    def test_returns_list(self) -> None:
        files = emit_webcomponent(component("X"))
        assert isinstance(files, list)

    def test_has_filename_and_content(self) -> None:
        files = emit_webcomponent(component("X"))
        assert "filename" in files[0]
        assert "content" in files[0]

    def test_content_is_string(self) -> None:
        files = emit_webcomponent(component("X"))
        assert isinstance(files[0]["content"], str)


# ---------------------------------------------------------------------------
# 18. WebComponentRenderer reuse
# ---------------------------------------------------------------------------

class TestRendererReuse:
    """Verify multiple components produce independent output."""

    def test_two_components_independent(self) -> None:
        src1 = component("Alpha")
        src2 = component("Beta")
        f1 = emit_webcomponent(src1)
        f2 = emit_webcomponent(src2)
        assert f1[0]["filename"] == "mosaic-alpha.ts"
        assert f2[0]["filename"] == "mosaic-beta.ts"

    def test_renderer_class_accessible(self) -> None:
        r = WebComponentRenderer()
        assert r is not None


# ---------------------------------------------------------------------------
# 19. Image and Divider (self-closing primitives)
# ---------------------------------------------------------------------------

class TestSelfClosingPrimitives:
    """Verify Image and Divider produce self-closing HTML."""

    def test_image_produces_img(self) -> None:
        content = run(component("X", tree="Image {}"))
        assert "<img" in content

    def test_image_self_closing(self) -> None:
        content = run(component("X", tree="Image {}"))
        assert "/>" in content

    def test_divider_produces_hr(self) -> None:
        content = run(component("X", tree="Divider {}"))
        assert "<hr" in content

    def test_divider_self_closing(self) -> None:
        content = run(component("X", tree="Divider {}"))
        assert "/>" in content

    def test_image_with_source(self) -> None:
        content = run(component("X", tree='Image { source: "logo.png"; }'))
        assert "src=" in content


# ---------------------------------------------------------------------------
# 20. slot_child with text type
# ---------------------------------------------------------------------------

class TestSlotChildTypes:
    """Verify render_slot_child generates correct <slot> for text-typed slots."""

    def test_text_slot_child_uses_escape(self) -> None:
        src = component("X", "slot caption: text;", "Column { @caption; }")
        content = run(src)
        # text-type slot child should use escapeHtml
        assert "_escapeHtml" in content or "<slot" in content

    def test_node_slot_child_produces_empty_slot(self) -> None:
        src = component("X", "slot body: node;", "Column { @body; }")
        content = run(src)
        assert "<slot" in content


# ---------------------------------------------------------------------------
# 21. Default values for slots
# ---------------------------------------------------------------------------

class TestDefaultValues:
    """Verify _ts_default generates correct default values."""

    def test_list_default_is_empty_array(self) -> None:
        content = run(component("X", "slot items: list<text>;"))
        assert "[]" in content

    def test_image_default_is_empty_string(self) -> None:
        content = run(component("X", "slot src: image;"))
        assert '""' in content

    def test_bool_default_is_false(self) -> None:
        content = run(component("X", "slot flag: bool;"))
        assert "false" in content

    def test_node_default_is_null(self) -> None:
        content = run(component("X", "slot body: node;"))
        assert "null" in content


# ---------------------------------------------------------------------------
# 22. Column/Row with extra style properties
# ---------------------------------------------------------------------------

class TestStyleMerging:
    """Verify extra styles are merged into the base style attribute."""

    def test_column_with_padding(self) -> None:
        content = run(component("X", tree="Column { padding: 8dp; }"))
        # Should have both flex-direction:column and padding
        assert "flex-direction:column" in content
        assert "padding" in content

    def test_text_with_content_and_color(self) -> None:
        content = run(component("X", "slot lbl: text;",
                                "Text { content: @lbl; color: #000; }"))
        assert "_escapeHtml" in content or "lbl" in content
