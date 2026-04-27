use strict;
use warnings;
use Test2::V0;

use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';
use lib '../wasm-validator/lib';
use lib '../wasm-execution/lib';

use CodingAdventures::WasmRuntime;

# ============================================================================
# Helper: LEB128 encoding
# ============================================================================
#
# WASM uses LEB128 (Little Endian Base 128) for variable-length integers.
# Each byte encodes 7 bits; bit 7 indicates whether more bytes follow.

sub _leb128_unsigned {
    my ($n) = @_;
    my @bytes;
    do {
        my $byte = $n & 0x7F;
        $n >>= 7;
        $byte |= 0x80 if $n > 0;
        push @bytes, $byte;
    } while ($n > 0);
    return @bytes;
}

# ============================================================================
# Helper: build a WASM section
# ============================================================================
#
# Each WASM section is: section_id (1 byte) + size (LEB128) + payload.

sub _build_section {
    my ($section_id, @payload) = @_;
    my $len = scalar(@payload);
    return ($section_id, _leb128_unsigned($len), @payload);
}

# ============================================================================
# Helper: build the square.wasm module
# ============================================================================
#
# This hand-assembles a WASM binary that implements:
#
#   (module
#     (type (func (param i32) (result i32)))
#     (func (type 0)
#       local.get 0
#       local.get 0
#       i32.mul)
#     (export "square" (func 0)))
#
# The WASM binary format has these sections in order:
#   magic + version (8 bytes)
#   Type section (id=1): one function type (i32) -> (i32)
#   Function section (id=3): function 0 uses type 0
#   Export section (id=7): "square" -> function 0
#   Code section (id=10): one function body

sub _build_square_wasm {
    my @header = (0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);

    # Type section: 1 type entry: (i32) -> (i32)
    my @type_payload = (
        _leb128_unsigned(1),  # count = 1
        0x60,                  # func type tag
        0x01, 0x7F,           # params: 1 x i32
        0x01, 0x7F,           # results: 1 x i32
    );
    my @type_section = _build_section(1, @type_payload);

    # Function section: 1 function using type 0
    my @func_payload = (_leb128_unsigned(1), _leb128_unsigned(0));
    my @func_section = _build_section(3, @func_payload);

    # Export section: "square" -> func 0
    my @export_name = map { ord($_) } split //, 'square';
    my @export_payload = (
        _leb128_unsigned(1),                    # count = 1
        _leb128_unsigned(scalar @export_name),  # name length
        @export_name,                           # name bytes
        0x00,                                   # kind = function
        _leb128_unsigned(0),                    # function index
    );
    my @export_section = _build_section(7, @export_payload);

    # Code section: 1 body
    my @body_code = (
        0x20, 0x00,  # local.get 0
        0x20, 0x00,  # local.get 0
        0x6C,        # i32.mul
        0x0B,        # end
    );
    # Body = local_count (0) + code
    my @body = (_leb128_unsigned(0), @body_code);
    my @body_with_size = (_leb128_unsigned(scalar @body), @body);
    my @code_payload = (_leb128_unsigned(1), @body_with_size);
    my @code_section = _build_section(10, @code_payload);

    return pack('C*', @header, @type_section, @func_section,
                @export_section, @code_section);
}

# ============================================================================
# Helper: build a simple add.wasm module
# ============================================================================
#
#   (module
#     (type (func (param i32 i32) (result i32)))
#     (func (type 0)
#       local.get 0
#       local.get 1
#       i32.add)
#     (export "add" (func 0)))

sub _build_add_wasm {
    my @header = (0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);

    my @type_payload = (
        _leb128_unsigned(1),
        0x60,
        0x02, 0x7F, 0x7F,    # params: 2 x i32
        0x01, 0x7F,           # results: 1 x i32
    );
    my @type_section = _build_section(1, @type_payload);

    my @func_payload = (_leb128_unsigned(1), _leb128_unsigned(0));
    my @func_section = _build_section(3, @func_payload);

    my @export_name = map { ord($_) } split //, 'add';
    my @export_payload = (
        _leb128_unsigned(1),
        _leb128_unsigned(scalar @export_name),
        @export_name,
        0x00,
        _leb128_unsigned(0),
    );
    my @export_section = _build_section(7, @export_payload);

    my @body_code = (
        0x20, 0x00,  # local.get 0
        0x20, 0x01,  # local.get 1
        0x6A,        # i32.add
        0x0B,        # end
    );
    my @body = (_leb128_unsigned(0), @body_code);
    my @body_with_size = (_leb128_unsigned(scalar @body), @body);
    my @code_payload = (_leb128_unsigned(1), @body_with_size);
    my @code_section = _build_section(10, @code_payload);

    return pack('C*', @header, @type_section, @func_section,
                @export_section, @code_section);
}

# _memory_module_with_data_section(\@payload) — build the smallest parsed module
# shape that reaches instantiate()'s data-segment application path.  The parser
# stores data sections as raw byte arrays, so this helper lets security tests
# exercise malformed raw sections without requiring a full binary fixture.
sub _memory_module_with_data_section {
    my ($payload) = @_;
    return {
        types     => [],
        imports   => [],
        functions => [],
        codes     => [],
        tables    => [],
        memories  => [{ limits => { min => 1, max => undef } }],
        globals   => [],
        exports   => [],
        elements  => [],
        data      => [$payload],
    };
}

# ============================================================================
# WasmRuntime API tests
# ============================================================================

subtest 'new() creates a runtime' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    ok($rt, 'runtime created');
};

subtest 'load() parses WASM bytes' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $wasm = _build_square_wasm();
    my $module = $rt->load($wasm);
    ok($module, 'module parsed');
    ok($module->{types}, 'module has types');
    is(scalar(@{ $module->{types} }), 1, 'one type');
    is(scalar(@{ $module->{functions} }), 1, 'one function');
    is(scalar(@{ $module->{exports} }), 1, 'one export');
    is($module->{exports}[0]{name}, 'square', 'export name is "square"');
};

subtest 'validate() validates a parsed module' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $wasm = _build_square_wasm();
    my $module = $rt->load($wasm);
    my $validated = $rt->validate($module);
    ok($validated, 'validation passes');
    ok($validated->{func_types}, 'func_types populated');
};

subtest 'instantiate() creates an instance' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $wasm = _build_square_wasm();
    my $module = $rt->load($wasm);
    my $instance = $rt->instantiate($module);
    ok($instance, 'instance created');
    ok($instance->{exports}, 'instance has exports');
    ok($instance->{exports}{square}, 'instance has "square" export');
    is($instance->{exports}{square}{kind}, 'func', 'square is a function');
};

subtest 'instantiate() rejects malformed raw data segments before slicing payloads' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();

    # Data section payload:
    #   count=1, memory_index=0, offset=(i32.const 0; end), size=32
    # The actual data bytes are intentionally absent.  Before this guard, the
    # runtime created the slice [$pos .. $pos + size - 1] even when the payload
    # was truncated, letting malformed modules force large temporary lists.
    my @truncated = (
        _leb128_unsigned(1),
        _leb128_unsigned(0),
        0x41, 0x00, 0x0B,
        _leb128_unsigned(32),
    );
    my $truncated_error;
    eval {
        $rt->instantiate(_memory_module_with_data_section(\@truncated));
        1;
    } or $truncated_error = $@;
    like(
        "$truncated_error",
        qr/data segment payload shorter than declared size/,
        'truncated raw data payloads fail closed',
    );

    my @oversized = (
        _leb128_unsigned(1),
        _leb128_unsigned(0),
        0x41, 0x00, 0x0B,
        _leb128_unsigned(16 * 1024 * 1024 + 1),
    );
    my $oversized_error;
    eval {
        $rt->instantiate(_memory_module_with_data_section(\@oversized));
        1;
    } or $oversized_error = $@;
    like(
        "$oversized_error",
        qr/data segment size exceeds limit/,
        'oversized raw data payload declarations fail closed',
    );
};

subtest 'call() invokes an exported function' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $wasm = _build_square_wasm();
    my $module = $rt->load($wasm);
    my $instance = $rt->instantiate($module);
    my $result = $rt->call($instance, 'square', [5]);
    is($result, [25], 'square(5) = 25');
};

# ============================================================================
# End-to-end: square function
# ============================================================================

subtest 'square(5) = 25' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $wasm = _build_square_wasm();
    my $result = $rt->load_and_run($wasm, 'square', [5]);
    is($result, [25], 'square(5) = 25');
};

subtest 'square(0) = 0' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_square_wasm(), 'square', [0]);
    is($result, [0], 'square(0) = 0');
};

subtest 'square(-3) = 9' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_square_wasm(), 'square', [-3]);
    is($result, [9], 'square(-3) = 9');
};

subtest 'square(1) = 1' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_square_wasm(), 'square', [1]);
    is($result, [1], 'square(1) = 1');
};

subtest 'square(2147483647) wraps to 1 in i32' => sub {
    # 2147483647^2 mod 2^32 = 1 (as signed i32)
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_square_wasm(), 'square', [2147483647]);
    is($result, [1], 'square(MAX_INT) wraps to 1');
};

# ============================================================================
# End-to-end: add function
# ============================================================================

subtest 'add(3, 4) = 7' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_add_wasm(), 'add', [3, 4]);
    is($result, [7], 'add(3, 4) = 7');
};

subtest 'add(0, 0) = 0' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_add_wasm(), 'add', [0, 0]);
    is($result, [0], 'add(0, 0) = 0');
};

subtest 'add(-10, 10) = 0' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_add_wasm(), 'add', [-10, 10]);
    is($result, [0], 'add(-10, 10) = 0');
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'call() traps on unknown export' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $instance = $rt->instantiate($rt->load(_build_square_wasm()));
    my $died = 0;
    eval { $rt->call($instance, 'nonexistent', [5]) };
    $died = 1 if $@;
    ok($died, 'calling unknown export traps');
};

subtest 'load_and_run convenience method works' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run(_build_square_wasm(), 'square', [7]);
    is($result, [49], 'load_and_run: square(7) = 49');
};

# ============================================================================
# WasiStub
# ============================================================================

subtest 'WasiStub resolves proc_exit' => sub {
    my $stub = CodingAdventures::WasmRuntime::WasiStub->new();
    my $fn = $stub->resolve_function('wasi_snapshot_preview1', 'proc_exit');
    ok($fn, 'proc_exit resolved');
    ok(ref($fn) eq 'CODE', 'proc_exit is a coderef');
};

subtest 'WasiStub resolves fd_write' => sub {
    my $stub = CodingAdventures::WasmRuntime::WasiStub->new();
    my $fn = $stub->resolve_function('wasi_snapshot_preview1', 'fd_write');
    ok($fn, 'fd_write resolved');
};

subtest 'WasiHost aliases WasiStub' => sub {
    my $host = CodingAdventures::WasmRuntime::WasiHost->new();
    isa_ok($host, 'CodingAdventures::WasmRuntime::WasiStub');
    my $fn = $host->resolve_function('wasi_snapshot_preview1', 'fd_read');
    ok($fn, 'fd_read resolved through WasiHost alias');
};

subtest 'WasiStub returns undef for unknown functions' => sub {
    my $stub = CodingAdventures::WasmRuntime::WasiStub->new();
    my $fn = $stub->resolve_function('wasi_snapshot_preview1', 'unknown');
    ok(!defined($fn), 'unknown function returns undef');
};

subtest 'WasiStub returns undef for unknown modules' => sub {
    my $stub = CodingAdventures::WasmRuntime::WasiStub->new();
    my $fn = $stub->resolve_function('env', 'proc_exit');
    ok(!defined($fn), 'non-wasi module returns undef');
};

done_testing;
