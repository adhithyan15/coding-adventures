/**
 * WASI Tier 3 Tests — args, environ, clock, random, sched_yield.
 *
 * ==========================================================================
 * Test Strategy
 * ==========================================================================
 *
 * These tests verify the 8 new WASI functions added in Tier 3. Each test:
 *
 * 1. Instantiates WasiStub with a specific config.
 * 2. Sets up a real LinearMemory (from wasm-execution).
 * 3. Calls the host function directly — no need to spin up a full runtime.
 * 4. Reads values back from memory and asserts correctness.
 *
 * **Deterministic clock tests**: We inject a FakeClock that returns fixed
 * nanosecond values. This avoids flakiness from wall-clock variation and
 * makes assertions exact. A FakeClock returning 1_700_000_000_000_000_001n
 * for realtime lets us assert `> 1_700_000_000_000_000_000n` (year 2023).
 *
 * **Memory layout**: We allocate 1 page (64 KiB) and place:
 *   - Result pointers at low addresses (0x0000-0x00FF)
 *   - Data buffers at higher addresses (0x0200+)
 * This avoids overlap between the pointer array and string data.
 *
 * ==========================================================================
 * FakeClock — Deterministic Time for Tests
 * ==========================================================================
 *
 * Injecting a FakeClock is the key technique for testing WASI time functions
 * without flakiness. The FakeClock returns fixed bigint values, so we can
 * assert exact results rather than "greater than some floor".
 *
 * @module
 */

import { describe, it, expect } from "vitest";
import { WasiStub, WasiClock, WasiRandom } from "../src/wasi_stub.js";
import { LinearMemory, i32, i64 } from "@coding-adventures/wasm-execution";

// =========================================================================
// FakeClock — Deterministic Time Source
// =========================================================================
//
// Returns fixed bigint nanosecond values so clock tests are 100% deterministic.
// The realtime value (1_700_000_000_000_000_001n) is "November 14, 2023
// 22:13:20 UTC + 1 nanosecond" — safely after the year 2023 sanity threshold.

class FakeClock implements WasiClock {
  realtimeNs(): bigint {
    // A specific nanosecond timestamp after 2023-01-01.
    // 1_700_000_000_000_000_001n ≈ November 14, 2023 22:13:20 UTC
    return 1_700_000_000_000_000_001n;
  }

  monotonicNs(): bigint {
    // 42 seconds since VM start — a recognizable fixed value.
    return 42_000_000_000n;
  }

  resolutionNs(_id: number): bigint {
    // 1 millisecond = 1,000,000 nanoseconds — the browser security floor.
    return 1_000_000n;
  }
}

// =========================================================================
// FakeRandom — Predictable Bytes for Tests
// =========================================================================
//
// Fills the buffer with a repeating 0xAB pattern. This lets us assert that
// random_get actually writes bytes (they're not all zero) and that it writes
// the right number of bytes.

class FakeRandom implements WasiRandom {
  fillBytes(buf: Uint8Array): void {
    // Fill with 0xAB — a distinctive non-zero, non-FF pattern that's easy
    // to verify in assertions.
    buf.fill(0xAB);
  }
}

// =========================================================================
// args_sizes_get
// =========================================================================
//
// args_sizes_get reports how much memory the caller needs before calling
// args_get. We test with ["myapp", "hello"] — 2 args, 12 bytes total:
//   "myapp\0" = 6 bytes  (5 chars + null terminator)
//   "hello\0" = 6 bytes  (5 chars + null terminator)
//   Total    = 12 bytes

describe("args_sizes_get", () => {
  it("reports correct argc and buffer size for two args", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ args: ["myapp", "hello"] });
    wasi.setMemory(memory);

    const argcPtr = 0x0000;
    const bufSizePtr = 0x0004;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get")!;
    const result = fn.call([i32(argcPtr), i32(bufSizePtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    const argc = memory.loadI32(argcPtr);
    const bufSize = memory.loadI32(bufSizePtr);

    expect(argc).toBe(2);  // two args
    // "myapp\0" (6) + "hello\0" (6) = 12 bytes total
    expect(bufSize).toBe(12);
  });

  it("reports 0 for empty args", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ args: [] });
    wasi.setMemory(memory);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get")!;
    fn.call([i32(0x0000), i32(0x0004)]);

    expect(memory.loadI32(0x0000)).toBe(0); // argc = 0
    expect(memory.loadI32(0x0004)).toBe(0); // buf_size = 0
  });
});

describe("fd_read", () => {
  it("reads one byte into the caller buffer and reports bytes read", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({
      stdin: (_count) => "Z",
    });
    wasi.setMemory(memory);

    const iovsPtr = 0x0100;
    const bufPtr = 0x0200;
    const nreadPtr = 0x0300;

    memory.storeI32(iovsPtr, bufPtr);
    memory.storeI32(iovsPtr + 4, 1);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!;
    const result = fn.call([i32(0), i32(iovsPtr), i32(1), i32(nreadPtr)]);

    expect(result).toEqual([i32(0)]);
    expect(memory.loadI32_8u(bufPtr)).toBe("Z".charCodeAt(0));
    expect(memory.loadI32(nreadPtr)).toBe(1);
  });

  it("returns EBADF for non-stdin file descriptors", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({
      stdin: (_count) => "ignored",
    });
    wasi.setMemory(memory);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!;
    const result = fn.call([i32(1), i32(0), i32(0), i32(0)]);

    expect(result).toEqual([i32(8)]);
  });

  it("treats missing stdin data as EOF", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({
      stdin: (_count) => undefined,
    });
    wasi.setMemory(memory);

    const iovsPtr = 0x0100;
    const bufPtr = 0x0200;
    const nreadPtr = 0x0300;

    memory.storeI32(iovsPtr, bufPtr);
    memory.storeI32(iovsPtr + 4, 4);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!;
    const result = fn.call([i32(0), i32(iovsPtr), i32(1), i32(nreadPtr)]);

    expect(result).toEqual([i32(0)]);
    expect(memory.loadI32(nreadPtr)).toBe(0);
  });
});

// =========================================================================
// args_get
// =========================================================================
//
// args_get fills two regions:
//   - argv_buf: "hi\0there\0" (contiguous null-terminated strings)
//   - argv:     [ptr_to_hi, ptr_to_there] (two i32 pointers into argv_buf)
//
// For args ["hi", "there"]:
//   argv_buf starts at 0x0200
//   - offset 0x0200: 'h' 'i' '\0'
//   - offset 0x0203: 't' 'h' 'e' 'r' 'e' '\0'
//   argv starts at 0x0100
//   - offset 0x0100: 0x0200 (ptr to "hi")
//   - offset 0x0104: 0x0203 (ptr to "there")

describe("args_get", () => {
  it("writes pointers and null-terminated strings to memory", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ args: ["hi", "there"] });
    wasi.setMemory(memory);

    const argvPtr = 0x0100;    // pointer array
    const argvBufPtr = 0x0200; // string data

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "args_get")!;
    const result = fn.call([i32(argvPtr), i32(argvBufPtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    // argv[0] should point to the start of "hi\0" at argvBufPtr + 0
    const ptr0 = memory.loadI32(argvPtr + 0 * 4);
    expect(ptr0).toBe(argvBufPtr);

    // argv[1] should point to "there\0" at argvBufPtr + 3 ("hi\0" = 3 bytes)
    const ptr1 = memory.loadI32(argvPtr + 1 * 4);
    expect(ptr1).toBe(argvBufPtr + 3);

    // Verify "hi\0" in argv_buf starting at ptr0
    expect(memory.loadI32_8u(ptr0 + 0)).toBe("h".charCodeAt(0));
    expect(memory.loadI32_8u(ptr0 + 1)).toBe("i".charCodeAt(0));
    expect(memory.loadI32_8u(ptr0 + 2)).toBe(0); // null terminator

    // Verify "there\0" starting at ptr1
    expect(memory.loadI32_8u(ptr1 + 0)).toBe("t".charCodeAt(0));
    expect(memory.loadI32_8u(ptr1 + 1)).toBe("h".charCodeAt(0));
    expect(memory.loadI32_8u(ptr1 + 2)).toBe("e".charCodeAt(0));
    expect(memory.loadI32_8u(ptr1 + 3)).toBe("r".charCodeAt(0));
    expect(memory.loadI32_8u(ptr1 + 4)).toBe("e".charCodeAt(0));
    expect(memory.loadI32_8u(ptr1 + 5)).toBe(0); // null terminator
  });

  it("handles empty args list without writing anything", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ args: [] });
    wasi.setMemory(memory);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "args_get")!;
    const result = fn.call([i32(0x0100), i32(0x0200)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS — no-op
  });
});

// =========================================================================
// environ_sizes_get
// =========================================================================
//
// environ_sizes_get reports how much memory is needed for the env strings.
// With env = { HOME: "/home/user" }:
//   "HOME=/home/user\0" = 15 bytes (14 chars + null terminator)
//   count = 1

describe("environ_sizes_get", () => {
  it("reports correct count and buffer size for one env var", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ env: { HOME: "/home/user" } });
    wasi.setMemory(memory);

    const countPtr = 0x0000;
    const bufSizePtr = 0x0004;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "environ_sizes_get")!;
    const result = fn.call([i32(countPtr), i32(bufSizePtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    const count = memory.loadI32(countPtr);
    const bufSize = memory.loadI32(bufSizePtr);

    expect(count).toBe(1);
    // "HOME=/home/user\0" = 15 chars + 1 null = 16 bytes
    // H-O-M-E-=-/-h-o-m-e-/-u-s-e-r = 15 characters, plus the null terminator
    expect(bufSize).toBe(16);
  });

  it("reports 0 for empty env", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ env: {} });
    wasi.setMemory(memory);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "environ_sizes_get")!;
    fn.call([i32(0x0000), i32(0x0004)]);

    expect(memory.loadI32(0x0000)).toBe(0); // count = 0
    expect(memory.loadI32(0x0004)).toBe(0); // buf_size = 0
  });
});

// =========================================================================
// environ_get
// =========================================================================
//
// environ_get writes "HOME=/home/user\0" into the buffer and a pointer to
// it in the environ array.

describe("environ_get", () => {
  it("writes env string and pointer to memory", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ env: { HOME: "/home/user" } });
    wasi.setMemory(memory);

    const environPtr = 0x0100;    // pointer array
    const environBufPtr = 0x0200; // string data

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "environ_get")!;
    const result = fn.call([i32(environPtr), i32(environBufPtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    // The pointer at environ[0] should point to the start of the string buffer.
    const ptr0 = memory.loadI32(environPtr);
    expect(ptr0).toBe(environBufPtr);

    // Read "HOME=/home/user\0" from memory as a string.
    const expectedStr = "HOME=/home/user";
    for (let i = 0; i < expectedStr.length; i++) {
      expect(memory.loadI32_8u(environBufPtr + i)).toBe(
        expectedStr.charCodeAt(i),
        `Character mismatch at index ${i}`
      );
    }
    // Null terminator.
    expect(memory.loadI32_8u(environBufPtr + expectedStr.length)).toBe(0);
  });
});

// =========================================================================
// clock_time_get
// =========================================================================
//
// We use FakeClock for all clock tests — deterministic, no wall-time flakiness.
//
// Real-time test: result must be > 1_700_000_000_000_000_000n (after 2023).
// Monotonic test: result must be > 0n (always positive).

describe("clock_time_get", () => {
  it("returns realtime (id=0) as i64 nanoseconds from FakeClock", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ clock: new FakeClock() });
    wasi.setMemory(memory);

    const timePtr = 0x0100;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!;
    // Signature: clock_time_get(id: i32, precision: i64, time_ptr: i32) → errno
    const result = fn.call([i32(0), i64(1000n), i32(timePtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    const timeNs = memory.loadI64(timePtr);
    // FakeClock.realtimeNs() = 1_700_000_000_000_000_001n
    expect(timeNs).toBe(1_700_000_000_000_000_001n);
    // Sanity: after year 2023
    expect(timeNs).toBeGreaterThan(1_700_000_000_000_000_000n);
  });

  it("returns monotonic time (id=1) as a positive i64 from FakeClock", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ clock: new FakeClock() });
    wasi.setMemory(memory);

    const timePtr = 0x0100;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!;
    const result = fn.call([i32(1), i64(0n), i32(timePtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    const timeNs = memory.loadI64(timePtr);
    // FakeClock.monotonicNs() = 42_000_000_000n (42 seconds)
    expect(timeNs).toBe(42_000_000_000n);
    expect(timeNs).toBeGreaterThan(0n);
  });

  it("returns EINVAL (28) for unknown clock id", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ clock: new FakeClock() });
    wasi.setMemory(memory);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!;
    const result = fn.call([i32(99), i64(0n), i32(0x0100)]);

    expect(result).toEqual([i32(28)]); // EINVAL
  });

  it("approximates PROCESS_CPUTIME (id=2) with monotonic clock", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ clock: new FakeClock() });
    wasi.setMemory(memory);

    const timePtr = 0x0100;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!;
    const result = fn.call([i32(2), i64(0n), i32(timePtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS
    // id=2 uses monotonicNs() per the spec comment
    expect(memory.loadI64(timePtr)).toBe(42_000_000_000n);
  });
});

// =========================================================================
// clock_res_get
// =========================================================================
//
// clock_res_get writes the clock resolution as a u64 nanoseconds value.
// Our FakeClock always returns 1_000_000n (1ms) for all IDs.

describe("clock_res_get", () => {
  it("returns 1ms resolution (1_000_000n) for realtime clock (id=0)", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ clock: new FakeClock() });
    wasi.setMemory(memory);

    const resPtr = 0x0100;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_res_get")!;
    const result = fn.call([i32(0), i32(resPtr)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    const resNs = memory.loadI64(resPtr);
    expect(resNs).toBe(1_000_000n);
  });

  it("returns resolution for monotonic clock (id=1)", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ clock: new FakeClock() });
    wasi.setMemory(memory);

    const resPtr = 0x0100;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_res_get")!;
    fn.call([i32(1), i32(resPtr)]);

    expect(memory.loadI64(resPtr)).toBe(1_000_000n);
  });
});

// =========================================================================
// random_get
// =========================================================================
//
// random_get fills a buffer of buf_len bytes with random data.
// We use FakeRandom which fills with 0xAB — verifiable and non-zero.

describe("random_get", () => {
  it("fills 32 bytes with random data from FakeRandom (not all zero)", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ random: new FakeRandom() });
    wasi.setMemory(memory);

    const bufPtr = 0x0200;
    const bufLen = 32;

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "random_get")!;
    const result = fn.call([i32(bufPtr), i32(bufLen)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS

    // Verify 32 bytes were written and they're all 0xAB (FakeRandom pattern).
    let allZero = true;
    for (let i = 0; i < bufLen; i++) {
      const byte = memory.loadI32_8u(bufPtr + i);
      if (byte !== 0) allZero = false;
      // FakeRandom fills with 0xAB.
      expect(byte).toBe(0xAB);
    }
    expect(allZero).toBe(false);
  });

  it("handles zero-length buffer without error", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub({ random: new FakeRandom() });
    wasi.setMemory(memory);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "random_get")!;
    const result = fn.call([i32(0x0200), i32(0)]);

    expect(result).toEqual([i32(0)]); // ESUCCESS
  });
});

// =========================================================================
// sched_yield
// =========================================================================
//
// sched_yield is a no-op in single-threaded WASM. It must return ESUCCESS (0).

describe("sched_yield", () => {
  it("returns i32(0) immediately (no-op)", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "sched_yield")!;
    const result = fn.call([]);
    expect(result).toEqual([i32(0)]);
  });

  it("can be called multiple times without error", () => {
    const wasi = new WasiStub();
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "sched_yield")!;
    // Call it many times — should never throw or change state.
    for (let i = 0; i < 100; i++) {
      expect(fn.call([])).toEqual([i32(0)]);
    }
  });
});

// =========================================================================
// SystemClock — Live System Tests
// =========================================================================
//
// These tests use the real SystemClock (no injection) and verify that real
// timestamps are plausible. We use a floor of 1_700_000_000_000_000_000n
// (November 2023) since the tests run after that date.

describe("SystemClock (real clock)", () => {
  it("realtimeNs() returns a timestamp after year 2023", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub(); // defaults to SystemClock
    wasi.setMemory(memory);

    const timePtr = 0x0100;
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!;
    fn.call([i32(0), i64(0n), i32(timePtr)]);

    const timeNs = memory.loadI64(timePtr);
    // Must be after 2023-11-14 (our reference point for "after 2023").
    expect(timeNs).toBeGreaterThan(1_700_000_000_000_000_000n);
  });

  it("monotonicNs() returns a positive value", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub(); // defaults to SystemClock
    wasi.setMemory(memory);

    const timePtr = 0x0100;
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!;
    fn.call([i32(1), i64(0n), i32(timePtr)]);

    const timeNs = memory.loadI64(timePtr);
    expect(timeNs).toBeGreaterThan(0n);
  });
});

// =========================================================================
// SystemRandom — Live Random Tests
// =========================================================================
//
// Test the real SystemRandom by checking that 32 bytes are not all zero.
// We can't assert specific values (it's random!), but we can confirm that
// the bytes are written and are not trivially bad.

describe("SystemRandom (real random)", () => {
  it("fills buffer with non-all-zero bytes", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub(); // defaults to SystemRandom
    wasi.setMemory(memory);

    const bufPtr = 0x0200;
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "random_get")!;
    fn.call([i32(bufPtr), i32(32)]);

    // With 32 bytes of true random data, the probability of all-zero is
    // 2^-256 ≈ 10^-77. If this test ever fails, buy a lottery ticket.
    let allZero = true;
    for (let i = 0; i < 32; i++) {
      if (memory.loadI32_8u(bufPtr + i) !== 0) {
        allZero = false;
        break;
      }
    }
    expect(allZero).toBe(false);
  });
});

// =========================================================================
// WasiConfig — Default and Injected Config Tests
// =========================================================================

describe("WasiConfig defaults", () => {
  it("defaults to empty args and env", () => {
    const memory = new LinearMemory(1);
    const wasi = new WasiStub(); // no config at all
    wasi.setMemory(memory);

    // args_sizes_get should report 0 args, 0 buffer size.
    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get")!;
    fn.call([i32(0x0000), i32(0x0004)]);
    expect(memory.loadI32(0x0000)).toBe(0);
    expect(memory.loadI32(0x0004)).toBe(0);

    // environ_sizes_get should report 0 env vars.
    const fn2 = wasi.resolveFunction("wasi_snapshot_preview1", "environ_sizes_get")!;
    fn2.call([i32(0x0010), i32(0x0014)]);
    expect(memory.loadI32(0x0010)).toBe(0);
    expect(memory.loadI32(0x0014)).toBe(0);
  });

  it("backwards-compatible: still accepts {stdout, stderr} only", () => {
    const output: string[] = [];
    // The constructor used to only accept {stdout?, stderr?}.
    // WasiConfig is a superset — this must still work.
    const wasi = new WasiStub({ stdout: (t) => output.push(t) });
    const memory = new LinearMemory(1);
    wasi.setMemory(memory);

    // Write a simple iov and call fd_write.
    const text = "hello";
    const textBytes = new TextEncoder().encode(text);
    for (let i = 0; i < textBytes.length; i++) {
      memory.storeI32_8(0x0200 + i, textBytes[i]);
    }
    memory.storeI32(0, 0x0200);
    memory.storeI32(4, textBytes.length);

    const fn = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!;
    const result = fn.call([i32(1), i32(0), i32(1), i32(0x0100)]);

    expect(result).toEqual([i32(0)]);
    expect(output).toEqual(["hello"]);
  });
});
