package CodingAdventures::ImageCodecPNG::Adler32;

use strict;
use warnings;
use CodingAdventures::ImageCodecPNG::Constants qw(:all);

# Computes the RFC 1950 Adler-32 checksum.
#
# @param $data [Scalar, ArrayRef] binary data string or arrayref of byte values
# @return [Integer] 32-bit unsigned Adler-32 checksum
sub compute {
    my ($data) = @_;
    my $bytes = ref($data) eq 'ARRAY'
        ? pack(BYTE_ARRAY_PACK_FORMAT(), @$data)
        : $data;

    if (defined $bytes && utf8::is_utf8($bytes)) {
        utf8::downgrade($bytes, 1);
    }

    my $len = length($bytes);
    my $a = ADLER_INITIAL_A();
    my $b = ADLER_INITIAL_B();
    my $pos = 0;

    while ($pos < $len) {
        my $chunk_len = $len - $pos;
        $chunk_len = ADLER_CHUNK_SIZE() if $chunk_len > ADLER_CHUNK_SIZE();
        my $chunk = substr($bytes, $pos, $chunk_len);
        for my $byte (unpack(BYTE_ARRAY_PACK_FORMAT(), $chunk)) {
            $a += $byte;
            $b += $a;
        }
        $a %= ADLER_BASE();
        $b %= ADLER_BASE();
        $pos += $chunk_len;
    }

    return (($b << ADLER_B_SHIFT()) | $a) & UINT32_MASK();
}

1;
