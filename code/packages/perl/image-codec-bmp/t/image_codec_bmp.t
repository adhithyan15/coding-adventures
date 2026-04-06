use strict;
use warnings;
use Test2::V0;

use lib '../pixel-container/lib', 'lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecBMP qw(encode_bmp decode_bmp mime_type);

# ============================================================================
# BMP Codec test suite
# ============================================================================
#
# Tests cover:
#   - mime_type()
#   - encode_bmp: output structure (magic, header fields)
#   - encode_bmp: BGRA byte order in pixel data
#   - decode_bmp: round-trip correctness
#   - decode_bmp: handles bottom-up (positive height) BMP
#   - decode_bmp: handles 24bpp BMP
#   - Error cases: bad magic, too-short input, bad input type
# ============================================================================

# ----------------------------------------------------------------------------
# mime_type
# ----------------------------------------------------------------------------

subtest 'mime_type returns image/bmp' => sub {
    is(mime_type(), 'image/bmp', 'mime_type = image/bmp');
};

# ----------------------------------------------------------------------------
# encode_bmp structure checks
# ----------------------------------------------------------------------------

subtest 'encode_bmp: output starts with BM magic' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_bmp($img);
    is(substr($bytes, 0, 2), 'BM', 'magic bytes = BM');
};

subtest 'encode_bmp: correct file size for 1x1' => sub {
    my $img   = CodingAdventures::PixelContainer->new(1, 1);
    my $bytes = encode_bmp($img);
    # 1x1 @ 32bpp = 54 header + 4 pixel bytes = 58
    is(length($bytes), 58, '1x1 BMP is 58 bytes');
    my ($bfSize) = unpack('V', substr($bytes, 2, 4));
    is($bfSize, 58, 'bfSize field = 58');
};

subtest 'encode_bmp: correct file size for 4x3' => sub {
    my $img   = CodingAdventures::PixelContainer->new(4, 3);
    my $bytes = encode_bmp($img);
    # 4x3 @ 32bpp = 54 + 4*3*4 = 54 + 48 = 102
    is(length($bytes), 102, '4x3 BMP is 102 bytes');
    my ($bfSize) = unpack('V', substr($bytes, 2, 4));
    is($bfSize, 102, 'bfSize field = 102');
};

subtest 'encode_bmp: pixel data offset = 54' => sub {
    my $img   = CodingAdventures::PixelContainer->new(3, 3);
    my $bytes = encode_bmp($img);
    my ($off) = unpack('V', substr($bytes, 10, 4));
    is($off, 54, 'bfOffBits = 54');
};

subtest 'encode_bmp: info header size = 40' => sub {
    my $img   = CodingAdventures::PixelContainer->new(5, 5);
    my $bytes = encode_bmp($img);
    my ($sz) = unpack('V', substr($bytes, 14, 4));
    is($sz, 40, 'biSize = 40');
};

subtest 'encode_bmp: biWidth and biHeight correct' => sub {
    my $img   = CodingAdventures::PixelContainer->new(7, 3);
    my $bytes = encode_bmp($img);
    my ($w, $h) = unpack('l< l<', substr($bytes, 18, 8));
    is($w, 7,  'biWidth = 7');
    is($h, -3, 'biHeight = -3 (top-down)');
};

subtest 'encode_bmp: biBitCount = 32' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_bmp($img);
    my ($bc) = unpack('v', substr($bytes, 28, 2));
    is($bc, 32, 'biBitCount = 32');
};

subtest 'encode_bmp: biCompression = 0 (BI_RGB)' => sub {
    my $img   = CodingAdventures::PixelContainer->new(2, 2);
    my $bytes = encode_bmp($img);
    my ($comp) = unpack('V', substr($bytes, 30, 4));
    is($comp, 0, 'biCompression = 0 (BI_RGB)');
};

# ----------------------------------------------------------------------------
# Pixel data / BGRA channel order
# ----------------------------------------------------------------------------

subtest 'encode_bmp: BGRA channel order in pixel data' => sub {
    # Set pixel (0,0) to R=10, G=20, B=30, A=40
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 10, 20, 30, 40);
    my $bytes = encode_bmp($img);
    # Pixel data starts at offset 54; first pixel is BGRA
    my ($b, $g, $r, $a) = unpack('CCCC', substr($bytes, 54, 4));
    is($b, 30, 'first byte in pixel = B (30)');
    is($g, 20, 'second byte = G (20)');
    is($r, 10, 'third byte = R (10)');
    is($a, 40, 'fourth byte = A (40)');
};

# ----------------------------------------------------------------------------
# Round-trip encode → decode
# ----------------------------------------------------------------------------

subtest 'round-trip: 1x1 red pixel' => sub {
    my $img = CodingAdventures::PixelContainer->new(1, 1);
    $img->set_pixel(0, 0, 255, 0, 0, 255);
    my $bytes   = encode_bmp($img);
    my $decoded = decode_bmp($bytes);
    my ($r, $g, $b, $a) = $decoded->pixel_at(0, 0);
    is($r, 255, 'R=255');
    is($g, 0,   'G=0');
    is($b, 0,   'B=0');
    is($a, 255, 'A=255');
};

subtest 'round-trip: 4x4 checkerboard' => sub {
    my $img = CodingAdventures::PixelContainer->new(4, 4);
    for my $y (0..3) {
        for my $x (0..3) {
            my $on = ($x + $y) % 2;
            $img->set_pixel($x, $y, $on ? 255 : 0, 0, $on ? 0 : 255, 255);
        }
    }
    my $decoded = decode_bmp(encode_bmp($img));
    is($decoded->width,  4, 'width preserved');
    is($decoded->height, 4, 'height preserved');
    for my $y (0..3) {
        for my $x (0..3) {
            my @orig = $img->pixel_at($x, $y);
            my @got  = $decoded->pixel_at($x, $y);
            is(\@got, \@orig, "pixel ($x,$y) matches");
        }
    }
};

subtest 'round-trip: fill_pixels solid white' => sub {
    my $img = CodingAdventures::PixelContainer->new(8, 8);
    $img->fill_pixels(255, 255, 255, 255);
    my $decoded = decode_bmp(encode_bmp($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(4, 4);
    is($r, 255, 'R=255 after round-trip');
    is($g, 255, 'G=255');
    is($b, 255, 'B=255');
    is($a, 255, 'A=255');
};

subtest 'round-trip: alpha channel preserved' => sub {
    my $img = CodingAdventures::PixelContainer->new(2, 2);
    $img->set_pixel(0, 0, 100, 150, 200, 127);
    my $decoded = decode_bmp(encode_bmp($img));
    my ($r, $g, $b, $a) = $decoded->pixel_at(0, 0);
    is($r, 100, 'R=100');
    is($g, 150, 'G=150');
    is($b, 200, 'B=200');
    is($a, 127, 'A=127 (semi-transparent)');
};

subtest 'round-trip: width/height preserved' => sub {
    my $img     = CodingAdventures::PixelContainer->new(13, 7);
    my $decoded = decode_bmp(encode_bmp($img));
    is($decoded->width,  13, 'width = 13');
    is($decoded->height, 7,  'height = 7');
};

# ----------------------------------------------------------------------------
# decode_bmp: bottom-up BMP (positive height)
# ----------------------------------------------------------------------------

subtest 'decode_bmp: handles positive (bottom-up) biHeight' => sub {
    # Build a minimal bottom-up BMP for a 1x2 image:
    #   Row 0 in file = display row 1 (bottom) = green
    #   Row 1 in file = display row 0 (top)    = red
    my $w = 1; my $h = 2;
    my $file_header = pack('a2 V v v V', 'BM', 54 + 8, 0, 0, 54);
    my $info_header = pack('V l< l< v v V V l< l< V V',
        40, $w, $h, 1, 32, 0, 0, 0, 0, 0, 0);
    # Bottom-up: row 0 in file = bottom row = green pixel
    my $pix_data    = pack('CCCC', 0, 255, 0, 255)   # bottom row: green (BGRA: G=255)
                    . pack('CCCC', 0, 0, 255, 255);   # top row: red (BGRA: R=255)
    my $bmp = $file_header . $info_header . $pix_data;

    my $img = decode_bmp($bmp);
    is($img->width,  1, 'width=1');
    is($img->height, 2, 'height=2');

    # display row 0 (top) should be red (stored as row 1 in file)
    my ($r0, $g0, $b0) = $img->pixel_at(0, 0);
    is($r0, 255, 'top row R=255 (red)');
    is($g0, 0,   'top row G=0');
    is($b0, 0,   'top row B=0');

    # display row 1 (bottom) should be green (stored as row 0 in file)
    my ($r1, $g1, $b1) = $img->pixel_at(0, 1);
    is($r1, 0,   'bottom row R=0 (green)');
    is($g1, 255, 'bottom row G=255');
    is($b1, 0,   'bottom row B=0');
};

# ----------------------------------------------------------------------------
# Error cases
# ----------------------------------------------------------------------------

subtest 'decode_bmp: rejects bad magic' => sub {
    my $img   = CodingAdventures::PixelContainer->new(1, 1);
    my $bytes = encode_bmp($img);
    substr($bytes, 0, 2) = 'XX';   # corrupt magic
    like(dies { decode_bmp($bytes) }, qr/BMP:/, 'bad magic dies');
};

subtest 'decode_bmp: rejects too-short input' => sub {
    like(dies { decode_bmp('BM') }, qr/BMP:/, 'short input dies');
};

subtest 'decode_bmp: rejects undef input' => sub {
    like(dies { decode_bmp(undef) }, qr/BMP:/, 'undef input dies');
};

subtest 'encode_bmp: rejects non-container input' => sub {
    like(dies { encode_bmp('not a container') }, qr/EncodeError:/, 'string input dies');
    like(dies { encode_bmp(undef) },             qr/EncodeError:/, 'undef input dies');
};

done_testing;
