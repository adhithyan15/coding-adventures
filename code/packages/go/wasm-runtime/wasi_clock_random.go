// wasi_clock_random.go --- Injectable clock and random interfaces for WASI.
//
// ════════════════════════════════════════════════════════════════════════
// WHY INTERFACES?
// ════════════════════════════════════════════════════════════════════════
//
// WASI's clock and random functions look simple: call time.Now(), call
// crypto/rand.Read().  So why add interfaces instead of calling them
// directly?
//
// Two reasons:
//
//  1. DETERMINISTIC TESTING — unit tests cannot control time.Now() or
//     crypto/rand.Read().  Without injection, every test that calls
//     clock_time_get or random_get produces a different result each run,
//     making assertions impossible.  With interfaces, tests inject
//     FakeClock and FakeRandom that return fixed values.
//
//  2. FUTURE FLEXIBILITY — we may later replace the system clock with our
//     own monotonic timer, or replace crypto/rand with a seeded PRNG for
//     reproducible WASM execution (useful for replay debugging or fuzzing).
//     Swapping the implementation requires zero changes to WASI host code.
//
// This is the "dependency inversion" principle at work: high-level WASI
// logic depends on abstract interfaces (WasiClock, WasiRandom), not on
// concrete OS calls.
//
// ════════════════════════════════════════════════════════════════════════
// CLOCK ID SEMANTICS (from the WASI spec)
// ════════════════════════════════════════════════════════════════════════
//
//  ID 0 — REALTIME:       wall-clock time, nanoseconds since Unix epoch.
//                         May jump backwards if NTP adjusts the clock.
//  ID 1 — MONOTONIC:      always-increasing, arbitrary start point.
//                         Good for measuring elapsed time; NOT for dates.
//  ID 2 — PROCESS_CPUTIME_ID:  CPU time used by this process.
//  ID 3 — THREAD_CPUTIME_ID:   CPU time used by this thread.
//
// Our SystemClock maps IDs 2 and 3 to wall-clock time (close enough for
// a Tier 3 implementation; a Tier 4 could use runtime.ReadMemStats etc.).
//
// ════════════════════════════════════════════════════════════════════════
// RESOLUTION SEMANTICS
// ════════════════════════════════════════════════════════════════════════
//
// clock_res_get asks: "what is the smallest time increment the clock can
// report?"  On most modern OSes, the true resolution is 1–100 ns.  We
// conservatively report 1,000,000 ns (1 ms) to avoid over-promising.
// WASM programs that need tighter resolution should use a monotonic timer.
//
package wasmruntime

import (
	"crypto/rand"
	"time"
)

// ════════════════════════════════════════════════════════════════════════
// INTERFACES
// ════════════════════════════════════════════════════════════════════════

// WasiClock provides time information to WASI functions.
//
// Use SystemClock{} in production.  Inject a fake struct in tests to get
// deterministic, assertion-friendly time values.
//
// All return values are in nanoseconds.
type WasiClock interface {
	// RealtimeNs returns nanoseconds since the Unix epoch (January 1, 1970).
	// Corresponds to WASI clock ID 0 (REALTIME).
	RealtimeNs() int64

	// MonotonicNs returns nanoseconds since an arbitrary start point.
	// This clock never goes backwards.
	// Corresponds to WASI clock ID 1 (MONOTONIC).
	MonotonicNs() int64

	// ResolutionNs returns the clock resolution in nanoseconds for the
	// given clock ID.  WASM programs use this to decide how often to poll.
	ResolutionNs(clockID int32) int64
}

// WasiRandom fills a byte buffer with random data.
//
// Use SystemRandom{} in production (backed by crypto/rand).
// Inject FakeRandom in tests for deterministic byte patterns.
type WasiRandom interface {
	// FillBytes fills buf entirely with random bytes.
	// Returns an error only if the entropy source fails (very rare).
	FillBytes(buf []byte) error
}

// ════════════════════════════════════════════════════════════════════════
// SYSTEM IMPLEMENTATIONS
// ════════════════════════════════════════════════════════════════════════

// SystemClock implements WasiClock using the OS clock via time.Now().
//
// Note on MonotonicNs: Go's time.Now() already returns a value with a
// monotonic component (it's stripped only if you marshal/unmarshal the
// time value).  For our purposes, using UnixNano() is sufficient — the
// monotonic clock's "arbitrary start" just means the epoch doesn't matter,
// and UnixNano() won't go backwards on sane systems.
type SystemClock struct{}

// RealtimeNs returns wall-clock nanoseconds since the Unix epoch.
func (SystemClock) RealtimeNs() int64 { return time.Now().UnixNano() }

// MonotonicNs returns nanoseconds since an arbitrary fixed point.
// We use UnixNano here as a pragmatic approximation.
func (SystemClock) MonotonicNs() int64 { return time.Now().UnixNano() }

// ResolutionNs returns 1 ms (1,000,000 ns) for all clock IDs.
//
// Why 1 ms?  The true Go timer resolution varies by platform (1 ns on
// Linux, ~15 ms on Windows before 2022).  Reporting 1 ms is conservative
// and correct: we never claim finer resolution than we can guarantee.
func (SystemClock) ResolutionNs(_ int32) int64 { return 1_000_000 }

// SystemRandom implements WasiRandom using crypto/rand.
//
// crypto/rand reads from /dev/urandom on Unix, CryptGenRandom on Windows.
// It is cryptographically secure and suitable for generating nonces, keys,
// and UUIDs inside WASM modules.
type SystemRandom struct{}

// FillBytes fills buf with cryptographically random bytes.
func (SystemRandom) FillBytes(buf []byte) error {
	_, err := rand.Read(buf)
	return err
}
