"""mosaic-emit-webcomponent — Web Components backend: emits Custom Element classes.

This is the Web Components backend for the Mosaic compiler. It implements the
``MosaicRenderer`` interface and is driven by ``MosaicVM``. The renderer
produces a single TypeScript file containing a Custom Element that:

- Extends ``HTMLElement``
- Uses Shadow DOM for style encapsulation
- Exposes Mosaic slots as property setters/getters
- Rebuilds shadow DOM content via ``_render()`` on any property change
- Observes HTML attributes for primitive (text/number/bool/image/color) slots

Architecture: ``html +=`` Accumulation
---------------------------------------

The renderer accumulates HTML fragment strings during VM traversal and
serializes them into a ``_render()`` method body during ``emit()``.

The ``_render()`` method uses a mutable ``let html = ''`` accumulator::

    html += '<div style="display:flex;flex-direction:column">';
    html += `<span>${this._escapeHtml(this._title)}</span>`;
    html += '</div>';

Tag Name Convention
-------------------

PascalCase component names map to kebab-case element names with a ``mosaic-`` prefix:

    ``ProfileCard`` → ``<mosaic-profile-card>``
    ``Button``      → ``<mosaic-button>``
    ``HowItWorks``  → ``<mosaic-how-it-works>``

Security
--------

All text slot values are passed through ``_escapeHtml()`` before insertion.
Color values are always emitted as ``rgba()`` strings (never raw user strings).

Primitive Node → HTML Mapping
------------------------------

+----------+--------------------------------------------------+
| Mosaic   | HTML output                                      |
+==========+==================================================+
| Column   | ``<div style="display:flex;flex-direction:column">`` |
+----------+--------------------------------------------------+
| Row      | ``<div style="display:flex;flex-direction:row">``|
+----------+--------------------------------------------------+
| Box      | ``<div style="position:relative">``              |
+----------+--------------------------------------------------+
| Stack    | ``<div style="position:relative">``              |
+----------+--------------------------------------------------+
| Text     | ``<span>``                                       |
+----------+--------------------------------------------------+
| Image    | ``<img src="..." />`` (self-closing)             |
+----------+--------------------------------------------------+
| Spacer   | ``<div style="flex:1">``                         |
+----------+--------------------------------------------------+
| Scroll   | ``<div style="overflow:auto">``                  |
+----------+--------------------------------------------------+
| Divider  | ``<hr />``                                       |
+----------+--------------------------------------------------+
| Icon     | ``<span class="icon">``                          |
+----------+--------------------------------------------------+

Usage::

    from mosaic_analyzer import analyze
    from mosaic_vm import MosaicVM
    from mosaic_emit_webcomponent import WebComponentRenderer

    ir = analyze(source)
    vm = MosaicVM(ir)
    files = vm.run(WebComponentRenderer())
    # files[0] = {"filename": "mosaic-component-name.ts", "content": "..."}
"""

from __future__ import annotations

import re

from mosaic_analyzer import MosaicSlot
from mosaic_vm import MosaicRenderer, MosaicVM, SlotContext

__version__ = "0.1.0"

__all__ = [
    "WebComponentRenderer",
    "emit_webcomponent",
    "pascal_to_kebab",
]

# ---------------------------------------------------------------------------
# Primitive HTML mapping
# ---------------------------------------------------------------------------

_SELF_CLOSING_HTML = frozenset(["Image", "Divider"])

_PRIMITIVE_OPEN: dict[str, str] = {
    "Box":    '<div style="position:relative">',
    "Column": '<div style="display:flex;flex-direction:column">',
    "Row":    '<div style="display:flex;flex-direction:row">',
    "Text":   "<span>",
    "Spacer": '<div style="flex:1">',
    "Scroll": '<div style="overflow:auto">',
    "Stack":  '<div style="position:relative">',
    "Icon":   '<span class="icon">',
}

_PRIMITIVE_CLOSE: dict[str, str] = {
    "Box":    "</div>",
    "Column": "</div>",
    "Row":    "</div>",
    "Text":   "</span>",
    "Spacer": "</div>",
    "Scroll": "</div>",
    "Stack":  "</div>",
    "Icon":   "</span>",
}


# ---------------------------------------------------------------------------
# WebComponentRenderer
# ---------------------------------------------------------------------------


class WebComponentRenderer(MosaicRenderer):
    """Emits a TypeScript Custom Element class (.ts).

    Each component maps to one ``.ts`` file with:

    - An auto-generated header comment
    - A ``MosaicComponentName extends HTMLElement`` class
    - Private backing fields for each slot
    - Property getters and setters that call ``_render()`` on change
    - A ``_render()`` method that builds the Shadow DOM HTML string
    - ``customElements.define(...)`` registration at the bottom

    Example output for ``component Label { slot title: text; Text { content: @title; } }``::

        // AUTO-GENERATED by mosaic-emit-webcomponent — do not edit by hand
        class MosaicLabel extends HTMLElement {
          private _title: string = "";

          get title() { return this._title; }
          set title(v: string) { this._title = v; this._render(); }

          connectedCallback() { this.attachShadow({ mode: "open" }); this._render(); }

          private _render() {
            if (!this.shadowRoot) return;
            let html = "";
            html += "<span>";
            html += `${this._escapeHtml(this._title)}`;
            html += "</span>";
            this.shadowRoot.innerHTML = html;
          }

          private _escapeHtml(s: string): string {
            return String(s)
              .replace(/&/g, "&amp;")
              .replace(/</g, "&lt;")
              .replace(/>/g, "&gt;")
              .replace(/"/g, "&quot;");
          }
        }
        customElements.define("mosaic-label", MosaicLabel);
    """

    def __init__(self) -> None:
        self._component_name = ""
        self._tag_name = ""
        self._slots: list[MosaicSlot] = []
        # Accumulated render fragments as html += lines
        self._render_lines: list[str] = []
        # Stack of open tag names (for proper closing)
        self._tag_stack: list[str] = []
        # Text content for the current Text node (set during begin_node)
        self._text_content: list[str] = []
        self._in_text = False

    # -------------------------------------------------------------------------
    # MosaicRenderer protocol
    # -------------------------------------------------------------------------

    def begin_component(self, name: str, slots: list[MosaicSlot]) -> None:
        self._component_name = name
        self._tag_name = f"mosaic-{pascal_to_kebab(name)}"
        self._slots = slots
        self._render_lines = []
        self._tag_stack = []
        self._text_content = []
        self._in_text = False

    def end_component(self) -> None:
        pass

    def begin_node(
        self,
        tag: str,
        is_primitive: bool,
        properties: list[dict],
        context: SlotContext,
    ) -> None:
        if is_primitive:
            html_line = self._primitive_open_html(tag, properties)
        else:
            # Non-primitive: render as a nested custom element
            kebab = f"mosaic-{pascal_to_kebab(tag)}"
            props_str = self._render_wc_props(properties)
            html_line = f'html += `<{kebab}{props_str}>`;'

        self._render_lines.append(f"    {html_line}")
        self._tag_stack.append(tag)

    def end_node(self, tag: str) -> None:
        self._tag_stack.pop()

        if tag in _SELF_CLOSING_HTML:
            # Already self-closed in begin_node — nothing to close
            return

        if tag in _PRIMITIVE_CLOSE:
            close = _PRIMITIVE_CLOSE[tag]
            self._render_lines.append(f'    html += "{close}";')
        else:
            kebab = f"mosaic-{pascal_to_kebab(tag)}"
            self._render_lines.append(f'    html += "</{kebab}>";')

    def render_slot_child(
        self, slot_name: str, slot_type: dict, context: SlotContext
    ) -> None:
        field = f"_{_camel(slot_name)}"
        kind = slot_type.get("kind", "text")

        if kind == "text":
            self._render_lines.append(
                f'    html += `<slot name="{slot_name}">${{{self._escape_expr(field)}}}</slot>`;'
            )
        elif kind == "node":
            self._render_lines.append(
                f'    html += \'<slot name="{slot_name}"></slot>\';'
            )
        else:
            self._render_lines.append(
                f'    html += \'<slot name="{slot_name}"></slot>\';'
            )

    def begin_when(self, slot_name: str, context: SlotContext) -> None:
        field = f"_{_camel(slot_name)}"
        self._render_lines.append(f"    if ({field}) {{")

    def end_when(self) -> None:
        self._render_lines.append("    }")

    def begin_each(
        self,
        slot_name: str,
        item_name: str,
        element_type: dict,
        context: SlotContext,
    ) -> None:
        import re as _re
        field = f"_{_camel(slot_name)}"
        # Validate item_name is a safe JS identifier to prevent code injection in forEach callback
        safe_item = item_name if _re.match(r'^[a-zA-Z_$][a-zA-Z0-9_$]*$', item_name) else "_item"
        self._render_lines.append(f"    {field}.forEach(({safe_item}) => {{")

    def end_each(self) -> None:
        self._render_lines.append("    });")

    def emit(self) -> list[dict]:
        """Build the complete .ts Custom Element file."""
        name = self._component_name
        tag = self._tag_name

        lines = [
            "// AUTO-GENERATED by mosaic-emit-webcomponent — do not edit by hand",
            "",
            f"class Mosaic{name} extends HTMLElement {{",
        ]

        # Backing fields
        for slot in self._slots:
            field = f"_{_camel(slot.name)}"
            ts_type = _mosaic_type_to_ts(slot.type)
            default = _ts_default(slot.type, slot.default_value)
            lines.append(f"  private {field}: {ts_type} = {default};")

        lines.append("")

        # Getters and setters
        for slot in self._slots:
            prop = _camel(slot.name)
            field = f"_{prop}"
            ts_type = _mosaic_type_to_ts(slot.type)
            lines.append(f"  get {prop}() {{ return this.{field}; }}")
            lines.append(f"  set {prop}(v: {ts_type}) {{ this.{field} = v; this._render(); }}")

        lines.append("")
        lines.append('  connectedCallback() { this.attachShadow({ mode: "open" }); this._render(); }')
        lines.append("")

        # _render() method
        lines.append("  private _render() {")
        lines.append("    if (!this.shadowRoot) return;")
        lines.append('    let html = "";')
        lines.extend(self._render_lines)
        lines.append("    this.shadowRoot.innerHTML = html;")
        lines.append("  }")
        lines.append("")

        # _escapeHtml helper
        lines.append("  private _escapeHtml(s: string): string {")
        lines.append("    return String(s)")
        lines.append('      .replace(/&/g, "&amp;")')
        lines.append('      .replace(/</g, "&lt;")')
        lines.append('      .replace(/>/g, "&gt;")')
        lines.append('      .replace(/"/g, "&quot;")')
        lines.append("      .replace(/'/g, \"&#39;\");")
        lines.append("  }")
        lines.append("}")
        lines.append("")
        lines.append(f'customElements.define("{tag}", Mosaic{name});')
        lines.append("")

        content = "\n".join(lines)
        return [{"filename": f"{tag}.ts", "content": content}]

    # -------------------------------------------------------------------------
    # Private helpers
    # -------------------------------------------------------------------------

    def _primitive_open_html(self, tag: str, properties: list[dict]) -> str:
        """Build a ``html += '...'`` statement for a primitive node's open tag."""
        extra_style: list[str] = []
        text_content: str | None = None
        src_value: str | None = None
        attrs: list[str] = []

        for prop in properties:
            pname = prop["name"]
            pval = prop["value"]

            if pname == "content":
                text_content = self._render_wc_value(pval)
            elif pname == "source":
                raw_src = self._render_wc_value(pval)
                # Reject javascript: URLs in literal src values at code-generation time
                if raw_src.lower().strip().startswith("javascript:"):
                    raw_src = "about:blank"
                # HTML-escape the value for the src attribute (prevents " injection)
                if not raw_src.startswith("${"):
                    raw_src = raw_src.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")
                src_value = raw_src
            else:
                css = _kebab_to_css(pname)
                rendered = self._render_css_value(pval)
                if rendered:
                    extra_style.append(f"{css}:{rendered}")

        if tag in _SELF_CLOSING_HTML:
            base_style = ""
            if tag == "Image":
                src_part = f' src="{src_value}"' if src_value else ""
                style_part = (
                    f' style="{";".join(extra_style)}"' if extra_style else ""
                )
                return f'html += `<img{src_part}{style_part} />`;'
            return f'html += `<hr />`;'

        base_open = _PRIMITIVE_OPEN.get(tag, "<div>")

        # Inject extra styles into the base open tag
        if extra_style:
            style_str = ";".join(extra_style)
            if 'style="' in base_open:
                # Append to existing style
                base_open = base_open.replace('style="', f'style="{style_str};')
            else:
                # Add style attribute before >
                base_open = base_open.rstrip(">") + f' style="{style_str}">'

        if text_content:
            # Inline text content for Text nodes — HTML-escape literal text to prevent XSS
            # via shadowRoot.innerHTML; slot_ref values are already wrapped in _escapeHtml()
            if not text_content.startswith("${"):
                text_content = text_content.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            return f"html += `{base_open}{text_content}`;"
        else:
            return f'html += \'{base_open}\';'

    def _render_wc_value(self, v: dict) -> str:
        """Render a value for inline HTML in the web component template string."""
        kind = v.get("kind")
        if kind == "string":
            return v["value"]
        if kind == "number":
            return str(v["value"])
        if kind == "bool":
            return str(v["value"]).lower()
        if kind == "slot_ref":
            field = f"_{_camel(v['slot_name'])}"
            return f"${{{self._escape_expr(field)}}}"
        if kind == "dimension":
            val = v["value"]
            unit = v["unit"]
            return f"{int(val) if val == int(val) else val}{unit}"
        if kind == "color":
            r, g, b, a = v["r"], v["g"], v["b"], v["a"]
            alpha = a / 255.0
            return f"rgba({r},{g},{b},{alpha:.3f})"
        if kind == "enum":
            return f"{v['namespace']}-{v['member']}"
        return str(v.get("value", ""))

    def _render_css_value(self, v: dict) -> str:
        """Render a CSS value string (for inline style attributes)."""
        kind = v.get("kind")
        if kind == "string":
            # Escape " to prevent breaking out of style="" HTML attribute
            return v["value"].replace('"', "&quot;")
        if kind == "number":
            return str(v["value"])
        if kind == "dimension":
            val = v["value"]
            unit = v["unit"]
            return f"{int(val) if val == int(val) else val}{unit}"
        if kind == "color":
            r, g, b, a = v["r"], v["g"], v["b"], v["a"]
            alpha = a / 255.0
            return f"rgba({r},{g},{b},{alpha:.3f})"
        if kind == "slot_ref":
            field = f"_{_camel(v['slot_name'])}"
            return f"${{{field}}}"
        if kind == "enum":
            return f"{v['namespace']}-{v['member']}"
        if kind == "ident":
            return v.get("value", "")
        return ""

    def _render_wc_props(self, properties: list[dict]) -> str:
        """Build HTML attribute string for non-primitive component props."""
        if not properties:
            return ""
        parts = []
        for prop in properties:
            attr_name = prop["name"]
            val = self._render_wc_value(prop["value"])
            parts.append(f' {attr_name}="{val}"')
        return "".join(parts)

    @staticmethod
    def _escape_expr(field: str) -> str:
        """Return the field expression wrapped in _escapeHtml for text types."""
        return f"this._escapeHtml(this.{field[1:]})"


# ---------------------------------------------------------------------------
# Convenience function
# ---------------------------------------------------------------------------


def emit_webcomponent(source: str) -> list[dict]:
    """Analyze Mosaic source and emit Web Component TypeScript files.

    This is the all-in-one entry point. Pass in ``.mosaic`` source text and
    get back a list of generated files.

    Args:
        source: The ``.mosaic`` source text.

    Returns:
        A list of ``{"filename": ..., "content": ...}`` dicts.

    Example::

        files = emit_webcomponent('''
            component Label {
                slot title: text;
                Text { content: @title; }
            }
        ''')
        print(files[0]["filename"])  # "mosaic-label.ts"
    """
    from mosaic_analyzer import analyze

    ir = analyze(source)
    vm = MosaicVM(ir)
    return vm.run(WebComponentRenderer())


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def pascal_to_kebab(name: str) -> str:
    """Convert a PascalCase name to kebab-case with a ``mosaic-`` prefix.

    The ``mosaic-`` prefix is NOT added by this function — it only converts
    the case. Callers that need the prefix add it explicitly.

    Examples::

        pascal_to_kebab("ProfileCard")  → "profile-card"
        pascal_to_kebab("Button")       → "button"
        pascal_to_kebab("HowItWorks")   → "how-it-works"
    """
    # Insert hyphen before each uppercase letter that follows a lowercase letter
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", name)
    return s.lower()


def _camel(kebab: str) -> str:
    """Convert kebab-case to camelCase."""
    parts = kebab.split("-")
    return parts[0] + "".join(p.capitalize() for p in parts[1:])


def _kebab_to_css(prop: str) -> str:
    """Return the CSS property name (already kebab-case in Mosaic)."""
    return prop


def _mosaic_type_to_ts(t: dict) -> str:
    """Convert a MosaicType dict to a TypeScript type annotation."""
    kind = t.get("kind")
    if kind == "text":
        return "string"
    if kind == "number":
        return "number"
    if kind == "bool":
        return "boolean"
    if kind == "image":
        return "string"
    if kind == "color":
        return "string"
    if kind == "node":
        return "HTMLElement | null"
    if kind == "component":
        return "HTMLElement | null"
    if kind == "list":
        inner = _mosaic_type_to_ts(t["element_type"])
        return f"{inner}[]"
    return "unknown"


def _ts_default(slot_type: dict, default_value: dict | None) -> str:
    """Return the TypeScript default value for a slot's backing field."""
    if default_value is not None:
        dkind = default_value.get("kind")
        if dkind == "string":
            return f'"{default_value["value"]}"'
        if dkind == "number":
            val = default_value["value"]
            return str(int(val) if val == int(val) else val)
        if dkind == "bool":
            return "true" if default_value["value"] else "false"
        if dkind == "color_hex":
            return f'"{default_value["value"]}"'
        if dkind == "dimension":
            val = default_value["value"]
            unit = default_value["unit"]
            return f'"{int(val) if val == int(val) else val}{unit}"'

    # No default — use type-appropriate zero value
    kind = slot_type.get("kind")
    if kind == "text":
        return '""'
    if kind == "number":
        return "0"
    if kind == "bool":
        return "false"
    if kind in ("image", "color"):
        return '""'
    if kind in ("node", "component"):
        return "null"
    if kind == "list":
        return "[]"
    return '""'
