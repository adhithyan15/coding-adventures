# Remaining edges: corpus properties and caps, work estimate, clone shapes, redacted
# generator errors, escape limits (VER05p sections 4 to 7).
use strict;
use warnings;
use Test::More;
use Storable ();
use Tie::Array;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of repo_root read_file bounded_by);

my $Corpus = 'CodingAdventures::TestBootstrap::Corpus';
my $Safe   = 'CodingAdventures::TestBootstrap::Safe';
my $Canon  = 'CodingAdventures::TestBootstrap::Canon';
my $G      = 'CodingAdventures::TestBootstrap::Gens';
my $R      = 'CodingAdventures::TestBootstrap::Runner';
my $Opts   = 'CodingAdventures::TestBootstrap::Runner::Options';
my $Budget = 'CodingAdventures::TestBootstrap::Budget';

my ($CASES) = $Corpus->can('parse')->( read_file( repo_root() . '/code/specs/fixtures/pbt-v1/cases.json' ) );

subtest 'corpus: the properties the committed cases do not name' => sub {
    my ($si) = grep { $CASES->[$_]{map}{op}{value} eq 'shrink_int' } 0 .. $#$CASES;
    for my $prop (qw(int_is_even int_abs_le_10)) {
        my $cases = Storable::dclone($CASES);
        my @one = ( $cases->[$si] );
        $one[0]{map}{property}{value} = $prop;
        $one[0]{map}{initial} = { kind => 'object', keys => [ 'kind', 'value' ], map => { kind => { kind => 'string', value => 'int' }, value => { kind => 'string', value => '55' } } };
        $one[0]{map}{min} = $one[0]{map}{target} = $one[0]{map}{initial};    # a degenerate range: min = max = target = 55
        $one[0]{map}{max} = $one[0]{map}{initial};
        my $r = $Corpus->can('evaluate')->( \@one, 0 );
        ok( scalar @{ $r->{problems} }, "$prop is evaluated (the corrupted expectations are reported)" );
    }
};

subtest 'corpus: caps and malformed cases' => sub {
    my @many = map { { kind => 'object', keys => [ 'id', 'op', 'input', 'valid' ], map => { id => { kind => 'string', value => "c$_" }, op => { kind => 'string', value => 'failure_id_valid' }, input => { kind => 'string', value => 'x' }, valid => { kind => 'bool', value => 0 } } } } 1 .. 10001;
    my $r = bounded_by( 60, sub { $Corpus->can('evaluate')->( \@many, 0 ) } );
    ok( ( grep { /exceeds the cap of 10000/ } @{ $r->{problems} } ), 'more than 10,000 cases is a problem' );
    $r = $Corpus->can('evaluate')->( [ 5, { kind => 'object', keys => [], map => {} } ], 0 );
    ok( ( grep { /case is not an object/ || /duplicate or missing case id/ } @{ $r->{problems} } ), 'a non-object case' );
    my $nid = { kind => 'object', keys => ['op'], map => { op => { kind => 'string', value => 'corpus_reject' } } };
    $r = $Corpus->can('evaluate')->( [ { kind => 'object', keys => ['id'], map => { id => { kind => 'string', value => 'a' } } }, $nid ], 0 );
    ok( ( grep { /missing/ } @{ $r->{problems} } ), 'a case without an id' );
};

subtest 'work estimate and clone shapes' => sub {
    is( $Safe->can('size_of')->("abcd"), 4, 'a string counts its bytes' );
    is( $Safe->can('size_of')->( [ 1, 2, 3 ] ), 3, 'a flat array counts its entries' );
    is( $Safe->can('size_of')->( [ "ab", [ 1, 2 ] ] ), 2 + 2 + 2, 'one level deeper strings and containers' );
    is( $Safe->can('size_of')->( { a => "xyz", b => [1] } ), 2 + 3 + 1, 'a hash counts entries and children' );
    is( $Safe->can('size_of')->(5), 0, 'a number has no size' );
    my %big = map { ( $_ => 1 ) } 1 .. 20000;
    cmp_ok( $Safe->can('size_of')->( \%big ), '<=', 10000, 'a huge hash is bounded by the visit allowance' );
    my @hole;
    $hole[3] = 1;
    is( $Canon->can('render')->( \@hole ), '[undef,undef,undef,1]', 'holes render as undef' );
    my ( $copy, $failed ) = $Safe->can('clone')->( \@hole );
    ok( !$failed && @$copy == 4 && !defined $copy->[0], 'a sparse array is cloned' );
    my $sref = \"text";
    ( $copy, $failed ) = $Safe->can('clone')->( [ $sref, \\"deep" ] );
    ok( !$failed && ${ $copy->[0] } eq 'text' && ${ ${ $copy->[1] } } eq 'deep', 'scalar references are cloned' );
    isnt( Scalar::Util::refaddr( $copy->[0] ), Scalar::Util::refaddr($sref), '... not shared' );
    my $shared = { k => 1 };
    ( $copy, $failed ) = $Safe->can('clone')->( [ $shared, $shared ] );
    is( Scalar::Util::refaddr( $copy->[0] ), Scalar::Util::refaddr( $copy->[1] ), 'shared substructure stays shared' );
    my $big = int_of('18446744073709551616');
    ( $copy, $failed ) = $Safe->can('clone')->( [$big] );
    ok( !$failed && Scalar::Util::refaddr( $copy->[0] ) != Scalar::Util::refaddr($big) && dec( $copy->[0] ) eq dec($big), 'a big is copied, not shared' );
    my $deep = [];
    my $t    = $deep;
    for ( 1 .. 100 ) { my $n = []; push @$t, $n; $t = $n }
    ( undef, $failed ) = $Safe->can('clone')->($deep);
    ok( $failed, 'a structure deeper than 64 fails closed' );
    my @c; push @c, \@c;
    ( $copy, $failed ) = $Safe->can('clone')->( \@c );
    ok( !$failed && $copy->[0] == $copy, 'a cycle is cloned as a cycle' );
    ( $copy, $failed ) = $Safe->can('clone')->( [ sub { 1 }, \*STDOUT, qr/x/ ] );
    ok( !$failed && ref $copy->[0] eq 'CODE', 'code and globs are shared' );
    my $cl = $Safe->can('array_copy')->( [ 1, 2 ], 'x' );
    is_deeply( $cl, [ 1, 2 ], 'array_copy' );
    ok( !eval { $Safe->can('array_copy')->( {}, 'x' ); 1 }, 'array_copy refuses a hash' );
    ok( !eval { $Safe->can('hash_copy')->( [], 'x' ); 1 }, 'hash_copy refuses an array' );
    tie my @ta, 'Tie::StdArray';
    ok( !eval { $Safe->can('array_copy')->( \@ta, 'x' ); 1 }, 'array_copy refuses a tied array' );
};

subtest 'nested checks with hash witnesses charge their size' => sub {
    my $guard = $Budget->can('root')->( 1000, 100000 );
    my $gen = CodingAdventures::TestBootstrap::Gen::new( sub { return { a => 'xyz', b => [ 1, 2 ] } } );
    my $v = $R->can('check')->( 1, $gen, sub { 1 }, $Opts->new( { ci => 'off', parent => $guard->scope, num_tests => 3 } ) );
    ok( $v->{passed}, 'passes' );
    cmp_ok( $guard->scope->work, '>=', 3 * 6, 'the sampled values were charged as work (' . $guard->scope->work . ')' );
};

subtest 'redacted generator errors name only the kind' => sub {
    my $gen = CodingAdventures::TestBootstrap::Gen::new( sub { die "secret generator message\n" } );
    my $v = $R->can('check')->( 1, $gen, sub { 1 }, $Opts->new( { ci => 'on' } ) );
    is( $v->{failure_id}, 'property-exhausted', 'exhausted' );
    like( $v->{message}, qr/generator error at iteration 1 \(string\)/, 'the kind only' );
    unlike( $v->{message}, qr/secret/, 'no message content' );
    $v = $R->can('check')->( 1, $gen, sub { 1 }, $Opts->new( { ci => 'off' } ) );
    like( $v->{message}, qr/secret generator message/, 'local mode describes the error' );
};

subtest 'escape, shrink_nested and u64 shrinking' => sub {
    like( $Canon->can('escape')->( "\xFF\xFE\xFD\xFC", 0, 2 ), qr/^0xfffe$/, 'an invalid string is hex-clipped to the limit' );
    my $gen = $G->can('integer')->( 0, 100 );
    is_deeply( [ $gen->shrink_nested( 50, sub { $_[0] >= 10 ? 1 : 0 }, 100, undef ) ], [ 10, 5 ], 'shrink_nested with no scope opens its own' );
    my ( $m, $s ) = $G->can('u64')->()->shrink( int_of('18446744073709551615'), 1000, sub { CodingAdventures::TestBootstrap::Int::cmp( $_[0], 1000 ) >= 0 ? 1 : 0 } );
    is( dec($m), '1000', 'a u64 shrinks across the 2^63 boundary' );
    ok( !eval { $gen->shrink( 5, 10, 'not code' ); 1 }, 'shrink needs a falsifies code reference' );
    ok( !eval { CodingAdventures::TestBootstrap::Gen::new('x'); 1 }, 'a sampler must be code' );
    ok( !eval { CodingAdventures::TestBootstrap::Gen::new( sub { 1 }, 'x' ); 1 }, 'a shrinker must be code' );
    ok( CodingAdventures::TestBootstrap::Gen::exhausted_error('m')->message eq 'm', 'the exhausted error carries its message' );
};

done_testing;
