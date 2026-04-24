"""Shared 2D barcode abstraction layer.

This package provides the two building blocks every 2D barcode format needs:

1. ``ModuleGrid`` — the universal intermediate representation produced by
   every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
   It is just a 2D boolean grid: ``True`` = dark module, ``False`` = light.

2. ``layout()`` — the single function that converts abstract module
   coordinates into pixel-level ``PaintScene`` instructions ready for the
   PaintVM (P2D01) to render.

## Where this fits in the pipeline

.. code-block::

    Input data
      → format encoder (qr-code, data-matrix, aztec…)
      → ModuleGrid          ← produced by the encoder
      → layout()            ← THIS PACKAGE converts to pixels
      → PaintScene          ← consumed by paint-vm (P2D01)
      → backend (SVG, Metal, Canvas, terminal…)

All coordinates *before* ``layout()`` are measured in "module units" —
abstract grid steps.  Only ``layout()`` multiplies by ``module_size_px``
to produce real pixel coordinates.  This means encoders never need to know
anything about screen resolution or output format.

## Supported module shapes

- **Square** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
  Each module becomes a ``PaintRectInstruction``.

- **Hex** (flat-top hexagons): used by MaxiCode (ISO/IEC 16023).  Each
  module becomes a ``PaintPathInstruction`` tracing six vertices.

## Annotations

The optional ``AnnotatedModuleGrid`` adds per-module role information useful
for visualizers (highlighting finder patterns, data codewords, etc.).
Annotations are never required for rendering — the renderer only looks at
the ``modules`` boolean grid.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from typing import Final, Literal

from paint_instructions import (
    PaintInstruction,
    PaintScene,
    PathCommand,
    paint_path,
    paint_rect,
    paint_scene,
)

__version__ = "0.1.0"

# ============================================================================
# ModuleShape — square vs. hex
# ============================================================================

ModuleShape = Literal["square", "hex"]
"""The shape of each module in the grid.

- ``"square"`` — used by QR Code, Data Matrix, Aztec Code, PDF417.  The
  overwhelmingly common shape.  Each module renders as a filled square.

- ``"hex"`` — used by MaxiCode (ISO/IEC 16023).  MaxiCode uses flat-top
  hexagons arranged in an offset-row grid.  Each module renders as a
  filled hexagon drawn with a ``PaintPathInstruction``.

The shape is stored on ``ModuleGrid`` so that ``layout()`` can pick the
right rendering path without the caller having to specify it again.
"""

# ============================================================================
# ModuleGrid — the universal output of every 2D barcode encoder
# ============================================================================


@dataclass(frozen=True)
class ModuleGrid:
    """Universal intermediate representation for 2D barcode encoders.

    A ``ModuleGrid`` is a 2D boolean grid:

    .. code-block::

        modules[row][col] is True   →  dark module (ink / filled)
        modules[row][col] is False  →  light module (background / empty)

    Row 0 is the top row.  Column 0 is the leftmost column.  This matches
    the natural reading order used in every 2D barcode standard.

    ### MaxiCode fixed size

    MaxiCode grids are always 33 rows × 30 columns with
    ``module_shape="hex"``.  The outer bullseye rings are placed at the
    geometric centre.  Physical MaxiCode symbols are always approximately
    1 inch × 1 inch.

    ### Immutability

    ``ModuleGrid`` is intentionally frozen.  Use ``set_module()`` to
    produce a new grid with one module changed, rather than mutating in
    place.  This makes encoders easy to test and compose.

    Example — create a 21×21 QR Code v1 grid and set a dark module::

        grid = make_module_grid(21, 21)
        grid2 = set_module(grid, 10, 10, True)
        assert grid.modules[10][10] is False   # original unchanged
        assert grid2.modules[10][10] is True   # new grid updated
    """

    cols: int
    rows: int
    # Two-dimensional boolean grid stored as a tuple of tuples.
    # Access with ``modules[row][col]``.
    # True = dark module, False = light module.
    modules: tuple[tuple[bool, ...], ...]
    module_shape: ModuleShape = "square"


# ============================================================================
# ModuleRole — what a module structurally represents
# ============================================================================

ModuleRole = Literal[
    "finder",
    "separator",
    "timing",
    "alignment",
    "format",
    "data",
    "ecc",
    "padding",
]
"""The structural role of a module within its barcode symbol.

These roles are generic — they apply across all 2D barcode formats:

- ``"finder"`` — a locator pattern (QR corner squares, Data Matrix L-bar,
  Aztec bullseye).
- ``"separator"`` — always-light quiet-zone strip around a finder.
- ``"timing"`` — alternating dark/light calibration strip.
- ``"alignment"`` — secondary locator patterns (QR v2+).
- ``"format"`` — ECC level + mask indicator metadata.
- ``"data"`` — one bit of an encoded codeword.
- ``"ecc"`` — one bit of an error-correction codeword.
- ``"padding"`` — remainder/filler bits (0xEC / 0x11 alternating in QR).

Format-specific roles (e.g. QR's "dark module") live in
``ModuleAnnotation.metadata["format_role"]`` as namespaced strings like
``"qr:dark-module"``.
"""

# ============================================================================
# ModuleAnnotation — per-module role metadata for visualizers
# ============================================================================


@dataclass(frozen=True)
class ModuleAnnotation:
    """Per-module role annotation used by visualizers to colour-code symbols.

    Annotations are entirely optional.  The renderer (``layout()``) only
    reads ``ModuleGrid.modules``; it never looks at annotations unless
    ``show_annotations=True`` is set in the layout config.

    ### codeword_index and bit_index

    For ``"data"`` and ``"ecc"`` modules, these identify exactly which bit
    in which codeword this module encodes:

    - ``codeword_index`` — zero-based index into the final interleaved
      codeword stream.
    - ``bit_index`` — zero-based bit index within that codeword (0 = MSB).

    For structural modules these are ``None``.

    ### metadata

    An escape hatch for format-specific annotations, e.g.:

    - QR dark module: ``{"format_role": "qr:dark-module"}``
    - QR masked bit: ``{"format_role": "qr:masked", "mask_pattern": "3"}``
    - Aztec mode message: ``{"format_role": "aztec:mode-message"}``
    """

    role: ModuleRole
    dark: bool
    codeword_index: int | None = None
    bit_index: int | None = None
    # Arbitrary format-specific key/value pairs.
    metadata: dict[str, str] = field(default_factory=dict)


# ============================================================================
# AnnotatedModuleGrid — ModuleGrid with per-module role annotations
# ============================================================================


@dataclass(frozen=True)
class AnnotatedModuleGrid:
    """A ``ModuleGrid`` extended with per-module role annotations.

    Used by visualizers to render colour-coded diagrams that teach how the
    barcode is structured.  The ``annotations`` tuple mirrors ``modules``
    exactly in size: ``annotations[row][col]`` corresponds to
    ``modules[row][col]``.

    A ``None`` annotation means "no annotation for this module" — this can
    happen when an encoder only annotates some modules (e.g. only the data
    region) and leaves structural modules un-annotated.

    This type is NOT required for rendering.  ``layout()`` accepts a plain
    ``ModuleGrid`` and works identically whether or not annotations are
    present.
    """

    # ModuleGrid fields (duplicated to keep this a flat frozen dataclass).
    cols: int
    rows: int
    modules: tuple[tuple[bool, ...], ...]
    module_shape: ModuleShape = "square"

    # Optional annotations — mirrors modules in shape.
    annotations: tuple[tuple[ModuleAnnotation | None, ...], ...] = field(
        default_factory=tuple  # type: ignore[arg-type]
    )


# ============================================================================
# Barcode2DLayoutConfig — pixel-level rendering options
# ============================================================================


@dataclass(frozen=True)
class Barcode2DLayoutConfig:
    """Configuration for ``layout()``.

    All fields have defaults so you can construct a partial config and
    the rest fills in from ``DEFAULT_BARCODE_2D_LAYOUT_CONFIG``.

    ### module_size_px

    The size of one module in pixels.  For square modules this is both
    width and height.  For hex modules it is the hexagon's width
    (flat-to-flat, also equal to the side length for a regular hexagon).

    Must be > 0.

    ### quiet_zone_modules

    The number of module-width quiet-zone units added on each side of the
    grid.  QR Code requires a minimum of 4 modules.  Data Matrix requires
    1.  MaxiCode requires 1.

    Must be ≥ 0.

    ### module_shape

    Must match ``ModuleGrid.module_shape``.  If they disagree, ``layout()``
    raises ``InvalidBarcode2DConfigError``.  This prevents accidentally
    rendering a MaxiCode hex grid with square modules.

    | Field              | Default    | Why                                   |
    |--------------------|------------|---------------------------------------|
    | module_size_px     | 10         | Readable QR at 210×210 px             |
    | quiet_zone_modules | 4          | QR Code minimum per ISO/IEC 18004     |
    | foreground         | "#000000"  | Black ink on white paper              |
    | background         | "#ffffff"  | White paper                           |
    | show_annotations   | False      | Opt-in for visualizers                |
    | module_shape       | "square"   | The overwhelmingly common case        |
    """

    module_size_px: int = 10
    quiet_zone_modules: int = 4
    foreground: str = "#000000"
    background: str = "#ffffff"
    show_annotations: bool = False
    module_shape: ModuleShape = "square"


DEFAULT_BARCODE_2D_LAYOUT_CONFIG: Final = Barcode2DLayoutConfig()
"""Sensible defaults for ``layout()``.

These match the QR Code ISO/IEC 18004 recommendations:
- 10 px per module → a 21-module QR v1 symbol with 4-module quiet zone
  becomes 290×290 px.
- 4-module quiet zone (the QR Code minimum).
- Black foreground, white background.
"""

# ============================================================================
# Error types
# ============================================================================


class Barcode2DError(Exception):
    """Base class for all barcode-2d errors.

    Catching ``Barcode2DError`` catches any error raised by this package.
    """


class InvalidBarcode2DConfigError(Barcode2DError):
    """Raised by ``layout()`` when the configuration is invalid.

    Examples of invalid configurations:

    - ``module_size_px <= 0``
    - ``quiet_zone_modules < 0``
    - ``config.module_shape != grid.module_shape``
    """


# ============================================================================
# make_module_grid — create an all-light grid
# ============================================================================


def make_module_grid(
    rows: int,
    cols: int,
    module_shape: ModuleShape = "square",
) -> ModuleGrid:
    """Create a new ``ModuleGrid`` with every module set to ``False`` (light).

    This is the starting point for every 2D barcode encoder.  The encoder
    calls ``make_module_grid(rows, cols)`` and then uses ``set_module()``
    to paint dark modules one by one as it places finder patterns, timing
    strips, data bits, and error correction bits.

    Parameters
    ----------
    rows:
        Number of rows (height of the grid).
    cols:
        Number of columns (width of the grid).
    module_shape:
        Shape of each module.  Defaults to ``"square"``.

    Returns
    -------
    ModuleGrid
        A frozen ``ModuleGrid`` with all modules set to ``False``.

    Example::

        grid = make_module_grid(21, 21)
        assert grid.modules[0][0] is False   # all light
        assert grid.rows == 21
        assert grid.cols == 21
    """
    # Build a tuple of tuples of False values.  Each row is a separate
    # tuple so that ``set_module()`` can replace individual rows cheaply
    # using tuple slicing rather than copying the entire 2D structure.
    modules: tuple[tuple[bool, ...], ...] = tuple(
        tuple(False for _ in range(cols)) for _ in range(rows)
    )
    return ModuleGrid(cols=cols, rows=rows, modules=modules, module_shape=module_shape)


# ============================================================================
# set_module — immutable single-module update
# ============================================================================


def set_module(
    grid: ModuleGrid,
    row: int,
    col: int,
    dark: bool,
) -> ModuleGrid:
    """Return a new ``ModuleGrid`` with module ``(row, col)`` set to ``dark``.

    This function is **pure and immutable** — it never modifies the input
    grid.  Only the affected row is re-allocated; all other rows are shared
    between old and new grids.

    ### Why immutability matters

    Barcode encoders often need to backtrack (e.g. trying different QR mask
    patterns).  Immutable grids make this trivial — save the grid before
    trying a mask, evaluate it, discard if the score is worse, keep the
    old one if it is better.  No undo stack needed.

    Parameters
    ----------
    grid:
        The source grid.  Not modified.
    row:
        Row index (0-based, 0 = top).
    col:
        Column index (0-based, 0 = left).
    dark:
        ``True`` to set a dark module; ``False`` for light.

    Returns
    -------
    ModuleGrid
        A new ``ModuleGrid`` identical to ``grid`` except at ``(row, col)``.

    Raises
    ------
    IndexError
        If ``row`` or ``col`` is outside the grid dimensions.

    Example::

        g = make_module_grid(3, 3)
        g2 = set_module(g, 1, 1, True)
        assert g.modules[1][1] is False   # original unchanged
        assert g2.modules[1][1] is True   # new grid updated
        assert g is not g2                 # different object
    """
    if row < 0 or row >= grid.rows:
        raise IndexError(f"set_module: row {row} out of range [0, {grid.rows - 1}]")
    if col < 0 or col >= grid.cols:
        raise IndexError(f"set_module: col {col} out of range [0, {grid.cols - 1}]")

    # Copy only the affected row; share all other rows (immutable tuples).
    old_row: tuple[bool, ...] = grid.modules[row]
    new_row: tuple[bool, ...] = old_row[:col] + (dark,) + old_row[col + 1 :]

    new_modules: tuple[tuple[bool, ...], ...] = (
        grid.modules[:row] + (new_row,) + grid.modules[row + 1 :]
    )

    return ModuleGrid(
        cols=grid.cols,
        rows=grid.rows,
        modules=new_modules,
        module_shape=grid.module_shape,
    )


# ============================================================================
# layout — ModuleGrid → PaintScene
# ============================================================================


def layout(
    grid: ModuleGrid,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Convert a ``ModuleGrid`` into a ``PaintScene`` ready for the PaintVM.

    This is the **only** function in the entire 2D barcode stack that knows
    about pixels.  Everything above this step works in abstract module
    units.  Everything below this step is handled by the paint backend.

    ### Square modules (the common case)

    Each dark module at ``(row, col)`` becomes one ``PaintRectInstruction``:

    .. code-block::

        quiet_zone_px = quiet_zone_modules * module_size_px
        x = quiet_zone_px + col * module_size_px
        y = quiet_zone_px + row * module_size_px

    Total symbol size (including quiet zone on all four sides):

    .. code-block::

        total_width  = (cols + 2 * quiet_zone_modules) * module_size_px
        total_height = (rows + 2 * quiet_zone_modules) * module_size_px

    The scene always starts with one background ``PaintRectInstruction``
    covering the full symbol.  This ensures the quiet zone and light
    modules are filled even if the backend has a transparent default.

    ### Hex modules (MaxiCode)

    Each dark module at ``(row, col)`` becomes one ``PaintPathInstruction``
    tracing a flat-top regular hexagon.  Odd-numbered rows are offset by
    half a hexagon width to produce the standard hexagonal tiling:

    .. code-block::

        Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
        Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
        Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡

    Geometry for a flat-top hexagon centred at ``(cx, cy)`` with
    circumradius R (centre to vertex distance):

    .. code-block::

        hex_width  = module_size_px
        hex_height = module_size_px * (√3 / 2)
        circum_r   = module_size_px / √3

        Vertex angles: 0°, 60°, 120°, 180°, 240°, 300°
        Vertex i: ( cx + R·cos(i·60°),  cy + R·sin(i·60°) )

    Centre coordinates including quiet-zone offset:

    .. code-block::

        cx = quiet_zone_px + col * hex_width + (row % 2) * (hex_width / 2)
        cy = quiet_zone_px + row * hex_height

    ### Validation

    Raises ``InvalidBarcode2DConfigError`` if:

    - ``module_size_px <= 0``
    - ``quiet_zone_modules < 0``
    - ``config.module_shape != grid.module_shape``

    Parameters
    ----------
    grid:
        The module grid to render.
    config:
        Layout configuration.  ``None`` uses the defaults from
        ``DEFAULT_BARCODE_2D_LAYOUT_CONFIG``.

    Returns
    -------
    PaintScene
        A ``PaintScene`` ready for the PaintVM.
    """
    # Use the supplied config or fall back to defaults.
    cfg: Barcode2DLayoutConfig = (
        config if config is not None else DEFAULT_BARCODE_2D_LAYOUT_CONFIG
    )

    # ── Validation ──────────────────────────────────────────────────────────
    if cfg.module_size_px <= 0:
        raise InvalidBarcode2DConfigError(
            f"module_size_px must be > 0, got {cfg.module_size_px}"
        )
    if cfg.quiet_zone_modules < 0:
        raise InvalidBarcode2DConfigError(
            f"quiet_zone_modules must be >= 0, got {cfg.quiet_zone_modules}"
        )
    if cfg.module_shape != grid.module_shape:
        raise InvalidBarcode2DConfigError(
            f'config.module_shape "{cfg.module_shape}" does not match '
            f'grid.module_shape "{grid.module_shape}"'
        )

    # Dispatch to the correct rendering path.
    if cfg.module_shape == "square":
        return _layout_square(grid, cfg)
    else:
        return _layout_hex(grid, cfg)


# ============================================================================
# _layout_square — internal helper for square-module grids
# ============================================================================


def _layout_square(grid: ModuleGrid, cfg: Barcode2DLayoutConfig) -> PaintScene:
    """Render a square-module ``ModuleGrid`` into a ``PaintScene``.

    Called only by ``layout()`` after validation.  Not exported because
    callers should always go through ``layout()`` to ensure the config is
    validated before this function runs.

    Algorithm:

    1. Compute total pixel dimensions including quiet zone.
    2. Emit one background ``PaintRectInstruction`` covering the full symbol.
    3. For each dark module, emit one filled ``PaintRectInstruction``.

    Light modules are implicitly covered by the background rect — no
    explicit light rects are emitted.  This keeps the instruction count
    proportional to the number of dark modules rather than the total grid
    size.

    For a QR Code v1 (21×21) with default 10 px modules and 4-module quiet
    zone, this produces:

    - 1 background rect (290×290 px)
    - ~202 dark-module rects (approximately 46% of a v1 grid is dark)
    - Total: ~203 instructions
    """
    module_size_px: int = cfg.module_size_px
    quiet_zone_modules: int = cfg.quiet_zone_modules
    foreground: str = cfg.foreground
    background: str = cfg.background

    # Quiet zone in pixels on each side.
    quiet_zone_px: int = quiet_zone_modules * module_size_px

    # Total canvas dimensions including quiet zone on all four sides.
    total_width: int = (grid.cols + 2 * quiet_zone_modules) * module_size_px
    total_height: int = (grid.rows + 2 * quiet_zone_modules) * module_size_px

    instructions: list[PaintInstruction] = []

    # 1. Background: a single rect covering the entire symbol including
    #    the quiet zone.  This ensures light modules and the quiet zone
    #    are always filled, even when the backend default is transparent.
    instructions.append(paint_rect(0, 0, total_width, total_height, fill=background))

    # 2. One PaintRect per dark module.
    for row in range(grid.rows):
        for col in range(grid.cols):
            if grid.modules[row][col]:
                # Pixel origin of this module (top-left corner of its square).
                x: int = quiet_zone_px + col * module_size_px
                y: int = quiet_zone_px + row * module_size_px
                instructions.append(
                    paint_rect(x, y, module_size_px, module_size_px, fill=foreground)
                )

    return paint_scene(total_width, total_height, instructions, background=background)


# ============================================================================
# _layout_hex — internal helper for hex-module grids (MaxiCode)
# ============================================================================


def _layout_hex(grid: ModuleGrid, cfg: Barcode2DLayoutConfig) -> PaintScene:
    """Render a hex-module ``ModuleGrid`` into a ``PaintScene``.

    Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
    offset-row grid.  Odd rows are shifted right by half a hexagon width.

    ### Flat-top hexagon geometry

    A "flat-top" hexagon has two flat edges at the top and bottom::

           ___
          /   \\      ← two vertices at top
         |     |
          \\___/      ← two vertices at bottom

    Contrast with "pointy-top" which has a vertex at the top.  MaxiCode
    and most industrial standards use flat-top.

    For a flat-top hexagon centred at ``(cx, cy)`` with circumradius R::

        Vertices at angles 0°, 60°, 120°, 180°, 240°, 300°:

          angle  cos    sin    role
            0°    1      0     right midpoint
           60°   0.5   √3/2   bottom-right
          120°  -0.5   √3/2   bottom-left
          180°  -1      0     left midpoint
          240°  -0.5  -√3/2   top-left
          300°   0.5  -√3/2   top-right

    ### Tiling

    Hex grids tile by setting::

        hex_width  = module_size_px
        hex_height = module_size_px * (√3 / 2)   ← vertical row step

    Odd rows are offset by ``hex_width / 2`` to interlock with even rows::

        Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
        Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hex_width/2)
        Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
    """
    module_size_px: int = cfg.module_size_px
    quiet_zone_modules: int = cfg.quiet_zone_modules
    foreground: str = cfg.foreground
    background: str = cfg.background

    # Hex geometry constants:
    #
    #   hex_width  = flat-to-flat distance = module_size_px
    #   hex_height = vertical distance between row centres = width * (√3 / 2)
    #   circum_r   = centre-to-vertex distance = width / √3
    #
    # Why these ratios?  For a regular hexagon with side length s:
    #   flat-to-flat  = s       → hex_width = module_size_px
    #   point-to-point = s * √3 → but we want the row step, not full height
    #   row step = s * (√3 / 2) = hex_width * (√3 / 2)
    #   circumR  = s / √3 = hex_width / √3
    hex_width: float = float(module_size_px)
    hex_height: float = module_size_px * (math.sqrt(3) / 2)
    circum_r: float = module_size_px / math.sqrt(3)

    quiet_zone_px: float = quiet_zone_modules * hex_width

    # Total canvas size.  The extra ``hex_width / 2`` accounts for the
    # odd-row offset so the rightmost modules on odd rows don't clip outside
    # the canvas.
    total_width: float = (
        grid.cols + 2 * quiet_zone_modules
    ) * hex_width + hex_width / 2
    total_height: float = (grid.rows + 2 * quiet_zone_modules) * hex_height

    instructions: list[PaintInstruction] = []

    # Background rect.
    instructions.append(
        paint_rect(0, 0, int(total_width), int(total_height), fill=background)
    )

    # One PaintPath per dark module.
    for row in range(grid.rows):
        for col in range(grid.cols):
            if grid.modules[row][col]:
                # Centre of this hexagon in pixel space.
                # Odd rows shift right by hex_width / 2.
                cx: float = (
                    quiet_zone_px + col * hex_width + (row % 2) * (hex_width / 2)
                )
                cy: float = quiet_zone_px + row * hex_height

                instructions.append(
                    paint_path(
                        _build_flat_top_hex_path(cx, cy, circum_r),
                        fill=foreground,
                    )
                )

    return paint_scene(
        int(total_width),
        int(total_height),
        instructions,
        background=background,
    )


# ============================================================================
# _build_flat_top_hex_path — geometry helper
# ============================================================================


def _build_flat_top_hex_path(
    cx: float,
    cy: float,
    circum_r: float,
) -> list[PathCommand]:
    """Build the six ``PathCommand`` objects for a flat-top regular hexagon.

    The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
    from the centre ``(cx, cy)`` at circumradius ``circum_r``::

        vertex_i = ( cx + R·cos(i·60°),  cy + R·sin(i·60°) )

    The path starts with a ``"move_to"`` to vertex 0, then five ``"line_to"``
    commands to vertices 1–5, then a ``"close"`` to return to vertex 0.

    Parameters
    ----------
    cx:
        Centre x in pixels.
    cy:
        Centre y in pixels.
    circum_r:
        Circumscribed circle radius (centre to vertex) in pixels.

    Returns
    -------
    list[PathCommand]
        Seven commands: one ``"move_to"``, five ``"line_to"``, one ``"close"``.
    """
    commands: list[PathCommand] = []
    deg_to_rad: float = math.pi / 180.0

    # First vertex: move_to at angle 0°.
    angle0: float = 0 * 60 * deg_to_rad
    commands.append(
        PathCommand(
            kind="move_to",
            x=cx + circum_r * math.cos(angle0),
            y=cy + circum_r * math.sin(angle0),
        )
    )

    # Remaining 5 vertices: line_to at angles 60°, 120°, 180°, 240°, 300°.
    for i in range(1, 6):
        angle: float = i * 60 * deg_to_rad
        commands.append(
            PathCommand(
                kind="line_to",
                x=cx + circum_r * math.cos(angle),
                y=cy + circum_r * math.sin(angle),
            )
        )

    # Close the polygon back to vertex 0.
    commands.append(PathCommand(kind="close"))

    return commands


# Re-export PaintScene so callers can type the return value of layout()
# without needing to import paint-instructions themselves.
__all__ = [
    "ModuleShape",
    "ModuleGrid",
    "ModuleRole",
    "ModuleAnnotation",
    "AnnotatedModuleGrid",
    "Barcode2DLayoutConfig",
    "DEFAULT_BARCODE_2D_LAYOUT_CONFIG",
    "Barcode2DError",
    "InvalidBarcode2DConfigError",
    "make_module_grid",
    "set_module",
    "layout",
    "PaintScene",
    "__version__",
]
