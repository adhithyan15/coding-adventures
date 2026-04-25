use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# CodingAdventures::MicroQR — comprehensive test suite
#
# This file covers:
#   1. Module constants and VERSION
#   2. Symbol dimensions (M1–M4)
#   3. Auto-version selection
#   4. Finder pattern structure
#   5. Separator modules
#   6. Timing pattern
#   7. Format information presence
#   8. ECC level constraints
#   9. Encoding modes
#  10. Capacity boundaries
#  11. Error conditions
#  12. Determinism
#  13. Cross-language corpus (symbol-size oracle)
# ---------------------------------------------------------------------------

use lib '../gf256/lib';
use lib '../barcode-2d/lib';
use lib '../paint-instructions/lib';
use lib 'lib';

require CodingAdventures::MicroQR;
my $pkg = 'CodingAdventures::MicroQR';

# Convenience: turn a ModuleGrid's modules into a flat string for comparison.
sub grid_str {
    my ($grid) = @_;
    return join('', map { join('', @$_) } @{ $grid->{modules} });
}

# ============================================================================
# 1. Module constants and VERSION
# ============================================================================

subtest 'VERSION is defined and semver' => sub {
    ok( defined $CodingAdventures::MicroQR::VERSION, 'VERSION is defined' );
    like(
        $CodingAdventures::MicroQR::VERSION,
        qr/^\d+\.\d+\.\d+$/,
        'VERSION matches semver format'
    );
};

subtest 'exported constants exist' => sub {
    is( CodingAdventures::MicroQR::M1(), 'M1', 'M1 constant' );
    is( CodingAdventures::MicroQR::M2(), 'M2', 'M2 constant' );
    is( CodingAdventures::MicroQR::M3(), 'M3', 'M3 constant' );
    is( CodingAdventures::MicroQR::M4(), 'M4', 'M4 constant' );
    is( CodingAdventures::MicroQR::DETECTION(), 'Detection', 'DETECTION constant' );
    is( CodingAdventures::MicroQR::ECC_L(),     'L',         'ECC_L constant' );
    is( CodingAdventures::MicroQR::ECC_M(),     'M',         'ECC_M constant' );
    is( CodingAdventures::MicroQR::ECC_Q(),     'Q',         'ECC_Q constant' );
};

# ============================================================================
# 2. Symbol dimensions — encode() returns the correct grid size
# ============================================================================

subtest 'M1 is 11×11' => sub {
    my $g = CodingAdventures::MicroQR::encode('1');
    is( $g->{rows}, 11, 'M1 rows == 11' );
    is( $g->{cols}, 11, 'M1 cols == 11' );
};

subtest 'M2 is 13×13 for HELLO' => sub {
    my $g = CodingAdventures::MicroQR::encode('HELLO');
    is( $g->{rows}, 13, 'M2 rows == 13' );
    is( $g->{cols}, 13, 'M2 cols == 13' );
};

subtest 'M3 is 15×15 for MICRO QR TEST' => sub {
    # 13 alphanumeric characters → M3-L (alpha_cap=14)
    my $g = CodingAdventures::MicroQR::encode('MICRO QR TEST');
    is( $g->{rows}, 15, 'M3 rows == 15' );
    is( $g->{cols}, 15, 'M3 cols == 15' );
};

subtest 'M4 is 17×17 for URL' => sub {
    my $g = CodingAdventures::MicroQR::encode('https://a.b');
    is( $g->{rows}, 17, 'M4 rows == 17' );
    is( $g->{cols}, 17, 'M4 cols == 17' );
};

subtest 'grid is always square' => sub {
    for my $input ('1', 'HELLO', 'hello', 'https://a.b', 'MICRO QR TEST') {
        my $g = CodingAdventures::MicroQR::encode($input);
        is( $g->{rows}, $g->{cols}, "'$input': rows == cols (square)" );
    }
};

subtest 'module_shape is always square' => sub {
    for my $input ('1', '12345', 'HELLO') {
        my $g = CodingAdventures::MicroQR::encode($input);
        is( $g->{module_shape}, 'square', "'$input': module_shape is 'square'" );
    }
};

subtest 'result is a hashref with required keys' => sub {
    my $g = CodingAdventures::MicroQR::encode('42');
    ok( ref $g eq 'HASH',          'result is a hashref' );
    ok( exists $g->{rows},         'has rows key' );
    ok( exists $g->{cols},         'has cols key' );
    ok( exists $g->{modules},      'has modules key' );
    ok( exists $g->{module_shape}, 'has module_shape key' );
};

subtest 'all module values are 0 or 1' => sub {
    for my $input ('1', 'HELLO', 'hello', 'https://a.b') {
        my $g  = CodingAdventures::MicroQR::encode($input);
        my $sz = $g->{rows};
        my $ok = 1;
        for my $r (0 .. $sz - 1) {
            for my $c (0 .. $sz - 1) {
                my $v = $g->{modules}[$r][$c];
                unless (defined $v && ($v == 0 || $v == 1)) {
                    $ok = 0;
                }
            }
        }
        ok( $ok, "'$input': all modules are 0 or 1" );
    }
};

# ============================================================================
# 3. Auto-version selection
# ============================================================================

subtest 'auto-selects M1 for single digit' => sub {
    is( CodingAdventures::MicroQR::encode('1')->{rows}, 11, 'single digit → M1 (11×11)' );
};

subtest 'auto-selects M1 for 12345' => sub {
    is( CodingAdventures::MicroQR::encode('12345')->{rows}, 11, '5 digits → M1 (11×11)' );
};

subtest 'auto-selects M2 for 6 digits (overflow M1)' => sub {
    is( CodingAdventures::MicroQR::encode('123456')->{rows}, 13, '6 digits → M2 (13×13)' );
};

subtest 'auto-selects M2 for HELLO (alphanumeric)' => sub {
    is( CodingAdventures::MicroQR::encode('HELLO')->{rows}, 13, 'HELLO → M2' );
};

subtest 'auto-selects M3+ for lowercase hello (byte mode)' => sub {
    # "hello" is 5 bytes → M3-L byte_cap=9 → M3
    my $rows = CodingAdventures::MicroQR::encode('hello')->{rows};
    ok( $rows >= 15, "'hello' byte mode → M3 or larger (got $rows)" );
};

subtest 'auto-selects M4 for URL (byte mode)' => sub {
    is( CodingAdventures::MicroQR::encode('https://a.b')->{rows}, 17, 'URL → M4 (17×17)' );
};

subtest 'forced version M4 works for short input' => sub {
    my $g = CodingAdventures::MicroQR::encode('1',
        CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_L());
    is( $g->{rows}, 17, 'forced M4 → 17×17' );
};

subtest 'forced ECC level M gives different grid from L for HELLO' => sub {
    my $gl = CodingAdventures::MicroQR::encode('HELLO',
        undef, CodingAdventures::MicroQR::ECC_L());
    my $gm = CodingAdventures::MicroQR::encode('HELLO',
        undef, CodingAdventures::MicroQR::ECC_M());
    isnt( grid_str($gl), grid_str($gm), 'ECC L and M produce different grids for HELLO' );
};

subtest 'auto-selects M2-L for 8-digit numeric' => sub {
    is( CodingAdventures::MicroQR::encode('01234567')->{rows}, 13, '8 digits → M2 (13×13)' );
};

# ============================================================================
# 4. Finder pattern structure
# ============================================================================
#
# The 7×7 finder at top-left must match exactly:
#   - Outer ring (border) always dark
#   - Inner ring (rows/cols 1 and 5 inside finder) always light
#   - 3×3 core (rows 2-4, cols 2-4) always dark

subtest 'finder pattern outer ring is all dark (M1)' => sub {
    my $m = CodingAdventures::MicroQR::encode('1')->{modules};
    for my $c (0..6) {
        ok( $m->[0][$c], "finder row 0 col $c is dark" );
        ok( $m->[6][$c], "finder row 6 col $c is dark" );
    }
    for my $r (1..5) {
        ok( $m->[$r][0], "finder col 0 row $r is dark" );
        ok( $m->[$r][6], "finder col 6 row $r is dark" );
    }
};

subtest 'finder pattern inner ring is all light (M1)' => sub {
    my $m = CodingAdventures::MicroQR::encode('1')->{modules};
    for my $c (1..5) {
        ok( !$m->[1][$c], "finder inner ring row 1 col $c is light" );
        ok( !$m->[5][$c], "finder inner ring row 5 col $c is light" );
    }
    for my $r (2..4) {
        ok( !$m->[$r][1], "finder inner ring col 1 row $r is light" );
        ok( !$m->[$r][5], "finder inner ring col 5 row $r is light" );
    }
};

subtest 'finder 3×3 core is all dark (M1)' => sub {
    my $m = CodingAdventures::MicroQR::encode('1')->{modules};
    for my $r (2..4) {
        for my $c (2..4) {
            ok( $m->[$r][$c], "finder core ($r,$c) is dark" );
        }
    }
};

# ============================================================================
# 5. Separator modules
# ============================================================================
#
# Row 7 (cols 0-7) and col 7 (rows 0-7) must all be light.

subtest 'separator row 7 is all light (M2+)' => sub {
    my $m = CodingAdventures::MicroQR::encode('HELLO')->{modules};
    for my $c (0..7) {
        ok( !$m->[7][$c], "separator row 7 col $c is light" );
    }
};

subtest 'separator col 7 is all light (M2+)' => sub {
    my $m = CodingAdventures::MicroQR::encode('HELLO')->{modules};
    for my $r (0..7) {
        ok( !$m->[$r][7], "separator col 7 row $r is light" );
    }
};

# ============================================================================
# 6. Timing pattern
# ============================================================================
#
# Row 0 (cols 8..size-1) and col 0 (rows 8..size-1) must alternate dark/light.
# Module at col/row 8 is dark (8 is even).

subtest 'timing row 0 alternates correctly (M4)' => sub {
    my $m = CodingAdventures::MicroQR::encode('https://a.b')->{modules};
    for my $c (8..16) {
        my $expected = ($c % 2 == 0) ? 1 : 0;
        is( $m->[0][$c], $expected, "timing row 0 col $c = $expected" );
    }
};

subtest 'timing col 0 alternates correctly (M4)' => sub {
    my $m = CodingAdventures::MicroQR::encode('https://a.b')->{modules};
    for my $r (8..16) {
        my $expected = ($r % 2 == 0) ? 1 : 0;
        is( $m->[$r][0], $expected, "timing col 0 row $r = $expected" );
    }
};

subtest 'timing row 0 alternates correctly (M1, 11×11)' => sub {
    my $m = CodingAdventures::MicroQR::encode('1')->{modules};
    for my $c (8..10) {
        my $expected = ($c % 2 == 0) ? 1 : 0;
        is( $m->[0][$c], $expected, "timing M1 row 0 col $c = $expected" );
    }
};

# ============================================================================
# 7. Format information — there should be some dark modules in the format area
# ============================================================================
#
# Row 8 cols 1-8 and col 8 rows 1-7 hold the format information.
# The XOR mask 0x4445 ensures the format area is never all-zero.

subtest 'format info is not all-zero (M4-L)' => sub {
    my $m = CodingAdventures::MicroQR::encode('HELLO',
        CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_L())->{modules};
    my $dark_row = grep { $m->[8][$_] } 1..8;
    my $dark_col = grep { $m->[$_][8] } 1..7;
    ok( $dark_row + $dark_col > 0,
        'format info area has at least one dark module (M4-L)' );
};

subtest 'format info is not all-zero (M1)' => sub {
    my $m = CodingAdventures::MicroQR::encode('1')->{modules};
    my $dark_row = grep { $m->[8][$_] } 1..8;
    my $dark_col = grep { $m->[$_][8] } 1..7;
    ok( $dark_row + $dark_col > 0,
        'format info area has at least one dark module (M1)' );
};

subtest 'format info differs between ECC levels' => sub {
    my $gl = CodingAdventures::MicroQR::encode('HELLO',
        CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_L());
    my $gm = CodingAdventures::MicroQR::encode('HELLO',
        CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_M());
    my $gq = CodingAdventures::MicroQR::encode('HELLO',
        CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_Q());
    isnt( grid_str($gl), grid_str($gm), 'M4-L and M4-M grids differ' );
    isnt( grid_str($gm), grid_str($gq), 'M4-M and M4-Q grids differ' );
    isnt( grid_str($gl), grid_str($gq), 'M4-L and M4-Q grids differ' );
};

# ============================================================================
# 8. ECC level constraints
# ============================================================================

subtest 'M1 accepts Detection ECC' => sub {
    my $g = CodingAdventures::MicroQR::encode('1',
        CodingAdventures::MicroQR::M1(),
        CodingAdventures::MicroQR::DETECTION());
    is( $g->{rows}, 11, 'M1/Detection → 11×11' );
};

subtest 'M4-Q encodes HELLO' => sub {
    my $g = CodingAdventures::MicroQR::encode('HELLO',
        CodingAdventures::MicroQR::M4(),
        CodingAdventures::MicroQR::ECC_Q());
    is( $g->{rows}, 17, 'M4-Q → 17×17' );
};

subtest 'M1 rejects ECC L (dies)' => sub {
    ok(
        dies {
            CodingAdventures::MicroQR::encode('1',
                CodingAdventures::MicroQR::M1(),
                CodingAdventures::MicroQR::ECC_L())
        },
        'M1/L combination dies'
    );
};

subtest 'M2 rejects ECC Q (dies)' => sub {
    ok(
        dies {
            CodingAdventures::MicroQR::encode('1',
                CodingAdventures::MicroQR::M2(),
                CodingAdventures::MicroQR::ECC_Q())
        },
        'M2/Q combination dies'
    );
};

subtest 'M3 rejects ECC Q (dies)' => sub {
    ok(
        dies {
            CodingAdventures::MicroQR::encode('1',
                CodingAdventures::MicroQR::M3(),
                CodingAdventures::MicroQR::ECC_Q())
        },
        'M3/Q combination dies'
    );
};

subtest 'M1 rejects ECC Q (dies)' => sub {
    ok(
        dies {
            CodingAdventures::MicroQR::encode('1',
                CodingAdventures::MicroQR::M1(),
                CodingAdventures::MicroQR::ECC_Q())
        },
        'M1/Q combination dies'
    );
};

# ============================================================================
# 9. Encoding modes
# ============================================================================

subtest 'numeric mode: single digit encodes in M1' => sub {
    my $g = CodingAdventures::MicroQR::encode('5');
    is( $g->{rows}, 11, 'single digit → M1' );
};

subtest 'alphanumeric mode: A1B2C3 encodes in M2' => sub {
    # 6 alphanumeric chars → M2-L (alpha_cap=6)
    my $g = CodingAdventures::MicroQR::encode('A1B2C3');
    is( $g->{rows}, 13, 'A1B2C3 (alpha, 6 chars) → M2 (13×13)' );
};

subtest 'byte mode: lowercase hello goes to M3+' => sub {
    # "hello" = 5 bytes; M2-L byte_cap=4 → M3-L byte_cap=9
    my $g = CodingAdventures::MicroQR::encode('hello');
    ok( $g->{rows} >= 15, "'hello' → M3 or larger" );
};

subtest 'byte mode: URL goes to M4' => sub {
    my $g = CodingAdventures::MicroQR::encode('https://a.b');
    is( $g->{rows}, 17, 'URL → M4 (byte mode)' );
};

subtest 'empty string encodes to M1 (numeric mode)' => sub {
    my $g = CodingAdventures::MicroQR::encode('');
    is( $g->{rows}, 11, 'empty string → M1' );
};

# ============================================================================
# 10. Capacity boundaries
# ============================================================================

subtest 'M1 maximum: 5 digits fit' => sub {
    my $g = CodingAdventures::MicroQR::encode('12345');
    is( $g->{rows}, 11, '5 digits → M1 (exactly at capacity)' );
};

subtest 'M1 overflow: 6 digits bump to M2' => sub {
    my $g = CodingAdventures::MicroQR::encode('123456');
    is( $g->{rows}, 13, '6 digits → M2 (M1 overflow)' );
};

subtest 'M4-L maximum: 35 digits fit' => sub {
    my $g = CodingAdventures::MicroQR::encode('1' x 35);
    is( $g->{rows}, 17, '35 digits → M4-L' );
};

subtest 'M4-Q maximum: 21 digits fit' => sub {
    my $g = CodingAdventures::MicroQR::encode('1' x 21,
        undef, CodingAdventures::MicroQR::ECC_Q());
    is( $g->{rows}, 17, '21 digits with ECC_Q → M4-Q' );
};

subtest 'M4-L maximum byte mode: 15 bytes fit' => sub {
    my $g = CodingAdventures::MicroQR::encode('a' x 15);
    is( $g->{rows}, 17, '15 lowercase → M4-L byte mode' );
};

# ============================================================================
# 11. Error conditions
# ============================================================================

subtest 'input too long (36+ digits) dies' => sub {
    ok( dies { CodingAdventures::MicroQR::encode('1' x 36) },
        '36 digits dies (exceeds M4 capacity)' );
};

subtest 'nonexistent version/ECC combo dies' => sub {
    ok(
        dies {
            CodingAdventures::MicroQR::encode('1',
                CodingAdventures::MicroQR::M1(),
                CodingAdventures::MicroQR::ECC_L())
        },
        'M1/L combination dies with descriptive error'
    );
};

subtest 'encode_at requires version' => sub {
    ok( dies { CodingAdventures::MicroQR::encode_at('1', undef, 'L') },
        'encode_at without version dies' );
};

subtest 'encode_at requires ecc' => sub {
    ok( dies { CodingAdventures::MicroQR::encode_at('1', 'M1', undef) },
        'encode_at without ecc dies' );
};

# ============================================================================
# 12. Determinism
# ============================================================================

subtest 'encoding is deterministic' => sub {
    for my $input ('1', '12345', 'HELLO', 'A1B2C3', 'hello', 'https://a.b') {
        my $g1 = CodingAdventures::MicroQR::encode($input);
        my $g2 = CodingAdventures::MicroQR::encode($input);
        is( grid_str($g1), grid_str($g2),
            "deterministic encoding for '$input'" );
    }
};

subtest 'different inputs produce different grids' => sub {
    my $g1 = CodingAdventures::MicroQR::encode('1');
    my $g2 = CodingAdventures::MicroQR::encode('2');
    isnt( grid_str($g1), grid_str($g2), "'1' and '2' produce different grids" );
};

subtest 'different ECC levels produce different grids' => sub {
    my $gl = CodingAdventures::MicroQR::encode('HELLO',
        undef, CodingAdventures::MicroQR::ECC_L());
    my $gm = CodingAdventures::MicroQR::encode('HELLO',
        undef, CodingAdventures::MicroQR::ECC_M());
    isnt( grid_str($gl), grid_str($gm),
        'M2-L and M2-M grids differ for HELLO' );
};

# ============================================================================
# 13. Cross-language corpus
# ============================================================================
#
# These expected sizes are verified against the Rust, Ruby, Elixir, Swift,
# Perl, and Lua implementations. Changing them breaks cross-language parity.

subtest 'cross-language size corpus' => sub {
    my @cases = (
        ['1',              11, 'M1: single digit'],
        ['12345',          11, 'M1 max: 5 digits'],
        ['HELLO',          13, 'M2-L: alphanumeric'],
        ['01234567',       13, 'M2-L: 8 numeric'],
        ['https://a.b',    17, 'M4: byte mode URL'],
        ['MICRO QR TEST',  15, 'M3-L: 13 alphanumeric'],
    );
    for my $case (@cases) {
        my ($input, $expected_size, $label) = @$case;
        my $g = CodingAdventures::MicroQR::encode($input);
        is( $g->{rows}, $expected_size, "$label → ${expected_size}×${expected_size}" );
    }
};

# ============================================================================
# 14. encode_at — explicit version + ECC
# ============================================================================

subtest 'encode_at M1/Detection works for short numeric' => sub {
    my $g = CodingAdventures::MicroQR::encode_at('1',
        CodingAdventures::MicroQR::M1(),
        CodingAdventures::MicroQR::DETECTION());
    is( $g->{rows}, 11, 'encode_at M1/Detection → 11×11' );
};

subtest 'encode_at M4/Q works for short string' => sub {
    my $g = CodingAdventures::MicroQR::encode_at('HELLO',
        CodingAdventures::MicroQR::M4(),
        CodingAdventures::MicroQR::ECC_Q());
    is( $g->{rows}, 17, 'encode_at M4/Q → 17×17' );
};

subtest 'encode_at matches encode with explicit args' => sub {
    my $g1 = CodingAdventures::MicroQR::encode_at('HELLO',
        CodingAdventures::MicroQR::M2(),
        CodingAdventures::MicroQR::ECC_L());
    my $g2 = CodingAdventures::MicroQR::encode('HELLO',
        CodingAdventures::MicroQR::M2(),
        CodingAdventures::MicroQR::ECC_L());
    is( grid_str($g1), grid_str($g2),
        'encode_at and encode produce identical grids for M2-L HELLO' );
};

# ============================================================================
# 15. Grid completeness — modules array is fully populated
# ============================================================================

subtest 'modules array has correct dimensions' => sub {
    for my $input ('1', 'HELLO', 'hello', 'https://a.b') {
        my $g  = CodingAdventures::MicroQR::encode($input);
        my $sz = $g->{rows};
        is( scalar @{ $g->{modules} }, $sz, "'$input': $sz rows in modules" );
        for my $r (0 .. $sz - 1) {
            is( scalar @{ $g->{modules}[$r] }, $sz, "'$input': row $r has $sz cols" );
        }
    }
};

done_testing;
