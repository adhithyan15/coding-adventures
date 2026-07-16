use strict;
use warnings;
use utf8;
use Encode qw(encode);
use Test::More;
use CodingAdventures::HashFunctions qw(
    fnv1a_32
    fnv1a_64
    djb2
    polynomial_rolling
    murmur3_32
    avalanche_score
    distribution_test
    uint64_hex
);

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'FNV-1a 32-bit vectors and binary input' => sub {
    is(fnv1a_32(''), 2166136261, 'empty');
    is(fnv1a_32('a'), 3826002220, 'a');
    is(fnv1a_32('abc'), 440920331, 'abc');
    is(fnv1a_32('hello'), 1335831723, 'hello');
    is(fnv1a_32('foobar'), 3214735720, 'foobar');

    my $all_bytes = pack('C*', 0 .. 255);
    is(fnv1a_32($all_bytes), fnv1a_32($all_bytes), 'all bytes deterministic');
    isnt(fnv1a_32("a\0b"), fnv1a_32('ab'), 'null byte participates');
    my $text = "caf\x{e9}";
    utf8::upgrade($text);
    is(fnv1a_32($text), fnv1a_32(encode('UTF-8', $text)), 'UTF-8 text');
};

subtest 'exact FNV-1a 64-bit vectors' => sub {
    is(fnv1a_64('')->bstr, '14695981039346656037', 'empty decimal');
    is(fnv1a_64('a')->bstr, '12638187200555641996', 'a decimal');
    is(fnv1a_64('abc')->bstr, '16654208175385433931', 'abc decimal');
    is(fnv1a_64('hello')->bstr, '11831194018420276491', 'hello decimal');
    is(uint64_hex(fnv1a_64('hello')), 'a430d84680aabd0b', 'hello bits');
};

subtest 'DJB2 vectors and 64-bit wrapping' => sub {
    is(djb2('')->bstr, '5381', 'empty');
    is(djb2('a')->bstr, '177670', 'a');
    is(djb2('abc')->bstr, '193485963', 'abc');
    is(djb2('hello')->bstr, '210714636441', 'hello');
    is(uint64_hex(djb2('a' x 1000)), 'cb2c236ad13cc66d', 'long word wraps');
};

subtest 'polynomial rolling parameters and large intermediates' => sub {
    is(polynomial_rolling('')->bstr, '0', 'empty');
    is(polynomial_rolling('a')->bstr, '97', 'a');
    is(polynomial_rolling('ab')->bstr, '3105', 'ab');
    is(polynomial_rolling('abc')->bstr, '96354', 'abc');
    isnt(
        polynomial_rolling('hello', 31)->bstr,
        polynomial_rolling('hello', 37)->bstr,
        'custom base',
    );
    cmp_ok(polynomial_rolling('hello world', 31, 100)->numify, '<', 100, 'custom modulus');
    cmp_ok(polynomial_rolling('hash me' x 500)->numify, '>=', 0, 'long input');
    dies_like(
        sub { polynomial_rolling('x', 31, 0) },
        qr/modulus must be positive/,
        'zero modulus rejected',
    );
};

subtest 'MurmurHash3 vectors and tail paths' => sub {
    is(murmur3_32('', 0), 0, 'empty seed zero');
    is(murmur3_32('', 1), 0x514e28b7, 'empty seed one');
    is(murmur3_32('a', 0), 0x3c2569b2, 'a');
    is(murmur3_32('abc', 0), 0xb3dd93fa, 'abc');
    isnt(murmur3_32('abcd'), murmur3_32('abce'), 'full block changes');
    cmp_ok(murmur3_32('abcde'), '>=', 0, 'one-byte tail');
    cmp_ok(murmur3_32('abcdef'), '>=', 0, 'two-byte tail');
    cmp_ok(murmur3_32('abcdefg'), '>=', 0, 'three-byte tail');
    isnt(murmur3_32('hello', 0), murmur3_32('hello', 1), 'seed changes hash');
};

subtest 'deterministic quality metrics' => sub {
    my $fnv_score = avalanche_score(\&fnv1a_32, 32, 8);
    my $murmur_score = avalanche_score(\&murmur3_32, 32, 8);
    cmp_ok($fnv_score, '>=', 0, 'FNV score lower bound');
    cmp_ok($fnv_score, '<=', 1, 'FNV score upper bound');
    cmp_ok($murmur_score, '>=', 0, 'Murmur score lower bound');
    cmp_ok($murmur_score, '<=', 1, 'Murmur score upper bound');
    is(avalanche_score(\&fnv1a_32, 32, 8), $fnv_score, 'analysis deterministic');

    my $chi_squared = distribution_test(sub { 0 }, [qw(a b c d)], 4);
    is($chi_squared, 12, 'exact clustered statistic');
    cmp_ok(
        distribution_test(\&fnv1a_64, [qw(a b c d e)], 7),
        '>=',
        0,
        'big integer distribution',
    );
};

subtest 'public input validation' => sub {
    dies_like(sub { fnv1a_32({}) }, qr/data must be a scalar/, 'reference input');
    dies_like(sub { murmur3_32('x', 1.5) }, qr/seed must be an integer/, 'fractional seed');
    dies_like(
        sub { avalanche_score(\&fnv1a_32, 0, 1) },
        qr/output_bits must be in 1\.\.64/,
        'output width',
    );
    dies_like(
        sub { avalanche_score(\&fnv1a_32, 32, 0) },
        qr/sample_size must be positive/,
        'sample size',
    );
    dies_like(
        sub { distribution_test(\&fnv1a_32, [], 10) },
        qr/inputs must be a non-empty array reference/,
        'empty inputs',
    );
    dies_like(
        sub { distribution_test(\&fnv1a_32, ['x'], 0) },
        qr/num_buckets must be positive/,
        'bucket count',
    );
};

done_testing;
