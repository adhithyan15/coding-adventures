package CodingAdventures::ImageCodec;

# ============================================================================
# CodingAdventures::ImageCodec — Base Interface for Image Codecs
# ============================================================================
#
# # What Is a Codec Interface?
#
# In object-oriented design, an "interface" (sometimes called a "role" or
# "protocol") is a contract: any module that claims to be an image codec must
# provide these three operations:
#
#   mime_type()              — return the MIME type string (e.g. 'image/bmp')
#   encode($container)       — convert a PixelContainer to raw bytes
#   decode($bytes)           — parse raw bytes into a PixelContainer
#
# Perl does not enforce interfaces at the language level (unlike Java or
# Typescript), so this module serves as documentation and a namespace anchor.
# Implementers should subclass or simply provide the same sub names.
#
# # The Image Codec Stack
#
#   IC00  PixelContainer  — raw RGBA8 pixel storage
#   IC01  ImageCodecBMP   — BMP (Windows Bitmap) encode/decode
#   IC02  ImageCodecPPM   — PPM (Portable Pixmap) encode/decode
#   IC03  ImageCodecQOI   — QOI (Quite OK Image) encode/decode
#         ^^^^^^^^^^^^^^^^^
#         All implement this interface
#
# # Contract Summary
#
#   mime_type()
#     Returns: string like 'image/bmp', 'image/x-ppm', 'image/qoi'
#
#   encode($container)
#     Input:  CodingAdventures::PixelContainer object
#     Output: binary string (the encoded image file bytes)
#     Errors: die "EncodeError: ..." on bad input
#
#   decode($bytes)
#     Input:  binary string (the encoded image file bytes)
#     Output: CodingAdventures::PixelContainer object
#     Errors: die "DecodeError: ..." or "FormatError: ..." on bad data
#
# ============================================================================

use strict;
use warnings;
use 5.026;

our $VERSION = '0.01';

1;

__END__

=head1 NAME

CodingAdventures::ImageCodec - Base interface for image codecs (IC00)

=head1 SYNOPSIS

    # This module is a documentation anchor — no methods to call directly.
    # Codec implementers provide: mime_type(), encode($container), decode($bytes).

    package CodingAdventures::ImageCodecBMP;
    use CodingAdventures::ImageCodec;   # signals intent to implement the interface

    sub mime_type { 'image/bmp' }
    sub encode_bmp { ... }
    sub decode_bmp { ... }

=head1 DESCRIPTION

Defines the conceptual interface all image codecs in the coding-adventures
pipeline must implement.  Since Perl does not enforce interface contracts at
compile time, this module serves as a namespace anchor and living documentation.

Any codec module should provide:

=over 4

=item C<mime_type()>

Return the MIME type string for the format (e.g. C<'image/bmp'>).

=item C<encode($container)>

Accept a L<CodingAdventures::PixelContainer> and return the encoded binary
bytes as a Perl string.

=item C<decode($bytes)>

Accept a binary string and return a L<CodingAdventures::PixelContainer>.
Should die with a meaningful message on malformed input.

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
