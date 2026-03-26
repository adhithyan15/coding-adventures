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
    """A filled rectangle in scene coordinates."""

    x: int
    y: int
    width: int
    height: int
    fill: str = "#000000"
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="rect", init=False)


@dataclass(frozen=True)
class DrawTextInstruction:
    """A text label in scene coordinates."""

    x: int
    y: int
    value: str
    fill: str = "#000000"
    font_family: str = "monospace"
    font_size: int = 16
    align: str = "middle"
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="text", init=False)


@dataclass(frozen=True)
class DrawGroupInstruction:
    """A logical grouping of child instructions."""

    children: list["DrawInstruction"]
    metadata: DrawMetadata = field(default_factory=dict)

    kind: str = field(default="group", init=False)


DrawInstruction = DrawRectInstruction | DrawTextInstruction | DrawGroupInstruction


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
) -> DrawRectInstruction:
    """Construct a rectangle instruction."""

    return DrawRectInstruction(x, y, width, height, fill, metadata or {})


def draw_text(
    x: int,
    y: int,
    value: str,
    *,
    fill: str = "#000000",
    font_family: str = "monospace",
    font_size: int = 16,
    align: str = "middle",
    metadata: DrawMetadata | None = None,
) -> DrawTextInstruction:
    """Construct a text instruction."""

    return DrawTextInstruction(
        x,
        y,
        value,
        fill,
        font_family,
        font_size,
        align,
        metadata or {},
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
