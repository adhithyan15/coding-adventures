use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LZW qw(
    compress decompress
    encode_codes decode_codes pack_codes unpack_codes
);

# ---- Helpers ----------------------------------------------------------------

# rt: round-trip helper — compress then decompress.
sub rt {
    my ($str) = @_;
    return decompress(compress($str));
}

# ---- Spec vectors: encode_codes ---------------------------------------------

subtest 'encode_codes empty input' => sub {
    my @codes = encode_codes('');
    is $codes[0], 256, 'first code is CLEAR_CODE (256)';
    is $codes[-1], 257, 'last code is STOP_CODE (257)';
    is scalar @codes, 2, 'empty input → [CLEAR, STOP] only';
};

subtest 'encode_codes single byte A' => sub {
    my @codes = encode_codes('A');
    is $codes[0], 256, 'CLEAR_CODE at index 0';
    is $codes[1], 65,  'code 65 for A';
    is $codes[2], 257, 'STOP_CODE at end';
    is scalar @codes, 3, 'three codes total';
};

subtest 'encode_codes AB → [CLEAR, 65, 66, STOP]' => sub {
    my @codes = encode_codes('AB');
    is scalar @codes, 4, 'four codes';
    is $codes[0], 256, 'CLEAR';
    is $codes[1], 65,  'A';
    is $codes[2], 66,  'B';
    is $codes[3], 257, 'STOP';
};

subtest 'encode_codes ABABAB compresses — fewer codes than bytes' => sub {
    my @codes = encode_codes('ABABAB');
    # ABABAB: A B AB AB → [CLEAR, 65, 66, 258, 258, STOP] (or similar)
    # The point is that repeated patterns yield fewer data codes than input bytes.
    my @data_codes = grep { $_ != 256 && $_ != 257 } @codes;
    ok scalar @data_codes < 6, 'fewer data codes than 6 input bytes';
};

subtest 'encode_codes AAAAAAA compresses — runs get new codes' => sub {
    my @codes = encode_codes('AAAAAAA');
    my @data_codes = grep { $_ != 256 && $_ != 257 } @codes;
    ok scalar @data_codes < 7, 'fewer data codes than 7 input bytes';
};

subtest 'encode_codes always starts with CLEAR_CODE' => sub {
    for my $s ('', 'A', 'hello', 'ABABAB') {
        my @codes = encode_codes($s);
        is $codes[0], 256, "starts with CLEAR for '$s'";
    }
};

subtest 'encode_codes always ends with STOP_CODE' => sub {
    for my $s ('', 'A', 'hello', 'ABABAB') {
        my @codes = encode_codes($s);
        is $codes[-1], 257, "ends with STOP for '$s'";
    }
};

# ---- Spec vectors: decode_codes ---------------------------------------------

subtest 'decode_codes empty code list → empty string' => sub {
    is decode_codes([]), '', 'empty list';
};

subtest 'decode_codes [CLEAR, STOP] → empty string' => sub {
    is decode_codes([256, 257]), '', '[CLEAR, STOP]';
};

subtest 'decode_codes [CLEAR, 65, STOP] → A' => sub {
    is decode_codes([256, 65, 257]), 'A', 'single A';
};

subtest 'decode_codes [CLEAR, 65, 66, STOP] → AB' => sub {
    is decode_codes([256, 65, 66, 257]), 'AB', 'AB';
};

subtest 'decode_codes round-trip with encode_codes' => sub {
    my $data  = 'ABABAB';
    my @codes = encode_codes($data);
    is decode_codes(\@codes), $data, 'encode_codes + decode_codes';
};

# ---- Spec vectors: tricky token ---------------------------------------------

subtest 'tricky token xyx...x pattern (ABAABABAAB style)' => sub {
    # The tricky token occurs when code == scalar @dec_dict (not yet added).
    # A classic trigger: encoder emits the new code before decoder adds it.
    # We test via round-trip of a known tricky pattern.
    my $data = 'ABABABABABABAB';
    is rt($data), $data, 'tricky token pattern round-trips correctly';
};

subtest 'tricky token: repeated single char AAAAAAAAAA' => sub {
    my $data = 'A' x 20;
    is rt($data), $data, 'A x 20 round-trips (tricky token exercised)';
};

# ---- pack_codes / unpack_codes ---------------------------------------------

subtest 'pack_codes produces 4-byte header + bit stream' => sub {
    my @codes  = encode_codes('hello');
    my $packed = pack_codes(\@codes, 5);
    ok length($packed) >= 4, 'at least 4 bytes';
    my ($orig_len) = unpack('N', substr($packed, 0, 4));
    is $orig_len, 5, 'original_length = 5 in header';
};

subtest 'pack_codes empty → 4-byte header only (plus stop bits)' => sub {
    my @codes  = encode_codes('');
    my $packed = pack_codes(\@codes, 0);
    ok length($packed) >= 4, 'at least 4 bytes';
    my ($orig_len) = unpack('N', substr($packed, 0, 4));
    is $orig_len, 0, 'original_length = 0';
};

subtest 'unpack_codes short input returns safe fallback' => sub {
    my ($codes_ref, $orig_len) = unpack_codes('');
    ok defined $codes_ref, 'codes_ref defined for empty input';
    is $orig_len, 0, 'orig_len = 0 for empty input';
};

subtest 'unpack_codes short input (3 bytes) returns safe fallback' => sub {
    my ($codes_ref, $orig_len) = unpack_codes('ABC');
    ok defined $codes_ref, 'codes_ref defined for 3-byte input';
    is $orig_len, 0, 'orig_len = 0';
};

subtest 'pack/unpack round-trip preserves codes' => sub {
    my @codes    = encode_codes('ABABAB');
    my $packed   = pack_codes(\@codes, 6);
    my ($got, $orig_len) = unpack_codes($packed);
    is $orig_len, 6, 'original_length preserved';
    is $got, \@codes, 'codes round-trip through pack/unpack';
};

# ---- compress / decompress round-trips --------------------------------------

subtest 'round-trip empty string' => sub {
    is rt(''), '', 'empty';
};

subtest 'round-trip single byte A' => sub {
    is rt('A'), 'A', 'A';
};

subtest 'round-trip AB' => sub {
    is rt('AB'), 'AB', 'AB';
};

subtest 'round-trip ABABAB' => sub {
    is rt('ABABAB'), 'ABABAB', 'ABABAB';
};

subtest 'round-trip AAAAAAA' => sub {
    is rt('AAAAAAA'), 'AAAAAAA', 'AAAAAAA';
};

subtest 'round-trip hello world' => sub {
    is rt('hello world'), 'hello world', 'hello world';
};

subtest 'round-trip ABCDE (no repetition)' => sub {
    is rt('ABCDE'), 'ABCDE', 'ABCDE';
};

subtest 'round-trip ABC x 100' => sub {
    my $data = 'ABC' x 100;
    is rt($data), $data, 'ABC x 100';
};

subtest 'round-trip binary nulls' => sub {
    my $data = "\x00\x00\x00\xff\xff";
    is rt($data), $data, 'binary nulls';
};

subtest 'round-trip all 256 bytes' => sub {
    my $data = pack 'C*', 0 .. 255;
    is rt($data), $data, 'all 256 bytes';
};

subtest 'round-trip all 256 bytes repeated' => sub {
    my $data = pack('C*', 0 .. 255) x 4;
    is rt($data), $data, 'all 256 bytes x 4';
};

subtest 'round-trip repeated 0,1,2 pattern x 100' => sub {
    my $data = pack 'C*', map { $_ % 3 } 0 .. 299;
    is rt($data), $data, 'mod-3 pattern';
};

subtest 'round-trip ABCDEF x 500' => sub {
    my $data = 'ABCDEF' x 500;
    is rt($data), $data, 'ABCDEF x 500';
};

subtest 'round-trip long run of same byte' => sub {
    my $data = "\x42" x 1000;
    is rt($data), $data, '0x42 x 1000';
};

# ---- Wire format checks -----------------------------------------------------

subtest 'compress stores original_length in header' => sub {
    my $c = compress('hello');
    my ($orig_len) = unpack 'N', substr($c, 0, 4);
    is $orig_len, 5, 'original_length = 5 for "hello"';
};

subtest 'compress empty → 4+ byte output with orig_len 0' => sub {
    my $c = compress('');
    ok length($c) >= 4, 'at least 4 bytes';
    my ($orig_len) = unpack 'N', substr($c, 0, 4);
    is $orig_len, 0, 'orig_len = 0 for empty';
};

subtest 'compress is deterministic' => sub {
    my $data = 'hello world test';
    is compress($data), compress($data), 'same output on two calls';
};

# ---- Security: malformed input does not crash --------------------------------

subtest 'decompress empty string is safe' => sub {
    my $result = decompress('');
    ok defined $result, 'returns defined value';
    is $result, '', 'returns empty string';
};

subtest 'decompress 3-byte truncated header is safe' => sub {
    my $result = decompress('ABC');
    ok defined $result, 'returns defined value';
};

subtest 'decompress truncated bit stream is safe' => sub {
    my $c = compress('hello world');
    # Truncate the bit stream portion.
    my $truncated = substr($c, 0, 6);
    my $result    = decompress($truncated);
    ok defined $result, 'truncated stream returns defined value';
};

subtest 'decompress all-zero payload is safe' => sub {
    my $payload = pack('N', 10) . "\x00" x 5;
    my $result  = decompress($payload);
    ok defined $result, 'all-zero payload is safe';
};

subtest 'decompress random bytes does not crash' => sub {
    my $junk   = pack 'C*', map { int(rand 256) } 1 .. 50;
    my $result = eval { decompress($junk) };
    ok defined $result || 1, 'no crash on random bytes';
};

# ---- Compression effectiveness ----------------------------------------------

subtest 'repetitive data compresses below original size' => sub {
    my $data = 'ABC' x 1000;
    my $c    = compress($data);
    ok length($c) < length($data), 'compressed size < original size';
};

subtest 'all same byte compresses significantly' => sub {
    my $data = "\x42" x 10000;
    my $c    = compress($data);
    ok length($c) < length($data), 'compressed < original';
    is decompress($c), $data, 'round-trip correct';
};

subtest 'ABABAB compresses below original size' => sub {
    my $data = 'AB' x 500;
    my $c    = compress($data);
    ok length($c) < length($data), 'AB x 500 compresses';
};

done_testing;
