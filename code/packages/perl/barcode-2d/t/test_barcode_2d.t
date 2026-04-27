use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# CodingAdventures::Barcode2D — comprehensive test suite
#
# Tests mirror the TypeScript barcode-2d test cases and also exercise the
# Perl-specific API (hashref shapes, croak messages, etc.).
# ---------------------------------------------------------------------------

use lib '../paint-instructions/lib';
require CodingAdventures::Barcode2D;
my $pkg = 'CodingAdventures::Barcode2D';

# ============================================================================
# 1. Module constants
# ============================================================================

subtest 'module shape constants' => sub {
    is( CodingAdventures::Barcode2D->SHAPE_SQUARE, 'square', 'SHAPE_SQUARE is "square"' );
    is( CodingAdventures::Barcode2D->SHAPE_HEX,    'hex',    'SHAPE_HEX is "hex"' );
};

# ============================================================================
# 2. make_module_grid
# ============================================================================

subtest 'make_module_grid — basic properties' => sub {
    my $grid = $pkg->make_module_grid(21, 21);
    is( $grid->{rows},         21,       'rows == 21' );
    is( $grid->{cols},         21,       'cols == 21' );
    is( $grid->{module_shape}, 'square', 'default shape is square' );
    is( scalar @{ $grid->{modules} }, 21, '21 rows in modules array' );
    is( scalar @{ $grid->{modules}[0] }, 21, '21 cols in row 0' );

    # All modules start light (0).
    my $all_zero = 1;
    for my $r (0 .. 20) {
        for my $c (0 .. 20) {
            $all_zero = 0 if $grid->{modules}[$r][$c] != 0;
        }
    }
    ok( $all_zero, 'all modules initialised to 0 (light)' );
};

subtest 'make_module_grid — hex shape' => sub {
    my $grid = $pkg->make_module_grid(33, 30, 'hex');
    is( $grid->{rows},         33,   'rows == 33' );
    is( $grid->{cols},         30,   'cols == 30' );
    is( $grid->{module_shape}, 'hex', 'shape is hex' );
};

subtest 'make_module_grid — non-square dimensions' => sub {
    my $grid = $pkg->make_module_grid(5, 10);
    is( $grid->{rows}, 5,  'rows == 5' );
    is( $grid->{cols}, 10, 'cols == 10' );
    is( scalar @{ $grid->{modules} },    5,  '5 rows' );
    is( scalar @{ $grid->{modules}[0] }, 10, '10 cols per row' );
};

subtest 'make_module_grid — invalid shape croaks' => sub {
    ok(
        dies { $pkg->make_module_grid(5, 5, 'triangle') },
        'invalid shape croaks',
    );
};

# ============================================================================
# 3. set_module
# ============================================================================

subtest 'set_module — immutability' => sub {
    my $g  = $pkg->make_module_grid(3, 3);
    my $g2 = $pkg->set_module($g, 1, 1, 1);

    # Original unchanged.
    is( $g->{modules}[1][1],  0, 'original grid unchanged at (1,1)' );

    # New grid has the dark module.
    is( $g2->{modules}[1][1], 1, 'new grid has dark at (1,1)' );

    # Unaffected modules in new grid match original.
    is( $g2->{modules}[0][0], 0, 'new grid (0,0) still light' );
    is( $g2->{modules}[2][2], 0, 'new grid (2,2) still light' );
};

subtest 'set_module — round-trip multiple calls' => sub {
    my $g = $pkg->make_module_grid(5, 5);
    $g = $pkg->set_module($g, 0, 0, 1);
    $g = $pkg->set_module($g, 4, 4, 1);
    $g = $pkg->set_module($g, 2, 2, 1);

    is( $g->{modules}[0][0], 1, '(0,0) dark' );
    is( $g->{modules}[4][4], 1, '(4,4) dark' );
    is( $g->{modules}[2][2], 1, '(2,2) dark' );
    is( $g->{modules}[0][1], 0, '(0,1) still light' );
};

subtest 'set_module — set back to light' => sub {
    my $g  = $pkg->make_module_grid(3, 3);
    my $g2 = $pkg->set_module($g, 1, 1, 1);
    my $g3 = $pkg->set_module($g2, 1, 1, 0);

    is( $g3->{modules}[1][1], 0, 'module set back to light' );
};

subtest 'set_module — row out of range croaks' => sub {
    my $g = $pkg->make_module_grid(3, 3);
    ok( dies { $pkg->set_module($g, -1, 0, 1) }, 'negative row croaks' );
    ok( dies { $pkg->set_module($g, 3,  0, 1) }, 'row == rows croaks' );
};

subtest 'set_module — col out of range croaks' => sub {
    my $g = $pkg->make_module_grid(3, 3);
    ok( dies { $pkg->set_module($g, 0, -1, 1) }, 'negative col croaks' );
    ok( dies { $pkg->set_module($g, 0,  3, 1) }, 'col == cols croaks' );
};

subtest 'set_module — preserves module_shape' => sub {
    my $g  = $pkg->make_module_grid(3, 3, 'hex');
    my $g2 = $pkg->set_module($g, 0, 0, 1);
    is( $g2->{module_shape}, 'hex', 'module_shape preserved through set_module' );
};

# ============================================================================
# 4. default_layout_config
# ============================================================================

subtest 'default_layout_config' => sub {
    my $cfg = $pkg->default_layout_config();
    is( $cfg->{module_size_px},     10,       'module_size_px default 10' );
    is( $cfg->{quiet_zone_modules}, 4,        'quiet_zone_modules default 4' );
    is( $cfg->{foreground},         '#000000','foreground default #000000' );
    is( $cfg->{background},         '#ffffff','background default #ffffff' );
    is( $cfg->{module_shape},       'square', 'module_shape default square' );
};

# ============================================================================
# 5. layout — square modules
# ============================================================================

subtest 'layout — empty 1×1 grid (all light) produces just background' => sub {
    my $grid  = $pkg->make_module_grid(1, 1);
    my $scene = $pkg->layout($grid, { quiet_zone_modules => 0 });

    # Total size with no quiet zone: 1×1 modules × 10 px each.
    is( $scene->{width},  10, 'scene width 10' );
    is( $scene->{height}, 10, 'scene height 10' );

    # Only one instruction: the background rect.
    is( scalar @{ $scene->{instructions} }, 1, 'one instruction (background only)' );
    is( $scene->{instructions}[0]{kind}, 'rect', 'instruction is a rect' );
    is( $scene->{instructions}[0]{fill}, '#ffffff', 'fill is background color' );
};

subtest 'layout — single dark module produces 2 instructions' => sub {
    my $grid  = $pkg->make_module_grid(1, 1);
    $grid = $pkg->set_module($grid, 0, 0, 1);
    my $scene = $pkg->layout($grid, { quiet_zone_modules => 0 });

    # Background + one dark rect.
    is( scalar @{ $scene->{instructions} }, 2, 'two instructions' );
    is( $scene->{instructions}[1]{kind}, 'rect', 'second instruction is a rect' );
    is( $scene->{instructions}[1]{fill}, '#000000', 'dark module fill is foreground' );
};

subtest 'layout — 3×3 grid with all dark modules' => sub {
    my $grid = $pkg->make_module_grid(3, 3);
    for my $r (0 .. 2) {
        for my $c (0 .. 2) {
            $grid = $pkg->set_module($grid, $r, $c, 1);
        }
    }

    my $scene = $pkg->layout($grid, { quiet_zone_modules => 0, module_size_px => 5 });

    # Total: 3 cols × 5 px = 15 wide,  3 rows × 5 px = 15 tall.
    is( $scene->{width},  15, 'scene width 15' );
    is( $scene->{height}, 15, 'scene height 15' );

    # Background + 9 dark module rects.
    is( scalar @{ $scene->{instructions} }, 10, '10 instructions (1 bg + 9 modules)' );
};

subtest 'layout — quiet zone contributes to total size' => sub {
    # 5×5 grid with 4-module quiet zone, 10 px per module.
    my $grid  = $pkg->make_module_grid(5, 5);
    my $scene = $pkg->layout($grid);  # defaults: qz=4, sz=10

    # (5 + 2*4) * 10 = 130
    is( $scene->{width},  130, 'total width 130 (5 + 8 qz) * 10' );
    is( $scene->{height}, 130, 'total height 130' );
};

subtest 'layout — quiet zone 0 produces exact module size' => sub {
    my $grid  = $pkg->make_module_grid(21, 21);
    my $scene = $pkg->layout($grid, { quiet_zone_modules => 0, module_size_px => 2 });

    is( $scene->{width},  42, 'width 21*2=42' );
    is( $scene->{height}, 42, 'height 21*2=42' );
};

subtest 'layout — module pixel positions are correct' => sub {
    # 2×2 grid; all dark. No quiet zone. module_size_px=5.
    my $grid = $pkg->make_module_grid(2, 2);
    for my $r (0 .. 1) {
        for my $c (0 .. 1) {
            $grid = $pkg->set_module($grid, $r, $c, 1);
        }
    }

    my $scene = $pkg->layout($grid, { quiet_zone_modules => 0, module_size_px => 5 });

    # instructions[0] is background. instructions[1..4] are the four modules.
    # Expected positions (row, col) → (x, y):
    #   (0,0) → (0,0)   (0,1) → (5,0)
    #   (1,0) → (0,5)   (1,1) → (5,5)
    my @dark = @{ $scene->{instructions} }[1 .. 4];

    # Sort by (y, x) to get a deterministic order.
    my @sorted = sort { $a->{y} <=> $b->{y} || $a->{x} <=> $b->{x} } @dark;

    is( $sorted[0]{x}, 0, 'module (0,0) x=0' );
    is( $sorted[0]{y}, 0, 'module (0,0) y=0' );
    is( $sorted[1]{x}, 5, 'module (0,1) x=5' );
    is( $sorted[1]{y}, 0, 'module (0,1) y=0' );
    is( $sorted[2]{x}, 0, 'module (1,0) x=0' );
    is( $sorted[2]{y}, 5, 'module (1,0) y=5' );
    is( $sorted[3]{x}, 5, 'module (1,1) x=5' );
    is( $sorted[3]{y}, 5, 'module (1,1) y=5' );
};

subtest 'layout — custom colors' => sub {
    my $grid = $pkg->make_module_grid(1, 1);
    $grid = $pkg->set_module($grid, 0, 0, 1);
    my $scene = $pkg->layout($grid, {
        quiet_zone_modules => 0,
        foreground         => '#ff0000',
        background         => '#00ff00',
    });

    is( $scene->{background}, '#00ff00', 'scene background color' );
    is( $scene->{instructions}[0]{fill}, '#00ff00', 'bg rect fill is background' );
    is( $scene->{instructions}[1]{fill}, '#ff0000', 'dark module fill is foreground' );
};

subtest 'layout — module_size_px=1 edge case' => sub {
    my $grid  = $pkg->make_module_grid(3, 3);
    my $scene = $pkg->layout($grid, { quiet_zone_modules => 0, module_size_px => 1 });
    is( $scene->{width},  3, 'width 3' );
    is( $scene->{height}, 3, 'height 3' );
};

# ============================================================================
# 6. layout — validation errors
# ============================================================================

subtest 'layout — module_size_px <= 0 croaks' => sub {
    my $grid = $pkg->make_module_grid(3, 3);
    ok( dies { $pkg->layout($grid, { module_size_px => 0  }) },  'module_size_px=0 croaks' );
    ok( dies { $pkg->layout($grid, { module_size_px => -1 }) },  'module_size_px=-1 croaks' );
};

subtest 'layout — quiet_zone_modules < 0 croaks' => sub {
    my $grid = $pkg->make_module_grid(3, 3);
    ok( dies { $pkg->layout($grid, { quiet_zone_modules => -1 }) }, 'qz=-1 croaks' );
};

subtest 'layout — module_shape mismatch croaks' => sub {
    my $hex_grid = $pkg->make_module_grid(3, 3, 'hex');
    ok(
        dies { $pkg->layout($hex_grid, { module_shape => 'square' }) },
        'shape mismatch croaks',
    );

    my $sq_grid = $pkg->make_module_grid(3, 3, 'square');
    ok(
        dies { $pkg->layout($sq_grid, { module_shape => 'hex' }) },
        'sq grid with hex config croaks',
    );
};

# ============================================================================
# 7. layout — hex modules (MaxiCode)
# ============================================================================

subtest 'layout — hex grid: all light produces only background' => sub {
    my $grid  = $pkg->make_module_grid(3, 3, 'hex');
    my $scene = $pkg->layout($grid, {
        module_shape       => 'hex',
        module_size_px     => 10,
        quiet_zone_modules => 0,
    });

    # Only the background rect.
    is( scalar @{ $scene->{instructions} }, 1, 'only bg instruction for all-light hex grid' );
    is( $scene->{instructions}[0]{kind}, 'rect', 'background is a rect' );
};

subtest 'layout — hex grid: dark module produces a path' => sub {
    my $grid  = $pkg->make_module_grid(3, 3, 'hex');
    $grid = $pkg->set_module($grid, 0, 0, 1);
    my $scene = $pkg->layout($grid, {
        module_shape       => 'hex',
        module_size_px     => 10,
        quiet_zone_modules => 0,
    });

    # bg rect + 1 path.
    is( scalar @{ $scene->{instructions} }, 2, 'bg + 1 path' );
    is( $scene->{instructions}[1]{kind}, 'path', 'dark hex module is a path' );

    # The path must have exactly 7 commands: move_to + 5×line_to + close.
    my $cmds = $scene->{instructions}[1]{commands};
    is( scalar @$cmds, 7, 'hex path has 7 commands' );
    is( $cmds->[0]{kind}, 'move_to', 'first command is move_to' );
    is( $cmds->[6]{kind}, 'close',   'last command is close' );
};

subtest 'layout — hex grid: all dark modules produce correct count' => sub {
    my $grid = $pkg->make_module_grid(2, 3, 'hex');
    for my $r (0 .. 1) {
        for my $c (0 .. 2) {
            $grid = $pkg->set_module($grid, $r, $c, 1);
        }
    }
    my $scene = $pkg->layout($grid, {
        module_shape       => 'hex',
        module_size_px     => 10,
        quiet_zone_modules => 0,
    });

    # 1 bg + 6 paths.
    is( scalar @{ $scene->{instructions} }, 7, '7 instructions for 2×3 all-dark hex' );
    for my $i (1 .. 6) {
        is( $scene->{instructions}[$i]{kind}, 'path', "instruction $i is a path" );
    }
};

subtest 'layout — hex grid: total size formula' => sub {
    # 4 cols × 2 rows, module_size_px=10, qz=1.
    # hex_width=10, hex_height=10*(sqrt(3)/2) ≈ 8.660
    # total_width  = (4 + 2*1) * 10 + 10/2 = 65
    # total_height = (2 + 2*1) * 8.660 ≈ 34.641
    my $grid  = $pkg->make_module_grid(2, 4, 'hex');
    my $scene = $pkg->layout($grid, {
        module_shape       => 'hex',
        module_size_px     => 10,
        quiet_zone_modules => 1,
    });

    # Use numeric comparisons with tolerance for floating-point.
    ok( abs($scene->{width} - 65) < 0.001, "hex total width ≈ 65 (got $scene->{width})" );
    my $expected_h = 4 * 10 * (sqrt(3) / 2);
    ok( abs($scene->{height} - $expected_h) < 0.001, "hex total height ≈ $expected_h" );
};

subtest 'layout — hex grid: odd-row offset shifts cx' => sub {
    # Row 0 col 0 and row 1 col 0, qz=0, size=10.
    # Row 0 cx = 0 + 0 * 10 + 0 * 5 = 0 (first vertex at x = 0 + circum_r)
    # Row 1 cx = 0 + 0 * 10 + 1 * 5 = 5
    my $grid = $pkg->make_module_grid(2, 1, 'hex');
    $grid = $pkg->set_module($grid, 0, 0, 1);
    $grid = $pkg->set_module($grid, 1, 0, 1);
    my $scene = $pkg->layout($grid, {
        module_shape       => 'hex',
        module_size_px     => 10,
        quiet_zone_modules => 0,
    });

    # instructions[1] = row-0 path, instructions[2] = row-1 path.
    my $row0_path = $scene->{instructions}[1];
    my $row1_path = $scene->{instructions}[2];

    # circum_r = 10 / sqrt(3) ≈ 5.773
    my $r = 10 / sqrt(3);

    # Row 0: cx=0, first vertex x = cx + r = r
    # Row 1: cx=5, first vertex x = cx + r = 5 + r
    my $row0_first_x = $row0_path->{commands}[0]{x};
    my $row1_first_x = $row1_path->{commands}[0]{x};

    ok( abs($row0_first_x - $r) < 0.001,       "row-0 first vertex x ≈ $r" );
    ok( abs($row1_first_x - (5 + $r)) < 0.001, "row-1 first vertex x ≈ " . (5+$r) );
};

# ============================================================================
# 8. Larger realistic QR-like grid
# ============================================================================

subtest 'layout — 21×21 grid with finder-like pattern' => sub {
    # Mimics QR Code v1 top-left finder pattern (7×7 filled border).
    my $grid = $pkg->make_module_grid(21, 21);

    # Paint the 7×7 finder pattern border at top-left.
    for my $r (0 .. 6) {
        for my $c (0 .. 6) {
            # Outer ring: row/col 0 or 6.
            # Inner ring: row/col 2..4 (3×3 center).
            my $is_outer = ($r == 0 || $r == 6 || $c == 0 || $c == 6);
            my $is_inner = ($r >= 2 && $r <= 4 && $c >= 2 && $c <= 4);
            if ($is_outer || $is_inner) {
                $grid = $pkg->set_module($grid, $r, $c, 1);
            }
        }
    }

    my $scene = $pkg->layout($grid);

    # Total size: (21 + 8) * 10 = 290.
    is( $scene->{width},  290, 'scene width 290 (21 + 8 qz) * 10' );
    is( $scene->{height}, 290, 'scene height 290' );

    # All non-background instructions should be rects.
    for my $i (1 .. $#{ $scene->{instructions} }) {
        is( $scene->{instructions}[$i]{kind}, 'rect', "instruction $i is rect" );
    }
};

done_testing;
