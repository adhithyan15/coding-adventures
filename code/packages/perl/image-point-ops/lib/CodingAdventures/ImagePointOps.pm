package CodingAdventures::ImagePointOps;

# ============================================================================
# CodingAdventures::ImagePointOps — Per-pixel point operations for Perl PixelContainer images
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::ImagePointOps;
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';
use lib '../pixel-container/lib';

use CodingAdventures::PixelContainer;

our $VERSION = '0.01';
our @EXPORT_OK = qw(
    invert threshold threshold_luminance posterize swap_rgb_bgr extract_channel
    brightness contrast gamma exposure greyscale sepia colour_matrix saturate
    hue_rotate srgb_to_linear_image linear_to_srgb_image apply_lut1d_u8
    build_lut1d_u8 build_gamma_lut
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
    my ($linear) = @_;
    my $c = $linear <= 0.0031308
        ? $linear * 12.92
        : 1.055 * ($linear ** (1.0 / 2.4)) - 0.055;
    $c = 0.0 if $c < 0.0;
    $c = 1.0 if $c > 1.0;
    return int($c * 255 + 0.5);
}

sub _byte {
    my ($value) = @_;
    $value = 0 if !defined $value || $value < 0;
    $value = 255 if $value > 255;
    return int($value + 0.5);
}

sub _mod_float {
    my ($value, $modulus) = @_;
    return $value - int($value / $modulus) * $modulus if $value >= 0;
    return $value - (int($value / $modulus) - 1) * $modulus;
}

sub _map_pixels {
    my ($src, $fn) = @_;
    my $out = CodingAdventures::PixelContainer->new($src->width, $src->height);
    for my $y (0 .. $src->height - 1) {
        for my $x (0 .. $src->width - 1) {
            my ($r, $g, $b, $a) = $src->pixel_at($x, $y);
            my ($nr, $ng, $nb, $na) = $fn->($r, $g, $b, $a);
            $out->set_pixel($x, $y, _byte($nr), _byte($ng), _byte($nb), _byte($na));
        }
    }
    return $out;
}

sub invert {
    my ($src) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return 255 - $r, 255 - $g, 255 - $b, $a;
    });
}

sub threshold {
    my ($src, $value) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my $luma = int(($r + $g + $b) / 3);
        my $v = $luma >= $value ? 255 : 0;
        return $v, $v, $v, $a;
    });
}

sub threshold_luminance {
    my ($src, $value) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my $luma = 0.2126 * $r + 0.7152 * $g + 0.0722 * $b;
        my $v = $luma >= $value ? 255 : 0;
        return $v, $v, $v, $a;
    });
}

sub posterize {
    my ($src, $levels) = @_;
    my $step = 255.0 / ($levels - 1);
    my $quantize = sub {
        my ($v) = @_;
        return int(int($v / $step + 0.5) * $step + 0.5);
    };
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return $quantize->($r), $quantize->($g), $quantize->($b), $a;
    });
}

sub swap_rgb_bgr {
    my ($src) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return $b, $g, $r, $a;
    });
}

sub extract_channel {
    my ($src, $channel) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return $r, 0, 0, $a if $channel == 0;
        return 0, $g, 0, $a if $channel == 1;
        return 0, 0, $b, $a if $channel == 2;
        return $r, $g, $b, $a;
    });
}

sub brightness {
    my ($src, $offset) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return _byte($r + $offset), _byte($g + $offset), _byte($b + $offset), $a;
    });
}

sub contrast {
    my ($src, $factor) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return _encode(0.5 + $factor * (_decode($r) - 0.5)),
               _encode(0.5 + $factor * (_decode($g) - 0.5)),
               _encode(0.5 + $factor * (_decode($b) - 0.5)),
               $a;
    });
}

sub gamma {
    my ($src, $g) = @_;
    return _map_pixels($src, sub {
        my ($r, $gv, $b, $a) = @_;
        return _encode(_decode($r) ** $g),
               _encode(_decode($gv) ** $g),
               _encode(_decode($b) ** $g),
               $a;
    });
}

sub exposure {
    my ($src, $stops) = @_;
    my $factor = 2 ** $stops;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return _encode(_decode($r) * $factor),
               _encode(_decode($g) * $factor),
               _encode(_decode($b) * $factor),
               $a;
    });
}

sub greyscale {
    my ($src, $method) = @_;
    $method //= 'rec709';
    my ($wr, $wg, $wb) = $method eq 'bt601' ? (0.2989, 0.5870, 0.1140)
        : $method eq 'average' ? (1 / 3, 1 / 3, 1 / 3)
        : (0.2126, 0.7152, 0.0722);

    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my $y = _encode($wr * _decode($r) + $wg * _decode($g) + $wb * _decode($b));
        return $y, $y, $y, $a;
    });
}

sub sepia {
    my ($src) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my ($lr, $lg, $lb) = (_decode($r), _decode($g), _decode($b));
        return _encode(0.393 * $lr + 0.769 * $lg + 0.189 * $lb),
               _encode(0.349 * $lr + 0.686 * $lg + 0.168 * $lb),
               _encode(0.272 * $lr + 0.534 * $lg + 0.131 * $lb),
               $a;
    });
}

sub colour_matrix {
    my ($src, $matrix) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my ($lr, $lg, $lb) = (_decode($r), _decode($g), _decode($b));
        return _encode($matrix->[0][0] * $lr + $matrix->[0][1] * $lg + $matrix->[0][2] * $lb),
               _encode($matrix->[1][0] * $lr + $matrix->[1][1] * $lg + $matrix->[1][2] * $lb),
               _encode($matrix->[2][0] * $lr + $matrix->[2][1] * $lg + $matrix->[2][2] * $lb),
               $a;
    });
}

sub saturate {
    my ($src, $factor) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my ($lr, $lg, $lb) = (_decode($r), _decode($g), _decode($b));
        my $grey = 0.2126 * $lr + 0.7152 * $lg + 0.0722 * $lb;
        return _encode($grey + $factor * ($lr - $grey)),
               _encode($grey + $factor * ($lg - $grey)),
               _encode($grey + $factor * ($lb - $grey)),
               $a;
    });
}

sub _rgb_to_hsv {
    my ($r, $g, $b) = @_;
    my $max = $r > $g ? ($r > $b ? $r : $b) : ($g > $b ? $g : $b);
    my $min = $r < $g ? ($r < $b ? $r : $b) : ($g < $b ? $g : $b);
    my $delta = $max - $min;
    my $v = $max;
    my $s = $max == 0 ? 0 : $delta / $max;
    my $h = 0;

    if ($delta != 0) {
        if ($max == $r) {
            $h = _mod_float(($g - $b) / $delta, 6);
        } elsif ($max == $g) {
            $h = ($b - $r) / $delta + 2;
        } else {
            $h = ($r - $g) / $delta + 4;
        }
        $h = _mod_float($h * 60 + 360, 360);
    }

    return $h, $s, $v;
}

sub _hsv_to_rgb {
    my ($h, $s, $v) = @_;
    my $c = $v * $s;
    my $x = $c * (1 - abs(_mod_float($h / 60, 2) - 1));
    my $m = $v - $c;
    my ($r, $g, $b);
    my $sector = int($h / 60);
    if ($sector == 0) {
        ($r, $g, $b) = ($c, $x, 0);
    } elsif ($sector == 1) {
        ($r, $g, $b) = ($x, $c, 0);
    } elsif ($sector == 2) {
        ($r, $g, $b) = (0, $c, $x);
    } elsif ($sector == 3) {
        ($r, $g, $b) = (0, $x, $c);
    } elsif ($sector == 4) {
        ($r, $g, $b) = ($x, 0, $c);
    } else {
        ($r, $g, $b) = ($c, 0, $x);
    }
    return $r + $m, $g + $m, $b + $m;
}

sub hue_rotate {
    my ($src, $degrees) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        my ($h, $s, $v) = _rgb_to_hsv(_decode($r), _decode($g), _decode($b));
        my ($nr, $ng, $nb) = _hsv_to_rgb(_mod_float($h + $degrees + 360, 360), $s, $v);
        return _encode($nr), _encode($ng), _encode($nb), $a;
    });
}

sub srgb_to_linear_image {
    my ($src) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return int(_decode($r) * 255 + 0.5),
               int(_decode($g) * 255 + 0.5),
               int(_decode($b) * 255 + 0.5),
               $a;
    });
}

sub linear_to_srgb_image {
    my ($src) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return _encode($r / 255), _encode($g / 255), _encode($b / 255), $a;
    });
}

sub apply_lut1d_u8 {
    my ($src, $lut_r, $lut_g, $lut_b) = @_;
    return _map_pixels($src, sub {
        my ($r, $g, $b, $a) = @_;
        return $lut_r->[$r], $lut_g->[$g], $lut_b->[$b], $a;
    });
}

sub build_lut1d_u8 {
    my ($fn) = @_;
    my @lut;
    for my $i (0 .. 255) {
        $lut[$i] = _encode($fn->(_decode($i)));
    }
    return \@lut;
}

sub build_gamma_lut {
    my ($g) = @_;
    return build_lut1d_u8(sub { $_[0] ** $g });
}

1;

__END__

=head1 NAME

CodingAdventures::ImagePointOps - Per-pixel point operations for Perl PixelContainer images

=head1 SYNOPSIS

    use CodingAdventures::ImagePointOps;

=head1 DESCRIPTION

Per-pixel point operations for Perl PixelContainer images

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
