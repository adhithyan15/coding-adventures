"""SVG renderer for backend-neutral draw instructions."""

from __future__ import annotations

from draw_instructions import (
    DrawGroupInstruction,
    DrawInstruction,
    DrawRectInstruction,
    DrawRenderer,
    DrawScene,
    DrawTextInstruction,
)

__version__ = "0.1.0"


def _xml_escape(value: str) -> str:
    return (
        value.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
    )


def _metadata_to_attributes(metadata: dict[str, object]) -> str:
    if not metadata:
        return ""
    return "".join(
        f' data-{key}="{_xml_escape(str(value))}"' for key, value in metadata.items()
    )


def _render_rect(instruction: DrawRectInstruction) -> str:
    return (
        f'  <rect x="{instruction.x}" y="{instruction.y}" '
        f'width="{instruction.width}" height="{instruction.height}" '
        f'fill="{_xml_escape(instruction.fill)}"'
        f"{_metadata_to_attributes(instruction.metadata)} />"
    )


def _render_text(instruction: DrawTextInstruction) -> str:
    return (
        f'  <text x="{instruction.x}" y="{instruction.y}" '
        f'text-anchor="{instruction.align}" '
        f'font-family="{_xml_escape(instruction.font_family)}" '
        f'font-size="{instruction.font_size}" '
        f'fill="{_xml_escape(instruction.fill)}"'
        f"{_metadata_to_attributes(instruction.metadata)}>"
        f"{_xml_escape(instruction.value)}</text>"
    )


def _render_group(instruction: DrawGroupInstruction) -> str:
    children = "\n".join(_render_instruction(child) for child in instruction.children)
    return f"  <g{_metadata_to_attributes(instruction.metadata)}>\n{children}\n  </g>"


def _render_instruction(instruction: DrawInstruction) -> str:
    if instruction.kind == "rect":
        return _render_rect(instruction)
    if instruction.kind == "text":
        return _render_text(instruction)
    return _render_group(instruction)


class SvgRenderer(DrawRenderer[str]):
    """Render a generic draw scene as an SVG string."""

    def render(self, scene: DrawScene) -> str:
        instructions = "\n".join(_render_instruction(item) for item in scene.instructions)
        label = _xml_escape(str(scene.metadata.get("label", "draw instructions scene")))
        return "\n".join(
            [
                f'<svg xmlns="http://www.w3.org/2000/svg" width="{scene.width}" height="{scene.height}" '
                f'viewBox="0 0 {scene.width} {scene.height}" role="img" aria-label="{label}">',
                f'  <rect x="0" y="0" width="{scene.width}" height="{scene.height}" fill="{_xml_escape(scene.background)}" />',
                instructions,
                "</svg>",
            ]
        )


SVG_RENDERER = SvgRenderer()


def render_svg(scene: DrawScene) -> str:
    """Convenience wrapper around the SVG renderer instance."""

    return SVG_RENDERER.render(scene)
