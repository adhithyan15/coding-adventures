package CodingAdventures::ImageCodecPNG::Constants;

use strict;
use warnings;
use B ();
use Exporter 'import';

# PNG signature bytes and MIME type.
use constant PNG_SIGNATURE      => "\x89PNG\r\n\x1a\n";
use constant SIGNATURE_LENGTH   => 8;
use constant MIME_TYPE_PNG      => 'image/png';

# Chunk layout boundaries (RFC 2083 Section 5.3).
use constant CHUNK_LENGTH_SIZE  => 4;
use constant CHUNK_TYPE_SIZE    => 4;
use constant CHUNK_CRC_SIZE     => 4;
use constant CHUNK_HEADER_SIZE  => 8;
use constant CHUNK_OVERHEAD     => 12;

# Standard chunk types.
use constant CHUNK_IHDR         => 'IHDR';
use constant CHUNK_PLTE         => 'PLTE';
use constant CHUNK_IDAT         => 'IDAT';
use constant CHUNK_IEND         => 'IEND';
use constant CHUNK_TRNS         => 'tRNS';

# Animated PNG (APNG) chunk types.
use constant CHUNK_ACTL         => 'acTL';
use constant CHUNK_FCTL         => 'fcTL';
use constant CHUNK_FDAT         => 'fdAT';

# Chunk type name bit flags.
use constant CHUNK_RESERVED_BIT  => 0x20;
use constant CHUNK_ANCILLARY_BIT => 0x20;

# ASCII character codes for chunk type validation.
use constant ASCII_UPPER_A      => 65;
use constant ASCII_UPPER_Z      => 90;
use constant ASCII_LOWER_A      => 97;
use constant ASCII_LOWER_Z      => 122;

# IHDR structural constraints.
use constant IHDR_LENGTH                    => 13;
use constant SUPPORTED_BIT_DEPTH            => 8;
use constant COLOUR_TYPE_GREY               => 0;
use constant COLOUR_TYPE_RGB                => 2;
use constant COLOUR_TYPE_GREY_ALPHA         => 4;
use constant COLOUR_TYPE_RGBA               => 6;
use constant COMPRESSION_METHOD_DEFLATE     => 0;
use constant FILTER_METHOD_ADAPTIVE         => 0;
use constant INTERLACE_METHOD_NONE          => 0;

# Channel counts by colour type.
use constant CHANNELS_GREY         => 1;
use constant CHANNELS_RGB          => 3;
use constant CHANNELS_GREY_ALPHA   => 2;
use constant CHANNELS_RGBA         => 4;
use constant BYTES_PER_RGBA_PIXEL  => 4;

# Image dimension and pixel count bounds.
use constant PNG_MAX_DIMENSION     => 16_384;
use constant PNG_MAX_PIXELS        => 33_554_432;
use constant DEFAULT_MAX_PIXELS    => 33_554_432;

# Scanline prediction filters (RFC 2083 Section 6).
use constant FILTER_NONE           => 0;
use constant FILTER_SUB            => 1;
use constant FILTER_UP             => 2;
use constant FILTER_AVERAGE        => 3;
use constant FILTER_PAETH          => 4;
use constant FILTER_COUNT          => 5;
use constant MAX_FILTER_TYPE       => 4;

# PLTE constraints.
use constant PLTE_MIN_LENGTH       => 3;
use constant PLTE_MAX_LENGTH       => 768;
use constant PLTE_ENTRY_SIZE       => 3;

# tRNS constraints.
use constant TRNS_GREY_LENGTH      => 2;
use constant TRNS_RGB_LENGTH       => 6;
use constant MAX_SAMPLE_VALUE      => 255;

# Alpha channel values.
use constant TRANSPARENT_ALPHA     => 0;
use constant OPAQUE_ALPHA          => 255;

# RFC 1950 zlib stream layout.
use constant ZLIB_CMF_DEFLATE      => 0x78;
use constant ZLIB_FLG_DEFAULT      => 0x9C;
use constant ZLIB_MIN_STREAM_SIZE  => 6;
use constant ZLIB_HEADER_SIZE      => 2;
use constant ZLIB_TRAILER_SIZE     => 4;
use constant ZLIB_OVERHEAD         => 6;
use constant ZLIB_CMF_METHOD_MASK  => 0x0F;
use constant ZLIB_METHOD_DEFLATE   => 8;
use constant ZLIB_CMF_CINFO_SHIFT  => 4;
use constant ZLIB_CINFO_MAX        => 7;
use constant ZLIB_FCHECK_MODULUS   => 31;
use constant ZLIB_FDICT_MASK       => 0x20;

# Adler-32 algorithm constants.
use constant ADLER_BASE            => 65521;
use constant ADLER_INITIAL_A       => 1;
use constant ADLER_INITIAL_B       => 0;
use constant ADLER_CHUNK_SIZE      => 5552;
use constant ADLER_B_SHIFT         => 16;

# Bit manipulation and arithmetic masks.
use constant BITS_PER_BYTE         => 8;
use constant BYTE_MASK             => 0xFF;
use constant BYTE_MIDPOINT         => 128;
use constant BYTE_MODULUS          => 256;
use constant UINT32_MASK           => 0xFFFFFFFF;

# Pack formats.
use constant CHUNK_LENGTH_PACK_FORMAT => 'N';
use constant CRC_PACK_FORMAT          => 'N';
use constant IHDR_PACK_FORMAT         => 'NNCCCCC';
use constant ADLER32_PACK_FORMAT      => 'N';
use constant TRNS_GREY_PACK_FORMAT    => 'n';
use constant TRNS_RGB_PACK_FORMAT     => 'nnn';
use constant ZLIB_HEADER_PACK_FORMAT  => 'CC';
use constant UINT8_PACK_FORMAT        => 'C';
use constant BYTE_ARRAY_PACK_FORMAT   => 'C*';
use constant RGBA_PACK_FORMAT         => 'CCCC';
use constant CHUNK_TYPE_PACK_FORMAT   => 'CCCC';

# Error identifiers matching fixture taxonomy.
use constant ERR_INVALID_MAX_PIXELS        => 'invalid-max-pixels';
use constant ERR_INVALID_IMAGE_DIMENSIONS  => 'invalid-image-dimensions';
use constant ERR_INVALID_PIXEL_DATA_LENGTH => 'invalid-pixel-data-length';
use constant ERR_FILE_TOO_SHORT            => 'file-too-short';
use constant ERR_INVALID_SIGNATURE         => 'invalid-signature';
use constant ERR_TRUNCATED_CHUNK           => 'truncated-chunk';
use constant ERR_INVALID_CHUNK_TYPE        => 'invalid-chunk-type';
use constant ERR_CHUNK_CRC_MISMATCH        => 'chunk-crc-mismatch';
use constant ERR_CHUNK_BEFORE_IHDR         => 'chunk-before-ihdr';
use constant ERR_DUPLICATE_IHDR            => 'duplicate-ihdr';
use constant ERR_INVALID_IHDR_LENGTH       => 'invalid-ihdr-length';
use constant ERR_INVALID_DIMENSIONS        => 'invalid-dimensions';
use constant ERR_DIMENSION_LIMIT           => 'dimension-limit';
use constant ERR_PIXEL_LIMIT               => 'pixel-limit';
use constant ERR_UNSUPPORTED_FEATURE       => 'unsupported-feature';
use constant ERR_INVALID_PLTE              => 'invalid-plte';
use constant ERR_INVALID_TRNS              => 'invalid-trns';
use constant ERR_NONCONSECUTIVE_IDAT       => 'nonconsecutive-idat';
use constant ERR_INVALID_IEND              => 'invalid-iend';
use constant ERR_TRAILING_DATA             => 'trailing-data';
use constant ERR_UNKNOWN_CRITICAL_CHUNK    => 'unknown-critical-chunk';
use constant ERR_MISSING_REQUIRED_CHUNK    => 'missing-required-chunk';
use constant ERR_INVALID_ZLIB_HEADER       => 'invalid-zlib-header';
use constant ERR_PRESET_DICTIONARY         => 'preset-dictionary';
use constant ERR_INFLATE_FAILED            => 'inflate-failed';
use constant ERR_INFLATED_LENGTH_MISMATCH  => 'inflated-length-mismatch';
use constant ERR_IDAT_CAVITY               => 'idat-cavity';
use constant ERR_ADLER_MISMATCH            => 'adler-mismatch';
use constant ERR_INVALID_FILTER            => 'invalid-filter';
use constant ERROR_UNKNOWN                 => 'unknown-error';

# External raw inflate error codes.
use constant ZIP_ERR_OUTPUT_LIMIT_EXCEEDED => 'output-limit-exceeded';

# Closed set of 29 portable error identifiers in exact fixture specification order.
our @PNG_ERROR_CODES = (
    ERR_INVALID_MAX_PIXELS,
    ERR_INVALID_IMAGE_DIMENSIONS,
    ERR_INVALID_PIXEL_DATA_LENGTH,
    ERR_FILE_TOO_SHORT,
    ERR_INVALID_SIGNATURE,
    ERR_TRUNCATED_CHUNK,
    ERR_INVALID_CHUNK_TYPE,
    ERR_CHUNK_CRC_MISMATCH,
    ERR_CHUNK_BEFORE_IHDR,
    ERR_DUPLICATE_IHDR,
    ERR_INVALID_IHDR_LENGTH,
    ERR_INVALID_DIMENSIONS,
    ERR_DIMENSION_LIMIT,
    ERR_PIXEL_LIMIT,
    ERR_UNSUPPORTED_FEATURE,
    ERR_INVALID_PLTE,
    ERR_INVALID_TRNS,
    ERR_NONCONSECUTIVE_IDAT,
    ERR_INVALID_IEND,
    ERR_TRAILING_DATA,
    ERR_UNKNOWN_CRITICAL_CHUNK,
    ERR_MISSING_REQUIRED_CHUNK,
    ERR_INVALID_ZLIB_HEADER,
    ERR_PRESET_DICTIONARY,
    ERR_INFLATE_FAILED,
    ERR_INFLATED_LENGTH_MISMATCH,
    ERR_IDAT_CAVITY,
    ERR_ADLER_MISMATCH,
    ERR_INVALID_FILTER,
);

our %CHANNELS_BY_COLOUR_TYPE = (
    COLOUR_TYPE_GREY()       => CHANNELS_GREY(),
    COLOUR_TYPE_RGB()        => CHANNELS_RGB(),
    COLOUR_TYPE_GREY_ALPHA() => CHANNELS_GREY_ALPHA(),
    COLOUR_TYPE_RGBA()       => CHANNELS_RGBA(),
);

our @SUPPORTED_COLOUR_TYPES = (
    COLOUR_TYPE_GREY(),
    COLOUR_TYPE_RGB(),
    COLOUR_TYPE_GREY_ALPHA(),
    COLOUR_TYPE_RGBA(),
);

our @APNG_CHUNKS = (
    CHUNK_ACTL(),
    CHUNK_FCTL(),
    CHUNK_FDAT(),
);

# Strict integer type guard: ensures $val is a defined scalar, not a reference/blessed object,
# has integer flags (IOK), and contains no fractional/decimal components.
sub is_strict_integer {
    my ($val) = @_;
    return 0 unless defined $val;
    return 0 if ref($val);
    my $sv = B::svref_2object(\$val);
    return 0 unless ($sv->FLAGS & B::SVf_IOK);
    my $str = "$val";
    return 0 if $str !~ /^[+-]?\d+$/;
    return 1;
}

our @EXPORT_OK = (
    qw(
        PNG_SIGNATURE SIGNATURE_LENGTH MIME_TYPE_PNG
        CHUNK_LENGTH_SIZE CHUNK_TYPE_SIZE CHUNK_CRC_SIZE CHUNK_HEADER_SIZE CHUNK_OVERHEAD
        CHUNK_IHDR CHUNK_PLTE CHUNK_IDAT CHUNK_IEND CHUNK_TRNS
        CHUNK_ACTL CHUNK_FCTL CHUNK_FDAT
        CHUNK_RESERVED_BIT CHUNK_ANCILLARY_BIT
        ASCII_UPPER_A ASCII_UPPER_Z ASCII_LOWER_A ASCII_LOWER_Z
        IHDR_LENGTH SUPPORTED_BIT_DEPTH
        COLOUR_TYPE_GREY COLOUR_TYPE_RGB COLOUR_TYPE_GREY_ALPHA COLOUR_TYPE_RGBA
        COMPRESSION_METHOD_DEFLATE FILTER_METHOD_ADAPTIVE INTERLACE_METHOD_NONE
        CHANNELS_GREY CHANNELS_RGB CHANNELS_GREY_ALPHA CHANNELS_RGBA BYTES_PER_RGBA_PIXEL
        PNG_MAX_DIMENSION PNG_MAX_PIXELS DEFAULT_MAX_PIXELS
        FILTER_NONE FILTER_SUB FILTER_UP FILTER_AVERAGE FILTER_PAETH FILTER_COUNT MAX_FILTER_TYPE
        PLTE_MIN_LENGTH PLTE_MAX_LENGTH PLTE_ENTRY_SIZE
        TRNS_GREY_LENGTH TRNS_RGB_LENGTH MAX_SAMPLE_VALUE
        TRANSPARENT_ALPHA OPAQUE_ALPHA
        ZLIB_CMF_DEFLATE ZLIB_FLG_DEFAULT ZLIB_MIN_STREAM_SIZE ZLIB_HEADER_SIZE ZLIB_TRAILER_SIZE ZLIB_OVERHEAD
        ZLIB_CMF_METHOD_MASK ZLIB_METHOD_DEFLATE ZLIB_CMF_CINFO_SHIFT ZLIB_CINFO_MAX ZLIB_FCHECK_MODULUS ZLIB_FDICT_MASK
        ADLER_BASE ADLER_INITIAL_A ADLER_INITIAL_B ADLER_CHUNK_SIZE ADLER_B_SHIFT
        BITS_PER_BYTE BYTE_MASK BYTE_MIDPOINT BYTE_MODULUS UINT32_MASK
        CHUNK_LENGTH_PACK_FORMAT CRC_PACK_FORMAT IHDR_PACK_FORMAT ADLER32_PACK_FORMAT
        TRNS_GREY_PACK_FORMAT TRNS_RGB_PACK_FORMAT ZLIB_HEADER_PACK_FORMAT UINT8_PACK_FORMAT BYTE_ARRAY_PACK_FORMAT
        RGBA_PACK_FORMAT CHUNK_TYPE_PACK_FORMAT is_strict_integer
        ERR_INVALID_MAX_PIXELS ERR_INVALID_IMAGE_DIMENSIONS ERR_INVALID_PIXEL_DATA_LENGTH
        ERR_FILE_TOO_SHORT ERR_INVALID_SIGNATURE ERR_TRUNCATED_CHUNK ERR_INVALID_CHUNK_TYPE
        ERR_CHUNK_CRC_MISMATCH ERR_CHUNK_BEFORE_IHDR ERR_DUPLICATE_IHDR ERR_INVALID_IHDR_LENGTH
        ERR_INVALID_DIMENSIONS ERR_DIMENSION_LIMIT ERR_PIXEL_LIMIT ERR_UNSUPPORTED_FEATURE
        ERR_INVALID_PLTE ERR_INVALID_TRNS ERR_NONCONSECUTIVE_IDAT ERR_INVALID_IEND
        ERR_TRAILING_DATA ERR_UNKNOWN_CRITICAL_CHUNK ERR_MISSING_REQUIRED_CHUNK
        ERR_INVALID_ZLIB_HEADER ERR_PRESET_DICTIONARY ERR_INFLATE_FAILED
        ERR_INFLATED_LENGTH_MISMATCH ERR_IDAT_CAVITY ERR_ADLER_MISMATCH ERR_INVALID_FILTER
        ERROR_UNKNOWN ZIP_ERR_OUTPUT_LIMIT_EXCEEDED
    ),
    qw(@PNG_ERROR_CODES %CHANNELS_BY_COLOUR_TYPE @SUPPORTED_COLOUR_TYPES @APNG_CHUNKS),
);

our %EXPORT_TAGS = (all => \@EXPORT_OK);

1;
