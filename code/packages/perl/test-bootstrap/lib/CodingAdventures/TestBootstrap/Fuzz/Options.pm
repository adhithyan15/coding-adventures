package CodingAdventures::TestBootstrap::Fuzz::Options;

# Bounds of a campaign; every field is validated (CWE-400).
#
# `minimize_budget` is the most target executions spent minimising ONE novel crash; on top
# of it the whole campaign spends at most `max_iterations` minimisation executions.
# `step_budget` and `mem_budget` are the per-execution budgets of `Handle::tick` and
# `Handle::reserve`; `timeout_ms` is the wall bound the alarm enforces (VER06q 5.1).

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int          ();
use CodingAdventures::TestBootstrap::Canon        ();
use CodingAdventures::TestBootstrap::Limits       ();
use CodingAdventures::TestBootstrap::Safe         ();
use CodingAdventures::TestBootstrap::Fuzz::Limits ();

my $int_check   = \&CodingAdventures::TestBootstrap::Int::check;
my $check_range = \&CodingAdventures::TestBootstrap::Limits::check_range;
my $hash_copy   = \&CodingAdventures::TestBootstrap::Safe::hash_copy;
my $blessed     = \&Scalar::Util::blessed;
my $L = 'CodingAdventures::TestBootstrap::Fuzz::Limits';

my %CI_MODES = ( on => 1, off => 1, unset => 1 );

# name => [default, lo, hi]
my %NATIVE = (
    max_iterations  => [ 1000, 0, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_ITERATIONS() ],
    max_input_len   => [ 4096, 0, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_INPUT_BYTES() ],
    base_energy     => [ 10, 0, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_ITERATIONS() ],
    minimize_budget => [ CodingAdventures::TestBootstrap::Fuzz::Limits::DEFAULT_MINIMIZE_BUDGET(), 0, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_MINIMIZE_BUDGET() ],
    step_budget     => [ CodingAdventures::TestBootstrap::Fuzz::Limits::DEFAULT_STEP_BUDGET(), 0, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_STEP_BUDGET() ],
    mem_budget      => [ CodingAdventures::TestBootstrap::Fuzz::Limits::DEFAULT_MEM_BUDGET(), 0, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_MEM_BUDGET() ],
    timeout_ms      => [ CodingAdventures::TestBootstrap::Fuzz::Limits::DEFAULT_TIMEOUT_MS(), 1, CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_TIMEOUT_MS() ],
);

# `Options->new({ max_iterations => 1000, seed => 42, ..., ci => 'unset' })`.  The hash is
# author-supplied: it is copied without running any overload or tie.  `ci` is "on", "off" or
# "unset"; "unset" is CI (fail closed): the verdict carries no seed and no witness.
sub new {
    my ( $class, $t ) = @_;
    $t = {} if !defined $t;
    my $h = $hash_copy->( $t, 'options' );
    for my $k ( keys %$h ) {
        die "unknown option\n" if !$NATIVE{$k} && $k ne 'seed' && $k ne 'ci';
    }
    my %o;
    for my $name ( keys %NATIVE ) {
        my $spec = $NATIVE{$name};
        my $v    = defined $h->{$name} ? $h->{$name} : $spec->[0];
        $o{$name} = $check_range->( $v, $spec->[1], $spec->[2], $name );
    }
    $o{seed} = $int_check->( defined $h->{seed} ? $h->{seed} : 42, 'seed' );
    my $ci = defined $h->{ci} ? $h->{ci} : 'unset';
    die "ci must be one of on, off, unset\n" if ref $ci || !$CI_MODES{$ci};
    $o{ci} = $ci;
    return bless \%o, __PACKAGE__;
}

# True for an Options this module built.
sub is {
    my $c = $blessed->( $_[0] );
    return ( defined $c && $c eq __PACKAGE__ ) ? 1 : 0;
}

# The seed is private (VER06 section 6): never printed.
use overload '""' => sub { '#<Fuzz.Options max_iterations=' . $_[0]{max_iterations} . '>' }, fallback => 1;

1;
