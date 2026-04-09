use strict;
use warnings;
use Test2::V0;

use lib '../pixel-container/lib', 'lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecQOI qw(encode_qoi decode_qoi mime_type);

# ============================================================================
# QOI Codec test suite
# ============================================================================
#
# Tests cover:
#   - mime_type()
#   - encode_qoi: header structure (magic, dimensions, channels, colorspace)
#   - encode_qoi: end marker present
#   - QOI_OP_RUN: single-colour image (all pixels same)
#   - QOI_OP_INDEX: pixel hash table recall
#   - QOI_OP_DIFF: small channel deltas
#   - QOI_OP_LUMA: medium channel deltas
#   - QOI_OP_RGB / QOI_OP_RGBA: raw pixel fallback
#   - Round-trip: various image types
#   - Error cases: bad magic, short file, undef input
# ============================================================================

# ----------------------------------------------------------------------------
# mime_type
# ----------------------------------------------------------------------------

subtest 'mime_type returns image/qoi' => sub {
    is(mime_type(), 'image/qoi', 'mime_type = image/qoi');
};

# ----------------------------------------------------------------------------
# Header structure
# ----------------------------------------------------------------------------

subtest 'encode_qoi: starts with qoif magic' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_qoi($img);
    is(substr($bytes, 0, 4), 'qoif', 'magic = qoif');
};

subtest 'encode_qoi: correct width and height in header (big-endian)' => sub {
    my $img   = CodingAdventures::PixelContainer->new(7, 3);
    my $bytes = encode_qoi($img);
    my ($w, $h) = unpack('N N', substr($bytes, 4, 8));
    is($w, 7, 'width = 7');
    is($h, 3, 'height = 3');
};

subtest 'encode_qoi: channels = 4 (RGBA)' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_qoi($img);
    my ($ch) = unpack('C', substr($bytes, 12, 1));
    is($ch, 4, 'channels = 4');
};

subtest 'encode_qoi: colorspace = 0 (sRGB)' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_qoi($img);
    my ($cs) = unpack('C', substr($bytes, 13, 1));
    is($cs, 0, 'colorspace = 0');
};

subtest 'encode_qoi: ends with 8-byte end marker' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_qoi($img);
    my $end   = substr($bytes, -8);
    is($end, "\x00\x00\x00\x00\x00\x00\x00\x01", 'end marker present');
};

# ----------------------------------------------------------------------------
# QOI_OP_RUN: single-colour image should be very compact
# ----------------------------------------------------------------------------

subtest 'encode_qoi: solid-colour image uses run encoding' => sub {
    # A 100x100 solid red image has 10000 pixels.
    # Initial prev=(0,0,0,255) and alpha matches, so first pixel emits OP_RGB
    # (4 bytes). Remaining 9999 pixels form runs of 62, each costing 1 byte.
    # Total: 14 header + 4 (OP_RGB) + ~162 run bytes + 8 end = ~188 bytes.
    # Raw would be 40014 bytes. Assert well under 250 bytes (already ~186x compression).
    my $img = CodingAdventures::PixelContainer->new(100, 100);
    $img->fill_pixels(255, 0, 0, 255);
    my $bytes = encode_qoi($img);
    ok(length($bytes) < 250, 'solid 100x100 image < 250 bytes (run-encoded)');
};

subtest 'encode_qoi: run encoding caps at 62 and continues' => sub {
    # A 200-pixel single row (all same colour) should use multiple run chunks.
    # 200 / 62 = 3 full runs (186 pixels) + 1 partial run (14 pixels).
    my $img = CodingAdventures::PixelContainer->new(200, 1);
    $img->fill_pixels(128, 64, 32, 255);
    my $bytes   = encode_qoi($img);
    my $decoded = decode_qoi($bytes);
    my ($r, $g, $b, $a) = $decoded->pixel_at(100, 0);
    is($r, 128, 'R=128 at midpoint of long run');
    is($g, 64,  'G=64');
    is($b, 32,  'B=32');
    is($a, 255, 'A=255');
};

# ----------------------------------------------------------------------------
# Round-trip tests
# ----------------------------------------------------------------------------

subtest 'round-trip: 1x1 pixel' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 10, 20, 30, 40);
    my $decoded = decode_qoi(encode_qoi($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(0, 0);
    is($r, 10, 'R=10');
    is($g, 20, 'G=20');
    is($b, 30, 'B=30');
    is($a, 40, 'A=40');
};

subtest 'round-trip: 2x2 checkerboard (exercises INDEX path)' => sub {
    # Alternating pixels: (255,0,0,255) and (0,255,0,255).
    # After a few pixels, the hash table will have both colours, and
    # QOI_OP_INDEX will be used to recall them.
    my $img = CodingAdventures::PixelContainer->new(8, 8);
    for my $y (0..7) {
        for my $x (0..7) {
            if (($x + $y) % 2 == 0) {
                $img->set_pixel($x, $y, 255, 0, 0, 255);
            } else {
                $img->set_pixel($x, $y, 0, 255, 0, 255);
            }
        }
    }
    my $decoded = decode_qoi(encode_qoi($img));
    is($decoded->width,  8, 'width=8');
    is($decoded->height, 8, 'height=8');
    for my $y (0..7) {
        for my $x (0..7) {
            my @orig = $img->pixel_at($x, $y);
            my @got  = $decoded->pixel_at($x, $y);
            is(\@got, \@orig, "checkerboard pixel ($x,$y) correct");
        }
    }
};

subtest 'round-trip: gradient exercises DIFF and LUMA paths' => sub {
    # Build a horizontal gradient: pixel (x,0) = (x,x,x,255) for x in 0..15.
    # Adjacent pixels differ by 1 in all channels → QOI_OP_DIFF.
    my $img = CodingAdventures::PixelContainer->new(16, 1);
    for my $x (0..15) {
        $img->set_pixel($x, 0, $x * 16, $x * 16, $x * 16, 255);
    }
    my $decoded = decode_qoi(encode_qoi($img));
    for my $x (0..15) {
        my @orig = $img->pixel_at($x, 0);
        my @got  = $decoded->pixel_at($x, 0);
        is(\@got, \@orig, "gradient pixel ($x,0) correct");
    }
};

subtest 'round-trip: varying alpha channel' => sub {
    my $img = CodingAdventures::PixelContainer->new(4, 1);
    $img->set_pixel(0, 0, 255, 0,   0,   0);
    $img->set_pixel(1, 0, 255, 0,   0,   85);
    $img->set_pixel(2, 0, 255, 0,   0,   170);
    $img->set_pixel(3, 0, 255, 0,   0,   255);
    my $decoded = decode_qoi(encode_qoi($img));
    for my $x (0..3) {
        my @orig = $img->pixel_at($x, 0);
        my @got  = $decoded->pixel_at($x, 0);
        is(\@got, \@orig, "alpha-varying pixel ($x,0) correct");
    }
};

subtest 'round-trip: all-zeros image' => sub {
    my $img = CodingAdventures::PixelContainer->new(5, 5);
    # Default is all zeros — transparent black
    my $decoded = decode_qoi(encode_qoi($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(2, 2);
    is($r, 0, 'R=0');
    is($g, 0, 'G=0');
    is($b, 0, 'B=0');
    is($a, 0, 'A=0');
};

subtest 'round-trip: all max values (255) image' => sub {
    my $img = CodingAdventures::PixelContainer->new(4, 4);
    $img->fill_pixels(255, 255, 255, 255);
    my $decoded = decode_qoi(encode_qoi($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(1, 1);
    is($r, 255, 'R=255');
    is($g, 255, 'G=255');
    is($b, 255, 'B=255');
    is($a, 255, 'A=255');
};

subtest 'round-trip: dimensions preserved' => sub {
    my $img     = CodingAdventures::PixelContainer->new(13, 7);
    my $decoded = decode_qoi(encode_qoi($img));
    is($decoded->width,  13, 'width=13');
    is($decoded->height, 7,  'height=7');
};

subtest 'round-trip: large mixed image pixel-perfect' => sub {
    # 10x10 image with distinct pixel pattern
    my $img = CodingAdventures::PixelContainer->new(10, 10);
    for my $y (0..9) {
        for my $x (0..9) {
            $img->set_pixel($x, $y,
                ($x * 25) % 256,
                ($y * 25) % 256,
                (($x + $y) * 12) % 256,
                255
            );
        }
    }
    my $decoded = decode_qoi(encode_qoi($img));
    for my $y (0..9) {
        for my $x (0..9) {
            my @orig = $img->pixel_at($x, $y);
            my @got  = $decoded->pixel_at($x, $y);
            is(\@got, \@orig, "mixed image pixel ($x,$y) correct");
        }
    }
};

subtest 'round-trip: single-row long image' => sub {
    my $img = CodingAdventures::PixelContainer->new(64, 1);
    for my $x (0..63) {
        $img->set_pixel($x, 0, $x * 4, 255 - $x * 4, 128, 255);
    }
    my $decoded = decode_qoi(encode_qoi($img));
    for my $x (0..63) {
        my @orig = $img->pixel_at($x, 0);
        my @got  = $decoded->pixel_at($x, 0);
        is(\@got, \@orig, "long row pixel ($x,0) correct");
    }
};

# ----------------------------------------------------------------------------
# QOI_OP_RGBA path (alpha changes)
# ----------------------------------------------------------------------------

subtest 'round-trip: alpha change forces RGBA op' => sub {
    # Start at (255,0,0,255) → (255,0,0,100): alpha changes by -155.
    # Cannot be DIFF or LUMA (alpha is not encoded in those ops).
    # Must use QOI_OP_RGBA.
    my $img = CodingAdventures::PixelContainer->new(2, 1);
    $img->set_pixel(0, 0, 255, 0, 0, 255);
    $img->set_pixel(1, 0, 255, 0, 0, 100);
    my $decoded = decode_qoi(encode_qoi($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(1, 0);
    is($r, 255, 'R=255');
    is($g, 0,   'G=0');
    is($b, 0,   'B=0');
    is($a, 100, 'A=100 (RGBA op used)');
};

# ----------------------------------------------------------------------------
# Error cases
# ----------------------------------------------------------------------------

subtest 'decode_qoi: rejects bad magic' => sub {
    my $img   = CodingAdventures::PixelContainer->new(1, 1);
    my $bytes = encode_qoi($img);
    substr($bytes, 0, 4) = 'XXXX';
    like(dies { decode_qoi($bytes) }, qr/QOI:/, 'bad magic dies');
};

subtest 'decode_qoi: rejects too-short input' => sub {
    like(dies { decode_qoi('qoif') }, qr/QOI:/, 'short input dies');
};

subtest 'decode_qoi: rejects undef input' => sub {
    like(dies { decode_qoi(undef) }, qr/QOI:/, 'undef input dies');
};

subtest 'encode_qoi: rejects non-container input' => sub {
    like(dies { encode_qoi('not a container') }, qr/EncodeError:/, 'string input dies');
    like(dies { encode_qoi(undef) },             qr/EncodeError:/, 'undef input dies');
};

done_testing;
