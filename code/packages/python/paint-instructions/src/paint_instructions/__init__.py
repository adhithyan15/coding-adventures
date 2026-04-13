"""Backend-neutral paint scene primitives."""

from __future__ import annotations

from dataclasses import dataclass, field

__version__ = "0.1.0"

PaintMetadataValue = str | int | float | bool
PaintMetadata = dict[str, PaintMetadataValue]


@dataclass(frozen=True)
class PaintRectInstruction:
    """A filled rectangle in scene coordinates."""

    x: int
    y: int
    width: int
    height: int
    fill: str = "#000000"
    metadata: PaintMetadata = field(default_factory=dict)

    kind: str = field(default="rect", init=False)


PaintInstruction = PaintRectInstruction


@dataclass(frozen=True)
class PaintScene:
    """A complete renderable paint scene."""

    width: int
    height: int
    instructions: list[PaintInstruction]
    background: str = "#ffffff"
    metadata: PaintMetadata = field(default_factory=dict)


def paint_rect(
    x: int,
    y: int,
    width: int,
    height: int,
    fill: str = "#000000",
    metadata: PaintMetadata | None = None,
) -> PaintRectInstruction:
    return PaintRectInstruction(x, y, width, height, fill, metadata or {})


def paint_scene(
    width: int,
    height: int,
    instructions: list[PaintInstruction],
    background: str = "#ffffff",
    metadata: PaintMetadata | None = None,
) -> PaintScene:
    return PaintScene(width, height, instructions, background, metadata or {})


def create_scene(
    width: int,
    height: int,
    instructions: list[PaintInstruction],
    background: str = "#ffffff",
    metadata: PaintMetadata | None = None,
) -> PaintScene:
    return paint_scene(width, height, instructions, background, metadata)
