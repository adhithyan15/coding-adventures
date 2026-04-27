package CodingAdventures::ImageGeometricTransforms;

# ============================================================================
# CodingAdventures::ImageGeometricTransforms — Geometric transforms for Perl PixelContainer images
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::ImageGeometricTransforms;
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';
use POSIX qw(ceil floor);
use lib '../pixel-container/lib';

use CodingAdventures::PixelContainer;

our $VERSION = '0.01';
our @EXPORT_OK = qw(
    flip_horizontal flip_vertical rotate_90_cw rotate_90_ccw rotate_180
    crop pad scale rotate affine perspective_warp
);

my @SRGB_TO_LINEAR = map {
    my $c = $_ / 255.0;
    $c <= 0.04045 ? $c / 12.92 : (($c + 0.055) / 1.055) ** 2.4;
} 0 .. 255;

sub _decode {
    my ($byte) = @_;
    return $SRGB_TO_LINEAR[$byte // 0];
}

sub _encode {
    my ($v) = @_;
    $v = 0.0 if $v < 0.0;
    $v = 1.0 if $v > 1.0;
    my $c = $v <= 0.0031308
        ? 12.92 * $v
        : 1.055 * ($v ** (1.0 / 2.4)) - 0.055;
    $c = 0.0 if $c < 0.0;
    $c = 1.0 if $c > 1.0;
    return int($c * 255.0 + 0.5);
}

sub _clamp_byte {
    my ($value) = @_;
    $value = 0 if !defined $value || $value < 0;
    $value = 255 if $value > 255;
    return int($value + 0.5);
}

sub _mod_int {
    my ($value, $modulus) = @_;
    my $r = $value % $modulus;
    return $r < 0 ? $r + $modulus : $r;
}

sub _resolve_coord {
    my ($x, $max, $oob) = @_;
    $oob //= 'zero';
    if ($oob eq 'zero') {
        return ($x >= 0 && $x < $max) ? $x : undef;
    }
    if ($oob eq 'replicate') {
        return 0 if $x < 0;
        return $max - 1 if $x >= $max;
        return $x;
    }
    if ($oob eq 'reflect') {
        my $period = 2 * $max;
        $x = _mod_int($x, $period);
        $x = $period - 1 - $x if $x >= $max;
        return $x;
    }
    return _mod_int($x, $max);
}

sub _catmull_rom {
    my ($d) = @_;
    $d = abs($d);
    return 1.5 * $d * $d * $d - 2.5 * $d * $d + 1.0 if $d < 1.0;
    return -0.5 * $d * $d * $d + 2.5 * $d * $d - 4.0 * $d + 2.0 if $d < 2.0;
    return 0.0;
}

sub _sample_nearest {
    my ($img, $u, $v, $oob) = @_;
    my $ix = floor($u);
    my $iy = floor($v);
    my $rx = _resolve_coord($ix, $img->width, $oob);
    my $ry = _resolve_coord($iy, $img->height, $oob);
    return (0, 0, 0, 0) if !defined $rx || !defined $ry;
    return $img->pixel_at($rx, $ry);
}

sub _sample_bilinear {
    my ($img, $u, $v, $oob) = @_;
    my $x0 = floor($u);
    my $y0 = floor($v);
    my $tx = $u - $x0;
    my $ty = $v - $y0;

    my $get = sub {
        my ($ix, $iy) = @_;
        my $rx = _resolve_coord($ix, $img->width, $oob);
        my $ry = _resolve_coord($iy, $img->height, $oob);
        return (0, 0, 0, 0) if !defined $rx || !defined $ry;
        return $img->pixel_at($rx, $ry);
    };

    my ($r00, $g00, $b00, $a00) = $get->($x0,     $y0);
    my ($r10, $g10, $b10, $a10) = $get->($x0 + 1, $y0);
    my ($r01, $g01, $b01, $a01) = $get->($x0,     $y0 + 1);
    my ($r11, $g11, $b11, $a11) = $get->($x0 + 1, $y0 + 1);

    my $w00 = (1 - $tx) * (1 - $ty);
    my $w10 = $tx * (1 - $ty);
    my $w01 = (1 - $tx) * $ty;
    my $w11 = $tx * $ty;

    my $lr = $w00 * _decode($r00) + $w10 * _decode($r10) + $w01 * _decode($r01) + $w11 * _decode($r11);
    my $lg = $w00 * _decode($g00) + $w10 * _decode($g10) + $w01 * _decode($g01) + $w11 * _decode($g11);
    my $lb = $w00 * _decode($b00) + $w10 * _decode($b10) + $w01 * _decode($b01) + $w11 * _decode($b11);
    my $la = $w00 * $a00 + $w10 * $a10 + $w01 * $a01 + $w11 * $a11;

    return _encode($lr), _encode($lg), _encode($lb), int($la + 0.5);
}

sub _sample_bicubic {
    my ($img, $u, $v, $oob) = @_;
    my $x0 = floor($u);
    my $y0 = floor($v);
    my $tx = $u - $x0;
    my $ty = $v - $y0;

    my $get = sub {
        my ($ix, $iy) = @_;
        my $rx = _resolve_coord($ix, $img->width, $oob);
        my $ry = _resolve_coord($iy, $img->height, $oob);
        return (0, 0, 0, 0) if !defined $rx || !defined $ry;
        return $img->pixel_at($rx, $ry);
    };

    my @wx = (_catmull_rom($tx + 1), _catmull_rom($tx), _catmull_rom(1.0 - $tx), _catmull_rom(2.0 - $tx));
    my @wy = (_catmull_rom($ty + 1), _catmull_rom($ty), _catmull_rom(1.0 - $ty), _catmull_rom(2.0 - $ty));
    my ($sum_lr, $sum_lg, $sum_lb, $sum_la) = (0, 0, 0, 0);

    for my $dy (0 .. 3) {
        my $iy = $y0 - 1 + $dy;
        for my $dx (0 .. 3) {
            my $ix = $x0 - 1 + $dx;
            my ($r, $g, $b, $a) = $get->($ix, $iy);
            my $w = $wx[$dx] * $wy[$dy];
            $sum_lr += $w * _decode($r);
            $sum_lg += $w * _decode($g);
            $sum_lb += $w * _decode($b);
            $sum_la += $w * $a;
        }
    }

    return _encode($sum_lr), _encode($sum_lg), _encode($sum_lb), _clamp_byte($sum_la);
}

sub _sample {
    my ($img, $u, $v, $mode, $oob) = @_;
    return _sample_nearest($img, $u, $v, $oob) if defined $mode && $mode eq 'nearest';
    return _sample_bicubic($img, $u, $v, $oob) if defined $mode && $mode eq 'bicubic';
    return _sample_bilinear($img, $u, $v, $oob);
}

sub flip_horizontal {
    my ($src) = @_;
    my ($w, $h) = ($src->width, $src->height);
    my $out = CodingAdventures::PixelContainer->new($w, $h);
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            $out->set_pixel($w - 1 - $x, $y, $src->pixel_at($x, $y));
        }
    }
    return $out;
}

sub flip_vertical {
    my ($src) = @_;
    my ($w, $h) = ($src->width, $src->height);
    my $out = CodingAdventures::PixelContainer->new($w, $h);
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            $out->set_pixel($x, $h - 1 - $y, $src->pixel_at($x, $y));
        }
    }
    return $out;
}

sub rotate_90_cw {
    my ($src) = @_;
    my ($w, $h) = ($src->width, $src->height);
    my $out = CodingAdventures::PixelContainer->new($h, $w);
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            $out->set_pixel($h - 1 - $y, $x, $src->pixel_at($x, $y));
        }
    }
    return $out;
}

sub rotate_90_ccw {
    my ($src) = @_;
    my ($w, $h) = ($src->width, $src->height);
    my $out = CodingAdventures::PixelContainer->new($h, $w);
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            $out->set_pixel($y, $w - 1 - $x, $src->pixel_at($x, $y));
        }
    }
    return $out;
}

sub rotate_180 {
    my ($src) = @_;
    my ($w, $h) = ($src->width, $src->height);
    my $out = CodingAdventures::PixelContainer->new($w, $h);
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            $out->set_pixel($w - 1 - $x, $h - 1 - $y, $src->pixel_at($x, $y));
        }
    }
    return $out;
}

sub crop {
    my ($src, $x0, $y0, $w, $h) = @_;
    my $out = CodingAdventures::PixelContainer->new($w, $h);
    for my $dy (0 .. $h - 1) {
        for my $dx (0 .. $w - 1) {
            $out->set_pixel($dx, $dy, $src->pixel_at($x0 + $dx, $y0 + $dy));
        }
    }
    return $out;
}

sub pad {
    my ($src, $top, $right, $bottom, $left, $fill) = @_;
    $fill //= [0, 0, 0, 0];
    my ($fr, $fg, $fb, $fa) = @$fill;
    my ($w, $h) = ($src->width, $src->height);
    my ($out_w, $out_h) = ($w + $left + $right, $h + $top + $bottom);
    my $out = CodingAdventures::PixelContainer->new($out_w, $out_h);
    $out->fill_pixels($fr, $fg, $fb, $fa);

    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            $out->set_pixel($x + $left, $y + $top, $src->pixel_at($x, $y));
        }
    }
    return $out;
}

sub scale {
    my ($src, $out_w, $out_h, $mode) = @_;
    $mode //= 'bilinear';
    my ($w, $h) = ($src->width, $src->height);
    my $out = CodingAdventures::PixelContainer->new($out_w, $out_h);
    for my $oy (0 .. $out_h - 1) {
        for my $ox (0 .. $out_w - 1) {
            my $u = ($ox + 0.5) * $w / $out_w - 0.5;
            my $v = ($oy + 0.5) * $h / $out_h - 0.5;
            $out->set_pixel($ox, $oy, _sample($src, $u, $v, $mode, 'replicate'));
        }
    }
    return $out;
}

sub rotate {
    my ($src, $radians, $mode, $bounds) = @_;
    $mode //= 'bilinear';
    $bounds //= 'fit';
    my ($w, $h) = ($src->width, $src->height);
    my $cos_a = cos($radians);
    my $sin_a = sin($radians);
    my $out_w = $bounds eq 'fit' ? ceil($w * abs($cos_a) + $h * abs($sin_a)) : $w;
    my $out_h = $bounds eq 'fit' ? ceil($w * abs($sin_a) + $h * abs($cos_a)) : $h;
    my ($cx_in, $cy_in) = ($w / 2.0, $h / 2.0);
    my ($cx_out, $cy_out) = ($out_w / 2.0, $out_h / 2.0);
    my $out = CodingAdventures::PixelContainer->new($out_w, $out_h);

    for my $oy (0 .. $out_h - 1) {
        for my $ox (0 .. $out_w - 1) {
            my $dx = ($ox + 0.5) - $cx_out;
            my $dy = ($oy + 0.5) - $cy_out;
            my $u = $cx_in + $cos_a * $dx + $sin_a * $dy - 0.5;
            my $v = $cy_in - $sin_a * $dx + $cos_a * $dy - 0.5;
            $out->set_pixel($ox, $oy, _sample($src, $u, $v, $mode, 'zero'));
        }
    }
    return $out;
}

sub affine {
    my ($src, $matrix, $out_w, $out_h, $mode, $oob) = @_;
    $mode //= 'bilinear';
    $oob //= 'zero';
    my ($a, $b, $tx) = @{ $matrix->[0] };
    my ($c, $d, $ty) = @{ $matrix->[1] };
    my $det = $a * $d - $b * $c;
    die "image_geometric_transforms.affine: matrix is singular (det ~= 0)"
        if abs($det) < 1e-10;

    my $inv_a  =  $d / $det;
    my $inv_b  = -$b / $det;
    my $inv_tx = ($b * $ty - $d * $tx) / $det;
    my $inv_c  = -$c / $det;
    my $inv_d  =  $a / $det;
    my $inv_ty = ($c * $tx - $a * $ty) / $det;
    my $out = CodingAdventures::PixelContainer->new($out_w, $out_h);

    for my $oy (0 .. $out_h - 1) {
        for my $ox (0 .. $out_w - 1) {
            my $px = $ox + 0.5;
            my $py = $oy + 0.5;
            my $u = $inv_a * $px + $inv_b * $py + $inv_tx - 0.5;
            my $v = $inv_c * $px + $inv_d * $py + $inv_ty - 0.5;
            $out->set_pixel($ox, $oy, _sample($src, $u, $v, $mode, $oob));
        }
    }
    return $out;
}

sub perspective_warp {
    my ($src, $h, $out_w, $out_h, $mode, $oob) = @_;
    $mode //= 'bilinear';
    $oob //= 'zero';
    my ($h00, $h01, $h02) = @{ $h->[0] };
    my ($h10, $h11, $h12) = @{ $h->[1] };
    my ($h20, $h21, $h22) = @{ $h->[2] };

    my $det = $h00 * ($h11 * $h22 - $h12 * $h21)
            - $h01 * ($h10 * $h22 - $h12 * $h20)
            + $h02 * ($h10 * $h21 - $h11 * $h20);
    die "image_geometric_transforms.perspective_warp: homography matrix is singular"
        if abs($det) < 1e-10;

    my $inv_det = 1.0 / $det;
    my $i00 = ($h11 * $h22 - $h12 * $h21) * $inv_det;
    my $i01 = ($h02 * $h21 - $h01 * $h22) * $inv_det;
    my $i02 = ($h01 * $h12 - $h02 * $h11) * $inv_det;
    my $i10 = ($h12 * $h20 - $h10 * $h22) * $inv_det;
    my $i11 = ($h00 * $h22 - $h02 * $h20) * $inv_det;
    my $i12 = ($h02 * $h10 - $h00 * $h12) * $inv_det;
    my $i20 = ($h10 * $h21 - $h11 * $h20) * $inv_det;
    my $i21 = ($h01 * $h20 - $h00 * $h21) * $inv_det;
    my $i22 = ($h00 * $h11 - $h01 * $h10) * $inv_det;
    my $out = CodingAdventures::PixelContainer->new($out_w, $out_h);

    for my $oy (0 .. $out_h - 1) {
        for my $ox (0 .. $out_w - 1) {
            my $px = $ox + 0.5;
            my $py = $oy + 0.5;
            my $wx = $i00 * $px + $i01 * $py + $i02;
            my $wy = $i10 * $px + $i11 * $py + $i12;
            my $ww = $i20 * $px + $i21 * $py + $i22;
            my $u = $wx / $ww - 0.5;
            my $v = $wy / $ww - 0.5;
            $out->set_pixel($ox, $oy, _sample($src, $u, $v, $mode, $oob));
        }
    }
    return $out;
}

1;

__END__

=head1 NAME

CodingAdventures::ImageGeometricTransforms - Geometric transforms for Perl PixelContainer images

=head1 SYNOPSIS

    use CodingAdventures::ImageGeometricTransforms;

=head1 DESCRIPTION

Geometric transforms for Perl PixelContainer images

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
