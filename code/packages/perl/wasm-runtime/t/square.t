use strict;
use warnings;
use Test2::V0;

# ============================================================================
# End-to-end test: square(n) = n * n
# ============================================================================
#
# This test hand-assembles a WASM module that exports a "square" function,
# then runs it through the full pipeline:
#
#   .wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Call
#
# The WAT (WebAssembly Text format) equivalent is:
#
#   (module
#     (type (func (param i32) (result i32)))
#     (func (type 0) (param i32) (result i32)
#       local.get 0
#       local.get 0
#       i32.mul)
#     (export "square" (func 0)))
#
# This is the "hello world" of WASM execution: if square(5)=25 works, it
# proves the entire pipeline is correct — parsing, validation, instantiation,
# bytecode decoding, instruction dispatch, typed stack operations, and
# function calling all work together.
#
# ============================================================================

use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';
use lib '../wasm-validator/lib';
use lib '../wasm-execution/lib';

use CodingAdventures::WasmRuntime;

# ============================================================================
# LEB128 encoder — variable-length integer encoding used by WASM
# ============================================================================
#
# LEB128 (Little Endian Base 128) encodes integers using 7 bits per byte,
# with bit 7 indicating whether more bytes follow.
#
# Examples:
#   0     -> [0x00]          (1 byte)
#   127   -> [0x7F]          (1 byte, max single-byte value)
#   128   -> [0x80, 0x01]    (2 bytes)
#   624485 -> [0xE5, 0x8E, 0x26]  (3 bytes)

sub _leb128 {
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
# Section builder
# ============================================================================
#
# Every WASM section follows the same layout:
#   section_id (1 byte)  |  payload_length (LEB128)  |  payload (N bytes)

sub _section {
    my ($id, @payload) = @_;
    return ($id, _leb128(scalar @payload), @payload);
}

# ============================================================================
# Build the square.wasm binary
# ============================================================================
#
# A WASM module consists of:
#   1. Header: magic number (\0asm) + version (1)
#   2. Sections: type, function, export, code
#
# Each section contains exactly the data needed:
#
#   Type section (id=1):
#     1 type: (i32) -> (i32)
#
#   Function section (id=3):
#     1 function, referencing type 0
#
#   Export section (id=7):
#     1 export: "square" -> function 0
#
#   Code section (id=10):
#     1 function body:
#       0 local declarations
#       local.get 0    (push param onto stack)
#       local.get 0    (push param again)
#       i32.mul        (pop both, push product)
#       end            (return the top of stack)

sub _build_square_wasm {
    # Magic number (\0asm) + version 1 (little-endian 32-bit)
    my @header = (0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);

    # Type section: 1 function signature, (i32) -> (i32)
    my @type_section = _section(1,
        _leb128(1),          # 1 type entry
        0x60,                # "func" type constructor
        0x01, 0x7F,          # 1 param, type i32
        0x01, 0x7F,          # 1 result, type i32
    );

    # Function section: 1 function using type index 0
    my @func_section = _section(3, _leb128(1), _leb128(0));

    # Export section: "square" mapped to function index 0
    my @name = map { ord($_) } split //, 'square';
    my @export_section = _section(7,
        _leb128(1),               # 1 export
        _leb128(scalar @name),    # name length
        @name,                    # name bytes
        0x00,                     # export kind: function
        _leb128(0),               # function index
    );

    # Code section: 1 function body
    #   Body = local_count + instructions
    #   The body is prefixed with its byte length.
    my @instructions = (
        0x20, 0x00,   # local.get 0 — push the parameter
        0x20, 0x00,   # local.get 0 — push it again
        0x6C,         # i32.mul     — multiply them
        0x0B,         # end         — return the result
    );
    my @body = (_leb128(0), @instructions);  # 0 local declarations
    my @code_section = _section(10,
        _leb128(1),                   # 1 function body
        _leb128(scalar @body), @body, # body with size prefix
    );

    return pack('C*', @header, @type_section, @func_section,
                @export_section, @code_section);
}

# ============================================================================
# Tests
# ============================================================================

my $wasm = _build_square_wasm();

subtest 'square(5) = 25 — the canonical WASM test' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [5]);
    is($result, [25], 'square(5) = 25');
};

subtest 'square(0) = 0 — zero times zero' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [0]);
    is($result, [0], 'square(0) = 0');
};

subtest 'square(1) = 1 — identity' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [1]);
    is($result, [1], 'square(1) = 1');
};

subtest 'square(-1) = 1 — negative times negative' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [-1]);
    is($result, [1], 'square(-1) = 1');
};

subtest 'square(-3) = 9 — negative input' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [-3]);
    is($result, [9], 'square(-3) = 9');
};

subtest 'square(10) = 100' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [10]);
    is($result, [100], 'square(10) = 100');
};

subtest 'square(256) = 65536' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [256]);
    is($result, [65536], 'square(256) = 65536');
};

subtest 'square(2147483647) wraps to 1 in i32 arithmetic' => sub {
    # 2147483647 is the maximum signed 32-bit integer (2^31 - 1).
    # 2147483647^2 = 4611686014132420609
    # In i32 arithmetic (mod 2^32): 4611686014132420609 mod 4294967296 = 1
    # Signed interpretation of 1 is just 1.
    my $rt = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [2147483647]);
    is($result, [1], 'square(MAX_INT) wraps to 1 due to i32 overflow');
};

subtest 'step-by-step: load, validate, instantiate, call' => sub {
    my $rt = CodingAdventures::WasmRuntime->new();

    # Step 1: Parse
    my $module = $rt->load($wasm);
    ok($module, 'module parsed successfully');

    # Step 2: Validate
    my $validated = $rt->validate($module);
    ok($validated, 'module validates successfully');
    is(scalar(@{ $validated->{func_types} }), 1, 'one function type resolved');

    # Step 3: Instantiate
    my $instance = $rt->instantiate($module);
    ok($instance, 'instance created');
    ok($instance->{exports}{square}, 'square export exists');

    # Step 4: Call
    my $result = $rt->call($instance, 'square', [5]);
    is($result, [25], 'square(5) = 25 via step-by-step API');

    # Call again with different argument (engine reuse)
    $result = $rt->call($instance, 'square', [7]);
    is($result, [49], 'square(7) = 49 via step-by-step API');
};

done_testing;
