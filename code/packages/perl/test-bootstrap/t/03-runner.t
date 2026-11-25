# The honest runner: verdicts, failure vocabulary, witness re-verification, CI
# redaction (VER05p section 6).
use strict;
use warnings;
use Test::More;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of bounded_by);

my $R    = 'CodingAdventures::TestBootstrap::Runner';
my $G    = 'CodingAdventures::TestBootstrap::Gens';
my $Opts = 'CodingAdventures::TestBootstrap::Runner::Options';
my $F    = 'CodingAdventures::TestBootstrap::FailureIds';

sub opts { return $Opts->new( { ci => 'off', @_ } ) }
sub check { my @a = @_; return bounded_by( 20, sub { $R->can('check')->(@a) } ) }

my $int = $G->can('integer')->( 0, 1000 );
my $le100 = sub { $_[0] <= 100 ? 1 : 0 };

subtest 'a falsified property shrinks to the boundary' => sub {
    my $v = check( 42, $int, $le100, opts() );
    ok( $v->{failed} && !$v->{passed}, 'fails' );
    is( $v->{failure_id}, 'property-counterexample', 'failure id' );
    is( $v->{minimal}, 101, 'minimal witness' );
    ok( $v->{shrink_steps} > 0, 'shrunk' );
    is( dec( $v->{seed} ), '42', 'the seed is reported locally' );
    like( $v->{message}, qr/seed=42.*initial=\d+, minimal=101 after \d+ shrink steps/, 'message names seed, initial and minimal' );
};

subtest 'a passing property' => sub {
    my $v = check( 1, $int, sub { $_[0] <= 1000 ? 1 : 0 }, opts( num_tests => 50 ) );
    ok( $v->{passed} && !$v->{failed}, 'passes' );
    is( $v->{cases_run}, 50, 'all cases ran' );
    ok( !defined $v->{failure_id}, 'no failure id' );
};

subtest 'verdict honesty' => sub {
    my $v = check( 1, $int, sub { 1 }, opts( num_tests => 0 ) );
    is( $v->{failure_id}, 'property-exhausted', 'zero cases is exhausted, never a pass' );
    ok( $v->{failed}, '... and a failure' );
    for my $r ( [ 'returns 0', sub { 0 } ], [ 'returns undef', sub { undef } ], [ 'returns empty', sub { return } ], [ 'returns ""', sub { '' } ], [ 'returns 2', sub { 2 } ], [ 'returns a ref', sub { [1] } ], [ 'returns "yes"', sub { 'yes' } ] ) {
        my $w = check( 1, $int, $r->[1], opts( num_tests => 3 ) );
        is( $w->{failure_id}, 'property-counterexample', $r->[0] . ' falsifies' );
    }
    for my $ok ( [ 'the number 1', sub { 1 } ], [ 'the string "1"', sub { '1' } ], [ '!!1', sub { !!1 } ], [ '1 == 1', sub { 1 == 1 } ] ) {
        my $w = check( 1, $int, $ok->[1], opts( num_tests => 3 ) );
        ok( $w->{passed}, $ok->[0] . ' holds' );
    }
    my $gen_dies = CodingAdventures::TestBootstrap::Gen::new( sub { die "boom\n" } );
    $v = check( 1, $gen_dies, sub { 1 }, opts() );
    is( $v->{failure_id}, 'property-exhausted', 'a generator error is exhausted' );
    my $never = $G->can('filter')->( $int, sub { 0 }, 5 );
    $v = check( 1, $never, sub { 1 }, opts() );
    is( $v->{failure_id}, 'property-exhausted', 'a filter that never accepts is exhausted' );
    like( $v->{message}, qr/filter precondition/, '... and says so' );
    $v = check( 1, $int, sub { die "x\n" }, opts() );
    is( $v->{failure_id}, 'property-panic', 'a dying property is a panic' );
    ok( ( grep { $_ eq 'property-timeout' } @CodingAdventures::TestBootstrap::FailureIds::ALL ) && !$F->can('is_failure_id')->('nope'), 'timeout registered, never produced' );
};

subtest 'witness re-verification: a defective shrinker cannot change the verdict' => sub {
    my %bad = (
        'returns a passing value'  => sub { return ( 5, 3 ) },
        'dies'                     => sub { die "shrinker bug\n" },
        'returns a bad step count' => sub { return ( $_[0], 99999 ) },
        'returns negative steps'   => sub { return ( $_[0], -1 ) },
        'returns a float step'     => sub { return ( $_[0], 1.5 ) },
        'returns nothing'          => sub { return },
        'a non-local exit'         => sub { no warnings; last },
    );
    for my $name ( sort keys %bad ) {
        my $v = check( 42, $int, $le100, opts(), $bad{$name} );
        is( $v->{failure_id}, 'property-counterexample', "$name: still a counterexample" );
        ok( $v->{minimal} > 100, "$name: the reported witness still fails" );
        is( $v->{shrink_steps}, 0, "$name: reported with 0 steps" );
        is( $v->{minimal}, $v->{counterexample}, "$name: it is the original candidate" );
    }
    # a property that dies on smaller inputs: the candidate is not a falsifier
    my $v = check( 42, $int, sub { die "small\n" if $_[0] < 300 && $_[0] > 100; return $_[0] <= 100 ? 1 : 0 }, opts() );
    ok( $v->{failed}, 'a property that dies on smaller inputs still fails' );
    ok( $v->{minimal} == 0 || $v->{minimal} > 100, 'the minimal witness really fails (never a dying input)' ) if defined $v->{minimal};
};

subtest 'CI redaction: on and unset hide the seed and the witness' => sub {
    for my $mode ( 'on', 'unset' ) {
        my $v = check( 42, $int, $le100, $Opts->new( { ci => $mode } ) );
        is( $v->{failure_id}, 'property-counterexample', "$mode: still a counterexample" );
        ok( !defined $v->{seed} && !defined $v->{counterexample} && !defined $v->{minimal}, "$mode: no seed or witness fields" );
        unlike( $v->{message}, qr/seed|initial=|minimal=|\b101\b/, "$mode: the message carries no concrete value" );
        like( $v->{message}, qr/repro-fnv: [0-9a-f]{16}\)/, "$mode: a 16-digit correlation id" );
        like( $v->{digest}, qr/\A[0-9a-f]{16}\z/, "$mode: the digest field" );
    }
    my $a = check( 42, $int, $le100, $Opts->new( { ci => 'on' } ) );
    my $b = check( 42, $int, $le100, $Opts->new( { ci => 'on' } ) );
    is( $a->{digest}, $b->{digest}, 'the digest is deterministic' );
    my $d = check( 42, $int, sub { $_[0] <= 200 ? 1 : 0 }, $Opts->new( { ci => 'on' } ) );
    isnt( $a->{digest}, $d->{digest}, '... and depends on the witness' );
    my $p = check( 1, $int, sub { die "secret-value\n" }, $Opts->new( { ci => 'on' } ) );
    unlike( $p->{message}, qr/secret/, 'a redacted panic names only the kind' );
    like( $p->{message}, qr/\(string\)/, '... which is the kind of the error' );
    my $l = check( 1, $int, sub { die "visible\n" }, opts() );
    like( $l->{message}, qr/string: "visible\\n"/, 'a local panic renders the error structurally' );
    is( $Opts->can('ci_from_value')->(undef), 'off', 'CI unset in the environment is off' );
    is( $Opts->can('ci_from_value')->($_), 'off', "CI='$_' is off" ) for ( '', '0', 'false', ' FALSE ', '  ' );
    is( $Opts->can('ci_from_value')->($_), 'on', "CI='$_' is on" ) for ( '1', 'true', 'yes', 'anything' );
    ok( !eval { $Opts->can('ci_from_value')->( [] ); 1 }, 'a reference is not an environment value' );
};

subtest 'argument validation raises instead of returning a verdict' => sub {
    ok( !eval { $R->can('check')->( 1, $int, 'not code', opts() ); 1 }, 'property must be code' );
    ok( !eval { $R->can('check')->( 1, 'not a gen', sub { 1 }, opts() ); 1 }, 'gen must be a Gen' );
    ok( !eval { $R->can('check')->( 1.5, $int, sub { 1 }, opts() ); 1 }, 'float seed' );
    ok( !eval { $R->can('check')->( 1, $int, sub { 1 }, {} ); 1 }, 'options must be Options' );
    ok( !eval { $R->can('check')->( 1, $int, sub { 1 }, opts(), 'x' ); 1 }, 'shrink must be code' );
    ok( !eval { $Opts->new( { bogus => 1 } ); 1 }, 'unknown option' );
    ok( !eval { $Opts->new( { ci => 'maybe' } ); 1 }, 'bad ci' );
    ok( !eval { $Opts->new( { num_tests => 100001 } ); 1 }, 'num_tests above the cap' );
    ok( !eval { $Opts->new( { num_tests => 1.5 } ); 1 }, 'float num_tests' );
    ok( !eval { $Opts->new( { max_shrink_steps => 10001 } ); 1 }, 'max_shrink_steps above the cap' );
    ok( !eval { $Opts->new( { parent => 'x' } ); 1 }, 'parent must be a scope' );
    ok( !eval { $Opts->new('x'); 1 }, 'options must be a hash' );
    is( $Opts->new->{ci}, 'unset', 'the default mode is unset (fail closed)' );
    is( $Opts->new->{num_tests}, 100, 'the default is 100 cases' );
};

subtest 'a seed of any size is reduced modulo 2^64' => sub {
    my $a = check( -1, $int, $le100, opts() );
    my $b = check( int_of('18446744073709551615'), $int, $le100, opts() );
    is( dec( $a->{seed} ), '18446744073709551615', 'the reported seed is the reduced one' );
    is( $a->{iteration}, $b->{iteration}, 'seed -1 is seed 2^64 - 1' );
};

done_testing;
