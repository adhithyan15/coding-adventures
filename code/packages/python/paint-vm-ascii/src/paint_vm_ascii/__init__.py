"""Terminal backend for backend-neutral paint scenes."""

from __future__ import annotations

import math
from dataclasses import dataclass

from paint_instructions import PaintRectInstruction, PaintScene

__version__ = "0.1.0"


@dataclass(frozen=True)
class AsciiOptions:
    """How scene coordinates map to character cells."""

    scale_x: int = 8
    scale_y: int = 16


class _CharBuffer:
    def __init__(self, rows: int, cols: int) -> None:
        self.rows = rows
        self.cols = cols
        self.chars: list[list[str]] = [[" "] * cols for _ in range(rows)]

    def write_char(self, row: int, col: int, ch: str) -> None:
        if row < 0 or row >= self.rows or col < 0 or col >= self.cols:
            return
        self.chars[row][col] = ch

    def to_string(self) -> str:
        return "\n".join("".join(row).rstrip() for row in self.chars).rstrip()


def _to_col(x: int, scale_x: int) -> int:
    return round(x / scale_x)


def _to_row(y: int, scale_y: int) -> int:
    return round(y / scale_y)


def _render_rect(
    inst: PaintRectInstruction,
    buf: _CharBuffer,
    *,
    scale_x: int,
    scale_y: int,
) -> None:
    if inst.fill in {"", "transparent", "none"}:
        return

    c1 = _to_col(inst.x, scale_x)
    r1 = _to_row(inst.y, scale_y)
    c2 = _to_col(inst.x + inst.width, scale_x)
    r2 = _to_row(inst.y + inst.height, scale_y)

    for row in range(r1, r2 + 1):
        for col in range(c1, c2 + 1):
            buf.write_char(row, col, "\u2588")


def render(scene: PaintScene, options: AsciiOptions | None = None) -> str:
    """Render a paint scene as a plain terminal string."""

    opts = options or AsciiOptions()
    cols = math.ceil(scene.width / opts.scale_x)
    rows = math.ceil(scene.height / opts.scale_y)
    buf = _CharBuffer(rows, cols)

    for inst in scene.instructions:
        if isinstance(inst, PaintRectInstruction):
            _render_rect(inst, buf, scale_x=opts.scale_x, scale_y=opts.scale_y)
            continue
        raise ValueError(f"paint-vm-ascii: unsupported paint instruction kind: {inst.kind}")

    return buf.to_string()
