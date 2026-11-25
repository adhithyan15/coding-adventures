package CodingAdventures::TestBootstrap;

# CodingAdventures::TestBootstrap: the Perl lane of the property-based testing
# engine (VER05p).  Core Perl only, no dependencies, no I/O.
#
#   use CodingAdventures::TestBootstrap;
#   my $TB = 'CodingAdventures::TestBootstrap';
#   my $verdict = $TB->check(
#       42,                                   # seed (an Int)
#       CodingAdventures::TestBootstrap::Gens::integer( 0, 1000 ),
#       sub { return $_[0] <= 100 ? 1 : 0 },  # holds only on the plain 1
#       CodingAdventures::TestBootstrap::Runner::Options->new( { num_tests => 100, ci => 'off' } ) );
#   # $verdict->{failure_id} eq 'property-counterexample', $verdict->{minimal} == 101

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::TestBootstrap::Int        ();
use CodingAdventures::TestBootstrap::Limits     ();
use CodingAdventures::TestBootstrap::FailureIds ();
use CodingAdventures::TestBootstrap::Hooks      ();
use CodingAdventures::TestBootstrap::Prng       ();
use CodingAdventures::TestBootstrap::Safe       ();
use CodingAdventures::TestBootstrap::Budget     ();
use CodingAdventures::TestBootstrap::Gen        ();
use CodingAdventures::TestBootstrap::Gens       ();
use CodingAdventures::TestBootstrap::Shrink     ();
use CodingAdventures::TestBootstrap::Runner     ();
use CodingAdventures::TestBootstrap::Canon      ();
use CodingAdventures::TestBootstrap::StrictJson ();
use CodingAdventures::TestBootstrap::Values     ();
use CodingAdventures::TestBootstrap::Tfv        ();
use CodingAdventures::TestBootstrap::Corpus     ();
use CodingAdventures::TestBootstrap::Fuzz       ();

# `check` is the one-call entry point: Runner::check.
sub check { shift if defined $_[0] && !ref $_[0] && $_[0] eq __PACKAGE__; return CodingAdventures::TestBootstrap::Runner::check(@_) }

# Library hygiene (VER05p section 4): the engine's own subs become read-only globs,
# so a property cannot `local`ise or reassign one.
CodingAdventures::TestBootstrap::Safe::seal(
    map { "CodingAdventures::TestBootstrap::$_" }
      qw(Int Limits FailureIds Hooks Prng Prng::Rng Safe Budget Budget::Guard Gen Gens Shrink Runner Runner::Options Canon StrictJson Values Tfv Corpus)
);

1;
