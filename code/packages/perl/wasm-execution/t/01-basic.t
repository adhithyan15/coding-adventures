use strict;
use warnings;
use Test2::V0;

use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';

use CodingAdventures::WasmExecution qw(i32 i64 f32 f64 default_value evaluate_const_expr);
use CodingAdventures::VirtualMachine;

# ============================================================================
# WasmValue Constructors
# ============================================================================

subtest 'i32 constructor' => sub {
    my $v = i32(42);
    is($v->{type}, 0x7F, 'i32 type code is 0x7F');
    is($v->{value}, 42, 'i32 value is correct');

    # Wrapping: large unsigned values become signed
    my $wrap = i32(0xFFFFFFFF);
    is($wrap->{value}, -1, 'i32 wraps 0xFFFFFFFF to -1');

    my $wrap2 = i32(0x80000000);
    is($wrap2->{value}, -2147483648, 'i32 wraps 0x80000000 to min i32');
};

subtest 'i64 constructor' => sub {
    my $v = i64(100);
    is($v->{type}, 0x7E, 'i64 type code is 0x7E');
    is($v->{value}, 100, 'i64 value is correct');
};

subtest 'f32 constructor' => sub {
    my $v = f32(3.14);
    is($v->{type}, 0x7D, 'f32 type code is 0x7D');
    # f32 rounds to single precision
    ok(abs($v->{value} - 3.14) < 0.001, 'f32 value is approximately correct');
};

subtest 'f64 constructor' => sub {
    my $v = f64(3.14159265358979);
    is($v->{type}, 0x7C, 'f64 type code is 0x7C');
    ok(abs($v->{value} - 3.14159265358979) < 1e-10, 'f64 value is precise');
};

subtest 'default_value' => sub {
    my $i32_def = default_value(0x7F);
    is($i32_def->{value}, 0, 'default i32 is 0');
    is($i32_def->{type}, 0x7F, 'default i32 type is correct');

    my $i64_def = default_value(0x7E);
    is($i64_def->{value}, 0, 'default i64 is 0');

    my $f32_def = default_value(0x7D);
    is($f32_def->{value}, 0, 'default f32 is 0');

    my $f64_def = default_value(0x7C);
    is($f64_def->{value}, 0, 'default f64 is 0');

    # Unknown type should trap
    my $died = 0;
    eval { default_value(0x00) };
    $died = 1 if $@;
    ok($died, 'default_value traps on unknown type');
};

# ============================================================================
# LinearMemory
# ============================================================================

subtest 'LinearMemory basics' => sub {
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    is($mem->size(), 1, 'initial size is 1 page');
    is($mem->byte_length(), 65536, 'byte length is 64 KiB');

    # Store and load i32
    $mem->store_i32(0, 42);
    is($mem->load_i32(0), 42, 'i32 store/load roundtrip');

    # Store and load i32 at offset
    $mem->store_i32(100, -1);
    is($mem->load_i32(100), -1, 'i32 store/load negative');

    # Store/load f64
    $mem->store_f64(200, 3.14);
    ok(abs($mem->load_f64(200) - 3.14) < 1e-10, 'f64 store/load roundtrip');
};

subtest 'LinearMemory bounds checking' => sub {
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    my $died = 0;
    eval { $mem->load_i32(65536) };
    $died = 1 if $@;
    ok($died, 'out-of-bounds load traps');
};

subtest 'LinearMemory grow' => sub {
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, 3);
    is($mem->grow(1), 1, 'grow returns old page count');
    is($mem->size(), 2, 'size increased to 2');

    is($mem->grow(1), 2, 'grow returns 2');
    is($mem->size(), 3, 'size increased to 3');

    # Exceeds max
    is($mem->grow(1), -1, 'grow beyond max returns -1');
    is($mem->size(), 3, 'size unchanged after failed grow');
};

subtest 'LinearMemory narrow loads' => sub {
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    $mem->store_i32_8(0, 0xFF);
    is($mem->load_i32_8s(0), -1, 'i32.load8_s: 0xFF = -1 signed');
    is($mem->load_i32_8u(0), 255, 'i32.load8_u: 0xFF = 255 unsigned');
};

# ============================================================================
# Table
# ============================================================================

subtest 'Table basics' => sub {
    my $table = CodingAdventures::WasmExecution::Table->new(10, 20);
    is($table->size(), 10, 'initial size');

    $table->set(0, 42);
    is($table->get(0), 42, 'set/get roundtrip');

    # Null element
    ok(!defined($table->get(5)), 'unset element is undef');

    # Out of bounds
    my $died = 0;
    eval { $table->get(10) };
    $died = 1 if $@;
    ok($died, 'out-of-bounds get traps');
};

# ============================================================================
# Bytecode Decoder
# ============================================================================

subtest 'decode_function_body simple' => sub {
    # local.get 0, local.get 0, i32.mul, end
    my $body = {
        code   => [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B],
        locals => [],
    };
    my $instructions = CodingAdventures::WasmExecution::decode_function_body($body);
    is(scalar(@$instructions), 4, 'decoded 4 instructions');
    is($instructions->[0]{opcode}, 0x20, 'first is local.get');
    is($instructions->[0]{operand}, 0, 'operand is 0');
    is($instructions->[2]{opcode}, 0x6C, 'third is i32.mul');
    is($instructions->[3]{opcode}, 0x0B, 'fourth is end');
};

subtest 'decode_function_body with i32.const' => sub {
    # i32.const 5, end
    my $body = {
        code   => [0x41, 0x05, 0x0B],
        locals => [],
    };
    my $instructions = CodingAdventures::WasmExecution::decode_function_body($body);
    is(scalar(@$instructions), 2, 'decoded 2 instructions');
    is($instructions->[0]{opcode}, 0x41, 'opcode is i32.const');
    is($instructions->[0]{operand}, 5, 'operand is 5');
};

# ============================================================================
# Control Flow Map
# ============================================================================

subtest 'build_control_flow_map' => sub {
    # block, i32.const 1, end
    my $instructions = [
        { opcode => 0x02, operand => 0x40 },  # block
        { opcode => 0x41, operand => 1 },      # i32.const 1
        { opcode => 0x0B },                     # end
    ];
    my $map = CodingAdventures::WasmExecution::build_control_flow_map($instructions);
    ok(exists $map->{0}, 'block at 0 is mapped');
    is($map->{0}{end_pc}, 2, 'block end is at 2');
};

# ============================================================================
# Constant Expression Evaluator
# ============================================================================

subtest 'evaluate_const_expr i32' => sub {
    # i32.const 42, end
    my $expr = [0x41, 42, 0x0B];
    my $result = evaluate_const_expr($expr);
    is($result->{type}, 0x7F, 'result is i32');
    is($result->{value}, 42, 'value is 42');
};

subtest 'evaluate_const_expr global.get' => sub {
    my $globals = [i32(99)];
    my $expr = [0x23, 0x00, 0x0B];
    my $result = evaluate_const_expr($expr, $globals);
    is($result->{value}, 99, 'global.get returns correct value');
};

# ============================================================================
# Engine — Direct function calls
# ============================================================================

subtest 'Engine call_function: i32 add' => sub {
    # func (param i32 i32) (result i32): local.get 0, local.get 1, i32.add, end
    my $body = {
        code   => [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, [i32(10), i32(20)]);
    is(scalar(@$results), 1, 'one result');
    is($results->[0]{value}, 30, '10 + 20 = 30');
};

subtest 'Engine call_function: i32 mul (square)' => sub {
    # func (param i32) (result i32): local.get 0, local.get 0, i32.mul, end
    my $body = {
        code   => [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );

    my $results = $engine->call_function(0, [i32(5)]);
    is($results->[0]{value}, 25, 'square(5) = 25');

    $results = $engine->call_function(0, [i32(0)]);
    is($results->[0]{value}, 0, 'square(0) = 0');

    $results = $engine->call_function(0, [i32(-3)]);
    is($results->[0]{value}, 9, 'square(-3) = 9');
};

subtest 'Engine call_function: i32 sub' => sub {
    my $body = {
        code   => [0x20, 0x00, 0x20, 0x01, 0x6B, 0x0B],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, [i32(50), i32(8)]);
    is($results->[0]{value}, 42, '50 - 8 = 42');
};

subtest 'Engine call_function: i32 div_s' => sub {
    my $body = {
        code   => [0x20, 0x00, 0x20, 0x01, 0x6D, 0x0B],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, [i32(10), i32(3)]);
    is($results->[0]{value}, 3, '10 / 3 = 3 (truncated)');

    # Division by zero traps
    my $died = 0;
    eval { $engine->call_function(0, [i32(10), i32(0)]) };
    $died = 1 if $@;
    ok($died, 'division by zero traps');
};

subtest 'Engine: i32 comparison ops' => sub {
    # Test eqz: local.get 0, i32.eqz, end
    my $eqz_body = { code => [0x20, 0x00, 0x45, 0x0B], locals => [] };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F], results => [0x7F] }],
        func_bodies    => [$eqz_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [i32(0)])->[0]{value}, 1, 'eqz(0) = 1');
    is($engine->call_function(0, [i32(5)])->[0]{value}, 0, 'eqz(5) = 0');
};

subtest 'Engine: i32 bitwise ops' => sub {
    # and: local.get 0, local.get 1, i32.and, end
    my $and_body = { code => [0x20, 0x00, 0x20, 0x01, 0x71, 0x0B], locals => [] };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$and_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [i32(0xFF), i32(0x0F)])->[0]{value}, 0x0F, 'FF & 0F = 0F');

    # or
    my $or_body = { code => [0x20, 0x00, 0x20, 0x01, 0x72, 0x0B], locals => [] };
    $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$or_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [i32(0xF0), i32(0x0F)])->[0]{value}, 0xFF, 'F0 | 0F = FF');

    # xor
    my $xor_body = { code => [0x20, 0x00, 0x20, 0x01, 0x73, 0x0B], locals => [] };
    $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$xor_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [i32(0xFF), i32(0x0F)])->[0]{value}, 0xF0, 'FF ^ 0F = F0');
};

subtest 'Engine: i32 shift/rotate ops' => sub {
    # shl
    my $shl_body = { code => [0x20, 0x00, 0x20, 0x01, 0x74, 0x0B], locals => [] };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F, 0x7F], results => [0x7F] }],
        func_bodies    => [$shl_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [i32(1), i32(4)])->[0]{value}, 16, '1 << 4 = 16');
};

subtest 'Engine: i32 const' => sub {
    # i32.const 42, end  (42 = 0x2A, fits in 7 bits signed LEB128)
    my $body = { code => [0x41, 42, 0x0B], locals => [] };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, []);
    is($results->[0]{value}, 42, 'i32.const 42');

    # i32.const 99, end  (99 = 0x63, bit 6 set so needs 2-byte signed LEB128: 0xE3 0x00)
    my $body2 = { code => [0x41, 0xE3, 0x00, 0x0B], locals => [] };
    $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [0x7F] }],
        func_bodies    => [$body2],
        host_functions => [undef],
    );
    $results = $engine->call_function(0, []);
    is($results->[0]{value}, 99, 'i32.const 99 (2-byte LEB128)');
};

subtest 'Engine: block and branch' => sub {
    # block 0x7F (returns i32)
    #   i32.const 42
    #   br 0
    #   i32.const 99  (unreachable)
    # end
    my $body = {
        code => [
            0x02, 0x7F,        # block (returns i32)
            0x41, 42,          # i32.const 42
            0x0C, 0x00,        # br 0
            0x41, 99,          # i32.const 99 (dead code)
            0x0B,              # end (block)
            0x0B,              # end (function)
        ],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, []);
    is($results->[0]{value}, 42, 'branch skips dead code');
};

subtest 'Engine: host function call' => sub {
    my $host_fn = sub {
        my ($args) = @_;
        my $x = $args->[0]{value};
        return [i32($x * 2)];
    };
    # Call function 0 (host), which doubles the argument
    # func (param i32) (result i32): local.get 0, call 0 (itself doesn't work, use host at index 0)
    # Actually, let's set up: func 0 = host (double), func 1 = module calling func 0
    my $body = {
        code => [0x20, 0x00, 0x10, 0x00, 0x0B],  # local.get 0, call 0, end
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [
            { params => [0x7F], results => [0x7F] },  # func 0: host
            { params => [0x7F], results => [0x7F] },  # func 1: module
        ],
        func_bodies    => [undef, $body],
        host_functions => [$host_fn, undef],
    );
    my $results = $engine->call_function(1, [i32(21)]);
    is($results->[0]{value}, 42, 'host function returns doubled value');
};

subtest 'Engine: local variables with declared locals' => sub {
    # func (param i32) (result i32):
    #   local.get 0     -- get the parameter
    #   local.set 1     -- store in declared local
    #   local.get 1     -- reload from declared local
    #   local.get 1     -- reload again
    #   i32.add         -- sum = param + param
    #   end
    my $body = {
        code   => [0x20, 0x00, 0x21, 0x01, 0x20, 0x01, 0x20, 0x01, 0x6A, 0x0B],
        locals => [0x7F],  # one declared i32 local
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7F], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, [i32(7)]);
    is($results->[0]{value}, 14, 'local set/get works: 7 + 7 = 14');
};

subtest 'Engine: unreachable traps' => sub {
    my $body = { code => [0x00, 0x0B], locals => [] };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $died = 0;
    eval { $engine->call_function(0, []) };
    $died = 1 if $@;
    ok($died, 'unreachable instruction traps');
};

subtest 'Engine: drop and select' => sub {
    # drop: i32.const 10, i32.const 20, drop, end -> 10
    my $drop_body = {
        code => [0x41, 10, 0x41, 20, 0x1A, 0x0B],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [0x7F] }],
        func_bodies    => [$drop_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [])->[0]{value}, 10, 'drop discards top');

    # select: i32.const 10, i32.const 20, i32.const 1, select, end -> 10
    my $sel_body = {
        code => [0x41, 10, 0x41, 20, 0x41, 0x01, 0x1B, 0x0B],
        locals => [],
    };
    $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [0x7F] }],
        func_bodies    => [$sel_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [])->[0]{value}, 10, 'select(10,20,1) = 10');

    # select with 0 condition: should pick second
    my $sel0_body = {
        code => [0x41, 10, 0x41, 20, 0x41, 0x00, 0x1B, 0x0B],
        locals => [],
    };
    $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [], results => [0x7F] }],
        func_bodies    => [$sel0_body],
        host_functions => [undef],
    );
    is($engine->call_function(0, [])->[0]{value}, 20, 'select(10,20,0) = 20');
};

subtest 'Engine: f64 arithmetic' => sub {
    # f64.add: f64.const 1.5, f64.const 2.5, f64.add, end
    # f64.const requires 8-byte LE, we'll use the simpler path via i32 math
    # Instead test through Engine directly with typed args
    my $body = {
        code => [0x20, 0x00, 0x20, 0x01, 0xA0, 0x0B],
        locals => [],
    };
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        func_types     => [{ params => [0x7C, 0x7C], results => [0x7C] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );
    my $results = $engine->call_function(0, [f64(1.5), f64(2.5)]);
    ok(abs($results->[0]{value} - 4.0) < 1e-10, 'f64 1.5 + 2.5 = 4.0');
};

done_testing;
