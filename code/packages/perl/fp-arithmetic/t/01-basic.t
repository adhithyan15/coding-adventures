use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::FpArithmetic; 1 }, 'module loads');

CodingAdventures::FpArithmetic->import(qw(
    encode_f32 decode_f32
    f32_add f32_mul
    f32_to_string float_to_f32 f32_to_float
));

# ============================================================================
# encode_f32 / decode_f32 round-trip
# ============================================================================

subtest 'encode_f32 basic' => sub {
    my $bits = encode_f32(0, 127, 0);  # 1.0
    is($bits, 0x3F800000, 'encode 1.0');

    my $bits2 = encode_f32(0, 128, 0x400000);  # 1.5 * 2^1 = 3.0? No: exp=128 → true=1, mant=0x400000=0.5 → 1.5 * 2 = 3.0
    is(($bits2 >> 31) & 1, 0, 'sign is 0');
    is(($bits2 >> 23) & 0xFF, 128, 'exponent is 128');
};

subtest 'decode_f32 round-trip' => sub {
    my $bits = encode_f32(1, 130, 0x200000);
    my ($s, $e, $m) = decode_f32($bits);
    is($s, 1, 'sign round-trip');
    is($e, 130, 'exponent round-trip');
    is($m, 0x200000, 'mantissa round-trip');
};

subtest 'decode_f32 positive zero' => sub {
    my ($s, $e, $m) = decode_f32(0);
    is($s, 0, 'sign of +0');
    is($e, 0, 'exp of +0');
    is($m, 0, 'mant of +0');
};

# ============================================================================
# float_to_f32 / f32_to_float
# ============================================================================

subtest 'float_to_f32 known values' => sub {
    my $bits = float_to_f32(1.0);
    is($bits, 0x3F800000, '1.0 → 0x3F800000');

    my $bits2 = float_to_f32(2.0);
    is($bits2, 0x40000000, '2.0 → 0x40000000');

    my $bits3 = float_to_f32(0.5);
    is($bits3, 0x3F000000, '0.5 → 0x3F000000');
};

subtest 'f32_to_float known values' => sub {
    my $f = f32_to_float(0x3F800000);
    ok(abs($f - 1.0) < 1e-6, '0x3F800000 → 1.0');

    my $f2 = f32_to_float(0x40000000);
    ok(abs($f2 - 2.0) < 1e-6, '0x40000000 → 2.0');
};

subtest 'float_to_f32 round-trip' => sub {
    for my $v (1.5, 3.14, -2.0, 0.125, 100.0) {
        my $bits   = float_to_f32($v);
        my $back   = f32_to_float($bits);
        ok(abs($back - $v) / (abs($v) + 1e-10) < 1e-5, "round-trip $v");
    }
};

# ============================================================================
# f32_add
# ============================================================================

subtest 'f32_add basic' => sub {
    my $a = float_to_f32(1.0);
    my $b = float_to_f32(2.0);
    my $c = f32_add($a, $b);
    my $f = f32_to_float($c);
    ok(abs($f - 3.0) < 1e-5, '1.0 + 2.0 = 3.0');
};

subtest 'f32_add with fractions' => sub {
    my $a = float_to_f32(1.5);
    my $b = float_to_f32(0.25);
    my $c = f32_add($a, $b);
    my $f = f32_to_float($c);
    ok(abs($f - 1.75) < 1e-5, '1.5 + 0.25 = 1.75');
};

subtest 'f32_add zero identity' => sub {
    my $a = float_to_f32(5.0);
    my $z = float_to_f32(0.0);
    my $c = f32_add($a, $z);
    my $f = f32_to_float($c);
    ok(abs($f - 5.0) < 1e-5, '5.0 + 0.0 = 5.0');
};

subtest 'f32_add negative' => sub {
    my $a = float_to_f32(3.0);
    my $b = float_to_f32(-1.0);
    my $c = f32_add($a, $b);
    my $f = f32_to_float($c);
    ok(abs($f - 2.0) < 1e-5, '3.0 + (-1.0) = 2.0');
};

subtest 'f32_add NaN propagation' => sub {
    my $nan  = CodingAdventures::FpArithmetic->POS_NAN;
    my $one  = float_to_f32(1.0);
    my $res  = f32_add($nan, $one);
    ok(CodingAdventures::FpArithmetic::_is_nan($res), 'NaN + 1 = NaN');
};

subtest 'f32_add Inf + Inf' => sub {
    my $inf = CodingAdventures::FpArithmetic->POS_INF;
    my $res = f32_add($inf, $inf);
    ok(CodingAdventures::FpArithmetic::_is_inf($res), '+Inf + +Inf = +Inf');
};

subtest 'f32_add Inf + (-Inf) = NaN' => sub {
    my $pos_inf = CodingAdventures::FpArithmetic->POS_INF;
    my $neg_inf = CodingAdventures::FpArithmetic->NEG_INF;
    my $res     = f32_add($pos_inf, $neg_inf);
    ok(CodingAdventures::FpArithmetic::_is_nan($res), '+Inf + (-Inf) = NaN');
};

# ============================================================================
# f32_mul
# ============================================================================

subtest 'f32_mul basic' => sub {
    my $a = float_to_f32(2.0);
    my $b = float_to_f32(3.0);
    my $c = f32_mul($a, $b);
    my $f = f32_to_float($c);
    ok(abs($f - 6.0) < 1e-5, '2.0 * 3.0 = 6.0');
};

subtest 'f32_mul by zero' => sub {
    my $a = float_to_f32(100.0);
    my $z = float_to_f32(0.0);
    my $c = f32_mul($a, $z);
    ok(CodingAdventures::FpArithmetic::_is_zero($c), '100.0 * 0.0 = 0');
};

subtest 'f32_mul sign rules' => sub {
    my $p3 = float_to_f32(3.0);
    my $n2 = float_to_f32(-2.0);
    my $c  = f32_mul($p3, $n2);
    my $f  = f32_to_float($c);
    ok(abs($f - (-6.0)) < 1e-5, '3 * -2 = -6');

    my $c2 = f32_mul($n2, $n2);
    my $f2 = f32_to_float($c2);
    ok(abs($f2 - 4.0) < 1e-5, '-2 * -2 = 4');
};

subtest 'f32_mul Inf * 0 = NaN' => sub {
    my $inf  = CodingAdventures::FpArithmetic->POS_INF;
    my $zero = float_to_f32(0.0);
    my $res  = f32_mul($inf, $zero);
    ok(CodingAdventures::FpArithmetic::_is_nan($res), 'Inf * 0 = NaN');
};

# ============================================================================
# f32_to_string
# ============================================================================

subtest 'f32_to_string special values' => sub {
    is(f32_to_string(CodingAdventures::FpArithmetic->POS_NAN), 'NaN',  'NaN string');
    is(f32_to_string(CodingAdventures::FpArithmetic->POS_INF), '+Inf', '+Inf string');
    is(f32_to_string(CodingAdventures::FpArithmetic->NEG_INF), '-Inf', '-Inf string');
    is(f32_to_string(0), '+0', '+0 string');
};

subtest 'f32_to_string normal value' => sub {
    my $str = f32_to_string(float_to_f32(1.5));
    like($str, qr/1\.5/, 'string contains 1.5');
};

done_testing;
