"""Backend-neutral 2D draw instructions.

This package defines a tiny scene model that sits between producer logic and
renderer logic.

Producer packages answer: "what should be drawn?"
Renderer packages answer: "how should that scene be serialized or painted?"

That separation keeps the architecture clean. A Code 39 package should not need
to know SVG syntax, and an SVG package should not need to know barcode rules.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol, TypeVar

__version__ = "0.1.0"

DrawMetadataValue = str | int | float | bool
DrawMetadata = dict[str, DrawMetadataValue]


@dataclass(frozen=True)
class DrawRectInstruction:
    """A filled rectangle in scene coordinates.

    Supports optional stroke and stroke_width for drawing outlined rectangles.
    When stroke is None (the default), the rectangle is filled only.  When
    stroke is set to a CSS colour string the renderer should draw an outline
    with the given stroke_width (defaulting to 1.0 when omitted).
    """

    x: int
    y: int
    width: int
    height: int
    fill: str = "#000000"
    stroke: str | None = None
    stroke_width: float | None = None
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="rect", init=False)


@dataclass(frozen=True)
class DrawTextInstruction:
    """A text label in scene coordinates.

    The optional font_weight field controls the weight of the rendered text.
    Accepted values are ``"normal"`` (default) and ``"bold"``.  Renderers
    should map this to the equivalent backend attribute (e.g. CSS
    ``font-weight``).
    """

    x: int
    y: int
    value: str
    fill: str = "#000000"
    font_family: str = "monospace"
    font_size: int = 16
    align: str = "middle"
    font_weight: str | None = None
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="text", init=False)


@dataclass(frozen=True)
class DrawLineInstruction:
    """A straight line segment between two points.

    Lines are defined by their start (x1, y1) and end (x2, y2) coordinates,
    a stroke colour, and a stroke width.  They carry no fill -- only the
    stroke is rendered.

    Example::

        draw_line(0, 0, 100, 100, stroke="#ff0000", stroke_width=2.0)

    produces a red diagonal line from the top-left corner to (100, 100).
    """

    x1: float
    y1: float
    x2: float
    y2: float
    stroke: str = "#000000"
    stroke_width: float = 1.0
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="line", init=False)


@dataclass(frozen=True)
class DrawClipInstruction:
    """A rectangular clipping region that masks its children.

    Everything drawn by the *children* instructions is clipped to the
    rectangle defined by (x, y, width, height).  Content outside that
    rectangle is hidden.

    Children are stored as a *tuple* (immutable) to preserve the frozen
    invariant of the dataclass.

    Example::

        draw_clip(10, 10, 80, 80, [
            draw_rect(0, 0, 200, 200),
        ])

    The large rectangle is clipped so only the 80x80 visible area appears.
    """

    x: float
    y: float
    width: float
    height: float
    children: tuple["DrawInstruction", ...] = ()
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="clip", init=False)


@dataclass(frozen=True)
class DrawGroupInstruction:
    """A logical grouping of child instructions."""

    children: list["DrawInstruction"]
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="group", init=False)


DrawInstruction = (
    DrawRectInstruction
    | DrawTextInstruction
    | DrawLineInstruction
    | DrawClipInstruction
    | DrawGroupInstruction
)


@dataclass(frozen=True)
class DrawScene:
    """A complete renderable scene."""

    width: int
    height: int
    instructions: list[DrawInstruction]
    background: str = "#ffffff"
    metadata: DrawMetadata = field(default_factory=dict)


OutputT = TypeVar("OutputT")


class DrawRenderer(Protocol[OutputT]):
    """Protocol implemented by render backends."""

    def render(self, scene: DrawScene) -> OutputT: ...


def draw_rect(
    x: int,
    y: int,
    width: int,
    height: int,
    fill: str = "#000000",
    metadata: DrawMetadata | None = None,
    *,
    stroke: str | None = None,
    stroke_width: float | None = None,
) -> DrawRectInstruction:
    """Construct a rectangle instruction.

    Parameters
    ----------
    stroke:
        Optional CSS colour for the rectangle outline.
    stroke_width:
        Width of the outline in scene units.  Only meaningful when
        *stroke* is also provided.
    """

    return DrawRectInstruction(
        x, y, width, height, fill, stroke, stroke_width, metadata or {}
    )


def draw_text(
    x: int,
    y: int,
    value: str,
    *,
    fill: str = "#000000",
    font_family: str = "monospace",
    font_size: int = 16,
    align: str = "middle",
    font_weight: str | None = None,
    metadata: DrawMetadata | None = None,
) -> DrawTextInstruction:
    """Construct a text instruction.

    Parameters
    ----------
    font_weight:
        Optional weight string -- ``"normal"`` or ``"bold"``.  When *None*
        (the default) the renderer uses its own default (usually normal).
    """

    return DrawTextInstruction(
        x,
        y,
        value,
        fill,
        font_family,
        font_size,
        align,
        font_weight,
        metadata or {},
    )


def draw_line(
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    stroke: str = "#000000",
    stroke_width: float = 1.0,
    metadata: DrawMetadata | None = None,
) -> DrawLineInstruction:
    """Construct a line instruction between two points."""

    return DrawLineInstruction(x1, y1, x2, y2, stroke, stroke_width, metadata or {})


def draw_clip(
    x: float,
    y: float,
    width: float,
    height: float,
    children: list[DrawInstruction],
    metadata: DrawMetadata | None = None,
) -> DrawClipInstruction:
    """Construct a clip instruction.

    The *children* list is converted to a tuple internally so that the
    resulting dataclass instance stays frozen (hashable).
    """

    return DrawClipInstruction(
        x, y, width, height, tuple(children), metadata or {}
    )


def draw_group(
    children: list[DrawInstruction],
    metadata: DrawMetadata | None = None,
) -> DrawGroupInstruction:
    """Construct a group instruction."""

    return DrawGroupInstruction(children, metadata or {})


def create_scene(
    width: int,
    height: int,
    instructions: list[DrawInstruction],
    *,
    background: str = "#ffffff",
    metadata: DrawMetadata | None = None,
) -> DrawScene:
    """Construct a complete scene."""

    return DrawScene(width, height, instructions, background, metadata or {})


def render_with(scene: DrawScene, renderer: DrawRenderer[OutputT]) -> OutputT:
    """Delegate rendering to a backend implementation."""

    return renderer.render(scene)
