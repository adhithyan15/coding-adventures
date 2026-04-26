use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Zstd qw(compress decompress);

# ============================================================================
# TC-1: Empty round-trip
# ============================================================================
# The empty string is a valid ZStd input. The encoder must produce a valid
# frame (magic + FHD + FCS + one empty Raw block) and the decoder must return
# an empty string.

subtest 'TC-1: empty round-trip' => sub {
    my $compressed   = compress("");
    my $decompressed = decompress($compressed);
    is($decompressed, "", "empty string round-trips correctly");

    # Frame must start with the ZStd magic bytes: 0x28 0xB5 0x2F 0xFD
    my @magic_bytes = unpack('C4', $compressed);
    is($magic_bytes[0], 0x28, "magic byte 0");
    is($magic_bytes[1], 0xB5, "magic byte 1");
    is($magic_bytes[2], 0x2F, "magic byte 2");
    is($magic_bytes[3], 0xFD, "magic byte 3");
};

# ============================================================================
# TC-2: Single byte round-trip
# ============================================================================
# The smallest non-empty input: a single byte 0x42.

subtest 'TC-2: single byte' => sub {
    my $compressed   = compress("\x42");
    my $decompressed = decompress($compressed);
    is($decompressed, "\x42", "single byte 0x42 round-trips");
};

# ============================================================================
# TC-3: All 256 byte values
# ============================================================================
# Every possible byte value 0x00..0xFF in ascending order. Tests literal
# encoding of zero bytes, non-ASCII bytes, and 0xFF.

subtest 'TC-3: all 256 byte values' => sub {
    my $input        = join('', map { chr($_) } 0 .. 255);
    my $compressed   = compress($input);
    my $decompressed = decompress($compressed);
    is(length($decompressed), 256, "decompressed length matches");
    is($decompressed, $input, "all 256 byte values round-trip");
};

# ============================================================================
# TC-4: RLE — 1024 identical bytes
# ============================================================================
# 1024 copies of 'A' must be detected as an RLE block. The compressed frame
# should be tiny: magic(4) + FHD(1) + FCS(8) + block_header(3) + byte(1) = 17.

subtest 'TC-4: RLE compression' => sub {
    my $input        = "A" x 1024;
    my $compressed   = compress($input);
    my $decompressed = decompress($compressed);
    is($decompressed, $input, "RLE 1024 x 'A' round-trips");
    ok(length($compressed) < 30,
       "RLE compresses to < 30 bytes (got " . length($compressed) . ")");
};

# ============================================================================
# TC-5: English prose — compression ratio check
# ============================================================================
# Repeated English text has strong LZ77 back-references. The compressed output
# must be less than 80% of the original size.

subtest 'TC-5: prose compression ratio' => sub {
    my $text         = "the quick brown fox jumps over the lazy dog " x 25;
    my $input        = $text;
    my $compressed   = compress($input);
    my $decompressed = decompress($compressed);
    is($decompressed, $input, "prose round-trips correctly");
    my $threshold    = int(length($input) * 80 / 100);
    ok(length($compressed) < $threshold,
       "prose compresses to < 80% (got " . length($compressed) . " vs threshold $threshold)");
};

# ============================================================================
# TC-6: Pseudo-random data (LCG seed=42)
# ============================================================================
# LCG random bytes have no repetition structure — no significant compression
# is expected. But the round-trip must be exact regardless of which block type
# (Raw, Compressed) the encoder chooses.

subtest 'TC-6: LCG random data' => sub {
    my $seed  = 42;
    my @bytes;
    for my $i (1 .. 512) {
        $seed = (($seed * 1664525) + 1013904223) & 0xFFFFFFFF;
        push @bytes, $seed & 0xFF;
    }
    my $input        = pack('C*', @bytes);
    my $compressed   = compress($input);
    my $decompressed = decompress($compressed);
    is($decompressed, $input, "pseudo-random 512 bytes round-trips");
};

# ============================================================================
# TC-7: 200 KB single-byte run — multi-block round-trip
# ============================================================================
# 200 KB > MAX_BLOCK_SIZE (128 KB), so the encoder must split this across at
# least two blocks. Both should be RLE blocks (all bytes 0xAB).

subtest 'TC-7: 200 KB single-byte run' => sub {
    my $input        = "\xAB" x (200 * 1024);
    my $compressed   = compress($input);
    my $decompressed = decompress($compressed);
    is(length($decompressed), 200 * 1024, "decompressed length = 200 KB");
    is($decompressed, $input, "200 KB single-byte run round-trips");
};

# ============================================================================
# TC-8: 300 KB repetitive text
# ============================================================================
# Large highly-compressible text exercises multi-block compressed output.

subtest 'TC-8: 300 KB repetitive text' => sub {
    my $unit  = "Hello, ZStd! This is a repetitive test pattern. " x 100;
    # ~4800 bytes per repetition × 63 ≈ 300 KB
    my $input = $unit x 63;
    my $compressed   = compress($input);
    my $decompressed = decompress($compressed);
    is(length($decompressed), length($input), "decompressed length matches");
    is($decompressed, $input, "300 KB repetitive text round-trips");
    my $ratio = length($compressed) / length($input);
    ok($ratio < 0.80, "300 KB repetitive text compresses below 80% (ratio=$ratio)");
};

# ============================================================================
# TC-9: Bad magic → exception
# ============================================================================
# A frame with the wrong magic number must die (not silently produce garbage).

subtest 'TC-9: bad magic dies' => sub {
    # Replace magic with 0xDEADBEEF (LE bytes: DE AD BE EF)
    my $bad = "\xDE\xAD\xBE\xEF" . "\x00" x 20;
    my $result = eval { decompress($bad); 1 };
    ok(!$result, "decompress of bad-magic frame throws an exception");
    like($@, qr/magic/i, "error message mentions magic");
};

# ============================================================================
# Additional round-trip tests
# ============================================================================

subtest 'RT-1: hello world' => sub {
    my $input = "hello world";
    is(decompress(compress($input)), $input, "hello world round-trips");
};

subtest 'RT-2: binary data with zeros and 0xFF' => sub {
    my @bytes = map { $_ % 256 } (0 .. 299);
    my $input = pack('C*', @bytes);
    is(decompress(compress($input)), $input, "binary data round-trips");
};

subtest 'RT-3: all zeros' => sub {
    my $input = "\x00" x 1000;
    is(decompress(compress($input)), $input, "1000 zero bytes round-trip");
};

subtest 'RT-4: all 0xFF' => sub {
    my $input = "\xFF" x 1000;
    is(decompress(compress($input)), $input, "1000 0xFF bytes round-trip");
};

subtest 'RT-5: repeated pattern' => sub {
    my @data  = map { $_ % 6 } (0 .. 2999);
    my $input = pack('C*', @data);
    is(decompress(compress($input)), $input, "3000-byte cyclic pattern round-trips");
};

subtest 'RT-6: single char various values' => sub {
    for my $byte (0, 1, 127, 128, 255) {
        my $input = chr($byte);
        is(decompress(compress($input)), $input, "single byte 0x" . sprintf('%02X', $byte) . " round-trips");
    }
};

subtest 'RT-7: compress called as method' => sub {
    my $input      = "method call test " x 10;
    my $compressed = CodingAdventures::Zstd->compress($input);
    is(decompress($compressed), $input, "compress as class method round-trips");
};

subtest 'RT-8: two-byte input' => sub {
    my $input = "\x01\x02";
    is(decompress(compress($input)), $input, "two-byte input round-trips");
};

subtest 'RT-9: alternating pattern' => sub {
    my $input = ("AB" x 2000);
    is(decompress(compress($input)), $input, "alternating AB 4000 bytes round-trips");
};

subtest 'RT-10: exact MAX_BLOCK_SIZE boundary' => sub {
    # 128 KB exactly — should be handled as exactly one block.
    my $input = "X" x (128 * 1024);
    is(decompress(compress($input)), $input, "128 KB (one full block) round-trips");
};

# ============================================================================
# Unit tests for internal helpers
# ============================================================================

subtest 'UNIT-1: _ll_to_code identity 0..15' => sub {
    for my $i (0 .. 15) {
        my $code = CodingAdventures::Zstd::_ll_to_code($i);
        is($code, $i, "LL code for literal length $i");
    }
};

subtest 'UNIT-2: _ml_to_code identity 3..34' => sub {
    for my $i (3 .. 34) {
        my $code = CodingAdventures::Zstd::_ml_to_code($i);
        is($code, $i - 3, "ML code for match length $i");
    }
};

subtest 'UNIT-3: FSE decode table coverage' => sub {
    my @ll_norm = (4,3,2,2,2,2,2,2,2,2,2,2,2,1,1,1,2,2,2,2,2,2,2,2,2,3,2,1,1,1,1,1,-1,-1,-1,-1);
    my $ll_acc_log = 6;
    my $tbl = CodingAdventures::Zstd::_build_decode_table(\@ll_norm, $ll_acc_log);
    is(scalar @$tbl, 1 << $ll_acc_log, "decode table has correct size");
    for my $cell (@$tbl) {
        ok($cell->{sym} < 36, "symbol in range (got $cell->{sym})");
    }
};

subtest 'UNIT-4: RevBitWriter / RevBitReader round-trip' => sub {
    # Write 3 values, read them back in reverse order.
    # Write A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
    # Decoder reads: C first, then B, then A.
    my $bw = RevBitWriter->new();
    $bw->add_bits(0b101,      3);   # A — written first, read last
    $bw->add_bits(0b11001100, 8);   # B
    $bw->add_bits(0b1,        1);   # C — written last, read first
    $bw->flush();
    my $buf = $bw->finish();

    my $br = RevBitReader->new($buf);
    is($br->read_bits(1), 0b1,        "C: last written, first read");
    is($br->read_bits(8), 0b11001100, "B");
    is($br->read_bits(3), 0b101,      "A: first written, last read");
};

subtest 'UNIT-5: _encode_seq_count / _decode_seq_count round-trip' => sub {
    for my $n (0, 1, 50, 127, 128, 255, 515, 1000, 0x7EFF) {
        my $enc = CodingAdventures::Zstd::_encode_seq_count($n);
        my ($dec, $consumed) = CodingAdventures::Zstd::_decode_seq_count($enc, 0);
        is($dec, $n, "seq count $n round-trips");
        ok($consumed > 0 && $consumed <= 3, "consumed $consumed bytes");
    }
};

subtest 'UNIT-6: literals section round-trip (short, medium, large)' => sub {
    for my $size (0, 20, 200, 5000) {
        my @lits = map { $_ % 256 } (0 .. $size - 1);
        my $enc  = CodingAdventures::Zstd::_encode_literals_section(\@lits);
        my ($dec, $consumed) = CodingAdventures::Zstd::_decode_literals_section($enc, 0);
        is(scalar @$dec, $size, "literals section size=$size: decoded count matches");
        is(join(',', @$dec), join(',', @lits), "literals section size=$size: data matches");
    }
};

done_testing;
