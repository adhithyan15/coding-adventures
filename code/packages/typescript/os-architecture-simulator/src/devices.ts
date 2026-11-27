import type { JsonValue } from "./core.ts";
import { BusFault, type BusTarget, type DeviceEffect } from "./bus.ts";

/** Mutable byte memory; the board decides whether reads, writes, and fetches are allowed. */
export class Ram implements BusTarget {
  private readonly bytes: Uint8Array;
  readonly size: number;
  constructor(size: number) {
    if (!Number.isSafeInteger(size) || size <= 0) throw new Error("RAM size must be a positive integer");
    this.size = size;
    this.bytes = new Uint8Array(size);
  }
  load(offset: number, image: Uint8Array): void {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset + image.length > this.size) throw new Error("program image does not fit in RAM");
    this.bytes.set(image, offset);
  }
  read(offset: number): number {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset >= this.size) throw new Error("RAM offset is out of range");
    return this.bytes[offset]!;
  }
  write(offset: number, value: number): DeviceEffect[] {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset >= this.size) throw new Error("RAM offset is out of range");
    if (!Number.isInteger(value) || value < 0 || value > 0xff) throw new Error("RAM byte is out of range");
    this.bytes[offset] = value;
    return [];
  }
  snapshot(): JsonValue { return Array.from(this.bytes); }
}

/** A tiny read-only register block carrying the board's console address. */
export class ConfigurationRom implements BusTarget {
  private readonly bytes: Uint8Array;
  readonly size: number;
  constructor(consoleAddress: number) {
    if (!Number.isSafeInteger(consoleAddress) || consoleAddress < 0 || consoleAddress >= 0x10000) {
      throw new Error("console address is outside the 16-bit address width");
    }
    this.size = 2;
    this.bytes = new Uint8Array([consoleAddress & 0xff, (consoleAddress >>> 8) & 0xff]);
  }
  read(offset: number): number {
    if (offset < 0 || offset >= this.size) throw new Error("configuration offset is out of range");
    return this.bytes[offset]!;
  }
  write(_offset: number, _value: number): DeviceEffect[] { throw new Error("configuration ROM is read-only"); }
  snapshot(): JsonValue { return Array.from(this.bytes); }
}

/** Simulated memory-mapped output. It records bytes and never writes to host I/O. */
export class ConsoleDevice implements BusTarget {
  readonly size = 1;
  private readonly bytes: number[] = [];
  read(_offset: number): number { throw new BusFault("EDEVICE", 0, "read"); }
  write(offset: number, value: number): DeviceEffect[] {
    if (offset !== 0) throw new Error("console register offset is out of range");
    if (!Number.isInteger(value) || value < 0 || value > 0xff) throw new Error("console byte is out of range");
    this.bytes.push(value);
    return [{ kind: "console.byte-written", data: { byte: String(value), character: String.fromCharCode(value) } }];
  }
  get outputBytes(): number[] { return [...this.bytes]; }
  get text(): string { return this.bytes.map(byte => String.fromCharCode(byte)).join(""); }
  snapshot(): JsonValue { return [...this.bytes]; }
}
