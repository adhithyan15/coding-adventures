package CodingAdventures::TestBootstrap::Runner;

# The honest runner (VER05 7.1, VER05e 5, VER05p section 6).
#
#   * zero cases evaluated is `property-exhausted`, never a pass;
#   * a generator error (any value thrown while sampling) is `property-exhausted`;
#   * a property that dies (ANY value, `undef` and objects included) or leaves
#     through `last` / `next` is `property-panic`;
#   * a property holds ONLY when it returns exactly one (see Safe::holds): `0`, `undef`, `""`,
#     a reference and "forgot to return" all falsify, so a property cannot pass
#     vacuously;
#   * the reported minimal witness is RE-VERIFIED: if the shrinker dies, returns
#     something that no longer fails, a value the property dies on, or a
#     nonsensical step count, the original candidate is reported with 0 steps;
#   * redacted mode never emits the witness or the seed; an UNSET mode is redacted
#     (fail closed);
#   * the call always returns a verdict (invalid arguments `die` instead).
#
# Errors are classified by `eval` and by `blessed`/`reftype` alone (see Safe.pm):
# the engine never stringifies, or reads a field of, an error value.
#
# There is no wall-clock timeout: `property-timeout` is registered and never
# produced, so a property that loops forever hangs the run.

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int        ();
use CodingAdventures::TestBootstrap::Prng       ();
use CodingAdventures::TestBootstrap::Limits     ();
use CodingAdventures::TestBootstrap::FailureIds ();
use CodingAdventures::TestBootstrap::Budget     ();
use CodingAdventures::TestBootstrap::Gen        ();
use CodingAdventures::TestBootstrap::Canon      ();
use CodingAdventures::TestBootstrap::Safe       ();

my $Int = 'CodingAdventures::TestBootstrap::Int';
my $Safe = 'CodingAdventures::TestBootstrap::Safe';

# ---------------------------------------------------------------- Options

package CodingAdventures::TestBootstrap::Runner::Options;

use strict;
use warnings;

my %CI_MODES    = ( on => 1, off => 1, unset => 1 );
my %OPTION_KEYS = map { $_ => 1 } qw(num_tests max_shrink_steps ci parent);

# `{ num_tests => 100, max_shrink_steps => 1000, ci => 'unset', parent => undef }`.
# `ci` is 'on', 'off' or 'unset'; 'unset' is treated as CI: the engine fails closed
# and redacts.  The package never reads the environment; `ci_from_value` is a pure
# helper (pass `$ENV{CI}`).  `parent` is the budget scope of an enclosing check; a
# check nested in a property finds the ambient scope by itself, so `parent` is only
# needed to pass an explicit (possibly stale) one.
sub new {
    my ( $class, $t ) = @_;
    $t = {} if !defined $t;
    my $h = CodingAdventures::TestBootstrap::Safe::hash_copy( $t, 'options' );
    for my $k ( keys %$h ) {
        die "unknown option\n" if !$OPTION_KEYS{$k};
    }
    my $ci = defined $h->{ci} ? $h->{ci} : 'unset';
    die "ci must be one of on, off, unset\n" if ref $ci || !$CI_MODES{$ci};
    my $num_tests = defined $h->{num_tests}        ? $h->{num_tests}        : 100;
    my $max_steps = defined $h->{max_shrink_steps} ? $h->{max_shrink_steps} : 1000;
    $num_tests = CodingAdventures::TestBootstrap::Limits::check_range( $num_tests, 0, CodingAdventures::TestBootstrap::Limits::MAX_CASES, 'num_tests' );
    $max_steps = CodingAdventures::TestBootstrap::Limits::check_range( $max_steps, 0, CodingAdventures::TestBootstrap::Limits::MAX_SHRINK_STEPS, 'max_shrink_steps' );
    my $parent = $h->{parent};
    die "parent must be a budget scope\n" if defined $parent && !CodingAdventures::TestBootstrap::Budget::is_scope($parent);
    return bless { num_tests => $num_tests, max_shrink_steps => $max_steps, ci => $ci, parent => $parent }, $class;
}

# `CI` set to anything except empty, "0" or "false" (case-insensitive, trimmed)
# means CI.  `$value` is the environment value or undef.
sub ci_from_value {
    my ($value) = @_;
    return 'off' if !defined $value;
    die "CI value must be a string or undef\n" if ref $value;
    ( my $v = lc $value ) =~ s/\A\s+|\s+\z//g;
    return ( $v eq '' || $v eq '0' || $v eq 'false' ) ? 'off' : 'on';
}

package CodingAdventures::TestBootstrap::Runner;

# ----------------------------------------------------------------- verdicts

sub _verdict {
    my ( $passed, $failure_id, $message, $cases_run, $iteration, $extra ) = @_;
    $extra ||= {};
    return {
        passed         => $passed ? 1 : 0,
        failed         => $passed ? 0 : 1,
        failure_id     => $failure_id,
        message        => $message,
        cases_run      => $cases_run,
        iteration      => $iteration,
        shrink_steps   => $extra->{shrink_steps} || 0,
        seed           => $extra->{seed},
        counterexample => $extra->{counterexample},
        minimal        => $extra->{minimal},
        digest         => $extra->{digest},
    };
}

# The enclosing shrink budget was spent: exhausted, never a pass.
sub _spent {
    my ($iteration) = @_;
    return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::EXHAUSTED, "the enclosing shrink budget was spent at iteration $iteration", $iteration > 1 ? $iteration - 1 : 0, $iteration );
}

# `$status` / `$err` as returned by Safe::call; an escape has no value.
sub _kind { return $_[0] eq 'escaped' ? 'escaped' : CodingAdventures::TestBootstrap::Safe::kind( $_[1] ) }

sub _describe {
    my ( $status, $err ) = @_;
    return 'escaped: left through a non-local exit' if $status eq 'escaped';
    return CodingAdventures::TestBootstrap::Safe::kind($err) . ': ' . CodingAdventures::TestBootstrap::Canon::clip( CodingAdventures::TestBootstrap::Canon::render($err), 200 );
}

sub _exhausted {
    my ( $status, $err, $iteration, $redact ) = @_;
    my $text;
    if ( $status eq 'died' && CodingAdventures::TestBootstrap::Gen::is_exhausted_error($err) ) {
        $text = "filter precondition rejected all candidates at iteration $iteration";
    }
    elsif ($redact) {
        $text = sprintf( 'generator error at iteration %d (%s)', $iteration, _kind( $status, $err ) );
    }
    else {
        $text = sprintf( 'generator error at iteration %d: %s', $iteration, _describe( $status, $err ) );
    }
    return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::EXHAUSTED, $text, $iteration - 1, $iteration );
}

sub _panic {
    my ( $seed, $iteration, $status, $err, $redact ) = @_;
    if ($redact) {
        return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::PANIC, sprintf( 'property evaluation raised at iteration %d (%s)', $iteration, _kind( $status, $err ) ), $iteration, $iteration );
    }
    return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::PANIC, sprintf( 'property evaluation raised at iteration %d: %s', $iteration, _describe( $status, $err ) ), $iteration, $iteration, { seed => $seed } );
}

# `$prop->($value, $scope)` holds only on the plain 1.  A candidate the property
# dies on is not a counterexample to the claim; it is a different failure, so it
# never counts as falsifying (and is never accepted as "smaller").
sub _falsifies_with {
    my ( $prop, $v, $scope ) = @_;
    my ( $st, $r ) = CodingAdventures::TestBootstrap::Safe::call( $prop, $v, $scope );
    return $st eq 'ok' && !CodingAdventures::TestBootstrap::Safe::holds($r) ? 1 : 0;
}

sub _falsified {
    no warnings 'recursion';
    my ( $seed, $iteration, $candidate, $gen, $prop, $shrink, $opts, $redact, $check_scope ) = @_;
    my $initial_text = $redact ? '' : CodingAdventures::TestBootstrap::Canon::render($candidate);
    my ( $minimal, $steps ) = ( $candidate, 0 );
    my $max = $opts->{max_shrink_steps};

    # Untrusted shrinker: fall back to the original.  The shrinker runs on a COPY of
    # the candidate, in a scope that is a child of the check scope (itself a child of
    # any enclosing shrink): every evaluation is charged to the whole chain.
    my ( $copy, $failed ) = CodingAdventures::TestBootstrap::Safe::clone($candidate);
    if ( !$failed ) {
        my ( $st, $res ) = CodingAdventures::TestBootstrap::Safe::call(
            sub {
                my $own   = CodingAdventures::TestBootstrap::Budget::open_child( $check_scope, $max );
                my $scope = $own->scope;
                local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
                my $budgeted = $scope->charged( sub { return _falsifies_with( $prop, $_[0], $scope ) } );
                my ( $v, $u ) = $shrink ? $shrink->( $copy, $budgeted, $max, $scope ) : $gen->shrink_nested( $copy, $budgeted, $max, $scope );
                return [ $v, $u ];
            }
        );
        # Re-verify: the witness we report must really fail the property.  One attempt
        # is charged to the check scope; if the chain refuses it the shrunk value is
        # not adopted.
        if ( $st eq 'ok' && ref $res eq 'ARRAY' ) {
            my ( $value, $used ) = @$res;
            if (   CodingAdventures::TestBootstrap::Int::is($used)
                && !CodingAdventures::TestBootstrap::Int::is_big($used)
                && $used >= 0
                && $used <= $max
                && $check_scope->charge(1)
                && _falsifies_with( $prop, $value, $check_scope ) )
            {
                ( $minimal, $steps ) = ( $value, $used );
            }
        }
    }

    if ($redact) {
        my $preimage = "iter:$iteration;steps:$steps;shrunk:" . CodingAdventures::TestBootstrap::Canon::render($minimal);
        my $digest   = sprintf( '%016x', CodingAdventures::TestBootstrap::Prng::fnv1a64_word($preimage) );
        return _verdict(
            0, CodingAdventures::TestBootstrap::FailureIds::COUNTEREXAMPLE,
            sprintf( 'property falsified after %d iterations (shrunk in %d steps, repro-fnv: %s)', $iteration, $steps, $digest ),
            $iteration, $iteration, { shrink_steps => $steps, digest => $digest }
        );
    }

    return _verdict(
        0, CodingAdventures::TestBootstrap::FailureIds::COUNTEREXAMPLE,
        sprintf(
            'property falsified at iteration %d (seed=%s): initial=%s, minimal=%s after %d shrink steps',
            $iteration, CodingAdventures::TestBootstrap::Int::tostring($seed), $initial_text, CodingAdventures::TestBootstrap::Canon::render($minimal), $steps
        ),
        $iteration, $iteration,
        { shrink_steps => $steps, seed => $seed, counterexample => $candidate, minimal => $minimal }
    );
}

# Runs `$prop->($candidate, $scope)` over `num_tests` candidates from `$seed`.  The
# property returns the plain 1 when it holds.  `$shrink` (optional) is
# `sub ($candidate, $falsifies, $max_steps, $scope) -> ($minimal, $steps)` and
# replaces the generator's shrinker, so a defective shrinker can be tested.
sub check {
    no warnings 'recursion';
    my ( $seed, $gen, $prop, $options, $shrink ) = @_;
    die "a property code reference is required\n" if ref $prop ne 'CODE';
    die "gen must be a Gen\n" if !CodingAdventures::TestBootstrap::Gen::is($gen);
    $seed = CodingAdventures::TestBootstrap::Int::check( $seed, 'seed' );
    die "shrink must be a code reference\n" if defined $shrink && ref $shrink ne 'CODE';
    my $opts = defined $options ? $options : CodingAdventures::TestBootstrap::Runner::Options->new;
    my $c = Scalar::Util::blessed($opts);
    die "options must be Options\n" if !defined $c || $c ne 'CodingAdventures::TestBootstrap::Runner::Options';
    # Arguments are valid.  From here on the call ALWAYS returns a verdict: anything
    # the engine itself cannot finish is an honest property-panic that names only the
    # error's kind, never a thrown error and never a pass.
    my ( $st, $result ) = CodingAdventures::TestBootstrap::Safe::call( sub { return _run_check( $seed, $gen, $prop, $opts, $shrink ) } );
    return $result if $st eq 'ok';
    return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::PANIC, 'the engine could not complete the check (' . _kind( $st, $result ) . ')', 0, 0 );
}

sub _run_check {
    no warnings 'recursion';
    my ( $seed, $gen, $prop, $opts, $shrink ) = @_;
    my $seed_int = CodingAdventures::TestBootstrap::Int::from_u64( CodingAdventures::TestBootstrap::Int::to_u64($seed) );
    my $redact   = $opts->{ci} ne 'off';
    return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::EXHAUSTED, 'no cases were evaluated (num_tests is 0); a property that never ran is not a pass', 0, 0 )
      if $opts->{num_tests} == 0;

    # A check nested in a property (a parent scope, explicit or ambient) pays one
    # attempt at entry, one per case after the first and the size of every sampled
    # collection as work, all charged up the chain: neither nesting depth, branching
    # nor num_tests multiplies the outermost budget.  A starved nested check reports
    # property-exhausted without evaluating anything.  Nesting is capped.
    my $parent = defined $opts->{parent} ? $opts->{parent} : CodingAdventures::TestBootstrap::Budget::current();
    my $nested = defined $parent;
    if ($nested) {
        if ( $parent->checks >= CodingAdventures::TestBootstrap::Budget::MAX_CHECK_NESTING || $parent->depth + 1 >= CodingAdventures::TestBootstrap::Budget::MAX_DEPTH ) {
            $parent->mark_refused;
            return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::EXHAUSTED, 'checks are nested too deeply; nothing was evaluated', 0, 0 );
        }
        return _spent(0) if !$parent->charge(1);
    }

    # The WHOLE check runs in one scope (case loop included).
    my $own   = CodingAdventures::TestBootstrap::Budget::open_check( $parent, $opts->{num_tests}, $opts->{max_shrink_steps} );
    my $scope = $own->scope;
    local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
    my $rng = CodingAdventures::TestBootstrap::Prng::new_rng($seed_int);

    for my $iteration ( 1 .. $opts->{num_tests} ) {
        return _spent($iteration) if $nested && $iteration > 1 && !$scope->charge(1);

        my ( $sst, $candidate ) = CodingAdventures::TestBootstrap::Safe::call( sub { return $gen->sample( $rng, $scope ) } );
        return _exhausted( $sst, $candidate, $iteration, $redact ) if $sst ne 'ok';

        return _spent($iteration) if $nested && !$scope->charge_copy( CodingAdventures::TestBootstrap::Safe::size_of($candidate) );

        my ( $pst, $result ) = CodingAdventures::TestBootstrap::Safe::call( $prop, $candidate, $scope );
        return _panic( $seed_int, $iteration, $pst, $result, $redact ) if $pst ne 'ok';
        return _falsified( $seed_int, $iteration, $candidate, $gen, $prop, $shrink, $opts, $redact, $scope )
          if !CodingAdventures::TestBootstrap::Safe::holds($result);
    }

    # A nested check or generator inside this run was starved, so the property was
    # not fully evaluated: never report that as a pass.
    return _verdict( 0, CodingAdventures::TestBootstrap::FailureIds::EXHAUSTED, 'a nested check was starved by the shrink budget; the property was not fully evaluated', $opts->{num_tests}, $opts->{num_tests} )
      if !$nested && $scope->refused;

    return _verdict( 1, undef, undef, $opts->{num_tests}, 0 );
}

1;
