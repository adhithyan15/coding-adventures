import { describe, it, expect } from "vitest";
import { Bootloader, defaultBootloaderConfig, DiskImage, DISK_KERNEL_OFFSET, BOOT_PROTOCOL_MAGIC } from "../src/index.js";

describe("Bootloader", () => {
  it("generates non-empty machine code", () => {
    const config = defaultBootloaderConfig();
    config.kernelSize = 64;
    const bl = new Bootloader(config);
    const code = bl.generate();
    expect(code.length).toBeGreaterThan(0);
    expect(code.length % 4).toBe(0); // instructions are 4 bytes
  });

  it("generateWithComments returns annotated instructions", () => {
    const config = defaultBootloaderConfig();
    config.kernelSize = 64;
    const bl = new Bootloader(config);
    const annotated = bl.generateWithComments();
    expect(annotated.length).toBeGreaterThan(5);
    expect(annotated[0].address).toBe(config.entryAddress);
    expect(annotated[0].comment).toContain("Phase 1");
  });

  it("instructionCount returns correct count", () => {
    const config = defaultBootloaderConfig();
    config.kernelSize = 128;
    const bl = new Bootloader(config);
    expect(bl.instructionCount()).toBeGreaterThan(10);
  });

  it("estimateCycles scales with kernel size", () => {
    const config = defaultBootloaderConfig();
    config.kernelSize = 1024;
    const bl = new Bootloader(config);
    const cycles = bl.estimateCycles();
    expect(cycles).toBeGreaterThan(100);
  });
});

describe("DiskImage", () => {
  it("creates zero-filled disk", () => {
    const disk = new DiskImage(1024);
    expect(disk.size()).toBe(1024);
    expect(disk.readWord(0)).toBe(0);
  });

  it("loadKernel places data at kernel offset", () => {
    const disk = new DiskImage(1024 * 1024);
    disk.loadKernel([0xde, 0xad, 0xbe, 0xef]);
    expect(disk.readWord(DISK_KERNEL_OFFSET)).toBe(0xefbeadde >>> 0);
  });

  it("loadAt places data at specified offset", () => {
    const disk = new DiskImage(1024);
    disk.loadAt(100, [0x42]);
    expect(disk.data[100]).toBe(0x42);
  });

  it("throws if data exceeds disk size", () => {
    const disk = new DiskImage(10);
    expect(() => disk.loadAt(5, new Uint8Array(10))).toThrow("exceeds disk size");
  });
});
