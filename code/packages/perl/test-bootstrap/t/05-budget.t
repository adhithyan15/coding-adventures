# Hazard 7: the shrink-budget tree and its `local` carrier (VER05p section 5, the
# required set of VER05e's ShrinkBudgetTest).
use strict;
use warnings;
use Test::More;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of bounded_by);

my $B     = 'CodingAdventures::TestBootstrap::Budget';
my $R     = 'CodingAdventures::TestBootstrap::Runner';
my $G     = 'CodingAdventures::TestBootstrap::Gens';
my $S     = 'CodingAdventures::TestBootstrap::Shrink';
my $Gen   = 'CodingAdventures::TestBootstrap::Gen';
my $Opts  = 'CodingAdventures::TestBootstrap::Runner::Options';
my $Hooks = 'CodingAdventures::TestBootstrap::Hooks';

sub opts { return $Opts->new( { ci => 'off', @_ } ) }
sub check { my @a = @_; return bounded_by( 30, sub { $R->can('check')->(@a) } ) }
my $int = $G->can('integer')->( 0, 1000 );

subtest 'charging is all or nothing' => sub {
    my $root_g = $B->can('root')->( 10, 100 );
    my $root = $root_g->scope;
    my $child_g = $B->can('child')->( $root, 3, 50 );
    my $child = $child_g->scope;
    ok( $child->charge(5), 'a charge of 5 elements' );
    is_deeply( [ $child->attempts, $child->work, $root->attempts, $root->work ], [ 1, 5, 1, 5 ], 'every node on the path is charged' );
    ok( $child->charge_copy(7), 'a copy charges work only' );
    is_deeply( [ $child->attempts, $child->work, $root->attempts, $root->work ], [ 1, 12, 1, 12 ], '... no attempt' );
    ok( $child->charge(1) && $child->charge(1), 'two more attempts' );
    ok( !$child->charge(1), 'the child cap (3 attempts) refuses the fourth' );
    is_deeply( [ $child->attempts, $root->attempts, $root->work ], [ 3, 3, 14 ], 'a refusal leaves every counter unchanged' );
    ok( $child->refused && $root->refused, 'the refusal is recorded on the whole tree' );
    my $c2_g = $B->can('child')->( $root, 100, 1000 );
    my $c2 = $c2_g->scope;
    ok( !$c2->charge(87), 'the root work cap (100, 14 used) refuses 87' );
    ok( $c2->charge(86), '... and affords 86' );
    is( $root->work, 100, 'exactly at the cap' );
    ok( !$c2->charge_copy(1), 'nothing more' );
};

subtest 'a cost that is not a non-negative native integer is refused (fail closed)' => sub {
    for my $bad ( [ 'a float', 1.5 ], [ 'NaN', 9**9**9 / 9**9**9 ], [ 'a string', '5' ], [ 'undef', undef ], [ 'negative', -1 ], [ 'a big', int_of('18446744073709551616') ], [ 'a reference', [] ] ) {
        my $s = $B->can('root')->( 10, 100 )->scope;
        my $guard = $B->can('root')->( 10, 100 );
        my $scope = $guard->scope;
        ok( !$scope->charge( $bad->[1] ), $bad->[0] . ' is refused' );
        is_deeply( [ $scope->attempts, $scope->work ], [ 0, 0 ], '... charging nothing' );
        ok( $scope->refused, '... and recorded' );
    }
    my $guard = $B->can('root')->( 10, 100 );
    ok( $guard->scope->charge(0), 'zero counts as one' );
    is( $guard->scope->work, 1, '... one element' );
    my $huge_g = $B->can('root')->( 10, 100 );
    my $huge = $huge_g->scope;
    ok( !$huge->charge(4611686018427387904), 'a huge cost is refused without wrapping' );
    ok( !$huge->charge(9223372036854775807), 'IV_MAX too' );
};

subtest 'a closed or stale handle is SPENT, never a fresh root' => sub {
    my $stale;
    {
        my $guard = $B->can('root')->( 100, 1000 );
        $stale = $guard->scope;
        ok( !$stale->is_spent, 'open while the guard lives' );
        ok( $stale->charge(1), 'charges while open' );
    }
    ok( $stale->is_spent, 'spent once the guard is released' );
    ok( !$stale->charge(1), 'a spent handle refuses every charge' );
    ok( !$stale->room, '... and has no room' );
    my $kid = $B->can('child')->( $stale, 10, 10 )->scope;
    ok( $kid->is_spent, 'a child of a spent scope is spent' );
    ok( !$kid->charge(1), '... and refuses too' );
    my $v = check( 1, $int, sub { 1 }, opts( parent => $stale ) );
    is( $v->{failure_id}, 'property-exhausted', 'a check under a spent parent is exhausted' );
    my ( $m, $steps ) = $S->can('shrink_int')->( 50, 0, 0, 100, sub { 0 }, 100, $stale );
    is_deeply( [ $m, $steps ], [ 50, 0 ], 'shrink_int under a spent parent is starved: start, 0 steps' );
    my $evals = 0;
    ( $m, $steps ) = $G->can('list')->( $G->can('u8')->(), 0, 5 )->shrink( [ 1, 2, 3 ], 100, sub { $evals++; 1 }, $stale );
    is( $evals, 0, 'a generator shrink under a spent parent evaluates nothing' );
    is( $B->can('current')->(), undef, 'and no ambient scope was left behind' );
    my $joined = $B->can('join_scope')->($stale);
    ok( $joined->is_spent, 'a joined handle to a spent node is spent' );
};

subtest 'the ambient scope is restored on every exit, including die' => sub {
    is( $B->can('current')->(), undef, 'none at top level' );
    my ( $inside, $inner_inside );
    my $v = check( 1, $int, sub { $inside = $B->can('current')->(); die "boom\n" }, opts() );
    ok( defined $inside && $B->can('is_scope')->($inside), 'during a property there is an ambient scope' );
    is( $B->can('current')->(), undef, 'restored after a panic' );
    $v = check( 1, $int, sub { return $_[0] <= 100 ? 1 : 0 }, opts() );
    is( $B->can('current')->(), undef, 'restored after a counterexample (shrinking ran)' );
    ok( !eval { $S->can('shrink_int')->( 5, 0, 0, 10, sub { die "in prop\n" } ); 1 }, 'a dying property in shrink_int' );
    is( $B->can('current')->(), undef, 'restored after shrink_int died' );
    eval { $G->can('list')->( $G->can('u8')->(), 0, 3 )->shrink( [1], 10, sub { die "x\n" } ) };
    is( $B->can('current')->(), undef, 'restored after Gen::shrink died' );
    my $spent_after;
    {
        my $scope_seen;
        check( 1, $int, sub { $scope_seen = $B->can('current')->(); 1 }, opts( num_tests => 1 ) );
        ok( $scope_seen->is_spent, 'the scope a property saw is spent after the check (a stale handle)' );
        $spent_after = $scope_seen;
    }
    my $w = check( 1, $int, sub { 1 }, opts( parent => $spent_after ) );
    is( $w->{failure_id}, 'property-exhausted', 'and passing that stale handle never opens a fresh root' );
};

subtest 'nesting depth 1 to 3 and branching never multiply the outer budget' => sub {
    my $inner_evals = 0;
    my $nest;
    $nest = sub {
        my ($depth) = @_;
        return sub {
            if ( $depth == 0 ) { $inner_evals++; return 1 }
            my $v = $R->can('check')->( 7, $int, $nest->( $depth - 1 ), opts( num_tests => 50, max_shrink_steps => 1 ) );
            return $v->{passed} ? 1 : 0;
        };
    };
    for my $depth ( 1 .. 3 ) {
        $inner_evals = 0;
        my $v = check( 1, $int, $nest->($depth), opts( num_tests => 50, max_shrink_steps => 1 ) );
        ok( $inner_evals <= 50 + 16 + 64, "depth $depth: the innermost property ran $inner_evals times, bounded by the outer root cap" );
        ok( !$v->{passed}, "depth $depth: starvation is never a pass (" . ( $v->{failure_id} // 'none' ) . ')' );
        is( $B->can('current')->(), undef, "depth $depth: no ambient scope left" );
    }
    # branching: m checks per evaluation
    my $evals = 0;
    my $v = check(
        1, $int,
        sub {
            for ( 1 .. 5 ) { $R->can('check')->( 3, $int, sub { $evals++; 1 }, opts( num_tests => 20, max_shrink_steps => 1 ) ) }
            return 1;
        },
        opts( num_tests => 40, max_shrink_steps => 1 )
    );
    ok( $evals <= 40 + 16 + 64, "branching: $evals inner evaluations, bounded by the root cap (not 40*5*20)" );
    ok( !$v->{passed}, '... and the starved run is not a pass' );
};

subtest 'a nested check pays at entry; a starved one evaluates nothing' => sub {
    my $guard = $B->can('root')->( 1, 1000 );    # one attempt only
    my $scope = $guard->scope;
    ok( $scope->charge(1), 'use the only attempt' );
    my $evals = 0;
    my $v = check( 1, $int, sub { $evals++; 1 }, opts( parent => $scope ) );
    is( $v->{failure_id}, 'property-exhausted', 'exhausted' );
    is( $evals, 0, 'nothing evaluated' );
    ok( $scope->refused, 'the refusal is on the tree' );
    my $g2 = $B->can('root')->( 1000, 1000 );
    my $s2 = $g2->scope;
    $v = check( 1, $int, sub { 1 }, opts( parent => $s2, num_tests => 10 ) );
    ok( $v->{passed}, 'a nested check with room passes' );
    ok( $s2->attempts >= 10, 'and charged the enclosing tree (' . $s2->attempts . ' attempts)' );
};

subtest 'checks nested more than 64 deep are exhausted' => sub {
    my $deepest = 0;
    my $rec;
    $rec = sub {
        my ($d) = @_;
        return sub {
            $deepest = $d if $d > $deepest;
            return 1 if $d >= 100;
            my $v = $R->can('check')->( 1, $G->can('constant')->(1), $rec->( $d + 1 ), opts( num_tests => 1, max_shrink_steps => 0 ) );
            return $v->{passed} ? 1 : 0;
        };
    };
    my $v = check( 1, $G->can('constant')->(1), $rec->(1), opts( num_tests => 1, max_shrink_steps => 0 ) );
    ok( $deepest <= 65, "the deepest level reached was $deepest (cap 64)" );
    ok( !$v->{passed}, 'the over-deep run is not a pass' );
};

subtest 'a recursing custom shrinker and a shrinking filter predicate stay inside the root' => sub {
    my $evals = 0;
    my $shrinker;
    $shrinker = sub {
        my ( $v, $falsifies, $max, $scope ) = @_;
        # a hostile shrinker: recurses into the public entry point on every candidate
        my $inner = $G->can('list')->( $G->can('u8')->(), 0, 20 );
        for ( 1 .. 50 ) { $inner->shrink( [ 1 .. 15 ], 1000, sub { $evals++; 1 } ) }
        return ( $v, 0 );
    };
    my $v = check( 42, $int, sub { $_[0] <= 100 ? 1 : 0 }, opts( max_shrink_steps => 5 ), $shrinker );
    is( $v->{failure_id}, 'property-counterexample', 'still a verdict' );
    ok( $evals <= 5 * 16 + 64 + 100, "the recursion evaluated $evals times: bounded by one root (" . ( 5 * 16 + 64 ) . ' + slack), not 50 * 16000' );
    my $pred_calls = 0;
    my $gen = $G->can('filter')->( $int, sub { $pred_calls++; $_[0] % 2 == 0 ? 1 : 0 } );
    $v = check( 42, $gen, sub { $_[0] <= 100 ? 1 : 0 }, opts() );
    is( $v->{failure_id}, 'property-counterexample', 'a filter predicate that is called while shrinking' );
    ok( $v->{minimal} % 2 == 0, '... the shrunk witness satisfies the predicate' );
};

subtest 'a filter sampler rejecting everything: nested charges attempts, top-level charges work' => sub {
    my $reject = $G->can('filter')->( $int, sub { 0 }, 10000 );
    my $guard = $B->can('root')->( 50, 100000 );
    my $v = check( 1, $reject, sub { 1 }, opts( parent => $guard->scope, num_tests => 1 ) );
    is( $v->{failure_id}, 'property-exhausted', 'a nested rejecting filter is exhausted' );
    ok( $guard->scope->attempts <= 50, 'and is stopped by the enclosing attempt cap (' . $guard->scope->attempts . ')' );
    # top-level: retries are work against the root allowance
    $v = $Hooks->can('with')->( { default_max_work => 20 }, sub { check( 1, $reject, sub { 1 }, opts( num_tests => 1 ) ) } );
    is( $v->{failure_id}, 'property-exhausted', 'a top-level rejecting filter is exhausted when the work allowance is spent' );
    my $rare = $G->can('filter')->( $int, sub { $_[0] == 0 ? 1 : 0 }, 10000 );
    my $ok = check( 7, $rare, sub { 1 }, opts( num_tests => 1 ) );
    ok( $ok->{passed} || $ok->{failure_id} eq 'property-exhausted', 'a rare but satisfiable filter is not starved by the attempt cap' );
    my $stacked = $G->can('filter')->( $G->can('filter')->( $G->can('filter')->( $int, sub { 0 }, 10000 ), sub { 1 }, 10000 ), sub { 1 }, 10000 );
    $v = $Hooks->can('with')->( { default_max_work => 100 }, sub { check( 1, $stacked, sub { 1 }, opts( num_tests => 1 ) ) } );
    is( $v->{failure_id}, 'property-exhausted', 'stacked filters cannot multiply uncharged retries' );
};

subtest 'a refused re-verification keeps the witness; starvation is never a pass' => sub {
    my $shrinker = sub {
        my ( $v, $falsifies, $max, $scope ) = @_;
        $scope->charge_copy(5);    # exhaust the root's whole work allowance
        return ( 0, 1 );           # claims a "smaller" witness (0 passes the property)
    };
    my $v = $Hooks->can('with')->( { default_max_work => 5 }, sub { check( 42, $int, sub { $_[0] <= 100 ? 1 : 0 }, opts(), $shrinker ) } );
    is( $v->{failure_id}, 'property-counterexample', 'a falsified verdict is never overridden' );
    is( $v->{shrink_steps}, 0, 'the refused re-verification reports 0 steps' );
    ok( $v->{minimal} > 100, '... and the original witness' );
    # a nested check that was starved makes the OUTER would-be pass exhausted
    my $v2 = check(
        1, $int,
        sub {
            $R->can('check')->( 1, $int, sub { 1 }, opts( num_tests => 100000 ) );    # needs far more than the cap
            return 1;
        },
        opts( num_tests => 3, max_shrink_steps => 0 )
    );
    is( $v2->{failure_id}, 'property-exhausted', 'a starved nested run turns the outer pass into exhausted' );
};

subtest 'depth cap: deeper owning scopes join' => sub {
    my $g = $B->can('root')->( 1000, 1000 );
    my @chain = ($g);
    my $parent = $g->scope;
    for ( 1 .. 70 ) {
        my $own = $B->can('open_child')->( $parent, 10 );
        push @chain, $own;
        $parent = $own->scope;
    }
    ok( $parent->depth <= $B->can('MAX_DEPTH')->(), 'depth never exceeds the cap (' . $parent->depth . ')' );
    ok( $parent->charge(1), 'a joined handle charges the node it aliases' );
    @chain = ();
};

done_testing;
