package CodingAdventures::ImageCodecPNG::Encoder;

use strict;
use warnings;
use Scalar::Util qw(blessed looks_like_number);
use CodingAdventures::Zip qw(raw_deflate);
use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Error;
use CodingAdventures::ImageCodecPNG::Chunk;
use CodingAdventures::ImageCodecPNG::Filter;
use CodingAdventures::ImageCodecPNG::Adler32;

# Encodes an RGBA8 PixelContainer buffer into deterministic portable PNG bytes.
sub encode {
    my ($pixels) = @_;

    unless (defined $pixels && blessed($pixels) &&
            $pixels->can('width') && $pixels->can('height') && $pixels->can('data')) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_IMAGE_DIMENSIONS());
    }

    my $width = $pixels->width;
    my $height = $pixels->height;

    unless (is_strict_integer($width) && $width > 0 && $width <= PNG_MAX_DIMENSION() &&
            is_strict_integer($height) && $height > 0 && $height <= PNG_MAX_DIMENSION()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_IMAGE_DIMENSIONS());
    }

    my $pixel_count = $width * $height;
    if ($pixel_count > PNG_MAX_PIXELS()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_IMAGE_DIMENSIONS());
    }

    my $dref = $pixels->data;
    unless (defined $dref && ref($dref) eq 'SCALAR' && defined($$dref) &&
            !utf8::is_utf8($$dref) &&
            length($$dref) == $pixel_count * BYTES_PER_RGBA_PIXEL()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_PIXEL_DATA_LENGTH());
    }

    my $ihdr_payload = pack(
        IHDR_PACK_FORMAT(),
        $width,
        $height,
        SUPPORTED_BIT_DEPTH(),
        COLOUR_TYPE_RGBA(),
        COMPRESSION_METHOD_DEFLATE(),
        FILTER_METHOD_ADAPTIVE(),
        INTERLACE_METHOD_NONE(),
    );

    my $filtered = filter_scanlines($$dref, $width, $height);
    my $adler = CodingAdventures::ImageCodecPNG::Adler32::compute($filtered);
    my $adler_bytes = pack(ADLER32_PACK_FORMAT(), $adler);

    my $deflated = CodingAdventures::Zip::raw_deflate($filtered);
    my $zlib_stream = pack(ZLIB_HEADER_PACK_FORMAT(), ZLIB_CMF_DEFLATE(), ZLIB_FLG_DEFAULT())
        . $deflated
        . $adler_bytes;

    return PNG_SIGNATURE()
        . CodingAdventures::ImageCodecPNG::Chunk::format(CHUNK_IHDR(), $ihdr_payload)
        . CodingAdventures::ImageCodecPNG::Chunk::format(CHUNK_IDAT(), $zlib_stream)
        . CodingAdventures::ImageCodecPNG::Chunk::format(CHUNK_IEND());
}

# Applies adaptive row filtering across all scanlines.
sub filter_scanlines {
    my ($raw_data, $width, $height) = @_;
    my $stride = $width * BYTES_PER_RGBA_PIXEL();
    my $bpp = CHANNELS_RGBA();

    my @prior = (0) x $stride;
    my @raw = (0) x $stride;
    my @scratch = (0) x $stride;
    my @best = (0) x $stride;
    my $filtered = '';

    for (my $y = 0; $y < $height; $y++) {
        my $row_offset = $y * $stride;
        my $row_slice = substr($raw_data, $row_offset, $stride);
        @raw = unpack(BYTE_ARRAY_PACK_FORMAT(), $row_slice);

        my $filter_type = CodingAdventures::ImageCodecPNG::Filter::choose_filter(
            \@raw, \@prior, $bpp, \@scratch, \@best
        );

        $filtered .= pack(UINT8_PACK_FORMAT(), $filter_type);
        $filtered .= pack(BYTE_ARRAY_PACK_FORMAT(), @best);

        @prior = @raw;
    }

    return $filtered;
}

1;
