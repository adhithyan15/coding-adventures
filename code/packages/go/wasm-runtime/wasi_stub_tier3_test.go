// wasi_stub_tier3_test.go --- Tests for WASI Tier 3 host functions.
//
// ════════════════════════════════════════════════════════════════════════
// TESTING PHILOSOPHY
// ════════════════════════════════════════════════════════════════════════
//
// WASI host functions write their results directly into WASM linear memory.
// Our test strategy is:
//
//  1. Allocate a LinearMemory with a known initial pattern (zeroes).
//  2. Call the WASI function via its HostFunction.Call() method.
//  3. Read the bytes back from the expected memory offsets.
//  4. Assert the values match what we wrote in step 1's preconditions.
//
// All tests inject FakeClock and FakeRandom so results are deterministic
// across runs (no real clock, no real randomness).
//
// ════════════════════════════════════════════════════════════════════════
// FAKE IMPLEMENTATIONS FOR DETERMINISTIC TESTS
// ════════════════════════════════════════════════════════════════════════
//
// FakeClock returns fixed nanosecond values:
//   - RealtimeNs  → 1_700_000_000_000_000_001  (a concrete Unix timestamp)
//   - MonotonicNs → 42_000_000_000             (42 seconds since start)
//   - ResolutionNs→ 1_000_000                  (1 ms resolution)
//
// FakeRandom fills every byte in the buffer with 0xAB, making it easy to
// check that random_get wrote the right number of bytes at the right place.
package wasmruntime

import (
	"testing"

	wasmexecution "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution"
)

// ════════════════════════════════════════════════════════════════════════
// FAKE CLOCK
// ════════════════════════════════════════════════════════════════════════

// FakeClock returns hard-coded nanosecond values so tests are repeatable.
type FakeClock struct{}

func (FakeClock) RealtimeNs() int64          { return 1_700_000_000_000_000_001 }
func (FakeClock) MonotonicNs() int64         { return 42_000_000_000 }
func (FakeClock) ResolutionNs(_ int32) int64 { return 1_000_000 }

// ════════════════════════════════════════════════════════════════════════
// FAKE RANDOM
// ════════════════════════════════════════════════════════════════════════

// FakeRandom fills every byte with 0xAB, a distinctive sentinel value.
// This makes it easy to spot if random_get only partially filled the buffer.
type FakeRandom struct{}

func (FakeRandom) FillBytes(buf []byte) error {
	for i := range buf {
		buf[i] = 0xAB
	}
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// HELPER: newTier3Stub
// ════════════════════════════════════════════════════════════════════════

// newTier3Stub returns a WasiStub wired with FakeClock and FakeRandom,
// backed by 1 page of zero-initialised linear memory.
//
// args and env are optional.  Pass nil for empty lists.
func newTier3Stub(args, env []string) (*WasiStub, *wasmexecution.LinearMemory) {
	mem := wasmexecution.NewLinearMemory(1, -1) // 64 KiB, no max
	stub := NewWasiStubFromConfig(WasiConfig{
		Args:   args,
		Env:    env,
		Clock:  FakeClock{},
		Random: FakeRandom{},
	})
	stub.SetMemory(mem)
	return stub, mem
}

// ════════════════════════════════════════════════════════════════════════
// ARGS TESTS
// ════════════════════════════════════════════════════════════════════════

// TestArgsSizesGet verifies that args_sizes_get writes the correct argument
// count and total buffer size into memory.
//
// args = ["myapp", "hello"]
//
//	argc     = 2
//	buf_size = len("myapp")+1 + len("hello")+1 = 6 + 6 = 12
func TestArgsSizesGet(t *testing.T) {
	stub, mem := newTier3Stub([]string{"myapp", "hello"}, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "args_sizes_get")
	if fn == nil {
		t.Fatal("args_sizes_get not found")
	}

	// Write results to offsets 100 (argc) and 104 (buf_size).
	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(100), // argc_ptr
		wasmexecution.I32(104), // argv_buf_size_ptr
	})

	// Function should return ESUCCESS (0).
	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	argc := mem.LoadI32(100)
	bufSize := mem.LoadI32(104)

	if argc != 2 {
		t.Errorf("expected argc=2, got %d", argc)
	}
	if bufSize != 12 {
		t.Errorf("expected buf_size=12, got %d (myapp\\0=6, hello\\0=6)", bufSize)
	}
}

// TestArgsGet verifies that args_get writes argument pointers and
// null-terminated strings to the correct memory locations.
//
// With args = ["myapp", "hello"]:
//
//	argv_buf (at offset 200): m y a p p \0 h e l l o \0
//	                           0 1 2 3 4  5 6 7 8 9 10 11
//	argv array (at offset 100):
//	  argv[0] = 200  (pointer to "myapp\0")
//	  argv[1] = 206  (pointer to "hello\0")
func TestArgsGet(t *testing.T) {
	stub, mem := newTier3Stub([]string{"myapp", "hello"}, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "args_get")
	if fn == nil {
		t.Fatal("args_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(100), // argv_ptr (array of pointers)
		wasmexecution.I32(200), // argv_buf_ptr (packed strings)
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	// Check argv pointer array.
	argv0 := mem.LoadI32(100)
	argv1 := mem.LoadI32(104)

	if argv0 != 200 {
		t.Errorf("argv[0] expected 200 (start of argv_buf), got %d", argv0)
	}
	// "myapp\0" is 6 bytes, so argv[1] starts at 200+6=206.
	if argv1 != 206 {
		t.Errorf("argv[1] expected 206, got %d", argv1)
	}

	// Check that "myapp\0" is written at offset 200.
	want0 := []byte("myapp\x00")
	for i, b := range want0 {
		got := byte(mem.LoadI32_8u(200 + i))
		if got != b {
			t.Errorf("argv_buf[%d]: expected 0x%02x, got 0x%02x", i, b, got)
		}
	}

	// Check that "hello\0" is written at offset 206.
	want1 := []byte("hello\x00")
	for i, b := range want1 {
		got := byte(mem.LoadI32_8u(206 + i))
		if got != b {
			t.Errorf("argv_buf[%d]: expected 0x%02x, got 0x%02x", 6+i, b, got)
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// ENVIRON TESTS
// ════════════════════════════════════════════════════════════════════════

// TestEnvironSizesGet verifies the environ_sizes_get result.
//
// env = ["HOME=/home/user"]
//
//	count    = 1
//	buf_size = len("HOME=/home/user")+1 = 15+1 = 16
//
// Wait — "HOME=/home/user" has 15 chars, so "HOME=/home/user\0" = 16 bytes.
// But the spec comment in the task says buf_size=15.  Let me count:
//
//	H O M E = / h o m e / u s e r  → 15 characters → +1 null → 16 bytes.
//
// The task says "buf_size=15" which appears to be counting WITHOUT the null.
// We always include the null terminator in the buffer, so we assert 16.
func TestEnvironSizesGet(t *testing.T) {
	stub, mem := newTier3Stub(nil, []string{"HOME=/home/user"})

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "environ_sizes_get")
	if fn == nil {
		t.Fatal("environ_sizes_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(100), // count_ptr
		wasmexecution.I32(104), // buf_size_ptr
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	count := mem.LoadI32(100)
	bufSize := mem.LoadI32(104)

	if count != 1 {
		t.Errorf("expected count=1, got %d", count)
	}
	// "HOME=/home/user" = 15 chars + 1 null = 16 bytes.
	if bufSize != 16 {
		t.Errorf("expected buf_size=16 (15 chars + null), got %d", bufSize)
	}
}

// TestEnvironGet verifies that environ_get writes the correct pointer and
// null-terminated string for a single environment variable.
func TestEnvironGet(t *testing.T) {
	stub, mem := newTier3Stub(nil, []string{"HOME=/home/user"})

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "environ_get")
	if fn == nil {
		t.Fatal("environ_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(100), // environ_ptr  (pointer array)
		wasmexecution.I32(200), // environ_buf_ptr (packed strings)
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	// environ[0] should point to the start of environ_buf.
	ptr := mem.LoadI32(100)
	if ptr != 200 {
		t.Errorf("environ[0] expected 200, got %d", ptr)
	}

	// The string "HOME=/home/user\0" should be at offset 200.
	want := []byte("HOME=/home/user\x00")
	for i, b := range want {
		got := byte(mem.LoadI32_8u(200 + i))
		if got != b {
			t.Errorf("environ_buf[%d]: expected 0x%02x (%q), got 0x%02x", i, b, rune(b), got)
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// CLOCK TESTS
// ════════════════════════════════════════════════════════════════════════

// TestClockTimeGetRealtime verifies clock_time_get with clock ID 0 (REALTIME).
//
// FakeClock.RealtimeNs() returns 1_700_000_000_000_000_001.
// Stored as a little-endian i64 at the given pointer.
func TestClockTimeGetRealtime(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "clock_time_get")
	if fn == nil {
		t.Fatal("clock_time_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(0),   // id = REALTIME
		wasmexecution.I64(0),   // precision (ignored)
		wasmexecution.I32(100), // time_ptr
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	got := mem.LoadI64(100)
	const want = int64(1_700_000_000_000_000_001)
	if got != want {
		t.Errorf("clock_time_get(REALTIME): expected %d, got %d", want, got)
	}
}

// TestClockTimeGetMonotonic verifies clock_time_get with clock ID 1 (MONOTONIC).
//
// FakeClock.MonotonicNs() returns 42_000_000_000 (42 seconds).
func TestClockTimeGetMonotonic(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "clock_time_get")
	if fn == nil {
		t.Fatal("clock_time_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(1),   // id = MONOTONIC
		wasmexecution.I64(0),   // precision (ignored)
		wasmexecution.I32(100), // time_ptr
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	got := mem.LoadI64(100)
	const want = int64(42_000_000_000)
	if got != want {
		t.Errorf("clock_time_get(MONOTONIC): expected %d, got %d", want, got)
	}
}

// TestClockTimeGetInvalidID verifies that an unknown clock ID returns EINVAL.
//
// WASI spec: clock IDs 0–3 are defined; anything else is invalid.
func TestClockTimeGetInvalidID(t *testing.T) {
	stub, _ := newTier3Stub(nil, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "clock_time_get")

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(99),  // id = unknown
		wasmexecution.I64(0),   // precision
		wasmexecution.I32(100), // time_ptr
	})

	errno := wasmexecution.AsI32(result[0])
	if errno != int32(wasiEInval) {
		t.Errorf("expected EINVAL(%d), got %d", wasiEInval, errno)
	}
}

// TestClockResGet verifies clock_res_get writes the resolution value.
//
// FakeClock.ResolutionNs() returns 1_000_000 (1 ms) for any clock ID.
func TestClockResGet(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "clock_res_get")
	if fn == nil {
		t.Fatal("clock_res_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(0),   // id = REALTIME
		wasmexecution.I32(100), // resolution_ptr
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	got := mem.LoadI64(100)
	const want = int64(1_000_000) // 1 ms
	if got != want {
		t.Errorf("clock_res_get: expected %d ns, got %d ns", want, got)
	}
}

// ════════════════════════════════════════════════════════════════════════
// RANDOM TESTS
// ════════════════════════════════════════════════════════════════════════

// TestRandomGet verifies that random_get fills the requested memory region.
//
// FakeRandom fills every byte with 0xAB.  We ask for 4 bytes at offset 500.
// After the call, bytes 500–503 should all be 0xAB and byte 504 unchanged.
func TestRandomGet(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "random_get")
	if fn == nil {
		t.Fatal("random_get not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(500), // buf_ptr
		wasmexecution.I32(4),   // buf_len
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}

	// All 4 bytes should be 0xAB.
	for i := 0; i < 4; i++ {
		b := byte(mem.LoadI32_8u(500 + i))
		if b != 0xAB {
			t.Errorf("random_get: byte[%d] expected 0xAB, got 0x%02x", i, b)
		}
	}

	// Byte just past the end should still be zero.
	after := byte(mem.LoadI32_8u(504))
	if after != 0x00 {
		t.Errorf("random_get: byte[4] should be untouched (0x00), got 0x%02x", after)
	}
}

// ════════════════════════════════════════════════════════════════════════
// SCHED_YIELD TEST
// ════════════════════════════════════════════════════════════════════════

// TestSchedYield verifies that sched_yield returns ESUCCESS immediately.
//
// In single-threaded WASM there is nothing to yield to; success is correct.
func TestFdReadReadsStdinIntoGuestMemory(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)
	stub.StdinCallback = func(count int) []byte {
		if count >= 3 {
			return []byte("cat")
		}
		return []byte("cat")[:count]
	}

	mem.StoreI32(100, 200)
	mem.StoreI32(104, 3)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "fd_read")
	if fn == nil {
		t.Fatal("fd_read not found")
	}

	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(0),
		wasmexecution.I32(100),
		wasmexecution.I32(1),
		wasmexecution.I32(300),
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}
	if mem.LoadI32(300) != 3 {
		t.Fatalf("expected nread=3, got %d", mem.LoadI32(300))
	}

	want := []byte("cat")
	for i, b := range want {
		got := byte(mem.LoadI32_8u(200 + i))
		if got != b {
			t.Errorf("stdin byte[%d]: expected 0x%02x, got 0x%02x", i, b, got)
		}
	}
}

func TestFdReadRejectsNonStdinFD(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)
	stub.StdinCallback = func(count int) []byte { return []byte("ignored") }

	mem.StoreI32(100, 200)
	mem.StoreI32(104, 4)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "fd_read")
	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(1),
		wasmexecution.I32(100),
		wasmexecution.I32(1),
		wasmexecution.I32(300),
	})

	if wasmexecution.AsI32(result[0]) != int32(wasiEBadf) {
		t.Fatalf("expected EBADF(%d), got %d", wasiEBadf, wasmexecution.AsI32(result[0]))
	}
	if mem.LoadI32(300) != 0 {
		t.Fatalf("expected nread to remain 0, got %d", mem.LoadI32(300))
	}
}

func TestFdReadEOFReturnsZeroBytes(t *testing.T) {
	stub, mem := newTier3Stub(nil, nil)
	stub.StdinCallback = func(count int) []byte { return nil }

	mem.StoreI32(100, 200)
	mem.StoreI32(104, 4)
	mem.StoreI32(300, 99)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "fd_read")
	result := fn.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(0),
		wasmexecution.I32(100),
		wasmexecution.I32(1),
		wasmexecution.I32(300),
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}
	if mem.LoadI32(300) != 0 {
		t.Fatalf("expected nread=0 at EOF, got %d", mem.LoadI32(300))
	}
	if mem.LoadI32_8u(200) != 0 {
		t.Fatalf("expected stdin buffer to remain unchanged, got %d", mem.LoadI32_8u(200))
	}
}

func TestSchedYield(t *testing.T) {
	stub, _ := newTier3Stub(nil, nil)

	fn := stub.ResolveFunction("wasi_snapshot_preview1", "sched_yield")
	if fn == nil {
		t.Fatal("sched_yield not found")
	}

	result := fn.Call(nil)

	if len(result) != 1 {
		t.Fatalf("expected 1 result, got %d", len(result))
	}
	if wasmexecution.AsI32(result[0]) != 0 {
		t.Errorf("expected ESUCCESS(0), got %d", wasmexecution.AsI32(result[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// REGRESSION: EXISTING SQUARE TEST STILL PASSES
// ════════════════════════════════════════════════════════════════════════

// TestTier3SquareRegression ensures that the WasiConfig constructor does
// not break the existing runtime pipeline that uses NewWasiStub.
//
// This replicates TestRuntimeSquare from wasm_runtime_test.go but goes
// through NewWasiStubFromConfig to verify the new constructor path.
func TestTier3WasiConfigDefaults(t *testing.T) {
	// Constructing with an empty config should not panic.
	stub := NewWasiStubFromConfig(WasiConfig{})

	// Clock and random should default to system implementations.
	if stub.clock == nil {
		t.Fatal("expected default clock to be set")
	}
	if stub.random == nil {
		t.Fatal("expected default random to be set")
	}

	// ResolveFunction for unknown function should return a stub (not nil).
	unknown := stub.ResolveFunction("wasi_snapshot_preview1", "unknown_function_xyz")
	if unknown == nil {
		t.Fatal("unknown WASI function should return ENOSYS stub, not nil")
	}
	res := unknown.Call(nil)
	if wasmexecution.AsI32(res[0]) != 52 {
		t.Errorf("expected ENOSYS(52), got %d", wasmexecution.AsI32(res[0]))
	}
}
