/**
 * Tests for global memory -- VRAM / HBM simulation.
 */

import { describe, it, expect } from "vitest";
import { SimpleGlobalMemory } from "../src/global-memory.js";

// =========================================================================
// Basic read/write
// =========================================================================

describe("read/write", () => {
  it("should write and read back data", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    mem.write(0, new Uint8Array([0x41, 0x42, 0x43, 0x44]));
    const data = mem.read(0, 4);
    expect(data).toEqual(new Uint8Array([0x41, 0x42, 0x43, 0x44]));
  });

  it("should return zeros for uninitialized memory", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    const data = mem.read(0, 8);
    expect(data).toEqual(new Uint8Array(8));
  });

  it("should write and read float via DataView", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    const buf = new ArrayBuffer(4);
    new DataView(buf).setFloat32(0, 3.14, true);
    mem.write(0, new Uint8Array(buf));
    const data = mem.read(0, 4);
    const result = new DataView(data.buffer).getFloat32(0, true);
    expect(Math.abs(result - 3.14)).toBeLessThan(0.01);
  });

  it("should throw on read out of range", () => {
    const mem = new SimpleGlobalMemory({ capacity: 64 });
    expect(() => mem.read(60, 8)).toThrow(RangeError);
  });

  it("should throw on write out of range", () => {
    const mem = new SimpleGlobalMemory({ capacity: 64 });
    expect(() => mem.write(60, new Uint8Array(8))).toThrow(RangeError);
  });

  it("should throw on read with negative address", () => {
    const mem = new SimpleGlobalMemory({ capacity: 64 });
    expect(() => mem.read(-1, 4)).toThrow(RangeError);
  });

  it("should throw on write with negative address", () => {
    const mem = new SimpleGlobalMemory({ capacity: 64 });
    expect(() => mem.write(-1, new Uint8Array(1))).toThrow(RangeError);
  });

  it("should handle multiple writes at different addresses", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    mem.write(0, new Uint8Array([1, 2]));
    mem.write(100, new Uint8Array([3, 4]));
    expect(mem.read(0, 2)).toEqual(new Uint8Array([1, 2]));
    expect(mem.read(100, 2)).toEqual(new Uint8Array([3, 4]));
  });

  it("should overwrite data", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    mem.write(0, new Uint8Array([1, 2]));
    mem.write(0, new Uint8Array([3, 4]));
    expect(mem.read(0, 2)).toEqual(new Uint8Array([3, 4]));
  });
});

// =========================================================================
// Allocation
// =========================================================================

describe("allocation", () => {
  it("should return aligned address", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 * 1024 });
    const addr = mem.allocate(256, 256);
    expect(addr % 256).toBe(0);
  });

  it("should not overlap sequential allocations", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 * 1024 });
    const a1 = mem.allocate(256);
    const a2 = mem.allocate(256);
    expect(a2).toBeGreaterThanOrEqual(a1 + 256);
  });

  it("should throw on out of memory", () => {
    const mem = new SimpleGlobalMemory({ capacity: 512 });
    mem.allocate(256);
    expect(() => mem.allocate(512)).toThrow();
  });

  it("should handle free without crashing", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    const addr = mem.allocate(128);
    mem.free(addr);
    mem.free(addr); // Double free is a no-op
  });

  it("should use default alignment of 256", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 * 1024 });
    const addr = mem.allocate(64);
    expect(addr % 256).toBe(0);
  });
});

// =========================================================================
// Host transfers
// =========================================================================

describe("host transfers", () => {
  it("should copy from host with latency", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      hostBandwidth: 64.0,
      hostLatency: 100,
    });
    const cycles = mem.copyFromHost(0, new Uint8Array(128).fill(1));
    expect(cycles).toBeGreaterThan(0);
    expect(mem.read(0, 4)).toEqual(new Uint8Array([1, 1, 1, 1]));
  });

  it("should copy to host", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      hostBandwidth: 64.0,
      hostLatency: 100,
    });
    mem.write(0, new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd]));
    const [data, cycles] = mem.copyToHost(0, 4);
    expect(data).toEqual(new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd]));
    expect(cycles).toBeGreaterThan(0);
  });

  it("should have zero-cost transfers for unified memory", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024, unified: true });
    const cycles = mem.copyFromHost(0, new Uint8Array(256).fill(1));
    expect(cycles).toBe(0);

    const [data, copyBackCycles] = mem.copyToHost(0, 256);
    expect(copyBackCycles).toBe(0);
    expect(data).toEqual(new Uint8Array(256).fill(1));
  });

  it("should track transfer stats", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      hostBandwidth: 64.0,
      hostLatency: 10,
    });
    mem.copyFromHost(0, new Uint8Array(128));
    const stats = mem.stats;
    expect(stats.hostToDeviceBytes).toBe(128);
    expect(stats.hostTransferCycles).toBeGreaterThan(0);
  });

  it("should track device to host stats", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      hostBandwidth: 64.0,
      hostLatency: 10,
    });
    mem.write(0, new Uint8Array(64));
    mem.copyToHost(0, 64);
    const stats = mem.stats;
    expect(stats.deviceToHostBytes).toBe(64);
  });
});

// =========================================================================
// Coalescing
// =========================================================================

describe("coalescing", () => {
  it("should coalesce fully contiguous access into 1 transaction", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      transactionSize: 128,
    });
    const addrs = Array.from({ length: 32 }, (_, i) => i * 4);
    const transactions = mem.coalesce(addrs);
    expect(transactions.length).toBe(1);
    expect(transactions[0].size).toBe(128);
    expect(transactions[0].address).toBe(0);
  });

  it("should produce many transactions for scattered access", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024 * 1024,
      transactionSize: 128,
    });
    const addrs = [0, 512, 1024, 1536];
    const transactions = mem.coalesce(addrs);
    expect(transactions.length).toBe(4);
  });

  it("should produce 2 transactions for two regions", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      transactionSize: 128,
    });
    const addrs = [
      ...Array.from({ length: 32 }, (_, i) => i * 4),
      ...Array.from({ length: 32 }, (_, i) => 128 + i * 4),
    ];
    const transactions = mem.coalesce(addrs);
    expect(transactions.length).toBe(2);
  });

  it("should set correct thread masks", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      transactionSize: 128,
    });
    const addrs = [0, 4, 256];
    const transactions = mem.coalesce(addrs);
    expect(transactions.length).toBe(2);
    const first = transactions.find((t) => t.address === 0)!;
    expect(first.threadMask & 0b11).toBe(0b11);
  });

  it("should track coalescing stats", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      transactionSize: 128,
    });
    mem.coalesce(Array.from({ length: 32 }, (_, i) => i * 4));
    const stats = mem.stats;
    expect(stats.totalRequests).toBe(32);
    expect(stats.totalTransactions).toBe(1);
    expect(stats.coalescingEfficiency).toBe(32.0);
  });
});

// =========================================================================
// Partition conflicts
// =========================================================================

describe("partition conflicts", () => {
  it("should detect no conflicts when spread across channels", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 1024,
      channels: 4,
      transactionSize: 128,
    });
    const addrs = [0, 128, 256, 384];
    mem.coalesce(addrs);
    const stats = mem.stats;
    expect(stats.partitionConflicts).toBe(0);
  });

  it("should detect partition conflict", () => {
    const mem = new SimpleGlobalMemory({
      capacity: 4096,
      channels: 4,
      transactionSize: 128,
    });
    const addrs = [0, 512];
    mem.coalesce(addrs);
    const stats = mem.stats;
    expect(stats.partitionConflicts).toBeGreaterThanOrEqual(1);
  });
});

// =========================================================================
// Reset
// =========================================================================

describe("reset", () => {
  it("should clear data", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    mem.write(0, new Uint8Array([0xff, 0xff, 0xff, 0xff]));
    mem.reset();
    expect(mem.read(0, 4)).toEqual(new Uint8Array(4));
  });

  it("should clear stats", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    mem.write(0, new Uint8Array(1));
    mem.read(0, 1);
    mem.reset();
    const stats = mem.stats;
    expect(stats.totalReads).toBe(0);
    expect(stats.totalWrites).toBe(0);
  });

  it("should clear allocations", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024 });
    mem.allocate(512);
    mem.reset();
    const addr = mem.allocate(512);
    expect(addr).toBe(0);
  });
});

// =========================================================================
// Properties
// =========================================================================

describe("properties", () => {
  it("should report capacity", () => {
    const mem = new SimpleGlobalMemory({ capacity: 4096 });
    expect(mem.capacity).toBe(4096);
  });

  it("should report bandwidth", () => {
    const mem = new SimpleGlobalMemory({ capacity: 1024, bandwidth: 3350.0 });
    expect(mem.bandwidth).toBe(3350.0);
  });
});
