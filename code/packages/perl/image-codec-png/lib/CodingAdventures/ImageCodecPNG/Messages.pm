package CodingAdventures::ImageCodecPNG::Messages;

use strict;
use warnings;
use Exporter 'import';
use CodingAdventures::ImageCodecPNG::Constants qw(:all);

# Standing Rule #20: 100% of error messages, diagnostic strings, and placeholders
# are strictly isolated in this module and guaranteed payload-blind.

use constant REDACTED_PLACEHOLDER  => '<redacted>';
use constant INVALID_PLACEHOLDER   => '<invalid>';
use constant UNKNOWN_ERROR_MESSAGE => 'image-codec-png: unknown error';

our %ERROR_MESSAGES = (
    ERR_INVALID_MAX_PIXELS()        => 'image-codec-png: max_pixels must be a positive integer <= 33554432',
    ERR_INVALID_IMAGE_DIMENSIONS()  => 'image-codec-png: image dimensions must be positive integers within limit',
    ERR_INVALID_PIXEL_DATA_LENGTH() => 'image-codec-png: pixel buffer length does not match width * height * 4',
    ERR_FILE_TOO_SHORT()            => 'image-codec-png: input too short to contain PNG signature',
    ERR_INVALID_SIGNATURE()         => 'image-codec-png: invalid PNG signature',
    ERR_TRUNCATED_CHUNK()           => 'image-codec-png: chunk header or payload truncated',
    ERR_INVALID_CHUNK_TYPE()        => 'image-codec-png: invalid chunk type name',
    ERR_CHUNK_CRC_MISMATCH()        => 'image-codec-png: chunk CRC mismatch',
    ERR_CHUNK_BEFORE_IHDR()         => 'image-codec-png: chunk found before IHDR',
    ERR_DUPLICATE_IHDR()            => 'image-codec-png: duplicate IHDR chunk',
    ERR_INVALID_IHDR_LENGTH()       => 'image-codec-png: IHDR length must be exactly 13 bytes',
    ERR_INVALID_DIMENSIONS()        => 'image-codec-png: image dimensions must be non-zero',
    ERR_DIMENSION_LIMIT()           => 'image-codec-png: image dimensions exceed limit',
    ERR_PIXEL_LIMIT()               => 'image-codec-png: image pixel count exceeds limit',
    ERR_UNSUPPORTED_FEATURE()       => 'image-codec-png: unsupported PNG feature or APNG chunk',
    ERR_INVALID_PLTE()              => 'image-codec-png: invalid, duplicate, or misplaced PLTE chunk',
    ERR_INVALID_TRNS()              => 'image-codec-png: invalid, duplicate, or misplaced tRNS chunk',
    ERR_NONCONSECUTIVE_IDAT()       => 'image-codec-png: non-consecutive IDAT chunks',
    ERR_INVALID_IEND()              => 'image-codec-png: IEND chunk must be empty',
    ERR_TRAILING_DATA()             => 'image-codec-png: trailing data found after IEND chunk',
    ERR_UNKNOWN_CRITICAL_CHUNK()    => 'image-codec-png: unknown critical chunk',
    ERR_MISSING_REQUIRED_CHUNK()    => 'image-codec-png: missing required chunk (IHDR, IDAT, or IEND)',
    ERR_INVALID_ZLIB_HEADER()       => 'image-codec-png: invalid zlib header in IDAT',
    ERR_PRESET_DICTIONARY()         => 'image-codec-png: preset dictionary is unsupported',
    ERR_INFLATE_FAILED()            => 'image-codec-png: DEFLATE decompression failed',
    ERR_INFLATED_LENGTH_MISMATCH()  => 'image-codec-png: decompressed length does not match expected scanline size',
    ERR_IDAT_CAVITY()               => 'image-codec-png: unconsumed bytes between compressed data and Adler-32',
    ERR_ADLER_MISMATCH()            => 'image-codec-png: Adler-32 checksum mismatch',
    ERR_INVALID_FILTER()            => 'image-codec-png: invalid scanline filter type',
);

# Resolves a canonical human-readable diagnostic message for an error identifier.
sub message_for {
    my ($code) = @_;
    return UNKNOWN_ERROR_MESSAGE() unless defined $code;
    return $ERROR_MESSAGES{$code} // UNKNOWN_ERROR_MESSAGE();
}

our @EXPORT_OK = qw(
    REDACTED_PLACEHOLDER INVALID_PLACEHOLDER UNKNOWN_ERROR_MESSAGE
    message_for %ERROR_MESSAGES
);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

1;
