use strict;
use warnings;
use Test2::V0;

use lib '../pixel-container/lib', 'lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageGeometricTransforms qw(
    flip_horizontal flip_vertical rotate_90_cw rotate_90_ccw rotate_180
    crop pad scale rotate affine perspective_warp
);

sub solid {
    my ($w, $h, $r, $g, $b, $a) = @_;
    my $img = CodingAdventures::PixelContainer->new($w, $h);
    $img->fill_pixels($r, $g, $b, $a);
    return $img;
}

sub at {
    my ($img, $x, $y) = @_;
    return [ $img->pixel_at($x, $y) ];
}

sub images_equal {
    my ($a, $b, $label) = @_;
    is($a->width, $b->width, "$label width");
    is($a->height, $b->height, "$label height");
    for my $y (0 .. $a->height - 1) {
        for my $x (0 .. $a->width - 1) {
            is(at($a, $x, $y), at($b, $x, $y), "$label pixel $x,$y");
        }
    }
}

sub images_close {
    my ($a, $b, $tol, $label) = @_;
    is($a->width, $b->width, "$label width");
    is($a->height, $b->height, "$label height");
    for my $y (0 .. $a->height - 1) {
        for my $x (0 .. $a->width - 1) {
            my @av = $a->pixel_at($x, $y);
            my @bv = $b->pixel_at($x, $y);
            for my $i (0 .. 2) {
                ok(abs($av[$i] - $bv[$i]) <= $tol, "$label channel $i at $x,$y");
            }
        }
    }
}

subtest 'lossless transforms' => sub {
    my $src = CodingAdventures::PixelContainer->new(4, 3);
    $src->set_pixel(0, 0, 10, 20, 30, 255);
    $src->set_pixel(3, 2, 100, 200, 50, 128);
    images_equal($src, flip_horizontal(flip_horizontal($src)), 'double horizontal flip');
    images_equal($src, flip_vertical(flip_vertical($src)), 'double vertical flip');

    my $cw = rotate_90_cw($src);
    is($cw->width, 3, 'rotate_90_cw width');
    is($cw->height, 4, 'rotate_90_cw height');
    is(at($cw, 2, 0), [10, 20, 30, 255], 'top-left moves to top-right');
    images_equal($src, rotate_90_ccw(rotate_90_cw($src)), 'CW then CCW identity');
    images_equal($src, rotate_180(rotate_180($src)), 'double rotate_180 identity');
};

subtest 'crop and pad' => sub {
    my $src = CodingAdventures::PixelContainer->new(5, 5);
    $src->set_pixel(2, 3, 100, 150, 200, 255);
    my $cropped = crop($src, 2, 3, 2, 2);
    is($cropped->width, 2, 'crop width');
    is($cropped->height, 2, 'crop height');
    is(at($cropped, 0, 0), [100, 150, 200, 255], 'crop copies source pixel');
    is(at(crop($src, 4, 4, 2, 2), 1, 1), [0, 0, 0, 0], 'crop OOB is transparent black');

    my $padded = pad($cropped, 1, 1, 1, 1, [200, 100, 50, 255]);
    is($padded->width, 4, 'pad width');
    is($padded->height, 4, 'pad height');
    is(at($padded, 0, 0), [200, 100, 50, 255], 'pad applies fill');
    is(at($padded, 1, 1), [100, 150, 200, 255], 'pad preserves interior');
};

subtest 'scale and rotate' => sub {
    my $src = solid(4, 4, 128, 64, 200, 255);
    my $scaled = scale($src, 8, 6, 'bilinear');
    is($scaled->width, 8, 'scale output width');
    is($scaled->height, 6, 'scale output height');
    images_close($src, scale($src, 4, 4, 'nearest'), 1, 'nearest scale identity');
    images_close($src, scale($src, 4, 4, 'bilinear'), 2, 'bilinear scale identity');
    images_close($src, scale($src, 4, 4, 'bicubic'), 2, 'bicubic scale identity');

    my $rotated = rotate($src, 0.0, 'bilinear', 'crop');
    images_close($src, $rotated, 2, 'zero rotation identity');
    my $fit = rotate(CodingAdventures::PixelContainer->new(10, 10), 3.141592653589793 / 4, 'nearest', 'fit');
    ok($fit->width > 10, 'fit rotation grows width');
    ok($fit->height > 10, 'fit rotation grows height');
};

subtest 'affine and perspective transforms' => sub {
    my $src = solid(5, 5, 200, 100, 50, 255);
    my $id2 = [[1, 0, 0], [0, 1, 0]];
    images_close($src, affine($src, $id2, 5, 5, 'bilinear', 'replicate'), 2, 'affine identity');

    my $translated = affine($src, [[1, 0, 2], [0, 1, 1]], 5, 5, 'nearest', 'zero');
    is(at($translated, 0, 0), [0, 0, 0, 0], 'translation exposes transparent edge');
    is(at($translated, 2, 1), [200, 100, 50, 255], 'translation shifts source pixel');

    ok(!eval { affine($src, [[0, 0, 0], [0, 0, 0]], 4, 4); 1 }, 'singular affine dies');

    my $id3 = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
    images_close($src, perspective_warp($src, $id3, 5, 5, 'bilinear', 'replicate'), 2, 'perspective identity');
    ok(!eval { perspective_warp($src, [[0, 0, 0], [0, 0, 0], [0, 0, 0]], 4, 4); 1 }, 'singular perspective dies');
};

subtest 'OOB modes and interpolation' => sub {
    my $edge = CodingAdventures::PixelContainer->new(3, 3);
    $edge->set_pixel(0, 0, 200, 100, 50, 255);
    my $replicate = affine($edge, [[1, 0, 5], [0, 1, 0]], 3, 3, 'nearest', 'replicate');
    is(at($replicate, 0, 0), [200, 100, 50, 255], 'replicate clamps to edge');

    my $tile = CodingAdventures::PixelContainer->new(2, 1);
    $tile->set_pixel(0, 0, 255, 0, 0, 255);
    $tile->set_pixel(1, 0, 0, 0, 255, 255);
    my $wrapped = affine($tile, [[1, 0, 2], [0, 1, 0]], 2, 1, 'nearest', 'wrap');
    my $pix = at($wrapped, 0, 0);
    ok(($pix->[0] == 255 && $pix->[2] == 0) || ($pix->[0] == 0 && $pix->[2] == 255), 'wrap samples a tiled pixel');

    my $gradient = CodingAdventures::PixelContainer->new(2, 1);
    $gradient->set_pixel(0, 0, 0, 0, 0, 255);
    $gradient->set_pixel(1, 0, 255, 255, 255, 255);
    my $blend = scale($gradient, 4, 1, 'bilinear');
    my $r = at($blend, 1, 0)->[0];
    ok($r > 10 && $r < 245, "bilinear blend is between endpoints: $r");
};

done_testing;
