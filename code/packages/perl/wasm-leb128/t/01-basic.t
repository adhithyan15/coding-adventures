use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmLeb128;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::WasmLeb128->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# encode_unsigned tests
# ---------------------------------------------------------------------------

# Test 1: encode 0 -> [0x00]
is(
    [CodingAdventures::WasmLeb128::encode_unsigned(0)],
    [0x00],
    'encode_unsigned(0) = [0x00]'
);

# Test 2: encode 1 -> [0x01]
is(
    [CodingAdventures::WasmLeb128::encode_unsigned(1)],
    [0x01],
    'encode_unsigned(1) = [0x01]'
);

# Test 3: encode 127 -> [0x7F]  (maximum single-byte value)
is(
    [CodingAdventures::WasmLeb128::encode_unsigned(127)],
    [0x7F],
    'encode_unsigned(127) = [0x7F]'
);

# Test 4: encode 128 -> [0x80, 0x01]  (first two-byte value)
is(
    [CodingAdventures::WasmLeb128::encode_unsigned(128)],
    [0x80, 0x01],
    'encode_unsigned(128) = [0x80, 0x01]'
);

# Test 5: encode 300 -> [0xAC, 0x02]
is(
    [CodingAdventures::WasmLeb128::encode_unsigned(300)],
    [0xAC, 0x02],
    'encode_unsigned(300) = [0xAC, 0x02]'
);

# Test 6: encode 624485 -> [0xE5, 0x8E, 0x26]  (WebAssembly spec example)
is(
    [CodingAdventures::WasmLeb128::encode_unsigned(624485)],
    [0xE5, 0x8E, 0x26],
    'encode_unsigned(624485) = [0xE5, 0x8E, 0x26]'
);

# ---------------------------------------------------------------------------
# decode_unsigned tests
# ---------------------------------------------------------------------------

# Test 7: decode [0x00] -> 0
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_unsigned([0x00]);
    is($val,   0, 'decode_unsigned([0x00]) value');
    is($count, 1, 'decode_unsigned([0x00]) count');
}

# Test 8: decode [0x7F] -> 127
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_unsigned([0x7F]);
    is($val,   127, 'decode_unsigned([0x7F]) value');
    is($count, 1,   'decode_unsigned([0x7F]) count');
}

# Test 9: decode [0x80, 0x01] -> 128
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_unsigned([0x80, 0x01]);
    is($val,   128, 'decode_unsigned([0x80,0x01]) value');
    is($count, 2,   'decode_unsigned([0x80,0x01]) count');
}

# Test 10: decode [0xE5, 0x8E, 0x26] -> 624485
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_unsigned([0xE5, 0x8E, 0x26]);
    is($val,   624485, 'decode_unsigned(624485 bytes) value');
    is($count, 3,      'decode_unsigned(624485 bytes) count');
}

# Test 11: decode with offset
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_unsigned(
        [0x00, 0xE5, 0x8E, 0x26], 1
    );
    is($val,   624485, 'decode_unsigned with offset=1 value');
    is($count, 3,      'decode_unsigned with offset=1 count');
}

# ---------------------------------------------------------------------------
# encode_signed tests
# ---------------------------------------------------------------------------

# Test 12: encode_signed(0) -> [0x00]
is(
    [CodingAdventures::WasmLeb128::encode_signed(0)],
    [0x00],
    'encode_signed(0) = [0x00]'
);

# Test 13: encode_signed(-1) -> [0x7F]
is(
    [CodingAdventures::WasmLeb128::encode_signed(-1)],
    [0x7F],
    'encode_signed(-1) = [0x7F]'
);

# Test 14: encode_signed(-2) -> [0x7E]
is(
    [CodingAdventures::WasmLeb128::encode_signed(-2)],
    [0x7E],
    'encode_signed(-2) = [0x7E]'
);

# Test 15: encode_signed(63) -> [0x3F]  (positive, fits in one byte, sign bit 0)
is(
    [CodingAdventures::WasmLeb128::encode_signed(63)],
    [0x3F],
    'encode_signed(63) = [0x3F]'
);

# Test 16: encode_signed(64) -> [0xC0, 0x00]  (needs two bytes; 64 has sign bit set in 7-bit)
is(
    [CodingAdventures::WasmLeb128::encode_signed(64)],
    [0xC0, 0x00],
    'encode_signed(64) = [0xC0, 0x00]'
);

# Test 17: encode_signed(-128) -> [0x80, 0x7F]
is(
    [CodingAdventures::WasmLeb128::encode_signed(-128)],
    [0x80, 0x7F],
    'encode_signed(-128) = [0x80, 0x7F]'
);

# ---------------------------------------------------------------------------
# decode_signed tests
# ---------------------------------------------------------------------------

# Test 18: decode_signed([0x7E]) -> -2
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_signed([0x7E]);
    is($val,   -2, 'decode_signed([0x7E]) value');
    is($count, 1,  'decode_signed([0x7E]) count');
}

# Test 19: decode_signed([0x7F]) -> -1
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_signed([0x7F]);
    is($val,   -1, 'decode_signed([0x7F]) value = -1');
    is($count, 1,  'decode_signed([0x7F]) count');
}

# Test 20: decode_signed([0x3F]) -> 63
{
    my ($val, $count) = CodingAdventures::WasmLeb128::decode_signed([0x3F]);
    is($val,   63, 'decode_signed([0x3F]) value = 63');
    is($count, 1,  'decode_signed([0x3F]) count');
}

# Test 21: roundtrip unsigned
{
    for my $n (0, 1, 127, 128, 300, 624485, 16383, 16384) {
        my @enc = CodingAdventures::WasmLeb128::encode_unsigned($n);
        my ($dec, $cnt) = CodingAdventures::WasmLeb128::decode_unsigned(\@enc);
        is($dec, $n, "unsigned roundtrip: $n");
    }
}

# Test 22: roundtrip signed
{
    for my $n (-128, -64, -2, -1, 0, 1, 63, 64, 127, 300, -300) {
        my @enc = CodingAdventures::WasmLeb128::encode_signed($n);
        my ($dec, $cnt) = CodingAdventures::WasmLeb128::decode_signed(\@enc);
        is($dec, $n, "signed roundtrip: $n");
    }
}

# Test 23: unterminated sequence dies
{
    ok(
        dies { CodingAdventures::WasmLeb128::decode_unsigned([0x80]) },
        'decode_unsigned dies on unterminated sequence'
    );
    ok(
        dies { CodingAdventures::WasmLeb128::decode_signed([0x80]) },
        'decode_signed dies on unterminated sequence'
    );
}

done_testing;
