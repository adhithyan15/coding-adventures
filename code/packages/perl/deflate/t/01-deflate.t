use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Deflate qw(compress decompress);

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

sub roundtrip {
    my ($data, $label) = @_;
    $label //= 'data';
    my $compressed   = compress($data);
    my $decompressed = decompress($compressed);
    is $decompressed, $data, "roundtrip: $label";
}

# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------

subtest 'empty input' => sub {
    my $compressed = compress('');
    my $result     = decompress($compressed);
    is $result, '', 'empty decompresses to empty';
};

subtest 'single byte 0x00' => sub {
    roundtrip("\x00", 'single NUL');
};

subtest 'single byte 0xFF' => sub {
    roundtrip("\xFF", 'single 0xFF');
};

subtest 'single byte A' => sub {
    roundtrip('A', 'single A');
};

subtest 'single byte repeated' => sub {
    roundtrip('A' x 20, 'A×20');
    roundtrip("\x00" x 100, 'NUL×100');
};

# ---------------------------------------------------------------------------
# Spec examples
# ---------------------------------------------------------------------------

subtest 'AAABBC — all literals' => sub {
    my $data = 'AAABBC';
    roundtrip($data, 'AAABBC');
    my $compressed = compress($data);
    my (undef, undef, $dist_count) = unpack('Nnn', $compressed);
    is $dist_count, 0, 'dist_entry_count=0 for all-literals input';
};

subtest 'AABCBBABC — one match' => sub {
    my $data = 'AABCBBABC';
    roundtrip($data, 'AABCBBABC');
    my $compressed = compress($data);
    my ($orig_len, undef, $dist_count) = unpack('Nnn', $compressed);
    is $orig_len, 9, 'original_length=9';
    ok $dist_count > 0, 'dist_entry_count>0 for input with a match';
};

# ---------------------------------------------------------------------------
# Match tests
# ---------------------------------------------------------------------------

subtest 'overlapping match (run encoding)' => sub {
    roundtrip('AAAAAAA', 'run of A');
    roundtrip('ABABABABABAB', 'ABAB...');
};

subtest 'multiple matches' => sub {
    roundtrip('ABCABCABCABC', 'ABCABC×3');
    roundtrip('hello hello hello world', 'hello×3');
};

subtest 'max match length ~255' => sub {
    roundtrip('A' x 300, 'A×300');
};

# ---------------------------------------------------------------------------
# Data variety
# ---------------------------------------------------------------------------

subtest 'all 256 byte values' => sub {
    my $data = join('', map { chr($_) } 0..255);
    roundtrip($data, 'all-bytes');
};

subtest 'binary data 1000 bytes' => sub {
    my $data = join('', map { chr($_ % 256) } 0..999);
    roundtrip($data, 'binary-1000');
};

subtest 'longer text with repetition' => sub {
    my $base = 'the quick brown fox jumps over the lazy dog ';
    roundtrip($base x 10, 'pangram×10');
};

# ---------------------------------------------------------------------------
# Compression ratio
# ---------------------------------------------------------------------------

subtest 'compression ratio' => sub {
    my $data = 'ABCABC' x 100;
    my $compressed = compress($data);
    ok length($compressed) < length($data) / 2,
       "highly repetitive data compresses to < 50% (got " . length($compressed) . " vs " . length($data) . ")";
};

# ---------------------------------------------------------------------------
# Various match lengths
# ---------------------------------------------------------------------------

subtest 'various match lengths' => sub {
    for my $length (3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255) {
        my $prefix = 'A' x $length;
        my $data   = $prefix . 'BBB' . $prefix;
        roundtrip($data, "length=$length");
    }
};

done_testing;
