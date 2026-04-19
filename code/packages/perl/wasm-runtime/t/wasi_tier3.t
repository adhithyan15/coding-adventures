use strict;
use warnings;
use Test2::V0;

# ============================================================================
# WASI Tier 3 tests
# ============================================================================
#
# This file tests the eight new WASI host functions added in Tier 3:
#
#   args_sizes_get    — report argc and total argv buffer size
#   args_get          — fill argv pointer array and argv_buf
#   environ_sizes_get — report environment variable count and buffer size
#   environ_get       — fill environ pointer array and environ_buf
#   clock_time_get    — read the current time for a given clock ID
#   clock_res_get     — read the resolution of a given clock
#   random_get        — fill a buffer with random bytes
#   sched_yield       — no-op scheduling hint
#
# ## Test strategy: fake clock and random
#
# Rather than depending on the real system clock (which would make tests
# non-deterministic), we inject FakeClock and FakeRandom objects. This is
# the "dependency injection" / "duck-typing" design baked into WasiStub.
#
# FakeClock always returns the same nanosecond values, so we can compare
# exact expected bytes in memory. FakeRandom always returns 0xAB bytes,
# so we can assert the exact memory content.
#
# ## Test strategy: direct function call
#
# We resolve each WASI function from WasiStub, create a LinearMemory, call
# set_memory() on the stub, then invoke the resolved coderef directly with
# synthetic WasmValue arguments. This exercises the functions without needing
# a full WASM module (no parser, validator, or bytecode interpreter involved).
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
use CodingAdventures::WasmExecution qw(i32 i64);

# ============================================================================
# Fake clock — deterministic time values for testing
# ============================================================================
#
# realtime_ns returns a large nanosecond timestamp:
#   1_700_000_000_000_000_001
#
# This value exercises the full 64-bit range. Let's verify it doesn't fit
# in a 32-bit integer:
#   2^32 = 4_294_967_296
#   1_700_000_000_000_000_001 >> 32 = 395_881_629  (non-zero → uses high word)
#
# monotonic_ns returns 42_000_000_000 (42 seconds in nanoseconds).
#   42_000_000_000 >> 32 = 9  (non-zero → exercises the high word path too)

package FakeClock;
sub new { bless {}, shift }
sub realtime_ns  { return 1_700_000_000_000_000_001 }
sub monotonic_ns { return 42_000_000_000 }
sub resolution_ns { return 1_000_000 }

# ============================================================================
# Fake random — predictable bytes for testing
# ============================================================================
#
# fill_bytes($n) always returns an arrayref of $n copies of 0xAB (171 decimal).
# This lets us assert that exactly those bytes appear in memory after random_get.

package FakeRandom;
sub new { bless {}, shift }
sub fill_bytes {
    my ($self, $n) = @_;
    return [ (0xAB) x $n ];
}

package ExplodingRandom;
sub new { bless {}, shift }
sub fill_bytes { die 'random source should not be called for invalid requests' }

# ============================================================================
# Test helpers
# ============================================================================

package main;

# _make_stub(%opts) — create a WasiStub with fake clock/random, plus a 1-page
# LinearMemory, wired together via set_memory(). Returns ($stub, $mem).
#
# Having a 1-page (65536 byte) memory is plenty for all our test pointers,
# which all live in the first few hundred bytes.
sub _make_stub {
    my (%opts) = @_;

    my $stub = CodingAdventures::WasmRuntime::WasiStub->new(
        args   => $opts{args}   // [],
        env    => $opts{env}    // {},
        clock  => FakeClock->new(),
        random => $opts{random} // FakeRandom->new(),
    );

    # Create one 64 KiB page of memory (minimum WASM page size).
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    $stub->set_memory($mem);

    return ($stub, $mem);
}

# _call($stub, $name, \@wasm_args) — resolve and immediately invoke a WASI fn.
#
# Returns the arrayref of WasmValue results (same as the execution engine would).
sub _call {
    my ($stub, $name, $wasm_args) = @_;
    my $fn = $stub->resolve_function('wasi_snapshot_preview1', $name);
    die "no function '$name'" unless $fn;
    return $fn->($wasm_args);
}

# _read_i64_le($mem, $offset) — read a little-endian i64 from two i32 words.
#
# WASI writes 64-bit timestamps as two consecutive i32 values:
#   bytes [offset+0 .. offset+3] = low  32 bits
#   bytes [offset+4 .. offset+7] = high 32 bits
#
# We reconstruct the full 64-bit value:  hi * 2^32 + lo
sub _read_i64_le {
    my ($mem, $offset) = @_;
    my $lo = $mem->load_i32($offset);
    my $hi = $mem->load_i32($offset + 4);
    # Use unsigned arithmetic: mask lo to 32 bits (load_i32 may return signed).
    return ($hi & 0xFFFFFFFF) * 4_294_967_296 + ($lo & 0xFFFFFFFF);
}

# ============================================================================
# Test 1: args_sizes_get — ["myapp", "hello"]
# ============================================================================
#
# args = ["myapp", "hello"]
#   "myapp" → 5 UTF-8 bytes + NUL = 6 bytes
#   "hello" → 5 UTF-8 bytes + NUL = 6 bytes
#   Total buf_size = 12
#   argc = 2

subtest 'args_sizes_get: argc=2, buf_size=12' => sub {
    my ($stub, $mem) = _make_stub(args => ['myapp', 'hello']);

    # We place the two output pointers at offsets 0 and 4 in memory.
    my $argc_ptr     = 0;
    my $buf_size_ptr = 4;

    my $result = _call($stub, 'args_sizes_get', [
        i32($argc_ptr),
        i32($buf_size_ptr),
    ]);

    is($result->[0]{value}, 0, 'args_sizes_get returns ESUCCESS (0)');
    is($mem->load_i32($argc_ptr),     2,  'argc = 2');
    is($mem->load_i32($buf_size_ptr), 12, 'buf_size = 12 ("myapp\0hello\0")');
};

# ============================================================================
# Test 2: args_get — verify argv pointers and argv_buf content
# ============================================================================
#
# Layout after args_get(argv_ptr=100, argv_buf_ptr=200):
#
#   Memory[200] = 'm','y','a','p','p', 0
#   Memory[206] = 'h','e','l','l','o', 0
#
#   Memory[100] = 200  (pointer to "myapp\0")
#   Memory[104] = 206  (pointer to "hello\0")
#
# We verify the pointer values (as i32) and that the bytes at those addresses
# match the expected ASCII codes + NUL.

subtest 'args_get: fills argv pointers and argv_buf correctly' => sub {
    my ($stub, $mem) = _make_stub(args => ['myapp', 'hello']);

    my $argv_ptr     = 100;
    my $argv_buf_ptr = 200;

    my $result = _call($stub, 'args_get', [
        i32($argv_ptr),
        i32($argv_buf_ptr),
    ]);

    is($result->[0]{value}, 0, 'args_get returns ESUCCESS (0)');

    # argv[0] should point to argv_buf_ptr (= 200)
    is($mem->load_i32($argv_ptr),     200, 'argv[0] = 200 (start of argv_buf)');
    # argv[1] should point to 200 + 6 = 206 (after "myapp\0")
    is($mem->load_i32($argv_ptr + 4), 206, 'argv[1] = 206 (after "myapp\\0")');

    # Check the bytes of "myapp\0" at offset 200
    my @myapp_bytes = (ord('m'), ord('y'), ord('a'), ord('p'), ord('p'), 0);
    for my $i (0 .. $#myapp_bytes) {
        is($mem->load_i32_8u($argv_buf_ptr + $i), $myapp_bytes[$i],
            "argv_buf[$i] = $myapp_bytes[$i]");
    }

    # Check the bytes of "hello\0" at offset 206
    my @hello_bytes = (ord('h'), ord('e'), ord('l'), ord('l'), ord('o'), 0);
    for my $i (0 .. $#hello_bytes) {
        is($mem->load_i32_8u($argv_buf_ptr + 6 + $i), $hello_bytes[$i],
            "argv_buf[" . (6+$i) . "] = $hello_bytes[$i]");
    }
};

# ============================================================================
# Test 3: environ_sizes_get — {HOME => "/home/user"}
# ============================================================================
#
# env = {HOME => "/home/user"}
#   "HOME=/home/user" → 15 UTF-8 bytes + NUL = 16 bytes
#   count = 1, buf_size = 16

subtest 'environ_sizes_get: count=1, buf_size=16' => sub {
    my ($stub, $mem) = _make_stub(env => { HOME => '/home/user' });

    my $count_ptr    = 8;
    my $buf_size_ptr = 12;

    my $result = _call($stub, 'environ_sizes_get', [
        i32($count_ptr),
        i32($buf_size_ptr),
    ]);

    is($result->[0]{value}, 0,  'environ_sizes_get returns ESUCCESS (0)');
    is($mem->load_i32($count_ptr),    1,  'count = 1');
    is($mem->load_i32($buf_size_ptr), 16, 'buf_size = 16 ("HOME=/home/user\\0")');
};

# ============================================================================
# Test 4: environ_get — verify environ pointers and environ_buf content
# ============================================================================
#
# Layout after environ_get(environ_ptr=300, environ_buf_ptr=400):
#
#   Memory[400] = 'H','O','M','E','=','/','h','o','m','e','/','u','s','e','r', 0
#   Memory[300] = 400  (pointer to "HOME=/home/user\0")

subtest 'environ_get: fills environ pointers and environ_buf correctly' => sub {
    my ($stub, $mem) = _make_stub(env => { HOME => '/home/user' });

    my $environ_ptr     = 300;
    my $environ_buf_ptr = 400;

    my $result = _call($stub, 'environ_get', [
        i32($environ_ptr),
        i32($environ_buf_ptr),
    ]);

    is($result->[0]{value}, 0, 'environ_get returns ESUCCESS (0)');

    # environ[0] should point to environ_buf_ptr (= 400)
    is($mem->load_i32($environ_ptr), 400, 'environ[0] = 400');

    # Check the "HOME=/home/user\0" bytes
    my $entry = 'HOME=/home/user';
    my @entry_bytes = (map { ord($_) } split //, $entry);
    push @entry_bytes, 0;  # NUL terminator

    for my $i (0 .. $#entry_bytes) {
        is($mem->load_i32_8u($environ_buf_ptr + $i), $entry_bytes[$i],
            "environ_buf[$i] = $entry_bytes[$i]");
    }
};

# ============================================================================
# Test 5: clock_time_get(id=0) — realtime clock
# ============================================================================
#
# FakeClock::realtime_ns returns 1_700_000_000_000_000_001.
#
# Let's verify the lo/hi split:
#   1_700_000_000_000_000_001 in hex = 0x_178C_29AE_9AB1_0001
#   lo = 0x9AB1_0001 = 2_595_848_193
#   hi = 0x178C_29AE =   395_881_390  ... wait, let's recompute.
#
# Actually: 1_700_000_000_000_000_001
#   / 2^32 (= 4_294_967_296):
#   1_700_000_000_000_000_001 / 4_294_967_296 = 395_816_864 remainder ...
#
# We don't need to compute this by hand — the test reads back the i64 using
# _read_i64_le and compares to the original value.

subtest 'clock_time_get(id=0): realtime → 1_700_000_000_000_000_001 ns' => sub {
    my ($stub, $mem) = _make_stub();

    my $time_ptr = 500;
    # clock_time_get(id=0, precision=0, time_ptr)
    # precision is i64 → needs two i32 args in the wasm args list?
    # Actually in WASM, i64 is a single value. Our WasmExecution::i64() creates
    # a WasmValue with type i64. The args array just has one slot for precision.
    my $result = _call($stub, 'clock_time_get', [
        i32(0),        # clock id = CLOCK_REALTIME
        i64(0),        # precision (ignored)
        i32($time_ptr),
    ]);

    is($result->[0]{value}, 0, 'clock_time_get(0) returns ESUCCESS (0)');

    my $ns = _read_i64_le($mem, $time_ptr);
    is($ns, 1_700_000_000_000_000_001,
        'realtime timestamp = 1_700_000_000_000_000_001 ns');
};

# ============================================================================
# Test 6: clock_time_get(id=1) — monotonic clock
# ============================================================================
#
# FakeClock::monotonic_ns returns 42_000_000_000 (42 seconds in ns).

subtest 'clock_time_get(id=1): monotonic → 42_000_000_000 ns' => sub {
    my ($stub, $mem) = _make_stub();

    my $time_ptr = 520;
    my $result = _call($stub, 'clock_time_get', [
        i32(1),        # clock id = CLOCK_MONOTONIC
        i64(0),        # precision
        i32($time_ptr),
    ]);

    is($result->[0]{value}, 0, 'clock_time_get(1) returns ESUCCESS (0)');

    my $ns = _read_i64_le($mem, $time_ptr);
    is($ns, 42_000_000_000, 'monotonic timestamp = 42_000_000_000 ns');
};

# ============================================================================
# Test 7: clock_res_get(id=0) — resolution = 1_000_000 ns (1 ms)
# ============================================================================
#
# FakeClock::resolution_ns returns 1_000_000.
#   lo = 1_000_000 & 0xFFFFFFFF = 1_000_000
#   hi = 0 (fits in 32 bits)

subtest 'clock_res_get(id=0): resolution = 1_000_000 ns' => sub {
    my ($stub, $mem) = _make_stub();

    my $res_ptr = 540;
    my $result = _call($stub, 'clock_res_get', [
        i32(0),         # clock id
        i32($res_ptr),
    ]);

    is($result->[0]{value}, 0, 'clock_res_get(0) returns ESUCCESS (0)');

    my $ns = _read_i64_le($mem, $res_ptr);
    is($ns, 1_000_000, 'resolution = 1_000_000 ns (1 ms)');
};

# ============================================================================
# Test 8: random_get — 4 bytes of 0xAB
# ============================================================================
#
# FakeRandom::fill_bytes(4) returns [0xAB, 0xAB, 0xAB, 0xAB].
# We place the buffer at offset 600 and check all 4 bytes.

subtest 'random_get: fills 4 bytes with 0xAB' => sub {
    my ($stub, $mem) = _make_stub();

    my $buf_ptr = 600;
    my $buf_len = 4;

    my $result = _call($stub, 'random_get', [
        i32($buf_ptr),
        i32($buf_len),
    ]);

    is($result->[0]{value}, 0, 'random_get returns ESUCCESS (0)');

    for my $i (0 .. $buf_len - 1) {
        is($mem->load_i32_8u($buf_ptr + $i), 0xAB,
            "random byte[$i] = 0xAB");
    }
};

subtest 'random_get: rejects oversized buffers before requesting entropy' => sub {
    my ($stub, $mem) = _make_stub(random => ExplodingRandom->new());

    my $result = _call($stub, 'random_get', [
        i32(0),
        i32(1024 * 1024 + 1),
    ]);

    is($result->[0]{value}, 28, 'oversized random_get returns EINVAL');
};

# ============================================================================
# Test 9: sched_yield — always returns 0
# ============================================================================
#
# sched_yield() takes no arguments and always returns ESUCCESS. It is a
# scheduling hint that our single-threaded runtime can safely ignore.

subtest 'sched_yield: returns ESUCCESS (0)' => sub {
    my ($stub, $mem) = _make_stub();

    my $result = _call($stub, 'sched_yield', []);

    is($result->[0]{value}, 0, 'sched_yield returns ESUCCESS (0)');
};

# ============================================================================
# Test 10: clock_time_get with unknown clock id → EINVAL (28)
# ============================================================================

subtest 'clock_time_get: unknown id → EINVAL (28)' => sub {
    my ($stub, $mem) = _make_stub();

    my $result = _call($stub, 'clock_time_get', [
        i32(99),       # unknown clock id
        i64(0),
        i32(700),
    ]);

    is($result->[0]{value}, 28, 'unknown clock id returns EINVAL (28)');
};

# ============================================================================
# Test 11: args with empty argv — argc=0, buf_size=0
# ============================================================================

subtest 'args_sizes_get: empty args → argc=0, buf_size=0' => sub {
    my ($stub, $mem) = _make_stub(args => []);

    my $result = _call($stub, 'args_sizes_get', [i32(0), i32(4)]);

    is($result->[0]{value}, 0, 'returns ESUCCESS');
    is($mem->load_i32(0), 0, 'argc = 0');
    is($mem->load_i32(4), 0, 'buf_size = 0');
};

# ============================================================================
# Test 12: environ with empty env — count=0, buf_size=0
# ============================================================================

subtest 'environ_sizes_get: empty env → count=0, buf_size=0' => sub {
    my ($stub, $mem) = _make_stub(env => {});

    my $result = _call($stub, 'environ_sizes_get', [i32(0), i32(4)]);

    is($result->[0]{value}, 0, 'returns ESUCCESS');
    is($mem->load_i32(0), 0, 'count = 0');
    is($mem->load_i32(4), 0, 'buf_size = 0');
};

# ============================================================================
# Test 13: clock_time_get(id=2) — process clock maps to realtime
# ============================================================================

subtest 'clock_time_get(id=2): process clock → realtime' => sub {
    my ($stub, $mem) = _make_stub();

    my $time_ptr = 800;
    my $result = _call($stub, 'clock_time_get', [
        i32(2),
        i64(0),
        i32($time_ptr),
    ]);

    is($result->[0]{value}, 0, 'returns ESUCCESS');
    my $ns = _read_i64_le($mem, $time_ptr);
    is($ns, 1_700_000_000_000_000_001, 'process clock = realtime');
};

# ============================================================================
# Test 14: fd_read copies stdin bytes into guest buffers
# ============================================================================

subtest 'fd_read: copies stdin bytes into guest buffers' => sub {
    my $stub = CodingAdventures::WasmRuntime::WasiHost->new(
        stdin  => sub { return 'hi' },
        clock  => FakeClock->new(),
        random => FakeRandom->new(),
    );
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    $stub->set_memory($mem);

    $mem->store_i32(0, 200);
    $mem->store_i32(4, 2);

    my $result = _call($stub, 'fd_read', [
        i32(0),
        i32(0),
        i32(1),
        i32(100),
    ]);

    is($result->[0]{value}, 0, 'fd_read returns ESUCCESS');
    is($mem->load_i32(100), 2, 'nread = 2');
    is($mem->load_i32_8u(200), ord('h'), 'first byte copied');
    is($mem->load_i32_8u(201), ord('i'), 'second byte copied');
};

subtest 'fd_write: rejects oversized iovec counts before copying guest data' => sub {
    my @output;
    my $stub = CodingAdventures::WasmRuntime::WasiHost->new(
        stdout => sub { push @output, $_[0] },
        clock  => FakeClock->new(),
        random => FakeRandom->new(),
    );
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    $stub->set_memory($mem);

    my $result = _call($stub, 'fd_write', [
        i32(1),
        i32(0),
        i32(1025),
        i32(100),
    ]);

    is($result->[0]{value}, 28, 'fd_write returns EINVAL for oversized iovec counts');
    is(\@output, [], 'stdout callback is not invoked');
};

subtest 'fd_read: rejects oversized iovec counts before requesting stdin' => sub {
    my $read_calls = 0;
    my $stub = CodingAdventures::WasmRuntime::WasiHost->new(
        stdin  => sub { $read_calls++; return 'blocked'; },
        clock  => FakeClock->new(),
        random => FakeRandom->new(),
    );
    my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
    $stub->set_memory($mem);

    my $result = _call($stub, 'fd_read', [
        i32(0),
        i32(0),
        i32(1025),
        i32(100),
    ]);

    is($result->[0]{value}, 28, 'fd_read returns EINVAL for oversized iovec counts');
    is($read_calls, 0, 'stdin callback is not invoked');
};

# ============================================================================
# Test 15: existing square test still passes (regression)
# ============================================================================
#
# This verifies that Tier 3 additions did not break the existing end-to-end
# pipeline (parse → validate → instantiate → call).

subtest 'regression: square(5) = 25 still works' => sub {
    # Build the hand-assembled square.wasm (see t/square.t for full commentary).
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

    sub _section {
        my ($id, @payload) = @_;
        return ($id, _leb128(scalar @payload), @payload);
    }

    my @header = (0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00);
    my @type_sec  = _section(1, _leb128(1), 0x60, 0x01, 0x7F, 0x01, 0x7F);
    my @func_sec  = _section(3, _leb128(1), _leb128(0));
    my @name      = map { ord($_) } split //, 'square';
    my @exp_sec   = _section(7, _leb128(1), _leb128(scalar @name), @name, 0x00, _leb128(0));
    my @instr     = (0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B);
    my @body      = (_leb128(0), @instr);
    my @code_sec  = _section(10, _leb128(1), _leb128(scalar @body), @body);

    my $wasm = pack('C*', @header, @type_sec, @func_sec, @exp_sec, @code_sec);

    my $rt     = CodingAdventures::WasmRuntime->new();
    my $result = $rt->load_and_run($wasm, 'square', [5]);
    is($result, [25], 'square(5) = 25 still works after Tier 3 additions');
};

done_testing;
