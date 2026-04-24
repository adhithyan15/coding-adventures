"""Backend-neutral paint scene primitives.

This module provides the fundamental drawing instructions that the PaintVM
(P2D01) executes. Every backend — SVG, Canvas, Metal, terminal — receives
a ``PaintScene`` containing a list of these instructions and executes them
in order.

## Instruction types

- ``PaintRectInstruction`` — a filled axis-aligned rectangle.  Used by all
  square-module barcodes (QR Code, Data Matrix, Aztec Code, PDF417).

- ``PaintPathInstruction`` — a filled polygon described by a sequence of
  ``PathCommand`` objects (move_to, line_to, close).  Used by hex-module
  barcodes (MaxiCode) and any future curvilinear symbols.

## Immutability

All instruction types and ``PaintScene`` are frozen dataclasses.  This
means they can be hashed, safely shared between threads, and used as dict
keys.  Encoder code never mutates instructions; it builds new ones.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

__version__ = "0.1.0"

PaintMetadataValue = str | int | float | bool
PaintMetadata = dict[str, PaintMetadataValue]

# ============================================================================
# PaintRectInstruction — filled rectangle
# ============================================================================


@dataclass(frozen=True)
class PaintRectInstruction:
    """A filled rectangle in scene coordinates.

    Coordinates are in pixels, measured from the top-left corner of the
    scene canvas.  ``x`` and ``y`` are the top-left corner of the
    rectangle.  ``width`` and ``height`` are always positive integers.

    ``fill`` is a CSS-style colour string (e.g. ``"#000000"`` for black).

    Example — a 10×10 black square at (5, 5)::

        rect = PaintRectInstruction(x=5, y=5, width=10, height=10,
                                    fill="#000000")
    """

    x: int
    y: int
    width: int
    height: int
    fill: str = "#000000"
    metadata: PaintMetadata = field(default_factory=dict)

    # ``kind`` is a read-only tag that backends use to dispatch without
    # isinstance checks.  It is not part of the constructor.
    kind: str = field(default="rect", init=False)


# ============================================================================
# PathCommand — individual drawing verbs for PaintPathInstruction
# ============================================================================

PathCommandKind = Literal["move_to", "line_to", "close"]


@dataclass(frozen=True)
class PathCommand:
    """A single drawing verb in a path.

    Three verbs are supported:

    - ``"move_to"``  — lift the pen and place it at ``(x, y)``.
      Starts the path (or a new sub-path after ``"close"``).
    - ``"line_to"``  — draw a straight line from the current pen position
      to ``(x, y)``.
    - ``"close"``    — draw a straight line back to the last ``"move_to"``
      point, closing the current sub-path into a filled polygon.
      ``x`` and ``y`` are unused for ``"close"`` and default to ``0.0``.

    Together these three verbs can describe any convex or concave polygon,
    including the flat-top hexagons used by MaxiCode.

    Hex example (6-vertex flat-top hexagon at centre (cx, cy), circumR R)::

        commands = [
            PathCommand("move_to", cx + R, cy),          # vertex 0 (0°)
            PathCommand("line_to", cx + R*0.5, cy + R*0.866),  # vertex 1 (60°)
            PathCommand("line_to", cx - R*0.5, cy + R*0.866),  # vertex 2 (120°)
            PathCommand("line_to", cx - R, cy),          # vertex 3 (180°)
            PathCommand("line_to", cx - R*0.5, cy - R*0.866),  # vertex 4 (240°)
            PathCommand("line_to", cx + R*0.5, cy - R*0.866),  # vertex 5 (300°)
            PathCommand("close"),                         # back to vertex 0
        ]
    """

    kind: PathCommandKind
    x: float = 0.0
    y: float = 0.0


# ============================================================================
# PaintPathInstruction — filled polygon
# ============================================================================


@dataclass(frozen=True)
class PaintPathInstruction:
    """A filled polygon described by a sequence of ``PathCommand`` objects.

    Used for shapes that cannot be expressed as an axis-aligned rectangle.
    The primary use-case in this repo is MaxiCode's flat-top hexagon modules.

    The path is **filled** (not stroked).  The ``fill`` colour follows the
    same CSS-string convention as ``PaintRectInstruction``.

    ``commands`` must begin with a ``"move_to"`` verb and end with a
    ``"close"`` verb to form a closed, filled polygon.

    Example — a triangle::

        path = PaintPathInstruction(
            commands=(
                PathCommand("move_to", 10, 0),
                PathCommand("line_to", 20, 20),
                PathCommand("line_to", 0, 20),
                PathCommand("close"),
            ),
            fill="#ff0000",
        )
    """

    commands: tuple[PathCommand, ...]
    fill: str = "#000000"
    metadata: PaintMetadata = field(default_factory=dict)

    # Tag for backend dispatch — never set by the caller.
    kind: str = field(default="path", init=False)


# ============================================================================
# Union type — a single PaintInstruction is either a rect or a path
# ============================================================================

PaintInstruction = PaintRectInstruction | PaintPathInstruction

# ============================================================================
# PaintScene — the complete render-ready representation
# ============================================================================


@dataclass(frozen=True)
class PaintScene:
    """A complete renderable paint scene.

    A ``PaintScene`` is the output of any layout function (1D or 2D barcode
    layout, document layout, etc.) and the input to every PaintVM backend.

    ``instructions`` is an ordered list of drawing commands.  Instructions
    are executed in order, so later instructions paint over earlier ones.
    The first instruction is conventionally a full-canvas background
    ``PaintRectInstruction`` that fills the scene with the background colour.

    ``width`` and ``height`` are in pixels.

    ``background`` is a CSS-style colour string.  It records the intended
    background so backends that clear the canvas before drawing can use the
    correct colour even if the first instruction is not a background rect.
    """

    width: int
    height: int
    instructions: list[PaintInstruction]
    background: str = "#ffffff"
    metadata: PaintMetadata = field(default_factory=dict)


# ============================================================================
# Builder helpers
# ============================================================================


def paint_rect(
    x: int,
    y: int,
    width: int,
    height: int,
    fill: str = "#000000",
    metadata: PaintMetadata | None = None,
) -> PaintRectInstruction:
    """Build a ``PaintRectInstruction``.

    This thin wrapper makes call sites more readable than the raw dataclass
    constructor and provides a ``None``-safe ``metadata`` default.

    >>> r = paint_rect(0, 0, 10, 10, fill="#ff0000")
    >>> r.kind
    'rect'
    """
    return PaintRectInstruction(x, y, width, height, fill, metadata or {})


def paint_path(
    commands: tuple[PathCommand, ...] | list[PathCommand],
    fill: str = "#000000",
    metadata: PaintMetadata | None = None,
) -> PaintPathInstruction:
    """Build a ``PaintPathInstruction``.

    Accepts either a ``tuple`` or a ``list`` of ``PathCommand`` objects for
    convenience; internally the commands are stored as a ``tuple`` to keep
    the dataclass frozen (lists are not hashable).

    >>> from paint_instructions import PathCommand
    >>> cmd = PathCommand("move_to", 0.0, 0.0)
    >>> p = paint_path([cmd, PathCommand("close")])
    >>> p.kind
    'path'
    """
    return PaintPathInstruction(
        commands=tuple(commands),
        fill=fill,
        metadata=metadata or {},
    )


def paint_scene(
    width: int,
    height: int,
    instructions: list[PaintInstruction],
    background: str = "#ffffff",
    metadata: PaintMetadata | None = None,
) -> PaintScene:
    """Build a ``PaintScene``.

    >>> s = paint_scene(100, 100, [])
    >>> s.background
    '#ffffff'
    """
    return PaintScene(width, height, instructions, background, metadata or {})


def create_scene(
    width: int,
    height: int,
    instructions: list[PaintInstruction],
    background: str = "#ffffff",
    metadata: PaintMetadata | None = None,
) -> PaintScene:
    """Alias for ``paint_scene``.  Kept for backwards compatibility."""
    return paint_scene(width, height, instructions, background, metadata)
