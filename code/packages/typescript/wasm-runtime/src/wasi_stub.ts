/**
 * WASI Stub — Minimal WebAssembly System Interface Implementation.
 *
 * ==========================================================================
 * Chapter 1: What Is WASI?
 * ==========================================================================
 *
 * WASI (WebAssembly System Interface) is a standardized set of "host functions"
 * that give WASM programs access to the outside world — file I/O, console
 * output, environment variables, command-line arguments, etc.
 *
 * When you compile a Rust or C program to WASM with WASI support, the compiler
 * generates import declarations like:
 *
 * ```wasm
 * (import "wasi_snapshot_preview1" "fd_write" (func $fd_write ...))
 * (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit ...))
 * (import "wasi_snapshot_preview1" "args_get" (func $args_get ...))
 * (import "wasi_snapshot_preview1" "clock_time_get" (func $clock_time_get ...))
 * ```
 *
 * The runtime must provide implementations of these functions. Without them,
 * the program can't print to stdout, check the time, or read its arguments.
 *
 * ==========================================================================
 * Chapter 2: This Stub — Tier 3
 * ==========================================================================
 *
 * This is a **Tier 3 stub** — it goes beyond the minimal Tier 1 (fd_write +
 * fd_read + proc_exit) to implement the most commonly needed WASI functions:
 *
 * - **fd_write**: Capture stdout/stderr output (for testing and Hello World).
 * - **fd_read**: Read bytes from stdin into linear memory.
 * - **proc_exit**: Terminate execution with an exit code.
 * - **args_sizes_get / args_get**: Expose command-line arguments.
 * - **environ_sizes_get / environ_get**: Expose environment variables.
 * - **clock_res_get / clock_time_get**: Expose wall clock and monotonic time.
 * - **random_get**: Fill a buffer with random bytes.
 * - **sched_yield**: Yield the CPU (no-op in single-threaded WASM).
 *
 * Everything else returns ENOSYS (errno 52 — "function not implemented").
 * This is the honest answer: we acknowledge the syscall exists but haven't
 * implemented it yet.
 *
 * ==========================================================================
 * Chapter 3: The Interface Injection Pattern
 * ==========================================================================
 *
 * Clock and random are injected via interfaces rather than called directly.
 * This follows the Dependency Inversion Principle: the stub depends on
 * *abstractions*, not concretions.
 *
 * Why does this matter?
 *
 * 1. **Testability**: Tests can inject a FakeClock that returns fixed values,
 *    making clock tests deterministic (no flakiness due to wall time).
 *
 * 2. **Future extensibility**: When we implement our own PRNG or high-resolution
 *    clock, we swap the implementation without touching the stub's logic.
 *
 * 3. **Browser portability**: Node.js and browsers have different APIs for
 *    time and randomness. The default SystemClock / SystemRandom handle this
 *    detection, but alternative implementations can do it differently.
 *
 * The pattern mirrors how the JVM's `Clock` and Python's `time.clock_gettime`
 * are designed: the algorithm is separate from the time source.
 *
 * @module
 */

import type { FuncType } from "@coding-adventures/wasm-types";
import { ValueType, makeFuncType } from "@coding-adventures/wasm-types";
import type {
  HostInterface,
  HostFunction,
  LinearMemory,
  Table,
  WasmValue,
} from "@coding-adventures/wasm-execution";
import { i32 } from "@coding-adventures/wasm-execution";

// =========================================================================
// WASI Error Codes
// =========================================================================

/** WASI errno: Function not implemented. */
const ENOSYS = 52;

/** WASI errno: Success. */
const ESUCCESS = 0;

/** WASI errno: Bad file descriptor. */
const EBADF = 8;

/** WASI errno: Invalid argument. */
const EINVAL = 28;

// =========================================================================
// Clock Interface and Implementations
// =========================================================================

/**
 * An injectable clock abstraction for WASI time functions.
 *
 * WASI exposes two clocks:
 *
 * - **Realtime** (id=0): Wall clock time, anchored to the Unix epoch
 *   (January 1, 1970 00:00:00 UTC). Used for "what time is it now?"
 *
 * - **Monotonic** (id=1): A clock that only moves forward, not anchored
 *   to any real-world time. Used for measuring elapsed time (e.g., "how
 *   long did this operation take?"). Monotonic clocks cannot go backward,
 *   even if the system clock is adjusted (NTP, daylight saving, etc.).
 *
 * All values are in nanoseconds (10^-9 seconds) as required by WASI.
 *
 * **Why inject this?** In tests, we need deterministic behavior. A FakeClock
 * that always returns the same value makes clock tests reliable — no race
 * conditions, no timezone sensitivity, no CI flakiness.
 */
export interface WasiClock {
  /** Nanoseconds since Unix epoch (January 1, 1970 UTC). */
  realtimeNs(): bigint;

  /**
   * Nanoseconds since some arbitrary, fixed start point.
   * This value only increases over time (monotonic guarantee).
   */
  monotonicNs(): bigint;

  /**
   * The resolution (smallest measurable increment) for the given clock ID,
   * in nanoseconds.
   *
   * @param clockId - 0 for realtime, 1 for monotonic, 2/3 for CPU time.
   */
  resolutionNs(clockId: number): bigint;
}

/**
 * An injectable source of cryptographically random bytes.
 *
 * WASI's `random_get` syscall fills a buffer with random bytes. The source
 * should be cryptographically secure (CSPRNG), not Math.random().
 *
 * Why inject this? Same reason as WasiClock: testability and future-proofing.
 * In the future, we might implement our own deterministic PRNG for reproducible
 * simulations; swapping the implementation is trivial with this interface.
 */
export interface WasiRandom {
  /**
   * Fill `buf` with random bytes in-place.
   *
   * @param buf - The Uint8Array to fill with random bytes.
   */
  fillBytes(buf: Uint8Array): void;
}

/**
 * The default clock implementation using platform APIs.
 *
 * Detects the environment at construction time and selects the appropriate
 * time APIs:
 *
 * - **Node.js**: Uses `process.hrtime.bigint()` for monotonic time (nanosecond
 *   precision) and `Date.now()` for realtime.
 *
 * - **Browser**: Uses `performance.now()` (which has millisecond precision,
 *   deliberately clamped by browsers for Spectre mitigation) for monotonic time,
 *   and `Date.now()` for realtime.
 *
 * The resolution is reported as 1ms (1,000,000 nanoseconds) for all clock IDs.
 * This is conservative and always accurate: even Node's nanosecond timer is
 * bounded by OS scheduler resolution in practice.
 */
export class SystemClock implements WasiClock {
  realtimeNs(): bigint {
    // Date.now() returns milliseconds since Unix epoch.
    // Multiply by 1,000,000 to convert to nanoseconds.
    return BigInt(Date.now()) * 1_000_000n;
  }

  monotonicNs(): bigint {
    // In Node.js, process.hrtime.bigint() returns nanoseconds since process
    // start with full nanosecond precision. This is the ideal monotonic source.
    //
    // In browsers, performance.now() returns milliseconds since navigation
    // start with sub-millisecond precision (but browsers clamp it to 1ms
    // increments for Spectre security). We reconstruct a bigint from the
    // float via Math.round to avoid fractional nanoseconds.
    if (
      typeof process !== "undefined" &&
      process.versions?.node !== undefined
    ) {
      return process.hrtime.bigint();
    }
    return BigInt(
      Math.round(performance.timeOrigin * 1_000_000 + performance.now() * 1_000_000)
    );
  }

  resolutionNs(_clockId: number): bigint {
    // Report 1ms resolution for all clocks. Browsers clamp performance.now()
    // to 1ms for security (Spectre mitigation), so this is always accurate.
    // Node's hrtime is higher resolution in theory, but 1ms is a safe,
    // conservative value that programs can rely on.
    return 1_000_000n;
  }
}

/**
 * The default random byte source using platform cryptographic APIs.
 *
 * - **Node.js**: Uses `crypto.randomFillSync()` from the built-in `crypto` module.
 *   This is a CSPRNG backed by the OS entropy source (e.g., /dev/urandom on Linux).
 *
 * - **Browser**: Uses `globalThis.crypto.getRandomValues()`, the standard Web
 *   Crypto API. Also backed by OS entropy.
 *
 * Both sources are cryptographically secure — suitable for key generation,
 * nonces, session tokens, and other security-sensitive uses.
 *
 * **Important**: `Math.random()` is NOT cryptographically secure. It's a
 * pseudorandom number generator with a predictable internal state. Never
 * use it where security matters.
 */
export class SystemRandom implements WasiRandom {
  fillBytes(buf: Uint8Array): void {
    if (
      typeof process !== "undefined" &&
      process.versions?.node !== undefined
    ) {
      // Node.js: use the built-in crypto module's synchronous fill.
      // We use dynamic import to avoid a hard dependency at module load time —
      // this lets the same module load in browsers without a bundler shim.
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const { randomFillSync } = require("crypto") as {
        randomFillSync: (buf: Uint8Array) => void;
      };
      randomFillSync(buf);
    } else {
      // Browser: use the standard Web Crypto API.
      globalThis.crypto.getRandomValues(buf);
    }
  }
}

// =========================================================================
// WasiConfig
// =========================================================================

/**
 * Configuration for the WASI host environment.
 *
 * This mirrors the concept of a "process environment" on a Unix system:
 * the arguments the program was invoked with, the environment variables
 * set in the shell, the I/O streams, and the clock/random sources.
 *
 * All fields are optional. Sensible defaults are provided:
 * - No command-line args (empty array).
 * - No environment variables (empty object).
 * - stdin reads EOF by default.
 * - stdout/stderr silently discarded.
 * - System clock and cryptographic random.
 *
 * **Example: Running a WASM program with args and env:**
 *
 * ```typescript
 * const wasi = new WasiStub({
 *   args: ["myapp", "--verbose", "input.txt"],
 *   env: { HOME: "/home/user", PATH: "/usr/bin" },
 *   stdout: (text) => process.stdout.write(text),
 *   stderr: (text) => process.stderr.write(text),
 * });
 * ```
 */
export interface WasiConfig {
  /** Command-line arguments (argv). Defaults to []. */
  args?: string[];

  /** Environment variables as a key→value map. Defaults to {}. */
  env?: Record<string, string>;

  /**
   * Called when the program reads from stdin (fd 0).
   * Returns up to `count` bytes, or an empty result to signal EOF.
   */
  stdin?: (count: number) => Uint8Array | readonly number[] | string | null | undefined;

  /** Called when the program writes to stdout (fd 1). */
  stdout?: (text: string) => void;

  /** Called when the program writes to stderr (fd 2). */
  stderr?: (text: string) => void;

  /**
   * Clock source for WASI time syscalls.
   * Defaults to SystemClock (uses Date.now / process.hrtime.bigint).
   */
  clock?: WasiClock;

  /**
   * Random byte source for WASI random_get.
   * Defaults to SystemRandom (uses Node crypto / Web Crypto).
   */
  random?: WasiRandom;
}

// =========================================================================
// Proc Exit Error
// =========================================================================

/**
 * Thrown when a WASM program calls ``proc_exit``.
 *
 * This is not an error in the traditional sense — it's the WASM program
 * requesting clean termination. The runtime catches this and returns
 * the exit code.
 */
export class ProcExitError extends Error {
  readonly exitCode: number;

  constructor(exitCode: number) {
    super(`proc_exit(${exitCode})`);
    this.name = "ProcExitError";
    this.exitCode = exitCode;
  }
}

// =========================================================================
// WASI Stub Host
// =========================================================================

/**
 * A Tier 3 WASI host implementation.
 *
 * Implements the most commonly needed WASI functions for running real-world
 * WASM programs: I/O, arguments, environment, time, and randomness.
 *
 * **Usage with defaults (pure computation):**
 *
 * ```typescript
 * const wasi = new WasiStub();
 * const runtime = new WasmRuntime(wasi);
 * runtime.loadAndRun(wasmBytes, "square", [5]); // → [25]
 * ```
 *
 * **Usage with full config (I/O program):**
 *
 * ```typescript
 * const output: string[] = [];
 * const wasi = new WasiStub({
 *   args: ["myapp", "--verbose"],
 *   env: { HOME: "/home/user" },
 *   stdout: (text) => output.push(text),
 * });
 * runtime.loadAndRun(wasmBytes);
 * console.log(output.join(""));
 * ```
 *
 * **Usage with injected clock (deterministic tests):**
 *
 * ```typescript
 * class FakeClock implements WasiClock {
 *   realtimeNs() { return 1_700_000_000_000_000_000n; }
 *   monotonicNs() { return 42_000_000_000n; }
 *   resolutionNs(_id: number) { return 1_000_000n; }
 * }
 * const wasi = new WasiStub({ clock: new FakeClock() });
 * ```
 */
export class WasiStub implements HostInterface {
  private readonly stdinCallback: (count: number) => Uint8Array | readonly number[] | string | null | undefined;
  private readonly stdoutCallback: (text: string) => void;
  private readonly stderrCallback: (text: string) => void;
  private readonly args: string[];
  private readonly env: Record<string, string>;
  private readonly clock: WasiClock;
  private readonly random: WasiRandom;
  private instanceMemory: LinearMemory | null = null;

  constructor(options?: WasiConfig) {
    this.stdinCallback = options?.stdin ?? ((_count: number) => new Uint8Array(0));
    this.stdoutCallback = options?.stdout ?? ((_t: string) => {});
    this.stderrCallback = options?.stderr ?? ((_t: string) => {});
    this.args = options?.args ?? [];
    this.env = options?.env ?? {};
    this.clock = options?.clock ?? new SystemClock();
    this.random = options?.random ?? new SystemRandom();
  }

  /**
   * Set the instance's memory (needed for memory-accessing WASI functions).
   * Called by the runtime after instantiation.
   */
  setMemory(memory: LinearMemory): void {
    this.instanceMemory = memory;
  }

  resolveFunction(moduleName: string, name: string): HostFunction | undefined {
    if (moduleName !== "wasi_snapshot_preview1") return undefined;

    switch (name) {
      case "fd_write":
        return this.makeFdWrite();
      case "fd_read":
        return this.makeFdRead();
      case "proc_exit":
        return this.makeProcExit();
      case "args_sizes_get":
        return this.makeArgsSizesGet();
      case "args_get":
        return this.makeArgsGet();
      case "environ_sizes_get":
        return this.makeEnvironSizesGet();
      case "environ_get":
        return this.makeEnvironGet();
      case "clock_res_get":
        return this.makeClockResGet();
      case "clock_time_get":
        return this.makeClockTimeGet();
      case "random_get":
        return this.makeRandomGet();
      case "sched_yield":
        return this.makeSchedYield();
      default:
        // Return a stub that returns ENOSYS for any unimplemented function.
        return this.makeStub(name);
    }
  }

  resolveGlobal(_moduleName: string, _name: string): { type: { valueType: number; mutable: boolean }; value: WasmValue } | undefined {
    return undefined;
  }

  resolveMemory(_moduleName: string, _name: string): LinearMemory | undefined {
    return undefined;
  }

  resolveTable(_moduleName: string, _name: string): Table | undefined {
    return undefined;
  }

  // ── fd_write ──────────────────────────────────────────────────────
  //
  // fd_write(fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32) -> i32
  //
  // Writes data from iov (I/O vector) buffers to a file descriptor.
  // For fd=1 (stdout) and fd=2 (stderr), we capture the output.
  //
  // An iov is a pair of (ptr: i32, len: i32) in linear memory:
  //   iov[i].buf_ptr = memory[iovs_ptr + i*8 .. iovs_ptr + i*8 + 4]
  //   iov[i].buf_len = memory[iovs_ptr + i*8 + 4 .. iovs_ptr + i*8 + 8]
  //
  private makeFdWrite(): HostFunction {
    const self = this;
    return {
      type: makeFuncType(
        [ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32],
        [ValueType.I32],
      ),
      call(args: WasmValue[]): WasmValue[] {
        const fd = args[0].value as number;
        const iovsPtr = args[1].value as number;
        const iovsLen = args[2].value as number;
        const nwrittenPtr = args[3].value as number;

        if (!self.instanceMemory) {
          return [i32(ENOSYS)];
        }

        const memory = self.instanceMemory;
        let totalWritten = 0;

        for (let i = 0; i < iovsLen; i++) {
          const bufPtr = memory.loadI32(iovsPtr + i * 8) >>> 0;
          const bufLen = memory.loadI32(iovsPtr + i * 8 + 4) >>> 0;

          // Read the bytes from memory.
          const bytes: number[] = [];
          for (let j = 0; j < bufLen; j++) {
            bytes.push(memory.loadI32_8u(bufPtr + j));
          }

          const text = String.fromCharCode(...bytes);
          totalWritten += bufLen;

          // Route to stdout or stderr.
          if (fd === 1) {
            self.stdoutCallback(text);
          } else if (fd === 2) {
            self.stderrCallback(text);
          }
          // Other fds are silently ignored.
        }

        // Write the number of bytes written to nwritten_ptr.
        memory.storeI32(nwrittenPtr, totalWritten);

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── fd_read ───────────────────────────────────────────────────────
  //
  // fd_read(fd: i32, iovs_ptr: i32, iovs_len: i32, nread_ptr: i32) -> i32
  //
  // Reads bytes into guest memory buffers. Only fd=0 (stdin) is supported.
  //
  private makeFdRead(): HostFunction {
    const self = this;
    return {
      type: makeFuncType(
        [ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32],
        [ValueType.I32],
      ),
      call(args: WasmValue[]): WasmValue[] {
        const fd = args[0].value as number;
        const iovsPtr = args[1].value as number;
        const iovsLen = args[2].value as number;
        const nreadPtr = args[3].value as number;

        if (!self.instanceMemory) {
          return [i32(ENOSYS)];
        }
        if (fd !== 0) {
          return [i32(EBADF)];
        }

        const memory = self.instanceMemory;
        let totalRead = 0;

        for (let i = 0; i < iovsLen; i++) {
          const bufPtr = memory.loadI32(iovsPtr + i * 8) >>> 0;
          const bufLen = memory.loadI32(iovsPtr + i * 8 + 4) >>> 0;
          const chunk = normalizeInputChunk(self.stdinCallback(bufLen), bufLen);

          for (let j = 0; j < chunk.length; j++) {
            memory.storeI32_8(bufPtr + j, chunk[j]);
          }

          totalRead += chunk.length;
          if (chunk.length < bufLen) {
            break;
          }
        }

        memory.storeI32(nreadPtr, totalRead);
        return [i32(ESUCCESS)];
      },
    };
  }

  // ── proc_exit ─────────────────────────────────────────────────────
  //
  // proc_exit(code: i32) -> never
  //
  // Terminates execution with an exit code. Throws ProcExitError which
  // the runtime catches.
  //
  private makeProcExit(): HostFunction {
    return {
      type: makeFuncType([ValueType.I32], []),
      call(args: WasmValue[]): WasmValue[] {
        const exitCode = args[0].value as number;
        throw new ProcExitError(exitCode);
      },
    };
  }

  // ── args_sizes_get ────────────────────────────────────────────────
  //
  // args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) -> errno
  //
  // Reports how much memory the caller needs to allocate before calling
  // args_get. This two-phase pattern (sizes first, then data) is common
  // in WASI: the caller doesn't know buffer sizes in advance, so it asks
  // first, allocates, then calls the data function.
  //
  // The buf_size counts null terminators: each arg needs strlen(arg)+1 bytes.
  //
  private makeArgsSizesGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const argcPtr = args[0].value as number;
        const argvBufSizePtr = args[1].value as number;
        const memory = self.instanceMemory;

        // Count of arguments.
        memory.storeI32(argcPtr, self.args.length);

        // Total buffer size: each arg as UTF-8 bytes plus a null terminator.
        // The null terminator (0x00) separates args in the contiguous buffer
        // that args_get will write into.
        const encoder = new TextEncoder();
        let bufSize = 0;
        for (const arg of self.args) {
          bufSize += encoder.encode(arg).length + 1; // +1 for '\0'
        }
        memory.storeI32(argvBufSizePtr, bufSize);

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── args_get ──────────────────────────────────────────────────────
  //
  // args_get(argv: i32, argv_buf: i32) -> errno
  //
  // Fills two regions in the caller's memory:
  //
  //   argv_buf: contiguous null-terminated UTF-8 strings
  //     [arg0\0][arg1\0][arg2\0]...
  //
  //   argv: array of i32 pointers into argv_buf, one per arg
  //     [ptr_to_arg0][ptr_to_arg1][ptr_to_arg2]...
  //
  // This layout matches the C `char *argv[]` convention used by the
  // C standard library's startup code (crt0). The WASM program uses argv
  // to find each argument without knowing their lengths in advance.
  //
  private makeArgsGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const argvPtr = args[0].value as number;    // pointer-array base
        const argvBufPtr = args[1].value as number; // string data base
        const memory = self.instanceMemory;
        const encoder = new TextEncoder();

        let offset = argvBufPtr;
        for (let i = 0; i < self.args.length; i++) {
          // Write the pointer to this arg's start into the argv array.
          // argv[i] is at argvPtr + i*4 (each pointer is an i32 = 4 bytes).
          memory.storeI32(argvPtr + i * 4, offset);

          // Encode the arg as UTF-8 and write each byte, then a null terminator.
          const encoded = encoder.encode(self.args[i]);
          for (const byte of encoded) {
            memory.storeI32_8(offset++, byte);
          }
          memory.storeI32_8(offset++, 0); // null terminator '\0'
        }

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── environ_sizes_get ─────────────────────────────────────────────
  //
  // environ_sizes_get(environ_count_ptr: i32, environ_buf_size_ptr: i32) -> errno
  //
  // Same shape as args_sizes_get but for environment variables.
  // Each env var is encoded as "KEY=VALUE\0" in the buffer.
  //
  // This matches the POSIX `environ` convention: the shell passes key=value
  // pairs to child processes in exactly this format.
  //
  private makeEnvironSizesGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const environCountPtr = args[0].value as number;
        const environBufSizePtr = args[1].value as number;
        const memory = self.instanceMemory;
        const encoder = new TextEncoder();

        const entries = Object.entries(self.env);

        // Count of environment variables.
        memory.storeI32(environCountPtr, entries.length);

        // Total buffer size for "KEY=VALUE\0" strings.
        let bufSize = 0;
        for (const [key, value] of entries) {
          bufSize += encoder.encode(`${key}=${value}`).length + 1; // +1 for '\0'
        }
        memory.storeI32(environBufSizePtr, bufSize);

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── environ_get ───────────────────────────────────────────────────
  //
  // environ_get(environ: i32, environ_buf: i32) -> errno
  //
  // Fills the environ pointer-array and string buffer — same layout as
  // args_get but strings are "KEY=VALUE\0".
  //
  // The C runtime uses this layout to populate the `environ` global
  // variable that `getenv()` and `putenv()` operate on.
  //
  private makeEnvironGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const environPtr = args[0].value as number;    // pointer-array base
        const environBufPtr = args[1].value as number; // string data base
        const memory = self.instanceMemory;
        const encoder = new TextEncoder();

        const entries = Object.entries(self.env);
        let offset = environBufPtr;

        for (let i = 0; i < entries.length; i++) {
          const [key, value] = entries[i];
          const str = `${key}=${value}`;

          // Write the pointer to this env string into the environ pointer array.
          memory.storeI32(environPtr + i * 4, offset);

          // Encode and write the "KEY=VALUE" bytes followed by a null terminator.
          const encoded = encoder.encode(str);
          for (const byte of encoded) {
            memory.storeI32_8(offset++, byte);
          }
          memory.storeI32_8(offset++, 0); // null terminator '\0'
        }

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── clock_res_get ─────────────────────────────────────────────────
  //
  // clock_res_get(id: i32, resolution_ptr: i32) -> errno
  //
  // Writes the clock resolution (smallest measurable time increment) as
  // a u64 nanoseconds value to memory at resolution_ptr.
  //
  // This lets programs decide how to use time: e.g., don't busy-wait if
  // the resolution is coarse. For our stub, we always report 1ms = 1,000,000ns
  // because that's the practical floor on most platforms.
  //
  private makeClockResGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const clockId = args[0].value as number;
        const resolutionPtr = args[1].value as number;
        const memory = self.instanceMemory;

        // Ask the injected clock for its resolution.
        // storeI64 writes a bigint as a little-endian 64-bit value.
        const resNs = self.clock.resolutionNs(clockId);
        memory.storeI64(resolutionPtr, resNs);

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── clock_time_get ────────────────────────────────────────────────
  //
  // clock_time_get(id: i32, precision: i64, time_ptr: i32) -> errno
  //
  // Writes the current time for the given clock as nanoseconds into memory.
  // The `precision` argument is a hint for the desired precision (in ns);
  // we ignore it since we return the best precision we have anyway.
  //
  // Clock IDs (from the WASI spec):
  //   0 = REALTIME  — wall clock (nanoseconds since Unix epoch)
  //   1 = MONOTONIC — monotonic clock (nanoseconds since arbitrary start)
  //   2 = PROCESS_CPUTIME_ID — CPU time for this process
  //   3 = THREAD_CPUTIME_ID  — CPU time for this thread
  //
  // For 2 and 3 we approximate with the monotonic clock — a reasonable
  // approximation for single-threaded WASM programs.
  //
  private makeClockTimeGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType(
        [ValueType.I32, ValueType.I64, ValueType.I32],
        [ValueType.I32],
      ),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const clockId = args[0].value as number;
        // args[1] is the precision hint (i64) — we receive it but ignore it.
        const timePtr = args[2].value as number;
        const memory = self.instanceMemory;

        let timeNs: bigint;
        switch (clockId) {
          case 0: // REALTIME — nanoseconds since 1970-01-01 00:00:00 UTC
            timeNs = self.clock.realtimeNs();
            break;
          case 1: // MONOTONIC — nanoseconds since process/VM start
          case 2: // PROCESS_CPUTIME_ID — approximate with monotonic
          case 3: // THREAD_CPUTIME_ID  — approximate with monotonic
            timeNs = self.clock.monotonicNs();
            break;
          default:
            // Unknown clock ID — return EINVAL per the WASI spec.
            return [i32(EINVAL)];
        }

        // Write the 64-bit nanosecond timestamp as a little-endian i64.
        memory.storeI64(timePtr, timeNs);

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── random_get ────────────────────────────────────────────────────
  //
  // random_get(buf: i32, buf_len: i32) -> errno
  //
  // Fills buf_len bytes of linear memory starting at buf with random bytes.
  // Used by programs for: UUIDs, nonces, hash seeds, session tokens, etc.
  //
  // The implementation allocates a JS Uint8Array, fills it via the injected
  // WasiRandom, then copies it into WASM linear memory via writeBytes().
  //
  private makeRandomGet(): HostFunction {
    const self = this;
    return {
      type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
      call(args: WasmValue[]): WasmValue[] {
        if (!self.instanceMemory) return [i32(ENOSYS)];

        const bufPtr = args[0].value as number;
        const bufLen = args[1].value as number;
        const memory = self.instanceMemory;

        // Allocate a temporary JS buffer and fill it with random bytes.
        const bytes = new Uint8Array(bufLen);
        self.random.fillBytes(bytes);

        // Copy the random bytes into WASM linear memory.
        memory.writeBytes(bufPtr, bytes);

        return [i32(ESUCCESS)];
      },
    };
  }

  // ── sched_yield ───────────────────────────────────────────────────
  //
  // sched_yield() -> errno
  //
  // In a multi-threaded or multi-process system, sched_yield() surrenders
  // the CPU to another runnable thread. Programs call it in spin-wait loops
  // to avoid busy-waiting and burning CPU.
  //
  // In single-threaded WASM (which we are), there's nothing to yield to.
  // We simply return success. This is correct behavior: POSIX also allows
  // sched_yield() to be a no-op if there are no other runnable threads.
  //
  private makeSchedYield(): HostFunction {
    return {
      type: makeFuncType([], [ValueType.I32]),
      call(_args: WasmValue[]): WasmValue[] {
        return [i32(ESUCCESS)];
      },
    };
  }

  // ── Stub for unimplemented WASI functions ─────────────────────────
  //
  // Returns a generic stub that accepts any number of i32 args and
  // returns ENOSYS. This lets programs that import uncommon WASI functions
  // load without crashing (they'll get an error code instead of a
  // missing-import failure).
  //
  private makeStub(_name: string): HostFunction {
    return {
      type: makeFuncType([], [ValueType.I32]),
      call(_args: WasmValue[]): WasmValue[] {
        return [i32(ENOSYS)];
      },
    };
  }
}

function normalizeInputChunk(
  value: Uint8Array | readonly number[] | string | null | undefined,
  maxLen: number,
): Uint8Array {
  if (value == null) {
    return new Uint8Array(0);
  }

  let bytes: Uint8Array;
  if (typeof value === "string") {
    bytes = new Uint8Array(
      Array.from(value, (char) => char.charCodeAt(0) & 0xff),
    );
  } else if (value instanceof Uint8Array) {
    bytes = value;
  } else {
    bytes = Uint8Array.from(value, (byte) => byte & 0xff);
  }

  if (bytes.length <= maxLen) {
    return bytes;
  }
  return bytes.subarray(0, maxLen);
}

/**
 * Preferred name for the full WASI host surface.
 * `WasiStub` remains as a backwards-compatible alias.
 */
export const WasiHost = WasiStub;
