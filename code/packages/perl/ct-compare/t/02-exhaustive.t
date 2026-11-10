use strict;
use warnings;
use Test2::V0;
no warnings q{portable};

use CodingAdventures::CtCompare qw(
    ct_eq ct_eq_fixed ct_select_bytes ct_eq_u64
    ct_eq_bit ct_eq_fixed_bit ct_eq_u64_bit
);

# Checks the corpus only samples: every byte position, all 256 byte values
# through selection, length mismatches, all 64 single-bit flips, validation.

# ---------------------------------------------------------------- equality --

subtest 'equal and empty strings' => sub {
    ok(ct_eq('', ''), 'empty');
    ok(ct_eq('secret', 'secret'), 'same');
    is(ct_eq_bit('x', 'x'), 1, 'bit 1');
    is(ct_eq_fixed_bit('', ''), 1, 'fixed bit 1');
};

subtest 'mismatch at every byte position of 64 bytes' => sub {
    my @base = map { ($_ * 7) % 256 } 1 .. 64;
    my $a = pack 'C*', @base;
    for my $pos (0 .. 63) {
        my @copy = @base;
        $copy[$pos] ^= 0x01;
        my $b = pack 'C*', @copy;
        ok(!ct_eq($a, $b), "position $pos");
        is(ct_eq_bit($a, $b), 0, "bit at $pos");
        ok(!ct_eq_fixed($a, $b), "fixed at $pos");
    }
};

subtest 'every single-bit difference of every byte value' => sub {
    my $bad = 0;
    for my $v (0 .. 255) {
        ok(ct_eq(chr $v, chr $v), "value $v equals itself") or $bad++;
        for my $bit (0 .. 7) {
            $bad++ if ct_eq(chr $v, chr($v ^ (1 << $bit)));
        }
    }
    is($bad, 0, 'no single-bit difference compares equal');
};

subtest 'length mismatch' => sub {
    ok(!ct_eq('a', 'ab'), 'shorter left');
    ok(!ct_eq('ab', 'a'), 'shorter right');
    ok(!ct_eq('', 'a'), 'empty vs one');
    is(ct_eq_bit('', 'a'), 0, 'bit 0');
    ok(!ct_eq_fixed('abc', 'abcd'), 'fixed mismatch');
};

subtest 'bytes with NUL and high bit' => sub {
    ok(!ct_eq("a\0b", "a\0c"), 'NUL data differs');
    ok(ct_eq("a\0b", "a\0b"), 'NUL data equal');
    ok(ct_eq("\xff\x80", "\xff\x80"), 'high bits equal');
    ok(!ct_eq("\xff", "\x7f"), 'high bit differs');
};

subtest 'array references of byte values' => sub {
    ok(ct_eq([1, 2, 3], "\x01\x02\x03"), 'array vs string');
    ok(ct_eq([], ''), 'empty array');
    ok(!ct_eq([1, 2, 3], [1, 2, 4]), 'arrays differ');
    my @src = (1, 2, 3);
    ct_eq(\@src, \@src);
    is(\@src, [1, 2, 3], 'input array untouched');
};

subtest 'malformed byte inputs are rejected, not coerced' => sub {
    ok(dies { ct_eq([256], "\0") }, '256');
    ok(dies { ct_eq([-1], "\xff") }, 'negative');
    ok(dies { ct_eq([97.5], 'a') }, 'float');
    ok(dies { ct_eq(['9e1'], 'Z') }, 'exponent string');
    ok(dies { ct_eq(['a'], 'a') }, 'letter');
    ok(dies { ct_eq([undef], "\0") }, 'undef element');
    ok(dies { ct_eq([[1]], "\1") }, 'nested ref');
    ok(dies { ct_eq(undef, '') }, 'undef left');
    ok(dies { ct_eq('', undef) }, 'undef right');
    ok(dies { ct_eq({}, '') }, 'hash ref');
    ok(dies { ct_eq(sub { 1 }, '') }, 'code ref');
    ok(dies { ct_eq(bless([], 'Foo'), '') }, 'blessed array');
    ok(dies { ct_eq("\x{100}", "\x{100}") }, 'wide characters');
};

# --------------------------------------------------------------- selection --

subtest 'picks left for 1, right for 0' => sub {
    is(ct_select_bytes('AB', 'CD', 1), 'AB', 'left');
    is(ct_select_bytes('AB', 'CD', 0), 'CD', 'right');
};

subtest 'all 256 byte values against all 256 byte values' => sub {
    my $right = pack 'C*', 0 .. 255;
    my $bad = 0;
    for my $lv (0 .. 255) {
        my $left = chr($lv) x 256;
        $bad++ unless ct_select_bytes($left, $right, 1) eq $left;
        $bad++ unless ct_select_bytes($left, $right, 0) eq $right;
    }
    is($bad, 0, 'every (left, right) byte pair selects correctly');
};

subtest 'ascending versus descending in one pass' => sub {
    my $asc  = pack 'C*', 0 .. 255;
    my $desc = reverse $asc;
    is(ct_select_bytes($asc, $desc, 1), $asc, 'left');
    is(ct_select_bytes($asc, $desc, 0), $desc, 'right');
};

subtest 'empty inputs' => sub {
    is(ct_select_bytes('', '', 1), '', 'choice 1');
    is(ct_select_bytes('', '', 0), '', 'choice 0');
};

subtest 'unequal lengths raise CT_SELECT_LENGTH_MISMATCH' => sub {
    for my $pair (['a', 'ab'], ['ab', 'a'], ['', 'a']) {
        like(dies { ct_select_bytes(@{$pair}, 1) },
            qr/CT_SELECT_LENGTH_MISMATCH/, join('/', @{$pair}));
    }
};

subtest 'choice must be the integer 0 or 1' => sub {
    for my $bad (2, -1, 3, 255, 256, 1.5, '1.0', 'x', '', '1e0', [], {}, undef, '9' x 30) {
        my $label = defined $bad ? (ref $bad || "'$bad'") : 'undef';
        ok(dies { ct_select_bytes('a', 'b', $bad) }, "rejects $label");
    }
    ok(dies { ct_select_bytes('a', 'b') }, 'missing choice');
};

subtest 'composes with the bit-returning equalities' => sub {
    is(ct_select_bytes('yes', 'no!', ct_eq_bit('tag', 'tag')), 'yes', 'equal');
    is(ct_select_bytes('yes', 'no!', ct_eq_bit('tag', 'tab')), 'no!', 'different');
    is(ct_select_bytes('yes', 'no!', ct_eq_u64_bit(5, 6)), 'no!', 'u64');
};

subtest 'array inputs give a string and are not mutated' => sub {
    my @l = (1, 2);
    my @r = (3, 4);
    is(ct_select_bytes(\@l, \@r, 1), "\x01\x02", 'left');
    is(ct_select_bytes(\@l, \@r, 0), "\x03\x04", 'right');
    is(\@l, [1, 2], 'left untouched');
    is(\@r, [3, 4], 'right untouched');
};

# ------------------------------------------------------------------- u64 ---

my $MAX = 18446744073709551615;

subtest 'equal values including 0, max and the sign bit' => sub {
    for my $v (0, 1, $MAX, 9223372036854775808, 9223372036854775807, 0x0123456789ABCDEF) {
        ok(ct_eq_u64($v, $v), "equal $v");
        is(ct_eq_u64_bit($v, $v), 1, "bit $v");
    }
};

subtest 'all 64 single-bit flips from several bases' => sub {
    for my $base (0, $MAX, 0x5555555555555555, 0x0123456789ABCDEF, 9223372036854775808) {
        for my $bit (0 .. 63) {
            my $flipped = $base ^ (1 << $bit);
            ok(!ct_eq_u64($base, $flipped), "base $base bit $bit");
            is(ct_eq_u64_bit($flipped, $base), 0, "bit result base $base bit $bit");
        }
    }
};

subtest 'decimal strings and numbers agree' => sub {
    ok(ct_eq_u64('18446744073709551615', $MAX), 'max as string');
    ok(!ct_eq_u64('0', $MAX), 'zero vs max');
    ok(!ct_eq_u64(9223372036854775807, 9223372036854775808), 'around sign bit');
};

subtest 'invalid u64 values are rejected' => sub {
    for my $bad (-1, '-1', 1.5, '1.5', '1e3', '18446744073709551616',
        '99999999999999999999', '00', '01', '', 'x', ' 1', "1\n", [], undef) {
        my $label = defined $bad ? (ref $bad || "'$bad'") : 'undef';
        ok(dies { ct_eq_u64($bad, 1) }, "left $label");
        ok(dies { ct_eq_u64(1, $bad) }, "right $label");
    }
};

done_testing;
