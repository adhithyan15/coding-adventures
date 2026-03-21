import { describe, it, expect } from "vitest";
import { ROM, defaultROMConfig, hardwareInfoToBytes, hardwareInfoFromBytes, defaultHardwareInfo, HARDWARE_INFO_SIZE } from "../src/index.js";

describe("ROM", () => {
  it("reads firmware bytes at correct addresses", () => {
    const firmware = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
    const rom = new ROM(defaultROMConfig(), firmware);
    expect(rom.read(0xffff0000)).toBe(0xde);
    expect(rom.read(0xffff0003)).toBe(0xef);
  });

  it("readWord returns little-endian word", () => {
    const firmware = new Uint8Array([0x78, 0x56, 0x34, 0x12]);
    const rom = new ROM(defaultROMConfig(), firmware);
    expect(rom.readWord(0xffff0000)).toBe(0x12345678);
  });

  it("write is silently ignored", () => {
    const firmware = new Uint8Array([0xab]);
    const rom = new ROM(defaultROMConfig(), firmware);
    rom.write(0xffff0000, 0xff);
    expect(rom.read(0xffff0000)).toBe(0xab);
  });

  it("out-of-range reads return 0", () => {
    const rom = new ROM(defaultROMConfig(), []);
    expect(rom.read(0x00000000)).toBe(0);
  });

  it("contains checks address range", () => {
    const rom = new ROM(defaultROMConfig(), []);
    expect(rom.contains(0xffff0000)).toBe(true);
    expect(rom.contains(0x00000000)).toBe(false);
  });

  it("throws if firmware exceeds ROM size", () => {
    const cfg = { baseAddress: 0x1000, size: 4 };
    expect(() => new ROM(cfg, new Uint8Array(5))).toThrow("firmware larger");
  });
});

describe("HardwareInfo", () => {
  it("round-trips through toBytes / fromBytes", () => {
    const info = defaultHardwareInfo();
    info.memorySize = 1024 * 1024;
    info.displayColumns = 80;
    info.displayRows = 25;
    const bytes = hardwareInfoToBytes(info);
    expect(bytes.length).toBe(HARDWARE_INFO_SIZE);
    const restored = hardwareInfoFromBytes(bytes);
    expect(restored.memorySize).toBe(1024 * 1024);
    expect(restored.displayColumns).toBe(80);
    expect(restored.displayRows).toBe(25);
    expect(restored.framebufferBase).toBe(0xfffb0000);
    expect(restored.idtEntries).toBe(256);
    expect(restored.bootloaderEntry).toBe(0x00010000);
  });
});
