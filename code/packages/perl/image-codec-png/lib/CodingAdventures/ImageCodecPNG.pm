package CodingAdventures::ImageCodecPNG;

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.1.0';

use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Messages qw(:all);
use CodingAdventures::ImageCodecPNG::Error;
use CodingAdventures::ImageCodecPNG::Adler32;
use CodingAdventures::ImageCodecPNG::Chunk;
use CodingAdventures::ImageCodecPNG::Filter;
use CodingAdventures::ImageCodecPNG::Decoder;
use CodingAdventures::ImageCodecPNG::Encoder;
use CodingAdventures::ImageCodecPNG::Codec;

our @EXPORT_OK = qw(
    encode_png
    decode_png
    adler32
    png_error_codes
    PNG_MAX_DIMENSION
    PNG_MAX_PIXELS
    codec
);

our %EXPORT_TAGS = (all => \@EXPORT_OK);

# Ordered list of all 29 closed error codes.
sub png_error_codes {
    return @CodingAdventures::ImageCodecPNG::Constants::PNG_ERROR_CODES;
}

# Encodes a PixelContainer::Container to PNG binary bytes.
sub encode_png {
    my ($pixels) = @_;
    return CodingAdventures::ImageCodecPNG::Encoder::encode($pixels);
}

# Decodes a bounded, non-interlaced 8-bit PNG stream into a PixelContainer::Container.
sub decode_png {
    my ($data, @opts) = @_;
    return CodingAdventures::ImageCodecPNG::Decoder::decode($data, @opts);
}

# Computes the RFC 1950 Adler-32 checksum.
sub adler32 {
    my ($data) = @_;
    return CodingAdventures::ImageCodecPNG::Adler32::compute($data);
}

my $DEFAULT_CODEC;

# Returns a default singleton codec instance or a custom configured instance.
sub codec {
    my (%opts) = @_;
    if (%opts) {
        return CodingAdventures::ImageCodecPNG::Codec->new(%opts);
    }
    return $DEFAULT_CODEC //= CodingAdventures::ImageCodecPNG::Codec->new();
}

1;

__END__

=head1 NAME

CodingAdventures::ImageCodecPNG - Portable bounded IC18 PNG encoder and decoder

=head1 SYNOPSIS

    use CodingAdventures::ImageCodecPNG qw(encode_png decode_png adler32);
    use CodingAdventures::PixelContainer;

    my $container = CodingAdventures::PixelContainer->new(2, 2);
    $container->set_pixel(0, 0, 255, 0, 0, 255);

    my $png_bytes = encode_png($container);
    my $decoded   = decode_png($png_bytes);

=head1 DESCRIPTION

Pure-Perl, memory-safe, bounded implementation of the RFC 2083 Portable Network
Graphics format specification (IC18), reusing repository ZIP DEFLATE and CRC-32
primitives and PixelContainer raster buffers.

=cut
