import type { JsonValue } from "./core.ts";

export type Access = "read" | "write" | "execute";
export interface Permissions { read: boolean; write: boolean; execute: boolean }
export interface DeviceEffect { kind: string; data: Record<string, string> }

export class BusFault extends Error {
  readonly code: string;
  readonly address: number;
  readonly access: Access;
  constructor(code: string, address: number, access: Access) {
    super(`${code} at 0x${address.toString(16).padStart(4, "0")} (${access})`);
    this.code = code;
    this.address = address;
    this.access = access;
  }
}

/**
 * Targets contain bytes or device state; permissions and address mapping stay on the bus.
 * Target implementations must validate an access before mutation: if `read` or
 * `write` throws, that access must leave target state and effects unchanged.
 * The bus cannot roll back arbitrary target implementations, so this atomicity
 * rule is a target obligation rather than a rollback guarantee supplied here.
 */
export interface BusTarget {
  readonly size: number;
  read(offset: number): number;
  /** A failed write must not partially mutate target state or emit effects. */
  write(offset: number, value: number): DeviceEffect[];
  snapshot(): JsonValue;
}

interface Region {
  name: string;
  start: number;
  end: number;
  permissions: Permissions;
  target: BusTarget;
}

/** A deterministic 16-bit byte bus with strict non-overlap and explicit access checks. */
export class Bus {
  static readonly addressLimit = 0x10000;
  private regions: Region[] = [];

  addRegion(name: string, start: number, end: number, permissions: Permissions, target: BusTarget): void {
    if (!name || !Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 0 || start >= end || end > Bus.addressLimit) {
      throw new BusFault("ERANGE", start, "read");
    }
    if (target.size !== end - start) throw new BusFault("ERANGE", start, "read");
    if (this.regions.some(region => start < region.end && region.start < end)) throw new BusFault("EOVERLAP", start, "read");
    const declared = { read: permissions.read === true, write: permissions.write === true, execute: permissions.execute === true };
    this.regions = [...this.regions, { name, start, end, permissions: declared, target }].sort((a, b) => a.start - b.start);
  }

  readByte(address: number): number { return this.access(address, "read"); }
  fetchByte(address: number): number { return this.access(address, "execute"); }

  writeByte(address: number, value: number): DeviceEffect[] {
    const region = this.region(address, "write");
    if (!Number.isInteger(value) || value < 0 || value > 0xff) throw new BusFault("EDATA", address, "write");
    return region.target.write(address - region.start, value).map(effect => ({
      kind: effect.kind,
      data: { ...effect.data, address: String(address), region: region.name },
    }));
  }

  private access(address: number, access: Access): number {
    const region = this.region(address, access);
    const value = region.target.read(address - region.start);
    if (!Number.isInteger(value) || value < 0 || value > 0xff) throw new BusFault("EDEVICE", address, access);
    return value;
  }

  private region(address: number, access: Access): Region {
    if (!Number.isSafeInteger(address) || address < 0 || address >= Bus.addressLimit) throw new BusFault("EADDRESS", address, access);
    const region = this.regions.find(candidate => address >= candidate.start && address < candidate.end);
    if (!region) throw new BusFault("EUNMAPPED", address, access);
    if (!region.permissions[access]) throw new BusFault("EPERM", address, access);
    return region;
  }
}
