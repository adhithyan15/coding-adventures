// WasmRuntime.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasmRuntime -- The Complete WebAssembly 1.0 Runtime
// ============================================================================
//
// A WASM runtime composes all the lower-level packages into a single,
// easy-to-use API. It handles the full pipeline:
//
//   .wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Execute
//       |              |           |             |              |
//   [UInt8]       WasmModule  ValidatedModule  WasmInstance  WasmValue[]
//
// The convenience method loadAndRun() does all four steps in one call:
//
//   let runtime = WasmRuntime()
//   let result = try runtime.loadAndRun(squareWasm, entry: "square", args: [5])
//   // result == [25]
//
// ============================================================================
// Instantiation
// ============================================================================
//
// Instantiation transforms a static module definition into a live instance:
//
// 1. Resolve imports: ask the host for functions/memory/tables/globals
// 2. Allocate memory: create LinearMemory from the memory section
// 3. Allocate tables: create Tables from the table section
// 4. Initialize globals: evaluate constant expressions
// 5. Apply data segments: copy bytes into memory
// 6. Apply element segments: copy function refs into tables
// 7. Call start function: if the module declares one
//
// ============================================================================

import Foundation
import WasmLeb128
import WasmTypes
import WasmOpcodes
import WasmModuleParser
import WasmValidator
import WasmExecution
import VirtualMachine

// ============================================================================
// MARK: - WasmInstance
// ============================================================================

/// A live, executable instance of a WASM module.
///
/// Contains all allocated runtime state and provides access to exports.
/// Think of a module as a class and an instance as an object: the module
/// defines the blueprint, the instance is a live entity with its own state.
public struct WasmInstance {
    /// The original parsed module.
    public let module: WasmModule

    /// Allocated linear memory (nil if module has no memory).
    public let memory: LinearMemory?

    /// Allocated tables.
    public let tables: [Table]

    /// Current global variable values.
    public var globals: [WasmValue]

    /// Global type descriptors.
    public let globalTypes: [GlobalType]

    /// All function type signatures (imports + module functions).
    public let funcTypes: [FuncType]

    /// Function bodies (nil for imported functions).
    public let funcBodies: [FunctionBody?]

    /// Host function implementations (nil for module-defined functions).
    public let hostFunctions: [HostFunction?]

    /// Export lookup table: name -> (kind, index).
    public let exports: [String: (kind: ExternalKind, index: UInt32)]

    /// The host interface used to resolve imports.
    public let host: HostInterface?
}

// ============================================================================
// MARK: - WasiConfig
// ============================================================================

/// Configuration for the WASI host stub.
///
/// WasiConfig bundles all the things a WASI program can query from the host:
///
///   - args    : command-line arguments (argv)
///   - env     : environment variables (key=value pairs)
///   - stdout  : optional callback invoked on each stdout write
///   - stderr  : optional callback invoked on each stderr write
///   - clock   : injectable clock (swap SystemClock for FakeClock in tests)
///   - random  : injectable randomness (swap SystemRandom for FakeRandom in tests)
///
/// The protocol-based clock and random fields are the critical extension points.
/// Because they are typed as protocols rather than concrete structs, tests can
/// pass in any conforming type without touching production code.
///
/// Example (production):
///
///   let config = WasiConfig(args: ["myapp", "--verbose"])
///   let wasi = WasiStub(config: config)
///
/// Example (tests with deterministic clock and PRNG):
///
///   let config = WasiConfig(clock: FakeClock(), random: FakeRandom())
///   let wasi = WasiStub(config: config)
///
public struct WasiConfig {
    /// Command-line arguments passed to the program (analogous to C's argv).
    public var args: [String]

    /// Environment variables as a dictionary (analogous to C's environ).
    public var env: [String: String]

    /// Optional callback invoked when the program reads from stdin (fd 0).
    public var stdin: ((Int) -> [UInt8])?

    /// Optional callback invoked whenever the program writes to stdout (fd 1).
    public var stdout: ((String) -> Void)?

    /// Optional callback invoked whenever the program writes to stderr (fd 2).
    public var stderr: ((String) -> Void)?

    /// Clock implementation (default: SystemClock using Foundation).
    public var clock: WasiClock

    /// Random implementation (default: SystemRandom using Swift's CSPRNG).
    public var random: WasiRandom

    public init(
        args: [String] = [],
        env: [String: String] = [:],
        stdin: ((Int) -> [UInt8])? = nil,
        stdout: ((String) -> Void)? = nil,
        stderr: ((String) -> Void)? = nil,
        clock: WasiClock = SystemClock(),
        random: WasiRandom = SystemRandom()
    ) {
        self.args = args
        self.env = env
        self.stdin = stdin
        self.stdout = stdout
        self.stderr = stderr
        self.clock = clock
        self.random = random
    }
}

// ============================================================================
// MARK: - WasiStub
// ============================================================================

/// WASI Tier 1–3 implementation for programs that do I/O, read args/env,
/// query the clock, or generate random bytes.
///
/// WASI (WebAssembly System Interface) defines a portable ABI that lets WASM
/// programs call host-provided functions instead of platform syscalls. This
/// stub implements the most commonly needed functions:
///
///   Tier 1 (original):
///     fd_write    — write bytes to stdout/stderr
///     proc_exit   — terminate with an exit code
///
///   Tier 3 (added here):
///     args_sizes_get  — query argc and total argv buffer size
///     args_get        — copy argv strings + pointer table into memory
///     environ_sizes_get — query envc and total environ buffer size
///     environ_get       — copy environ strings + pointer table into memory
///     clock_res_get   — query clock resolution
///     clock_time_get  — read current time from realtime or monotonic clock
///     random_get      — fill a buffer with random bytes
///     sched_yield     — yield (no-op on cooperative runtimes)
///
/// Clock and randomness are injected via WasiConfig, making tests fully
/// deterministic without mocking or patching global state.
///
/// Usage:
///
///   let wasi = WasiStub()                     // defaults: no args, SystemClock
///   let wasi = WasiStub(config: myConfig)     // full control
///   let wasi = WasiStub(onStdout: { print($0) })  // legacy convenience init
///
public class WasiStub: HostInterface {
    /// Captured stdout output (accumulated across all writes).
    public var stdoutOutput: String = ""

    /// Captured stderr output (accumulated across all writes).
    public var stderrOutput: String = ""

    /// Exit code set by proc_exit (nil if proc_exit was never called).
    public var exitCode: Int32? = nil

    /// The configuration that drives args, env, clock, and random.
    public let config: WasiConfig

    /// The memory instance (set during instantiation by WasmRuntime).
    public var memory: LinearMemory?

    // ENOSYS: errno 52 — "Function not implemented"
    // Returned by any WASI function that is not yet implemented.
    private let ENOSYS: Int32 = 52

    // EBADF: errno 8 — "Bad file descriptor"
    private let EBADF: Int32 = 8

    // EINVAL: errno 28 — "Invalid argument"
    // Returned when an unknown clock ID is requested.
    private let EINVAL: Int32 = 28

    /// Convenience initialiser matching the original API.
    ///
    /// - Parameter onStdout: Callback invoked on each stdout write (required, pass nil explicitly).
    ///
    /// Note: the `onStdout` label is required so `WasiStub()` (no label) unambiguously
    /// calls the primary `init(config:)`, avoiding "ambiguous use of init" in callers.
    public convenience init(onStdout: ((String) -> Void)?) {
        self.init(config: WasiConfig(stdout: onStdout))
    }

    /// Primary initialiser with full configuration control.
    ///
    /// - Parameter config: The WasiConfig controlling args, env, clock, random.
    public init(config: WasiConfig = WasiConfig()) {
        self.config = config
    }

    // ========================================================================
    // MARK: - HostInterface conformance
    // ========================================================================

    public func resolveFunction(moduleName: String, name: String) -> HostFunction? {
        guard moduleName == "wasi_snapshot_preview1" || moduleName == "wasi_unstable" else {
            return nil
        }

        switch name {

        // --------------------------------------------------------------------
        // Tier 1: I/O and process control
        // --------------------------------------------------------------------

        case "fd_write":
            // fd_write(fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32, .i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.fdWrite(args)
                }
            )

        case "fd_read":
            return HostFunction(
                type: FuncType(params: [.i32, .i32, .i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.fdRead(args)
                }
            )

        case "proc_exit":
            // proc_exit(code: i32) -> (never returns)
            return HostFunction(
                type: FuncType(params: [.i32], results: []),
                call: { [weak self] args in
                    if case .i32(let code) = args[0] {
                        self?.exitCode = code
                    }
                    // Returning normally — the caller should check exitCode.
                    return []
                }
            )

        // --------------------------------------------------------------------
        // Tier 3: args
        // --------------------------------------------------------------------

        case "args_sizes_get":
            // args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.argsSizesGet(args)
                }
            )

        case "args_get":
            // args_get(argv_ptr: i32, argv_buf_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.argsGet(args)
                }
            )

        // --------------------------------------------------------------------
        // Tier 3: environ
        // --------------------------------------------------------------------

        case "environ_sizes_get":
            // environ_sizes_get(envc_ptr: i32, environ_buf_size_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.environSizesGet(args)
                }
            )

        case "environ_get":
            // environ_get(environ_ptr: i32, environ_buf_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.environGet(args)
                }
            )

        // --------------------------------------------------------------------
        // Tier 3: clock
        // --------------------------------------------------------------------

        case "clock_res_get":
            // clock_res_get(id: i32, resolution_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.clockResGet(args)
                }
            )

        case "clock_time_get":
            // clock_time_get(id: i32, precision: i64, time_ptr: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i64, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.clockTimeGet(args)
                }
            )

        // --------------------------------------------------------------------
        // Tier 3: random
        // --------------------------------------------------------------------

        case "random_get":
            // random_get(buf_ptr: i32, buf_len: i32) -> errno
            return HostFunction(
                type: FuncType(params: [.i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    guard let self = self else { return [.i32(52)] }
                    return try self.randomGet(args)
                }
            )

        // --------------------------------------------------------------------
        // Tier 3: scheduler
        // --------------------------------------------------------------------

        case "sched_yield":
            // sched_yield() -> errno
            // Cooperative runtimes have nothing to yield to; just return success.
            return HostFunction(
                type: FuncType(params: [], results: [.i32]),
                call: { _ in return [.i32(0)] }
            )

        // --------------------------------------------------------------------
        // Fallback: return ENOSYS for anything else
        // --------------------------------------------------------------------

        default:
            // We don't know the exact signature, so provide a generic one.
            // Any call will return ENOSYS (52).
            return HostFunction(
                type: FuncType(params: [.i32], results: [.i32]),
                call: { [weak self] _ in
                    return [.i32(self?.ENOSYS ?? 52)]
                }
            )
        }
    }

    public func resolveGlobal(moduleName: String, name: String) -> (type: GlobalType, value: WasmValue)? {
        return nil
    }

    public func resolveMemory(moduleName: String, name: String) -> LinearMemory? {
        return nil
    }

    public func resolveTable(moduleName: String, name: String) -> Table? {
        return nil
    }

    // ========================================================================
    // MARK: - Tier 1: fd_write / fd_read
    // ========================================================================
    //
    // fd_write(fd: i32, iovs: i32, iovsLen: i32, nwritten: i32) -> i32
    //
    // Reads iovec structures from memory (each is [ptr:i32, len:i32]),
    // concatenates the referenced byte ranges, and writes them to the
    // appropriate file descriptor.
    //
    // iovec layout (8 bytes each):
    //   offset 0: i32 — pointer to the data buffer
    //   offset 4: i32 — length of the data buffer

    private func fdWrite(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let fd = try args[0].asI32()
        let iovsPtr = try args[1].asI32()
        let iovsLen = try args[2].asI32()
        let nwrittenPtr = try args[3].asI32()

        var totalBytes: Int32 = 0
        var output = ""

        for i in 0..<iovsLen {
            let iovOffset = Int(iovsPtr) + Int(i) * 8
            let ptr = try mem.loadI32(iovOffset)
            let len = try mem.loadI32(iovOffset + 4)

            if len > 0 {
                var bytes: [UInt8] = []
                for j in 0..<Int(len) {
                    let b = try mem.loadI32_8u(Int(ptr) + j)
                    bytes.append(UInt8(b))
                }
                if let str = String(bytes: bytes, encoding: .utf8) {
                    output += str
                }
                totalBytes &+= len
            }
        }

        // Write to appropriate fd.
        if fd == 1 {
            stdoutOutput += output
            config.stdout?(output)
        } else if fd == 2 {
            stderrOutput += output
            config.stderr?(output)
        }

        // Write number of bytes written.
        try mem.storeI32(Int(nwrittenPtr), totalBytes)

        return [.i32(0)]  // Success (errno = 0)
    }

    private func fdRead(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let fd = try args[0].asI32()
        let iovsPtr = try args[1].asI32()
        let iovsLen = try args[2].asI32()
        let nreadPtr = try args[3].asI32()

        guard fd == 0 else { return [.i32(EBADF)] }

        var totalBytes: Int32 = 0

        for i in 0..<iovsLen {
            let iovOffset = Int(iovsPtr) + Int(i) * 8
            let ptr = try mem.loadI32(iovOffset)
            let len = try mem.loadI32(iovOffset + 4)
            let chunk = Array((config.stdin?(Int(len)) ?? []).prefix(Int(len)))

            for (offset, byte) in chunk.enumerated() {
                try mem.storeI32_8(Int(ptr) + offset, Int32(byte))
            }

            totalBytes &+= Int32(chunk.count)
            if chunk.count < Int(len) {
                break
            }
        }

        try mem.storeI32(Int(nreadPtr), totalBytes)
        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: args_sizes_get
    // ========================================================================
    //
    // args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) -> errno
    //
    // Writes two values to memory:
    //   *argc_ptr         = number of arguments
    //   *argv_buf_size_ptr = total bytes needed for all argv strings
    //                        (each string is NUL-terminated, so length + 1)
    //
    // Example: args = ["myapp", "hello"]
    //   argc = 2
    //   bufSize = len("myapp\0") + len("hello\0") = 6 + 6 = 12

    private func argsSizesGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let argcPtr = try args[0].asI32()
        let argvBufSizePtr = try args[1].asI32()

        let argc = Int32(config.args.count)
        let bufSize = config.args.reduce(0) { $0 + $1.utf8.count + 1 }  // +1 for NUL terminator

        try mem.storeI32(Int(argcPtr), argc)
        try mem.storeI32(Int(argvBufSizePtr), Int32(bufSize))

        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: args_get
    // ========================================================================
    //
    // args_get(argv_ptr: i32, argv_buf_ptr: i32) -> errno
    //
    // Populates two regions of memory:
    //   argv_ptr     : array of i32 pointers (one per argument)
    //   argv_buf_ptr : contiguous buffer of NUL-terminated UTF-8 strings
    //
    // Memory layout with args = ["myapp", "hello"] at argv_ptr=100, buf=200:
    //
    //   addr 100: i32 = 200       <- points to "myapp\0"
    //   addr 104: i32 = 206       <- points to "hello\0"
    //   addr 200: 6D 79 61 70 70 00   <- "myapp\0"
    //   addr 206: 68 65 6C 6C 6F 00   <- "hello\0"
    //
    // The pointer table and string buffer are completely separate; C programs
    // access them via argv[i] which dereferences the pointer.

    private func argsGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let argvPtr = try args[0].asI32()
        let argvBufPtr = try args[1].asI32()

        var offset = Int(argvBufPtr)

        for (i, arg) in config.args.enumerated() {
            // Write the pointer for this argument into the argv table.
            try mem.storeI32(Int(argvPtr) + i * 4, Int32(offset))

            // Write the NUL-terminated UTF-8 string into the buffer.
            let bytes = Array(arg.utf8) + [0]  // [UInt8] with trailing NUL
            try mem.writeBytes(offset, bytes)

            offset += bytes.count
        }

        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: environ_sizes_get
    // ========================================================================
    //
    // environ_sizes_get(envc_ptr: i32, environ_buf_size_ptr: i32) -> errno
    //
    // Same structure as args_sizes_get but for environment variables.
    // Each env var is formatted as "KEY=VALUE\0".
    //
    // Example: env = {"HOME": "/home/user"}
    //   envc = 1
    //   bufSize = len("HOME=/home/user\0") = 15 + 1 = 16

    private func environSizesGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let envcPtr = try args[0].asI32()
        let environBufSizePtr = try args[1].asI32()

        let envc = Int32(config.env.count)
        let bufSize = config.env.reduce(0) { sum, kv in
            // "KEY=VALUE\0" — key + '=' + value + NUL
            sum + kv.key.utf8.count + 1 + kv.value.utf8.count + 1
        }

        try mem.storeI32(Int(envcPtr), envc)
        try mem.storeI32(Int(environBufSizePtr), Int32(bufSize))

        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: environ_get
    // ========================================================================
    //
    // environ_get(environ_ptr: i32, environ_buf_ptr: i32) -> errno
    //
    // Same structure as args_get but for environment variables.
    // Each string in the buffer is "KEY=VALUE\0".
    //
    // The iteration order over config.env is dictionary order (non-deterministic
    // in general), which matches real environ behaviour.

    private func environGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let environPtr = try args[0].asI32()
        let environBufPtr = try args[1].asI32()

        var offset = Int(environBufPtr)

        for (i, (key, value)) in config.env.enumerated() {
            // Write the pointer for this env string into the pointer table.
            try mem.storeI32(Int(environPtr) + i * 4, Int32(offset))

            // Write "KEY=VALUE\0" into the buffer.
            let str = "\(key)=\(value)"
            let bytes = Array(str.utf8) + [0]
            try mem.writeBytes(offset, bytes)

            offset += bytes.count
        }

        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: clock_res_get
    // ========================================================================
    //
    // clock_res_get(id: i32, resolution_ptr: i32) -> errno
    //
    // Writes the clock resolution (in nanoseconds) as a little-endian i64
    // to *resolution_ptr.
    //
    // WASI clock IDs:
    //   0 — CLOCK_REALTIME
    //   1 — CLOCK_MONOTONIC
    //   2 — CLOCK_PROCESS_CPUTIME_ID
    //   3 — CLOCK_THREAD_CPUTIME_ID

    private func clockResGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let id = try args[0].asI32()
        let resolutionPtr = try args[1].asI32()

        let ns = config.clock.resolutionNs(clockId: id)
        try mem.storeI64(Int(resolutionPtr), ns)

        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: clock_time_get
    // ========================================================================
    //
    // clock_time_get(id: i32, precision: i64, time_ptr: i32) -> errno
    //
    // Reads the current time from the injected clock and writes it as a
    // little-endian i64 (nanoseconds) to *time_ptr.
    //
    // The `precision` parameter is a hint the caller provides about how precise
    // it needs the answer to be.  We ignore it (real OS implementations also
    // mostly ignore it) and return the best reading we have.
    //
    // Clock IDs:
    //   0 — REALTIME             -> config.clock.realtimeNs()
    //   1 — MONOTONIC            -> config.clock.monotonicNs()
    //   2 — PROCESS_CPUTIME_ID  -> approximated by realtimeNs()
    //   3 — THREAD_CPUTIME_ID   -> approximated by realtimeNs()
    //   other -> return EINVAL (28)

    private func clockTimeGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let id = try args[0].asI32()
        let timePtr = try args[2].asI32()   // args[1] is precision (i64), we skip it

        let ns: Int64
        switch id {
        case 0, 2, 3:
            ns = config.clock.realtimeNs()
        case 1:
            ns = config.clock.monotonicNs()
        default:
            return [.i32(EINVAL)]
        }

        try mem.storeI64(Int(timePtr), ns)

        return [.i32(0)]
    }

    // ========================================================================
    // MARK: - Tier 3: random_get
    // ========================================================================
    //
    // random_get(buf_ptr: i32, buf_len: i32) -> errno
    //
    // Fills the memory range [buf_ptr, buf_ptr + buf_len) with random bytes
    // from config.random.
    //
    // In production, config.random is SystemRandom (cryptographically secure).
    // In tests, config.random can be FakeRandom (deterministic).

    private func randomGet(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let bufPtr = try args[0].asI32()
        let bufLen = try args[1].asI32()

        let bytes = config.random.fillBytes(count: Int(bufLen))
        try mem.writeBytes(Int(bufPtr), bytes)

        return [.i32(0)]
    }
}

// ============================================================================
// MARK: - WasmRuntime
// ============================================================================

/// Complete WebAssembly 1.0 runtime.
///
/// Composes the parser, validator, and execution engine into a single
/// user-facing API. Optionally accepts a host interface for import
/// resolution (e.g., a WASI implementation).
///
/// Usage:
///
///   // Simple: compute square(5) from a .wasm binary
///   let runtime = WasmRuntime()
///   let result = try runtime.loadAndRun(squareWasm, entry: "square", args: [5])
///   // result == [25]
///
///   // With WASI for programs that do I/O:
///   let wasi = WasiStub()
///   let runtime = WasmRuntime(host: wasi)
///   try runtime.loadAndRun(helloWorldWasm)
///   print(wasi.stdoutOutput)
///
public class WasmRuntime {
    private let parser: WasmModuleParser
    private let host: HostInterface?

    /// Create a runtime with an optional host interface.
    public init(host: HostInterface? = nil) {
        self.parser = WasmModuleParser()
        self.host = host
    }

    // ========================================================================
    // MARK: - Parse
    // ========================================================================

    /// Parse a .wasm binary into a WasmModule.
    ///
    /// - Parameter wasmBytes: The raw .wasm binary data.
    /// - Returns: The parsed module structure.
    /// - Throws: WasmParseError on malformed binary data.
    public func load(_ wasmBytes: [UInt8]) throws -> WasmModule {
        return try parser.parse(wasmBytes)
    }

    // ========================================================================
    // MARK: - Validate
    // ========================================================================

    /// Validate a parsed module for structural correctness.
    ///
    /// - Parameter module: The parsed WASM module.
    /// - Returns: The validated module with resolved type information.
    /// - Throws: ValidationError on validation failures.
    public func validateModule(_ module: WasmModule) throws -> ValidatedModule {
        return try validate(module)
    }

    // ========================================================================
    // MARK: - Instantiate
    // ========================================================================

    /// Create a live instance from a parsed module.
    ///
    /// This allocates all runtime resources: memory, tables, globals.
    /// Resolves imports, applies data/element segments, and calls the
    /// start function if one is declared.
    ///
    /// - Parameter module: The parsed WASM module.
    /// - Returns: A live, executable instance.
    public func instantiate(_ module: WasmModule) throws -> WasmInstance {
        // Step 1: Build combined function type array (imports + module functions).
        var funcTypes: [FuncType] = []
        var funcBodies: [FunctionBody?] = []
        var hostFunctions: [HostFunction?] = []
        var globalTypes: [GlobalType] = []
        var globals: [WasmValue] = []

        // Step 2: Resolve imports.
        var memory: LinearMemory? = nil
        var tables: [Table] = []

        for imp in module.imports {
            switch imp.kind {
            case .function:
                if case .function(let typeIndex) = imp.typeInfo {
                    let funcType = module.types[Int(typeIndex)]
                    funcTypes.append(funcType)
                    funcBodies.append(nil)  // No body for imports.

                    let hostFunc = host?.resolveFunction(moduleName: imp.moduleName, name: imp.name)
                    hostFunctions.append(hostFunc)
                }

            case .memory:
                let importedMem = host?.resolveMemory(moduleName: imp.moduleName, name: imp.name)
                if let m = importedMem {
                    memory = m
                }

            case .table:
                let importedTable = host?.resolveTable(moduleName: imp.moduleName, name: imp.name)
                if let t = importedTable {
                    tables.append(t)
                }

            case .global:
                let importedGlobal = host?.resolveGlobal(moduleName: imp.moduleName, name: imp.name)
                if let g = importedGlobal {
                    globalTypes.append(g.type)
                    globals.append(g.value)
                }
            }
        }

        // Step 3: Add module-defined functions.
        for i in 0..<module.functions.count {
            let typeIdx = module.functions[i]
            funcTypes.append(module.types[Int(typeIdx)])
            if i < module.code.count {
                funcBodies.append(module.code[i])
            } else {
                funcBodies.append(nil)
            }
            hostFunctions.append(nil)
        }

        // Step 4: Allocate memory (from memory section, if not imported).
        if memory == nil && !module.memories.isEmpty {
            let memType = module.memories[0]
            memory = LinearMemory(
                initialPages: Int(memType.limits.min),
                maxPages: memType.limits.max.map { Int($0) }
            )
        }

        // Give WASI access to memory.
        if let wasi = host as? WasiStub {
            wasi.memory = memory
        }

        // Step 5: Allocate tables (from table section, if not imported).
        for tableType in module.tables {
            tables.append(Table(
                initialSize: Int(tableType.limits.min),
                maxSize: tableType.limits.max.map { Int($0) }
            ))
        }

        // Step 6: Initialize globals (from global section).
        for global in module.globals {
            globalTypes.append(global.globalType)
            let value = try evaluateConstExpr(global.initExpr, globals: globals)
            globals.append(value)
        }

        // Step 7: Apply data segments (copy bytes to memory).
        if let mem = memory {
            for seg in module.data {
                let offset = try evaluateConstExpr(seg.offsetExpr, globals: globals)
                if case .i32(let off) = offset {
                    try mem.writeBytes(Int(off), seg.data)
                }
            }
        }

        // Step 8: Apply element segments (copy func refs to tables).
        for elem in module.elements {
            if Int(elem.tableIndex) < tables.count {
                let table = tables[Int(elem.tableIndex)]
                let offset = try evaluateConstExpr(elem.offsetExpr, globals: globals)
                if case .i32(let off) = offset {
                    for j in 0..<elem.functionIndices.count {
                        try table.set(Int(off) + j, Int(elem.functionIndices[j]))
                    }
                }
            }
        }

        // Build the export map.
        var exports: [String: (kind: ExternalKind, index: UInt32)] = [:]
        for exp in module.exports {
            exports[exp.name] = (kind: exp.kind, index: exp.index)
        }

        let instance = WasmInstance(
            module: module,
            memory: memory,
            tables: tables,
            globals: globals,
            globalTypes: globalTypes,
            funcTypes: funcTypes,
            funcBodies: funcBodies,
            hostFunctions: hostFunctions,
            exports: exports,
            host: host
        )

        // Step 9: Call start function (if present).
        if let startIdx = module.start {
            let engine = WasmExecutionEngine(
                memory: instance.memory,
                tables: instance.tables,
                globals: instance.globals,
                globalTypes: instance.globalTypes,
                funcTypes: instance.funcTypes,
                funcBodies: instance.funcBodies,
                hostFunctions: instance.hostFunctions
            )
            _ = try engine.callFunction(Int(startIdx), [])
        }

        return instance
    }

    // ========================================================================
    // MARK: - Call
    // ========================================================================

    /// Call an exported function by name.
    ///
    /// - Parameters:
    ///   - instance: The live WASM instance.
    ///   - name: The export name (e.g., "square", "add", "_start").
    ///   - args: Arguments as plain numbers (converted to WasmValues).
    /// - Returns: Return values as plain numbers.
    /// - Throws: TrapError if the export doesn't exist or on runtime errors.
    public func call(_ instance: inout WasmInstance, name: String, args: [Double] = []) throws -> [Double] {
        guard let exp = instance.exports[name] else {
            throw TrapError("export \"\(name)\" not found")
        }
        guard exp.kind == .function else {
            throw TrapError("export \"\(name)\" is not a function")
        }

        let funcIndex = Int(exp.index)
        guard funcIndex < instance.funcTypes.count else {
            throw TrapError("function type not found for export \"\(name)\"")
        }
        let funcType = instance.funcTypes[funcIndex]

        // Convert plain numbers to WasmValues based on parameter types.
        var wasmArgs: [WasmValue] = []
        for i in 0..<args.count {
            if i < funcType.params.count {
                switch funcType.params[i] {
                case .i32: wasmArgs.append(.i32(Int32(args[i])))
                case .i64: wasmArgs.append(.i64(Int64(args[i])))
                case .f32: wasmArgs.append(.f32(Float(args[i])))
                case .f64: wasmArgs.append(.f64(args[i]))
                }
            } else {
                wasmArgs.append(.i32(Int32(args[i])))
            }
        }

        // Create execution engine and call.
        let engine = WasmExecutionEngine(
            memory: instance.memory,
            tables: instance.tables,
            globals: instance.globals,
            globalTypes: instance.globalTypes,
            funcTypes: instance.funcTypes,
            funcBodies: instance.funcBodies,
            hostFunctions: instance.hostFunctions
        )

        let results = try engine.callFunction(funcIndex, wasmArgs)

        // Propagate global mutations back to instance.
        instance.globals = engine.globals

        // Convert WasmValues back to plain numbers.
        return results.map { $0.numericValue }
    }

    // ========================================================================
    // MARK: - Convenience
    // ========================================================================

    /// Parse, validate, instantiate, and call in one step.
    ///
    /// This is the easiest way to run a WASM module:
    ///
    ///   let result = try runtime.loadAndRun(wasmBytes, entry: "square", args: [5])
    ///   // result == [25]
    ///
    /// - Parameters:
    ///   - wasmBytes: The raw .wasm binary.
    ///   - entry: The export function name to call (default: "_start").
    ///   - args: Arguments as plain numbers (default: []).
    /// - Returns: Return values as plain numbers.
    public func loadAndRun(_ wasmBytes: [UInt8], entry: String = "_start", args: [Double] = []) throws -> [Double] {
        let module = try load(wasmBytes)
        _ = try validateModule(module)
        var instance = try instantiate(module)
        return try call(&instance, name: entry, args: args)
    }
}

public typealias WasiHost = WasiStub
