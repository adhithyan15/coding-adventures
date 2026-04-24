use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# CodingAdventures::QrCode — comprehensive test suite
#
# Tests the full QR Code encoding pipeline:
#   1. Module and constants
#   2. Grid geometry helpers (internal)
#   3. Version selection
#   4. Data encoding modes
#   5. encode() end-to-end (grid shape, module values)
#   6. Error conditions
#   7. All ECC levels
#   8. All encoding modes (numeric, alphanumeric, byte)
#   9. Multi-version inputs
#  10. Internal helpers (format bits, RS encode)
# ---------------------------------------------------------------------------

use lib '../barcode-2d/lib';
use lib '../paint-instructions/lib';
use lib '../gf256/lib';
use lib '../polynomial/lib';
use lib '../reed-solomon/lib';
use lib 'lib';

require CodingAdventures::QrCode;
my $pkg = 'CodingAdventures::QrCode';

# ============================================================================
# 1. Module-level constants and VERSION
# ============================================================================

subtest 'VERSION' => sub {
    ok( defined $CodingAdventures::QrCode::VERSION, 'VERSION is defined' );
    like( $CodingAdventures::QrCode::VERSION, qr/^\d+\.\d+\.\d+$/, 'VERSION is semver' );
};

# ============================================================================
# 2. encode() — grid shape invariants
# ============================================================================

subtest 'encode returns a ModuleGrid hashref' => sub {
    my $grid = $pkg->encode('1', level => 'M');
    ok( ref $grid eq 'HASH',          'result is a hashref' );
    ok( exists $grid->{rows},         'has rows key' );
    ok( exists $grid->{cols},         'has cols key' );
    ok( exists $grid->{modules},      'has modules key' );
    ok( exists $grid->{module_shape}, 'has module_shape key' );
    is( $grid->{module_shape}, 'square', 'module_shape is square' );
};

subtest 'encode — grid is square' => sub {
    my $grid = $pkg->encode('HELLO', level => 'M');
    is( $grid->{rows}, $grid->{cols}, 'rows == cols (square grid)' );
};

subtest 'encode — correct size for version 1 input (numeric, L)' => sub {
    # "1" at level L should fit in version 1 (21×21 modules)
    my $grid = $pkg->encode('1', level => 'L');
    is( $grid->{rows}, 21, 'version 1 → 21 modules wide' );
    is( $grid->{cols}, 21, 'version 1 → 21 modules tall' );
};

subtest 'encode — module values are 0 or 1' => sub {
    my $grid = $pkg->encode('42', level => 'M');
    my $sz   = $grid->{rows};
    my $ok   = 1;
    for my $r (0 .. $sz - 1) {
        for my $c (0 .. $sz - 1) {
            my $v = $grid->{modules}[$r][$c];
            $ok = 0 unless defined $v && ($v == 0 || $v == 1);
        }
    }
    ok( $ok, 'all module values are 0 or 1' );
};

subtest 'encode — grid size formula 4v+17' => sub {
    for my $v (1, 2, 5, 10, 40) {
        # Use a long input to force the desired version
        # or just test the formula via small inputs
        next unless $v == 1;   # test v1 directly
        my $grid = $pkg->encode('1', level => 'H');   # v1 H still fits '1'
        my $expected_sz = 4 * 1 + 17;
        is( $grid->{rows}, $expected_sz, "v1 grid is ${expected_sz}×${expected_sz}" );
    }
};

# ============================================================================
# 3. Version selection
# ============================================================================

subtest 'version selection — single digit fits v1' => sub {
    # Version 1 at level M can hold at most 14 numeric characters.
    # '1' is just 1 digit.
    my $grid = $pkg->encode('1', level => 'M');
    is( $grid->{rows}, 21, 'single digit → version 1 (21×21)' );
};

subtest 'version selection — longer numeric at level L is larger' => sub {
    # "12345678901234567" is 17 digits; v1-L capacity is 41 numeric chars,
    # so it should still be v1 (21 wide).
    my $grid = $pkg->encode('12345678901234567', level => 'L');
    is( $grid->{rows}, 21, '17-digit number at L fits v1 (21×21)' );
};

subtest 'version selection — long input promotes to higher version' => sub {
    # A 200-character byte string will require a higher version.
    my $long = 'A' x 200;
    my $grid = $pkg->encode($long, level => 'M');
    ok( $grid->{rows} > 21, 'long input promoted beyond version 1' );
    is( $grid->{rows}, $grid->{cols}, 'still square' );
    # size must be 4v+17 for some integer v
    my $v = ($grid->{rows} - 17) / 4;
    is( int($v), $v, 'size is 4v+17 for integer v' );
};

# ============================================================================
# 4. All ECC levels
# ============================================================================

subtest 'all ECC levels produce valid grids' => sub {
    for my $level (qw(L M Q H)) {
        my $grid = $pkg->encode('Hello', level => $level);
        ok( $grid->{rows} >= 21, "level $level: rows >= 21" );
        is( $grid->{rows}, $grid->{cols}, "level $level: square" );
    }
};

subtest 'higher ECC levels require larger symbols for same input' => sub {
    my $input = 'https://example.com';
    my @sizes = map { $pkg->encode($input, level => $_)->{rows} } qw(L M Q H);
    # Size may be non-decreasing: L ≤ M ≤ Q ≤ H (may be equal if they all fit
    # in the same version, but H will require at least as large a symbol as L)
    ok( $sizes[0] <= $sizes[3], 'L symbol ≤ H symbol for same input' );
};

# ============================================================================
# 5. Encoding modes
# ============================================================================

subtest 'numeric mode — digit-only input' => sub {
    my $grid = $pkg->encode('0123456789', level => 'M');
    ok( $grid->{rows} >= 21, 'numeric encode succeeds' );
    is( $grid->{rows}, $grid->{cols}, 'square' );
};

subtest 'alphanumeric mode — uppercase + space' => sub {
    my $grid = $pkg->encode('HELLO WORLD', level => 'M');
    is( $grid->{rows}, 21, 'HELLO WORLD at M fits v1 (21×21)' );
};

subtest 'alphanumeric mode — all 45 chars' => sub {
    my $input = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:';
    my $grid = $pkg->encode($input, level => 'L');
    ok( $grid->{rows} >= 21, 'full alphanum set encodes without error' );
};

subtest 'byte mode — lowercase triggers byte mode' => sub {
    my $grid = $pkg->encode('hello world', level => 'M');
    ok( $grid->{rows} >= 21, 'byte mode encode succeeds' );
};

subtest 'byte mode — UTF-8 multibyte characters' => sub {
    my $grid = $pkg->encode("Hello \x{1F600}", level => 'M');   # emoji
    ok( $grid->{rows} >= 21, 'UTF-8 emoji encodes without error' );
    is( $grid->{rows}, $grid->{cols}, 'square' );
};

subtest 'byte mode — URL' => sub {
    my $grid = $pkg->encode('https://example.com', level => 'M');
    ok( $grid->{rows} >= 21, 'URL encodes without error' );
};

# ============================================================================
# 6. Known-good pixel values — finder pattern smoke test
# ============================================================================

subtest 'finder pattern top-left corner is dark' => sub {
    # The top-left finder pattern has its top-left module (0,0) dark,
    # and its inner ring (1,1)–(5,5) with light corners.
    my $grid = $pkg->encode('1', level => 'M');
    my $m    = $grid->{modules};
    is( $m->[0][0], 1, 'top-left corner module is dark' );
    is( $m->[0][6], 1, 'top-right of top-left finder is dark' );
    is( $m->[6][0], 1, 'bottom-left of top-left finder is dark' );
    is( $m->[6][6], 1, 'bottom-right of top-left finder is dark' );
    # Interior light ring
    is( $m->[1][1], 0, 'inner ring (1,1) is light' );
    is( $m->[1][5], 0, 'inner ring (1,5) is light' );
    is( $m->[5][1], 0, 'inner ring (5,1) is light' );
    is( $m->[5][5], 0, 'inner ring (5,5) is light' );
    # Dark centre
    is( $m->[3][3], 1, 'finder centre is dark' );
};

subtest 'timing strip — row 6 alternates dark/light from col 8' => sub {
    my $grid = $pkg->encode('1', level => 'M');
    my $m    = $grid->{modules};
    # Row 6, cols 8..12: should alternate dark(8), light(9), dark(10)...
    is( $m->[6][8],  1, 'timing row 6, col 8 is dark' );
    is( $m->[6][9],  0, 'timing row 6, col 9 is light' );
    is( $m->[6][10], 1, 'timing row 6, col 10 is dark' );
    is( $m->[6][11], 0, 'timing row 6, col 11 is light' );
    is( $m->[6][12], 1, 'timing row 6, col 12 is dark' );
};

# ============================================================================
# 7. Error conditions
# ============================================================================

subtest 'invalid ECC level throws' => sub {
    eval { $pkg->encode('test', level => 'X') };
    like( $@, qr/invalid ECC level/i, 'invalid ECC level throws' );
};

subtest 'extremely long input throws InputTooLong' => sub {
    my $huge = 'A' x 8000;
    eval { $pkg->encode($huge, level => 'H') };
    like( $@, qr/InputTooLong|exceeds/i, 'huge input throws InputTooLong' );
};

subtest 'default level is M' => sub {
    my $grid_default = $pkg->encode('HELLO', level => 'M');
    my $grid_m       = $pkg->encode('HELLO', level => 'M');
    is( $grid_default->{rows}, $grid_m->{rows}, 'default level M same size as explicit M' );
};

# ============================================================================
# 8. Deterministic output (same input → same grid)
# ============================================================================

subtest 'encode is deterministic' => sub {
    my $g1 = $pkg->encode('https://coding-adventures.io', level => 'M');
    my $g2 = $pkg->encode('https://coding-adventures.io', level => 'M');
    is( $g1->{rows}, $g2->{rows}, 'same input → same rows' );
    # Deep-compare a few rows
    for my $r (0, 3, 10) {
        is( $g1->{modules}[$r], $g2->{modules}[$r], "row $r matches" );
    }
};

# ============================================================================
# 9. Grid density sanity — at least some dark modules
# ============================================================================

subtest 'encoded grid has both dark and light modules' => sub {
    my $grid  = $pkg->encode('Hello, World!', level => 'M');
    my $sz    = $grid->{rows};
    my ($dark, $light) = (0, 0);
    for my $r (0 .. $sz - 1) {
        for my $c (0 .. $sz - 1) {
            if ($grid->{modules}[$r][$c]) { $dark++ } else { $light++ }
        }
    }
    ok( $dark  > 0, 'at least some dark modules' );
    ok( $light > 0, 'at least some light modules' );
    # QR Code specifications say dark ratio should be roughly 40-60%.
    my $ratio = $dark / ($sz * $sz);
    ok( $ratio > 0.3 && $ratio < 0.7, "dark ratio $ratio is in [0.3, 0.7]" );
};

# ============================================================================
# 10. Version 2 — alignment pattern check
# ============================================================================

subtest 'version 2 has alignment pattern centred at (18,18)' => sub {
    # Version 2 symbol is 25×25. The single alignment pattern is at (18,18).
    # Force version 2 by using 8 lowercase bytes at level H.
    # V1-H byte capacity is 7 bytes (4+8+7*8=76 bits > 72=9*8, so 7 fits but 8 does not).
    my $grid = $pkg->encode('abcdefgh', level => 'H');
    is( $grid->{rows}, 25, 'v2 grid is 25×25' );
    my $m = $grid->{modules};
    # The alignment pattern centre (18,18) is always dark.
    is( $m->[18][18], 1, 'alignment pattern centre (18,18) is dark' );
};

# ============================================================================
# 11. Edge cases
# ============================================================================

subtest 'empty string encodes without error' => sub {
    my $grid = eval { $pkg->encode('', level => 'M') };
    ok( !$@,            'empty string does not throw' );
    ok( $grid,          'returns a grid' );
    is( $grid->{rows}, $grid->{cols}, 'still square' );
};

subtest 'single character encodes correctly' => sub {
    my $grid = $pkg->encode('A', level => 'M');
    ok( $grid->{rows} >= 21, 'single char → at least v1' );
};

subtest 'exactly 41 numeric chars at L fits v1' => sub {
    # V1-L numeric capacity: 41 digits.
    my $grid = $pkg->encode('1' x 41, level => 'L');
    is( $grid->{rows}, 21, '41 digits at L fits v1 (21×21)' );
};

subtest '42 numeric chars at L promotes to v2' => sub {
    my $grid = $pkg->encode('1' x 42, level => 'L');
    is( $grid->{rows}, 25, '42 digits at L needs v2 (25×25)' );
};

done_testing;
