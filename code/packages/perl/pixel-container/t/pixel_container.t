use strict;
use warnings;
use Test2::V0;

use lib '../lib', 'lib';
use CodingAdventures::PixelContainer;

# ============================================================================
# PixelContainer test suite
# ============================================================================
#
# Tests cover:
#   - Construction (valid, invalid dimensions)
#   - Accessors: width, height, data
#   - pixel_at: normal, out-of-bounds
#   - set_pixel: normal, out-of-bounds (no-op)
#   - fill_pixels
#   - Round-trip: set then read back
#   - Edge: 1×1 image
#   - Data reference correctness
# ============================================================================

# ----------------------------------------------------------------------------
# Construction
# ----------------------------------------------------------------------------

subtest 'new: valid construction 10x20' => sub {
    my $img = CodingAdventures::PixelContainer->new(10, 20);
    ok(defined $img, 'object created');
    is($img->width,  10, 'width = 10');
    is($img->height, 20, 'height = 20');
};

subtest 'new: valid construction 1x1' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    is($img->width,  1, 'width = 1');
    is($img->height, 1, 'height = 1');
};

subtest 'new: initial pixels are all zeros' => sub {
    my $img = CodingAdventures::PixelContainer->new(3, 3);
    for my $y (0..2) {
        for my $x (0..2) {
            my ($r, $g, $b, $a) = $img->pixel_at($x, $y);
            is($r, 0, "($x,$y) R=0");
            is($g, 0, "($x,$y) G=0");
            is($b, 0, "($x,$y) B=0");
            is($a, 0, "($x,$y) A=0");
        }
    }
};

subtest 'new: invalid width dies' => sub {
    like(dies { CodingAdventures::PixelContainer->new(0, 10) },
         qr/InvalidInput/, 'width=0 dies');
    like(dies { CodingAdventures::PixelContainer->new(-1, 10) },
         qr/InvalidInput/, 'negative width dies');
    like(dies { CodingAdventures::PixelContainer->new(undef, 10) },
         qr/InvalidInput/, 'undef width dies');
};

subtest 'new: invalid height dies' => sub {
    like(dies { CodingAdventures::PixelContainer->new(10, 0) },
         qr/InvalidInput/, 'height=0 dies');
    like(dies { CodingAdventures::PixelContainer->new(10, -5) },
         qr/InvalidInput/, 'negative height dies');
    like(dies { CodingAdventures::PixelContainer->new(10, undef) },
         qr/InvalidInput/, 'undef height dies');
};

# ----------------------------------------------------------------------------
# data() accessor
# ----------------------------------------------------------------------------

subtest 'data: returns scalar reference' => sub {
    my $img = CodingAdventures::PixelContainer->new(4, 4);
    my $ref = $img->data;
    ok(ref($ref) eq 'SCALAR', 'data() returns a SCALAR ref');
};

subtest 'data: buffer length = width * height * 4' => sub {
    my $img = CodingAdventures::PixelContainer->new(5, 3);
    my $ref = $img->data;
    is(length($$ref), 5 * 3 * 4, 'buffer length = 60');
};

subtest 'data: 1x1 buffer is 4 bytes' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    is(length(${ $img->data }), 4, '1×1 buffer is 4 bytes');
};

# ----------------------------------------------------------------------------
# set_pixel / pixel_at round-trips
# ----------------------------------------------------------------------------

subtest 'set_pixel + pixel_at: red pixel at (0,0)' => sub {
    my $img = CodingAdventures::PixelContainer->new(5, 5);
    $img->set_pixel(0, 0, 255, 0, 0, 255);
    my ($r, $g, $b, $a) = $img->pixel_at(0, 0);
    is($r, 255, 'R=255');
    is($g, 0,   'G=0');
    is($b, 0,   'B=0');
    is($a, 255, 'A=255');
};

subtest 'set_pixel + pixel_at: green pixel at (3,2)' => sub {
    my $img = CodingAdventures::PixelContainer->new(10, 10);
    $img->set_pixel(3, 2, 0, 128, 0, 200);
    my ($r, $g, $b, $a) = $img->pixel_at(3, 2);
    is($r, 0,   'R=0');
    is($g, 128, 'G=128');
    is($b, 0,   'B=0');
    is($a, 200, 'A=200');
};

subtest 'set_pixel + pixel_at: multiple pixels do not overlap' => sub {
    my $img = CodingAdventures::PixelContainer->new(4, 4);
    $img->set_pixel(0, 0, 10, 20, 30, 40);
    $img->set_pixel(1, 0, 50, 60, 70, 80);
    $img->set_pixel(0, 1, 90, 100, 110, 120);

    my @p00 = $img->pixel_at(0, 0);
    my @p10 = $img->pixel_at(1, 0);
    my @p01 = $img->pixel_at(0, 1);
    my @p11 = $img->pixel_at(1, 1);

    is(\@p00, [10,  20,  30,  40],  '(0,0) correct');
    is(\@p10, [50,  60,  70,  80],  '(1,0) correct');
    is(\@p01, [90, 100, 110, 120], '(0,1) correct');
    is(\@p11, [0,    0,   0,   0], '(1,1) still zero (untouched)');
};

subtest 'set_pixel + pixel_at: last pixel in image (bottom-right)' => sub {
    my $img = CodingAdventures::PixelContainer->new(8, 6);
    $img->set_pixel(7, 5, 1, 2, 3, 4);
    my ($r, $g, $b, $a) = $img->pixel_at(7, 5);
    is($r, 1, 'R=1');
    is($g, 2, 'G=2');
    is($b, 3, 'B=3');
    is($a, 4, 'A=4');
};

subtest 'set_pixel + pixel_at: max values (255) round-trip' => sub {
    my $img = CodingAdventures::PixelContainer->new(2, 2);
    $img->set_pixel(1, 1, 255, 255, 255, 255);
    my ($r, $g, $b, $a) = $img->pixel_at(1, 1);
    is($r, 255, 'R=255');
    is($g, 255, 'G=255');
    is($b, 255, 'B=255');
    is($a, 255, 'A=255');
};

subtest 'set_pixel + pixel_at: overwrite existing pixel' => sub {
    my $img = CodingAdventures::PixelContainer->new(3, 3);
    $img->set_pixel(1, 1, 10, 20, 30, 40);
    $img->set_pixel(1, 1, 50, 60, 70, 80);   # overwrite
    my ($r, $g, $b, $a) = $img->pixel_at(1, 1);
    is($r, 50, 'R overwritten to 50');
    is($g, 60, 'G overwritten to 60');
    is($b, 70, 'B overwritten to 70');
    is($a, 80, 'A overwritten to 80');
};

# ----------------------------------------------------------------------------
# Out-of-bounds behaviour
# ----------------------------------------------------------------------------

subtest 'pixel_at: out-of-bounds returns (0,0,0,0)' => sub {
    my $img = CodingAdventures::PixelContainer->new(5, 5);
    $img->fill_pixels(100, 100, 100, 100);

    # All these coordinates are outside the 5×5 grid
    for my $coord ([-1, 0], [0, -1], [5, 0], [0, 5], [10, 10], [-5, -5]) {
        my ($x, $y) = @$coord;
        my ($r, $g, $b, $a) = $img->pixel_at($x, $y);
        is($r, 0, "pixel_at($x,$y) R=0 (OOB)");
        is($a, 0, "pixel_at($x,$y) A=0 (OOB)");
    }
};

subtest 'set_pixel: out-of-bounds is a no-op' => sub {
    my $img = CodingAdventures::PixelContainer->new(3, 3);
    # These should not die or alter the buffer
    $img->set_pixel(-1, 0, 255, 255, 255, 255);
    $img->set_pixel(0, -1, 255, 255, 255, 255);
    $img->set_pixel(3, 0,  255, 255, 255, 255);
    $img->set_pixel(0, 3,  255, 255, 255, 255);

    # Verify the buffer is still all zeros
    for my $y (0..2) {
        for my $x (0..2) {
            my ($r, $g, $b, $a) = $img->pixel_at($x, $y);
            is($r + $g + $b + $a, 0, "($x,$y) still zero after OOB writes");
        }
    }
};

# ----------------------------------------------------------------------------
# fill_pixels
# ----------------------------------------------------------------------------

subtest 'fill_pixels: fills every pixel with given colour' => sub {
    my $img = CodingAdventures::PixelContainer->new(4, 3);
    $img->fill_pixels(123, 45, 67, 200);

    for my $y (0..2) {
        for my $x (0..3) {
            my ($r, $g, $b, $a) = $img->pixel_at($x, $y);
            is($r, 123, "($x,$y) R=123");
            is($g, 45,  "($x,$y) G=45");
            is($b, 67,  "($x,$y) B=67");
            is($a, 200, "($x,$y) A=200");
        }
    }
};

subtest 'fill_pixels: transparent black fill' => sub {
    my $img = CodingAdventures::PixelContainer->new(2, 2);
    $img->set_pixel(0, 0, 1, 2, 3, 4);
    $img->fill_pixels(0, 0, 0, 0);   # reset to transparent black
    my ($r, $g, $b, $a) = $img->pixel_at(0, 0);
    is($r + $g + $b + $a, 0, 'pixel zeroed after fill_pixels(0,0,0,0)');
};

subtest 'fill_pixels: buffer length unchanged after fill' => sub {
    my $img = CodingAdventures::PixelContainer->new(6, 4);
    my $before = length(${ $img->data });
    $img->fill_pixels(255, 128, 64, 255);
    my $after = length(${ $img->data });
    is($before, $after, 'buffer length unchanged after fill');
};

# ----------------------------------------------------------------------------
# 1×1 edge case
# ----------------------------------------------------------------------------

subtest '1x1 image: set and get single pixel' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 42, 43, 44, 45);
    my ($r, $g, $b, $a) = $img->pixel_at(0, 0);
    is($r, 42, 'R=42');
    is($g, 43, 'G=43');
    is($b, 44, 'B=44');
    is($a, 45, 'A=45');
};

subtest '1x1 image: OOB at (1,0) returns zero' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 99, 99, 99, 99);
    my ($r, $g, $b, $a) = $img->pixel_at(1, 0);
    is($r, 0, 'R=0 (OOB)');
};

done_testing;
