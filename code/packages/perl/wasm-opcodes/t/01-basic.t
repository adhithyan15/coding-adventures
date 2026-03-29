use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmOpcodes qw(opcode_name is_valid_opcode get_opcode_info);

# ---------------------------------------------------------------------------
# OPCODES table sanity
# ---------------------------------------------------------------------------

ok(defined %CodingAdventures::WasmOpcodes::OPCODES, 'OPCODES hash is defined');
ok(scalar keys %CodingAdventures::WasmOpcodes::OPCODES >= 40,
   'OPCODES has at least 40 entries');

# ---------------------------------------------------------------------------
# Control flow opcodes
# ---------------------------------------------------------------------------

subtest 'control flow' => sub {
    is(opcode_name(0x00), 'unreachable',    '0x00 = unreachable');
    is(opcode_name(0x01), 'nop',            '0x01 = nop');
    is(opcode_name(0x02), 'block',          '0x02 = block');
    is(opcode_name(0x03), 'loop',           '0x03 = loop');
    is(opcode_name(0x04), 'if',             '0x04 = if');
    is(opcode_name(0x05), 'else',           '0x05 = else');
    is(opcode_name(0x0b), 'end',            '0x0b = end');
    is(opcode_name(0x0c), 'br',             '0x0c = br');
    is(opcode_name(0x0d), 'br_if',          '0x0d = br_if');
    is(opcode_name(0x0e), 'br_table',       '0x0e = br_table');
    is(opcode_name(0x0f), 'return',         '0x0f = return');
    is(opcode_name(0x10), 'call',           '0x10 = call');
    is(opcode_name(0x11), 'call_indirect',  '0x11 = call_indirect');
};

# ---------------------------------------------------------------------------
# Parametric opcodes
# ---------------------------------------------------------------------------

subtest 'parametric' => sub {
    is(opcode_name(0x1a), 'drop',   '0x1a = drop');
    is(opcode_name(0x1b), 'select', '0x1b = select');
};

# ---------------------------------------------------------------------------
# Variable opcodes
# ---------------------------------------------------------------------------

subtest 'variable' => sub {
    is(opcode_name(0x20), 'local.get',  '0x20 = local.get');
    is(opcode_name(0x21), 'local.set',  '0x21 = local.set');
    is(opcode_name(0x22), 'local.tee',  '0x22 = local.tee');
    is(opcode_name(0x23), 'global.get', '0x23 = global.get');
    is(opcode_name(0x24), 'global.set', '0x24 = global.set');
};

# ---------------------------------------------------------------------------
# Memory opcodes
# ---------------------------------------------------------------------------

subtest 'memory' => sub {
    is(opcode_name(0x28), 'i32.load',    '0x28 = i32.load');
    is(opcode_name(0x29), 'i64.load',    '0x29 = i64.load');
    is(opcode_name(0x2a), 'f32.load',    '0x2a = f32.load');
    is(opcode_name(0x2b), 'f64.load',    '0x2b = f64.load');
    is(opcode_name(0x2c), 'i32.load8_s', '0x2c = i32.load8_s');
    is(opcode_name(0x2d), 'i32.load8_u', '0x2d = i32.load8_u');
    is(opcode_name(0x2e), 'i32.load16_s','0x2e = i32.load16_s');
    is(opcode_name(0x2f), 'i32.load16_u','0x2f = i32.load16_u');
    is(opcode_name(0x30), 'i64.load8_s', '0x30 = i64.load8_s');
    is(opcode_name(0x31), 'i64.load8_u', '0x31 = i64.load8_u');
    is(opcode_name(0x36), 'i32.store',   '0x36 = i32.store');
    is(opcode_name(0x3a), 'i32.store8',  '0x3a = i32.store8');
    is(opcode_name(0x3b), 'i32.store16', '0x3b = i32.store16');
    is(opcode_name(0x3f), 'memory.size', '0x3f = memory.size');
    is(opcode_name(0x40), 'memory.grow', '0x40 = memory.grow');
};

# ---------------------------------------------------------------------------
# i32 numeric opcodes
# ---------------------------------------------------------------------------

subtest 'i32 numeric' => sub {
    is(opcode_name(0x41), 'i32.const',  '0x41 = i32.const');
    is(opcode_name(0x45), 'i32.eqz',   '0x45 = i32.eqz');
    is(opcode_name(0x46), 'i32.eq',    '0x46 = i32.eq');
    is(opcode_name(0x47), 'i32.ne',    '0x47 = i32.ne');
    is(opcode_name(0x48), 'i32.lt_s',  '0x48 = i32.lt_s');
    is(opcode_name(0x6a), 'i32.add',   '0x6a = i32.add');
    is(opcode_name(0x6b), 'i32.sub',   '0x6b = i32.sub');
    is(opcode_name(0x6c), 'i32.mul',   '0x6c = i32.mul');
    is(opcode_name(0x6d), 'i32.div_s', '0x6d = i32.div_s');
    is(opcode_name(0x71), 'i32.and',   '0x71 = i32.and');
    is(opcode_name(0x72), 'i32.or',    '0x72 = i32.or');
    is(opcode_name(0x73), 'i32.xor',   '0x73 = i32.xor');
    is(opcode_name(0x74), 'i32.shl',   '0x74 = i32.shl');
    is(opcode_name(0x75), 'i32.shr_s', '0x75 = i32.shr_s');
};

# ---------------------------------------------------------------------------
# i64 numeric opcodes
# ---------------------------------------------------------------------------

subtest 'i64 numeric' => sub {
    is(opcode_name(0x42), 'i64.const', '0x42 = i64.const');
    is(opcode_name(0x7c), 'i64.add',   '0x7c = i64.add');
    is(opcode_name(0x7d), 'i64.sub',   '0x7d = i64.sub');
    is(opcode_name(0x7e), 'i64.mul',   '0x7e = i64.mul');
};

# ---------------------------------------------------------------------------
# f32 numeric opcodes
# ---------------------------------------------------------------------------

subtest 'f32 numeric' => sub {
    is(opcode_name(0x43), 'f32.const', '0x43 = f32.const');
    is(opcode_name(0x92), 'f32.add',   '0x92 = f32.add');
    is(opcode_name(0x93), 'f32.sub',   '0x93 = f32.sub');
    is(opcode_name(0x94), 'f32.mul',   '0x94 = f32.mul');
};

# ---------------------------------------------------------------------------
# f64 numeric opcodes
# ---------------------------------------------------------------------------

subtest 'f64 numeric' => sub {
    is(opcode_name(0x44), 'f64.const', '0x44 = f64.const');
    is(opcode_name(0xa0), 'f64.add',   '0xa0 = f64.add');
    is(opcode_name(0xa1), 'f64.sub',   '0xa1 = f64.sub');
    is(opcode_name(0xa2), 'f64.mul',   '0xa2 = f64.mul');
};

# ---------------------------------------------------------------------------
# Conversion opcodes
# ---------------------------------------------------------------------------

subtest 'conversions' => sub {
    is(opcode_name(0xa7), 'i32.wrap_i64',     '0xa7 = i32.wrap_i64');
    is(opcode_name(0xa8), 'i32.trunc_f32_s',  '0xa8 = i32.trunc_f32_s');
    is(opcode_name(0xac), 'i64.extend_i32_s', '0xac = i64.extend_i32_s');
    is(opcode_name(0xb6), 'f32.demote_f64',   '0xb6 = f32.demote_f64');
    is(opcode_name(0xbb), 'f64.promote_f32',  '0xbb = f64.promote_f32');
};

# ---------------------------------------------------------------------------
# opcode_name — unknown bytes
# ---------------------------------------------------------------------------

subtest 'unknown bytes' => sub {
    is(opcode_name(0x99), 'unknown_0x99', 'unknown 0x99');
    is(opcode_name(0xff), 'unknown_0xff', 'unknown 0xff');
    is(opcode_name(0x50), 'unknown_0x50', 'unknown 0x50');
};

# ---------------------------------------------------------------------------
# is_valid_opcode
# ---------------------------------------------------------------------------

subtest 'is_valid_opcode' => sub {
    # All entries in OPCODES should be valid
    for my $byte ( keys %CodingAdventures::WasmOpcodes::OPCODES ) {
        ok(is_valid_opcode($byte), sprintf("is_valid_opcode(0x%02x) = true", $byte));
    }

    # Specific known opcodes
    ok(is_valid_opcode(0x00),  'unreachable is valid');
    ok(is_valid_opcode(0x6a),  'i32.add is valid');
    ok(is_valid_opcode(0x10),  'call is valid');

    # Invalid bytes
    ok(!is_valid_opcode(0x99), '0x99 is not valid');
    ok(!is_valid_opcode(0xff), '0xff is not valid');
    ok(!is_valid_opcode(0x50), '0x50 is not valid');
    ok(!is_valid_opcode(0x15), '0x15 is not valid');
};

# ---------------------------------------------------------------------------
# get_opcode_info
# ---------------------------------------------------------------------------

subtest 'get_opcode_info' => sub {
    # Returns hashref with name and operands
    my $info = get_opcode_info(0x6a);
    ok(defined $info,                       'get_opcode_info(0x6a) defined');
    is(ref $info, 'HASH',                   'returns a hashref');
    is($info->{name},     'i32.add',        'name = i32.add');
    is($info->{operands}, 'none',           'operands = none');

    # call has non-trivial operands
    my $call_info = get_opcode_info(0x10);
    like($call_info->{operands}, qr/func_idx/, 'call operands mention func_idx');

    # load has memarg operands
    my $load_info = get_opcode_info(0x28);
    like($load_info->{operands}, qr/memarg/, 'i32.load operands mention memarg');

    # Unknown byte returns undef
    ok(!defined get_opcode_info(0x99), 'get_opcode_info(0x99) = undef');
    ok(!defined get_opcode_info(0xff), 'get_opcode_info(0xff) = undef');

    # All entries have non-empty name and operands
    for my $byte ( keys %CodingAdventures::WasmOpcodes::OPCODES ) {
        my $entry = $CodingAdventures::WasmOpcodes::OPCODES{$byte};
        ok(length($entry->{name}) > 0,
           sprintf("OPCODES[0x%02x].name is non-empty", $byte));
        ok(defined $entry->{operands},
           sprintf("OPCODES[0x%02x].operands is defined", $byte));
    }
};

done_testing;
