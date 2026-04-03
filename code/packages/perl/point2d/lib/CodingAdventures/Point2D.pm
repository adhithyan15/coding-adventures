package CodingAdventures::Point2D;

# =============================================================================
# CodingAdventures::Point2D — Immutable 2D Point/Vector and Rect
# =============================================================================
#
# A Point is (x, y) represented as a blessed arrayref [x, y].
# A Rect  is (x, y, w, h) represented as a blessed arrayref.
#
# All operations are pure: no mutation occurs.
# =============================================================================

use strict;
use warnings;
use Exporter 'import';
use lib '../trig/lib';
use CodingAdventures::Trig qw(sqrt_approx atan2_approx);

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(new_point new_rect);

# ---------------------------------------------------------------------------
# Point
# ---------------------------------------------------------------------------

sub new_point {
    my ($x, $y) = @_;
    return bless { x => $x, y => $y }, 'CodingAdventures::Point2D::Point';
}

package CodingAdventures::Point2D::Point;

use lib '../../trig/lib';
use CodingAdventures::Trig qw(sqrt_approx atan2_approx);

sub x { $_[0]->{x} }
sub y { $_[0]->{y} }

sub add {
    my ($self, $other) = @_;
    CodingAdventures::Point2D::new_point($self->{x} + $other->{x}, $self->{y} + $other->{y});
}

sub subtract {
    my ($self, $other) = @_;
    CodingAdventures::Point2D::new_point($self->{x} - $other->{x}, $self->{y} - $other->{y});
}

sub scale {
    my ($self, $s) = @_;
    CodingAdventures::Point2D::new_point($self->{x} * $s, $self->{y} * $s);
}

sub negate {
    my ($self) = @_;
    CodingAdventures::Point2D::new_point(-$self->{x}, -$self->{y});
}

sub dot {
    my ($self, $other) = @_;
    $self->{x} * $other->{x} + $self->{y} * $other->{y};
}

sub cross {
    my ($self, $other) = @_;
    $self->{x} * $other->{y} - $self->{y} * $other->{x};
}

sub magnitude_squared {
    my ($self) = @_;
    $self->{x} * $self->{x} + $self->{y} * $self->{y};
}

sub magnitude {
    my ($self) = @_;
    sqrt_approx($self->magnitude_squared);
}

sub normalize {
    my ($self) = @_;
    my $m = $self->magnitude;
    return $self if $m < 1e-15;
    $self->scale(1.0 / $m);
}

sub distance_squared {
    my ($self, $other) = @_;
    $self->subtract($other)->magnitude_squared;
}

sub distance {
    my ($self, $other) = @_;
    sqrt_approx($self->distance_squared($other));
}

sub lerp {
    my ($self, $other, $t) = @_;
    CodingAdventures::Point2D::new_point(
        $self->{x} + $t * ($other->{x} - $self->{x}),
        $self->{y} + $t * ($other->{y} - $self->{y}),
    );
}

sub perpendicular {
    my ($self) = @_;
    CodingAdventures::Point2D::new_point(-$self->{y}, $self->{x});
}

sub angle {
    my ($self) = @_;
    atan2_approx($self->{y}, $self->{x});
}

# ---------------------------------------------------------------------------
# Rect
# ---------------------------------------------------------------------------

package CodingAdventures::Point2D;

sub new_rect {
    my ($x, $y, $w, $h) = @_;
    return bless { x => $x, y => $y, width => $w, height => $h },
        'CodingAdventures::Point2D::Rect';
}

package CodingAdventures::Point2D::Rect;

sub x      { $_[0]->{x} }
sub y      { $_[0]->{y} }
sub width  { $_[0]->{width} }
sub height { $_[0]->{height} }

sub contains_point {
    my ($self, $pt) = @_;
    $pt->{x} >= $self->{x} && $pt->{x} < $self->{x} + $self->{width}
        && $pt->{y} >= $self->{y} && $pt->{y} < $self->{y} + $self->{height};
}

sub union {
    my ($self, $other) = @_;
    my $x0 = $self->{x} < $other->{x} ? $self->{x} : $other->{x};
    my $y0 = $self->{y} < $other->{y} ? $self->{y} : $other->{y};
    my $x1 = ($self->{x} + $self->{width})  > ($other->{x} + $other->{width})
           ? ($self->{x} + $self->{width}) : ($other->{x} + $other->{width});
    my $y1 = ($self->{y} + $self->{height}) > ($other->{y} + $other->{height})
           ? ($self->{y} + $self->{height}) : ($other->{y} + $other->{height});
    CodingAdventures::Point2D::new_rect($x0, $y0, $x1 - $x0, $y1 - $y0);
}

sub intersection {
    my ($self, $other) = @_;
    my $x0 = $self->{x} > $other->{x} ? $self->{x} : $other->{x};
    my $y0 = $self->{y} > $other->{y} ? $self->{y} : $other->{y};
    my $x1 = ($self->{x} + $self->{width})  < ($other->{x} + $other->{width})
           ? ($self->{x} + $self->{width}) : ($other->{x} + $other->{width});
    my $y1 = ($self->{y} + $self->{height}) < ($other->{y} + $other->{height})
           ? ($self->{y} + $self->{height}) : ($other->{y} + $other->{height});
    return undef if $x1 <= $x0 || $y1 <= $y0;
    CodingAdventures::Point2D::new_rect($x0, $y0, $x1 - $x0, $y1 - $y0);
}

sub expand_by {
    my ($self, $margin) = @_;
    CodingAdventures::Point2D::new_rect(
        $self->{x} - $margin, $self->{y} - $margin,
        $self->{width} + 2 * $margin, $self->{height} + 2 * $margin,
    );
}

package CodingAdventures::Point2D;
1;
