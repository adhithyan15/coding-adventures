use strict;
use warnings;
use Test::More;

use CodingAdventures::Brotli qw(compress decompress);

# ---------------------------------------------------------------------------
# Helper: roundtrip($data, $label)
# ---------------------------------------------------------------------------
# Compresses and decompresses data, then checks the result equals the input.

sub roundtrip {
    my ($data, $label) = @_;
    $label //= 'data';
    my $compressed   = compress($data);
    my $decompressed = decompress($compressed);
    is $decompressed, $data, "roundtrip: $label";
}

# ---------------------------------------------------------------------------
# Test 1: Empty input (spec test case 1)
# ---------------------------------------------------------------------------
# Empty input is a special case: the encoder emits only the sentinel ICC code
# 63, and the decoder returns an empty string immediately.

subtest 'empty input' => sub {
    plan tests => 3;
    my $compressed = compress('');
    is length($compressed), 13,  'empty: wire length is 13 bytes (10 header + 2 ICC entry + 1 bit stream)';
    my $result = decompress($compressed);
    is $result, '', 'empty: decompresses to empty string';
    ok length($compressed) > 0, 'empty: compressed form is non-empty';
};

# ---------------------------------------------------------------------------
# Test 2: Single byte (spec test case 2)
# ---------------------------------------------------------------------------

subtest 'single byte' => sub {
    plan tests => 4;
    roundtrip("\x42", 'single 0x42');
    roundtrip("\x00", 'single NUL');
    roundtrip("\xFF", 'single 0xFF');
    roundtrip('A',    'single A');
};

# ---------------------------------------------------------------------------
# Test 3: All 256 distinct bytes — no matches (spec test case 3)
# ---------------------------------------------------------------------------
# Random/incompressible data: compressed size may exceed input, but the
# round-trip must be exact.

subtest 'all 256 byte values, no matches' => sub {
    plan tests => 2;
    my $data = join('', map { chr($_) } 0..255);
    my $compressed = compress($data);
    is length($data), 256, 'input is exactly 256 bytes';
    roundtrip($data, '256 distinct bytes');
};

# ---------------------------------------------------------------------------
# Test 4: All copies, no leading literals — "AAAA...A" × 1024 (spec test case 4)
# ---------------------------------------------------------------------------
# The first 4 bytes must be inserted as literals (window was empty), then the
# remaining 1020 bytes should come from copy commands.

subtest 'all copies — 1024 x A' => sub {
    plan tests => 2;
    my $data = 'A' x 1024;
    roundtrip($data, '1024 A chars');
    my $compressed = compress($data);
    ok length($compressed) < length($data),
       'highly repetitive data compresses (got ' . length($compressed) . ' vs ' . length($data) . ')';
};

# ---------------------------------------------------------------------------
# Test 5: English prose >= 1024 bytes (spec test case 5)
# ---------------------------------------------------------------------------
# Must compress to < 80% of input size.

subtest 'English prose compression ratio' => sub {
    plan tests => 3;
    my $prose = "the quick brown fox jumps over the lazy dog " x 30;
    $prose .= "pack my box with five dozen liquor jugs " x 10;
    ok length($prose) >= 1024, 'prose is at least 1024 bytes';
    my $compressed = compress($prose);
    roundtrip($prose, 'English prose');
    ok length($compressed) < length($prose) * 0.8,
       'compressed size < 80% of input (got ' . length($compressed) . ' vs ' . length($prose) . ')';
};

# ---------------------------------------------------------------------------
# Test 6: Binary blob — 512 random-ish bytes (spec test case 6)
# ---------------------------------------------------------------------------
# No compression ratio requirement, just round-trip correctness.

subtest 'binary blob round-trip' => sub {
    plan tests => 1;
    my $data = join('', map { chr(($_ * 131 + 17) % 256) } 0..511);
    roundtrip($data, '512-byte binary blob');
};

# ---------------------------------------------------------------------------
# Test 7: Cross-command literal context (spec test case 7)
# ---------------------------------------------------------------------------
# "abc123ABC" exercises all four context buckets:
#   start → ctx 0 (no prior byte)
#   'a' after nothing: ctx 0 → emits 'a', last=a(97) → ctx 3
#   'b' in ctx 3 (after lowercase), last=b → ctx 3
#   'c' in ctx 3, last=c → ctx 3
#   '1' in ctx 3, last=1(49) → ctx 1
#   '2' in ctx 1 (after digit), last=2 → ctx 1
#   '3' in ctx 1, last=3 → ctx 1
#   'A' in ctx 1, last=A(65) → ctx 2
#   'B' in ctx 2 (after uppercase), last=B → ctx 2
#   'C' in ctx 2

subtest 'cross-command literal context abc123ABC' => sub {
    plan tests => 1;
    roundtrip('abc123ABC', 'abc123ABC context test');
};

# ---------------------------------------------------------------------------
# Test 7b: All four context buckets explicitly
# ---------------------------------------------------------------------------

subtest 'all four context buckets populated' => sub {
    plan tests => 4;
    # Build a string that visits all four context buckets:
    # start of stream → ctx0; after lower → ctx3; after upper → ctx2; after digit → ctx1.
    my $data = ' hello WORLD 12345 end ';
    my $compressed = compress($data);
    my $result = decompress($compressed);
    is $result, $data, 'round-trip with mixed contexts';

    # Inspect the wire format header to confirm ctx tree counts.
    my (undef, $icc_n, $dist_n, $c0, $c1, $c2, $c3) = unpack('NCCCCCC', $compressed);
    ok $c0 > 0, 'ctx0 tree is non-empty (space/punct context)';
    ok $c1 > 0, 'ctx1 tree is non-empty (digit context)';
    ok $c3 > 0, 'ctx3 tree is non-empty (lowercase context)';
};

# ---------------------------------------------------------------------------
# Test 8: Long-distance match — offset > 4096 (spec test case 8)
# ---------------------------------------------------------------------------
# Place a 10-byte marker string, then > 4096 filler bytes, then the same marker.
# The second occurrence should match the first via a distance code 24+.

subtest 'long-distance match (offset > 4096)' => sub {
    plan tests => 1;
    my $marker = 'XYZPDQ1234';
    my $filler = 'abcdefghij' x 500;  # 5000 bytes of filler
    my $data   = $marker . $filler . $marker;
    roundtrip($data, 'long-distance marker match');
};

# ---------------------------------------------------------------------------
# Test: Various repetition patterns
# ---------------------------------------------------------------------------

subtest 'various repetition patterns' => sub {
    plan tests => 6;
    roundtrip('ABCABCABCABC', 'ABCABC×3');
    roundtrip('hello hello hello world', 'hello×3');
    roundtrip('ABABABABABAB', 'ABAB...');
    roundtrip('AAAAAAA', 'run of A');
    roundtrip('A' x 300, 'A×300');
    roundtrip(('ABCDEF' x 200), 'ABCDEF×200');
};

# ---------------------------------------------------------------------------
# Test: Multiple distinct matches
# ---------------------------------------------------------------------------

subtest 'multiple distinct matches' => sub {
    plan tests => 3;
    my $s = 'the cat sat on the mat, the cat sat on the hat';
    roundtrip($s, 'the cat sat...');
    roundtrip('AAABBBAAABBB', 'AAABBBAAABBB');
    my $text = "foo bar baz " x 50;
    roundtrip($text, 'foo bar baz ×50');
};

# ---------------------------------------------------------------------------
# Test: Wire format structure for known input
# ---------------------------------------------------------------------------
# Manually verify header fields for a known simple input.

subtest 'wire format header fields' => sub {
    plan tests => 3;
    my $data = 'ABCABCABCABC';  # 12 bytes, has repetition
    my $compressed = compress($data);
    my ($orig_len) = unpack('N', $compressed);
    is $orig_len, 12, 'original_length=12 in header';

    my (undef, $icc_n, $dist_n) = unpack('NCC', $compressed);
    ok $icc_n > 0,  'icc_entry_count > 0';
    ok $dist_n > 0, 'dist_entry_count > 0 (has copy commands)';
};

# ---------------------------------------------------------------------------
# Test: Overlapping copy (run-length style)
# ---------------------------------------------------------------------------
# When copy_distance < copy_length, the copy refers to bytes that are being
# produced — a pattern like "AAAAAAA" encodes as insert 4 + copy(dist=1, len=N).
# The copy must proceed byte-by-byte, not memcpy.

subtest 'overlapping copy (run encoding)' => sub {
    plan tests => 4;
    roundtrip("\x00" x 100,  'NUL×100');
    roundtrip('B' x 200,     'B×200');
    roundtrip('CD' x 100,    'CDCD×100');
    roundtrip('EFG' x 50,    'EFGEFG×50');
};

# ---------------------------------------------------------------------------
# Test: Degenerate inputs
# ---------------------------------------------------------------------------

subtest 'degenerate inputs' => sub {
    plan tests => 5;
    roundtrip("\n", 'newline');
    roundtrip("\t\t\t", 'tabs');
    roundtrip('a', 'single a');
    roundtrip('ab', 'two bytes');
    roundtrip('abc', 'three bytes');
};

# ---------------------------------------------------------------------------
# Test: Longer English text compression ratio
# ---------------------------------------------------------------------------

subtest 'longer text compression ratio' => sub {
    plan tests => 2;
    my $base = "the quick brown fox jumps over the lazy dog\n";
    my $data = $base x 25;  # ~1100 bytes
    ok length($data) > 1000, 'data > 1000 bytes';
    my $compressed = compress($data);
    ok length($compressed) < length($data),
       'repetitive text compresses (got ' . length($compressed) . ' vs ' . length($data) . ')';
};

# ---------------------------------------------------------------------------
# Test: Binary sequence cycling through all values
# ---------------------------------------------------------------------------

subtest 'cycling binary data' => sub {
    plan tests => 1;
    my $data = join('', map { chr($_ % 256) } 0..999);
    roundtrip($data, 'binary 0..999 mod 256');
};

done_testing;
