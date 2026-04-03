package CodingAdventures::Affine2D;

# =============================================================================
# CodingAdventures::Affine2D — 2D Affine Transformation Matrix
# =============================================================================
#
# Stored as {a,b,c,d,e,f} matching SVG matrix(a,b,c,d,e,f):
#   [ a  c  e ]
#   [ b  d  f ]
#   [ 0  0  1 ]
# x' = a*x + c*y + e
# y' = b*x + d*y + f
# =============================================================================

use strict;
use warnings;
use Exporter 'import';
use lib '../trig/lib';
use lib '../point2d/lib';
use CodingAdventures::Trig qw(sin_approx cos_approx tan_approx);
use CodingAdventures::Point2D qw(new_point);

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(identity translate rotate rotate_around scale scale_uniform skew_x skew_y);

sub _new {
    my ($a, $b, $c, $d, $e, $f) = @_;
    bless { a=>$a, b=>$b, c=>$c, d=>$d, e=>$e, f=>$f }, 'CodingAdventures::Affine2D';
}

sub identity    { _new(1, 0, 0, 1, 0, 0) }
sub translate   { _new(1, 0, 0, 1, $_[0], $_[1]) }
sub scale       { _new($_[0], 0, 0, $_[1], 0, 0) }
sub scale_uniform { scale($_[0], $_[0]) }

sub rotate {
    my ($angle) = @_;
    my $c = cos_approx($angle);
    my $s = sin_approx($angle);
    _new($c, $s, -$s, $c, 0, 0);
}

sub rotate_around {
    my ($angle, $px, $py) = @_;
    translate($px, $py)->compose(rotate($angle))->compose(translate(-$px, -$py));
}

sub skew_x { _new(1, 0, tan_approx($_[0]), 1, 0, 0) }
sub skew_y { _new(1, tan_approx($_[0]), 0, 1, 0, 0) }

package CodingAdventures::Affine2D;

sub a { $_[0]->{a} }
sub b { $_[0]->{b} }
sub c { $_[0]->{c} }
sub d { $_[0]->{d} }
sub e { $_[0]->{e} }
sub f { $_[0]->{f} }

sub compose {
    my ($self, $other) = @_;
    my ($a1,$b1,$c1,$d1,$e1,$f1) = @{$self}{qw(a b c d e f)};
    my ($a2,$b2,$c2,$d2,$e2,$f2) = @{$other}{qw(a b c d e f)};
    CodingAdventures::Affine2D::_new(
        $a1*$a2 + $c1*$b2,
        $b1*$a2 + $d1*$b2,
        $a1*$c2 + $c1*$d2,
        $b1*$c2 + $d1*$d2,
        $a1*$e2 + $c1*$f2 + $e1,
        $b1*$e2 + $d1*$f2 + $f1,
    );
}

sub apply_to_point {
    my ($self, $pt) = @_;
    new_point(
        $self->{a} * $pt->{x} + $self->{c} * $pt->{y} + $self->{e},
        $self->{b} * $pt->{x} + $self->{d} * $pt->{y} + $self->{f},
    );
}

sub apply_to_vector {
    my ($self, $v) = @_;
    new_point(
        $self->{a} * $v->{x} + $self->{c} * $v->{y},
        $self->{b} * $v->{x} + $self->{d} * $v->{y},
    );
}

sub determinant {
    my ($self) = @_;
    $self->{a} * $self->{d} - $self->{b} * $self->{c};
}

sub invert {
    my ($self) = @_;
    my $det = $self->determinant;
    return undef if abs($det) < 1e-12;
    my $inv = 1.0 / $det;
    CodingAdventures::Affine2D::_new(
        $self->{d} * $inv,
        -$self->{b} * $inv,
        -$self->{c} * $inv,
        $self->{a} * $inv,
        ($self->{c} * $self->{f} - $self->{d} * $self->{e}) * $inv,
        ($self->{b} * $self->{e} - $self->{a} * $self->{f}) * $inv,
    );
}

sub is_identity {
    my ($self) = @_;
    $self->{a} == 1 && $self->{b} == 0 && $self->{c} == 0
        && $self->{d} == 1 && $self->{e} == 0 && $self->{f} == 0;
}

sub is_translation_only {
    my ($self) = @_;
    $self->{a} == 1 && $self->{b} == 0 && $self->{c} == 0 && $self->{d} == 1;
}

sub to_array {
    my ($self) = @_;
    [$self->{a}, $self->{b}, $self->{c}, $self->{d}, $self->{e}, $self->{f}];
}

1;
