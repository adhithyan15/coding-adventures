package CodingAdventures::ImageCodecPPM;

# ============================================================================
# CodingAdventures::ImageCodecPPM — PPM (Portable Pixmap) Encode / Decode
# ============================================================================
#
# # What Is PPM?
#
# PPM (Portable Pixmap) is the simplest colour image format in the Netpbm
# family.  It was designed to be trivially easy to read and write.  A PPM file
# has exactly two parts:
#
#   1. ASCII text header
#   2. Binary pixel data (or ASCII numbers in the P3 variant — not supported here)
#
# We implement the P6 (binary) variant only.
#
# # P6 PPM Format
#
#   P6\n
#   [# comment lines, each starting with '#', followed by newline]\n
#   <width> <height>\n
#   <maxval>\n
#   <binary RGB data>
#
# Example for a 2×1 image:
#
#   P6\n10 5\n255\n\xFF\x00\x00\x00\xFF\x00
#                           ^^^^^^^^^^^ ^^^^^^^^^^^
#                           pixel(0,0)  pixel(1,0)
#                           R=255,G=0,  R=0,G=255,
#                           B=0 (red)   B=0 (green)
#
# # Key Differences From BMP
#
#   * PPM stores RGB (no alpha channel).  When decoding we set alpha = 255.
#   * Pixels are stored top-down, left-to-right (no flipping needed).
#   * Rows have no padding — each pixel is exactly 3 bytes.
#   * The only compression is "none" — the file is the raw bytes.
#   * Comment lines (starting with '#') may appear before any header token.
#
# # Alpha Channel
#
# PPM has no alpha channel.  On encode we silently drop the alpha byte.
# On decode we set alpha = 255 (fully opaque) for every pixel.
#
# ============================================================================

use strict;
use warnings;
use 5.026;
use Exporter 'import';

use CodingAdventures::PixelContainer;

our $VERSION   = '0.01';
our @EXPORT_OK = qw(encode_ppm decode_ppm mime_type);

# ============================================================================
# mime_type() -> 'image/x-portable-pixmap'
# ============================================================================
sub mime_type { 'image/x-portable-pixmap' }

# ============================================================================
# encode_ppm($container) -> $bytes
# ============================================================================
#
# Encode a PixelContainer as a P6 PPM byte string.
#
# Output format:
#   "P6\n<W> <H>\n255\n" followed by W*H*3 bytes of RGB data.
#
# Alpha is discarded — PPM does not support transparency.
#
# @param $container  CodingAdventures::PixelContainer object
# @return            Binary string (the .ppm file bytes)
# @die               "EncodeError: ..." on bad input
# ============================================================================
sub encode_ppm {
    my ($container) = @_;

    die "EncodeError: expected a PixelContainer object"
        unless defined $container
            && ref($container)
            && $container->isa('CodingAdventures::PixelContainer');

    my $w = $container->width;
    my $h = $container->height;

    # -----------------------------------------------------------------------
    # Header: magic, dimensions, maxval
    # -----------------------------------------------------------------------
    # P6 means "binary colour image".
    # maxval = 255 means each channel uses one byte (values 0..255).
    my $header = "P6\n$w $h\n255\n";

    # -----------------------------------------------------------------------
    # Pixel data: iterate rows top-to-bottom, left-to-right
    # -----------------------------------------------------------------------
    # We accumulate all pixels into one string — Perl string concatenation
    # for small images is fine; for very large images a join() or vec() approach
    # could reduce allocations, but clarity wins here.
    my $pixels = '';
    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            my ($r, $g, $b, undef) = $container->pixel_at($x, $y);
            # pack('CCC', ...) packs three unsigned chars = 3 bytes RGB
            $pixels .= pack('CCC', $r, $g, $b);
        }
    }

    return $header . $pixels;
}

# ============================================================================
# decode_ppm($bytes) -> $container
# ============================================================================
#
# Decode a P6 PPM byte string into a PixelContainer.
#
# Parsing strategy:
#   1. Verify magic token "P6".
#   2. Skip any comment lines (lines beginning with '#').
#   3. Read width, height, and maxval as whitespace-separated ASCII tokens.
#      (The spec allows arbitrary whitespace between tokens.)
#   4. After the final newline following maxval, read W*H*3 raw bytes.
#
# maxval handling:
#   * If maxval == 255: read one byte per channel.
#   * If maxval <= 255: scale: channel = int(raw * 255 / maxval).
#   * maxval > 255 (two bytes per channel) is NOT supported and will die.
#
# Alpha is set to 255 for all pixels.
#
# @param $bytes  Binary string — raw .ppm file content
# @return        CodingAdventures::PixelContainer
# @die           "PPM: ..." on format errors
# ============================================================================
sub decode_ppm {
    my ($bytes) = @_;

    die "PPM: input must be a defined string"
        unless defined $bytes;
    die "PPM: file is empty" unless length($bytes) > 0;

    # -----------------------------------------------------------------------
    # We use an offset cursor into the binary string.
    # -----------------------------------------------------------------------
    # `_read_token` skips whitespace and comment lines, then reads up to the
    # next whitespace.  This is the canonical way to parse Netpbm headers
    # without assuming any specific line structure.
    my $pos = 0;

    # Local helper: skip whitespace (spaces, tabs, newlines, CR)
    # and PPM comment lines (lines starting with '#').
    my $skip_ws_comments = sub {
        while ($pos < length($bytes)) {
            my $ch = substr($bytes, $pos, 1);
            if ($ch eq '#') {
                # Skip the entire comment line, including the terminating '\n'
                while ($pos < length($bytes) && substr($bytes, $pos, 1) ne "\n") {
                    $pos++;
                }
                $pos++;   # skip the '\n' itself
            } elsif ($ch =~ /[ \t\r\n]/) {
                $pos++;
            } else {
                last;   # found a non-whitespace, non-comment character
            }
        }
    };

    # Local helper: read a whitespace-delimited ASCII token.
    my $read_token = sub {
        $skip_ws_comments->();
        my $start = $pos;
        while ($pos < length($bytes) && substr($bytes, $pos, 1) !~ /[ \t\r\n]/) {
            $pos++;
        }
        die "PPM: unexpected end of header" if $pos == $start;
        return substr($bytes, $start, $pos - $start);
    };

    # -----------------------------------------------------------------------
    # Parse header tokens
    # -----------------------------------------------------------------------
    my $magic = $read_token->();
    die "PPM: unsupported format '$magic' (only P6 supported)" unless $magic eq 'P6';

    my $w      = $read_token->();
    my $h      = $read_token->();
    my $maxval = $read_token->();

    die "PPM: width '$w' is not a positive integer"
        unless $w =~ /^\d+$/ && $w > 0;
    die "PPM: height '$h' is not a positive integer"
        unless $h =~ /^\d+$/ && $h > 0;
    die "PPM: maxval '$maxval' out of range (1..65535)"
        unless $maxval =~ /^\d+$/ && $maxval >= 1 && $maxval <= 65535;
    die "PPM: maxval > 255 (two-byte channels) not supported"
        if $maxval > 255;

    # After maxval there is exactly one whitespace character (typically '\n')
    # before the binary pixel data begins.
    $pos++;   # skip the single whitespace separator after maxval

    # -----------------------------------------------------------------------
    # Pixel data starts at $pos
    # -----------------------------------------------------------------------
    my $needed = $w * $h * 3;
    die "PPM: pixel data too short: need $needed bytes, got " . (length($bytes) - $pos)
        if length($bytes) - $pos < $needed;

    my $container = CodingAdventures::PixelContainer->new($w, $h);

    for my $y (0 .. $h - 1) {
        for my $x (0 .. $w - 1) {
            # Read 3 bytes: R, G, B
            my ($r, $g, $b) = unpack('CCC', substr($bytes, $pos, 3));
            $pos += 3;

            # Scale if maxval != 255
            # (integer rounding: round half-up via int(x + 0.5))
            if ($maxval != 255) {
                $r = int($r * 255 / $maxval + 0.5);
                $g = int($g * 255 / $maxval + 0.5);
                $b = int($b * 255 / $maxval + 0.5);
            }

            # Alpha is always 255 — PPM has no transparency.
            $container->set_pixel($x, $y, $r, $g, $b, 255);
        }
    }

    return $container;
}

1;

__END__

=head1 NAME

CodingAdventures::ImageCodecPPM - PPM (Portable Pixmap) image encode/decode (IC02)

=head1 SYNOPSIS

    use lib '../pixel-container/lib';
    use CodingAdventures::PixelContainer;
    use CodingAdventures::ImageCodecPPM qw(encode_ppm decode_ppm);

    # Encode
    my $img   = CodingAdventures::PixelContainer->new(8, 8);
    $img->fill_pixels(0, 128, 255, 255);
    my $bytes = encode_ppm($img);

    # Decode
    my $img2          = decode_ppm($bytes);
    my ($r, $g, $b, $a) = $img2->pixel_at(0, 0);
    # ($r, $g, $b, $a) == (0, 128, 255, 255)

=head1 DESCRIPTION

Implements PPM P6 (binary portable pixmap) encode and decode for
L<CodingAdventures::PixelContainer> objects.  Part of the IC02 layer.

PPM has no alpha channel: encode silently drops alpha; decode sets alpha=255.

=head1 FUNCTIONS

=over 4

=item C<mime_type()>

Returns C<'image/x-portable-pixmap'>.

=item C<encode_ppm($container)>

Encode a C<PixelContainer> as a P6 PPM binary string.
Dies with C<"EncodeError: ..."> if the input is not a PixelContainer.

=item C<decode_ppm($bytes)>

Decode a PPM binary string into a C<PixelContainer>.
Skips comment lines.  Scales channel values if maxval != 255.
Dies with C<"PPM: ..."> on format errors.

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
