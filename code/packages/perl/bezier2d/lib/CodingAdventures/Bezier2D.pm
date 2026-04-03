package CodingAdventures::Bezier2D;

# =============================================================================
# CodingAdventures::Bezier2D — Quadratic and Cubic Bezier Curves
# =============================================================================
#
# QuadraticBezier: bless { p0, p1, p2 }
# CubicBezier:     bless { p0, p1, p2, p3 }
# Points are CodingAdventures::Point2D::Point objects.
# =============================================================================

use strict;
use warnings;
use Exporter 'import';
use lib '../trig/lib';
use lib '../point2d/lib';
use CodingAdventures::Trig qw(sqrt_approx);
use CodingAdventures::Point2D qw(new_point new_rect);

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(new_quad new_cubic);

sub new_quad  { bless { p0=>$_[0], p1=>$_[1], p2=>$_[2] }, 'CodingAdventures::Bezier2D::Quad' }
sub new_cubic { bless { p0=>$_[0], p1=>$_[1], p2=>$_[2], p3=>$_[3] }, 'CodingAdventures::Bezier2D::Cubic' }

# ---------------------------------------------------------------------------
# QuadraticBezier
# ---------------------------------------------------------------------------
package CodingAdventures::Bezier2D::Quad;

sub p0 { $_[0]->{p0} }
sub p1 { $_[0]->{p1} }
sub p2 { $_[0]->{p2} }

sub eval {
    my ($self, $t) = @_;
    my $q0 = $self->{p0}->lerp($self->{p1}, $t);
    my $q1 = $self->{p1}->lerp($self->{p2}, $t);
    $q0->lerp($q1, $t);
}

sub deriv {
    my ($self, $t) = @_;
    my $d0 = $self->{p1}->subtract($self->{p0});
    my $d1 = $self->{p2}->subtract($self->{p1});
    $d0->lerp($d1, $t)->scale(2);
}

sub split {
    my ($self, $t) = @_;
    my $q0 = $self->{p0}->lerp($self->{p1}, $t);
    my $q1 = $self->{p1}->lerp($self->{p2}, $t);
    my $m  = $q0->lerp($q1, $t);
    (CodingAdventures::Bezier2D::new_quad($self->{p0}, $q0, $m),
     CodingAdventures::Bezier2D::new_quad($m, $q1, $self->{p2}));
}

sub polyline {
    my ($self, $tol) = @_;
    my $chord_mid = $self->{p0}->lerp($self->{p2}, 0.5);
    my $curve_mid = $self->eval(0.5);
    if ($chord_mid->distance($curve_mid) <= $tol) {
        return [$self->{p0}, $self->{p2}];
    }
    my ($left, $right) = $self->split(0.5);
    my $lpts = $left->polyline($tol);
    my $rpts = $right->polyline($tol);
    my @combined = @$lpts;
    push @combined, @{$rpts}[1..$#$rpts];
    return \@combined;
}

sub bbox {
    my ($self) = @_;
    my ($min_x, $max_x) = sort { $a <=> $b } ($self->{p0}->{x}, $self->{p2}->{x});
    my ($min_y, $max_y) = sort { $a <=> $b } ($self->{p0}->{y}, $self->{p2}->{y});

    my $dx = $self->{p0}->{x} - 2*$self->{p1}->{x} + $self->{p2}->{x};
    if (abs($dx) > 1e-12) {
        my $tx = ($self->{p0}->{x} - $self->{p1}->{x}) / $dx;
        if ($tx > 0 && $tx < 1) {
            my $px = $self->eval($tx)->{x};
            $min_x = $px if $px < $min_x;
            $max_x = $px if $px > $max_x;
        }
    }
    my $dy = $self->{p0}->{y} - 2*$self->{p1}->{y} + $self->{p2}->{y};
    if (abs($dy) > 1e-12) {
        my $ty = ($self->{p0}->{y} - $self->{p1}->{y}) / $dy;
        if ($ty > 0 && $ty < 1) {
            my $py = $self->eval($ty)->{y};
            $min_y = $py if $py < $min_y;
            $max_y = $py if $py > $max_y;
        }
    }
    CodingAdventures::Point2D::new_rect($min_x, $min_y, $max_x - $min_x, $max_y - $min_y);
}

sub elevate {
    my ($self) = @_;
    my $q1 = $self->{p0}->scale(1.0/3)->add($self->{p1}->scale(2.0/3));
    my $q2 = $self->{p1}->scale(2.0/3)->add($self->{p2}->scale(1.0/3));
    CodingAdventures::Bezier2D::new_cubic($self->{p0}, $q1, $q2, $self->{p2});
}

# ---------------------------------------------------------------------------
# CubicBezier
# ---------------------------------------------------------------------------
package CodingAdventures::Bezier2D::Cubic;

sub p0 { $_[0]->{p0} }
sub p1 { $_[0]->{p1} }
sub p2 { $_[0]->{p2} }
sub p3 { $_[0]->{p3} }

sub eval {
    my ($self, $t) = @_;
    my $p01  = $self->{p0}->lerp($self->{p1}, $t);
    my $p12  = $self->{p1}->lerp($self->{p2}, $t);
    my $p23  = $self->{p2}->lerp($self->{p3}, $t);
    my $p012 = $p01->lerp($p12, $t);
    my $p123 = $p12->lerp($p23, $t);
    $p012->lerp($p123, $t);
}

sub deriv {
    my ($self, $t) = @_;
    my $d0 = $self->{p1}->subtract($self->{p0});
    my $d1 = $self->{p2}->subtract($self->{p1});
    my $d2 = $self->{p3}->subtract($self->{p2});
    my $one_t = 1 - $t;
    my $r = $d0->scale($one_t*$one_t)->add($d1->scale(2*$one_t*$t))->add($d2->scale($t*$t));
    $r->scale(3);
}

sub split {
    my ($self, $t) = @_;
    my $p01   = $self->{p0}->lerp($self->{p1}, $t);
    my $p12   = $self->{p1}->lerp($self->{p2}, $t);
    my $p23   = $self->{p2}->lerp($self->{p3}, $t);
    my $p012  = $p01->lerp($p12, $t);
    my $p123  = $p12->lerp($p23, $t);
    my $p0123 = $p012->lerp($p123, $t);
    (CodingAdventures::Bezier2D::new_cubic($self->{p0}, $p01, $p012, $p0123),
     CodingAdventures::Bezier2D::new_cubic($p0123, $p123, $p23, $self->{p3}));
}

sub polyline {
    my ($self, $tol) = @_;
    my $chord_mid = $self->{p0}->lerp($self->{p3}, 0.5);
    my $curve_mid = $self->eval(0.5);
    if ($chord_mid->distance($curve_mid) <= $tol) {
        return [$self->{p0}, $self->{p3}];
    }
    my ($left, $right) = $self->split(0.5);
    my $lpts = $left->polyline($tol);
    my $rpts = $right->polyline($tol);
    my @combined = @$lpts;
    push @combined, @{$rpts}[1..$#$rpts];
    return \@combined;
}

sub bbox {
    my ($self) = @_;
    my ($min_x, $max_x) = sort { $a <=> $b } ($self->{p0}->{x}, $self->{p3}->{x});
    my ($min_y, $max_y) = sort { $a <=> $b } ($self->{p0}->{y}, $self->{p3}->{y});

    for my $tx (_extrema($self->{p0}->{x}, $self->{p1}->{x}, $self->{p2}->{x}, $self->{p3}->{x})) {
        my $px = $self->eval($tx)->{x};
        $min_x = $px if $px < $min_x;
        $max_x = $px if $px > $max_x;
    }
    for my $ty (_extrema($self->{p0}->{y}, $self->{p1}->{y}, $self->{p2}->{y}, $self->{p3}->{y})) {
        my $py = $self->eval($ty)->{y};
        $min_y = $py if $py < $min_y;
        $max_y = $py if $py > $max_y;
    }
    CodingAdventures::Point2D::new_rect($min_x, $min_y, $max_x - $min_x, $max_y - $min_y);
}

sub _extrema {
    my ($v0, $v1, $v2, $v3) = @_;
    my $a = -3*$v0 + 9*$v1 - 9*$v2 + 3*$v3;
    my $b =  6*$v0 - 12*$v1 + 6*$v2;
    my $c = -3*$v0 + 3*$v1;
    my @roots;
    if (abs($a) < 1e-12) {
        if (abs($b) > 1e-12) {
            my $tx = -$c / $b;
            push @roots, $tx if $tx > 0 && $tx < 1;
        }
    } else {
        my $disc = $b*$b - 4*$a*$c;
        if ($disc >= 0) {
            my $sq = CodingAdventures::Trig::sqrt_approx($disc);
            for my $tx ((-$b+$sq)/(2*$a), (-$b-$sq)/(2*$a)) {
                push @roots, $tx if $tx > 0 && $tx < 1;
            }
        }
    }
    return @roots;
}

package CodingAdventures::Bezier2D;
1;
