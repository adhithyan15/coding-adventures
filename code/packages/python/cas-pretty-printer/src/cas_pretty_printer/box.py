"""2D box-model layout engine for CAS pretty printing.

A ``Box`` is a rectangular region of text with a designated *baseline* row.
The baseline is the row that represents the mathematical "ground level" —
where additions, multiplications, etc. align their operands horizontally.

Mental model
------------
Think of how a typesetter would handle ``x + a/b``. The ``x`` sits on the
baseline. The fraction ``a/b`` has its bar on the baseline, ``a`` above it,
and ``b`` below it. When composing them with ``+``, we align their baselines
so the plus sign is vertically centred between them:

    ┌─────────────────────┐
    │      a              │  ← row 0  (numerator — above baseline)
    │ x + ───             │  ← row 1  (baseline — fraction bar, x, plus)
    │      b              │  ← row 2  (denominator — below baseline)
    └─────────────────────┘

Box composition rules
---------------------
``hbox(boxes, sep)``
    Align baselines horizontally. Each box is padded vertically so that its
    baseline sits on the common baseline. Then all boxes are concatenated
    column-by-column.

``vbox(boxes)``
    Stack boxes vertically, centred horizontally (pad to the widest box).
    No baseline alignment — used for internal stacking within e.g. a matrix.

Entry point
-----------
``pretty_2d(node, dialect) → str``
    Returns a multi-line string for the given IR node. Each CAS head has a
    layout rule. Unknown heads fall back to the linear function-call form.

Box dimensions
--------------
- ``width``    = max line length across all rows (some rows may be padded).
- ``height``   = number of rows.
- ``baseline`` = 0-based row index of the mathematical ground level.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

if TYPE_CHECKING:
    from cas_pretty_printer.dialect import Dialect


# ---------------------------------------------------------------------------
# Box dataclass
# ---------------------------------------------------------------------------


@dataclass
class Box:
    """A rectangular region of text with a baseline row.

    The ``lines`` list holds each row as a string. All rows are padded (on
    construction or by layout functions) to the same ``width``. The
    ``baseline`` attribute is the 0-based index of the row that represents
    the mathematical ground level for horizontal alignment.

    Attributes
    ----------
    lines:
        List of text rows. Length equals ``height``.
    baseline:
        Row index of the mathematical baseline. 0 means the top row is the
        baseline (typical for atoms, which are only one row tall).

    Derived attributes (computed on construction)
    ---------------------------------------------
    width:
        Maximum character width across all rows.
    height:
        Number of rows (= ``len(lines)``).
    """

    lines: list[str]
    baseline: int
    # Width and height are computed, not stored — keep them as properties
    # so callers cannot accidentally out-of-sync them.

    @property
    def width(self) -> int:
        """Width of the widest row, or 0 for an empty box."""
        return max((len(line) for line in self.lines), default=0)

    @property
    def height(self) -> int:
        """Number of rows."""
        return len(self.lines)

    def render(self) -> str:
        """Join rows into a single multi-line string."""
        return "\n".join(self.lines)

    def pad_width(self, target: int, align: str = "center") -> Box:
        """Return a copy with all rows padded to ``target`` width.

        Parameters
        ----------
        target:
            Desired width. If already ≥ target, the box is returned
            unchanged (no truncation).
        align:
            ``"center"`` (default), ``"left"``, or ``"right"``.
        """
        if target <= self.width:
            return self
        new_lines: list[str] = []
        for line in self.lines:
            pad = target - len(line)
            if align == "center":
                left_pad = pad // 2
                right_pad = pad - left_pad
                new_lines.append(" " * left_pad + line + " " * right_pad)
            elif align == "left":
                new_lines.append(line + " " * pad)
            else:  # "right"
                new_lines.append(" " * pad + line)
        return Box(new_lines, self.baseline)


# ---------------------------------------------------------------------------
# Primitive constructors
# ---------------------------------------------------------------------------


def atom_box(text: str) -> Box:
    """Create a single-line box at baseline 0.

    This is the starting point for every leaf — integers, floats, symbols,
    strings. The baseline is row 0 because the entire box *is* the baseline.

    Example::

        atom_box("x")  →  Box(lines=["x"], baseline=0)
        atom_box("42") →  Box(lines=["42"], baseline=0)
    """
    return Box([text], baseline=0)


# ---------------------------------------------------------------------------
# Horizontal composition — hbox
# ---------------------------------------------------------------------------


def hbox(boxes: list[Box], sep: str = "") -> Box:
    """Align boxes on their baselines and concatenate horizontally.

    Algorithm
    ---------
    1. Find the common baseline: the maximum of all boxes' ``baseline``
       values. Boxes with a smaller baseline need empty rows prepended above
       them; boxes with a larger baseline need empty rows appended below.

    2. Find the common height: ``above + max(baseline) + max(height - baseline - 1)``
       where ``above`` is always 0 after step 1 (we aligned to the max
       baseline). More precisely:

       ``height = max(baseline) + max(height - baseline - 1) + 1``

    3. For each box, pad it vertically so it is ``total_height`` rows tall:
       - prepend ``(common_baseline - b.baseline)`` empty rows above.
       - append enough empty rows below to reach ``total_height``.

    4. Concatenate each row across all boxes (with ``sep`` between them).

    The separator is applied *between* adjacent boxes, not after the last one.

    Example::

        hbox([atom_box("a"), atom_box("b")], sep=" + ")
        → Box(lines=["a + b"], baseline=0)
    """
    if not boxes:
        return atom_box("")

    # Step 1: compute the common (maximum) baseline.
    common_baseline = max(b.baseline for b in boxes)

    # Step 2: compute total height.
    # Below-baseline space of each box: (b.height - b.baseline - 1)
    max_below = max(b.height - b.baseline - 1 for b in boxes)
    total_height = common_baseline + 1 + max_below

    # Step 3: pad each box vertically to total_height.
    padded: list[list[str]] = []
    for b in boxes:
        above_rows = common_baseline - b.baseline  # rows to prepend above
        below_rows = total_height - b.height - above_rows  # rows to append below
        width = b.width  # use the box's own width for padding
        empty = " " * width
        rows = (
            [empty] * above_rows
            + b.lines
            + [empty] * below_rows
        )
        padded.append(rows)

    # Step 4: merge rows column-by-column with sep between boxes.
    result_lines: list[str] = []
    for row_idx in range(total_height):
        parts = [padded[i][row_idx] for i in range(len(boxes))]
        result_lines.append(sep.join(parts))

    return Box(result_lines, baseline=common_baseline)


# ---------------------------------------------------------------------------
# Vertical composition — vbox
# ---------------------------------------------------------------------------


def vbox(boxes: list[Box]) -> Box:
    """Stack boxes vertically, centred horizontally.

    The result has no meaningful baseline (it is set to the middle row).
    This function is used internally for e.g. matrix rows, not for
    aligning addends.

    All boxes are padded to the widest box's width before stacking.
    """
    if not boxes:
        return atom_box("")

    target_width = max(b.width for b in boxes)
    result_lines: list[str] = []
    for b in boxes:
        pb = b.pad_width(target_width, align="center")
        result_lines.extend(pb.lines)

    mid = len(result_lines) // 2
    return Box(result_lines, baseline=mid)


# ---------------------------------------------------------------------------
# Box builders for specific IR heads
# ---------------------------------------------------------------------------


def _div_box(num_box: Box, den_box: Box) -> Box:
    """Build a fraction box: numerator over bar over denominator.

    The fraction bar occupies the baseline row. Numerator is centred above
    it, denominator is centred below it.

    Visual (``a / b``)::

        row 0:  "  a  "
        row 1:  "─────"    ← baseline
        row 2:  "  b  "

    The bar width is ``max(num_width, den_width) + 2`` (one space padding
    on each side for readability).
    """
    bar_width = max(num_box.width, den_box.width) + 2
    bar_line = "─" * bar_width

    # Centre numerator and denominator to the bar width.
    num_padded = num_box.pad_width(bar_width, align="center")
    den_padded = den_box.pad_width(bar_width, align="center")

    lines = num_padded.lines + [bar_line] + den_padded.lines
    # The fraction bar is the baseline — it is at index len(num_lines).
    baseline = num_padded.height  # row index of the bar
    return Box(lines, baseline=baseline)


def _pow_box(base_box: Box, exp_box: Box) -> Box:
    """Build a power box: exponent superscripted above and to the right.

    The base occupies the bottom rows; the exponent is placed at the top
    right. The base's baseline becomes the combined box's baseline.

    Visual (``x^2``)::

        row 0:  "  2"   ← exponent
        row 1:  "x  "   ← base (baseline row of combined box)

    The baseline is aligned to the base's bottom row (the base's own
    baseline, raised to account for the exponent rows above).
    """
    exp_height = exp_box.height
    base_height = base_box.height

    # The combined box has exp_height + base_height rows.
    # Top exp_height rows: base part is blank, exp part is filled.
    # Bottom base_height rows: base part is filled, exp part is blank.
    base_blank = " " * base_box.width
    exp_blank = " " * exp_box.width

    result_lines: list[str] = []
    for i in range(exp_height):
        # Pad exp_box rows to exp_box.width to avoid ragged right.
        exp_row = exp_box.lines[i].ljust(exp_box.width)
        result_lines.append(base_blank + exp_row)

    for i in range(base_height):
        base_row = base_box.lines[i].ljust(base_box.width)
        result_lines.append(base_row + exp_blank)

    # The baseline of the combined box is the base's baseline,
    # offset down by the exponent's height (the rows we prepended above).
    combined_baseline = exp_height + base_box.baseline
    return Box(result_lines, baseline=combined_baseline)


def _sqrt_box(arg_box: Box) -> Box:
    """Build a square-root box with √ symbol and overline.

    Layout::

        row 0:  "  ┌───────┐"    ← overline (top corner)
        row 1:  "√ │ arg   │"    ← radical row (baseline if arg is single row)
        ...

    If ``arg`` is multi-line, the √ glyph aligns with the *baseline* row
    of the argument. The overline spans the full argument width + 2.
    """
    arg_width = arg_box.width
    inner_width = arg_width + 2  # one space padding on each side

    # Top border: "  ┌" + "─" * inner_width + "┐"
    # The "  " at the left aligns with the "√ " prefix used on the middle rows.
    top_line = "  ┌" + "─" * inner_width + "┐"

    # The √ sign aligns with the baseline row of the argument.
    # Rows above baseline: "  │ " + content + " │"
    # Baseline row:        "√ │ " + content + " │"
    # Rows below baseline: "  │ " + content + " │"

    result_lines: list[str] = [top_line]
    for i, line in enumerate(arg_box.lines):
        padded_line = line.ljust(arg_width)  # ensure uniform width
        content = " " + padded_line + " "
        if i == arg_box.baseline:
            result_lines.append("√ │" + content + "│")
        else:
            result_lines.append("  │" + content + "│")

    # The overall baseline: 1 (the top border) + arg_box.baseline.
    # +1 because we inserted the top border at row 0.
    combined_baseline = 1 + arg_box.baseline
    return Box(result_lines, baseline=combined_baseline)


# ---------------------------------------------------------------------------
# Main dispatch — pretty_2d
# ---------------------------------------------------------------------------


def pretty_2d(node: IRNode, dialect: Dialect) -> str:
    """Format ``node`` as a multi-line 2D string using the box engine.

    Dispatches based on node type and (for ``IRApply``) head name. Unknown
    heads fall back to the linear function-call format from the walker.

    Returns a multi-line string (rows joined by ``\\n``).
    """
    return _box(node, dialect).render()


def _box(node: IRNode, dialect: Dialect) -> Box:
    """Recursively build a ``Box`` for any IR node."""
    # ---- Leaves -----------------------------------------------------------
    if isinstance(node, IRInteger):
        return atom_box(dialect.format_integer(node.value))
    if isinstance(node, IRFloat):
        return atom_box(dialect.format_float(node.value))
    if isinstance(node, IRRational):
        return atom_box(dialect.format_rational(node.numer, node.denom))
    if isinstance(node, IRSymbol):
        return atom_box(dialect.format_symbol(node.name))
    if isinstance(node, IRString):
        return atom_box(dialect.format_string(node.value))

    # ---- Compound nodes ---------------------------------------------------
    if not isinstance(node, IRApply):
        # Fallback for unknown node types.
        return atom_box(repr(node))

    head = node.head
    head_name = head.name if isinstance(head, IRSymbol) else None

    # -- Negation: prefix "-" at baseline ------------------------------------
    if head_name == "Neg" and len(node.args) == 1:
        inner = _box(node.args[0], dialect)
        # Prepend "-" on the baseline row, blank on other rows.
        new_lines = []
        for i, line in enumerate(inner.lines):
            if i == inner.baseline:
                new_lines.append("-" + line)
            else:
                new_lines.append(" " + line)
        return Box(new_lines, baseline=inner.baseline)

    # -- Division: fraction layout -------------------------------------------
    if head_name == "Div" and len(node.args) == 2:
        num_box = _box(node.args[0], dialect)
        den_box = _box(node.args[1], dialect)
        return _div_box(num_box, den_box)

    # -- Power: superscript --------------------------------------------------
    if head_name == "Pow" and len(node.args) == 2:
        base_b = _box(node.args[0], dialect)
        exp_b = _box(node.args[1], dialect)
        return _pow_box(base_b, exp_b)

    # -- Square root: overline layout ----------------------------------------
    if head_name == "Sqrt" and len(node.args) == 1:
        arg_b = _box(node.args[0], dialect)
        return _sqrt_box(arg_b)

    # -- Addition: join with " + " at baseline --------------------------------
    if head_name == "Add" and len(node.args) >= 2:
        sep = atom_box(" + ")
        parts: list[Box] = []
        for i, arg in enumerate(node.args):
            if i > 0:
                parts.append(sep)
            parts.append(_box(arg, dialect))
        return hbox(parts)

    # -- Subtraction: join with " - " at baseline ----------------------------
    if head_name == "Sub" and len(node.args) == 2:
        left_b = _box(node.args[0], dialect)
        op_b = atom_box(" - ")
        right_b = _box(node.args[1], dialect)
        return hbox([left_b, op_b, right_b])

    # -- Multiplication: join with "*" at baseline ---------------------------
    if head_name == "Mul" and len(node.args) >= 2:
        parts = []
        for i, arg in enumerate(node.args):
            if i > 0:
                parts.append(atom_box("*"))
            parts.append(_box(arg, dialect))
        return hbox(parts)

    # -- List: "[item, item, ...]" linearly ----------------------------------
    if head_name == "List":
        open_b, close_b = dialect.list_brackets()
        if not node.args:
            return atom_box(f"{open_b}{close_b}")
        parts = []
        for i, arg in enumerate(node.args):
            if i > 0:
                parts.append(atom_box(", "))
            parts.append(_box(arg, dialect))
        inner_box = hbox(parts)
        return hbox([atom_box(open_b), inner_box, atom_box(close_b)])

    # -- Fallback: linear function-call format -------------------------------
    # For any head not specially handled above (e.g. Sin, Cos, Limit, …),
    # render as "name(arg1, arg2, …)" on a single line using the walker's
    # linear formatter. This ensures 2D mode degrades gracefully for
    # unsupported heads rather than crashing or returning garbage.
    from cas_pretty_printer.walker import _format  # local import avoids cycle

    linear_text = _format(node, dialect, min_prec=0)
    return atom_box(linear_text)
