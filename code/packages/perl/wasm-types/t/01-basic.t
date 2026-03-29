use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmTypes qw(
    is_val_type    is_ref_type    val_type_name
    encode_val_type decode_val_type
    encode_limits   decode_limits
    encode_func_type decode_func_type
);

# ---------------------------------------------------------------------------
# ValType constants
# ---------------------------------------------------------------------------

subtest 'ValType constants' => sub {
    is($CodingAdventures::WasmTypes::ValType{i32},       0x7F, 'i32 = 0x7F');
    is($CodingAdventures::WasmTypes::ValType{i64},       0x7E, 'i64 = 0x7E');
    is($CodingAdventures::WasmTypes::ValType{f32},       0x7D, 'f32 = 0x7D');
    is($CodingAdventures::WasmTypes::ValType{f64},       0x7C, 'f64 = 0x7C');
    is($CodingAdventures::WasmTypes::ValType{v128},      0x7B, 'v128 = 0x7B');
    is($CodingAdventures::WasmTypes::ValType{funcref},   0x70, 'funcref = 0x70');
    is($CodingAdventures::WasmTypes::ValType{externref}, 0x6F, 'externref = 0x6F');
};

# ---------------------------------------------------------------------------
# RefType constants
# ---------------------------------------------------------------------------

subtest 'RefType constants' => sub {
    is($CodingAdventures::WasmTypes::RefType{funcref},   0x70, 'funcref = 0x70');
    is($CodingAdventures::WasmTypes::RefType{externref}, 0x6F, 'externref = 0x6F');
};

# ---------------------------------------------------------------------------
# BlockType constants
# ---------------------------------------------------------------------------

subtest 'BlockType constants' => sub {
    is($CodingAdventures::WasmTypes::BlockType{empty}, 0x40, 'empty = 0x40');
};

# ---------------------------------------------------------------------------
# ExternType constants
# ---------------------------------------------------------------------------

subtest 'ExternType constants' => sub {
    is($CodingAdventures::WasmTypes::ExternType{func},   0, 'func = 0');
    is($CodingAdventures::WasmTypes::ExternType{table},  1, 'table = 1');
    is($CodingAdventures::WasmTypes::ExternType{mem},    2, 'mem = 2');
    is($CodingAdventures::WasmTypes::ExternType{global}, 3, 'global = 3');
};

# ---------------------------------------------------------------------------
# is_val_type
# ---------------------------------------------------------------------------

subtest 'is_val_type' => sub {
    ok(is_val_type(0x7F),  'i32 is val type');
    ok(is_val_type(0x7E),  'i64 is val type');
    ok(is_val_type(0x7D),  'f32 is val type');
    ok(is_val_type(0x7C),  'f64 is val type');
    ok(is_val_type(0x7B),  'v128 is val type');
    ok(is_val_type(0x70),  'funcref is val type');
    ok(is_val_type(0x6F),  'externref is val type');

    ok(!is_val_type(0x00),  '0x00 is not a val type');
    ok(!is_val_type(0x40),  '0x40 (BlockType.empty) is not a val type');
    ok(!is_val_type(0x60),  '0x60 (FuncType magic) is not a val type');
    ok(!is_val_type(0xFF),  '0xFF is not a val type');
    ok(!is_val_type(0x7A),  '0x7A is not a val type');
};

# ---------------------------------------------------------------------------
# is_ref_type
# ---------------------------------------------------------------------------

subtest 'is_ref_type' => sub {
    ok(is_ref_type(0x70),  'funcref is ref type');
    ok(is_ref_type(0x6F),  'externref is ref type');

    ok(!is_ref_type(0x7F),  'i32 is not ref type');
    ok(!is_ref_type(0x7E),  'i64 is not ref type');
    ok(!is_ref_type(0x7D),  'f32 is not ref type');
    ok(!is_ref_type(0x7C),  'f64 is not ref type');
    ok(!is_ref_type(0x7B),  'v128 is not ref type');
    ok(!is_ref_type(0x00),  '0x00 is not ref type');
};

# ---------------------------------------------------------------------------
# val_type_name
# ---------------------------------------------------------------------------

subtest 'val_type_name' => sub {
    is(val_type_name(0x7F), 'i32',       'name of 0x7F is i32');
    is(val_type_name(0x7E), 'i64',       'name of 0x7E is i64');
    is(val_type_name(0x7D), 'f32',       'name of 0x7D is f32');
    is(val_type_name(0x7C), 'f64',       'name of 0x7C is f64');
    is(val_type_name(0x7B), 'v128',      'name of 0x7B is v128');
    is(val_type_name(0x70), 'funcref',   'name of 0x70 is funcref');
    is(val_type_name(0x6F), 'externref', 'name of 0x6F is externref');

    is(val_type_name(0x00), 'unknown_0x00', 'unknown byte returns unknown_0x00');
    is(val_type_name(0x42), 'unknown_0x42', 'unknown byte returns unknown_0x42');
    is(val_type_name(0x60), 'unknown_0x60', 'functype magic returns unknown_0x60');
};

# ---------------------------------------------------------------------------
# encode_val_type
# ---------------------------------------------------------------------------

subtest 'encode_val_type' => sub {
    is([encode_val_type(0x7F)], [0x7F], 'encode i32 = [0x7F]');
    is([encode_val_type(0x7E)], [0x7E], 'encode i64 = [0x7E]');
    is([encode_val_type(0x70)], [0x70], 'encode funcref = [0x70]');
    is([encode_val_type(0x6F)], [0x6F], 'encode externref = [0x6F]');

    is(scalar( encode_val_type(0x7F) ), 1, 'returns exactly 1 byte');

    # Error on invalid type
    ok(dies { encode_val_type(0x42) }, 'dies on invalid val type 0x42');
    ok(dies { encode_val_type(0x00) }, 'dies on invalid val type 0x00');
};

# ---------------------------------------------------------------------------
# decode_val_type
# ---------------------------------------------------------------------------

subtest 'decode_val_type' => sub {
    my ($t, $n) = decode_val_type([0x7F]);
    is($t, 0x7F, 'decode [0x7F] → type = 0x7F');
    is($n, 1,    'decode [0x7F] → bytes_consumed = 1');

    my ($t2, $n2) = decode_val_type([0x7E]);
    is($t2, 0x7E, 'decode [0x7E] → type = 0x7E (i64)');

    # Decode at offset
    my ($t3, $n3) = decode_val_type([0x00, 0x7F], 1);
    is($t3, 0x7F, 'decode at offset 1');
    is($n3, 1,    'bytes_consumed = 1 at offset');

    # Error on invalid val type byte
    ok(dies { decode_val_type([0x42]) }, 'dies on invalid byte 0x42');

    # Error on out-of-range offset
    ok(dies { decode_val_type([0x7F], 5) }, 'dies on out-of-range offset');
};

# ---------------------------------------------------------------------------
# encode_limits
# ---------------------------------------------------------------------------

subtest 'encode_limits' => sub {
    # No max, min=0 → [0x00, 0x00]
    is([encode_limits({min=>0})], [0x00, 0x00], 'no-max min=0 → [0x00, 0x00]');

    # No max, min=1 → [0x00, 0x01]
    is([encode_limits({min=>1, max=>undef})], [0x00, 0x01], 'no-max min=1 → [0x00, 0x01]');

    # Has max, min=1, max=16 → [0x01, 0x01, 0x10]
    is([encode_limits({min=>1, max=>16})], [0x01, 0x01, 0x10],
       'bounded min=1 max=16 → [0x01, 0x01, 0x10]');

    # min=0, max=0 → [0x01, 0x00, 0x00]
    is([encode_limits({min=>0, max=>0})], [0x01, 0x00, 0x00],
       'min=0 max=0 → [0x01, 0x00, 0x00]');

    # Large min: 128 → LEB128 two bytes [0x80, 0x01]
    my @lb = encode_limits({min=>128});
    is($lb[0], 0x00, 'no-max flag for large min');
    is($lb[1], 0x80, 'first LEB128 byte of 128');
    is($lb[2], 0x01, 'second LEB128 byte of 128');
};

# ---------------------------------------------------------------------------
# decode_limits
# ---------------------------------------------------------------------------

subtest 'decode_limits' => sub {
    # No max: [0x00, 0x00] → min=0, max=undef
    my ($lim, $n) = decode_limits([0x00, 0x00]);
    is($lim->{min}, 0,     'no-max decode: min=0');
    ok(!defined $lim->{max}, 'no-max decode: max is undef');
    is($n, 2, 'bytes_consumed=2 for no-max');

    # Bounded: [0x01, 0x01, 0x10] → min=1, max=16
    my ($lim2, $n2) = decode_limits([0x01, 0x01, 0x10]);
    is($lim2->{min}, 1,  'bounded decode: min=1');
    is($lim2->{max}, 16, 'bounded decode: max=16');
    is($n2, 3, 'bytes_consumed=3 for bounded');

    # Round-trip: no max
    my %orig1 = (min => 42);
    my @enc1 = encode_limits(\%orig1);
    my ($dec1, $nc1) = decode_limits(\@enc1);
    is($dec1->{min}, 42, 'round-trip no-max: min=42');
    ok(!defined $dec1->{max}, 'round-trip no-max: max is undef');

    # Round-trip: bounded
    my %orig2 = (min => 10, max => 200);
    my @enc2 = encode_limits(\%orig2);
    my ($dec2, $nc2) = decode_limits(\@enc2);
    is($dec2->{min}, 10,  'round-trip bounded: min=10');
    is($dec2->{max}, 200, 'round-trip bounded: max=200');

    # Error on bad flag
    ok(dies { decode_limits([0x02, 0x00]) }, 'dies on invalid flag byte 0x02');
};

# ---------------------------------------------------------------------------
# encode_func_type
# ---------------------------------------------------------------------------

subtest 'encode_func_type' => sub {
    # () → (): [0x60, 0x00, 0x00]
    is(
        [encode_func_type({params=>[], results=>[]})],
        [0x60, 0x00, 0x00],
        '() → () encodes to [0x60, 0x00, 0x00]'
    );

    # (i32) → (): [0x60, 0x01, 0x7F, 0x00]
    is(
        [encode_func_type({params=>[0x7F], results=>[]})],
        [0x60, 0x01, 0x7F, 0x00],
        '(i32) → () encodes to [0x60, 0x01, 0x7F, 0x00]'
    );

    # (i32, i32) → i64: [0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E]
    is(
        [encode_func_type({params=>[0x7F, 0x7F], results=>[0x7E]})],
        [0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E],
        '(i32, i32) → i64 encodes correctly'
    );

    # First byte is always 0x60
    my @b = encode_func_type({params=>[0x7F], results=>[0x7F]});
    is($b[0], 0x60, 'first byte is 0x60 magic');
};

# ---------------------------------------------------------------------------
# decode_func_type
# ---------------------------------------------------------------------------

subtest 'decode_func_type' => sub {
    # () → ()
    my ($ft, $n) = decode_func_type([0x60, 0x00, 0x00]);
    is(scalar @{$ft->{params}},  0, '() → (): 0 params');
    is(scalar @{$ft->{results}}, 0, '() → (): 0 results');
    is($n, 3, 'bytes_consumed=3 for () → ()');

    # (i32) → ()
    my ($ft2, $n2) = decode_func_type([0x60, 0x01, 0x7F, 0x00]);
    is(scalar @{$ft2->{params}}, 1,    '(i32) → (): 1 param');
    is($ft2->{params}[0],        0x7F, '(i32) → (): param is i32');
    is(scalar @{$ft2->{results}}, 0,   '(i32) → (): 0 results');
    is($n2, 4, 'bytes_consumed=4');

    # (i32, i32) → i64
    my ($ft3, $n3) = decode_func_type([0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E]);
    is(scalar @{$ft3->{params}},  2,    '2 params');
    is($ft3->{params}[0],         0x7F, 'param[0] = i32');
    is($ft3->{params}[1],         0x7F, 'param[1] = i32');
    is(scalar @{$ft3->{results}}, 1,    '1 result');
    is($ft3->{results}[0],        0x7E, 'result[0] = i64');

    # Error: wrong magic byte
    ok(dies { decode_func_type([0x61, 0x00, 0x00]) }, 'dies on wrong magic byte 0x61');

    # Round-trip
    my %orig = (params => [0x7F, 0x7C], results => [0x7E, 0x7D]);
    my @enc  = encode_func_type(\%orig);
    my ($dec, $nc) = decode_func_type(\@enc);
    is(scalar @{$dec->{params}},  2,    'round-trip: 2 params');
    is(scalar @{$dec->{results}}, 2,    'round-trip: 2 results');
    is($dec->{params}[0],  0x7F, 'round-trip param[0]');
    is($dec->{params}[1],  0x7C, 'round-trip param[1]');
    is($dec->{results}[0], 0x7E, 'round-trip result[0]');
    is($dec->{results}[1], 0x7D, 'round-trip result[1]');
};

done_testing;
