use strict;
use warnings;
use Test2::V0;
use List::Util qw(all);

use lib '../lib', '../../gf256/lib';
use CodingAdventures::ReedSolomon qw(encode decode syndromes build_generator error_locator);

# ============================================================================
# build_generator tests
# ============================================================================
# The generator polynomial is the backbone of RS encoding/decoding.
# For n_check=2: g(x) = (x+α)(x+α²) = x² + 6x + 8
# In LE: [8, 6, 1]

subtest 'build_generator: cross-language test vector n_check=2' => sub {
    my $g = build_generator(2);
    is($g->[0], 8, 'g[0] = 8');
    is($g->[1], 6, 'g[1] = 6');
    is($g->[2], 1, 'g[2] = 1 (monic)');
};

subtest 'build_generator: length is n_check+1' => sub {
    for my $nc (2, 4, 6, 8, 10) {
        my $g = build_generator($nc);
        is(scalar(@$g), $nc + 1, "n_check=$nc: length = n_check+1");
    }
};

subtest 'build_generator: monic (leading coeff = 1)' => sub {
    for my $nc (2, 4, 6, 8) {
        my $g = build_generator($nc);
        is($g->[-1], 1, "n_check=$nc: g[-1] = 1 (monic)");
    }
};

subtest 'build_generator: constant term is nonzero for all valid n_check' => sub {
    # The constant term is the product of all roots α¹·α²·…·α^{n_check},
    # which is α^{1+2+…+n_check} ≠ 0 in GF(256).
    for my $nc (2, 4, 6) {
        my $g = build_generator($nc);
        ok($g->[0] != 0, "n_check=$nc: constant term != 0");
    }
};

subtest 'build_generator: invalid n_check dies' => sub {
    like(dies { build_generator(0) },  qr/InvalidInput/, 'n_check=0 dies');
    like(dies { build_generator(1) },  qr/InvalidInput/, 'n_check=1 (odd) dies');
    like(dies { build_generator(3) },  qr/InvalidInput/, 'n_check=3 (odd) dies');
    like(dies { build_generator(-2) }, qr/InvalidInput/, 'n_check=-2 dies');
};

subtest 'build_generator: n_check=4 has length 5' => sub {
    my $g = build_generator(4);
    is(scalar(@$g), 5, 'n_check=4: generator has 5 terms');
    is($g->[-1], 1, 'n_check=4: monic');
};

# ============================================================================
# encode tests
# ============================================================================

subtest 'encode: output length = message.length + n_check' => sub {
    my $msg = [1, 2, 3, 4, 5];
    for my $nc (2, 4, 6) {
        my $cw = encode($msg, $nc);
        is(scalar(@$cw), 5 + $nc, "n_check=$nc: length correct");
    }
};

subtest 'encode: systematic — message bytes unchanged' => sub {
    my $msg = [0x48, 0x65, 0x6C, 0x6C, 0x6F];   # "Hello"
    my $cw  = encode($msg, 4);
    for my $i (0 .. $#$msg) {
        is($cw->[$i], $msg->[$i], "cw[$i] = msg[$i] (systematic)");
    }
};

subtest 'encode: valid codeword has all-zero syndromes' => sub {
    my $msg = [10, 20, 30, 40];
    for my $nc (2, 4, 6) {
        my $cw = encode($msg, $nc);
        my $s  = syndromes($cw, $nc);
        my $all_zero = all { $_ == 0 } @$s;
        ok($all_zero, "n_check=$nc: all syndromes zero for valid codeword");
    }
};

subtest 'encode: single byte message' => sub {
    my $cw = encode([42], 2);
    is(scalar(@$cw), 3, 'single byte + 2 check = 3 bytes');
    is($cw->[0], 42, 'message byte preserved');
    my $s = syndromes($cw, 2);
    ok((all { $_ == 0 } @$s), 'syndromes zero');
};

subtest 'encode: invalid n_check dies' => sub {
    like(dies { encode([1,2,3], 0) }, qr/InvalidInput/, 'n_check=0 dies');
    like(dies { encode([1,2,3], 1) }, qr/InvalidInput/, 'n_check=1 (odd) dies');
    like(dies { encode([1,2,3], 3) }, qr/InvalidInput/, 'n_check=3 (odd) dies');
};

subtest 'encode: total length > 255 dies' => sub {
    # 252 message bytes + 4 check bytes = 256 > 255
    my @big = (1) x 252;
    like(dies { encode(\@big, 4) }, qr/InvalidInput/, 'total > 255 dies');
};

subtest 'encode: exactly 255 bytes total is allowed' => sub {
    my @msg = (0) x 251;
    my $cw = encode(\@msg, 4);
    is(scalar(@$cw), 255, 'exactly 255 bytes is valid');
};

# ============================================================================
# syndromes tests
# ============================================================================

subtest 'syndromes: valid codeword gives all zeros' => sub {
    my $cw = encode([1, 2, 3, 4, 5], 4);
    my $s  = syndromes($cw, 4);
    is(scalar(@$s), 4, 'syndromes returns 4 values');
    ok((all { $_ == 0 } @$s), 'all syndromes zero for valid codeword');
};

subtest 'syndromes: corrupted codeword gives nonzero syndromes' => sub {
    my $cw = encode([10, 20, 30], 4);
    my @bad = @$cw;
    $bad[0] ^= 0x55;   # corrupt first byte
    my $s = syndromes(\@bad, 4);
    ok((grep { $_ != 0 } @$s) > 0, 'at least one syndrome nonzero after corruption');
};

subtest 'syndromes: returns n_check values' => sub {
    my $cw = encode([0xAA, 0xBB], 6);
    my $s  = syndromes($cw, 6);
    is(scalar(@$s), 6, 'syndromes returns 6 values for n_check=6');
};

# ============================================================================
# decode tests
# ============================================================================

subtest 'decode: no errors returns original message' => sub {
    my $msg = [72, 101, 108, 108, 111];   # "Hello"
    my $cw  = encode($msg, 4);
    my $rec = decode($cw, 4);
    is($rec, $msg, 'no-error decode returns original message');
};

subtest 'decode: single error correction' => sub {
    my $msg = [1, 2, 3, 4, 5, 6];
    my $cw  = encode($msg, 4);
    my @bad = @$cw;
    $bad[2] ^= 0xAB;   # corrupt byte 2

    my $rec = decode(\@bad, 4);
    is($rec, $msg, 'single error corrected');
};

subtest 'decode: two errors with n_check=4 (t=2)' => sub {
    my $msg = [0x01, 0x02, 0x03, 0x04, 0x05];
    my $cw  = encode($msg, 4);
    my @bad = @$cw;
    $bad[0] ^= 0x11;
    $bad[4] ^= 0x22;

    my $rec = decode(\@bad, 4);
    is($rec, $msg, 'two errors corrected with n_check=4');
};

subtest 'decode: four errors with n_check=8 (t=4)' => sub {
    my $msg = [10, 20, 30, 40, 50, 60, 70, 80];
    my $cw  = encode($msg, 8);
    my @bad = @$cw;
    $bad[0]  ^= 0x01;
    $bad[3]  ^= 0x02;
    $bad[7]  ^= 0x04;
    $bad[10] ^= 0x08;

    my $rec = decode(\@bad, 8);
    is($rec, $msg, 'four errors corrected with n_check=8');
};

subtest 'decode: error in check byte region is corrected' => sub {
    my $msg = [0xDE, 0xAD, 0xBE, 0xEF];
    my $cw  = encode($msg, 4);
    my @bad = @$cw;
    $bad[-1] ^= 0xFF;   # corrupt last check byte

    my $rec = decode(\@bad, 4);
    is($rec, $msg, 'error in check byte corrected');
};

subtest 'decode: all-zeros message round-trips' => sub {
    my $msg = [(0) x 10];
    my $cw  = encode($msg, 4);
    my $rec = decode($cw, 4);
    is($rec, $msg, 'all-zeros message round-trips');
};

subtest 'decode: all-0xFF message round-trips' => sub {
    my $msg = [(0xFF) x 6];
    my $cw  = encode($msg, 4);
    my $rec = decode($cw, 4);
    is($rec, $msg, 'all-0xFF message round-trips');
};

subtest 'decode: TooManyErrors when errors exceed capacity' => sub {
    my $msg = [1, 2, 3, 4, 5];
    my $cw  = encode($msg, 4);   # t=2: can correct up to 2 errors
    my @bad = @$cw;
    $bad[0] ^= 0x11;
    $bad[1] ^= 0x22;
    $bad[2] ^= 0x33;   # 3 errors > t=2

    like(dies { decode(\@bad, 4) }, qr/TooManyErrors|InvalidInput/, 'too many errors dies');
};

subtest 'decode: InvalidInput for bad n_check' => sub {
    like(dies { decode([1,2,3,4], 0) }, qr/InvalidInput/, 'n_check=0 dies');
    like(dies { decode([1,2,3,4], 1) }, qr/InvalidInput/, 'n_check=1 dies');
    like(dies { decode([1,2,3,4], 3) }, qr/InvalidInput/, 'n_check=3 dies');
};

subtest 'decode: InvalidInput when received shorter than n_check' => sub {
    like(dies { decode([1, 2], 4) }, qr/InvalidInput/, 'received too short dies');
};

subtest 'decode: single byte message round-trip' => sub {
    my $cw  = encode([255], 2);
    my $rec = decode($cw, 2);
    is($rec, [255], 'single byte 255 round-trips');
};

subtest 'decode: large message round-trip' => sub {
    my @msg = map { $_ % 256 } 1 .. 100;
    my $cw  = encode(\@msg, 8);
    my $rec = decode($cw, 8);
    is($rec, \@msg, 'large message (100 bytes) round-trips');
};

# ============================================================================
# error_locator tests
# ============================================================================

subtest 'error_locator: all-zero syndromes → [1]' => sub {
    my $cw  = encode([1, 2, 3, 4], 4);
    my $s   = syndromes($cw, 4);
    my $lam = error_locator($s);
    is($lam, [1], 'no errors → lambda = [1]');
};

subtest 'error_locator: lambda[0] = 1 always' => sub {
    # Corrupt one byte to get nonzero syndromes
    my $cw = encode([10, 20, 30, 40, 50], 4);
    my @bad = @$cw;
    $bad[0] ^= 0xAA;
    my $s   = syndromes(\@bad, 4);
    my $lam = error_locator($s);
    is($lam->[0], 1, 'lambda[0] = 1');
};

subtest 'error_locator: degree matches number of errors (1 error)' => sub {
    my $cw = encode([1,2,3,4,5,6], 4);
    my @bad = @$cw;
    $bad[2] ^= 0x55;
    my $s   = syndromes(\@bad, 4);
    my $lam = error_locator($s);
    # Degree of lambda = number of errors = 1 → length should be 2 (indices 0..1)
    # (trailing zeros may be present, but the degree-1 term should be nonzero)
    ok(scalar(@$lam) >= 2, 'lambda has at least 2 terms for 1 error');
    ok($lam->[-1] != 0 || $lam->[1] != 0, 'degree >= 1 for 1 error');
};

subtest 'error_locator: degree matches number of errors (2 errors)' => sub {
    my $cw = encode([5,10,15,20,25], 4);
    my @bad = @$cw;
    $bad[0] ^= 0x10;
    $bad[4] ^= 0x20;
    my $s   = syndromes(\@bad, 4);
    my $lam = error_locator($s);
    ok(scalar(@$lam) >= 3, 'lambda has at least 3 terms for 2 errors');
};

# ============================================================================
# Round-trip property tests
# ============================================================================

subtest 'round-trip: various messages and n_check=2' => sub {
    for my $msg ([0,0,0], [1,1,1], [255,128,0], [0x48,0x65,0x6C]) {
        my $cw  = encode($msg, 2);
        my $rec = decode($cw, 2);
        is($rec, $msg, "round-trip: " . join(',', @$msg));
    }
};

subtest 'round-trip: error correction preserves message' => sub {
    # Property: for any 1 error and n_check >= 2, decode recovers message
    my $msg = [0x01, 0x23, 0x45, 0x67, 0x89];
    my $cw  = encode($msg, 4);
    my $n   = scalar(@$cw);

    for my $pos (0 .. $n - 1) {
        my @bad = @$cw;
        $bad[$pos] ^= 0x01;
        my $rec = decode(\@bad, 4);
        is($rec, $msg, "position $pos error corrected");
    }
};

subtest 'round-trip: n_check=6 corrects up to 3 errors' => sub {
    my $msg = [10, 20, 30, 40];
    my $cw  = encode($msg, 6);
    my @bad = @$cw;
    $bad[0] ^= 0x01;
    $bad[2] ^= 0x02;
    $bad[5] ^= 0x03;
    my $rec = decode(\@bad, 6);
    is($rec, $msg, '3 errors corrected with n_check=6');
};

done_testing;
