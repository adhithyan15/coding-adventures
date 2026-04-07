// WasiClockRandom.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasiClock and WasiRandom -- Injectable Clock and Randomness for WASI
// ============================================================================
//
// WASI programs need two facilities the host must provide:
//
//   1. Clock readings  (clock_time_get, clock_res_get)
//   2. Random bytes    (random_get)
//
// Rather than hard-coding Foundation's Date or SystemRandomNumberGenerator,
// we expose *protocols* so tests can inject deterministic fakes.  This is
// the classic "dependency injection" pattern applied to the host interface.
//
// The real (production) implementations live here too: SystemClock wraps
// Foundation and ProcessInfo; SystemRandom wraps Swift's built-in CSPRNG.
//
// ============================================================================
// Why protocols?
// ============================================================================
//
// A protocol is Swift's word for an interface or abstract type.  It says:
// "anything that conforms to me must provide these methods."  Callers program
// against the protocol, not the concrete type.
//
// In tests we write:
//
//   struct FakeClock: WasiClock {
//       func realtimeNs()  -> Int64 { 1_700_000_000_000_000_001 }
//       func monotonicNs() -> Int64 { 42_000_000_000 }
//       func resolutionNs(clockId: Int32) -> Int64 { 1_000_000 }
//   }
//
// The WASI functions are then perfectly deterministic regardless of wall time.
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - WasiClock
// ============================================================================

/// Protocol for WASI clock operations.
///
/// WASI defines four clock IDs:
///   0 — REALTIME   : wall-clock time (seconds since Unix epoch)
///   1 — MONOTONIC  : monotonically increasing (good for durations)
///   2 — PROCESS    : CPU time consumed by the current process
///   3 — THREAD     : CPU time consumed by the current thread
///
/// Both PROCESS and THREAD clocks are approximated by REALTIME in the
/// default implementation because Swift/Foundation does not expose
/// per-process CPU counters without platform-specific APIs.
///
/// Inject SystemClock for production; inject FakeClock for deterministic tests.
public protocol WasiClock {
    /// Nanoseconds since the Unix epoch (1970-01-01 00:00:00 UTC).
    func realtimeNs() -> Int64

    /// Nanoseconds since an arbitrary monotonic start (monotonically increasing).
    func monotonicNs() -> Int64

    /// Clock resolution in nanoseconds for the given WASI clock ID.
    ///
    /// The resolution tells callers the smallest meaningful time difference the
    /// clock can measure.  A value of 1_000_000 means 1 ms resolution.
    func resolutionNs(clockId: Int32) -> Int64
}

// ============================================================================
// MARK: - WasiRandom
// ============================================================================

/// Protocol for WASI random byte generation.
///
/// WASI's random_get fills a guest memory buffer with cryptographically
/// strong random bytes.  This protocol lets us swap in a deterministic fake
/// for testing.
///
/// Inject SystemRandom for production; inject FakeRandom for unit tests.
public protocol WasiRandom {
    /// Return an array of `count` random (or fake-random) bytes.
    func fillBytes(count: Int) -> [UInt8]
}

// ============================================================================
// MARK: - SystemClock (production)
// ============================================================================

/// Production clock using Foundation APIs.
///
/// - realtimeNs : Date().timeIntervalSince1970 converted to nanoseconds.
/// - monotonicNs: ProcessInfo.processInfo.systemUptime (seconds since boot).
/// - resolutionNs: Returns 1_000_000 (1 ms) as a conservative safe default.
///   Most modern OS clocks can do better, but advertising a finer resolution
///   than we can guarantee is misleading.
///
/// Example:
///
///   let clock = SystemClock()
///   print(clock.realtimeNs())   // e.g. 1_700_000_000_000_000_000
///   print(clock.monotonicNs())  // e.g.        42_000_000_000
///
public struct SystemClock: WasiClock {
    public init() {}

    /// Wall-clock time as nanoseconds since 1970-01-01 00:00:00 UTC.
    ///
    /// The conversion:
    ///   Date().timeIntervalSince1970  ->  Double seconds  ->  * 1e9  ->  Int64 ns
    ///
    /// The intermediate Double is accurate to ~microseconds for dates near
    /// today; nanosecond precision would require a platform-specific syscall.
    public func realtimeNs() -> Int64 {
        let seconds = Date().timeIntervalSince1970
        return Int64(seconds * 1_000_000_000)
    }

    /// Monotonic uptime as nanoseconds since an arbitrary boot epoch.
    ///
    /// ProcessInfo.processInfo.systemUptime is a Double of seconds since the
    /// system booted.  Monotonically increasing means it never goes backward,
    /// making it ideal for measuring elapsed time without wall-clock skew.
    public func monotonicNs() -> Int64 {
        let seconds = ProcessInfo.processInfo.systemUptime
        return Int64(seconds * 1_000_000_000)
    }

    /// Resolution in nanoseconds — 1 ms (1_000_000 ns) for all clock IDs.
    ///
    /// We advertise 1 ms regardless of ID because Foundation does not expose
    /// per-clock resolution metadata.  This is a conservative safe value:
    /// most OS clocks can do better, but we never claim more than we know.
    public func resolutionNs(clockId: Int32) -> Int64 {
        return 1_000_000  // 1 ms — conservative but correct
    }
}

// ============================================================================
// MARK: - SystemRandom (production)
// ============================================================================

/// Production random using Swift's SystemRandomNumberGenerator.
///
/// Swift's SystemRandomNumberGenerator is documented to use a
/// cryptographically secure source (e.g., arc4random, /dev/urandom, or
/// BCryptGenRandom on the relevant platforms).
///
/// Each call to UInt8.random(in:using:) draws one byte from the generator.
/// For large buffers the generator is held for the full loop, avoiding
/// repeated initialization overhead.
///
/// Example:
///
///   let rng = SystemRandom()
///   let bytes = rng.fillBytes(count: 4)  // e.g. [0x7A, 0x3F, 0xC2, 0x91]
///
public struct SystemRandom: WasiRandom {
    public init() {}

    /// Return `count` cryptographically random bytes.
    ///
    /// Implementation note: `var gen` must be declared as `var` because
    /// the `using:` parameter of UInt8.random(in:using:) is `inout`, meaning
    /// it mutates the generator's internal state.
    public func fillBytes(count: Int) -> [UInt8] {
        var gen = SystemRandomNumberGenerator()
        return (0..<count).map { _ in UInt8.random(in: 0...255, using: &gen) }
    }
}
