use strict;
use warnings;
use Test2::V0;

use lib '../pixel-container/lib', 'lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImagePointOps qw(
    invert threshold threshold_luminance posterize swap_rgb_bgr extract_channel
    brightness contrast gamma exposure greyscale sepia colour_matrix saturate
    hue_rotate srgb_to_linear_image linear_to_srgb_image apply_lut1d_u8
    build_lut1d_u8 build_gamma_lut
);

sub solid {
    my ($r, $g, $b, $a) = @_;
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, $r, $g, $b, $a);
    return $img;
}

sub px {
    my ($img) = @_;
    return [ $img->pixel_at(0, 0) ];
}

sub close_enough {
    my ($actual, $expected, $tolerance, $label) = @_;
    ok(abs($actual - $expected) <= $tolerance, "$label: $actual within $tolerance of $expected");
}

subtest 'dimensions are preserved' => sub {
    my $img = CodingAdventures::PixelContainer->new(3, 5);
    my $out = invert($img);
    is($out->width, 3, 'width preserved');
    is($out->height, 5, 'height preserved');
};

subtest 'u8-domain operations' => sub {
    is(px(invert(solid(10, 100, 200, 128))), [245, 155, 55, 128], 'invert flips RGB and preserves alpha');
    is(px(threshold(solid(200, 200, 200, 255), 128)), [255, 255, 255, 255], 'threshold above is white');
    is(px(threshold(solid(50, 50, 50, 255), 128))->[0], 0, 'threshold below is black');
    is(px(threshold_luminance(solid(255, 255, 255, 255), 128))->[0], 255, 'luminance threshold keeps white');
    ok(px(posterize(solid(50, 50, 50, 255), 2))->[0] == 0 || px(posterize(solid(50, 50, 50, 255), 2))->[0] == 255, 'posterize to two levels');
    is(px(swap_rgb_bgr(solid(255, 0, 0, 255))), [0, 0, 255, 255], 'swap RGB/BGR');
    is(px(extract_channel(solid(100, 150, 200, 255), 1)), [0, 150, 0, 255], 'extract green channel');
    is(px(brightness(solid(250, 10, 5, 255), 20)), [255, 30, 25, 255], 'brightness clamps high');
    is(px(brightness(solid(5, 10, 10, 255), -20))->[0], 0, 'brightness clamps low');
};

subtest 'linear-light operations' => sub {
    my $img = solid(100, 150, 200, 255);
    my $contrast = contrast($img, 1.0);
    close_enough(px($contrast)->[0], px($img)->[0], 1, 'contrast identity red');

    my $gamma = gamma(solid(128, 128, 128, 255), 0.5);
    ok(px($gamma)->[0] > 128, 'gamma below one brightens midtones');

    my $exposure = exposure(solid(100, 100, 100, 255), 1.0);
    ok(px($exposure)->[0] > 100, 'positive exposure brightens');

    for my $method ('rec709', 'bt601', 'average') {
        is(px(greyscale(solid(255, 255, 255, 255), $method)), [255, 255, 255, 255], "$method white stays white");
    }

    is(px(sepia(solid(128, 128, 128, 200)))->[3], 200, 'sepia preserves alpha');

    my $identity = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
    my $matrix = colour_matrix($img, $identity);
    close_enough(px($matrix)->[2], px($img)->[2], 1, 'identity colour matrix blue');

    my $grey = saturate(solid(200, 100, 50, 255), 0.0);
    is(px($grey)->[0], px($grey)->[1], 'saturate zero equalizes red/green');

    my $rotated = hue_rotate(solid(200, 80, 40, 255), 360.0);
    close_enough(px($rotated)->[0], 200, 2, '360-degree hue rotate red');
};

subtest 'colourspace and LUT helpers' => sub {
    my $img = solid(100, 150, 200, 255);
    my $roundtrip = linear_to_srgb_image(srgb_to_linear_image($img));
    close_enough(px($roundtrip)->[0], 100, 2, 'sRGB/linear roundtrip red');
    close_enough(px($roundtrip)->[1], 150, 2, 'sRGB/linear roundtrip green');

    my @lut = map { 255 - $_ } 0 .. 255;
    is(px(apply_lut1d_u8(solid(100, 0, 200, 255), \@lut, \@lut, \@lut)), [155, 255, 55, 255], 'apply invert LUT');

    my $identity = build_lut1d_u8(sub { $_[0] });
    close_enough($identity->[128], 128, 1, 'identity LUT midpoint');

    my $gamma_identity = build_gamma_lut(1.0);
    close_enough($gamma_identity->[200], 200, 1, 'gamma=1 LUT identity');
};

done_testing;
