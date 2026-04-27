/**
 * Tests for WasiStub — the minimal WASI host interface.
 *
 * ==========================================================================
 * Test Strategy
 * ==========================================================================
 *
 * The WasiStub implements HostInterface to provide WASI functions to WASM
 * modules. We test each method:
 *
 * 1. **resolveFunction** — returns HostFunction for "fd_write", "proc_exit",
 *    stubs for unknown WASI functions, and undefined for non-WASI modules.
 *
 * 2. **resolveGlobal / resolveMemory / resolveTable** — always return undefined,
 *    since WASI only provides functions.
 *
 * 3. **fd_write** — the most complex: reads iov vectors from linear memory,
 *    decodes text, and routes to stdout/stderr callbacks.
 *
 * 4. **proc_exit** — throws ProcExitError with the requested exit code.
 *
 * 5. **Stub functions** — return ENOSYS (52) for unimplemented syscalls.
 *
 * @module
 */

import { describe, it, expect } from "vitest";
import { WasiHost, WasiStub, ProcExitError } from "../src/wasi_stub.js";
import { LinearMemory, i32 } from "@coding-adventures/wasm-execution";

// =========================================================================
// resolveFunction — Dispatch Logic
// =========================================================================

describe("WasiStub.resolveFunction", () => {
  it("exports WasiHost as the preferred alias", () => {
    expect(WasiHost).toBe(WasiStub);
  });

  it("returns a HostFunction for 'fd_write'", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write");
    expect(fn).toBeDefined();
    expect(fn!.type.params.length).toBe(4); // fd, iovs_ptr, iovs_len, nwritten_ptr
    expect(fn!.type.results.length).toBe(1); // errno return
  });

  it("returns a HostFunction for 'fd_read'", () => {
    const wasi = new WasiHost();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read");
    expect(fn).toBeDefined();
    expect(fn!.type.params.length).toBe(4);
    expect(fn!.type.results.length).toBe(1);
  });

  it("returns a HostFunction for 'proc_exit'", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "proc_exit");
    expect(fn).toBeDefined();
    expect(fn!.type.params.length).toBe(1); // exit code
    expect(fn!.type.results.length).toBe(0); // no return (throws)
  });

  it("returns a stub for unknown WASI functions", () => {
    const wasi = new WasiStub();
    // fd_seek is not implemented in Tier 3 — it returns the generic ENOSYS stub.
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_seek");
    expect(fn).toBeDefined();
    // Stub returns ENOSYS (52)
    const result = fn!.call([]);
    expect(result).toEqual([i32(52)]);
  });

  it("returns undefined for non-WASI modules", () => {
    const wasi = new WasiStub();
    expect(wasi.resolveFunction("env", "fd_write")).toBeUndefined();
    expect(wasi.resolveFunction("env", "memory")).toBeUndefined();
    expect(wasi.resolveFunction("js", "log")).toBeUndefined();
  });
});

// =========================================================================
// resolveGlobal / resolveMemory / resolveTable — Always Undefined
// =========================================================================

describe("WasiStub resolve stubs", () => {
  it("resolveGlobal returns undefined", () => {
    const wasi = new WasiStub();
    expect(wasi.resolveGlobal("wasi_snapshot_preview1", "stack_pointer")).toBeUndefined();
    expect(wasi.resolveGlobal("env", "global")).toBeUndefined();
  });

  it("resolveMemory returns undefined", () => {
    const wasi = new WasiStub();
    expect(wasi.resolveMemory("wasi_snapshot_preview1", "memory")).toBeUndefined();
    expect(wasi.resolveMemory("env", "memory")).toBeUndefined();
  });

  it("resolveTable returns undefined", () => {
    const wasi = new WasiStub();
    expect(wasi.resolveTable("wasi_snapshot_preview1", "table")).toBeUndefined();
    expect(wasi.resolveTable("env", "table")).toBeUndefined();
  });
});

// =========================================================================
// fd_write — Console Output Capture
// =========================================================================
//
// fd_write reads "iov vectors" from linear memory. Each iov is a pair:
//   (buf_ptr: i32, buf_len: i32) — 8 bytes per entry.
//
// We must set up memory like this:
//   [iovs_ptr]     → buf_ptr (4 bytes LE)
//   [iovs_ptr + 4] → buf_len (4 bytes LE)
//   [buf_ptr]      → actual text bytes
//   [nwritten_ptr] → result: total bytes written (written by fd_write)

describe("fd_write", () => {
  /**
   * Helper to set up memory with iov vectors pointing to text data.
   * Returns { memory, iovsPtr, nwrittenPtr }.
   */
  function setupFdWriteMemory(texts: string[]): {
    memory: LinearMemory;
    iovsPtr: number;
    nwrittenPtr: number;
  } {
    const memory = new LinearMemory(1); // 1 page = 64 KiB

    // Layout:
    //   0x0000 - iov vectors (8 bytes each)
    //   0x0100 - nwritten result
    //   0x0200+ - text data buffers
    const iovsPtr = 0;
    const nwrittenPtr = 0x0100;
    let dataPtr = 0x0200;

    for (let i = 0; i < texts.length; i++) {
      const textBytes = new TextEncoder().encode(texts[i]);

      // Write the text bytes into memory at dataPtr.
      for (let j = 0; j < textBytes.length; j++) {
        memory.storeI32_8(dataPtr + j, textBytes[j]);
      }

      // Write the iov entry: (buf_ptr, buf_len) at iovsPtr + i * 8.
      memory.storeI32(iovsPtr + i * 8, dataPtr);
      memory.storeI32(iovsPtr + i * 8 + 4, textBytes.length);

      dataPtr += textBytes.length;
    }

    return { memory, iovsPtr, nwrittenPtr };
  }

  it("captures stdout output (fd=1) with single iov", () => {
    const output: string[] = [];
    const wasi = new WasiStub({ stdout: (text) => output.push(text) });

    const { memory, iovsPtr, nwrittenPtr } = setupFdWriteMemory(["Hello, World!\n"]);
    wasi.setMemory(memory);

    const fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    const result = fdWrite.call([
      i32(1),              // fd = stdout
      i32(iovsPtr),        // iovs pointer
      i32(1),              // iovs count
      i32(nwrittenPtr),    // nwritten pointer
    ]);

    // Should return ESUCCESS (0).
    expect(result).toEqual([i32(0)]);

    // Should have captured the text.
    expect(output).toEqual(["Hello, World!\n"]);

    // Should have written total byte count to nwritten_ptr.
    const nwritten = memory.loadI32(nwrittenPtr);
    expect(nwritten).toBe(14); // "Hello, World!\n" = 14 bytes
  });

  it("captures stderr output (fd=2)", () => {
    const errors: string[] = [];
    const wasi = new WasiStub({ stderr: (text) => errors.push(text) });

    const { memory, iovsPtr, nwrittenPtr } = setupFdWriteMemory(["Error!"]);
    wasi.setMemory(memory);

    const fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    const result = fdWrite.call([
      i32(2),              // fd = stderr
      i32(iovsPtr),
      i32(1),
      i32(nwrittenPtr),
    ]);

    expect(result).toEqual([i32(0)]);
    expect(errors).toEqual(["Error!"]);
  });

  it("handles multiple iov vectors", () => {
    const output: string[] = [];
    const wasi = new WasiStub({ stdout: (text) => output.push(text) });

    const { memory, iovsPtr, nwrittenPtr } = setupFdWriteMemory(["Hello", ", ", "World!"]);
    wasi.setMemory(memory);

    const fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    fdWrite.call([
      i32(1),
      i32(iovsPtr),
      i32(3),              // 3 iov vectors
      i32(nwrittenPtr),
    ]);

    expect(output).toEqual(["Hello", ", ", "World!"]);
    const nwritten = memory.loadI32(nwrittenPtr);
    expect(nwritten).toBe(13); // 5 + 2 + 6
  });

  it("silently ignores writes to unknown fds", () => {
    const output: string[] = [];
    const errors: string[] = [];
    const wasi = new WasiStub({
      stdout: (text) => output.push(text),
      stderr: (text) => errors.push(text),
    });

    const { memory, iovsPtr, nwrittenPtr } = setupFdWriteMemory(["data"]);
    wasi.setMemory(memory);

    const fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    const result = fdWrite.call([
      i32(42),             // fd = 42 (unknown)
      i32(iovsPtr),
      i32(1),
      i32(nwrittenPtr),
    ]);

    // Still succeeds (bytes are "written" but not captured).
    expect(result).toEqual([i32(0)]);
    expect(output).toEqual([]);
    expect(errors).toEqual([]);
    // But nwritten is still set to the byte count.
    expect(memory.loadI32(nwrittenPtr)).toBe(4);
  });

  it("returns ENOSYS when memory is not set", () => {
    const wasi = new WasiStub();
    // Do NOT call setMemory.

    const fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    const result = fdWrite.call([i32(1), i32(0), i32(1), i32(0)]);

    expect(result).toEqual([i32(52)]); // ENOSYS
  });
});

describe("fd_read", () => {
  it("reads stdin bytes into guest memory", () => {
    const wasi = new WasiHost({
      stdin: () => new TextEncoder().encode("hi"),
    });
    const memory = new LinearMemory(1);
    wasi.setMemory(memory);
    memory.storeI32(0, 0x0200);
    memory.storeI32(4, 2);

    const fdRead = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!;
    const result = fdRead.call([i32(0), i32(0), i32(1), i32(0x0100)]);

    expect(result).toEqual([i32(0)]);
    expect(memory.loadI32(0x0100)).toBe(2);
    expect(memory.loadI32_8u(0x0200)).toBe("h".charCodeAt(0));
    expect(memory.loadI32_8u(0x0201)).toBe("i".charCodeAt(0));
  });

  it("rejects non-stdin file descriptors", () => {
    const wasi = new WasiHost({ stdin: () => new Uint8Array([1]) });
    wasi.setMemory(new LinearMemory(1));
    const fdRead = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!;
    expect(fdRead.call([i32(1), i32(0), i32(0), i32(0)])).toEqual([i32(8)]);
  });
});

// =========================================================================
// proc_exit — Clean Termination
// =========================================================================

describe("proc_exit", () => {
  it("throws ProcExitError with exit code 0", () => {
    const wasi = new WasiStub();
    const procExit = wasi.resolveFunction("wasi_snapshot_preview1", "proc_exit")!;

    expect(() => procExit.call([i32(0)])).toThrow(ProcExitError);

    try {
      procExit.call([i32(0)]);
    } catch (e) {
      expect(e).toBeInstanceOf(ProcExitError);
      expect((e as ProcExitError).exitCode).toBe(0);
    }
  });

  it("throws ProcExitError with non-zero exit code", () => {
    const wasi = new WasiStub();
    const procExit = wasi.resolveFunction("wasi_snapshot_preview1", "proc_exit")!;

    try {
      procExit.call([i32(1)]);
    } catch (e) {
      expect(e).toBeInstanceOf(ProcExitError);
      expect((e as ProcExitError).exitCode).toBe(1);
    }
  });

  it("ProcExitError has correct name and message", () => {
    const err = new ProcExitError(42);
    expect(err.name).toBe("ProcExitError");
    expect(err.message).toBe("proc_exit(42)");
    expect(err.exitCode).toBe(42);
    expect(err).toBeInstanceOf(Error);
  });
});

// =========================================================================
// Stub Functions — ENOSYS for Still-Unimplemented WASI Calls
// =========================================================================
//
// Note: args_get, environ_get, and clock_time_get are now implemented in
// Tier 3 — they no longer return ENOSYS. This suite covers functions that
// are still unimplemented (less common syscalls).

describe("WASI stub functions", () => {
  it("returns ENOSYS (52) for fd_seek (unimplemented)", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_seek")!;
    expect(fn.call([])).toEqual([i32(52)]);
  });

  it("returns ENOSYS (52) for fd_close (unimplemented)", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_close")!;
    expect(fn.call([])).toEqual([i32(52)]);
  });

  it("returns ENOSYS (52) for path_open (unimplemented)", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "path_open")!;
    expect(fn.call([])).toEqual([i32(52)]);
  });
});

// =========================================================================
// Constructor Default Callbacks
// =========================================================================

describe("WasiStub constructor", () => {
  it("works with no options (default callbacks are no-ops)", () => {
    const wasi = new WasiStub();
    const memory = new LinearMemory(1);
    wasi.setMemory(memory);

    const { iovsPtr, nwrittenPtr } = (() => {
      const textBytes = new TextEncoder().encode("test");
      for (let j = 0; j < textBytes.length; j++) {
        memory.storeI32_8(0x0200 + j, textBytes[j]);
      }
      memory.storeI32(0, 0x0200);
      memory.storeI32(4, textBytes.length);
      return { iovsPtr: 0, nwrittenPtr: 0x0100 };
    })();

    const fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    // Should not throw even though no callbacks are provided.
    const result = fdWrite.call([i32(1), i32(iovsPtr), i32(1), i32(nwrittenPtr)]);
    expect(result).toEqual([i32(0)]);
  });
});
