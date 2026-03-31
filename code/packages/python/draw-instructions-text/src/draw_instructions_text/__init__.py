"""ASCII/Unicode text renderer for backend-neutral draw instructions.

This renderer proves the draw-instructions abstraction is truly backend-
neutral: the same DrawScene that produces SVG or paints a Canvas can also
render as box-drawing characters in a terminal.

=== How It Works ===

The renderer maps pixel-coordinate scenes to a fixed-width character grid.
Each cell in the grid is one character.  The mapping uses a configurable
scale factor (default: 8 px per char width, 16 px per char height).

::

    Scene coordinates (pixels)     Character grid
    +---------------------+        +----------+
    | rect at (0,0,200,32)|   ->   |++++++++++|
    |                     |        |++++++++++|
    +---------------------+        +----------+

=== Character Palette ===

Box-drawing characters create clean table grids::

    +------+-----+     Corners: top-left  top-right  bottom-left  bottom-right
    | Name | Age |     Edges:   horizontal  vertical
    +------+-----+     Tees:    top  bottom  left  right
    | Alice|  30 |     Cross:   cross
    +------+-----+     Fill:    block

=== Intersection Logic ===

When two drawing operations overlap at the same cell, the renderer merges
them into the correct junction character.  A horizontal line crossing a
vertical line becomes a cross.  A line meeting a box corner becomes the
appropriate tee.

This is tracked via a "tag" buffer parallel to the character buffer.
Each cell records which directions have lines passing through it
(up, down, left, right), and the tag is resolved to the correct
box-drawing character on each write.

=== Usage ===

::

    from draw_instructions import create_scene, draw_rect, draw_line, draw_text
    from draw_instructions_text import render_text

    scene = create_scene(160, 48, [
        draw_rect(0, 0, 160, 48, "transparent", stroke="#000", stroke_width=1),
        draw_line(0, 16, 160, 16, "#000", 1),
        draw_text(8, 12, "Hello", align="start"),
    ])

    print(render_text(scene))

"""

from __future__ import annotations

import math
from dataclasses import dataclass

from draw_instructions import (
    DrawClipInstruction,
    DrawGroupInstruction,
    DrawInstruction,
    DrawLineInstruction,
    DrawRectInstruction,
    DrawRenderer,
    DrawScene,
    DrawTextInstruction,
)

__version__ = "0.1.0"


# ---------------------------------------------------------------------------
# Direction flags
#
# Each cell in the tag buffer stores a bitmask of directions.  When
# multiple drawing operations overlap, we OR the flags together and
# resolve the combined tag to the correct box-drawing character.
#
#        UP (1)
#         |
# LEFT(8)-+-RIGHT(2)
#         |
#       DOWN(4)
# ---------------------------------------------------------------------------

UP = 1
RIGHT = 2
DOWN = 4
LEFT = 8
FILL = 16
TEXT = 32


# ---------------------------------------------------------------------------
# Box-drawing character resolution
#
# Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), return the
# correct Unicode box-drawing character.  This table covers all 16
# combinations of the 4 direction bits.
# ---------------------------------------------------------------------------

BOX_CHARS: dict[int, str] = {
    LEFT | RIGHT: "\u2500",  # horizontal  ---
    UP | DOWN: "\u2502",  # vertical    |
    DOWN | RIGHT: "\u250c",  # top-left corner
    DOWN | LEFT: "\u2510",  # top-right corner
    UP | RIGHT: "\u2514",  # bottom-left corner
    UP | LEFT: "\u2518",  # bottom-right corner
    LEFT | RIGHT | DOWN: "\u252c",  # top tee
    LEFT | RIGHT | UP: "\u2534",  # bottom tee
    UP | DOWN | RIGHT: "\u251c",  # left tee
    UP | DOWN | LEFT: "\u2524",  # right tee
    UP | DOWN | LEFT | RIGHT: "\u253c",  # cross
    RIGHT: "\u2500",  # half-lines default to full
    LEFT: "\u2500",
    UP: "\u2502",
    DOWN: "\u2502",
}


def _resolve_box_char(tag: int) -> str:
    """Resolve a direction bitmask to a box-drawing character.

    Falls back to ``"+"`` if the combination is not in our table (should
    not happen in practice).

    The resolution order is:
    1. FILL flag -> block character
    2. TEXT flag -> empty string (text chars stored directly, not via tags)
    3. Direction flags -> look up in BOX_CHARS table
    """
    if tag & FILL:
        return "\u2588"
    if tag & TEXT:
        return ""
    return BOX_CHARS.get(tag & (UP | DOWN | LEFT | RIGHT), "+")


# ---------------------------------------------------------------------------
# Clip bounds
# ---------------------------------------------------------------------------


@dataclass
class ClipBounds:
    """A rectangular clipping region in character-grid coordinates.

    Any write operation checks against these bounds before modifying the
    buffer.  Clips can be nested -- when processing a ``DrawClipInstruction``
    we intersect the parent clip with the new clip to produce a tighter
    region.
    """

    min_col: int
    min_row: int
    max_col: int
    max_row: int


# ---------------------------------------------------------------------------
# Character buffer
# ---------------------------------------------------------------------------


class CharBuffer:
    """A 2-D character buffer with a parallel tag buffer for intersections.

    The ``chars`` grid stores the actual character at each cell.  The
    ``tags`` grid stores a bitmask of directions passing through that
    cell.  When writing a box-drawing character we update the tag buffer
    and resolve the correct character from the combined tag.

    Example -- two perpendicular lines meeting at (1, 2)::

        buf.write_tag(1, 2, LEFT | RIGHT, clip)   # horizontal passes through
        buf.write_tag(1, 2, UP | DOWN, clip)       # vertical passes through
        # tags[1][2] is now UP | DOWN | LEFT | RIGHT -> cross character
    """

    def __init__(self, rows: int, cols: int) -> None:
        self.rows = rows
        self.cols = cols
        self.chars: list[list[str]] = [[" "] * cols for _ in range(rows)]
        self.tags: list[list[int]] = [[0] * cols for _ in range(rows)]

    def write_tag(
        self, row: int, col: int, dir_flags: int, clip: ClipBounds
    ) -> None:
        """Write a box-drawing element at *(row, col)* by adding direction flags.

        The actual character stored is resolved from the combined tag.
        Writes outside the clip bounds or buffer bounds are silently ignored.
        Text cells are never overwritten by box-drawing characters.
        """
        if row < clip.min_row or row >= clip.max_row:
            return
        if col < clip.min_col or col >= clip.max_col:
            return
        if row < 0 or row >= self.rows or col < 0 or col >= self.cols:
            return

        existing = self.tags[row][col]

        # Don't overwrite text with box-drawing
        if existing & TEXT:
            return

        merged = existing | dir_flags
        self.tags[row][col] = merged
        self.chars[row][col] = (
            "\u2588" if (dir_flags & FILL) else _resolve_box_char(merged)
        )

    def write_char(
        self, row: int, col: int, ch: str, clip: ClipBounds
    ) -> None:
        """Write a text character directly at *(row, col)*.

        Text overwrites any existing content -- it has the highest
        visual priority.
        """
        if row < clip.min_row or row >= clip.max_row:
            return
        if col < clip.min_col or col >= clip.max_col:
            return
        if row < 0 or row >= self.rows or col < 0 or col >= self.cols:
            return

        self.chars[row][col] = ch
        self.tags[row][col] = TEXT

    def to_string(self) -> str:
        """Join all rows, trim trailing whitespace, and return the result."""
        return "\n".join(
            "".join(row).rstrip() for row in self.chars
        ).rstrip()


# ---------------------------------------------------------------------------
# Coordinate mapping
#
# Scene coordinates are in pixels.  We divide by the scale factor and
# round to the nearest integer to get character-grid coordinates.
# ---------------------------------------------------------------------------


def _to_col(x: float, scale_x: float) -> int:
    """Map a pixel x-coordinate to a character column."""
    return round(x / scale_x)


def _to_row(y: float, scale_y: float) -> int:
    """Map a pixel y-coordinate to a character row."""
    return round(y / scale_y)


# ---------------------------------------------------------------------------
# Instruction renderers
# ---------------------------------------------------------------------------


def _render_rect(
    inst: DrawRectInstruction,
    buf: CharBuffer,
    sx: float,
    sy: float,
    clip: ClipBounds,
) -> None:
    """Render a rectangle instruction into the character buffer.

    Stroked rectangles produce box-drawing outlines (corners + edges).
    Filled rectangles produce solid block characters.  A transparent
    or empty fill with no stroke produces nothing.
    """
    c1 = _to_col(inst.x, sx)
    r1 = _to_row(inst.y, sy)
    c2 = _to_col(inst.x + inst.width, sx)
    r2 = _to_row(inst.y + inst.height, sy)

    has_stroke = inst.stroke is not None and inst.stroke != ""
    has_fill = inst.fill not in ("", "transparent", "none")

    if has_stroke:
        # Corners
        buf.write_tag(r1, c1, DOWN | RIGHT, clip)
        buf.write_tag(r1, c2, DOWN | LEFT, clip)
        buf.write_tag(r2, c1, UP | RIGHT, clip)
        buf.write_tag(r2, c2, UP | LEFT, clip)

        # Top and bottom edges
        for c in range(c1 + 1, c2):
            buf.write_tag(r1, c, LEFT | RIGHT, clip)
            buf.write_tag(r2, c, LEFT | RIGHT, clip)

        # Left and right edges
        for r in range(r1 + 1, r2):
            buf.write_tag(r, c1, UP | DOWN, clip)
            buf.write_tag(r, c2, UP | DOWN, clip)

    elif has_fill:
        # Fill the interior with block characters
        for r in range(r1, r2 + 1):
            for c in range(c1, c2 + 1):
                buf.write_tag(r, c, FILL, clip)


def _render_line(
    inst: DrawLineInstruction,
    buf: CharBuffer,
    sx: float,
    sy: float,
    clip: ClipBounds,
) -> None:
    """Render a line instruction into the character buffer.

    Horizontal and vertical lines use direction-aware endpoint flags:
    endpoints only point inward so that junctions with perpendicular
    elements resolve correctly.  For example, a left endpoint gets the
    RIGHT flag (pointing inward), not LEFT|RIGHT, so it merges with a
    vertical edge as a left-tee rather than a cross.

    Diagonal lines are approximated using Bresenham's algorithm.
    """
    c1 = _to_col(inst.x1, sx)
    r1 = _to_row(inst.y1, sy)
    c2 = _to_col(inst.x2, sx)
    r2 = _to_row(inst.y2, sy)

    if r1 == r2:
        # --- Horizontal line ---
        min_c = min(c1, c2)
        max_c = max(c1, c2)
        for c in range(min_c, max_c + 1):
            flags = 0
            if c > min_c:
                flags |= LEFT
            if c < max_c:
                flags |= RIGHT
            if c == min_c and c == max_c:
                flags = LEFT | RIGHT  # single-cell line
            buf.write_tag(r1, c, flags, clip)

    elif c1 == c2:
        # --- Vertical line ---
        min_r = min(r1, r2)
        max_r = max(r1, r2)
        for r in range(min_r, max_r + 1):
            flags = 0
            if r > min_r:
                flags |= UP
            if r < max_r:
                flags |= DOWN
            if r == min_r and r == max_r:
                flags = UP | DOWN  # single-cell line
            buf.write_tag(r, c1, flags, clip)

    else:
        # --- Diagonal line (Bresenham approximation) ---
        dr = abs(r2 - r1)
        dc = abs(c2 - c1)
        sr = 1 if r1 < r2 else -1
        sc = 1 if c1 < c2 else -1
        err = dc - dr
        r, c = r1, c1

        while True:
            buf.write_tag(
                r, c, (LEFT | RIGHT) if dc > dr else (UP | DOWN), clip
            )
            if r == r2 and c == c2:
                break
            e2 = 2 * err
            if e2 > -dr:
                err -= dr
                c += sc
            if e2 < dc:
                err += dc
                r += sr


def _render_text_inst(
    inst: DrawTextInstruction,
    buf: CharBuffer,
    sx: float,
    sy: float,
    clip: ClipBounds,
) -> None:
    """Render a text instruction into the character buffer.

    Alignment determines the anchor point:
    - ``"start"``: text begins at the x coordinate
    - ``"middle"``: text is centered on the x coordinate
    - ``"end"``: text ends at the x coordinate
    """
    row = _to_row(inst.y, sy)
    text = inst.value

    if inst.align == "middle":
        start_col = _to_col(inst.x, sx) - len(text) // 2
    elif inst.align == "end":
        start_col = _to_col(inst.x, sx) - len(text)
    else:  # "start"
        start_col = _to_col(inst.x, sx)

    for i, ch in enumerate(text):
        buf.write_char(row, start_col + i, ch, clip)


def _render_group(
    inst: DrawGroupInstruction,
    buf: CharBuffer,
    sx: float,
    sy: float,
    clip: ClipBounds,
) -> None:
    """Render a group by recursing into each child instruction."""
    for child in inst.children:
        _render_instruction(child, buf, sx, sy, clip)


def _render_clip(
    inst: DrawClipInstruction,
    buf: CharBuffer,
    sx: float,
    sy: float,
    parent_clip: ClipBounds,
) -> None:
    """Render a clip region by intersecting with the parent clip.

    The new clip bounds are the intersection of the parent clip and the
    clip instruction's rectangle.  Children are rendered with the
    tighter clip.
    """
    new_clip = ClipBounds(
        min_col=max(parent_clip.min_col, _to_col(inst.x, sx)),
        min_row=max(parent_clip.min_row, _to_row(inst.y, sy)),
        max_col=min(parent_clip.max_col, _to_col(inst.x + inst.width, sx)),
        max_row=min(parent_clip.max_row, _to_row(inst.y + inst.height, sy)),
    )
    for child in inst.children:
        _render_instruction(child, buf, sx, sy, new_clip)


def _render_instruction(
    inst: DrawInstruction,
    buf: CharBuffer,
    sx: float,
    sy: float,
    clip: ClipBounds,
) -> None:
    """Dispatch a single draw instruction to the appropriate renderer."""
    if inst.kind == "rect":
        _render_rect(inst, buf, sx, sy, clip)  # type: ignore[arg-type]
    elif inst.kind == "line":
        _render_line(inst, buf, sx, sy, clip)  # type: ignore[arg-type]
    elif inst.kind == "text":
        _render_text_inst(inst, buf, sx, sy, clip)  # type: ignore[arg-type]
    elif inst.kind == "group":
        _render_group(inst, buf, sx, sy, clip)  # type: ignore[arg-type]
    elif inst.kind == "clip":
        _render_clip(inst, buf, sx, sy, clip)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


class TextRenderer:
    """Text renderer that converts DrawScene objects into box-drawing strings.

    Implements the ``DrawRenderer[str]`` protocol from draw-instructions.

    Parameters
    ----------
    scale_x:
        Pixels per character column.  Default: 8.
    scale_y:
        Pixels per character row.  Default: 16.
    """

    def __init__(
        self, *, scale_x: float = 8, scale_y: float = 16
    ) -> None:
        self.scale_x = scale_x
        self.scale_y = scale_y

    def render(self, scene: DrawScene) -> str:
        """Render the scene as a Unicode box-drawing string."""
        cols = math.ceil(scene.width / self.scale_x)
        rows = math.ceil(scene.height / self.scale_y)
        buf = CharBuffer(rows, cols)

        full_clip = ClipBounds(
            min_col=0, min_row=0, max_col=cols, max_row=rows
        )

        for inst in scene.instructions:
            _render_instruction(inst, buf, self.scale_x, self.scale_y, full_clip)

        return buf.to_string()


#: Default text renderer with standard scale (8 px/char, 16 px/row).
TEXT_RENDERER = TextRenderer()


def render_text(
    scene: DrawScene,
    *,
    scale_x: float = 8,
    scale_y: float = 16,
) -> str:
    """Convenience wrapper: scene in, text string out.

    Parameters
    ----------
    scene:
        The draw-instructions scene to render.
    scale_x:
        Pixels per character column.  Default: 8.
    scale_y:
        Pixels per character row.  Default: 16.
    """
    return TextRenderer(scale_x=scale_x, scale_y=scale_y).render(scene)
