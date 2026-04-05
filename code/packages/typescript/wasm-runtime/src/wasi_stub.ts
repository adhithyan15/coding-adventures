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
 * ```
 *
 * The runtime must provide implementations of these functions. Without them,
 * the program can't print to stdout or exit cleanly.
 *
 * ==========================================================================
 * Chapter 2: This Stub
 * ==========================================================================
 *
 * This is a **minimal stub** — not a full WASI implementation. It provides
 * just enough to:
 *
 * - **fd_write**: Capture stdout/stderr output (for testing and Hello World).
 * - **proc_exit**: Terminate execution with an exit code.
 *
 * Everything else returns ``ENOSYS`` (errno 52 — "function not implemented").
 * This is the honest answer: we acknowledge the syscall exists but haven't
 * implemented it yet.
 *
 * Future versions will expand this as needed. For now, a pure-computation
 * module (like our ``square`` function) doesn't import WASI at all, so this
 * stub is only needed for programs that do I/O.
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
 * A minimal WASI host implementation.
 *
 * Provides ``fd_write`` (captures stdout/stderr) and ``proc_exit`` (terminates
 * execution). All other WASI functions return ENOSYS.
 *
 * **Usage:**
 *
 * ```typescript
 * const output: string[] = [];
 * const wasi = new WasiStub({
 *   stdout: (text) => output.push(text),
 * });
 *
 * const runtime = new WasmRuntime(wasi);
 * runtime.loadAndRun(wasmBytes);
 *
 * console.log(output.join("")); // "Hello, World!\n"
 * ```
 */
export class WasiStub implements HostInterface {
  private readonly stdoutCallback: (text: string) => void;
  private readonly stderrCallback: (text: string) => void;
  private instanceMemory: LinearMemory | null = null;

  constructor(options?: {
    stdout?: (text: string) => void;
    stderr?: (text: string) => void;
  }) {
    this.stdoutCallback = options?.stdout ?? ((_t: string) => {});
    this.stderrCallback = options?.stderr ?? ((_t: string) => {});
  }

  /**
   * Set the instance's memory (needed for fd_write to read iov buffers).
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
      case "proc_exit":
        return this.makeProcExit();
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
