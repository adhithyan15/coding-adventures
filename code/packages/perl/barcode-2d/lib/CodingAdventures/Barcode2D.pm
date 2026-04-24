package CodingAdventures::Barcode2D;

# =============================================================================
# CodingAdventures::Barcode2D
#
# Shared 2D barcode abstraction layer.
#
# This package provides the two building blocks every 2D barcode format needs:
#
#   1. ModuleGrid — the universal intermediate representation produced by every
#      2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode). It is
#      just a 2D boolean grid: 1 = dark module, 0 = light module.
#
#   2. layout() — the single function that converts abstract module coordinates
#      into pixel-level PaintScene instructions ready for the PaintVM (P2D01)
#      to render.
#
# ## Where this fits in the pipeline
#
#   Input data
#     → format encoder (qr-code, data-matrix, aztec…)
#     → ModuleGrid          ← produced by the encoder
#     → layout()            ← THIS PACKAGE converts to pixels
#     → PaintScene          ← consumed by paint-vm (P2D01)
#     → backend (SVG, Metal, Canvas, terminal…)
#
# All coordinates before layout() are measured in "module units" — abstract
# grid steps. Only layout() multiplies by module_size_px to produce real pixel
# coordinates. This means encoders never need to know anything about screen
# resolution or output format.
#
# ## Supported module shapes
#
# - square (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
#   Each module becomes a PaintRect.
#
# - hex (flat-top hexagons): used by MaxiCode. Each module becomes a PaintPath
#   tracing six vertices.
#
# ## Perl representation
#
# A ModuleGrid is a hashref:
#   {
#     rows        => $rows,        # number of rows
#     cols        => $cols,        # number of columns
#     modules     => \@modules,    # arrayref of arrayrefs of booleans (1/0)
#     module_shape => 'square',    # 'square' or 'hex'
#   }
#
# Immutability: make_module_grid() and set_module() always return new hashrefs.
# The original grid is never modified.
# =============================================================================

use strict;
use warnings;
use Carp qw(croak);
use POSIX ();

use CodingAdventures::PaintInstructions ();

our $VERSION = '0.1.0';

# =============================================================================
# Module shape constants
# =============================================================================

# The two supported module shapes.
#   SHAPE_SQUARE — rectangle grid; used by QR, Data Matrix, Aztec, PDF417
#   SHAPE_HEX    — offset-row hexagons; used by MaxiCode (ISO/IEC 16023)
use constant SHAPE_SQUARE => 'square';
use constant SHAPE_HEX    => 'hex';

# =============================================================================
# Default layout configuration
# =============================================================================

# Sensible defaults for layout().
#
# | Field              | Default   | Why                                     |
# |--------------------|-----------|-----------------------------------------|
# | module_size_px     | 10        | Produces a readable QR at ~210×210 px   |
# | quiet_zone_modules | 4         | QR Code minimum per ISO/IEC 18004       |
# | foreground         | #000000   | Black ink on white paper                |
# | background         | #ffffff   | White paper                             |
# | module_shape       | square    | The overwhelmingly common case          |
my %DEFAULT_LAYOUT_CONFIG = (
    module_size_px     => 10,
    quiet_zone_modules => 4,
    foreground         => '#000000',
    background         => '#ffffff',
    module_shape       => SHAPE_SQUARE,
);

sub default_layout_config { return { %DEFAULT_LAYOUT_CONFIG } }

# =============================================================================
# make_module_grid — create an all-light grid
# =============================================================================

# Create a new ModuleGrid of the given dimensions, with every module set to 0
# (light / background).
#
# This is the starting point for every 2D barcode encoder. The encoder calls
# make_module_grid($rows, $cols) and then uses set_module() to paint dark
# modules one by one as it places finder patterns, timing strips, data bits,
# and error correction bits.
#
# ### Example — start a 21×21 QR Code v1 grid
#
#   my $grid = CodingAdventures::Barcode2D->make_module_grid(21, 21);
#   # $grid->{modules}[0][0] == 0  (all light)
#   # $grid->{rows} == 21
#   # $grid->{cols} == 21
#
# Arguments:
#   $rows         - number of rows (height of the grid)
#   $cols         - number of columns (width of the grid)
#   $module_shape - optional shape string; defaults to 'square'
#
# Returns a ModuleGrid hashref.
sub make_module_grid {
    my ($class, $rows, $cols, $module_shape) = @_;
    $module_shape //= SHAPE_SQUARE;

    # Validate shape early so callers get a clear error message.
    croak "make_module_grid: invalid module_shape '$module_shape' (must be 'square' or 'hex')"
        unless $module_shape eq SHAPE_SQUARE || $module_shape eq SHAPE_HEX;

    # Build a 2D array of 0 values. Each row is an independent anonymous
    # arrayref so that set_module() can replace individual rows without copying
    # the entire grid.
    my @modules;
    for my $r (0 .. $rows - 1) {
        push @modules, [ (0) x $cols ];
    }

    return {
        rows         => $rows,
        cols         => $cols,
        modules      => \@modules,
        module_shape => $module_shape,
    };
}

# =============================================================================
# set_module — immutable single-module update
# =============================================================================

# Return a new ModuleGrid identical to $grid except that module at ($row, $col)
# is set to $dark (1 for dark, 0 for light).
#
# This function is pure and immutable — it never modifies the input grid. The
# original grid remains valid and unchanged. Only the affected row is
# re-allocated; all other rows are shared between old and new grids.
#
# ### Why immutability matters
#
# Barcode encoders often need to backtrack (e.g. trying different QR mask
# patterns). Immutable grids make this trivial — save the grid before trying a
# mask, evaluate it, discard if the score is worse, keep the old one if it is
# better. No undo stack needed.
#
# ### Out-of-bounds
#
# Croaks if $row or $col is outside the grid dimensions. This is a programming
# error in the encoder, not a user-facing error.
#
# ### Example
#
#   my $g  = CodingAdventures::Barcode2D->make_module_grid(3, 3);
#   my $g2 = CodingAdventures::Barcode2D->set_module($g, 1, 1, 1);
#   # $g->{modules}[1][1]  == 0  (original unchanged)
#   # $g2->{modules}[1][1] == 1
#
# Arguments:
#   $grid  - ModuleGrid hashref
#   $row   - zero-based row index
#   $col   - zero-based column index
#   $dark  - 1 for dark module, 0 for light module
#
# Returns a new ModuleGrid hashref.
sub set_module {
    my ($class, $grid, $row, $col, $dark) = @_;

    croak "set_module: row $row out of range [0, " . ($grid->{rows} - 1) . "]"
        if $row < 0 || $row >= $grid->{rows};
    croak "set_module: col $col out of range [0, " . ($grid->{cols} - 1) . "]"
        if $col < 0 || $col >= $grid->{cols};

    # Copy only the affected row; all other rows are shared (shallow copy).
    my @new_row = @{ $grid->{modules}[$row] };
    $new_row[$col] = $dark ? 1 : 0;

    # Build a new modules array. Unaffected rows are the same arrayrefs as
    # the original (no deep copy needed — we never mutate them).
    my @new_modules;
    for my $r (0 .. $grid->{rows} - 1) {
        if ($r == $row) {
            push @new_modules, \@new_row;
        } else {
            push @new_modules, $grid->{modules}[$r];
        }
    }

    return {
        rows         => $grid->{rows},
        cols         => $grid->{cols},
        modules      => \@new_modules,
        module_shape => $grid->{module_shape},
    };
}

# =============================================================================
# layout — ModuleGrid → PaintScene
# =============================================================================

# Convert a ModuleGrid into a PaintScene ready for the PaintVM.
#
# This is the ONLY function in the entire 2D barcode stack that knows about
# pixels. Everything above this step works in abstract module units. Everything
# below this step is handled by the paint backend.
#
# ### Square modules (the common case)
#
# Each dark module at (row, col) becomes one PaintRect:
#
#   quiet_zone_px = quiet_zone_modules * module_size_px
#   x = quiet_zone_px + col * module_size_px
#   y = quiet_zone_px + row * module_size_px
#
# Total symbol size (including quiet zone on all four sides):
#
#   total_width  = (cols + 2 * quiet_zone_modules) * module_size_px
#   total_height = (rows + 2 * quiet_zone_modules) * module_size_px
#
# The scene always starts with one background PaintRect covering the full
# symbol. This ensures the quiet zone and light modules are filled with the
# background color even if the backend has a transparent default.
#
# ### Hex modules (MaxiCode)
#
# Each dark module at (row, col) becomes one PaintPath tracing a flat-top
# regular hexagon. Odd-numbered rows are offset by half a hexagon width to
# produce the standard hexagonal tiling:
#
#   Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
#   Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
#   Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
#
# Geometry for a flat-top hexagon centered at (cx, cy) with circumradius R:
#
#   hex_width  = module_size_px
#   hex_height = module_size_px * (sqrt(3) / 2)
#   circum_r   = module_size_px / sqrt(3)
#
#   cx = quiet_zone_px + col * hex_width + (row % 2) * (hex_width / 2)
#   cy = quiet_zone_px + row * hex_height
#
# ### Validation
#
# Croaks with an InvalidBarcode2DConfigError-style message if:
#   - module_size_px <= 0
#   - quiet_zone_modules < 0
#   - config module_shape does not match grid module_shape
#
# Arguments:
#   $grid    - ModuleGrid hashref
#   $config  - optional hashref of layout config overrides
#
# Returns a PaintScene hashref.
sub layout {
    my ($class, $grid, $config) = @_;

    # Merge caller config on top of defaults.
    my %cfg = ( %DEFAULT_LAYOUT_CONFIG, %{ $config // {} } );

    # ── Validation ──────────────────────────────────────────────────────────
    croak "InvalidBarcode2DConfigError: module_size_px must be > 0, got $cfg{module_size_px}"
        if $cfg{module_size_px} <= 0;

    croak "InvalidBarcode2DConfigError: quiet_zone_modules must be >= 0, got $cfg{quiet_zone_modules}"
        if $cfg{quiet_zone_modules} < 0;

    croak "InvalidBarcode2DConfigError: config module_shape \"$cfg{module_shape}\" does not match grid module_shape \"$grid->{module_shape}\""
        if $cfg{module_shape} ne $grid->{module_shape};

    # Dispatch to the correct rendering path.
    if ($cfg{module_shape} eq SHAPE_SQUARE) {
        return $class->_layout_square($grid, \%cfg);
    } else {
        return $class->_layout_hex($grid, \%cfg);
    }
}

# =============================================================================
# _layout_square — internal helper for square-module grids
# =============================================================================

# Render a square-module ModuleGrid into a PaintScene.
#
# Called only by layout() after validation. The leading underscore marks this
# as an internal method — callers should always go through layout() to ensure
# config is validated.
#
# Algorithm:
#   1. Compute total pixel dimensions including quiet zone.
#   2. Emit one background PaintRect covering the entire symbol.
#   3. For each dark module, emit one filled PaintRect.
#
# Light modules are implicitly covered by the background rect — no explicit
# light rects are emitted. This keeps the instruction count proportional to
# the number of dark modules rather than the total grid size.
sub _layout_square {
    my ($class, $grid, $cfg) = @_;

    my $module_size_px     = $cfg->{module_size_px};
    my $quiet_zone_modules = $cfg->{quiet_zone_modules};
    my $foreground         = $cfg->{foreground};
    my $background         = $cfg->{background};

    # Quiet zone in pixels on each side.
    my $quiet_zone_px = $quiet_zone_modules * $module_size_px;

    # Total canvas dimensions including quiet zone on all four sides.
    my $total_width  = ($grid->{cols} + 2 * $quiet_zone_modules) * $module_size_px;
    my $total_height = ($grid->{rows} + 2 * $quiet_zone_modules) * $module_size_px;

    my @instructions;

    # 1. Background: a single rect covering the entire symbol including quiet
    #    zone. This ensures light modules and the quiet zone are always filled,
    #    even when the backend default is transparent.
    push @instructions, CodingAdventures::PaintInstructions->paint_rect(
        0, 0, $total_width, $total_height, $background,
    );

    # 2. One PaintRect per dark module.
    for my $row (0 .. $grid->{rows} - 1) {
        for my $col (0 .. $grid->{cols} - 1) {
            if ($grid->{modules}[$row][$col]) {
                # Pixel origin of this module (top-left corner of its square).
                my $x = $quiet_zone_px + $col * $module_size_px;
                my $y = $quiet_zone_px + $row * $module_size_px;

                push @instructions, CodingAdventures::PaintInstructions->paint_rect(
                    $x, $y, $module_size_px, $module_size_px, $foreground,
                );
            }
        }
    }

    return CodingAdventures::PaintInstructions->paint_scene(
        $total_width, $total_height, \@instructions, $background,
    );
}

# =============================================================================
# _layout_hex — internal helper for hex-module grids (MaxiCode)
# =============================================================================

# Render a hex-module ModuleGrid into a PaintScene.
#
# Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
# offset-row grid. Odd rows are shifted right by half a hexagon width.
#
# ### Flat-top hexagon geometry reminder
#
# A "flat-top" hexagon has two flat edges at the top and bottom:
#
#    ___
#   /   \      ← two vertices at top
#  |     |
#   \___/      ← two vertices at bottom
#
# For a flat-top hexagon centered at (cx, cy) with circumradius R:
#
#   Vertices at angles 0°, 60°, 120°, 180°, 240°, 300°:
#
#   angle    cos     sin      role
#     0°      1       0       right midpoint
#    60°     0.5    √3/2     bottom-right
#   120°    -0.5    √3/2     bottom-left
#   180°    -1       0       left midpoint
#   240°    -0.5   -√3/2    top-left
#   300°     0.5   -√3/2    top-right
#
# ### Tiling
#
# Hex grids tile by setting:
#   hex_width  = module_size_px
#   hex_height = module_size_px * (sqrt(3) / 2)  ← vertical distance between row centers
#
# Odd rows are offset by hex_width / 2 to interlock with even rows:
#
#   Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
#   Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hex_width/2)
#   Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
sub _layout_hex {
    my ($class, $grid, $cfg) = @_;

    my $module_size_px     = $cfg->{module_size_px};
    my $quiet_zone_modules = $cfg->{quiet_zone_modules};
    my $foreground         = $cfg->{foreground};
    my $background         = $cfg->{background};

    # Hex geometry:
    #   hex_width  = one module width (flat-to-flat = side length for regular hex)
    #   hex_height = vertical distance between row centers
    #   circum_r   = center-to-vertex distance (circumscribed circle radius)
    #
    # For a regular hexagon where side length = s:
    #   flat-to-flat distance = s  →  hex_width = module_size_px
    #   row step = s * (sqrt(3) / 2) = hex_width * (sqrt(3) / 2)
    #   circum_r = s / sqrt(3) = hex_width / sqrt(3)
    my $hex_width  = $module_size_px;
    my $hex_height = $module_size_px * (sqrt(3) / 2);
    my $circum_r   = $module_size_px / sqrt(3);

    my $quiet_zone_px = $quiet_zone_modules * $module_size_px;

    # Total canvas size. The + hex_width/2 accounts for the odd-row offset so
    # the rightmost modules on odd rows don't clip outside the canvas.
    my $total_width  = ($grid->{cols} + 2 * $quiet_zone_modules) * $hex_width + $hex_width / 2;
    my $total_height = ($grid->{rows} + 2 * $quiet_zone_modules) * $hex_height;

    my @instructions;

    # Background rect.
    push @instructions, CodingAdventures::PaintInstructions->paint_rect(
        0, 0, $total_width, $total_height, $background,
    );

    # One PaintPath per dark module.
    for my $row (0 .. $grid->{rows} - 1) {
        for my $col (0 .. $grid->{cols} - 1) {
            if ($grid->{modules}[$row][$col]) {
                # Center of this hexagon in pixel space.
                # Odd rows shift right by hex_width/2.
                my $cx = $quiet_zone_px
                       + $col * $hex_width
                       + ($row % 2) * ($hex_width / 2);
                my $cy = $quiet_zone_px + $row * $hex_height;

                my $commands = _build_flat_top_hex_path($cx, $cy, $circum_r);
                push @instructions, CodingAdventures::PaintInstructions->paint_path(
                    $commands, $foreground,
                );
            }
        }
    }

    return CodingAdventures::PaintInstructions->paint_scene(
        $total_width, $total_height, \@instructions, $background,
    );
}

# =============================================================================
# _build_flat_top_hex_path — geometry helper
# =============================================================================

# Build the six PathCommands for a flat-top regular hexagon.
#
# The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300° from
# the center (cx, cy) at circumradius R:
#
#   vertex_i = ( cx + R * cos(i * 60°),
#                cy + R * sin(i * 60°) )
#
# The path starts with a move_to to vertex 0, then five line_to commands to
# vertices 1–5, then a close to return to vertex 0.
#
# Arguments:
#   $cx       - center x in pixels
#   $cy       - center y in pixels
#   $circum_r - circumscribed circle radius (center to vertex) in pixels
#
# Returns an arrayref of PathCommand hashrefs.
sub _build_flat_top_hex_path {
    my ($cx, $cy, $circum_r) = @_;

    my $deg_to_rad = 3.14159265358979323846 / 180;
    my @commands;

    # First vertex: move_to
    my $angle0 = 0 * 60 * $deg_to_rad;
    push @commands, {
        kind => 'move_to',
        x    => $cx + $circum_r * cos($angle0),
        y    => $cy + $circum_r * sin($angle0),
    };

    # Remaining 5 vertices: line_to
    for my $i (1 .. 5) {
        my $angle = $i * 60 * $deg_to_rad;
        push @commands, {
            kind => 'line_to',
            x    => $cx + $circum_r * cos($angle),
            y    => $cy + $circum_r * sin($angle),
        };
    }

    # Close back to vertex 0.
    push @commands, { kind => 'close' };

    return \@commands;
}

1;
