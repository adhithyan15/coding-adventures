use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmSimulator;
use CodingAdventures::WasmLeb128 qw(encode_unsigned encode_signed);

# ============================================================================
# Binary building helpers
# ============================================================================
#
# We build minimal Wasm binaries by hand using pack(). This lets each test
# construct a complete .wasm file without needing an actual compiler.
# See the wasm_module_parser tests for a full explanation of the binary format.

# b(@bytes) — pack byte integers into a binary string
sub b { pack('C*', @_) }

# leb_u($n) — unsigned LEB128 binary string
sub leb_u {
    my ($n) = @_;
    return pack('C*', encode_unsigned($n));
}

# leb_s($n) — signed LEB128 binary string (for i32.const immediates)
sub leb_s {
    my ($n) = @_;
    return pack('C*', encode_signed($n));
}

# str_field($s) — length-prefixed UTF-8 string (for export/import names)
sub str_field {
    my ($s) = @_;
    return leb_u(length($s)) . $s;
}

# section_wrap($id, $content) — wrap binary content in a Wasm section envelope
sub section_wrap {
    my ($id, $content) = @_;
    return b($id) . leb_u(length($content)) . $content;
}

# The 8-byte Wasm module header (magic + version)
my $WASM_HEADER = b(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);

# ============================================================================
# Module component helpers
# ============================================================================

# type_sec(\@types) — build a type section
# Each type: { params => [...], results => [...] }
sub type_sec {
    my ($types) = @_;
    my $content = leb_u(scalar @$types);
    for my $t (@$types) {
        $content .= b(0x60);
        $content .= leb_u(scalar @{ $t->{params} });
        $content .= b($_) for @{ $t->{params} };
        $content .= leb_u(scalar @{ $t->{results} });
        $content .= b($_) for @{ $t->{results} };
    }
    return section_wrap(1, $content);
}

# func_sec(\@type_indices) — build a function section
sub func_sec {
    my ($indices) = @_;
    my $content = leb_u(scalar @$indices);
    $content .= leb_u($_) for @$indices;
    return section_wrap(3, $content);
}

# export_func($name, $idx) — one function export entry
sub export_func_entry {
    my ($name, $idx) = @_;
    return str_field($name) . b(0x00) . leb_u($idx);
}

# export_global($name, $idx) — one global export entry
sub export_global_entry {
    my ($name, $idx) = @_;
    return str_field($name) . b(0x03) . leb_u($idx);
}

# export_mem($name, $idx) — one memory export entry
sub export_mem_entry {
    my ($name, $idx) = @_;
    return str_field($name) . b(0x02) . leb_u($idx);
}

# export_sec(\@entries) — build an export section
sub export_sec {
    my ($entries) = @_;
    my $content = leb_u(scalar @$entries);
    $content .= $_ for @$entries;
    return section_wrap(7, $content);
}

# code_entry(\@local_groups, $body_binary) — one function code entry
# local_groups: [{ count => N, type => 0x7F }, ...]
sub code_entry {
    my ($local_groups, $body) = @_;
    my $locals = leb_u(scalar @$local_groups);
    for my $lg (@$local_groups) {
        $locals .= leb_u($lg->{count}) . b($lg->{type});
    }
    my $entry = $locals . $body;
    return leb_u(length($entry)) . $entry;
}

# code_sec(\@entries) — build a code section
sub code_sec {
    my ($entries) = @_;
    my $content = leb_u(scalar @$entries);
    $content .= $_ for @$entries;
    return section_wrap(10, $content);
}

# global_sec(\@globals) — build a global section
# globals: [{ val_type => 0x7F, mutable => 0|1, init => binary_string }, ...]
sub global_sec {
    my ($globals) = @_;
    my $content = leb_u(scalar @$globals);
    for my $g (@$globals) {
        $content .= b($g->{val_type}, $g->{mutable} ? 1 : 0) . $g->{init};
    }
    return section_wrap(6, $content);
}

# mem_sec(\@memories) — build a memory section
# memories: [{ min => N, max => M? }, ...]
sub mem_sec {
    my ($mems) = @_;
    my $content = leb_u(scalar @$mems);
    for my $m (@$mems) {
        if (defined $m->{max}) {
            $content .= b(0x01) . leb_u($m->{min}) . leb_u($m->{max});
        } else {
            $content .= b(0x00) . leb_u($m->{min});
        }
    }
    return section_wrap(5, $content);
}

# new_instance($wasm_binary_string) — parse and instantiate a Wasm module
sub new_instance {
    my ($wasm) = @_;
    my $mod = parse($wasm);
    return CodingAdventures::WasmSimulator->new($mod);
}

# ============================================================================
# Tests
# ============================================================================

# ---------------------------------------------------------------------------
# to_i32 wrapping
# ---------------------------------------------------------------------------

subtest 'to_i32 wrapping' => sub {
    is(CodingAdventures::WasmSimulator::to_i32(0),          0,           '0 unchanged');
    is(CodingAdventures::WasmSimulator::to_i32(42),         42,          '42 unchanged');
    is(CodingAdventures::WasmSimulator::to_i32(-1),         -1,          '-1 unchanged');
    is(CodingAdventures::WasmSimulator::to_i32(2147483648), -2147483648, 'wraps INT_MAX+1 to INT_MIN');
    is(CodingAdventures::WasmSimulator::to_i32(4294967295), -1,          'UINT32_MAX → -1');
    is(CodingAdventures::WasmSimulator::to_i32(4294967296), 0,           '2^32 → 0');
};

# ---------------------------------------------------------------------------
# Minimal module instantiation
# ---------------------------------------------------------------------------

subtest 'Instance.new minimal module' => sub {
    my $mod  = parse($WASM_HEADER);
    my $inst = CodingAdventures::WasmSimulator->new($mod);
    ok(ref($inst) eq 'CodingAdventures::WasmSimulator', 'returns instance');
    is($inst->{memory}{size_pages}, 0, 'no memory pages for empty module');
};

# ---------------------------------------------------------------------------
# i32.const: constant push
# ---------------------------------------------------------------------------

subtest 'i32.const' => sub {
    # Type: () → i32
    # Body: i32.const 42; end
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('answer', 0)])
        . code_sec([code_entry([], b(0x41, 0x2A, 0x0B))]);

    my $inst = new_instance($wasm);
    my @r = $inst->call('answer');
    is($r[0], 42, 'returns 42');

    # Negative constant: i32.const -1 = LEB128 0x7F
    my $wasm2 = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('neg', 0)])
        . code_sec([code_entry([], b(0x41, 0x7F, 0x0B))]);

    my $inst2 = new_instance($wasm2);
    is(($inst2->call('neg'))[0], -1, 'i32.const -1');
};

# ---------------------------------------------------------------------------
# local.get / local.set / local.tee
# ---------------------------------------------------------------------------

subtest 'local.get identity' => sub {
    # Type: (i32) → i32; Body: local.get 0; end
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('id', 0)])
        . code_sec([code_entry([], b(0x20, 0x00, 0x0B))]);

    my $inst = new_instance($wasm);
    is(($inst->call('id', 99))[0],  99,  '99 → 99');
    is(($inst->call('id', 0))[0],   0,   '0 → 0');
    is(($inst->call('id', -7))[0], -7,  '-7 → -7');
};

subtest 'local.set roundtrip' => sub {
    my $body = b(
        0x20, 0x00,   # local.get 0
        0x21, 0x01,   # local.set 1
        0x20, 0x01,   # local.get 1
        0x0B          # end
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('rt', 0)])
        . code_sec([code_entry([{count=>1, type=>0x7F}], $body)]);

    my $inst = new_instance($wasm);
    is(($inst->call('rt', 55))[0], 55, 'roundtrip 55');
};

subtest 'local.tee' => sub {
    my $body = b(
        0x20, 0x00,   # local.get 0
        0x22, 0x01,   # local.tee 1
        0x1A,         # drop
        0x20, 0x01,   # local.get 1
        0x0B          # end
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('tee', 0)])
        . code_sec([code_entry([{count=>1, type=>0x7F}], $body)]);

    my $inst = new_instance($wasm);
    is(($inst->call('tee', 77))[0], 77, 'tee stores and retrieves 77');
};

# ---------------------------------------------------------------------------
# i32 arithmetic
# ---------------------------------------------------------------------------

subtest 'i32 arithmetic' => sub {
    # Helper: make a module with a binary i32 op
    my $binop_module = sub {
        my ($opcode) = @_;
        my $body = b(0x20, 0x00, 0x20, 0x01, $opcode, 0x0B);
        return $WASM_HEADER
            . type_sec([{params=>[0x7F, 0x7F], results=>[0x7F]}])
            . func_sec([0])
            . export_sec([export_func_entry('op', 0)])
            . code_sec([code_entry([], $body)]);
    };

    my $add = new_instance($binop_module->(0x6A));
    is(($add->call('op', 3, 4))[0],          7,           'add: 3+4=7');
    is(($add->call('op', 2147483647, 1))[0], -2147483648, 'add wraps on overflow');

    my $sub = new_instance($binop_module->(0x6B));
    is(($sub->call('op', 10, 3))[0],          7,          'sub: 10-3=7');
    is(($sub->call('op', -2147483648, 1))[0], 2147483647, 'sub wraps on underflow');

    my $mul = new_instance($binop_module->(0x6C));
    is(($mul->call('op', 6, 7))[0],   42,  'mul: 6*7=42');
    is(($mul->call('op', -4, 5))[0], -20,  'mul: -4*5=-20');

    my $div = new_instance($binop_module->(0x6D));
    is(($div->call('op', 20, 4))[0], 5,  'div_s: 20/4=5');
    is(($div->call('op', -7, 2))[0], -3, 'div_s: -7/2=-3 (truncated)');
    ok(dies { $div->call('op', 5, 0) }, 'div_s: traps on div by zero');

    my $rem = new_instance($binop_module->(0x6F));
    is(($rem->call('op', 10, 3))[0],  1,  'rem_s: 10 rem 3 = 1');
    is(($rem->call('op', -10, 3))[0], -1, 'rem_s: -10 rem 3 = -1');
};

# ---------------------------------------------------------------------------
# i32 bitwise
# ---------------------------------------------------------------------------

subtest 'i32 bitwise' => sub {
    my $binop = sub {
        my ($opcode) = @_;
        my $body = b(0x20, 0x00, 0x20, 0x01, $opcode, 0x0B);
        return new_instance($WASM_HEADER
            . type_sec([{params=>[0x7F, 0x7F], results=>[0x7F]}])
            . func_sec([0])
            . export_sec([export_func_entry('op', 0)])
            . code_sec([code_entry([], $body)]));
    };

    my $and = $binop->(0x71);
    is(($and->call('op', 5, 3))[0],     1,      'and: 5&3=1');
    is(($and->call('op', 0xFF00, 0x0FF0))[0], 0x0F00, 'and mask');

    my $or = $binop->(0x72);
    is(($or->call('op', 5, 3))[0],  7, 'or: 5|3=7');

    my $xor = $binop->(0x73);
    is(($xor->call('op', 5, 3))[0],  6, 'xor: 5^3=6');
    is(($xor->call('op', 42, 42))[0], 0, 'xor: n^n=0');

    my $shl = $binop->(0x74);
    is(($shl->call('op', 1, 3))[0],   8,           'shl: 1<<3=8');
    is(($shl->call('op', 1, 31))[0], -2147483648,  'shl: 1<<31=INT_MIN');

    my $shr_s = $binop->(0x75);
    is(($shr_s->call('op', 8, 1))[0],  4,  'shr_s: 8>>1=4');
    is(($shr_s->call('op', -8, 1))[0], -4, 'shr_s: -8>>1=-4 (arithmetic)');
};

# ---------------------------------------------------------------------------
# i32 comparisons
# ---------------------------------------------------------------------------

subtest 'i32 comparisons' => sub {
    my $cmpop = sub {
        my ($opcode) = @_;
        my $body = b(0x20, 0x00, 0x20, 0x01, $opcode, 0x0B);
        return new_instance($WASM_HEADER
            . type_sec([{params=>[0x7F, 0x7F], results=>[0x7F]}])
            . func_sec([0])
            . export_sec([export_func_entry('cmp', 0)])
            . code_sec([code_entry([], $body)]));
    };

    my $eq = $cmpop->(0x46);
    is(($eq->call('cmp', 5, 5))[0], 1, 'eq: 5==5 → 1');
    is(($eq->call('cmp', 5, 6))[0], 0, 'eq: 5==6 → 0');

    my $ne = $cmpop->(0x47);
    is(($ne->call('cmp', 5, 6))[0], 1, 'ne: 5!=6 → 1');
    is(($ne->call('cmp', 5, 5))[0], 0, 'ne: 5!=5 → 0');

    my $lt = $cmpop->(0x48);
    is(($lt->call('cmp', 3, 5))[0],  1, 'lt_s: 3<5 → 1');
    is(($lt->call('cmp', -1, 0))[0], 1, 'lt_s: -1<0 → 1 (signed)');
    is(($lt->call('cmp', 5, 3))[0],  0, 'lt_s: 5<3 → 0');

    my $le = $cmpop->(0x4C);
    is(($le->call('cmp', 5, 5))[0], 1, 'le_s: 5<=5 → 1');
    is(($le->call('cmp', 5, 4))[0], 0, 'le_s: 5<=4 → 0');

    my $gt = $cmpop->(0x4A);
    is(($gt->call('cmp', 5, 3))[0], 1, 'gt_s: 5>3 → 1');

    my $ge = $cmpop->(0x4E);
    is(($ge->call('cmp', 5, 5))[0], 1, 'ge_s: 5>=5 → 1');
    is(($ge->call('cmp', 4, 5))[0], 0, 'ge_s: 4>=5 → 0');
};

# ---------------------------------------------------------------------------
# Stack operations: nop, drop, select
# ---------------------------------------------------------------------------

subtest 'stack operations' => sub {
    # nop; i32.const 7; end
    my $nop_wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], b(0x01, 0x41, 0x07, 0x0B))]);
    is((new_instance($nop_wasm)->call('f'))[0], 7, 'nop: returns 7');

    # i32.const 99; i32.const 42; drop; end → returns 99
    my $drop_wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], b(0x41, 0xE3, 0x00, 0x41, 0x2A, 0x1A, 0x0B))]);
    is((new_instance($drop_wasm)->call('f'))[0], 99, 'drop: discards top');

    # select with condition=1 picks val1
    my $sel1 = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], b(0x41, 0x0A, 0x41, 0x14, 0x41, 0x01, 0x1B, 0x0B))]);
    is((new_instance($sel1)->call('f'))[0], 10, 'select cond=1: picks first');

    # select with condition=0 picks val2
    my $sel0 = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], b(0x41, 0x0A, 0x41, 0x14, 0x41, 0x00, 0x1B, 0x0B))]);
    is((new_instance($sel0)->call('f'))[0], 20, 'select cond=0: picks second');
};

# ---------------------------------------------------------------------------
# return instruction
# ---------------------------------------------------------------------------

subtest 'return' => sub {
    # i32.const 1; return; i32.const 2; end → should return 1
    my $body = b(0x41, 0x01, 0x0F, 0x41, 0x02, 0x0B);
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], $body)]);
    is((new_instance($wasm)->call('f'))[0], 1, 'return exits early');
};

# ---------------------------------------------------------------------------
# Global variables
# ---------------------------------------------------------------------------

subtest 'global variables' => sub {
    # Global 0: const i32 = 42
    # Function: () → i32 = { global.get 0; end }
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . global_sec([{val_type=>0x7F, mutable=>0, init=>b(0x41, 0x2A, 0x0B)}])
        . export_sec([export_func_entry('get_g', 0), export_global_entry('g', 0)])
        . code_sec([code_entry([], b(0x23, 0x00, 0x0B))]);

    my $inst = new_instance($wasm);
    is(($inst->call('get_g'))[0], 42, 'global.get returns 42');
    is($inst->get_global('g'),    42, 'get_global API returns 42');
};

subtest 'mutable global' => sub {
    # Global 0: mutable i32 = 0
    # set func: (i32) → void = global.set 0; end
    # get func: () → i32    = global.get 0; end
    my $wasm = $WASM_HEADER
        . type_sec([
            {params=>[0x7F], results=>[]},     # type 0: setter
            {params=>[],     results=>[0x7F]}, # type 1: getter
          ])
        . func_sec([0, 1])
        . global_sec([{val_type=>0x7F, mutable=>1, init=>b(0x41, 0x00, 0x0B)}])
        . export_sec([
            export_func_entry('set_g', 0),
            export_func_entry('get_g', 1),
            export_global_entry('g', 0),
          ])
        . code_sec([
            code_entry([], b(0x20, 0x00, 0x24, 0x00, 0x0B)),
            code_entry([], b(0x23, 0x00, 0x0B)),
          ]);

    my $inst = new_instance($wasm);
    is($inst->get_global('g'), 0, 'initial global is 0');
    $inst->call('set_g', 99);
    is($inst->get_global('g'),          99, 'global updated to 99 after call');
    is(($inst->call('get_g'))[0],       99, 'get_g function also returns 99');
};

subtest 'set_global API' => sub {
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . global_sec([{val_type=>0x7F, mutable=>1, init=>b(0x41, 0x00, 0x0B)}])
        . export_sec([export_func_entry('get_g', 0), export_global_entry('counter', 0)])
        . code_sec([code_entry([], b(0x23, 0x00, 0x0B))]);

    my $inst = new_instance($wasm);
    $inst->set_global('counter', 777);
    is(($inst->call('get_g'))[0], 777, 'set_global API works');
};

subtest 'set_global immutable dies' => sub {
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . global_sec([{val_type=>0x7F, mutable=>0, init=>b(0x41, 0x01, 0x0B)}])
        . export_sec([export_func_entry('get_g', 0), export_global_entry('g', 0)])
        . code_sec([code_entry([], b(0x23, 0x00, 0x0B))]);

    my $inst = new_instance($wasm);
    ok(dies { $inst->set_global('g', 5) }, 'dies on immutable global');
};

# ---------------------------------------------------------------------------
# Memory operations
# ---------------------------------------------------------------------------

subtest 'memory_read/memory_write roundtrip' => sub {
    my $wasm  = $WASM_HEADER . mem_sec([{min=>1}]);
    my $inst  = new_instance($wasm);
    $inst->memory_write(0, [0xDE, 0xAD, 0xBE, 0xEF]);
    my @bytes = $inst->memory_read(0, 4);
    is($bytes[0], 0xDE, 'byte 0');
    is($bytes[1], 0xAD, 'byte 1');
    is($bytes[2], 0xBE, 'byte 2');
    is($bytes[3], 0xEF, 'byte 3');
};

subtest 'i32.store and i32.load' => sub {
    my $store_body = b(0x20, 0x00, 0x20, 0x01, 0x36, 0x00, 0x00, 0x0B);
    my $load_body  = b(0x20, 0x00, 0x28, 0x00, 0x00, 0x0B);
    my $wasm = $WASM_HEADER
        . type_sec([
            {params=>[0x7F, 0x7F], results=>[]},   # store
            {params=>[0x7F],       results=>[0x7F]},# load
          ])
        . func_sec([0, 1])
        . mem_sec([{min=>1}])
        . export_sec([
            export_func_entry('store', 0),
            export_func_entry('load',  1),
          ])
        . code_sec([
            code_entry([], $store_body),
            code_entry([], $load_body),
          ]);

    my $inst = new_instance($wasm);
    $inst->call('store', 0, 12345);
    is(($inst->call('load', 0))[0], 12345, 'load after store: 12345');

    $inst->call('store', 4, -99);
    is(($inst->call('load', 4))[0], -99, 'load after store: -99');
};

subtest 'memory.size' => sub {
    my $body = b(0x3F, 0x00, 0x0B);
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . mem_sec([{min=>2}])
        . export_sec([export_func_entry('size', 0)])
        . code_sec([code_entry([], $body)]);

    is((new_instance($wasm)->call('size'))[0], 2, 'memory.size returns 2');
};

subtest 'memory.grow' => sub {
    my $body = b(0x41, 0x01, 0x40, 0x00, 0x0B);
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . mem_sec([{min=>1}])
        . export_sec([export_func_entry('grow', 0)])
        . code_sec([code_entry([], $body)]);

    my $inst = new_instance($wasm);
    my $old  = ($inst->call('grow'))[0];
    is($old, 1, 'memory.grow returns old size (1)');
    is($inst->{memory}{size_pages}, 2, 'memory now has 2 pages');
};

subtest 'memory OOB traps' => sub {
    my $wasm = $WASM_HEADER . mem_sec([{min=>1}]);
    my $inst = new_instance($wasm);
    ok(dies { $inst->memory_read(65536, 1) }, 'memory_read OOB traps');
};

# ---------------------------------------------------------------------------
# Control flow: block, loop, br, br_if
# ---------------------------------------------------------------------------

subtest 'block falls through' => sub {
    my $body = b(
        0x02, 0x40,   # block void
        0x41, 0x2A,   #   i32.const 42
        0x1A,         #   drop
        0x0B,         # end (block)
        0x41, 0x07,   # i32.const 7
        0x0B          # end (function)
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], $body)]);
    is((new_instance($wasm)->call('f'))[0], 7, 'block falls through to 7');
};

subtest 'br jumps out of block' => sub {
    my $body = b(
        0x02, 0x40,         # block void
        0x41, 0x01,         #   i32.const 1
        0x1A,               #   drop
        0x0C, 0x00,         #   br 0 (exit block)
        0x41, 0xFF, 0x00,   #   i32.const 127 (skipped)
        0x0B,               # end (block)
        0x41, 0xE3, 0x00,   # i32.const 99 (SLEB128: 0xE3 0x00)
        0x0B                # end (function)
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], $body)]);
    is((new_instance($wasm)->call('f'))[0], 99, 'br exits block');
};

subtest 'br_if taken' => sub {
    my $body = b(
        0x02, 0x40,
        0x41, 0x01,
        0x0D, 0x00,         # br_if 0 (taken: condition=1)
        0x41, 0x07, 0x1A,   # skipped
        0x0B,
        0x41, 0xE4, 0x00,   # i32.const 100 (SLEB128: 0xE4 0x00)
        0x0B
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], $body)]);
    is((new_instance($wasm)->call('f'))[0], 100, 'br_if taken → 100');
};

subtest 'br_if not taken' => sub {
    my $body = b(
        0x02, 0x40,
        0x41, 0x00,
        0x0D, 0x00,           # br_if 0 (not taken: condition=0)
        0x41, 0x05, 0x1A,     # i32.const 5; drop
        0x0B,
        0x41, 0xC8, 0x01,     # i32.const 200
        0x0B
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], $body)]);
    is((new_instance($wasm)->call('f'))[0], 200, 'br_if not taken → 200');
};

subtest 'loop countdown' => sub {
    # (func (param i32) (result i32)
    #   (block (loop
    #     local.get 0; i32.eqz; br_if 1
    #     local.get 0; i32.const -1; i32.add; local.set 0
    #     br 0
    #   ))
    #   local.get 0
    # )
    my $body = b(
        0x02, 0x40,
        0x03, 0x40,
        0x20, 0x00,
        0x45,
        0x0D, 0x01,
        0x20, 0x00,
        0x41, 0x7F,
        0x6A,
        0x21, 0x00,
        0x0C, 0x00,
        0x0B,
        0x0B,
        0x20, 0x00,
        0x0B
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('countdown', 0)])
        . code_sec([code_entry([], $body)]);

    my $inst = new_instance($wasm);
    is(($inst->call('countdown', 5))[0], 0, 'countdown(5) → 0');
    is(($inst->call('countdown', 0))[0], 0, 'countdown(0) → 0');
    is(($inst->call('countdown', 3))[0], 0, 'countdown(3) → 0');
};

# ---------------------------------------------------------------------------
# if / else
# ---------------------------------------------------------------------------

subtest 'if/else' => sub {
    # if (local.get 0) → i32: i32.const 1 else i32.const 0 end
    my $body = b(
        0x20, 0x00,
        0x04, 0x7F,
        0x41, 0x01,
        0x05,
        0x41, 0x00,
        0x0B,
        0x0B
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('bool_id', 0)])
        . code_sec([code_entry([], $body)]);

    my $inst = new_instance($wasm);
    is(($inst->call('bool_id', 1))[0],  1, 'if/else: cond=1 → 1');
    is(($inst->call('bool_id', 99))[0], 1, 'if/else: cond=99 → 1');
    is(($inst->call('bool_id', 0))[0],  0, 'if/else: cond=0 → 0');
};

subtest 'if without else' => sub {
    my $body = b(
        0x20, 0x00,
        0x04, 0x40,
        0x41, 0x2A,
        0x21, 0x01,
        0x0B,
        0x20, 0x01,
        0x0B
    );
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('maybe42', 0)])
        . code_sec([code_entry([{count=>1, type=>0x7F}], $body)]);

    my $inst = new_instance($wasm);
    is(($inst->call('maybe42', 1))[0], 42, 'if no-else: cond=1 → 42');
    is(($inst->call('maybe42', 0))[0],  0, 'if no-else: cond=0 → 0');
};

# ---------------------------------------------------------------------------
# Function calls
# ---------------------------------------------------------------------------

subtest 'function calls' => sub {
    # double(x) = x + x
    # quad(x) = double(double(x))
    my $double = b(0x20, 0x00, 0x20, 0x00, 0x6A, 0x0B);
    my $quad   = b(0x20, 0x00, 0x10, 0x00, 0x10, 0x00, 0x0B);

    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0, 0])
        . export_sec([export_func_entry('quad', 1)])
        . code_sec([
            code_entry([], $double),
            code_entry([], $quad),
          ]);

    my $inst = new_instance($wasm);
    is(($inst->call('quad', 1))[0],  4,  'quad(1)=4');
    is(($inst->call('quad', 2))[0],  8,  'quad(2)=8');
    is(($inst->call('quad', 5))[0], 20,  'quad(5)=20');
};

subtest 'call_by_index' => sub {
    my $wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('f', 0)])
        . code_sec([code_entry([], b(0x41, 0x2A, 0x0B))]);
    my $inst = new_instance($wasm);
    is(($inst->call_by_index(0))[0], 42, 'call_by_index(0) returns 42');
};

# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------

subtest 'error handling' => sub {
    my $empty = new_instance($WASM_HEADER);
    ok(dies { $empty->call('no_such') },     'call to unknown export dies');
    ok(dies { $empty->get_global('nope') },  'get_global unknown dies');

    # unreachable
    my $trap_wasm = $WASM_HEADER
        . type_sec([{params=>[], results=>[]}])
        . func_sec([0])
        . export_sec([export_func_entry('trap', 0)])
        . code_sec([code_entry([], b(0x00, 0x0B))]);
    ok(dies { new_instance($trap_wasm)->call('trap') }, 'unreachable traps');

    # OOB memory
    my $load_body = b(0x20, 0x00, 0x28, 0x00, 0x00, 0x0B);
    my $oob_wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . mem_sec([{min=>1}])
        . export_sec([export_func_entry('load', 0)])
        . code_sec([code_entry([], $load_body)]);
    ok(dies { new_instance($oob_wasm)->call('load', 70000) }, 'OOB load traps');

    # div by zero
    my $div0 = $WASM_HEADER
        . type_sec([{params=>[], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('div0', 0)])
        . code_sec([code_entry([], b(0x41, 0x05, 0x41, 0x00, 0x6D, 0x0B))]);
    ok(dies { new_instance($div0)->call('div0') }, 'div by zero traps');
};

# ---------------------------------------------------------------------------
# Fibonacci (recursive, stress test for call stack)
# ---------------------------------------------------------------------------

subtest 'fibonacci' => sub {
    my $fib_body = b(
        0x20, 0x00,       # local.get 0
        0x41, 0x01,       # i32.const 1
        0x4C,             # i32.le_s
        0x04, 0x7F,       # if i32
        0x20, 0x00,       #   local.get 0
        0x05,             # else
        0x20, 0x00,       #   local.get 0
        0x41, 0x01,       #   i32.const 1
        0x6B,             #   i32.sub
        0x10, 0x00,       #   call 0
        0x20, 0x00,       #   local.get 0
        0x41, 0x02,       #   i32.const 2
        0x6B,             #   i32.sub
        0x10, 0x00,       #   call 0
        0x6A,             #   i32.add
        0x0B,             # end
        0x0B              # end function
    );

    my $wasm = $WASM_HEADER
        . type_sec([{params=>[0x7F], results=>[0x7F]}])
        . func_sec([0])
        . export_sec([export_func_entry('fib', 0)])
        . code_sec([code_entry([], $fib_body)]);

    my $inst = new_instance($wasm);
    is(($inst->call('fib', 0))[0],  0,  'fib(0)=0');
    is(($inst->call('fib', 1))[0],  1,  'fib(1)=1');
    is(($inst->call('fib', 2))[0],  1,  'fib(2)=1');
    is(($inst->call('fib', 5))[0],  5,  'fib(5)=5');
    is(($inst->call('fib', 10))[0], 55, 'fib(10)=55');
};

done_testing;
