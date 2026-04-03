package CodingAdventures::Arc2D;

# =============================================================================
# CodingAdventures::Arc2D — Elliptical Arc (Center Form and SVG Endpoint Form)
# =============================================================================
#
# CenterArc: bless { center, rx, ry, start_angle, sweep_angle, x_rotation }
# SvgArc:    bless { from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep }
# =============================================================================

use strict;
use warnings;
use Exporter 'import';
use CodingAdventures::Trig qw(sin_approx cos_approx tan_approx sqrt_approx atan2_approx);
use CodingAdventures::Point2D qw(new_point new_rect);
use CodingAdventures::Bezier2D qw(new_cubic);

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(new_center_arc new_svg_arc);

our $PI     = $CodingAdventures::Trig::PI;
our $TWO_PI = $CodingAdventures::Trig::TWO_PI;

sub new_center_arc {
    my ($center, $rx, $ry, $start_angle, $sweep_angle, $x_rotation) = @_;
    bless {
        center      => $center,
        rx          => $rx,
        ry          => $ry,
        start_angle => $start_angle,
        sweep_angle => $sweep_angle,
        x_rotation  => $x_rotation,
    }, 'CodingAdventures::Arc2D::CenterArc';
}

sub new_svg_arc {
    my ($from_pt, $to_pt, $rx, $ry, $x_rotation, $large_arc, $sweep) = @_;
    bless {
        from_pt    => $from_pt,
        to_pt      => $to_pt,
        rx         => $rx,
        ry         => $ry,
        x_rotation => $x_rotation,
        large_arc  => $large_arc,
        sweep      => $sweep,
    }, 'CodingAdventures::Arc2D::SvgArc';
}

# ---------------------------------------------------------------------------
# CenterArc
# ---------------------------------------------------------------------------
package CodingAdventures::Arc2D::CenterArc;

sub center      { $_[0]->{center} }
sub rx          { $_[0]->{rx} }
sub ry          { $_[0]->{ry} }
sub start_angle { $_[0]->{start_angle} }
sub sweep_angle { $_[0]->{sweep_angle} }
sub x_rotation  { $_[0]->{x_rotation} }

sub eval_at {
    my ($self, $t) = @_;
    my $theta = $self->{start_angle} + $t * $self->{sweep_angle};
    my $cos_t = CodingAdventures::Trig::cos_approx($theta);
    my $sin_t = CodingAdventures::Trig::sin_approx($theta);
    my $lx = $self->{rx} * $cos_t;
    my $ly = $self->{ry} * $sin_t;
    my $cos_r = CodingAdventures::Trig::cos_approx($self->{x_rotation});
    my $sin_r = CodingAdventures::Trig::sin_approx($self->{x_rotation});
    CodingAdventures::Point2D::new_point(
        $self->{center}->{x} + $cos_r*$lx - $sin_r*$ly,
        $self->{center}->{y} + $sin_r*$lx + $cos_r*$ly,
    );
}

sub bbox {
    my ($self) = @_;
    my $p0 = $self->eval_at(0);
    my ($min_x, $max_x, $min_y, $max_y) = ($p0->{x}, $p0->{x}, $p0->{y}, $p0->{y});
    for my $i (1..100) {
        my $p = $self->eval_at($i / 100.0);
        $min_x = $p->{x} if $p->{x} < $min_x;
        $max_x = $p->{x} if $p->{x} > $max_x;
        $min_y = $p->{y} if $p->{y} < $min_y;
        $max_y = $p->{y} if $p->{y} > $max_y;
    }
    CodingAdventures::Point2D::new_rect($min_x, $min_y, $max_x - $min_x, $max_y - $min_y);
}

sub to_cubic_beziers {
    my ($self) = @_;
    my $half_pi = $CodingAdventures::Arc2D::PI / 2;
    my $n_seg = int(abs($self->{sweep_angle}) / $half_pi) + 1;
    my $seg_sweep = $self->{sweep_angle} / $n_seg;
    my $cos_r = CodingAdventures::Trig::cos_approx($self->{x_rotation});
    my $sin_r = CodingAdventures::Trig::sin_approx($self->{x_rotation});
    my $cx = $self->{center}->{x};
    my $cy = $self->{center}->{y};
    my $rx = $self->{rx};
    my $ry = $self->{ry};

    my $l2w = sub {
        my ($lx, $ly) = @_;
        CodingAdventures::Point2D::new_point($cx + $cos_r*$lx - $sin_r*$ly, $cy + $sin_r*$lx + $cos_r*$ly);
    };

    my @curves;
    for my $i (0..$n_seg-1) {
        my $t0 = $self->{start_angle} + $i * $seg_sweep;
        my $t1 = $t0 + $seg_sweep;
        my $k  = (4.0 / 3.0) * CodingAdventures::Trig::tan_approx($seg_sweep / 4);
        my $cos0 = CodingAdventures::Trig::cos_approx($t0);
        my $sin0 = CodingAdventures::Trig::sin_approx($t0);
        my $cos1 = CodingAdventures::Trig::cos_approx($t1);
        my $sin1 = CodingAdventures::Trig::sin_approx($t1);
        my $p0 = $l2w->($rx*$cos0, $ry*$sin0);
        my $p3 = $l2w->($rx*$cos1, $ry*$sin1);
        my $p1 = $l2w->($rx*$cos0 - $k*$rx*$sin0, $ry*$sin0 + $k*$ry*$cos0);
        my $p2 = $l2w->($rx*$cos1 + $k*$rx*$sin1, $ry*$sin1 - $k*$ry*$cos1);
        push @curves, CodingAdventures::Bezier2D::new_cubic($p0, $p1, $p2, $p3);
    }
    return \@curves;
}

# ---------------------------------------------------------------------------
# SvgArc
# ---------------------------------------------------------------------------
package CodingAdventures::Arc2D::SvgArc;

sub to_center_arc {
    my ($self) = @_;
    my $from_pt = $self->{from_pt};
    my $to_pt   = $self->{to_pt};
    return undef if $from_pt->{x} == $to_pt->{x} && $from_pt->{y} == $to_pt->{y};
    my $rx = abs($self->{rx});
    my $ry = abs($self->{ry});
    return undef if $rx < 1e-12 || $ry < 1e-12;

    my $cos_r = CodingAdventures::Trig::cos_approx($self->{x_rotation});
    my $sin_r = CodingAdventures::Trig::sin_approx($self->{x_rotation});

    my $dx2 = ($from_pt->{x} - $to_pt->{x}) / 2.0;
    my $dy2 = ($from_pt->{y} - $to_pt->{y}) / 2.0;
    my $x1p =  $cos_r*$dx2 + $sin_r*$dy2;
    my $y1p = -$sin_r*$dx2 + $cos_r*$dy2;

    my $lambda_sq = ($x1p/$rx)**2 + ($y1p/$ry)**2;
    if ($lambda_sq > 1) {
        my $lam = CodingAdventures::Trig::sqrt_approx($lambda_sq);
        $rx *= $lam;
        $ry *= $lam;
    }

    my ($rx2, $ry2) = ($rx*$rx, $ry*$ry);
    my ($x1p2, $y1p2) = ($x1p*$x1p, $y1p*$y1p);
    my $num = $rx2*$ry2 - $rx2*$y1p2 - $ry2*$x1p2;
    my $den = $rx2*$y1p2 + $ry2*$x1p2;
    return undef if $den < 1e-24;

    my $sq_val = $num/$den > 0 ? CodingAdventures::Trig::sqrt_approx($num/$den) : 0.0;
    my $sq = ($self->{large_arc} xor $self->{sweep}) ? $sq_val : -$sq_val;

    my $cxp =  $sq * $rx * $y1p / $ry;
    my $cyp = -$sq * $ry * $x1p / $rx;

    my $mx = ($from_pt->{x} + $to_pt->{x}) / 2.0;
    my $my = ($from_pt->{y} + $to_pt->{y}) / 2.0;
    my $center_x = $cos_r*$cxp - $sin_r*$cyp + $mx;
    my $center_y = $sin_r*$cxp + $cos_r*$cyp + $my;

    my $ux = ($x1p - $cxp) / $rx;
    my $uy = ($y1p - $cyp) / $ry;
    my $vx = (-$x1p - $cxp) / $rx;
    my $vy = (-$y1p - $cyp) / $ry;

    my $start_angle = CodingAdventures::Trig::atan2_approx($uy, $ux);
    my $sweep_angle = _angle_between($ux, $uy, $vx, $vy);

    if (!$self->{sweep} && $sweep_angle > 0) {
        $sweep_angle -= $CodingAdventures::Arc2D::TWO_PI;
    } elsif ($self->{sweep} && $sweep_angle < 0) {
        $sweep_angle += $CodingAdventures::Arc2D::TWO_PI;
    }

    CodingAdventures::Arc2D::new_center_arc(
        CodingAdventures::Point2D::new_point($center_x, $center_y),
        $rx, $ry, $start_angle, $sweep_angle, $self->{x_rotation},
    );
}

sub _angle_between {
    my ($ux, $uy, $vx, $vy) = @_;
    my $dot   = $ux*$vx + $uy*$vy;
    my $mag_u = CodingAdventures::Trig::sqrt_approx($ux*$ux + $uy*$uy);
    my $mag_v = CodingAdventures::Trig::sqrt_approx($vx*$vx + $vy*$vy);
    return 0 if $mag_u < 1e-12 || $mag_v < 1e-12;
    my $cos_a = $dot / ($mag_u * $mag_v);
    $cos_a = 1  if $cos_a >  1;
    $cos_a = -1 if $cos_a < -1;
    my $sin_a = CodingAdventures::Trig::sqrt_approx(1 - $cos_a*$cos_a);
    my $angle = CodingAdventures::Trig::atan2_approx($sin_a, $cos_a);
    $angle = -$angle if $ux*$vy - $uy*$vx < 0;
    return $angle;
}

package CodingAdventures::Arc2D;
1;
