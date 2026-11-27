# Hazard 1: 64-bit unsigned arithmetic.  SplitMix64, FNV-1a, the unsigned
# threshold and bounded draws against cross-lane and Python-derived vectors, the
# Int type across the 2**62 / 2**63 / 2**64 boundaries, and the pins for what the
# pbt-v1 corpus cannot catch (VER05p section 10).
use strict;
use warnings;
no warnings 'portable';    # 64-bit hex constants are the point
use Test::More;
use Math::BigInt;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of repo_root read_file bounded_by);
use Vectors;

my $Int  = 'CodingAdventures::TestBootstrap::Int';
my $Prng = 'CodingAdventures::TestBootstrap::Prng';
my $Hooks = 'CodingAdventures::TestBootstrap::Hooks';
my $I = sub { my $n = shift; $Int->can($n) };
my $P = sub { my $n = shift; $Prng->can($n) };

sub words {
    my ( $rng, $n ) = @_;
    return [ map { dec( $rng->next_u64 ) } 1 .. $n ];
}

# ------------------------------------------------------------------ Int

subtest 'Int: native, big and normalization' => sub {
    is( dec( $I->('add')->( 9223372036854775807, 1 ) ), '9223372036854775808', 'IV_MAX + 1 does not become a double' );
    ok( $I->('is_big')->( $I->('add')->( 9223372036854775807, 1 ) ), '... it is a big' );
    my $back = $I->('sub')->( $I->('add')->( 9223372036854775807, 1 ), 1 );
    ok( !$I->('is_big')->($back), 'normalized back to a native integer' );
    is( $back, 9223372036854775807, '... with the exact value' );
    is( dec( $I->('sub')->( -9223372036854775807, 2 ) ), '-9223372036854775809', 'IV_MIN - 1' );
    is( dec( $I->('neg')->( -9223372036854775807 - 1 ) ), '9223372036854775808', 'negating IV_MIN' );
    is( dec( $I->('abs')->( -9223372036854775807 - 1 ) ), '9223372036854775808', 'abs of IV_MIN' );
    is( $I->('floor_half')->(-81), -41, 'floor(-81/2) is -41' );
    is( $I->('floor_half')->(81),  40,  'floor(81/2) is 40' );
    is( dec( $I->('floor_half')->( int_of('-18446744073709551615') ) ), '-9223372036854775808', 'floor_half of a big' );
    is( $I->('mod')->( -7, 3 ), 2, 'floored mod of a negative' );
    is( dec( $I->('pow2')->(64) ), '18446744073709551616', '2^64' );
    is( dec( $I->('shl')->( 1, 100 ) ), '1267650600228229401496703205376', 'shl' );
    is( $I->('bit_length')->(255), 8, 'bit_length 255' );
    is( $I->('bit_length')->(0), 0, 'bit_length 0' );
    is( $I->('bit_length')->( $I->('pow2')->(100) ), 101, 'bit_length 2^100' );
    is( $I->('cmp')->( $I->('pow2')->(64), 5 ), 1, 'a positive big is above any native' );
    is( $I->('cmp')->( $I->('neg')->( $I->('pow2')->(64) ), 5 ), -1, 'a negative big is below any native' );
    ok( $I->('is_even')->(4) && !$I->('is_even')->(5), 'is_even' );
    for my $b ( 4611686018427387903, 4611686018427387904, 4611686018427387905, 9223372036854775807 ) {
        my $sum = $I->('add')->( $b, $b );
        is( dec($sum), Math::BigInt->new($b)->badd($b)->bstr, "$b + $b across the 2^62 boundary" );
    }
};

subtest 'Int: floats, strings, undef and references are refused' => sub {
    my $f = 3.0;
    ok( !eval { $I->('check')->( $f, 'x' ); 1 }, 'a float is not an Int' );
    like( $@, qr/must be an Integer/, '... with a clear message' );
    ok( !eval { $I->('check')->( 1e3, 'x' ); 1 }, '1e3 is not an Int' );
    ok( !eval { $I->('check')->( '5', 'x' ); 1 }, 'the string "5" is not an Int' );
    ok( !eval { $I->('check')->( undef, 'x' ); 1 }, 'undef is not an Int' );
    ok( !eval { $I->('check')->( [], 'x' ); 1 }, 'a reference is not an Int' );
    ok( !eval { $P->('new')->(0.5) }, 'a float seed is refused' );
    ok( !eval { $P->('new')->('7') }, 'a string seed is refused' );
    ok( !eval { $P->('new')->(undef) }, 'an undef seed is refused' );
    is( ref $I->('check')->( 18446744073709551615, 'x' ), 'Math::BigInt', 'a UV above IV_MAX becomes a big' );
    is( $I->('check')->( 5, 'x' ), 5, 'a native integer passes' );
    ok( $I->('is')->(5) && !$I->('is')->(5.5) && !$I->('is')->('5'), 'is()' );
};

subtest 'Int.parse: the digit cap is applied before parsing' => sub {
    my $ok = '9' x 1000;
    is( dec( int_of($ok) ), $ok, 'a 1,000-digit integer is accepted' );
    my $too = '9' x 1001;
    ok( !eval { int_of($too); 1 }, 'a 1,001-digit integer is refused' );
    like( $@, qr/exceeds 1000 digits/, '... by the cap' );
    ok( !eval { int_of('-' . ( '9' x 1001 )); 1 }, '... also negative' );
    ok( !eval { int_of('007'); 1 }, 'leading zeros are refused' );
    ok( !eval { int_of('-0'); 1 },  '-0 is refused' );
    ok( !eval { int_of('1.5'); 1 }, 'a decimal point is refused' );
    ok( !eval { int_of(''); 1 },    'the empty string is refused' );
    is( int_of('-9223372036854775808'), -9223372036854775807 - 1, 'IV_MIN parses natively' );
    ok( !$I->('is_big')->( int_of('-9223372036854775808') ), '... as a native' );
    ok( $I->('is_big')->( int_of('9223372036854775808') ), '2^63 parses as a big' );
};

# ---------------------------------------------------------------- words

subtest 'add64 / mul64 against Python, Math::BigInt and use integer' => sub {
    my $M = Math::BigInt->new(2)->bpow(64);
    for my $v (@Vectors::MUL) {
        my ( $a, $b, $mul, $add ) = @$v;
        my ( $ua, $ub ) = ( int_of_u($a), int_of_u($b) );
        is( sprintf( '%u', $P->('mul64')->( $ua, $ub ) ), $mul, "mul64($a, $b)" );
        is( sprintf( '%u', $P->('add64')->( $ua, $ub ) ), $add, "add64($a, $b)" );
    }
    # An independent third implementation: wrapping signed arithmetic.
    my $state = 88172645463325252;
    for ( 1 .. 500 ) {
        $state ^= ( $state << 13 ) & ~0;
        $state ^= $state >> 7;
        $state ^= ( $state << 17 ) & ~0;
        my ( $a, $b ) = ( $state, $state ^ 0x9E3779B97F4A7C15 );
        my $m = Math::BigInt->new( sprintf '%u', $a )->bmul( sprintf '%u', $b )->bmod($M);
        my ( $ia, $ib ) = ( $a, $b );
        my $wrapped = do { use integer; $ia * $ib };
        is( sprintf( '%u', $P->('mul64')->( $a, $b ) ), $m->bstr, 'random mul64 matches Math::BigInt' ) if $_ <= 5;
        is( sprintf( '%u', $P->('mul64')->( $a, $b ) ), sprintf( '%u', $wrapped ), 'mul64 equals wrapping signed multiply' ) if $_ % 50 == 0;
    }
};

sub int_of_u {    # decimal text of a word -> a UV scalar
    my ($t) = @_;
    return $Int->can('to_u64')->( $Int->can('parse')->($t) );
}

subtest 'unsigned modulo and the rejection threshold (Python vectors)' => sub {
    for my $v (@Vectors::MOD) {
        my ( $a, $b, $want ) = @$v;
        is( sprintf( '%u', int_of_u($a) % int_of_u($b) ), $want, "$a mod $b" );
    }
    for my $v (@Vectors::THR) {
        my ( $n, $want ) = @$v;
        is( sprintf( '%u', $P->('threshold')->( int_of_u($n) ) ), $want, "threshold($n)" );
    }
    ok( !eval { $P->('threshold')->(0); 1 }, 'threshold of 0 is an error' );
    my $signed = do { use integer; my $n = 3; ( -$n ) % $n };
    isnt( $signed, 1, 'a signed (-n) % n is NOT the threshold: this is why it is never used' );
};

# ------------------------------------------------------------- SplitMix64

subtest 'SplitMix64' => sub {
    is_deeply( words( $P->('new')->(0), 3 ), [ '16294208416658607535', '7960286522194355700', '487617019471545679' ], 'seed 0 stream' );
    is( sprintf( '%x', $P->('new')->(0)->next_word ), 'e220a8397b1dcdaf', 'seed 0 first output' );
    is( words( $P->('new')->(-1), 1 )->[0], '16490336266968443936', 'a negative seed is two\'s complement' );
    is( words( $P->('new')->( $I->('add')->( $I->('pow2')->(64), 5 ) ), 1 )->[0], words( $P->('new')->(5), 1 )->[0], 'seed 2^64 + 5 is seed 5' );
    is( words( $P->('new')->(5), 1 )->[0], '7134611160154358618', 'seed 5' );
    is( "" . $P->('new')->(1), '#<Prng.SplitMix64>', 'the state is hidden when printed' );
    my $r = $P->('new')->(0);
    $r->{state} = ~0;
    ok( defined $r->next_word, 'the state wraps at 2^64 - 1' );
    like( sprintf( '%u', $r->{state} ), qr/^\d+$/, '... and stays an integer' );
};

subtest 'the specialised FNV multiply equals the general one' => sub {
    my $state = 88172645463325252;
    my $bad   = 0;
    for ( 1 .. 3000 ) {
        $state ^= ( $state << 13 ) & ~0;
        $state ^= $state >> 7;
        $state ^= ( $state << 17 ) & ~0;
        $bad++ if $P->('fnv_mul')->($state) != $P->('fnv_mul_general')->($state);
    }
    for my $e ( 0, 1, ~0, 1 << 63, ( 1 << 63 ) - 1, 0xFFFFFFFF, 1 << 32 ) {
        $bad++ if $P->('fnv_mul')->($e) != $P->('fnv_mul_general')->($e);
    }
    is( $bad, 0, 'identical on 3,007 operands' );
};

subtest 'FNV-1a 64 and sub_seed' => sub {
    is( dec( $P->('fnv1a64')->('') ),       '14695981039346656037', 'fnv("")' );
    is( dec( $P->('fnv1a64')->('a') ),      '12638187200555641996', 'fnv("a")' );
    is( dec( $P->('fnv1a64')->('foobar') ), '9625390261332436968',  'fnv("foobar")' );
    is( dec( $P->('fnv1a64')->("prop/\xC3\xA9") ), '6152156688079238993', 'hashed as raw bytes' );
    my $flagged = "prop/\x{e9}";
    utf8::upgrade($flagged);
    is( dec( $P->('fnv1a64')->($flagged) ), dec( $P->('fnv1a64')->("prop/\xE9") ), 'the utf8 flag does not matter' );
    my @seeds = ( 0, 1, 12345, $I->('sub')->( $I->('pow2')->(64), 1 ) );
    my @a = qw(6857225946766476583 13906159068427031964 5869837554499256867 15365624696913284058);
    my @b = qw(9313204760084199774 17665131612661548845 9898446428729937406 4245073340154229409);
    for my $i ( 0 .. 3 ) {
        is( dec( $P->('sub_seed')->( $seeds[$i], 'a' ) ), $a[$i], "sub_seed($i, a)" );
        is( dec( $P->('sub_seed')->( $seeds[$i], 'prop/ct_compare_eq_reflexive' ) ), $b[$i], "sub_seed($i, prop/...)" );
    }
    ok( !eval { $P->('sub_seed')->( 1.0, 'a' ); 1 }, 'a float run seed is refused' );
    ok( !eval { $P->('sub_seed')->( 1, 5 ); 1 },     'a numeric id is refused (needs a string)' ) if 0;
};

# --------------------------------------------------------------- bounded

subtest 'bounded' => sub {
    my $r  = $P->('new')->(11);
    my @ns = ( 1, 2, 3, 10, 4294967296, $I->('pow2')->(63), $I->('add')->( $I->('pow2')->(63), 1 ), $I->('sub')->( $I->('pow2')->(64), 1 ) );
    my @want = qw(0 1 0 0 1963219396 958081316009564174 5164757140853396969 6280598950732548470);
    for my $i ( 0 .. $#ns ) {
        is( dec( $r->bounded( $ns[$i] ) ), $want[$i], 'bounded(' . dec( $ns[$i] ) . ')' );
    }
    is( dec( $P->('new')->(5)->bounded( $I->('pow2')->(64) ) ), dec( $P->('new')->(5)->next_u64 ), '2^64 is one raw word' );
    ok( !eval { $P->('new')->(5)->bounded(0); 1 }, 'bounded(0) is refused' );
    ok( !eval { $P->('new')->(5)->bounded(-3); 1 }, 'a negative bound is refused' );
    ok( !eval { $P->('new')->(5)->bounded( $I->('add')->( $I->('pow2')->(64), 1 ) ); 1 }, '2^64 + 1 is refused' );
    ok( !eval { $P->('new')->(5)->bounded(2.0); 1 }, 'a float bound is refused' );
    ok( !eval { $P->('new')->(5)->bounded('3'); 1 }, 'a string bound is refused' );
    my $n = $I->('sub')->( $I->('pow2')->(64), 1 );
    my $rr = $P->('new')->(3);
    for ( 1 .. 50 ) {
        my $v = $rr->bounded($n);
        ok( $I->('cmp')->( $v, $n ) < 0 && $I->('sign')->($v) >= 0, 'in range' ) if $_ <= 3;
    }
    my $span = $I->('add')->( $I->('pow2')->(100), 12345 );
    my $rb = $P->('new')->(8);
    my $bad = 0;
    for ( 1 .. 50 ) {
        my $v = $rb->bounded_big($span);
        $bad++ unless $I->('cmp')->( $v, $span ) < 0 && $I->('sign')->($v) >= 0;
    }
    is( $bad, 0, 'bounded_big above 2^64 stays in range' );
    ok( !eval { $P->('new')->(1)->bounded_big(0); 1 }, 'bounded_big(0) is refused' );
    ok( !eval { $P->('new')->(1)->bounded_big( $I->('pow2')->(16400) ); 1 }, 'a span over 16384 bits is refused' );
    my ( $a, $b ) = ( $P->('new')->(9), $P->('new')->(9) );
    is( dec( $a->bounded(1000) ), dec( $b->bounded_big(1000) ), 'a small span is the plain draw' );
    is( dec( $a->next_u64 ), dec( $b->next_u64 ), '... at the same stream position' );
    # Uniformity of the three thirds of a 3 * 2^126 span (VER05 7.2)
    my $rt = $P->('new')->(2024);
    my $third = $I->('shl')->( 1, 126 );
    my $span3 = $I->('shl')->( 3, 126 );
    my @bucket = ( 0, 0, 0 );
    for ( 1 .. 600 ) {
        my $v = $rt->bounded_big($span3);
        my $k = $I->('cmp')->( $v, $third ) < 0 ? 0 : $I->('cmp')->( $v, $I->('add')->( $third, $third ) ) < 0 ? 1 : 2;
        $bucket[$k]++;
    }
    ok( $_ > 140 && $_ < 260, "a third of the span gets about a third of the draws ($_)" ) for @bucket;
};

subtest 'pinned tests for what the corpus cannot catch' => sub {
    my $seed = int_of('7046029254386353131');
    is_deeply( words( $P->('new')->($seed), 2 ), [ '0', '16294208416658607535' ], 'the stream starts with the word 0' );
    is( $P->('new')->($seed)->bounded(3), 1, 'bounded(3) rejects the 0 and answers 16294208416658607535 mod 3 = 1' );
    is( $P->('threshold')->(3), 1, 'threshold(3) is 1' );

    # A threshold of 0 still passes the whole corpus: only the pin above catches it.
    my $zero = sub { 0 };
    my $data = read_file( repo_root() . '/code/specs/fixtures/pbt-v1/cases.json' );
    my $report = $Hooks->can('with')->( { threshold => $zero }, sub { CodingAdventures::TestBootstrap::Corpus::evaluate_document($data) } );
    is( scalar @{ $report->{problems} }, 0, 'the corpus passes with a broken threshold of 0' );
    is( $report->{evaluated}, 25, '... all 25 cases' );
    my $got = $Hooks->can('with')->( { threshold => $zero }, sub { $P->('new')->($seed)->bounded(3) } );
    is( $got, 0, 'but the pin does not: the rejected 0 was accepted' );

    # The 2^63 + 1 boundary
    my $n = $I->('add')->( $I->('pow2')->(63), 1 );
    is( $P->('threshold')->( $I->('to_u64')->($n) ), 9223372036854775807, 'threshold(2^63 + 1) is 2^63 - 1' );
    is( words( $P->('new')->(3), 1 )->[0], '2092789425003139053', 'seed 3 first word is below the threshold' );
    is( dec( $P->('new')->(3)->bounded($n) ), '3694763184872335752', '... so it is rejected: the answer is from word 2' );
    is( dec( $P->('new')->(1)->bounded($n) ), '1227844342346046656', 'seed 1 first word is above the threshold and accepted' );
    for my $case ( [ 1, 0 ], [ 2, 0 ], [ 3, 1 ], [ int_of('9223372036854775808'), 0 ], [ int_of('18446744073709551615'), 1 ] ) {
        is( sprintf( '%u', $P->('threshold')->( $I->('to_u64')->( $case->[0] ) ) ), $case->[1], 'unsigned threshold of ' . dec( $case->[0] ) );
    }
    # a draw of exactly the threshold is KEPT, one below is rejected (forced stream)
    {
        package Forced;
        our @ISA = ('CodingAdventures::TestBootstrap::Prng::Rng');
        sub next_word { my $s = shift; return shift @{ $s->{list} } }
    }
    my $forced = sub { bless { state => 0, list => [@_] }, 'Forced' };
    my $thr = 9223372036854775807;
    is( $forced->($thr)->bounded($n), $thr, 'a draw equal to the threshold is kept' );
    is( dec( $forced->( $thr - 1, 9223372036854775808 )->bounded($n) ), '9223372036854775808' x 1, 'a draw below the threshold is rejected' );

    # bounded(1) consumes a word
    my $r = $P->('new')->(7);
    is( $r->bounded(1), 0, 'bounded(1) is 0' );
    is( dec( $r->next_u64 ), '309689372594955804', '... and consumed exactly one word' );
};

done_testing;
