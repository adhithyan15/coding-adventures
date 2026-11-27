# Regression tests for the security review (VER05p sections 3 to 5): NaN/Inf bigs,
# planted `can`, unnormalized bigs, root leaks, `redo`, filter predicates, strict
# "holds", and user code steering engine state.
use strict;
use warnings;
use Test::More;
use Math::BigInt;
use Scalar::Util ();
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of bounded_by);

my $Int  = 'CodingAdventures::TestBootstrap::Int';
my $Canon = 'CodingAdventures::TestBootstrap::Canon';
my $R    = 'CodingAdventures::TestBootstrap::Runner';
my $G    = 'CodingAdventures::TestBootstrap::Gens';
my $Safe = 'CodingAdventures::TestBootstrap::Safe';
my $Opts = 'CodingAdventures::TestBootstrap::Runner::Options';
my $B    = 'CodingAdventures::TestBootstrap::Budget';

sub opts { return $Opts->new( { ci => 'off', @_ } ) }
sub check { my @a = @_; return bounded_by( 20, sub { CodingAdventures::TestBootstrap::Runner::check(@a) } ) }
my $int = CodingAdventures::TestBootstrap::Gens::integer( 0, 1000 );

subtest 'M1: NaN and infinity bigs never hang' => sub {
    for my $c ( [ 'NaN', Math::BigInt->bnan, 'NaN' ], [ 'inf', Math::BigInt->binf, 'inf' ], [ '-inf', Math::BigInt->binf('-'), '-inf' ] ) {
        my $v = $c->[1];
        is( bounded_by( 5, sub { CodingAdventures::TestBootstrap::Canon::render($v) } ), $c->[2], "Canon renders $c->[0]" );
        is( bounded_by( 5, sub { CodingAdventures::TestBootstrap::Values::show($v) } ), $c->[2], "show renders $c->[0]" );
        ok( !eval { bounded_by( 5, sub { CodingAdventures::TestBootstrap::Int::bit_length($v) } ); 1 }, "bit_length refuses $c->[0]" );
        my $gen = CodingAdventures::TestBootstrap::Gen::new( sub { return Math::BigInt->new($v) } );
        for my $mode ( 'on', 'off' ) {
            my $r = check( 1, $gen, sub { 0 }, $Opts->new( { ci => $mode } ) );
            is( $r->{failure_id}, 'property-counterexample', "$c->[0] witness, ci $mode: a verdict" );
        }
        my $e = check( 1, $int, sub { die Math::BigInt->new($v) }, opts() );
        is( $e->{failure_id}, 'property-panic', "$c->[0] as an error value: a panic verdict" );
        ok( !eval { CodingAdventures::TestBootstrap::Int::check( $v, 'x' ); 1 }, "Int::check refuses $c->[0]" );
    }
    my $z = check( 1, $int, sub { my $q = Math::BigInt->new(1)->bdiv(0); return ( $q->is_nan || $q->is_inf ) ? 0 : 1 }, opts( num_tests => 2 ) );
    is( $z->{failure_id}, 'property-counterexample', 'a property dividing by zero is fine' );
    ok( CodingAdventures::TestBootstrap::Int::bit_length( Math::BigInt->new( '9' x 4000 ) ) > 13000, 'bit_length of a large finite big still works' );
};

subtest 'M2: a planted `can` (or any new sub) does not steer the engine' => sub {
    no warnings 'once';
    local *CodingAdventures::TestBootstrap::Safe::can      = sub { return sub { 1 } };
    local *CodingAdventures::TestBootstrap::Int::can       = sub { return sub { 0 } };
    local *CodingAdventures::TestBootstrap::Runner::can    = sub { return sub { 1 } };
    local *CodingAdventures::TestBootstrap::Shrink::can    = sub { return sub { 1 } };
    my $v = check( 42, $int, sub { return 0 }, opts() );
    is( $v->{failure_id}, 'property-counterexample', 'an always-false property still fails' );
    $v = check( 42, $int, sub { $_[0] <= 100 ? 1 : 0 }, opts() );
    is( $v->{minimal}, 101, '... with the right minimal witness' );
    is( CodingAdventures::TestBootstrap::Corpus::evaluate_document( TBHelper::read_file( TBHelper::repo_root() . '/code/specs/fixtures/pbt-v1/cases.json' ) )->{evaluated}, 25, 'and the corpus still evaluates 25' );
    my @src = glob("$FindBin::Bin/../lib/CodingAdventures/TestBootstrap/*.pm");
    my $uses = 0;
    for my $f (@src) { open my $fh, '<', $f or die; local $/; my $t = <$fh>; $uses++ while $t =~ /->can\(/g }
    is( $uses, 0, 'no engine source uses ->can' );
};

subtest 'user code cannot steer hooks or the ambient scope' => sub {
    my $v = check( 42, $int, sub { $CodingAdventures::TestBootstrap::Hooks::H{unit} = sub { 0 }; $CodingAdventures::TestBootstrap::Budget::CURRENT = undef; return $_[0] <= 100 ? 1 : 0 }, opts() );
    is( $v->{minimal}, 101, 'the shrink result is unchanged' );
    is( $CodingAdventures::TestBootstrap::Hooks::H{unit}, \&CodingAdventures::TestBootstrap::Hooks::default_unit, 'the hook was put back' );
};

subtest 'LOW-3: a caller big is normalized, copied, checked' => sub {
    is( CodingAdventures::TestBootstrap::Int::cmp( Math::BigInt->new(5), 100 ), -1, 'cmp of an unnormalized small big is correct' );
    is( CodingAdventures::TestBootstrap::Int::cmp( 100, Math::BigInt->new(5) ), 1, '... either way round' );
    my $n = CodingAdventures::TestBootstrap::Int::check( Math::BigInt->new(5), 'x' );
    ok( !ref $n && $n == 5, 'check normalizes a small big to a native integer' );
    ok( !eval { CodingAdventures::TestBootstrap::Int::check( Math::BigInt->new( '9' x 6000 ), 'x' ); 1 }, 'a huge big is refused by its digit count' );
    my $hi = Math::BigInt->new('100000000000000000000');
    my $gen = CodingAdventures::TestBootstrap::Gens::integer( $hi->copy->bneg, $hi );
    my $before = [ map { dec($_) } @{ CodingAdventures::TestBootstrap::Gens::gen_bounded( 1, $hi->copy->bneg, $hi, 3 ) } ];
    $hi->badd(5);    # the caller mutates its object after construction
    my $rng = CodingAdventures::TestBootstrap::Prng::new(1);
    my $got = [ map { dec( $gen->sample($rng) ) } 1 .. 3 ];
    is_deeply( $got, $before, 'the generator is unchanged by a later mutation' );
    my $c = CodingAdventures::TestBootstrap::Int::check( Math::BigInt->new('100000000000000000000'), 'x' );
    isnt( Scalar::Util::refaddr($c), undef, 'a big stays a big' );
};

subtest 'LOW-4: a root node is freed with its guard' => sub {
    my $weak;
    {
        my $g = CodingAdventures::TestBootstrap::Budget::root( 10, 10 );
        $weak = $g->scope;
        Scalar::Util::weaken($weak);
        ok( defined $weak, 'alive while the guard lives' );
    }
    ok( !defined $weak, 'freed after the guard (no self-reference leak)' );
    my $w2;
    check( 1, $int, sub { 1 }, opts( num_tests => 1 ) );
    {
        my $g = CodingAdventures::TestBootstrap::Budget::open_check( undef, 1, 1 );
        $w2 = $g->scope;
        Scalar::Util::weaken($w2);
    }
    ok( !defined $w2, 'a check scope is freed too' );
};

subtest 'LOW-5: redo cannot re-run user code' => sub {
    my $runs = 0;
    my ( $st, $v ) = CodingAdventures::TestBootstrap::Safe::call( sub { no warnings; $runs++; redo } );
    is( $st, 'escaped', 'redo is an escape' );
    is( $runs, 1, 'the code ran once' );
    $runs = 0;
    my $r = check( 1, $int, sub { no warnings; $runs++; redo }, opts( num_tests => 3 ) );
    is( $r->{failure_id}, 'property-panic', 'a property that redoes is a panic, never a pass' );
    is( $runs, 1, 'and ran once' );
};

subtest 'LOW-6: filter predicates run in scalar context through Safe::call' => sub {
    my $ctx;
    my $gen = CodingAdventures::TestBootstrap::Gens::filter( $int, sub { $ctx = wantarray; return ( 0, 1 ) } , 3 );
    eval { $gen->sample( CodingAdventures::TestBootstrap::Prng::new(1) ) };
    ok( defined $ctx && !$ctx, 'the predicate saw scalar context' );
    my $dies = CodingAdventures::TestBootstrap::Gens::filter( $int, sub { die "pred\n" } );
    my $v = check( 1, $dies, sub { 1 }, opts() );
    is( $v->{failure_id}, 'property-exhausted', 'a dying predicate is a generator error' );
    my $esc = CodingAdventures::TestBootstrap::Gens::filter( $int, sub { no warnings; last } );
    $v = check( 1, $esc, sub { 1 }, opts() );
    is( $v->{failure_id}, 'property-exhausted', 'a predicate that escapes is exhausted' );
};

subtest 'holds: exactly one, as an integer, an exact 1.0 or the string "1"' => sub {
    my $h = sub { return CodingAdventures::TestBootstrap::Safe::holds( $_[0] ) };
    ok( $h->(1), 'the number 1' );
    ok( $h->('1'), '"1"' );
    ok( $h->( !!1 ), '!!1' );
    ok( $h->( 1 == 1 ), '1 == 1' );
    ok( $h->(1.0), '1.0 is exactly one' );
    ok( $h->( 2 / 2 ), '2/2' );
    ok( $h->( 1**5 ), '1**5' );
    ok( $h->( sqrt(1) ), 'sqrt(1)' );
    my $s = '1';
    my $x = $s + 0.5;
    ok( $h->($s), 'a "1" used in float maths' );
    ok( !$h->( 1 + 2**-52 ), '1 + 2**-52 falsifies' );
    ok( !$h->(1.0000000000000002), '1.0000000000000002 falsifies' );
    ok( !$h->('1.0'), '"1.0" falsifies' );
    ok( !$h->(' 1'), '" 1"' );
    ok( !$h->('1abc'), '"1abc"' );
    ok( !$h->( Scalar::Util::dualvar( 2, '1' ) ), 'dualvar(2, "1")' );
    ok( !$h->( Scalar::Util::dualvar( 0, '1' ) ), 'dualvar(0, "1")' );
    ok( !$h->( Scalar::Util::dualvar( 1, 'x' ) ), 'dualvar(1, "x")' );
    ok( !$h->(2), 'the number 2' );
    ok( !$h->(0), 'the number 0' );
    ok( !$h->(''), '""' );
    ok( !$h->(undef), 'undef' );
    ok( !$h->( [1] ), 'a reference' );
    ok( !$h->( Math::BigInt->new(1) ), 'a Math::BigInt 1' );
    my $n = 1;
    my $str = "$n";
    ok( $h->($n), 'an integer that has been stringified' );
    {
        package TieOne; sub TIESCALAR { bless [] } sub FETCH { 1 }
    }
    tie my $t, 'TieOne';
    ok( !$h->($t), 'a tied scalar never holds' );
};

subtest 'NaN and infinity are not comparable and never equal a number' => sub {
    my $nan = Math::BigInt->bnan;
    my $inf = Math::BigInt->binf;
    for my $f (qw(cmp eq le)) {
        no strict 'refs';
        ok( !eval { &{"CodingAdventures::TestBootstrap::Int::$f"}( $nan, 5 ); 1 }, "$f(NaN, 5) refuses" );
        ok( !eval { &{"CodingAdventures::TestBootstrap::Int::$f"}( 5, $inf ); 1 }, "$f(5, inf) refuses" );
    }
    my $same = \&CodingAdventures::TestBootstrap::Values::same;
    ok( !$same->( $nan, 5 ), 'same(NaN, 5) is false' );
    ok( !$same->( $nan, $nan ), 'same(NaN, NaN) is false' );
    ok( !$same->( $inf, int_of('18446744073709551616') ), 'same(inf, a big) is false' );
    ok( !$same->( $inf, $inf ), 'same(inf, inf) is false' );
};

done_testing;
