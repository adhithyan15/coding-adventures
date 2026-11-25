package CodingAdventures::TestBootstrap::Hooks;

# Sabotage hooks: the only production indirections added for the negative
# controls (VER05p section 7).  A test swaps one with `with`, runs the corpus,
# and the runner must report problems.  Production code never writes them.  This
# hash is the ONLY mutable package state; it is a documented test seam and
# `with` restores it with `local`, so every exit (including `die`) restores it.

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int ();

# floor(diff / 2) for negatives: truncation would give -40 for diff -81 / 2.
sub default_bisect { return CodingAdventures::TestBootstrap::Int::sub( $_[0], CodingAdventures::TestBootstrap::Int::floor_half( $_[1] ) ) }
sub default_unit   { return CodingAdventures::TestBootstrap::Int::sub( $_[0], CodingAdventures::TestBootstrap::Int::sign( $_[1] ) ) }

my %KNOWN = map { $_ => 1 } qw(bisect unit skip_deletion new_rng threshold default_max_work);

our %H = (
    bisect           => \&default_bisect,
    unit             => \&default_unit,
    skip_deletion    => 0,
    new_rng          => undef,    # sub ($seed_int) -> rng; undef = SplitMix64
    threshold        => undef,    # sub ($n_word) -> word; undef = (2**64 - n) mod n
    default_max_work => undef,    # undef = Limits::MAX_SHRINK_WORK
);

# Runs `$code->(@args)` with the overrides installed, restoring them on every
# exit.  Returns what `$code` returns (scalar context); rethrows what it throws.
sub with {
    my ( $overrides, $code, @args ) = @_;
    for my $k ( keys %$overrides ) {
        die "unknown hook\n" if !$KNOWN{$k};
    }
    local %H = ( %H, %$overrides );
    return $code->(@args);
}

1;
