// WasiTier3Tests.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// WASI Tier 3 Tests — args, environ, clock, random, sched_yield
// ============================================================================
//
// These tests verify the eight new WASI host functions added in v0.3.0.
// Each test constructs a tiny WASM module that has memory + the relevant
// WASI import, instantiates it with a WasiStub using injected fakes, and
// then reads memory to verify the output.
//
// The key insight: because WasiClock and WasiRandom are protocols, we can
// swap in fully deterministic implementations (FakeClock, FakeRandom) without
// any patching of global state.  Every test is hermetic and reproducible.
//
// ============================================================================
// Test design
// ============================================================================
//
// Rather than encoding full WAT-compiled WASM binaries for each WASI function
// (which would require an assembler), we test the WASI host functions directly
// by:
//
//   1. Creating a LinearMemory (the guest's memory)
//   2. Setting wasi.memory to point at it
//   3. Calling the WASI HostFunction directly with crafted WasmValue args
//   4. Reading back the results from memory
//
// This is the correct level of abstraction for unit testing the host stub —
// we are not testing the WASM execution engine (that has its own test suite),
// we are testing the WASI *implementation*.
//
// ============================================================================

import XCTest
@testable import WasmRuntime
@testable import WasmExecution
@testable import WasmTypes

// ============================================================================
// MARK: - Fake implementations
// ============================================================================

/// FakeClock returns hard-coded values, making clock tests deterministic.
///
/// Constants chosen to be memorable and easy to verify:
///   realtimeNs  = 1_700_000_000_000_000_001  (one nanosecond past a round Unix time)
///   monotonicNs =        42_000_000_000       (42 seconds of uptime)
///   resolutionNs = 1_000_000                  (1 ms — same as SystemClock advertises)
struct FakeClock: WasiClock {
    func realtimeNs()  -> Int64 { 1_700_000_000_000_000_001 }
    func monotonicNs() -> Int64 { 42_000_000_000 }
    func resolutionNs(clockId: Int32) -> Int64 { 1_000_000 }
}

/// FakeRandom fills every byte with 0xAB, making random_get tests deterministic.
///
/// 0xAB = 171 decimal — distinctive enough that it can't be confused with
/// zero-initialised memory or accidental coincidences.
struct FakeRandom: WasiRandom {
    func fillBytes(count: Int) -> [UInt8] { Array(repeating: 0xAB, count: count) }
}

// ============================================================================
// MARK: - Helper
// ============================================================================

/// Make a minimal LinearMemory with `pages` pages (each page = 64 KiB).
private func makeMemory(pages: Int = 1) -> LinearMemory {
    return LinearMemory(initialPages: pages)
}

/// Read an i64 from a LinearMemory at the given byte offset.
///
/// Little-endian: the low byte is at `offset`, high byte at `offset + 7`.
private func readI64(_ mem: LinearMemory, at offset: Int) throws -> Int64 {
    return try mem.loadI64(offset)
}

/// Read an i32 from a LinearMemory at the given byte offset.
private func readI32(_ mem: LinearMemory, at offset: Int) throws -> Int32 {
    return try mem.loadI32(offset)
}

/// Read a NUL-terminated UTF-8 string from memory starting at `offset`.
private func readCString(_ mem: LinearMemory, at offset: Int) throws -> String {
    var bytes: [UInt8] = []
    var i = offset
    while true {
        let b = try mem.loadI32_8u(i)
        if b == 0 { break }
        bytes.append(UInt8(b))
        i += 1
    }
    return String(bytes: bytes, encoding: .utf8) ?? "<invalid utf8>"
}

// ============================================================================
// MARK: - WasiTier3Tests
// ============================================================================

final class WasiTier3Tests: XCTestCase {

    // ========================================================================
    // MARK: - 1. args_sizes_get
    // ========================================================================
    //
    // With args = ["myapp", "hello"]:
    //   argc    = 2
    //   bufSize = len("myapp\0") + len("hello\0") = 6 + 6 = 12

    func testArgsSizesGet() throws {
        let mem = makeMemory()
        let config = WasiConfig(args: ["myapp", "hello"])
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        // Call args_sizes_get(argc_ptr=0, argv_buf_size_ptr=4)
        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "args_sizes_get")
        XCTAssertNotNil(fn, "args_sizes_get must be resolvable")

        let result = try fn!.call([.i32(0), .i32(4)])
        XCTAssertEqual(result, [.i32(0)], "args_sizes_get should return errno=0 (success)")

        let argc = try readI32(mem, at: 0)
        let bufSize = try readI32(mem, at: 4)

        XCTAssertEqual(argc, 2, "argc should be 2")
        // "myapp\0" = 6 bytes, "hello\0" = 6 bytes — total 12
        XCTAssertEqual(bufSize, 12, "argv buffer size should be 12 bytes")
    }

    func testFdReadCopiesInputBytes() throws {
        let mem = makeMemory()
        let config = WasiConfig(stdin: { _ in Array("hi".utf8) })
        let wasi = WasiHost(config: config)
        wasi.memory = mem

        try mem.storeI32(0, 200)
        try mem.storeI32(4, 2)

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "fd_read")
        XCTAssertNotNil(fn, "fd_read must be resolvable")

        let result = try fn!.call([.i32(0), .i32(0), .i32(1), .i32(100)])
        XCTAssertEqual(result, [.i32(0)])
        XCTAssertEqual(try mem.loadI32(100), 2)
        XCTAssertEqual(try mem.loadI32_8u(200), 104)
        XCTAssertEqual(try mem.loadI32_8u(201), 105)
    }

    // ========================================================================
    // MARK: - 2. args_get
    // ========================================================================
    //
    // Verifies that:
    //   - The pointer table at argv_ptr is populated correctly
    //   - The NUL-terminated strings appear at the expected offsets
    //
    // Memory layout (argv_ptr=0, argv_buf_ptr=100):
    //   addr 0:   i32 = 100         <- pointer to "myapp\0"
    //   addr 4:   i32 = 106         <- pointer to "hello\0"
    //   addr 100: "myapp\0" (6 bytes)
    //   addr 106: "hello\0" (6 bytes)

    func testArgsGet() throws {
        let mem = makeMemory()
        let config = WasiConfig(args: ["myapp", "hello"])
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "args_get")
        XCTAssertNotNil(fn, "args_get must be resolvable")

        // argv_ptr=0, argv_buf_ptr=100
        let result = try fn!.call([.i32(0), .i32(100)])
        XCTAssertEqual(result, [.i32(0)], "args_get should return errno=0")

        // Check pointer table
        let ptr0 = try readI32(mem, at: 0)  // pointer to first arg
        let ptr1 = try readI32(mem, at: 4)  // pointer to second arg

        XCTAssertEqual(ptr0, 100, "First arg pointer should be 100")
        XCTAssertEqual(ptr1, 106, "Second arg pointer should be 106 (100 + 6 bytes for 'myapp\\0')")

        // Check string content via the pointers
        let arg0 = try readCString(mem, at: Int(ptr0))
        let arg1 = try readCString(mem, at: Int(ptr1))

        XCTAssertEqual(arg0, "myapp", "First argument should be 'myapp'")
        XCTAssertEqual(arg1, "hello", "Second argument should be 'hello'")
    }

    // ========================================================================
    // MARK: - 3. environ_sizes_get
    // ========================================================================
    //
    // With env = {"HOME": "/home/user"}:
    //   envc    = 1
    //   bufSize = len("HOME=/home/user\0") = 15 + 1 = 16

    func testEnvironSizesGet() throws {
        let mem = makeMemory()
        let config = WasiConfig(env: ["HOME": "/home/user"])
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "environ_sizes_get")
        XCTAssertNotNil(fn, "environ_sizes_get must be resolvable")

        // environ_sizes_get(envc_ptr=0, environ_buf_size_ptr=4)
        let result = try fn!.call([.i32(0), .i32(4)])
        XCTAssertEqual(result, [.i32(0)], "environ_sizes_get should return errno=0")

        let envc = try readI32(mem, at: 0)
        let bufSize = try readI32(mem, at: 4)

        XCTAssertEqual(envc, 1, "envc should be 1")
        // "HOME=/home/user\0" = 4 + 1 + 9 + 1 = 15... let's count:
        // H O M E = / h o m e / u s e r \0 = 16 bytes
        XCTAssertEqual(bufSize, 16, "environ buffer size should be 16 bytes for 'HOME=/home/user\\0'")
    }

    // ========================================================================
    // MARK: - 4. environ_get
    // ========================================================================
    //
    // Verifies that the env string "HOME=/home/user" is written at the correct
    // offset and is NUL-terminated.

    func testEnvironGet() throws {
        let mem = makeMemory()
        let config = WasiConfig(env: ["HOME": "/home/user"])
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "environ_get")
        XCTAssertNotNil(fn, "environ_get must be resolvable")

        // environ_ptr=0 (pointer table), environ_buf_ptr=200 (string data)
        let result = try fn!.call([.i32(0), .i32(200)])
        XCTAssertEqual(result, [.i32(0)], "environ_get should return errno=0")

        // The pointer at addr 0 should point to the start of the buffer
        let ptr = try readI32(mem, at: 0)
        XCTAssertEqual(ptr, 200, "Env pointer should be 200")

        let envStr = try readCString(mem, at: Int(ptr))
        XCTAssertEqual(envStr, "HOME=/home/user", "Env string should be 'HOME=/home/user'")
    }

    // ========================================================================
    // MARK: - 5. clock_time_get (REALTIME, id=0)
    // ========================================================================
    //
    // With FakeClock.realtimeNs() = 1_700_000_000_000_000_001, reading the i64
    // at time_ptr should produce exactly that value.

    func testClockTimeGetRealtime() throws {
        let mem = makeMemory()
        let config = WasiConfig(clock: FakeClock())
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "clock_time_get")
        XCTAssertNotNil(fn, "clock_time_get must be resolvable")

        // clock_time_get(id=0, precision=0, time_ptr=0)
        // precision is i64, so we pass .i64(0)
        let result = try fn!.call([.i32(0), .i64(0), .i32(0)])
        XCTAssertEqual(result, [.i32(0)], "clock_time_get should return errno=0")

        let ns = try readI64(mem, at: 0)
        XCTAssertEqual(ns, 1_700_000_000_000_000_001, "Realtime ns should match FakeClock.realtimeNs()")
    }

    // ========================================================================
    // MARK: - 6. clock_time_get (MONOTONIC, id=1)
    // ========================================================================

    func testClockTimeGetMonotonic() throws {
        let mem = makeMemory()
        let config = WasiConfig(clock: FakeClock())
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "clock_time_get")!

        // clock_time_get(id=1, precision=0, time_ptr=0)
        let result = try fn.call([.i32(1), .i64(0), .i32(0)])
        XCTAssertEqual(result, [.i32(0)], "clock_time_get (monotonic) should return errno=0")

        let ns = try readI64(mem, at: 0)
        XCTAssertEqual(ns, 42_000_000_000, "Monotonic ns should match FakeClock.monotonicNs()")
    }

    // ========================================================================
    // MARK: - 7. clock_res_get
    // ========================================================================

    func testClockResGet() throws {
        let mem = makeMemory()
        let config = WasiConfig(clock: FakeClock())
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "clock_res_get")
        XCTAssertNotNil(fn, "clock_res_get must be resolvable")

        // clock_res_get(id=0, resolution_ptr=0)
        let result = try fn!.call([.i32(0), .i32(0)])
        XCTAssertEqual(result, [.i32(0)], "clock_res_get should return errno=0")

        let resNs = try readI64(mem, at: 0)
        XCTAssertEqual(resNs, 1_000_000, "Resolution should be 1_000_000 ns (1 ms)")
    }

    // ========================================================================
    // MARK: - 8. random_get
    // ========================================================================
    //
    // FakeRandom fills every byte with 0xAB.
    // We request 4 bytes at buf_ptr=0; all four should equal 0xAB (171).

    func testRandomGet() throws {
        let mem = makeMemory()
        let config = WasiConfig(random: FakeRandom())
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "random_get")
        XCTAssertNotNil(fn, "random_get must be resolvable")

        // random_get(buf_ptr=0, buf_len=4)
        let result = try fn!.call([.i32(0), .i32(4)])
        XCTAssertEqual(result, [.i32(0)], "random_get should return errno=0")

        for i in 0..<4 {
            let byte = try mem.loadI32_8u(i)
            XCTAssertEqual(byte, 0xAB, "Byte \(i) should be 0xAB (FakeRandom fill value)")
        }
    }

    // ========================================================================
    // MARK: - 9. sched_yield
    // ========================================================================

    func testSchedYield() throws {
        let wasi = WasiStub(config: WasiConfig())

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "sched_yield")
        XCTAssertNotNil(fn, "sched_yield must be resolvable")

        let result: [WasmValue] = try fn!.call([])
        XCTAssertEqual(result, [.i32(0)], "sched_yield should return errno=0")
    }

    // ========================================================================
    // MARK: - 10. Regression: existing square test still passes
    // ========================================================================
    //
    // This ensures the Tier 3 changes do not break the baseline Tier 1 + 2
    // functionality (fd_write, proc_exit, basic execution pipeline).

    static let squareWasm: [UInt8] = [
        0x00, 0x61, 0x73, 0x6D,  // magic
        0x01, 0x00, 0x00, 0x00,  // version
        // Type section: (i32) -> (i32)
        0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F,
        // Function section
        0x03, 0x02, 0x01, 0x00,
        // Export section: "square" -> func 0
        0x07, 0x0A, 0x01, 0x06,
        0x73, 0x71, 0x75, 0x61, 0x72, 0x65,  // "square"
        0x00, 0x00,
        // Code section
        0x0A, 0x09, 0x01, 0x07, 0x00,
        0x20, 0x00,  // local.get 0
        0x20, 0x00,  // local.get 0
        0x6C,        // i32.mul
        0x0B,        // end
    ]

    func testSquareStillWorksAfterTier3Changes() throws {
        let runtime = WasmRuntime()
        let result = try runtime.loadAndRun(
            WasiTier3Tests.squareWasm,
            entry: "square",
            args: [7]
        )
        XCTAssertEqual(result, [49], "square(7) should equal 49 — regression guard")
    }

    // ========================================================================
    // MARK: - 11. WasiConfig with custom clock reaches WasiStub
    // ========================================================================
    //
    // Verifies the injection chain: WasiConfig(clock: FakeClock()) -> WasiStub
    // -> clock_time_get returns FakeClock.realtimeNs().

    func testWasiConfigInjection() throws {
        let mem = makeMemory()
        let fakeClock = FakeClock()
        let config = WasiConfig(
            args: ["prog"],
            env: ["FOO": "bar"],
            clock: fakeClock,
            random: FakeRandom()
        )
        let wasi = WasiStub(config: config)
        wasi.memory = mem

        // Verify args
        let argsFn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "args_sizes_get")!
        _ = try argsFn.call([.i32(0), .i32(4)])
        XCTAssertEqual(try readI32(mem, at: 0), 1, "argc should be 1")

        // Verify clock
        let clockFn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "clock_time_get")!
        _ = try clockFn.call([.i32(0), .i64(0), .i32(8)])
        XCTAssertEqual(try readI64(mem, at: 8), fakeClock.realtimeNs(),
            "Clock reading should come from FakeClock")
    }

    // ========================================================================
    // MARK: - 12. clock_time_get with unknown id returns EINVAL
    // ========================================================================

    func testClockTimeGetUnknownIdReturnsEINVAL() throws {
        let mem = makeMemory()
        let wasi = WasiStub(config: WasiConfig())
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "clock_time_get")!
        let args: [WasmValue] = [.i32(99), .i64(0), .i32(0)]
        let result: [WasmValue] = try fn.call(args)
        XCTAssertEqual(result, [.i32(28)], "Unknown clock id should return EINVAL (28)")
    }

    // ========================================================================
    // MARK: - 13. args_sizes_get with empty args
    // ========================================================================

    func testArgsSizesGetEmpty() throws {
        let mem = makeMemory()
        let wasi = WasiStub(config: WasiConfig(args: []))
        wasi.memory = mem

        let fn = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "args_sizes_get")!
        let result = try fn.call([.i32(0), .i32(4)])
        XCTAssertEqual(result, [.i32(0)])

        XCTAssertEqual(try readI32(mem, at: 0), 0, "argc should be 0 with empty args")
        XCTAssertEqual(try readI32(mem, at: 4), 0, "buf size should be 0 with empty args")
    }

    // ========================================================================
    // MARK: - 14. Legacy WasiStub(onStdout:) initialiser still works
    // ========================================================================

    func testLegacyOnStdoutInit() {
        var captured = ""
        let wasi = WasiStub(onStdout: { captured += $0 })
        // The stdout callback should be stored in config.stdout
        wasi.config.stdout?("hello")
        XCTAssertEqual(captured, "hello", "Legacy onStdout callback should still fire via config.stdout")
    }
}
