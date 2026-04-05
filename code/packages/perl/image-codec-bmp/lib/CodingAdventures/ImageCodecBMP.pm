package CodingAdventures::ImageCodecBMP;

# ============================================================================
# CodingAdventures::ImageCodecBMP — BMP (Windows Bitmap) Encode / Decode
# ============================================================================
#
# # What Is BMP?
#
# BMP (Bitmap) is one of the oldest and simplest image file formats, defined
# by Microsoft for Windows.  A BMP file has:
#
#   1. BITMAPFILEHEADER (14 bytes) — magic bytes, file size, pixel data offset
#   2. BITMAPINFOHEADER (40 bytes) — image dimensions, bit depth, compression
#   3. Pixel data — rows of pixels, bottom-up, padded to 4-byte row alignment
#
# This implementation uses 32 bits-per-pixel (BGRA order) and
# BI_RGB "compression" (value 0 = no compression), which actually means the
# alpha channel is stored but was historically ignored by Windows renderers.
# Modern viewers (GIMP, browser canvas, etc.) do respect the alpha channel.
#
# # BMP File Layout (total = 54 + W*H*4 bytes for 32bpp no-pad)
#
#   Offset  Size  Field                 Value
#   ------  ----  -----                 -----
#   0       2     bfType                0x4D42 ("BM")
#   2       4     bfSize                total file size
#   6       2     bfReserved1           0
#   8       2     bfReserved2           0
#   10      4     bfOffBits             54 (offset to pixel data)
#   14      4     biSize                40 (BITMAPINFOHEADER size)
#   18      4     biWidth               image width in pixels (signed LE int32)
#   22      4     biHeight              image height (negative = top-down)
#   26      2     biPlanes              1
#   28      2     biBitCount            32
#   30      4     biCompression         0 (BI_RGB)
#   34      4     biSizeImage           0 (can be 0 for BI_RGB)
#   38      4     biXPelsPerMeter       0
#   42      4     biYPelsPerMeter       0
#   46      4     biClrUsed             0
#   50      4     biClrImportant        0
#   54      ...   pixel data (BGRA rows)
#
# # Row Order
#
# Standard BMP stores rows bottom-up: row 0 in the file is the bottom row.
# When biHeight is negative the rows are stored top-down (the Windows way of
# writing "top-down DIBs").  This implementation writes top-down BMP (negative
# height) and reads both positive (bottom-up) and negative (top-down).
#
# # BGRA vs RGBA
#
# BMP stores channels in BGRA order (Blue first), not RGBA.  Our PixelContainer
# uses RGBA.  When encoding we swap R↔B; when decoding we swap back.
#
# # Pixel Row Padding
#
# BMP requires each pixel row to be a multiple of 4 bytes.  With 32 bpp and
# any width, each row is W*4 bytes — always a multiple of 4 — so no padding
# byte logic is needed here.
#
# ============================================================================

use strict;
use warnings;
use 5.026;
use Exporter 'import';

use CodingAdventures::PixelContainer;

our $VERSION  = '0.01';
our @EXPORT_OK = qw(encode_bmp decode_bmp mime_type);

# ============================================================================
# mime_type() -> 'image/bmp'
# ============================================================================
sub mime_type { 'image/bmp' }

# ============================================================================
# encode_bmp($container) -> $bytes
# ============================================================================
#
# Encode a PixelContainer as a 32bpp top-down BMP byte string.
#
# Steps:
#   1. Extract width W, height H from the container.
#   2. Compute file size = 54 + W*H*4.
#   3. Build BITMAPFILEHEADER (14 bytes) with pack.
#   4. Build BITMAPINFOHEADER (40 bytes) with pack.
#      - Use -H for biHeight to indicate top-down storage.
#   5. Iterate rows top-to-bottom; for each pixel swap R and B, pack as BGRA.
#
# pack format cheat-sheet:
#   'v'  = little-endian uint16  (2 bytes)
#   'V'  = little-endian uint32  (4 bytes)
#   'l<' = little-endian int32   (4 bytes, signed)
#   'C'  = unsigned char         (1 byte)
#
# @param $container  CodingAdventures::PixelContainer object
# @return            Binary string (the .bmp file bytes)
# @die               "EncodeError: ..." on bad input
# ============================================================================
sub encode_bmp {
    my ($container) = @_;

    die "EncodeError: expected a PixelContainer object"
        unless defined $container
            && ref($container)
            && $container->isa('CodingAdventures::PixelContainer');

    my $w = $container->width;
    my $h = $container->height;

    # Each pixel is 4 bytes (BGRA); no row padding needed for 32bpp.
    my $pixel_data_size = $w * $h * 4;
    my $file_size       = 54 + $pixel_data_size;   # header(14) + info(40) + pixels

    # -----------------------------------------------------------------------
    # BITMAPFILEHEADER  (14 bytes total)
    # -----------------------------------------------------------------------
    # bfType     = 'BM' = 0x424D stored as two chars
    # bfSize     = total file size (uint32 LE)
    # bfReserved1, bfReserved2 = 0 (uint16 LE each)
    # bfOffBits  = 54 — offset from start of file to pixel data (uint32 LE)
    my $file_header = pack('a2 V v v V',
        'BM',        # magic
        $file_size,  # total bytes in file
        0,           # reserved1
        0,           # reserved2
        54,          # pixel data starts at byte 54
    );

    # -----------------------------------------------------------------------
    # BITMAPINFOHEADER  (40 bytes total)
    # -----------------------------------------------------------------------
    # biSize          = 40 (this structure's size)       uint32
    # biWidth         = W (pixels)                       int32 signed LE
    # biHeight        = -H (negative = top-down)         int32 signed LE
    # biPlanes        = 1                                uint16
    # biBitCount      = 32 (32 bits per pixel)           uint16
    # biCompression   = 0 (BI_RGB = no compression)      uint32
    # biSizeImage     = 0 (valid for BI_RGB)             uint32
    # biXPelsPerMeter = 0                                int32
    # biYPelsPerMeter = 0                                int32
    # biClrUsed       = 0                                uint32
    # biClrImportant  = 0                                uint32
    my $info_header = pack('V l< l< v v V V l< l< V V',
        40,   # biSize
        $w,   # biWidth  (signed int32, positive = left-to-right)
        -$h,  # biHeight (signed int32, negative = top-down)
        1,    # biPlanes
        32,   # biBitCount
        0,    # biCompression (BI_RGB)
        0,    # biSizeImage (0 is valid for BI_RGB)
        0,    # biXPelsPerMeter
        0,    # biYPelsPerMeter
        0,    # biClrUsed
        0,    # biClrImportant
    );

    # -----------------------------------------------------------------------
    # Pixel data
    # -----------------------------------------------------------------------
    # BMP uses BGRA byte order (opposite of our RGBA storage).
    # We iterate rows top-to-bottom (matching top-down DIB, biHeight < 0).
    my $pixels = '';
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            my ($r, $g, $b, $a) = $container->pixel_at($x, $y);
            # Swap R and B → write as B, G, R, A
            $pixels .= pack('CCCC', $b, $g, $r, $a);
        }
    }

    return $file_header . $info_header . $pixels;
}

# ============================================================================
# decode_bmp($bytes) -> $container
# ============================================================================
#
# Decode a BMP byte string into a PixelContainer.
#
# The decoder:
#   1. Checks the 'BM' magic bytes.
#   2. Reads biWidth and biHeight from the info header.
#      - If biHeight > 0: rows are bottom-up (flip when reading).
#      - If biHeight < 0: rows are top-down (read in order).
#   3. Reads biBitCount and bfOffBits to locate pixel data.
#   4. Supports 24bpp and 32bpp only.
#   5. For each pixel, swaps B↔R back to RGBA.
#   6. Handles row padding: each row is padded to a 4-byte boundary.
#
# Row stride with padding:
#   stride = (W * bytes_per_pixel + 3) & ~3
#   The & ~3 rounds up to the next multiple of 4.
#
# @param $bytes  Binary string — the raw .bmp file content
# @return        CodingAdventures::PixelContainer
# @die           "BMP: ..." on format errors
# ============================================================================
sub decode_bmp {
    my ($bytes) = @_;

    die "BMP: input must be a defined string"
        unless defined $bytes;
    die "BMP: file too short to be a valid BMP (need at least 54 bytes)"
        if length($bytes) < 54;

    # --- BITMAPFILEHEADER ---------------------------------------------------
    # bytes 0-1: magic 'BM'
    my $magic = substr($bytes, 0, 2);
    die "BMP: invalid magic '$magic' (expected 'BM')" unless $magic eq 'BM';

    # bytes 10-13: bfOffBits — pixel data start offset (uint32 LE)
    my ($pix_offset) = unpack('V', substr($bytes, 10, 4));

    # --- BITMAPINFOHEADER ---------------------------------------------------
    # bytes 14-17: biSize (uint32 LE)
    my ($info_size) = unpack('V', substr($bytes, 14, 4));
    die "BMP: unsupported info header size $info_size (need >= 40)"
        if $info_size < 40;

    # bytes 18-21: biWidth  (signed int32 LE)
    # bytes 22-25: biHeight (signed int32 LE)
    my ($w, $h_signed) = unpack('l< l<', substr($bytes, 18, 8));

    die "BMP: width $w is not positive" unless $w > 0;
    die "BMP: height 0 is invalid" if $h_signed == 0;

    # Negative biHeight → top-down (rows stored in display order).
    # Positive biHeight → bottom-up (rows stored in reverse order).
    my $top_down = ($h_signed < 0) ? 1 : 0;
    my $h        = abs($h_signed);

    # bytes 28-29: biBitCount (uint16 LE)
    my ($bit_count) = unpack('v', substr($bytes, 28, 2));
    die "BMP: unsupported bit depth $bit_count (only 24 and 32 supported)"
        unless $bit_count == 24 || $bit_count == 32;

    # bytes 30-33: biCompression (uint32 LE) — must be 0 (BI_RGB)
    my ($compression) = unpack('V', substr($bytes, 30, 4));
    die "BMP: unsupported compression $compression (only BI_RGB=0 supported)"
        unless $compression == 0;

    # bytes_per_pixel = biBitCount / 8
    my $bpp = int($bit_count / 8);   # 3 for 24bpp, 4 for 32bpp

    # Row stride: rounded up to next multiple of 4 bytes.
    # Formula: (W * bpp + 3) & ~3
    # Example: W=5, bpp=3 → 15 + 3 = 18, 18 & ~3 = 16 (nearest multiple-of-4 ≥ 15)
    my $stride = ($w * $bpp + 3) & ~3;

    my $required = $pix_offset + $stride * $h;
    die "BMP: file too short: need $required bytes, got " . length($bytes)
        if length($bytes) < $required;

    # --- Build PixelContainer -----------------------------------------------
    my $container = CodingAdventures::PixelContainer->new($w, $h);

    for my $row (0 .. $h - 1) {
        # Map file row to display row:
        #   top_down  → file row 0 = display row 0
        #   bottom_up → file row 0 = display row (H-1), i.e. flip
        my $display_y = $top_down ? $row : ($h - 1 - $row);

        my $row_start = $pix_offset + $row * $stride;

        for my $x (0 .. $w - 1) {
            my $pixel_start = $row_start + $x * $bpp;
            my ($b, $g, $r, $a);

            if ($bpp == 4) {
                # 32bpp: B, G, R, A
                ($b, $g, $r, $a) = unpack('CCCC', substr($bytes, $pixel_start, 4));
            } else {
                # 24bpp: B, G, R — no alpha stored; assume fully opaque
                ($b, $g, $r) = unpack('CCC', substr($bytes, $pixel_start, 3));
                $a = 255;
            }

            # Swap B↔R back to RGBA storage order
            $container->set_pixel($x, $display_y, $r, $g, $b, $a);
        }
    }

    return $container;
}

1;

__END__

=head1 NAME

CodingAdventures::ImageCodecBMP - BMP (Windows Bitmap) image encode/decode (IC01)

=head1 SYNOPSIS

    use lib '../pixel-container/lib';
    use CodingAdventures::PixelContainer;
    use CodingAdventures::ImageCodecBMP qw(encode_bmp decode_bmp);

    # Encode
    my $img   = CodingAdventures::PixelContainer->new(64, 64);
    $img->fill_pixels(255, 0, 0, 255);   # solid red
    my $bytes = encode_bmp($img);

    # Write to file
    open my $fh, '>:raw', 'output.bmp' or die $!;
    print $fh $bytes;
    close $fh;

    # Decode
    open my $in, '<:raw', 'output.bmp' or die $!;
    local $/; my $data = <$in>; close $in;
    my $img2 = decode_bmp($data);
    my ($r, $g, $b, $a) = $img2->pixel_at(0, 0);
    # ($r, $g, $b, $a) == (255, 0, 0, 255)

=head1 DESCRIPTION

Implements BMP (Windows Bitmap) encode and decode for
L<CodingAdventures::PixelContainer> objects.  Part of the IC01 layer in the
coding-adventures image pipeline.

The encoder writes 32bpp top-down BMP (BITMAPINFOHEADER with negative height).
The decoder handles both top-down (negative height) and bottom-up (positive
height) BMP files, and both 24bpp and 32bpp variants.

=head1 FUNCTIONS

=over 4

=item C<mime_type()>

Returns C<'image/bmp'>.

=item C<encode_bmp($container)>

Encode a C<PixelContainer> as a binary BMP string.
Dies with C<"EncodeError: ..."> if the input is not a PixelContainer.

=item C<decode_bmp($bytes)>

Decode a BMP binary string into a C<PixelContainer>.
Dies with C<"BMP: ..."> on format errors.

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
