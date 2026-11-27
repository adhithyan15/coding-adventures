package CodingAdventures::TestBootstrap::Fuzz::Bitmap;

# The execution map, admission and the power schedule (VER06 section 4, VER06q 4.1).
#
# A map is a SET of edge ids reduced into [0, 65536): edge e is in the set iff it was
# executed.  Collisions are the known price of a fixed-size map; AFL pays the same one.
# The set form costs O(edges hit) to list, where an 8 KiB bitmap would cost O(8 KiB) per
# execution.

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int          ();
use CodingAdventures::TestBootstrap::Fuzz::Limits ();

my $int_is      = \&CodingAdventures::TestBootstrap::Int::is;
my $int_is_big  = \&CodingAdventures::TestBootstrap::Int::is_big;
my $int_mod     = \&CodingAdventures::TestBootstrap::Int::mod;
my $int_tostring = \&CodingAdventures::TestBootstrap::Int::tostring;
my $MAP_SIZE    = CodingAdventures::TestBootstrap::Fuzz::Limits::COVERAGE_MAP_SIZE();

# An edge id (any Int) as its slot in the 65,536-entry map: floored `mod 65536`, so -1 is
# 65535 and a big is reduced by `Int::mod`.  (For a native integer the low sixteen bits ARE
# the floored remainder, in two's complement.)
sub reduce {
    my ($edge) = @_;
    die "edge must be an Integer\n" if !$int_is->($edge);
    return $int_is_big->($edge) ? $int_mod->( $edge, $MAP_SIZE ) : $edge & 0xFFFF;
}

sub new { return bless { set => {}, n => 0 }, __PACKAGE__ }

sub record {
    my ( $self, $edge ) = @_;
    my $e = reduce($edge);
    if ( !$self->{set}{$e} ) { $self->{set}{$e} = 1; $self->{n}++ }
    return;
}

sub has { return $_[0]{set}{ reduce( $_[1] ) } ? 1 : 0 }

# OR a trace (an array ref of edge ids) into this cumulative map; returns (has_new, count_new).
sub admit {
    my ( $self, $trace ) = @_;
    my $count_new = 0;
    for my $edge (@$trace) {
        my $e = reduce($edge);
        if ( !$self->{set}{$e} ) { $self->{set}{$e} = 1; $self->{n}++; $count_new++ }
    }
    return ( $count_new > 0 ? 1 : 0, $count_new );
}

# The covered edges, ascending.
sub covered_edges { return [ map { 0 + $_ } sort { $a <=> $b } keys %{ $_[0]{set} } ] }

sub count { return $_[0]{n} }

sub reset { $_[0]{set} = {}; $_[0]{n} = 0; return }

use overload '""' => sub { '#<Fuzz.Bitmap edges=' . $_[0]{n} . '>' }, fallback => 1;

# A hash key for an unreduced edge: the decimal text (a big by its text, so no float is ever
# a key).
sub _key_of {
    my ($edge) = @_;
    die "edge must be an Integer\n" if !$int_is->($edge);
    return $int_tostring->($edge);
}

# Pure novelty check: does `trace` hold an edge absent from `existing`?  Returns
# (admitted, new_edges) counting UNIQUE novel edges over the full, unreduced ids.
#
#   corpus_admit([1, 2, 3], [2, 4])  --> (1, 1)
#   corpus_admit([1, 2, 3], [1, 2])  --> (0, 0)
sub corpus_admit {
    my ( $existing, $trace ) = @_;
    my ( %have, %seen );
    my $novel = 0;
    $have{ _key_of($_) } = 1 for @$existing;
    for my $edge (@$trace) {
        my $k = _key_of($edge);
        if ( !$have{$k} && !$seen{$k} ) { $seen{$k} = 1; $novel++ }
    }
    return ( $novel > 0 ? 1 : 0, $novel );
}

# Mutation slots for a seed: max(1, round(base * (1 + new/total) / sqrt(len))).  The edge
# bonus is 1.0 while nothing is covered.  Rounding is half away from zero (floor(x + 0.5))
# so every lane agrees on exact halves.
#
# base 10, len 4, 3 new of 6 total: 10 * 1.5 / 2 = 7.5 rounds to 8.
sub calculate_energy {
    my ( $base_energy, $seed_len, $new_edges, $total_edges ) = @_;
    my $base  = ( $base_energy > 1 ? $base_energy : 1 ) + 0.0;
    my $bonus = 1.0;
    $bonus = 1.0 + ( $new_edges + 0.0 ) / $total_edges if $total_edges > 0;
    my $v = int( $base * $bonus / sqrt( $seed_len > 1 ? $seed_len : 1 ) + 0.5 );
    return $v > 1 ? $v : 1;
}

1;
