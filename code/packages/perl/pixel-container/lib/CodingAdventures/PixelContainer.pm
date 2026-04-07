package CodingAdventures::PixelContainer;

# ============================================================================
# CodingAdventures::PixelContainer — In-Memory RGBA8 Pixel Buffer
# ============================================================================
#
# # What Is a Pixel Container?
#
# A pixel container is the simplest possible in-memory image: a flat array of
# colour values, one element per pixel.  We store each pixel as four bytes in
# RGBA order:
#
#   byte 0: Red   (0..255)
#   byte 1: Green (0..255)
#   byte 2: Blue  (0..255)
#   byte 3: Alpha (0..255, where 0 = fully transparent, 255 = fully opaque)
#
# "Flat" means all rows are stored back-to-back.  For a W×H image, pixel (x, y)
# lives at byte offset:
#
#   offset = (y * W + x) * 4
#
# For example, a 3×2 image lays out as:
#
#   (0,0)(1,0)(2,0)   ← row 0
#   (0,1)(1,1)(2,1)   ← row 1
#
#   byte indices: 0..3, 4..7, 8..11, 12..15, 16..19, 20..23
#
# # Why Perl Strings as Byte Buffers?
#
# Perl strings are not only text — they are arbitrary byte sequences.  When we
# do `"\x00" x N` we allocate N zero bytes in a single scalar.  We can then
# use `substr` (for slicing/overwriting) and `pack`/`unpack` (for serialising
# integers to/from bytes) to manipulate the buffer efficiently without any C
# extension.
#
# This module is IC00 in the coding-adventures barcode pipeline:
#
#   IC00  PixelContainer  — raw RGBA8 pixel storage        (THIS MODULE)
#   IC01  ImageCodecBMP   — BMP encode/decode
#   IC02  ImageCodecPPM   — PPM encode/decode
#   IC03  ImageCodecQOI   — QOI encode/decode
#
# ============================================================================

use strict;
use warnings;
use 5.026;

our $VERSION = '0.01';

# ============================================================================
# new($class, $width, $height)
# ============================================================================
#
# Construct an empty (all-black, fully-transparent) W×H pixel container.
#
# The pixel buffer is initialised to all-zero bytes, so every pixel starts as
# (R=0, G=0, B=0, A=0) — black and fully transparent.
#
# @param $width   Positive integer — number of columns
# @param $height  Positive integer — number of rows
# @return         Blessed hashref with keys: width, height, data
# @die            "InvalidInput: ..." if width or height <= 0 or non-integer
# ============================================================================
sub new {
    my ($class, $width, $height) = @_;

    # Validate dimensions: both must be positive integers.
    die "InvalidInput: width must be a positive integer, got " . (defined $width ? $width : 'undef')
        unless defined $width && $width =~ /^\d+$/ && $width > 0;
    die "InvalidInput: height must be a positive integer, got " . (defined $height ? $height : 'undef')
        unless defined $height && $height =~ /^\d+$/ && $height > 0;

    # Allocate width*height*4 zero bytes.
    # "\x00" x N creates a string of N NUL bytes (Perl's way of a byte array).
    my $data = "\x00" x ($width * $height * 4);

    return bless {
        width  => $width,
        height => $height,
        data   => $data,
    }, $class;
}

# ============================================================================
# width($self)  — accessor
# ============================================================================
# Returns the image width in pixels.
sub width  { $_[0]->{width}  }

# ============================================================================
# height($self)  — accessor
# ============================================================================
# Returns the image height in pixels.
sub height { $_[0]->{height} }

# ============================================================================
# data($self)  — accessor
# ============================================================================
#
# Returns a *reference* to the internal byte-string buffer (scalar ref).
# Callers that need direct binary access (e.g. codec encode) can dereference
# this to get the raw bytes.
#
# Returning a reference (rather than copying the string) avoids duplicating
# potentially large buffers on every call.
#
# @return  \scalar — reference to the internal byte buffer
# ============================================================================
sub data { \$_[0]->{data} }

# ============================================================================
# pixel_at($self, $x, $y) -> ($r, $g, $b, $a)
# ============================================================================
#
# Read the RGBA values of pixel (x, y).  Returns (0, 0, 0, 0) for any
# out-of-bounds coordinate — a safe sentinel rather than a die, because callers
# doing convolution or padding often probe outside the image intentionally.
#
# Byte layout reminder:
#   offset = (y * width + x) * 4
#   data[offset+0] = R, data[offset+1] = G, data[offset+2] = B, data[offset+3] = A
#
# We use `unpack('CCCC', ...)` to decode four unsigned bytes at once.
# 'C' in pack/unpack means unsigned char (uint8), range 0..255.
#
# @param $x  Column index (0-based)
# @param $y  Row index    (0-based)
# @return    4-element list ($r, $g, $b, $a)
# ============================================================================
sub pixel_at {
    my ($c, $x, $y) = @_;

    # Out-of-bounds guard — return transparent black rather than crashing.
    return (0, 0, 0, 0)
        if $x < 0 || $y < 0 || $x >= $c->{width} || $y >= $c->{height};

    # Compute the byte offset of the first channel (R) for this pixel.
    my $offset = ($y * $c->{width} + $x) * 4;

    # Extract 4 consecutive bytes as unsigned chars.
    return unpack('CCCC', substr($c->{data}, $offset, 4));
}

# ============================================================================
# set_pixel($self, $x, $y, $r, $g, $b, $a)
# ============================================================================
#
# Write a single pixel at (x, y).  No-op for out-of-bounds coordinates.
#
# We use `pack('CCCC', ...)` to encode four values into 4 bytes, then
# `substr(LVALUE, ...)` to splice them into position in the buffer.
#
# `substr` as an lvalue is a Perl idiom:
#   substr($string, $offset, $length) = $replacement
# replaces exactly $length bytes at $offset with the new string.
#
# @param $x,$y          Coordinates (0-based)
# @param $r,$g,$b,$a    Channel values 0..255
# ============================================================================
sub set_pixel {
    my ($c, $x, $y, $r, $g, $b, $a) = @_;

    # Silent no-op for out-of-bounds writes — matches canvas convention.
    return if $x < 0 || $y < 0 || $x >= $c->{width} || $y >= $c->{height};

    my $offset = ($y * $c->{width} + $x) * 4;

    # pack('CCCC', ...) packs four unsigned chars into a 4-byte string.
    substr($c->{data}, $offset, 4) = pack('CCCC', $r, $g, $b, $a);
}

# ============================================================================
# fill_pixels($self, $r, $g, $b, $a)
# ============================================================================
#
# Fill the entire image with a single colour.
#
# Rather than looping over every pixel (O(W*H) Perl iterations), we exploit
# the Perl string repetition operator `x`: pack the pixel once, then repeat
# it W*H times.  This is far faster for large images because the repetition
# is handled in C inside the Perl interpreter.
#
# Example: a 2×2 red-opaque fill:
#   pixel = pack('CCCC', 255, 0, 0, 255)  → "\xFF\x00\x00\xFF"
#   data  = pixel x 4                     → "\xFF\x00\x00\xFF" × 4
#
# @param $r,$g,$b,$a  Fill colour, each 0..255
# ============================================================================
sub fill_pixels {
    my ($c, $r, $g, $b, $a) = @_;

    # Encode one pixel…
    my $pixel = pack('CCCC', $r, $g, $b, $a);

    # …and tile it across the whole buffer.
    $c->{data} = $pixel x ($c->{width} * $c->{height});
}

1;

__END__

=head1 NAME

CodingAdventures::PixelContainer - In-memory RGBA8 pixel buffer (IC00)

=head1 SYNOPSIS

    use CodingAdventures::PixelContainer;

    # Create a 100×80 blank canvas
    my $img = CodingAdventures::PixelContainer->new(100, 80);

    # Write a red pixel at (10, 20)
    $img->set_pixel(10, 20, 255, 0, 0, 255);

    # Read it back
    my ($r, $g, $b, $a) = $img->pixel_at(10, 20);
    # ($r, $g, $b, $a) == (255, 0, 0, 255)

    # Fill with solid blue
    $img->fill_pixels(0, 0, 255, 255);

    # Access raw byte buffer for codec output
    my $raw_ref = $img->data;   # scalar reference

=head1 DESCRIPTION

C<CodingAdventures::PixelContainer> is the IC00 layer in the coding-adventures
image pipeline.  It stores an RGBA8 raster image as a contiguous byte string
using Perl's native string type as a byte buffer.

Pixel (x, y) lives at byte offset C<(y * width + x) * 4> in the buffer.
Channels are ordered R, G, B, A.  Out-of-bounds reads return C<(0,0,0,0)>;
out-of-bounds writes are silently ignored.

=head1 METHODS

=over 4

=item C<new($width, $height)>

Construct a W×H image filled with C<(0,0,0,0)> (transparent black).
Dies with C<"InvalidInput: ..."> if either dimension is not a positive integer.

=item C<width()>

Return the image width in pixels.

=item C<height()>

Return the image height in pixels.

=item C<data()>

Return a scalar reference to the internal byte buffer.  Useful for codecs
that need to read or replace the raw bytes directly.

=item C<pixel_at($x, $y)>

Return C<($r, $g, $b, $a)> for the pixel at column C<$x>, row C<$y>.
Returns C<(0, 0, 0, 0)> for out-of-bounds coordinates.

=item C<set_pixel($x, $y, $r, $g, $b, $a)>

Write the pixel at C<($x, $y)> with the given RGBA values.
No-op for out-of-bounds coordinates.

=item C<fill_pixels($r, $g, $b, $a)>

Fill the entire image with a single colour using the Perl C<x> repetition
operator for efficiency.

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
