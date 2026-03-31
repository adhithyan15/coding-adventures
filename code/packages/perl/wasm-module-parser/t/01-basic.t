use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmModuleParser qw(parse parse_header parse_section get_section);
use CodingAdventures::WasmLeb128 qw(encode_unsigned);

# ============================================================================
# Test helpers — build binary Wasm data from byte lists
# ============================================================================

# b(@bytes) — pack a list of byte integers into a binary string
sub b { pack('C*', @_) }

# leb_u($n) — encode n as unsigned LEB128 binary string
sub leb_u {
    my ($n) = @_;
    return pack('C*', encode_unsigned($n));
}

# str_field($s) — encode a string as length-prefixed UTF-8
sub str_field {
    my ($s) = @_;
    return leb_u(length($s)) . $s;
}

# section($id, $content_str) — wrap binary content in a section envelope
sub section_wrap {
    my ($id, $content) = @_;
    return b($id) . leb_u(length($content)) . $content;
}

# The 8-byte Wasm module header
my $WASM_HEADER = b(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);

# ============================================================================
# Minimal module — just the 8-byte header, no sections
# ============================================================================

subtest 'minimal module (header only)' => sub {
    my $wasm = $WASM_HEADER;
    my $mod  = parse($wasm);

    ok(ref($mod) eq 'HASH', 'returns a hashref');
    is($mod->{magic},   "\x00asm", 'magic is correct');
    is($mod->{version}, 1,         'version is 1');

    is(scalar @{ $mod->{types}     }, 0, 'types is empty');
    is(scalar @{ $mod->{imports}   }, 0, 'imports is empty');
    is(scalar @{ $mod->{functions} }, 0, 'functions is empty');
    is(scalar @{ $mod->{tables}    }, 0, 'tables is empty');
    is(scalar @{ $mod->{memories}  }, 0, 'memories is empty');
    is(scalar @{ $mod->{globals}   }, 0, 'globals is empty');
    is(scalar @{ $mod->{exports}   }, 0, 'exports is empty');
    is($mod->{start}, undef,            'start is undef');
    is(scalar @{ $mod->{codes}     }, 0, 'codes is empty');
    is(scalar @{ $mod->{custom}    }, 0, 'custom is empty');
};

# ============================================================================
# parse_header — explicit tests
# ============================================================================

subtest 'parse_header' => sub {
    use CodingAdventures::WasmModuleParser;
    my $bytes = [ unpack('C*', $WASM_HEADER) ];
    my $new_pos = CodingAdventures::WasmModuleParser::parse_header($bytes, 0);
    is($new_pos, 8, 'parse_header returns 8 after valid header');

    # Wrong magic
    my @bad = (0xFF, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);
    ok(dies { CodingAdventures::WasmModuleParser::parse_header(\@bad, 0) },
       'dies on wrong magic');

    # Wrong version
    my @bad_ver = (0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00);
    ok(dies { CodingAdventures::WasmModuleParser::parse_header(\@bad_ver, 0) },
       'dies on wrong version');
};

# ============================================================================
# Type section — one function type () → i32
# ============================================================================

subtest 'type section: () -> i32' => sub {
    # Type entry: 0x60 + params(0) + results(1, 0x7F)
    my $type_content = leb_u(1)           # count = 1
        . b(0x60)                          # func type marker
        . leb_u(0)                         # 0 params
        . leb_u(1) . b(0x7F);             # 1 result: i32

    my $wasm = $WASM_HEADER . section_wrap(1, $type_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{types} }, 1, 'one type entry');
    is(scalar @{ $mod->{types}[0]{params}  }, 0, 'no params');
    is(scalar @{ $mod->{types}[0]{results} }, 1, 'one result');
    is($mod->{types}[0]{results}[0], 0x7F, 'result is i32 (0x7F)');
};

subtest 'type section: (i32, i32) → i64' => sub {
    my $type_content = leb_u(1)
        . b(0x60)
        . leb_u(2) . b(0x7F, 0x7F)   # 2x i32 params
        . leb_u(1) . b(0x7E);         # 1x i64 result

    my $wasm = $WASM_HEADER . section_wrap(1, $type_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{types}[0]{params}  }, 2, 'two params');
    is($mod->{types}[0]{params}[0], 0x7F, 'param 0 is i32');
    is($mod->{types}[0]{params}[1], 0x7F, 'param 1 is i32');
    is(scalar @{ $mod->{types}[0]{results} }, 1, 'one result');
    is($mod->{types}[0]{results}[0], 0x7E, 'result is i64 (0x7E)');
};

subtest 'type section: multiple entries' => sub {
    # Type 0: () → ()
    # Type 1: (i32) → i32
    my $type_content = leb_u(2)
        . b(0x60) . leb_u(0) . leb_u(0)
        . b(0x60) . leb_u(1) . b(0x7F) . leb_u(1) . b(0x7F);

    my $wasm = $WASM_HEADER . section_wrap(1, $type_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{types} }, 2, 'two type entries');
    is(scalar @{ $mod->{types}[0]{params}  }, 0, 'type 0: no params');
    is(scalar @{ $mod->{types}[0]{results} }, 0, 'type 0: no results');
    is($mod->{types}[1]{params}[0],  0x7F, 'type 1: param is i32');
    is($mod->{types}[1]{results}[0], 0x7F, 'type 1: result is i32');
};

# ============================================================================
# Export section
# ============================================================================

subtest 'export section: function export "add"' => sub {
    my $exp_content = leb_u(1)
        . str_field("add")      # name = "add"
        . b(0x00) . leb_u(0);  # func, index 0

    my $wasm = $WASM_HEADER . section_wrap(7, $exp_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{exports} }, 1,      'one export');
    is($mod->{exports}[0]{name},    'add',  'name is "add"');
    is($mod->{exports}[0]{desc}{kind}, 'func', 'kind is func');
    is($mod->{exports}[0]{desc}{idx},  0,   'idx is 0');
};

subtest 'export section: mixed exports' => sub {
    my $exp_content = leb_u(2)
        . str_field("main")  . b(0x00) . leb_u(0)   # func export
        . str_field("mem")   . b(0x02) . leb_u(0);  # mem export

    my $wasm = $WASM_HEADER . section_wrap(7, $exp_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{exports} }, 2, 'two exports');
    is($mod->{exports}[0]{name},       'main', 'first export name');
    is($mod->{exports}[0]{desc}{kind}, 'func', 'first export kind');
    is($mod->{exports}[1]{name},       'mem',  'second export name');
    is($mod->{exports}[1]{desc}{kind}, 'mem',  'second export kind');
};

# ============================================================================
# Import section
# ============================================================================

subtest 'import section: function import' => sub {
    my $imp_content = leb_u(1)
        . str_field("env")
        . str_field("log")
        . b(0x00) . leb_u(0);   # func import, type_idx=0

    my $wasm = $WASM_HEADER . section_wrap(2, $imp_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{imports} }, 1,      'one import');
    is($mod->{imports}[0]{mod},     'env',  'module is "env"');
    is($mod->{imports}[0]{name},    'log',  'name is "log"');
    is($mod->{imports}[0]{desc}{kind},     'func', 'kind is func');
    is($mod->{imports}[0]{desc}{type_idx}, 0,      'type_idx is 0');
};

subtest 'import section: memory import (min=1, no max)' => sub {
    my $imp_content = leb_u(1)
        . str_field("env")
        . str_field("memory")
        . b(0x02)                    # mem import tag
        . b(0x00) . leb_u(1);       # limits: no max, min=1

    my $wasm = $WASM_HEADER . section_wrap(2, $imp_content);
    my $mod  = parse($wasm);

    is($mod->{imports}[0]{desc}{kind},             'mem', 'kind is mem');
    is($mod->{imports}[0]{desc}{limits}{min},       1,    'min=1');
    is($mod->{imports}[0]{desc}{limits}{max}, undef,      'max=undef');
};

subtest 'import section: global import (const i32)' => sub {
    my $imp_content = leb_u(1)
        . str_field("env")
        . str_field("stackPtr")
        . b(0x03)    # global import tag
        . b(0x7F)    # val_type = i32
        . b(0x00);   # mutability = const

    my $wasm = $WASM_HEADER . section_wrap(2, $imp_content);
    my $mod  = parse($wasm);

    is($mod->{imports}[0]{desc}{kind},     'global', 'kind is global');
    is($mod->{imports}[0]{desc}{val_type}, 0x7F,     'val_type is i32');
    is($mod->{imports}[0]{desc}{mutable},  0,        'not mutable');
};

subtest 'import section: table import (funcref, min=1)' => sub {
    my $imp_content = leb_u(1)
        . str_field("env")
        . str_field("table")
        . b(0x01)                    # table import tag
        . b(0x70)                    # ref_type = funcref
        . b(0x00) . leb_u(1);       # limits: no max, min=1

    my $wasm = $WASM_HEADER . section_wrap(2, $imp_content);
    my $mod  = parse($wasm);

    is($mod->{imports}[0]{desc}{kind},          'table', 'kind is table');
    is($mod->{imports}[0]{desc}{ref_type},       0x70,   'ref_type=funcref');
    is($mod->{imports}[0]{desc}{limits}{min},    1,      'min=1');
};

# ============================================================================
# Function section
# ============================================================================

subtest 'function section' => sub {
    my $func_content = leb_u(3)
        . leb_u(0) . leb_u(1) . leb_u(0);  # types: [0, 1, 0]

    my $wasm = $WASM_HEADER . section_wrap(3, $func_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{functions} }, 3, 'three function entries');
    is($mod->{functions}[0], 0, 'func 0 uses type 0');
    is($mod->{functions}[1], 1, 'func 1 uses type 1');
    is($mod->{functions}[2], 0, 'func 2 uses type 0');
};

# ============================================================================
# Memory section
# ============================================================================

subtest 'memory section: unbounded' => sub {
    my $mem_content = leb_u(1) . b(0x00) . leb_u(1);  # min=1, no max

    my $wasm = $WASM_HEADER . section_wrap(5, $mem_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{memories} }, 1,     'one memory');
    is($mod->{memories}[0]{limits}{min},    1,     'min=1');
    is($mod->{memories}[0]{limits}{max}, undef, 'max=undef');
};

subtest 'memory section: bounded' => sub {
    my $mem_content = leb_u(1)
        . b(0x01) . leb_u(1) . leb_u(16);  # min=1, max=16

    my $wasm = $WASM_HEADER . section_wrap(5, $mem_content);
    my $mod  = parse($wasm);

    is($mod->{memories}[0]{limits}{min}, 1,  'min=1');
    is($mod->{memories}[0]{limits}{max}, 16, 'max=16');
};

# ============================================================================
# Table section
# ============================================================================

subtest 'table section: funcref table' => sub {
    my $tbl_content = leb_u(1)
        . b(0x70)                    # funcref
        . b(0x00) . leb_u(10);      # min=10, no max

    my $wasm = $WASM_HEADER . section_wrap(4, $tbl_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{tables} },     1,    'one table');
    is($mod->{tables}[0]{ref_type},    0x70, 'ref_type=funcref');
    is($mod->{tables}[0]{limits}{min}, 10,   'min=10');
};

# ============================================================================
# Global section
# ============================================================================

subtest 'global section: const i32 = 42' => sub {
    # i32.const 42 = opcode 0x41, value 42 (0x2A as LEB128), end 0x0B
    my $global_content = leb_u(1)
        . b(0x7F)                    # val_type = i32
        . b(0x00)                    # mutability = const
        . b(0x41, 0x2A, 0x0B);      # i32.const 42; end

    my $wasm = $WASM_HEADER . section_wrap(6, $global_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{globals} }, 1,    'one global');
    is($mod->{globals}[0]{val_type},  0x7F, 'val_type=i32');
    is($mod->{globals}[0]{mutable},   0,    'not mutable');
    is($mod->{globals}[0]{init_expr}[0], 0x41, 'init_expr[0]=i32.const');
    is($mod->{globals}[0]{init_expr}[1], 0x2A, 'init_expr[1]=42');
    is($mod->{globals}[0]{init_expr}[2], 0x0B, 'init_expr[2]=end');
};

subtest 'global section: mutable i64' => sub {
    my $global_content = leb_u(1)
        . b(0x7E)          # val_type = i64
        . b(0x01)          # mutability = var (mutable)
        . b(0x42, 0x00, 0x0B);  # i64.const 0; end

    my $wasm = $WASM_HEADER . section_wrap(6, $global_content);
    my $mod  = parse($wasm);

    is($mod->{globals}[0]{val_type}, 0x7E, 'val_type=i64');
    is($mod->{globals}[0]{mutable},  1,    'is mutable');
};

# ============================================================================
# Start section
# ============================================================================

subtest 'start section' => sub {
    my $start_content = leb_u(5);  # function index 5

    my $wasm = $WASM_HEADER . section_wrap(8, $start_content);
    my $mod  = parse($wasm);

    is($mod->{start}, 5, 'start function index is 5');
};

# ============================================================================
# Code section
# ============================================================================

subtest 'code section: minimal function body' => sub {
    # Function body: no locals, just "end" (0x0B)
    # body_size = 2 bytes: local_count=0x00, end=0x0B
    my $body = b(0x00, 0x0B);    # local_count=0, end
    my $code_content = leb_u(1)  # count=1
        . leb_u(length($body))   # body_size
        . $body;

    my $wasm = $WASM_HEADER . section_wrap(10, $code_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{codes} },          1, 'one code entry');
    is(scalar @{ $mod->{codes}[0]{locals} }, 0, 'no locals');
    # body contains the end opcode
    is($mod->{codes}[0]{body}[0], 0x0B, 'body[0] is end opcode');
};

subtest 'code section: function with locals' => sub {
    # 2 local groups: 2x i32, 1x f64
    my $body = pack('C*',
        # local_decls_count = 2
        encode_unsigned(2),
        # group 0: count=2, type=i32
        encode_unsigned(2), 0x7F,
        # group 1: count=1, type=f64
        encode_unsigned(1), 0x7C,
        # end opcode
        0x0B,
    );
    my $code_content = leb_u(1) . leb_u(length($body)) . $body;

    my $wasm = $WASM_HEADER . section_wrap(10, $code_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{codes}[0]{locals} }, 2, 'two local groups');
    is($mod->{codes}[0]{locals}[0]{count}, 2,    'group 0: 2 locals');
    is($mod->{codes}[0]{locals}[0]{type},  0x7F, 'group 0: type=i32');
    is($mod->{codes}[0]{locals}[1]{count}, 1,    'group 1: 1 local');
    is($mod->{codes}[0]{locals}[1]{type},  0x7C, 'group 1: type=f64');
};

# ============================================================================
# Custom section
# ============================================================================

subtest 'custom section: name + data' => sub {
    my $custom_content = str_field("hello")    # name = "hello"
        . b(0x01, 0x02, 0x03);                 # data = [1, 2, 3]

    my $wasm = $WASM_HEADER . section_wrap(0, $custom_content);
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{custom} }, 1,       'one custom section');
    is($mod->{custom}[0]{name},    'hello', 'name is "hello"');
    is(scalar @{ $mod->{custom}[0]{data} }, 3, '3 data bytes');
    is($mod->{custom}[0]{data}[0], 0x01, 'data[0]=0x01');
    is($mod->{custom}[0]{data}[1], 0x02, 'data[1]=0x02');
    is($mod->{custom}[0]{data}[2], 0x03, 'data[2]=0x03');
};

subtest 'multiple custom sections' => sub {
    my $c1 = str_field("name")   . b(0xAA);
    my $c2 = str_field("source") . b(0xBB, 0xCC);

    my $wasm = $WASM_HEADER
        . section_wrap(0, $c1)
        . section_wrap(0, $c2);
    my $mod = parse($wasm);

    is(scalar @{ $mod->{custom} }, 2,        'two custom sections');
    is($mod->{custom}[0]{name},    'name',   'first is "name"');
    is($mod->{custom}[1]{name},    'source', 'second is "source"');
};

# ============================================================================
# get_section
# ============================================================================

subtest 'get_section' => sub {
    use CodingAdventures::WasmModuleParser qw(SECTION_TYPE SECTION_EXPORT);

    my $wasm = $WASM_HEADER . section_wrap(1,
        leb_u(1) . b(0x60) . leb_u(0) . leb_u(0)  # one empty type
    );
    my $mod = parse($wasm);

    my $types = get_section($mod, SECTION_TYPE);
    ok(ref($types) eq 'ARRAY', 'get_section SECTION_TYPE returns arrayref');
    is(scalar @$types, 1, 'one type entry');

    my $exports = get_section($mod, SECTION_EXPORT);
    ok(ref($exports) eq 'ARRAY', 'get_section SECTION_EXPORT returns arrayref');
    is(scalar @$exports, 0, 'no exports');

    my $unknown = get_section($mod, 99);
    is($unknown, undef, 'unknown section id returns undef');
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'error handling' => sub {
    # Wrong magic
    ok(dies { parse("NOPE\x01\x00\x00\x00") }, 'dies on wrong magic bytes');

    # Wrong version
    ok(dies { parse("\x00asm\x02\x00\x00\x00") }, 'dies on wrong version');

    # Too short
    ok(dies { parse("\x00asm") }, 'dies on truncated input (only 4 bytes)');

    # Empty string
    ok(dies { parse("") }, 'dies on empty input');
};

# ============================================================================
# Combined: type + function + export + code sections
# ============================================================================

subtest 'combined module' => sub {
    # Type section: () → i32
    my $type_sec = section_wrap(1,
        leb_u(1) . b(0x60) . leb_u(0) . leb_u(1) . b(0x7F)
    );

    # Function section: one function, type 0
    my $func_sec = section_wrap(3, leb_u(1) . leb_u(0));

    # Export section: "answer" → func 0
    my $exp_sec = section_wrap(7,
        leb_u(1) . str_field("answer") . b(0x00) . leb_u(0)
    );

    # Code section: one body, no locals, i32.const 42 (0x41 0x2A), end (0x0B)
    my $body = b(0x00, 0x41, 0x2A, 0x0B);  # locals=0, i32.const 42, end
    my $code_sec = section_wrap(10, leb_u(1) . leb_u(length($body)) . $body);

    my $wasm = $WASM_HEADER . $type_sec . $func_sec . $exp_sec . $code_sec;
    my $mod  = parse($wasm);

    is(scalar @{ $mod->{types} },     1, 'one type');
    is(scalar @{ $mod->{functions} }, 1, 'one function');
    is(scalar @{ $mod->{exports} },   1, 'one export');
    is(scalar @{ $mod->{codes} },     1, 'one code entry');

    is($mod->{types}[0]{results}[0], 0x7F,     'type result is i32');
    is($mod->{functions}[0],         0,         'function uses type 0');
    is($mod->{exports}[0]{name},     'answer',  'export name is "answer"');
    is($mod->{exports}[0]{desc}{kind}, 'func',  'export kind is func');

    # Code body: [0x41 (i32.const), 0x2A (42), 0x0B (end)]
    is($mod->{codes}[0]{body}[0], 0x41, 'body[0]=i32.const');
    is($mod->{codes}[0]{body}[1], 0x2A, 'body[1]=42');
    is($mod->{codes}[0]{body}[2], 0x0B, 'body[2]=end');
};

done_testing;
