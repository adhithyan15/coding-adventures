use strict;
use warnings;
use Test2::V0;

use lib '../pixel-container/lib', 'lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecPPM qw(encode_ppm decode_ppm mime_type);

# ============================================================================
# PPM Codec test suite
# ============================================================================
#
# Tests cover:
#   - mime_type()
#   - encode_ppm: header format, pixel data layout
#   - decode_ppm: round-trip, comment skipping, maxval scaling
#   - Error cases: bad magic, unsupported maxval, short data, undef input
# ============================================================================

# ----------------------------------------------------------------------------
# mime_type
# ----------------------------------------------------------------------------

subtest 'mime_type returns image/x-portable-pixmap' => sub {
    is(mime_type(), 'image/x-portable-pixmap', 'mime_type correct');
};

# ----------------------------------------------------------------------------
# encode_ppm header checks
# ----------------------------------------------------------------------------

subtest 'encode_ppm: starts with P6 magic' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_ppm($img);
    like($bytes, qr/^P6\n/, 'starts with P6\n');
};

subtest 'encode_ppm: header contains correct dimensions' => sub {
    my $img   = CodingAdventures::PixelContainer->new(7, 3);
    my $bytes = encode_ppm($img);
    like($bytes, qr/^P6\n7 3\n/, 'header has 7 3');
};

subtest 'encode_ppm: header contains maxval 255' => sub {
    my $img   = CodingAdventures::PixelContainer->new(4, 4);
    my $bytes = encode_ppm($img);
    like($bytes, qr/^P6\n4 4\n255\n/, 'header ends with 255\n');
};

subtest 'encode_ppm: total size = header + W*H*3' => sub {
    my $img   = CodingAdventures::PixelContainer->new(3, 2);
    my $bytes = encode_ppm($img);
    # Header: "P6\n3 2\n255\n" = 12 bytes; pixels = 3*2*3 = 18 bytes
    my $header_len = length("P6\n3 2\n255\n");
    is(length($bytes), $header_len + 3 * 2 * 3, 'total length correct');
};

subtest 'encode_ppm: pixel data follows header immediately' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 10, 20, 30, 255);
    my $bytes = encode_ppm($img);
    my $header = "P6\n1 1\n255\n";
    my $pix    = substr($bytes, length($header), 3);
    my ($r, $g, $b) = unpack('CCC', $pix);
    is($r, 10, 'R=10 in pixel data');
    is($g, 20, 'G=20 in pixel data');
    is($b, 30, 'B=30 in pixel data');
};

subtest 'encode_ppm: alpha is dropped (only 3 bytes per pixel)' => sub {
    my $img = CodingAdventures::PixelContainer->new(2, 1);
    $img->set_pixel(0, 0, 1, 2, 3, 99);
    $img->set_pixel(1, 0, 4, 5, 6, 88);
    my $bytes  = encode_ppm($img);
    my $header = "P6\n2 1\n255\n";
    # Pixel data should be exactly 6 bytes (2 pixels × 3 channels)
    my $pix_len = length($bytes) - length($header);
    is($pix_len, 6, 'pixel data is 6 bytes for 2x1 (no alpha)');
};

# ----------------------------------------------------------------------------
# Round-trip encode → decode
# ----------------------------------------------------------------------------

subtest 'round-trip: 1x1 pixel' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 200, 100, 50, 255);
    my $decoded = decode_ppm(encode_ppm($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(0, 0);
    is($r, 200, 'R=200');
    is($g, 100, 'G=100');
    is($b, 50,  'B=50');
    is($a, 255, 'A=255 (set by decoder)');
};

subtest 'round-trip: 3x2 image all pixels preserved' => sub {
    my $img = CodingAdventures::PixelContainer->new(3, 2);
    $img->set_pixel(0, 0, 10, 20, 30, 255);
    $img->set_pixel(1, 0, 40, 50, 60, 255);
    $img->set_pixel(2, 0, 70, 80, 90, 255);
    $img->set_pixel(0, 1, 100, 110, 120, 255);
    $img->set_pixel(1, 1, 130, 140, 150, 255);
    $img->set_pixel(2, 1, 160, 170, 180, 255);

    my $decoded = decode_ppm(encode_ppm($img));
    is($decoded->width,  3, 'width=3');
    is($decoded->height, 2, 'height=2');
    for my $y (0..1) {
        for my $x (0..2) {
            my @orig = $img->pixel_at($x, $y);
            $orig[3] = 255;   # alpha will be 255 in decoded
            my @got  = $decoded->pixel_at($x, $y);
            is(\@got, \@orig, "pixel ($x,$y) matches");
        }
    }
};

subtest 'round-trip: fill_pixels solid colour' => sub {
    my $img = CodingAdventures::PixelContainer->new(5, 5);
    $img->fill_pixels(0, 0, 255, 255);   # solid blue
    my $decoded = decode_ppm(encode_ppm($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(2, 2);
    is($r, 0,   'R=0 (blue)');
    is($g, 0,   'G=0');
    is($b, 255, 'B=255');
    is($a, 255, 'A=255');
};

subtest 'round-trip: dimensions preserved' => sub {
    my $img     = CodingAdventures::PixelContainer->new(11, 7);
    my $decoded = decode_ppm(encode_ppm($img));
    is($decoded->width,  11, 'width=11');
    is($decoded->height, 7,  'height=7');
};

subtest 'round-trip: max channel values preserved' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 255, 255, 255, 255);
    my $decoded = decode_ppm(encode_ppm($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(0, 0);
    is($r, 255, 'R=255');
    is($g, 255, 'G=255');
    is($b, 255, 'B=255');
};

subtest 'round-trip: zero channel values preserved' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 0, 0, 0, 255);
    my $decoded = decode_ppm(encode_ppm($img));
    my ($r, $g, $b) = $decoded->pixel_at(0, 0);
    is($r, 0, 'R=0');
    is($g, 0, 'G=0');
    is($b, 0, 'B=0');
};

# ----------------------------------------------------------------------------
# decode_ppm: comment line handling
# ----------------------------------------------------------------------------

subtest 'decode_ppm: skips comment lines in header' => sub {
    my $ppm = "P6\n# This is a comment\n# Another comment\n2 1\n255\n"
            . pack('CCC', 100, 150, 200)
            . pack('CCC', 50, 75, 100);
    my $img = decode_ppm($ppm);
    is($img->width,  2, 'width=2 despite comments');
    is($img->height, 1, 'height=1');
    my ($r, $g, $b) = $img->pixel_at(0, 0);
    is($r, 100, 'R=100');
    is($g, 150, 'G=150');
    is($b, 200, 'B=200');
};

# ----------------------------------------------------------------------------
# decode_ppm: maxval scaling
# ----------------------------------------------------------------------------

subtest 'decode_ppm: maxval=1 scales channel 1 to 255' => sub {
    # maxval=1 means each byte is 0 or 1; 1 → scale to 255
    my $ppm = "P6\n1 1\n1\n" . pack('CCC', 1, 0, 1);
    my $img = decode_ppm($ppm);
    my ($r, $g, $b) = $img->pixel_at(0, 0);
    is($r, 255, 'R scaled from 1 to 255');
    is($g, 0,   'G stays 0');
    is($b, 255, 'B scaled from 1 to 255');
};

# ----------------------------------------------------------------------------
# Error cases
# ----------------------------------------------------------------------------

subtest 'decode_ppm: rejects non-P6 magic' => sub {
    my $ppm = "P3\n1 1\n255\n255 0 0\n";
    like(dies { decode_ppm($ppm) }, qr/PPM:/, 'P3 (text PPM) dies');
};

subtest 'decode_ppm: rejects empty input' => sub {
    like(dies { decode_ppm('') }, qr/PPM:/, 'empty string dies');
};

subtest 'decode_ppm: rejects undef input' => sub {
    like(dies { decode_ppm(undef) }, qr/PPM:/, 'undef dies');
};

subtest 'decode_ppm: rejects too-short pixel data' => sub {
    # Header says 2x2 but only 3 bytes of pixel data provided (need 12)
    my $ppm = "P6\n2 2\n255\n" . pack('CCC', 255, 0, 0);
    like(dies { decode_ppm($ppm) }, qr/PPM:/, 'short pixel data dies');
};

subtest 'decode_ppm: rejects maxval > 255' => sub {
    my $ppm = "P6\n1 1\n65535\n";
    like(dies { decode_ppm($ppm) }, qr/PPM:/, 'maxval=65535 dies');
};

subtest 'encode_ppm: rejects non-container input' => sub {
    like(dies { encode_ppm('not a container') }, qr/EncodeError:/, 'string input dies');
    like(dies { encode_ppm(undef) },             qr/EncodeError:/, 'undef input dies');
};

done_testing;
