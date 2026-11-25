package CodingAdventures::TestBootstrap::Gen;

# A generator of values: it draws from a PRNG stream and knows how to shrink what
# it drew (VER05 3, 4, VER05p section 5).
#
# `sampler->($rng, $scope)` returns a value.  `$scope` is the budget scope of the
# running check (or undef outside one); only `filter` reads it.
#
# `shrinker->($value, $falsifies, $max_steps, $scope)` is undef (values are
# reported unshrunk) or a code reference returning `($value, $steps)`, where
# `$falsifies->($candidate)` is true while a candidate still falsifies the property
# and `$scope` is the scope the shrinker JOINS.

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Budget ();
use CodingAdventures::TestBootstrap::Limits ();

my $blessed = \&Scalar::Util::blessed;

# ----------------------------------------------- "could not produce a value"

# A sampling failure the runner reports as `property-exhausted`: a filter that
# never accepts, or a refused charge.  An object of this class, identified by
# `blessed` (never by its message).
package CodingAdventures::TestBootstrap::Gen::Exhausted;
sub message { return $_[0]{message} }

package CodingAdventures::TestBootstrap::Gen;

sub exhausted_error {
    return bless { message => $_[0] }, 'CodingAdventures::TestBootstrap::Gen::Exhausted';
}

sub is_exhausted_error {
    my $c = $blessed->( $_[0] );
    return ( defined $c && $c eq 'CodingAdventures::TestBootstrap::Gen::Exhausted' ) ? 1 : 0;
}

# ------------------------------------------------------------------ the class

sub new {
    my ( $sampler, $shrinker ) = @_;
    die "a sampler code reference is required\n" if ref $sampler ne 'CODE';
    die "a shrinker must be a code reference\n" if defined $shrinker && ref $shrinker ne 'CODE';
    return bless { sampler => $sampler, shrinker => $shrinker }, __PACKAGE__;
}

sub is {
    my $c = $blessed->( $_[0] );
    return ( defined $c && $c eq __PACKAGE__ ) ? 1 : 0;
}

# Draws one value.
sub sample { return $_[0]{sampler}->( $_[1], $_[2] ) }

sub _run {
    my ( $self, $value, $falsifies, $max_steps, $scope ) = @_;
    return ( $value, 0 ) if !defined $self->{shrinker};
    return $self->{shrinker}->( $value, $falsifies, $max_steps, $scope );
}

# Shrinks `$value` while `$falsifies` holds; `($value, 0)` if there is no
# shrinker.  A PUBLIC ENTRY POINT: it opens a child of `$parent` (the ambient scope
# when undef, a fresh root when there is none) with its own local cap and charges
# every evaluation of `$falsifies`, so however a property or a custom shrinker
# nests calls, the total work stays inside the outermost budget.
sub shrink {
    my ( $self, $value, $max_steps, $falsifies, $parent ) = @_;
    die "a falsifies code reference is required\n" if ref $falsifies ne 'CODE';
    CodingAdventures::TestBootstrap::Limits::check_budget($max_steps);
    my $own   = CodingAdventures::TestBootstrap::Budget::open_child( $parent, $max_steps );
    my $scope = $own->scope;
    local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
    return _run( $self, $value, $scope->charged($falsifies), $max_steps, $scope );
}

# For combinators shrinking their components: JOINS the scope it is given instead
# of opening a new one (a fresh root only when `$scope` is undef).  The caller's
# `$falsifies` is already charged, so nothing is charged twice.
sub shrink_nested {
    my ( $self, $value, $falsifies, $max_steps, $scope ) = @_;
    CodingAdventures::TestBootstrap::Limits::check_budget($max_steps);
    if ( !defined $scope ) {
        my $own = CodingAdventures::TestBootstrap::Budget::open_child( undef, $max_steps );
        local $CodingAdventures::TestBootstrap::Budget::CURRENT = $own->scope;
        return _run( $self, $value, $own->scope->charged($falsifies), $max_steps, $own->scope );
    }
    return _run( $self, $value, $falsifies, $max_steps, $scope );
}

1;
