/**
 * Tests for MemoryManager, Buffer, MappedMemory.
 */

import { describe, it, expect } from "vitest";
import {
  RuntimeInstance,
  MemoryType,
  BufferUsage,
  MappedMemory,
  hasMemoryType,
  hasBufferUsage,
} from "../src/index.js";

function makeDevice(vendor = "nvidia") {
  const instance = new RuntimeInstance();
  const physical = instance.enumeratePhysicalDevices().find((d) => d.vendor === vendor)!;
  return instance.createLogicalDevice(physical);
}

describe("Allocate", () => {
  it("basic allocation works", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(256, MemoryType.DEVICE_LOCAL);
    expect(buf.size).toBe(256);
    expect(buf.deviceAddress).toBeGreaterThanOrEqual(0);
    expect(buf.freed).toBe(false);
    expect(buf.mapped).toBe(false);
  });

  it("allocation with combined usage flags", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      128,
      MemoryType.DEVICE_LOCAL,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_DST,
    );
    expect(hasBufferUsage(buf.usage, BufferUsage.STORAGE)).toBe(true);
    expect(hasBufferUsage(buf.usage, BufferUsage.TRANSFER_DST)).toBe(true);
  });

  it("host visible allocation", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    expect(hasMemoryType(buf.memoryType, MemoryType.HOST_VISIBLE)).toBe(true);
  });

  it("unique buffer IDs", () => {
    const device = makeDevice();
    const b1 = device.memoryManager.allocate(64, MemoryType.DEVICE_LOCAL);
    const b2 = device.memoryManager.allocate(64, MemoryType.DEVICE_LOCAL);
    expect(b1.bufferId).not.toBe(b2.bufferId);
  });

  it("stats tracked", () => {
    const device = makeDevice();
    device.memoryManager.allocate(256, MemoryType.DEVICE_LOCAL);
    device.memoryManager.allocate(128, MemoryType.DEVICE_LOCAL);
    expect(device.stats.totalAllocations).toBe(2);
    expect(device.stats.totalAllocatedBytes).toBe(384);
    expect(device.stats.peakAllocatedBytes).toBe(384);
  });

  it("rejects zero size", () => {
    const device = makeDevice();
    expect(() => device.memoryManager.allocate(0, MemoryType.DEVICE_LOCAL)).toThrow(/positive/);
  });

  it("rejects negative size", () => {
    const device = makeDevice();
    expect(() => device.memoryManager.allocate(-100, MemoryType.DEVICE_LOCAL)).toThrow(/positive/);
  });
});

describe("Free", () => {
  it("basic free works", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(256, MemoryType.DEVICE_LOCAL);
    device.memoryManager.free(buf);
    expect(buf.freed).toBe(true);
    expect(device.stats.totalFrees).toBe(1);
  });

  it("double free throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(256, MemoryType.DEVICE_LOCAL);
    device.memoryManager.free(buf);
    expect(() => device.memoryManager.free(buf)).toThrow(/already freed/);
  });

  it("free while mapped throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.map(buf);
    expect(() => device.memoryManager.free(buf)).toThrow(/still mapped/);
  });

  it("current bytes after free", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(256, MemoryType.DEVICE_LOCAL);
    expect(device.memoryManager.currentAllocatedBytes).toBe(256);
    device.memoryManager.free(buf);
    expect(device.memoryManager.currentAllocatedBytes).toBe(0);
  });

  it("peak bytes preserved after free", () => {
    const device = makeDevice();
    const b1 = device.memoryManager.allocate(256, MemoryType.DEVICE_LOCAL);
    device.memoryManager.allocate(128, MemoryType.DEVICE_LOCAL);
    device.memoryManager.free(b1);
    expect(device.stats.peakAllocatedBytes).toBe(384);
    expect(device.memoryManager.currentAllocatedBytes).toBe(128);
  });
});

describe("Map", () => {
  it("maps host visible buffers", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    expect(mapped).toBeInstanceOf(MappedMemory);
    expect(buf.mapped).toBe(true);
    expect(device.stats.totalMaps).toBe(1);
  });

  it("cannot map DEVICE_LOCAL", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(64, MemoryType.DEVICE_LOCAL);
    expect(() => device.memoryManager.map(buf)).toThrow(/HOST_VISIBLE/);
  });

  it("cannot map freed buffer", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.free(buf);
    expect(() => device.memoryManager.map(buf)).toThrow(/freed/);
  });

  it("cannot double-map", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.map(buf);
    expect(() => device.memoryManager.map(buf)).toThrow(/already mapped/);
  });

  it("unmap works", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.map(buf);
    device.memoryManager.unmap(buf);
    expect(buf.mapped).toBe(false);
  });

  it("unmap not-mapped throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    expect(() => device.memoryManager.unmap(buf)).toThrow(/not mapped/);
  });

  it("Apple unified memory can be mapped", () => {
    const device = makeDevice("apple");
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    expect(mapped).toBeDefined();
    device.memoryManager.unmap(buf);
  });
});

describe("MappedMemory", () => {
  it("read and write", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    mapped.write(0, new Uint8Array(16).fill(0x42));
    const data = mapped.read(0, 16);
    expect(data.every((b) => b === 0x42)).toBe(true);
  });

  it("write at offset", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    mapped.write(32, new Uint8Array(8).fill(0xaa));
    const data = mapped.read(32, 8);
    expect(data.every((b) => b === 0xaa)).toBe(true);
  });

  it("read out of bounds throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      16,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    expect(() => mapped.read(0, 32)).toThrow(/out of bounds/);
  });

  it("write out of bounds throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      16,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    expect(() => mapped.write(0, new Uint8Array(32))).toThrow(/out of bounds/);
  });

  it("dirty flag", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    expect(mapped.dirty).toBe(false);
    mapped.write(0, new Uint8Array([1]));
    expect(mapped.dirty).toBe(true);
  });

  it("getData returns full contents", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      8,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    mapped.write(0, new Uint8Array([1, 2, 3, 4]));
    const data = mapped.getData();
    expect(data.length).toBe(8);
    expect(data[0]).toBe(1);
    expect(data[3]).toBe(4);
  });

  it("size property", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      128,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    expect(mapped.size).toBe(128);
  });
});

describe("Flush and Invalidate", () => {
  it("flush does not throw", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    const mapped = device.memoryManager.map(buf);
    mapped.write(0, new Uint8Array(64).fill(0xff));
    device.memoryManager.flush(buf);
    device.memoryManager.unmap(buf);
  });

  it("invalidate does not throw", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.invalidate(buf);
  });

  it("flush freed buffer throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.free(buf);
    expect(() => device.memoryManager.flush(buf)).toThrow(/freed/);
  });

  it("invalidate freed buffer throws", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
    );
    device.memoryManager.free(buf);
    expect(() => device.memoryManager.invalidate(buf)).toThrow(/freed/);
  });
});

describe("BufferCount", () => {
  it("tracks allocated buffer count", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    expect(mm.allocatedBufferCount).toBe(0);
    const b1 = mm.allocate(64, MemoryType.DEVICE_LOCAL);
    expect(mm.allocatedBufferCount).toBe(1);
    mm.allocate(64, MemoryType.DEVICE_LOCAL);
    expect(mm.allocatedBufferCount).toBe(2);
    mm.free(b1);
    expect(mm.allocatedBufferCount).toBe(1);
  });

  it("getBuffer retrieves by ID", () => {
    const device = makeDevice();
    const buf = device.memoryManager.allocate(64, MemoryType.DEVICE_LOCAL);
    expect(device.memoryManager.getBuffer(buf.bufferId)).toBe(buf);
  });

  it("getBuffer throws for missing ID", () => {
    const device = makeDevice();
    expect(() => device.memoryManager.getBuffer(9999)).toThrow(/not found/);
  });
});
