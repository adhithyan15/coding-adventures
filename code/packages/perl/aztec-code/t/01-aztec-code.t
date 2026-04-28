#!/usr/bin/env perl
# =============================================================================
# 01-aztec-code.t — test suite for CodingAdventures::AztecCode
# =============================================================================
#
# Mirrors code/packages/typescript/aztec-code/tests/aztec-code.test.ts.
# Uses Test::More so it works with both core Perl and CPAN. Coverage targets:
#   - Compact symbol sizes (1-4 layers, 15..27 modules)
#   - Full symbol sizes (1-32 layers, >= 19)
#   - Bullseye finder pattern (compact and full)
#   - Orientation marks at both compact and full mode-ring corners
#   - Grid structural invariants (square, dimensions)
#   - Byte-array vs scalar-string equivalence
#   - min_ecc_percent option (low vs high)
#   - Determinism (same input -> same grid)
#   - Mode message bit placement (indirect via differing inputs)
#   - GF(16) and GF(256)/0x12D RS arithmetic (indirect)
#   - Reference grid presence in full symbols
#   - Error path: input too long
# =============================================================================

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test::More;
use CodingAdventures::AztecCode qw(encode);

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# dark — return 1 iff the module at (row, col) of $grid is dark (truthy).
sub dark {
    my ( $grid, $row, $col ) = @_;
    return $grid->{modules}[$row][$col] ? 1 : 0;
}

# grid_string — flatten the entire module grid to a string of 0s and 1s,
# row-major. Useful for equality / inequality comparisons.
sub grid_string {
    my ($grid) = @_;
    return join '', map { join '', @$_ } @{ $grid->{modules} };
}

# count_dark — number of dark modules in the grid.
sub count_dark {
    my ($grid) = @_;
    my $n = 0;
    for my $row ( @{ $grid->{modules} } ) {
        $n += grep { $_ } @$row;
    }
    return $n;
}

# ---------------------------------------------------------------------------
# 1. Compact symbol sizes
# ---------------------------------------------------------------------------

subtest 'compact symbol sizes' => sub {
    # Single ASCII byte 'A' fits in compact 1-layer (15x15).
    my $g1 = encode("A");
    is $g1->{rows}, 15, "1-layer compact = 15 rows for 'A'";
    is $g1->{cols}, 15, "1-layer compact = 15 cols for 'A'";

    # 'Hello' (5 bytes + 5 BS escape + 5 length = 50 bits) fits in compact-2 (19x19).
    my $g2 = encode("Hello");
    is $g2->{rows}, 19, "2-layer compact = 19 rows for 'Hello'";
    is $g2->{cols}, 19, "2-layer compact = 19 cols for 'Hello'";

    # 20-byte input overflows compact-2 -> compact-3 (23x23).
    my $g3 = encode("12345678901234567890");
    is $g3->{rows}, 23, "3-layer compact = 23 rows for 20-byte input";
    is $g3->{cols}, 23, "3-layer compact = 23 cols for 20-byte input";

    # 40-byte input overflows compact-3 -> compact-4 (27x27).
    my $g4 = encode( "12345678901234567890" x 2 );
    is $g4->{rows}, 27, "4-layer compact = 27 rows for 40-byte input";
    is $g4->{cols}, 27, "4-layer compact = 27 cols for 40-byte input";
};

# ---------------------------------------------------------------------------
# 2. Full symbol sizes
# ---------------------------------------------------------------------------

subtest 'full symbol sizes' => sub {
    # 100 bytes overflows compact-4 -> a full symbol.
    my $g = encode( 'x' x 100 );
    cmp_ok $g->{rows}, '>=', 19, 'full symbol >= 19 modules';
    is $g->{rows} % 4, 3, 'full symbol size congruent to 3 mod 4 (15 + 4*L)';

    # Square invariant.
    my $big = encode( 'x' x 150 );
    is $big->{rows}, $big->{cols}, 'full symbol is square';
};

# ---------------------------------------------------------------------------
# 3. Bullseye pattern — compact 15x15
# ---------------------------------------------------------------------------

subtest 'bullseye pattern - compact 15x15' => sub {
    my $g  = encode("A");
    my $cx = 7;
    my $cy = 7;

    # Center module is always DARK.
    is dark( $g, $cy, $cx ), 1, 'center (d=0) DARK';

    # Distance-1 ring (8 neighbours) all DARK.
    for my $dr ( -1 .. 1 ) {
        for my $dc ( -1 .. 1 ) {
            next if $dr == 0 && $dc == 0;
            is dark( $g, $cy + $dr, $cx + $dc ), 1,
                "d=1 ring ($dr,$dc) DARK";
        }
    }

    # Distance-2 ring (corners + midpoints) all LIGHT.
    is dark( $g, $cy - 2, $cx - 2 ), 0, 'd=2 NW corner LIGHT';
    is dark( $g, $cy - 2, $cx + 2 ), 0, 'd=2 NE corner LIGHT';
    is dark( $g, $cy + 2, $cx - 2 ), 0, 'd=2 SW corner LIGHT';
    is dark( $g, $cy + 2, $cx + 2 ), 0, 'd=2 SE corner LIGHT';
    is dark( $g, $cy - 2, $cx ),     0, 'd=2 N midpoint LIGHT';
    is dark( $g, $cy + 2, $cx ),     0, 'd=2 S midpoint LIGHT';
    is dark( $g, $cy,     $cx - 2 ), 0, 'd=2 W midpoint LIGHT';
    is dark( $g, $cy,     $cx + 2 ), 0, 'd=2 E midpoint LIGHT';

    # Distance-3 ring (cardinals) DARK.
    is dark( $g, $cy - 3, $cx ),     1, 'd=3 N DARK';
    is dark( $g, $cy + 3, $cx ),     1, 'd=3 S DARK';
    is dark( $g, $cy,     $cx - 3 ), 1, 'd=3 W DARK';
    is dark( $g, $cy,     $cx + 3 ), 1, 'd=3 E DARK';

    # Distance-4 ring (cardinals) LIGHT.
    is dark( $g, $cy - 4, $cx ),     0, 'd=4 N LIGHT';
    is dark( $g, $cy + 4, $cx ),     0, 'd=4 S LIGHT';
    is dark( $g, $cy,     $cx - 4 ), 0, 'd=4 W LIGHT';
    is dark( $g, $cy,     $cx + 4 ), 0, 'd=4 E LIGHT';

    # Distance-5 ring (cardinals) DARK (outer bullseye ring).
    is dark( $g, $cy - 5, $cx ),     1, 'd=5 N DARK';
    is dark( $g, $cy + 5, $cx ),     1, 'd=5 S DARK';
    is dark( $g, $cy,     $cx - 5 ), 1, 'd=5 W DARK';
    is dark( $g, $cy,     $cx + 5 ), 1, 'd=5 E DARK';
};

# ---------------------------------------------------------------------------
# 4. Bullseye pattern — full symbol
# ---------------------------------------------------------------------------

subtest 'bullseye pattern - full symbol' => sub {
    my $g  = encode( 'x' x 100 );
    my $cx = int( $g->{cols} / 2 );
    my $cy = int( $g->{rows} / 2 );

    is dark( $g, $cy, $cx ), 1, 'center DARK';

    is dark( $g, $cy - 2, $cx ),     0, 'd=2 N LIGHT (full)';
    is dark( $g, $cy + 2, $cx ),     0, 'd=2 S LIGHT (full)';
    is dark( $g, $cy,     $cx - 2 ), 0, 'd=2 W LIGHT (full)';
    is dark( $g, $cy,     $cx + 2 ), 0, 'd=2 E LIGHT (full)';

    is dark( $g, $cy - 7, $cx ),     1, 'd=7 N DARK (outer ring full)';
    is dark( $g, $cy + 7, $cx ),     1, 'd=7 S DARK (outer ring full)';
    is dark( $g, $cy,     $cx - 7 ), 1, 'd=7 W DARK (outer ring full)';
    is dark( $g, $cy,     $cx + 7 ), 1, 'd=7 E DARK (outer ring full)';
};

# ---------------------------------------------------------------------------
# 5. Orientation marks — compact
# ---------------------------------------------------------------------------

subtest 'orientation marks - compact 15x15' => sub {
    my $g  = encode("A");
    my $cx = 7;
    my $cy = 7;
    my $r  = 6;    # bullseye_radius(compact) + 1 = 5+1

    is dark( $g, $cy - $r, $cx - $r ), 1, 'TL corner DARK';
    is dark( $g, $cy - $r, $cx + $r ), 1, 'TR corner DARK';
    is dark( $g, $cy + $r, $cx + $r ), 1, 'BR corner DARK';
    is dark( $g, $cy + $r, $cx - $r ), 1, 'BL corner DARK';
};

# ---------------------------------------------------------------------------
# 6. Orientation marks — full
# ---------------------------------------------------------------------------

subtest 'orientation marks - full symbol' => sub {
    my $g  = encode( 'x' x 100 );
    my $cx = int( $g->{cols} / 2 );
    my $cy = int( $g->{rows} / 2 );
    my $r  = 8;    # bullseye_radius(full)+1 = 7+1

    is dark( $g, $cy - $r, $cx - $r ), 1, 'TL corner DARK (full)';
    is dark( $g, $cy - $r, $cx + $r ), 1, 'TR corner DARK (full)';
    is dark( $g, $cy + $r, $cx + $r ), 1, 'BR corner DARK (full)';
    is dark( $g, $cy + $r, $cx - $r ), 1, 'BL corner DARK (full)';
};

# ---------------------------------------------------------------------------
# 7. Grid structural properties
# ---------------------------------------------------------------------------

subtest 'grid structure' => sub {
    my $g = encode("A");
    is scalar @{ $g->{modules} },     15, 'modules has 15 rows';
    is scalar @{ $g->{modules}[0] }, 15, 'first row has 15 cols';
    is $g->{module_shape}, 'square', 'module_shape = square';

    # rows / cols / modules consistency for a few sizes.
    for my $input ( "Hello, World!", "test", "A" x 32, "B" x 50 ) {
        my $gr = encode($input);
        is $gr->{rows}, $gr->{cols}, "square for length " . length($input);
        is $gr->{rows}, scalar @{ $gr->{modules} },
            "rows matches modules count for length " . length($input);
        is $gr->{cols}, scalar @{ $gr->{modules}[0] },
            "cols matches first row length for length " . length($input);
    }
};

# ---------------------------------------------------------------------------
# 8. Byte-array equivalence
# ---------------------------------------------------------------------------

subtest 'byte-array input equivalence' => sub {
    my $str   = "Hello";
    # Pack the same bytes — pack 'C*' is the round-trip of unpack 'C*'.
    my $bytes = pack 'C*', unpack 'C*', $str;
    my $g1    = encode($str);
    my $g2    = encode($bytes);
    is $g1->{rows}, $g2->{rows}, 'rows equal';
    is $g1->{cols}, $g2->{cols}, 'cols equal';
    is grid_string($g1), grid_string($g2), 'identical grids';
};

# ---------------------------------------------------------------------------
# 9. min_ecc_percent option
# ---------------------------------------------------------------------------

subtest 'min_ecc_percent option' => sub {
    my $low  = encode( "Hello", { min_ecc_percent => 10 } );
    my $high = encode( "Hello", { min_ecc_percent => 80 } );
    cmp_ok $high->{rows}, '>=', $low->{rows},
        'higher ECC requires larger or equal symbol';

    my $mid = encode( "Hello", { min_ecc_percent => 33 } );
    cmp_ok $mid->{rows}, '>=', 15, 'min_ecc_percent 33 produces a valid grid';

    # Default ECC (23%) is between 10 and 80.
    my $def = encode("Hello");
    cmp_ok $def->{rows}, '>=', $low->{rows},
        'default >= low-ECC symbol';
    cmp_ok $def->{rows}, '<=', $high->{rows},
        'default <= high-ECC symbol';
};

# ---------------------------------------------------------------------------
# 10. Determinism
# ---------------------------------------------------------------------------

subtest 'determinism' => sub {
    for my $input ( "A", "Hello, World!", "12345", "x" x 50, "" ) {
        my $g1 = encode($input);
        my $g2 = encode($input);
        is grid_string($g1), grid_string($g2),
            "deterministic for length " . length($input);
    }
};

# ---------------------------------------------------------------------------
# 11. Different inputs produce different grids
# ---------------------------------------------------------------------------

subtest 'different inputs produce different grids' => sub {
    my $g1 = encode("Hello");
    my $g2 = encode("World");
    isnt grid_string($g1), grid_string($g2),
        'distinct inputs -> distinct grids';

    # Single-byte change should still differ.
    my $g3 = encode("A");
    my $g4 = encode("B");
    isnt grid_string($g3), grid_string($g4),
        "single-byte change -> different grids";
};

# ---------------------------------------------------------------------------
# 12. Empty string
# ---------------------------------------------------------------------------

subtest 'empty string encodes to a small symbol' => sub {
    my $g = encode("");
    cmp_ok $g->{rows}, '>=', 15, 'empty string fits in >= 15x15';
    is $g->{rows}, $g->{cols}, 'empty string symbol is square';
};

# ---------------------------------------------------------------------------
# 13. Larger inputs do not throw
# ---------------------------------------------------------------------------

subtest 'larger inputs encode without error' => sub {
    for my $len ( 32, 50, 200, 500 ) {
        my $g;
        my $ok = eval { $g = encode( 'C' x $len ); 1; };
        ok $ok, "encode of $len bytes did not throw"
            or diag $@;
        cmp_ok $g->{rows}, '>=', 15, "$len-byte symbol >= 15";
        is $g->{rows}, $g->{cols}, "$len-byte symbol is square";
    }
};

# ---------------------------------------------------------------------------
# 14. All-bytes input
# ---------------------------------------------------------------------------

subtest 'all-bytes 0x00..0xFF input' => sub {
    my $bytes = pack 'C*', 0 .. 255;
    my $g     = encode($bytes);
    cmp_ok $g->{rows}, '>=', 19, 'binary corpus uses full symbol';
    is $g->{rows}, $g->{cols}, 'binary corpus is square';
};

# ---------------------------------------------------------------------------
# 15. UTF-8 input (callers pre-encode)
# ---------------------------------------------------------------------------

subtest 'utf-8 bytes encode' => sub {
    # Manually construct UTF-8 bytes for Japanese "konnichiwa". Avoids any
    # Encode::encode dependency.
    my $bytes = pack 'C*', 0xe3, 0x81, 0x93, 0xe3, 0x82, 0x93, 0xe3, 0x81,
        0xab, 0xe3, 0x81, 0xa1, 0xe3, 0x81, 0xaf;
    my $g = encode($bytes);
    cmp_ok $g->{rows}, '>=', 15, 'utf-8 corpus encodes';
    is $g->{rows}, $g->{cols}, 'utf-8 corpus is square';
};

# ---------------------------------------------------------------------------
# 16. Modules are integers 0/1 only
# ---------------------------------------------------------------------------

subtest 'modules are 0 or 1' => sub {
    my $g       = encode("Test");
    my $bad_cnt = 0;
    for my $row ( @{ $g->{modules} } ) {
        for my $cell (@$row) {
            $bad_cnt++ unless $cell == 0 || $cell == 1;
        }
    }
    is $bad_cnt, 0, 'every module is exactly 0 or 1';
};

# ---------------------------------------------------------------------------
# 17. Modules are mutable (return value is independent of internal state)
# ---------------------------------------------------------------------------

subtest 'mutating result does not affect future encodes' => sub {
    my $g        = encode("A");
    my $original = $g->{modules}[0][0];
    $g->{modules}[0][0] = $original ? 0 : 1;
    my $g2 = encode("A");
    is $g2->{modules}[0][0], $original,
        'second encode unaffected by mutation of first result';
};

# ---------------------------------------------------------------------------
# 18. Cross-language corpus (size invariants)
# ---------------------------------------------------------------------------

subtest 'cross-language corpus' => sub {
    my @cases = (
        [ 'A',                                   15 ],
        [ 'Hello',                               19 ],
        [ '12345678901234567890',                23 ],
        [ '12345678901234567890' x 2,            27 ],
    );
    for my $case (@cases) {
        my ( $input, $expected ) = @$case;
        my $g = encode($input);
        is $g->{rows}, $expected,
            "corpus '${\(substr($input, 0, 20))}' -> ${expected}x${expected}";
    }
};

# ---------------------------------------------------------------------------
# 19. InputTooLong error path
# ---------------------------------------------------------------------------

subtest 'input too long throws' => sub {
    # 'x' x 2000 exceeds even 32-layer full symbol capacity (1437 bytes max).
    my $threw = 0;
    my $msg   = '';
    eval {
        encode( 'x' x 2000 );
        1;
    } or do { $threw = 1; $msg = $@ };
    ok $threw, 'oversized input throws';
    like $msg, qr/InputTooLong/, 'error message mentions InputTooLong';
};

# ---------------------------------------------------------------------------
# 20. min_ecc_percent default behavior — 23% matches default
# ---------------------------------------------------------------------------

subtest 'default min_ecc_percent equals 23' => sub {
    my $g_default = encode("Hello");
    my $g_23      = encode( "Hello", { min_ecc_percent => 23 } );
    is grid_string($g_default), grid_string($g_23),
        'no options matches min_ecc_percent => 23';
};

# ---------------------------------------------------------------------------
# 21. Small-input bullseye stable across two characters
# ---------------------------------------------------------------------------

subtest 'bullseye unaffected by data' => sub {
    # Bullseye and orientation ring are RESERVED — they should not change
    # as the data changes.
    my $g1 = encode("A");
    my $g2 = encode("Z");
    my $cx = 7;
    my $cy = 7;
    for my $dr ( -5 .. 5 ) {
        for my $dc ( -5 .. 5 ) {
            is dark( $g1, $cy + $dr, $cx + $dc ),
                dark( $g2, $cy + $dr, $cx + $dc ),
                "bullseye ($dr,$dc) identical across data";
        }
    }
};

# ---------------------------------------------------------------------------
# 22. Mode message reservation — orientation corners stable across symbol sizes
# ---------------------------------------------------------------------------

subtest 'orientation marks for compact-2 (19x19)' => sub {
    my $g  = encode("Hello");    # compact-2
    my $cx = int( $g->{cols} / 2 );
    my $cy = int( $g->{rows} / 2 );
    my $r  = 6;                  # compact bullseye radius+1
    is dark( $g, $cy - $r, $cx - $r ), 1, 'compact-2 TL corner DARK';
    is dark( $g, $cy - $r, $cx + $r ), 1, 'compact-2 TR corner DARK';
    is dark( $g, $cy + $r, $cx + $r ), 1, 'compact-2 BR corner DARK';
    is dark( $g, $cy + $r, $cx - $r ), 1, 'compact-2 BL corner DARK';
};

done_testing;
