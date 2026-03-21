/**
 * ROM & BIOS -- read-only memory and BIOS firmware for the simulated computer.
 */

// === LE helpers ===
function putLE32(buf: Uint8Array, offset: number, val: number): void {
  buf[offset] = val & 0xff;
  buf[offset + 1] = (val >>> 8) & 0xff;
  buf[offset + 2] = (val >>> 16) & 0xff;
  buf[offset + 3] = (val >>> 24) & 0xff;
}
function readLE32(buf: Uint8Array, offset: number): number {
  return (buf[offset] | (buf[offset + 1] << 8) | (buf[offset + 2] << 16) | (buf[offset + 3] << 24)) >>> 0;
}

// === ROM ===
export const DEFAULT_ROM_BASE = 0xffff0000;
export const DEFAULT_ROM_SIZE = 65536;

export interface ROMConfig { baseAddress: number; size: number; }
export function defaultROMConfig(): ROMConfig { return { baseAddress: DEFAULT_ROM_BASE, size: DEFAULT_ROM_SIZE }; }

export class ROM {
  private readonly config: ROMConfig;
  private readonly data: Uint8Array;

  constructor(config: ROMConfig, firmware: Uint8Array | number[]) {
    if (firmware.length > config.size) throw new Error("firmware larger than ROM size");
    this.config = config;
    this.data = new Uint8Array(config.size);
    for (let i = 0; i < firmware.length; i++) this.data[i] = firmware[i];
  }

  read(address: number): number {
    const offset = address - this.config.baseAddress;
    if (offset < 0 || offset >= this.config.size) return 0;
    return this.data[offset];
  }

  readWord(address: number): number {
    const offset = address - this.config.baseAddress;
    if (offset < 0 || offset + 3 >= this.data.length) return 0;
    return readLE32(this.data, offset);
  }

  write(_address: number, _value: number): void { /* ROM: silently ignored */ }

  size(): number { return this.config.size; }
  baseAddress(): number { return this.config.baseAddress; }
  contains(address: number): boolean {
    const offset = address - this.config.baseAddress;
    return offset >= 0 && offset < this.config.size;
  }
}

// === HardwareInfo ===
export const HARDWARE_INFO_ADDRESS = 0x00001000;
export const HARDWARE_INFO_SIZE = 28;

export interface HardwareInfo {
  memorySize: number;
  displayColumns: number;
  displayRows: number;
  framebufferBase: number;
  idtBase: number;
  idtEntries: number;
  bootloaderEntry: number;
}

export function defaultHardwareInfo(): HardwareInfo {
  return {
    memorySize: 0, displayColumns: 80, displayRows: 25,
    framebufferBase: 0xfffb0000, idtBase: 0, idtEntries: 256,
    bootloaderEntry: 0x00010000,
  };
}

export function hardwareInfoToBytes(h: HardwareInfo): Uint8Array {
  const buf = new Uint8Array(HARDWARE_INFO_SIZE);
  putLE32(buf, 0, h.memorySize);
  putLE32(buf, 4, h.displayColumns);
  putLE32(buf, 8, h.displayRows);
  putLE32(buf, 12, h.framebufferBase);
  putLE32(buf, 16, h.idtBase);
  putLE32(buf, 20, h.idtEntries);
  putLE32(buf, 24, h.bootloaderEntry);
  return buf;
}

export function hardwareInfoFromBytes(data: Uint8Array): HardwareInfo {
  if (data.length < HARDWARE_INFO_SIZE) throw new Error("data too short");
  return {
    memorySize: readLE32(data, 0),
    displayColumns: readLE32(data, 4),
    displayRows: readLE32(data, 8),
    framebufferBase: readLE32(data, 12),
    idtBase: readLE32(data, 16),
    idtEntries: readLE32(data, 20),
    bootloaderEntry: readLE32(data, 24),
  };
}

// === BIOS Config ===
export interface BIOSConfig {
  memorySize: number;
  displayColumns: number;
  displayRows: number;
  framebufferBase: number;
  bootloaderEntry: number;
}

export function defaultBIOSConfig(): BIOSConfig {
  return {
    memorySize: 0, displayColumns: 80, displayRows: 25,
    framebufferBase: 0xfffb0000, bootloaderEntry: 0x00010000,
  };
}
