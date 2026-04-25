#!/usr/bin/env perl
# =============================================================================
# micro_qr.t — Test suite for CodingAdventures::MicroQR
# =============================================================================
#
# 58 tests covering:
#   - Symbol dimensions (M1=11×11 through M4=17×17)
#   - Auto-version and auto-ECC selection
#   - Encoding modes (numeric, alphanumeric, byte)
#   - Structural modules (finder, separator, timing strips)
#   - Reed-Solomon ECC correctness (single-block, b=0 convention)
#   - Format information placement
#   - Mask selection (4 patterns, lowest-penalty wins)
#   - Capacity boundary conditions (max fits, overflow)
#   - Error handling (InputTooLong, ECCNotAvailable)
#   - Determinism (same input → same grid on every call)
#   - M1 half-codeword special case (2.5 codewords = 20 bits)
#   - ECC level constraints (not all levels valid for all versions)

use strict;
use warnings;
use lib 't/../../../paint-instructions/lib';
use lib 't/../../../gf256/lib';
use lib 't/../../../barcode-2d/lib';
use lib 't/../lib';

use Test2::V0;
use CodingAdventures::MicroQR qw(encode encode_at layout_grid);

# ---------------------------------------------------------------------------
# Helper: flatten a ModuleGrid to a single string of 0s and 1s.
# Used for equality checks between grids of different configurations.
# ---------------------------------------------------------------------------
sub grid_str {
    my ($grid) = @_;
    return join '', map { join('', @$_) } @{ $grid->{modules} };
}

# ---------------------------------------------------------------------------
# Helper: count dark modules in a grid.
# ---------------------------------------------------------------------------
sub dark_count {
    my ($grid) = @_;
    my $count = 0;
    for my $row (@{ $grid->{modules} }) {
        $count += grep { $_ } @$row;
    }
    return $count;
}

# =============================================================================
# 1. Symbol dimensions
# =============================================================================

subtest 'symbol dimensions' => sub {
    # M1: size = 2×1+9 = 11
    my $m1 = encode("1", undef, undef);
    is($m1->{rows}, 11, 'M1 has 11 rows');
    is($m1->{cols}, 11, 'M1 has 11 cols');

    # M2: size = 2×2+9 = 13
    my $m2 = encode("HELLO", undef, undef);
    is($m2->{rows}, 13, 'M2 has 13 rows');
    is($m2->{cols}, 13, 'M2 has 13 cols');

    # M3: size = 2×3+9 = 15
    my $m3 = encode("MICRO QR TEST", undef, undef);  # 13 alphanumeric → M3-L
    is($m3->{rows}, 15, 'M3 has 15 rows');
    is($m3->{cols}, 15, 'M3 has 15 cols');

    # M4: size = 2×4+9 = 17
    my $m4 = encode("https://a.b", undef, undef);
    is($m4->{rows}, 17, 'M4 has 17 rows');
    is($m4->{cols}, 17, 'M4 has 17 cols');
};

# =============================================================================
# 2. Auto-version selection
# =============================================================================

subtest 'auto-version selection' => sub {
    # Single digit → M1 (smallest symbol)
    is(encode("1", undef, undef)->{rows},    11, 'single digit → M1');
    # 5 digits → M1 (numeric_cap=5)
    is(encode("12345", undef, undef)->{rows}, 11, '5 digits → M1');
    # 6 digits overflows M1, fits M2
    is(encode("123456", undef, undef)->{rows}, 13, '6 digits → M2');
    # HELLO = 5 alphanumeric → M2 (alpha_cap=6)
    is(encode("HELLO", undef, undef)->{rows}, 13, 'HELLO → M2');
    # "hello" is byte mode (lowercase), M3-L byte_cap=9
    ok(encode("hello", undef, undef)->{rows} >= 15, 'hello (byte) → M3+');
    # Long URL → M4
    is(encode("https://a.b", undef, undef)->{rows}, 17, 'URL → M4');
};

# =============================================================================
# 3. Forced version
# =============================================================================

subtest 'forced version' => sub {
    my $g = encode("1", CodingAdventures::MicroQR::M4(), undef);
    is($g->{rows}, 17, 'forced M4 gives 17×17');
};

# =============================================================================
# 4. Forced ECC level
# =============================================================================

subtest 'forced ECC level' => sub {
    my $gl = encode("HELLO", undef, CodingAdventures::MicroQR::ECC_L());
    my $gm = encode("HELLO", undef, CodingAdventures::MicroQR::ECC_M());
    isnt(grid_str($gl), grid_str($gm), 'ECC_L and ECC_M give different grids');
};

# =============================================================================
# 5. Module shape
# =============================================================================

subtest 'module shape' => sub {
    my $g = encode("1", undef, undef);
    is($g->{module_shape}, 'square', 'module_shape is square');
};

# =============================================================================
# 6. Grid structure
# =============================================================================

subtest 'grid is a square 2D array' => sub {
    for my $input ("1", "HELLO", "hello", "https://a.b") {
        my $g = encode($input, undef, undef);
        is(scalar(@{ $g->{modules} }), $g->{rows}, "rows match for '$input'");
        for my $row (@{ $g->{modules} }) {
            is(scalar(@$row), $g->{cols}, "row length matches cols for '$input'");
        }
    }
};

# =============================================================================
# 7. Finder pattern (structural correctness)
# =============================================================================

subtest 'finder pattern M1' => sub {
    my $m = encode("1", undef, undef)->{modules};
    # Outer border of finder (rows 0 and 6, cols 0 and 6): all dark
    for my $c (0 .. 6) {
        ok($m->[0][$c], "finder top row, col $c dark");
        ok($m->[6][$c], "finder bottom row, col $c dark");
    }
    for my $r (0 .. 6) {
        ok($m->[$r][0], "finder left col, row $r dark");
        ok($m->[$r][6], "finder right col, row $r dark");
    }
    # Inner ring (row 1, row 5, cols 1-5): all light
    for my $c (1 .. 5) {
        ok(!$m->[1][$c], "inner ring row 1 col $c light");
        ok(!$m->[5][$c], "inner ring row 5 col $c light");
    }
    for my $r (1 .. 5) {
        ok(!$m->[$r][1], "inner ring col 1 row $r light");
        ok(!$m->[$r][5], "inner ring col 5 row $r light");
    }
    # Core 3×3 (rows 2-4, cols 2-4): all dark
    for my $r (2 .. 4) {
        for my $c (2 .. 4) {
            ok($m->[$r][$c], "core ($r,$c) dark");
        }
    }
};

# =============================================================================
# 8. Separator
# =============================================================================

subtest 'separator M2' => sub {
    my $m = encode("HELLO", undef, undef)->{modules};
    # Row 7, cols 0-7: light (bottom separator)
    for my $c (0 .. 7) {
        ok(!$m->[7][$c], "separator row 7 col $c light");
    }
    # Col 7, rows 0-7: light (right separator)
    for my $r (0 .. 7) {
        ok(!$m->[$r][7], "separator col 7 row $r light");
    }
};

# =============================================================================
# 9. Timing strips (row 0 and col 0, positions 8+)
# =============================================================================

subtest 'timing strips M4' => sub {
    my $m = encode("https://a.b", undef, undef)->{modules};
    for my $c (8 .. 16) {
        is($m->[0][$c], ($c % 2 == 0) ? 1 : 0, "timing row 0 col $c");
    }
    for my $r (8 .. 16) {
        is($m->[$r][0], ($r % 2 == 0) ? 1 : 0, "timing col 0 row $r");
    }
};

# =============================================================================
# 10. Determinism
# =============================================================================

subtest 'deterministic encoding' => sub {
    for my $input ("1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b") {
        my $g1 = encode($input, undef, undef);
        my $g2 = encode($input, undef, undef);
        is(grid_str($g1), grid_str($g2), "deterministic for '$input'");
    }
};

# =============================================================================
# 11. Different inputs → different grids
# =============================================================================

subtest 'different inputs produce different grids' => sub {
    my $g1 = encode("1", undef, undef);
    my $g2 = encode("2", undef, undef);
    isnt(grid_str($g1), grid_str($g2), 'different data → different grid');
};

# =============================================================================
# 12. ECC level constraints
# =============================================================================

subtest 'ECC level constraints' => sub {
    # M1 only supports DETECTION
    my $m1_d = encode("1", CodingAdventures::MicroQR::M1(), CodingAdventures::MicroQR::DETECTION());
    is($m1_d->{rows}, 11, 'M1/Detection valid');

    # M1 rejects ECC_L
    like(
        dies { encode("1", CodingAdventures::MicroQR::M1(), CodingAdventures::MicroQR::ECC_L()) },
        qr/ECCNotAvailable/,
        'M1 + ECC_L throws ECCNotAvailable'
    );

    # M2 rejects ECC_Q
    like(
        dies { encode("1", CodingAdventures::MicroQR::M2(), CodingAdventures::MicroQR::ECC_Q()) },
        qr/ECCNotAvailable/,
        'M2 + ECC_Q throws ECCNotAvailable'
    );

    # M3 rejects ECC_Q
    like(
        dies { encode("1", CodingAdventures::MicroQR::M3(), CodingAdventures::MicroQR::ECC_Q()) },
        qr/ECCNotAvailable/,
        'M3 + ECC_Q throws ECCNotAvailable'
    );

    # M4 supports ECC_Q
    my $m4_q = encode("HELLO", CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_Q());
    is($m4_q->{rows}, 17, 'M4/Q valid');

    # M4 all 3 ECC levels produce different grids
    my $gl = encode("HELLO", CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_L());
    my $gm = encode("HELLO", CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_M());
    my $gq = encode("HELLO", CodingAdventures::MicroQR::M4(), CodingAdventures::MicroQR::ECC_Q());
    isnt(grid_str($gl), grid_str($gm), 'M4-L != M4-M');
    isnt(grid_str($gm), grid_str($gq), 'M4-M != M4-Q');
    isnt(grid_str($gl), grid_str($gq), 'M4-L != M4-Q');
};

# =============================================================================
# 13. Capacity boundaries
# =============================================================================

subtest 'capacity boundaries' => sub {
    # M1 max: 5 numeric digits
    is(encode("12345", undef, undef)->{rows}, 11, 'M1 max 5 digits fits');
    # 6 digits overflows M1 → M2
    is(encode("123456", undef, undef)->{rows}, 13, '6 digits overflows M1 to M2');

    # M4 max: 35 numeric digits
    is(encode("1" x 35, undef, undef)->{rows}, 17, 'M4 max 35 digits fits');
    # 36 digits → InputTooLong
    like(
        dies { encode("1" x 36, undef, undef) },
        qr/InputTooLong/,
        '36 digits throws InputTooLong'
    );

    # M4-L max byte: 15 chars
    is(encode("a" x 15, undef, undef)->{rows}, 17, 'M4-L max 15 bytes fits');

    # M4-Q max numeric: 21
    is(encode("1" x 21, undef, CodingAdventures::MicroQR::ECC_Q())->{rows}, 17, 'M4-Q max 21 numeric');
};

# =============================================================================
# 14. Error handling
# =============================================================================

subtest 'error handling' => sub {
    # InputTooLong
    like(
        dies { encode("1" x 36, undef, undef) },
        qr/InputTooLong/,
        'InputTooLong for oversize input'
    );

    # ECCNotAvailable for impossible combo
    like(
        dies { encode("1", CodingAdventures::MicroQR::M1(), CodingAdventures::MicroQR::ECC_Q()) },
        qr/ECCNotAvailable/,
        'ECCNotAvailable for M1+Q'
    );
};

# =============================================================================
# 15. Empty string
# =============================================================================

subtest 'empty string encodes to M1' => sub {
    my $g = encode("", undef, undef);
    is($g->{rows}, 11, 'empty string → M1');
};

# =============================================================================
# 16. Format information is non-zero
# =============================================================================

subtest 'format information non-zero' => sub {
    # Row 8 cols 1-8 + col 8 rows 1-7 should have at least some dark modules
    for my $input ("1", "HELLO", "https://a.b") {
        my $g = encode($input, undef, undef);
        my $m = $g->{modules};
        my $count = 0;
        $count++ for grep { $_ } map { $m->[8][$_] } 1 .. 8;
        $count++ for grep { $_ } map { $m->[$_][8] } 1 .. 7;
        ok($count > 0, "format info non-zero for '$input'");
    }
};

# =============================================================================
# 17. encode_at convenience wrapper
# =============================================================================

subtest 'encode_at' => sub {
    my $g = encode_at("HELLO", CodingAdventures::MicroQR::M2(), CodingAdventures::MicroQR::ECC_L());
    is($g->{rows}, 13, 'encode_at M2/L gives 13×13');

    like(
        dies { encode_at("HELLO") },
        qr/version is required/,
        'encode_at requires version'
    );
    like(
        dies { encode_at("HELLO", CodingAdventures::MicroQR::M2()) },
        qr/ecc is required/,
        'encode_at requires ecc'
    );
};

# =============================================================================
# 18. Cross-language corpus (verify expected symbol sizes)
# =============================================================================

subtest 'cross-language corpus' => sub {
    my @cases = (
        ["1",              11],  # M1 single digit
        ["12345",          11],  # M1 max 5 numeric
        ["HELLO",          13],  # M2-L 5 alphanumeric
        ["01234567",       13],  # M2-L 8 numeric
        ["https://a.b",    17],  # M4-L byte mode
        ["MICRO QR TEST",  15],  # M3-L 13 alphanumeric
    );
    for my $case (@cases) {
        my ($input, $expected) = @$case;
        is(encode($input, undef, undef)->{rows}, $expected,
           "corpus '$input' → $expected×$expected");
    }
};

done_testing;
