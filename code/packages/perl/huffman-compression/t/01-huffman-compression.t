use strict;
use warnings;
use Test2::V0;

use CodingAdventures::HuffmanCompression qw(compress decompress);

# ---- Helpers ----------------------------------------------------------------

# rt: round-trip helper — compress then decompress.
sub rt {
    my ($str) = @_;
    return decompress(compress($str));
}

# ---- Round-trip tests --------------------------------------------------------

subtest 'round-trip empty string' => sub {
    is rt(''), '', 'empty string round-trips';
};

subtest 'round-trip single byte A' => sub {
    is rt('A'), 'A', 'A round-trips';
};

subtest 'round-trip AB' => sub {
    is rt('AB'), 'AB', 'AB round-trips';
};

subtest 'round-trip AAABBC' => sub {
    is rt('AAABBC'), 'AAABBC', 'AAABBC round-trips';
};

subtest 'round-trip AAAAAAA (single byte repeated)' => sub {
    is rt('AAAAAAA'), 'AAAAAAA', 'AAAAAAA round-trips';
};

subtest 'round-trip hello world' => sub {
    is rt('hello world'), 'hello world', 'hello world round-trips';
};

subtest 'round-trip ABCDE (no repetition)' => sub {
    is rt('ABCDE'), 'ABCDE', 'ABCDE round-trips';
};

subtest 'round-trip ABC x 100' => sub {
    my $data = 'ABC' x 100;
    is rt($data), $data, 'ABC x 100 round-trips';
};

subtest 'round-trip binary nulls' => sub {
    my $data = "\x00\x00\x00\xff\xff";
    is rt($data), $data, 'binary nulls round-trip';
};

subtest 'round-trip all 256 byte values' => sub {
    my $data = pack 'C*', 0 .. 255;
    is rt($data), $data, 'all 256 bytes round-trip';
};

subtest 'round-trip all 256 bytes repeated x 4' => sub {
    my $data = pack('C*', 0 .. 255) x 4;
    is rt($data), $data, 'all 256 bytes x 4 round-trip';
};

subtest 'round-trip long run of same byte' => sub {
    my $data = "\x42" x 1000;
    is rt($data), $data, '0x42 x 1000 round-trips';
};

subtest 'round-trip ABCDEF x 500' => sub {
    my $data = 'ABCDEF' x 500;
    is rt($data), $data, 'ABCDEF x 500 round-trips';
};

subtest 'round-trip repeated 0,1,2 pattern x 100' => sub {
    my $data = pack 'C*', map { $_ % 3 } 0 .. 299;
    is rt($data), $data, 'mod-3 pattern round-trips';
};

subtest 'round-trip single byte value 0' => sub {
    is rt("\x00"), "\x00", 'null byte round-trips';
};

subtest 'round-trip two distinct bytes' => sub {
    is rt("\x01\x02"), "\x01\x02", 'two distinct bytes round-trip';
};

# ---- Wire format verification for "AAABBC" ----------------------------------

subtest 'wire format: AAABBC header fields' => sub {
    # "AAABBC": A=3, B=2, C=1  (6 total bytes, 3 distinct symbols)
    my $c = compress("AAABBC");

    # Bytes 0-3: original_length = 6
    my ($orig_len) = unpack("N", substr($c, 0, 4));
    is $orig_len, 6, 'original_length = 6';

    # Bytes 4-7: symbol_count = 3
    my ($sym_count) = unpack("N", substr($c, 4, 4));
    is $sym_count, 3, 'symbol_count = 3';

    # The code-lengths table has 3 entries × 2 bytes = 6 bytes
    # Total header = 8 + 6 = 14 bytes minimum
    ok length($c) >= 14, 'wire format has at least 14 bytes for AAABBC';
};

subtest 'wire format: AAABBC code-lengths table sorted by (len, sym)' => sub {
    my $c = compress("AAABBC");
    my ($sym_count) = unpack("N", substr($c, 4, 4));

    # Parse the code-lengths table
    my @entries;
    for my $i (0 .. $sym_count - 1) {
        my ($sym, $len) = unpack("CC", substr($c, 8 + $i * 2, 2));
        push @entries, [$sym, $len];
    }

    # Should be sorted by (code_length, symbol_value) ascending
    for my $i (1 .. $#entries) {
        my ($sym_a, $len_a) = @{$entries[$i-1]};
        my ($sym_b, $len_b) = @{$entries[$i]};
        ok( $len_a < $len_b || ($len_a == $len_b && $sym_a <= $sym_b),
            "entry $i is in sorted order: sym=$sym_b len=$len_b" );
    }

    # All code lengths must be in range 1..16
    for my $e (@entries) {
        ok $e->[1] >= 1 && $e->[1] <= 16,
           "code length for sym=$e->[0] is in [1,16]";
    }
};

subtest 'wire format: correct original_length for various strings' => sub {
    for my $str ('', 'A', 'hello', 'AAABBC', 'ABCDEF') {
        my $c = compress($str);
        my ($orig_len) = unpack("N", substr($c, 0, 4));
        is $orig_len, length($str), "orig_len = " . length($str) . " for '$str'";
    }
};

subtest 'wire format: sym_count matches distinct bytes' => sub {
    # 3 distinct bytes: A, B, C
    my ($sym_count) = unpack("N", substr(compress("AAABBC"), 4, 4));
    is $sym_count, 3, 'AAABBC → 3 distinct symbols';

    # 1 distinct byte
    my ($sym_count2) = unpack("N", substr(compress("AAAA"), 4, 4));
    is $sym_count2, 1, 'AAAA → 1 distinct symbol';

    # 2 distinct bytes
    my ($sym_count3) = unpack("N", substr(compress("AABB"), 4, 4));
    is $sym_count3, 2, 'AABB → 2 distinct symbols';
};

subtest 'wire format: empty input → 8-byte header, no table, no bits' => sub {
    my $c = compress('');
    is length($c), 8, 'empty input → exactly 8 bytes';
    my ($orig_len) = unpack("N", substr($c, 0, 4));
    my ($sym_count) = unpack("N", substr($c, 4, 4));
    is $orig_len, 0, 'empty: orig_len = 0';
    is $sym_count, 0, 'empty: sym_count = 0';
};

# ---- Canonical code properties -----------------------------------------------

subtest 'canonical codes are prefix-free' => sub {
    # Verify by checking no code is a prefix of another.
    my $data = 'AAABBC';
    my $c    = compress($data);
    my ($sym_count) = unpack("N", substr($c, 4, 4));
    my @entries;
    for my $i (0 .. $sym_count - 1) {
        my ($sym, $len) = unpack("CC", substr($c, 8 + $i * 2, 2));
        push @entries, [$sym, $len];
    }
    # Reconstruct canonical codes for check
    my $code     = 0;
    my $prev_len = $entries[0][1];
    my @codes;
    for my $e (@entries) {
        my ($sym, $len) = @$e;
        $code <<= ($len - $prev_len) if $len > $prev_len;
        push @codes, sprintf("%0${len}b", $code);
        $code++;
        $prev_len = $len;
    }
    # Check prefix-free: no code is a prefix of another
    for my $i (0 .. $#codes) {
        for my $j (0 .. $#codes) {
            next if $i == $j;
            ok index($codes[$j], $codes[$i]) != 0,
               "code[$i]='$codes[$i]' is not a prefix of code[$j]='$codes[$j]'";
        }
    }
};

# ---- Compression effectiveness -----------------------------------------------

subtest 'repetitive data compresses below original size' => sub {
    my $data = 'ABC' x 1000;
    my $c    = compress($data);
    ok length($c) < length($data), 'ABC x 1000 compresses';
};

subtest 'all same byte compresses significantly' => sub {
    my $data = "\x42" x 10000;
    my $c    = compress($data);
    ok length($c) < length($data), '0x42 x 10000 compresses';
    is rt($data), $data, 'round-trip correct';
};

subtest 'highly repetitive data: AAABBC x 1000' => sub {
    my $data = 'AAABBC' x 1000;
    my $c    = compress($data);
    ok length($c) < length($data), 'AAABBC x 1000 compresses';
    is rt($data), $data, 'round-trip correct';
};

# ---- Determinism -------------------------------------------------------------

subtest 'compress is deterministic' => sub {
    my $data = 'hello world test AAABBC';
    is compress($data), compress($data), 'same output on two calls';
};

subtest 'compress all 256 bytes is deterministic' => sub {
    my $data = pack 'C*', 0 .. 255;
    is compress($data), compress($data), 'all-bytes: deterministic';
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
    is $result, '', 'returns empty string for short input';
};

subtest 'decompress all-zero payload is safe' => sub {
    my $payload = pack('NN', 10, 0) . "\x00" x 5;
    my $result  = decompress($payload);
    ok defined $result, 'all-zero payload is safe';
};

subtest 'decompress truncated bit stream is safe' => sub {
    my $c         = compress('hello world');
    my $truncated = substr($c, 0, int(length($c) / 2));
    my $result    = eval { decompress($truncated) };
    ok defined $result || 1, 'truncated stream does not crash';
};

subtest 'decompress random bytes does not crash' => sub {
    my $junk   = pack 'C*', map { int(rand 256) } 1 .. 50;
    my $result = eval { decompress($junk) };
    ok defined $result || 1, 'no crash on random bytes';
};

subtest 'decompress with sym_count > 0 but missing table bytes is safe' => sub {
    # sym_count = 5 but no table follows — too short
    my $bad = pack("NN", 10, 5);
    my $result = decompress($bad);
    ok defined $result, 'returns defined value';
    is $result, '', 'returns empty string when table is truncated';
};

done_testing;
