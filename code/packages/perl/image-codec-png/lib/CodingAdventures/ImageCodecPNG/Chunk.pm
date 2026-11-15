package CodingAdventures::ImageCodecPNG::Chunk;

use strict;
use warnings;
use CodingAdventures::Zip qw(crc32);
use CodingAdventures::ImageCodecPNG::Constants qw(:all);

# Formats a complete PNG chunk: length (BE u32), type (4 bytes), payload, CRC-32 (BE u32).
sub format {
    my ($type, $payload) = @_;
    $payload //= '';

    my $length = length($payload);
    my $crc = CodingAdventures::Zip::crc32($type);
    if ($length > 0) {
        $crc = CodingAdventures::Zip::crc32($payload, $crc);
    }

    return pack(CHUNK_LENGTH_PACK_FORMAT(), $length)
        . $type
        . $payload
        . pack(CRC_PACK_FORMAT(), $crc);
}

# Validates whether a 4-byte chunk type adheres to RFC 2083 Section 5.4.
# All 4 bytes must be ASCII letters, and the 3rd byte (reserved bit) must be uppercase.
sub valid_type {
    my ($type) = @_;
    return 0 unless defined $type && length($type) == CHUNK_TYPE_SIZE();

    my ($b0, $b1, $b2, $b3) = unpack(CHUNK_TYPE_PACK_FORMAT(), $type);
    return 0 if ($b2 & CHUNK_RESERVED_BIT()) != 0;

    for my $b ($b0, $b1, $b2, $b3) {
        my $is_letter = ($b >= ASCII_UPPER_A() && $b <= ASCII_UPPER_Z()) ||
                        ($b >= ASCII_LOWER_A() && $b <= ASCII_LOWER_Z());
        return 0 unless $is_letter;
    }

    return 1;
}

# Checks whether a chunk type is critical (uppercase first letter).
sub critical {
    my ($type) = @_;
    return 0 unless defined $type && length($type) >= 1;
    my $b0 = ord(substr($type, 0, 1));
    return ($b0 & CHUNK_ANCILLARY_BIT()) == 0;
}

1;
