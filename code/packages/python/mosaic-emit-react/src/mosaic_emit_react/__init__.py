"""mosaic-emit-react — React backend: emits TSX functional components from MosaicIR.

This is the React backend for the Mosaic compiler. It implements the
``MosaicRenderer`` interface and is driven by ``MosaicVM``. Every time the VM
traverses the Mosaic IR tree, it calls methods on this renderer; the renderer
accumulates JSX strings and finalizes them when ``emit()`` is called.

Architecture: String Stack
--------------------------

The renderer maintains a **stack of string-line buffers**, one per open node.
When ``begin_node`` is called, a new buffer is pushed. When ``end_node`` is
called, the buffer is popped, wrapped in a JSX element string, and appended to
the parent buffer. This pattern handles arbitrary nesting without lookahead::

    begin_component("Card")         → stack: [component-lines]
    begin_node("Column")            → stack: [component-lines, column-lines]
    begin_node("Text")              → stack: [component-lines, column-lines, text-lines]
    end_node("Text")                → pop text-lines → "<span>...</span>"
                                       append to column-lines
    end_node("Column")              → pop column-lines → "<div>...<span>...</span></div>"
                                       append to component-lines
    end_component()                 → no-op; component-lines holds root JSX
    emit()                          → wrap in full function file

Output File Structure
---------------------

The generated ``.tsx`` file contains:

1. File header (auto-generated warning)
2. ``import React from "react";``
3. Props interface (``interface ComponentNameProps { ... }``)
4. Exported function component with destructured props

Primitive Node → JSX Element Mapping
-------------------------------------

+----------+------------------------------------------+
| Mosaic   | JSX element                              |
+==========+==========================================+
| Box      | ``<div style={{position:'relative'}}>``  |
+----------+------------------------------------------+
| Column   | ``<div style={{display:'flex',flexDirection:'column'}}>``  |
+----------+------------------------------------------+
| Row      | ``<div style={{display:'flex',flexDirection:'row'}}>``     |
+----------+------------------------------------------+
| Text     | ``<span>``                               |
+----------+------------------------------------------+
| Image    | ``<img ... />`` (self-closing)           |
+----------+------------------------------------------+
| Spacer   | ``<div style={{flex:1}}>``               |
+----------+------------------------------------------+
| Scroll   | ``<div style={{overflow:'auto'}}>``      |
+----------+------------------------------------------+
| Divider  | ``<hr />``                               |
+----------+------------------------------------------+
| Icon     | ``<span className="icon">``              |
+----------+------------------------------------------+
| Stack    | ``<div style={{position:'relative'}}>``  |
+----------+------------------------------------------+

Color Rendering
---------------

Colors from the VM are always emitted as ``rgba(r, g, b, a/255)`` CSS syntax.

Usage::

    from mosaic_analyzer import analyze
    from mosaic_vm import MosaicVM
    from mosaic_emit_react import ReactRenderer

    ir = analyze(source)
    vm = MosaicVM(ir)
    files = vm.run(ReactRenderer())
    # files[0] = {"filename": "ComponentName.tsx", "content": "..."}
"""

from __future__ import annotations

from mosaic_analyzer import MosaicSlot
from mosaic_vm import MosaicRenderer, MosaicVM, SlotContext

__version__ = "0.1.0"

__all__ = [
    "ReactRenderer",
    "emit_react",
]

# ---------------------------------------------------------------------------
# Primitive → HTML tag mapping
# ---------------------------------------------------------------------------

_SELF_CLOSING = frozenset(["Image", "Divider"])

_PRIMITIVE_TAG: dict[str, str] = {
    "Box":     "div",
    "Column":  "div",
    "Row":     "div",
    "Text":    "span",
    "Image":   "img",
    "Spacer":  "div",
    "Scroll":  "div",
    "Divider": "hr",
    "Icon":    "span",
    "Stack":   "div",
}

_PRIMITIVE_STYLE: dict[str, str] = {
    "Box":    "position:'relative'",
    "Column": "display:'flex',flexDirection:'column'",
    "Row":    "display:'flex',flexDirection:'row'",
    "Spacer": "flex:1",
    "Scroll": "overflow:'auto'",
    "Stack":  "position:'relative'",
    "Icon":   "",
}


# ---------------------------------------------------------------------------
# ReactRenderer
# ---------------------------------------------------------------------------


class ReactRenderer(MosaicRenderer):
    """Emits a TypeScript React functional component (.tsx).

    Each component maps to one ``.tsx`` file with:

    - An auto-generated header comment
    - ``import React from "react";``
    - A ``ComponentNameProps`` interface for all slots
    - An exported default function with destructured props

    Example output for ``component Label { slot text: text; Text { content: @text; } }``::

        // AUTO-GENERATED — do not edit by hand
        import React from "react";

        export interface LabelProps {
          text: string;
        }

        export default function Label({ text }: LabelProps) {
          return (
            <span>{text}</span>
          );
        }
    """

    def __init__(self) -> None:
        self._component_name = ""
        self._slots: list[MosaicSlot] = []
        # Stack of line-lists; each entry is the buffer for one open node.
        self._stack: list[list[str]] = []
        # Lines accumulated at the component level (conditional/loop wrappers)
        self._component_lines: list[str] = []
        # Indent depth counter (for when/each blocks)
        self._indent = 2

    # -------------------------------------------------------------------------
    # MosaicRenderer protocol
    # -------------------------------------------------------------------------

    def begin_component(self, name: str, slots: list[MosaicSlot]) -> None:
        self._component_name = name
        self._slots = slots
        self._stack = []
        self._component_lines = []

    def end_component(self) -> None:
        pass  # All content is on the stack; emit() finalizes.

    def begin_node(
        self,
        tag: str,
        is_primitive: bool,
        properties: list[dict],
        context: SlotContext,
    ) -> None:
        lines: list[str] = []
        self._stack.append(lines)

        if is_primitive:
            open_tag = self._render_primitive_open(tag, properties)
        else:
            # Non-primitive: render as a React component (PascalCase)
            props_str = self._render_component_props(properties)
            open_tag = f"<{tag}{props_str}>"

        lines.append(open_tag)

    def end_node(self, tag: str) -> None:
        lines = self._stack.pop()

        # Determine the closing tag
        if tag in _SELF_CLOSING:
            # Already self-closed in begin_node; no closing needed.
            # Replace the open tag with a self-closed variant.
            if lines and lines[-1].endswith(">"):
                lines[-1] = lines[-1][:-1] + " />"
            content = "".join(lines)
        else:
            html_tag = _PRIMITIVE_TAG.get(tag, tag)
            close = f"</{html_tag}>" if tag in _PRIMITIVE_TAG else f"</{tag}>"
            # Gather lines[1:] as inner content
            inner = "".join(lines[1:])
            open_tag = lines[0]
            if inner:
                content = f"{open_tag}{inner}{close}"
            else:
                content = f"{open_tag}{close}"

        if self._stack:
            self._stack[-1].append(content)
        else:
            self._component_lines.append(content)

    def render_slot_child(
        self, slot_name: str, slot_type: dict, context: SlotContext
    ) -> None:
        jsx = f"{{{_camel(slot_name)}}}"
        if self._stack:
            self._stack[-1].append(jsx)
        else:
            self._component_lines.append(jsx)

    def begin_when(self, slot_name: str, context: SlotContext) -> None:
        jsx = f"{{{_camel(slot_name)} && ("
        if self._stack:
            self._stack[-1].append(jsx)
        else:
            self._component_lines.append(jsx)

    def end_when(self) -> None:
        jsx = ")}"
        if self._stack:
            self._stack[-1].append(jsx)
        else:
            self._component_lines.append(jsx)

    def begin_each(
        self,
        slot_name: str,
        item_name: str,
        element_type: dict,
        context: SlotContext,
    ) -> None:
        jsx = f"{{{_camel(slot_name)}.map(({item_name}, i) => ("
        if self._stack:
            self._stack[-1].append(jsx)
        else:
            self._component_lines.append(jsx)

    def end_each(self) -> None:
        jsx = "))}"
        if self._stack:
            self._stack[-1].append(jsx)
        else:
            self._component_lines.append(jsx)

    def emit(self) -> list[dict]:
        """Build the complete .tsx file and return it as a single-element list."""
        name = self._component_name
        props_interface = self._build_props_interface()
        params = ", ".join(_camel(s.name) for s in self._slots)
        props_type = f"{name}Props" if self._slots else ""

        body_jsx = "".join(self._component_lines)

        lines = [
            "// AUTO-GENERATED by mosaic-emit-react — do not edit by hand",
            'import React from "react";',
            "",
        ]

        if props_interface:
            lines.append(props_interface)
            lines.append("")

        param_str = f"{{ {params} }}: {props_type}" if self._slots else ""
        lines.append(f"export default function {name}({param_str}) {{")
        lines.append("  return (")
        lines.append(f"    {body_jsx}")
        lines.append("  );")
        lines.append("}")
        lines.append("")

        content = "\n".join(lines)
        return [{"filename": f"{name}.tsx", "content": content}]

    # -------------------------------------------------------------------------
    # Private helpers
    # -------------------------------------------------------------------------

    def _build_props_interface(self) -> str:
        if not self._slots:
            return ""
        name = self._component_name
        parts = [f"export interface {name}Props {{"]
        for slot in self._slots:
            ts_type = _mosaic_type_to_ts(slot.type)
            optional = "?" if not slot.required else ""
            parts.append(f"  {_camel(slot.name)}{optional}: {ts_type};")
        parts.append("}")
        return "\n".join(parts)

    def _render_primitive_open(self, tag: str, properties: list[dict]) -> str:
        """Build the JSX open tag for a primitive node."""
        html_tag = _PRIMITIVE_TAG.get(tag, "div")
        base_style = _PRIMITIVE_STYLE.get(tag, "")

        style_parts: list[str] = []
        attrs: list[str] = []
        text_content: str | None = None

        if base_style:
            style_parts.append(base_style)

        for prop in properties:
            pname = prop["name"]
            pval = prop["value"]
            rendered = _render_value(pval)

            if pname == "content":
                text_content = rendered
            elif pname == "source":
                attrs.append(f'src={rendered}')
            elif pname.startswith("a11y-"):
                aria_name = "aria-" + pname[5:]
                attrs.append(f'{aria_name}={rendered}')
            elif pname == "style" and pval.get("kind") == "enum":
                ns = pval.get("namespace", "")
                mem = pval.get("member", "")
                attrs.append(f'className="mosaic-{ns}-{mem}"')
            elif pname in ("background", "color", "border-color"):
                css_prop = _kebab_to_camel(pname)
                style_parts.append(f"{css_prop}:{rendered}")
            elif pname.endswith("-size") or pname in ("padding", "margin",
                                                        "corner-radius",
                                                        "padding-top", "padding-bottom",
                                                        "padding-left", "padding-right"):
                css_prop = _kebab_to_camel(pname)
                style_parts.append(f"{css_prop}:{rendered}")
            else:
                css_prop = _kebab_to_camel(pname)
                style_parts.append(f"{css_prop}:{rendered}")

        style_str = ""
        if style_parts:
            style_str = f' style={{{{{",".join(style_parts)}}}}}'

        attrs_str = (" " + " ".join(attrs)) if attrs else ""

        if text_content is not None:
            return f"<{html_tag}{style_str}{attrs_str}>{text_content}"
        return f"<{html_tag}{style_str}{attrs_str}>"

    def _render_component_props(self, properties: list[dict]) -> str:
        """Render JSX props for a non-primitive (imported) component."""
        if not properties:
            return ""
        parts = []
        for prop in properties:
            pname = _camel(prop["name"])
            pval = _render_value(prop["value"])
            parts.append(f" {pname}={pval}")
        return "".join(parts)


# ---------------------------------------------------------------------------
# Convenience function
# ---------------------------------------------------------------------------


def emit_react(source: str) -> list[dict]:
    """Analyze Mosaic source and emit React TSX files.

    This is the all-in-one entry point. Pass in ``.mosaic`` source text and
    get back a list of generated files.

    Args:
        source: The ``.mosaic`` source text.

    Returns:
        A list of ``{"filename": ..., "content": ...}`` dicts.

    Example::

        files = emit_react('''
            component Label {
                slot text: text;
                Text { content: @text; }
            }
        ''')
        print(files[0]["filename"])  # "Label.tsx"
    """
    from mosaic_analyzer import analyze

    ir = analyze(source)
    vm = MosaicVM(ir)
    return vm.run(ReactRenderer())


# ---------------------------------------------------------------------------
# Value rendering helpers
# ---------------------------------------------------------------------------


def _render_value(v: dict) -> str:
    """Render a resolved value dict into a JSX expression string."""
    kind = v.get("kind")

    if kind == "string":
        return f'"{v["value"]}"'
    if kind == "number":
        return f'{{{v["value"]}}}'
    if kind == "bool":
        return f'{{{str(v["value"]).lower()}}}'
    if kind == "dimension":
        unit = v["unit"]
        val = v["value"]
        if unit in ("dp", "sp"):
            return f"{{{int(val) if val == int(val) else val}}}"
        return f'"{val}{unit}"'
    if kind == "color":
        r, g, b, a = v["r"], v["g"], v["b"], v["a"]
        alpha = a / 255.0
        return f'"rgba({r},{g},{b},{alpha:.3f})"'
    if kind == "slot_ref":
        return f'{{{_camel(v["slot_name"])}}}'
    if kind == "enum":
        return f'"{v["namespace"]}-{v["member"]}"'

    return f'"{v.get("value", "")}"'


def _mosaic_type_to_ts(t: dict) -> str:
    """Convert a MosaicType dict to a TypeScript type annotation string."""
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
        return "React.ReactNode"
    if kind == "component":
        return f'React.ReactNode'
    if kind == "list":
        inner = _mosaic_type_to_ts(t["element_type"])
        return f"{inner}[]"
    return "unknown"


def _camel(kebab: str) -> str:
    """Convert kebab-case to camelCase.

    Examples::

        _camel("avatar-url")   → "avatarUrl"
        _camel("display-name") → "displayName"
        _camel("title")        → "title"
    """
    parts = kebab.split("-")
    return parts[0] + "".join(p.capitalize() for p in parts[1:])


def _kebab_to_camel(prop: str) -> str:
    """Convert a CSS kebab-case property name to camelCase for React style objects."""
    return _camel(prop)
