/** A compact deterministic model of disk addressing and mechanical timing. */

import {ClockOverflowError, HddError, InvalidLbaError, InvalidSectorDataError} from "./errors.js";
import {calculateIoTiming, type IoTiming} from "./timing.js";

export {ClockOverflowError, HddError, InvalidLbaError, InvalidSectorDataError};
export type {IoTiming};

export interface HddGeometryV1 {
  readonly surfaces: 2;
  readonly tracksPerSurface: 4;
  readonly sectorsPerTrack: 4;
  readonly bytesPerSector: 8;
  readonly seekTicksPerTrack: 3;
  readonly transferTicksPerSector: 1;
  readonly sectorCount: 32;
  readonly sectorsPerCylinder: 8;
}

const HDD_GEOMETRY_V1: HddGeometryV1 = Object.freeze({
  surfaces: 2,
  tracksPerSurface: 4,
  sectorsPerTrack: 4,
  bytesPerSector: 8,
  seekTicksPerTrack: 3,
  transferTicksPerSector: 1,
  sectorCount: 32,
  sectorsPerCylinder: 8,
});

export interface SectorResult {
  /** A fresh byte array; mutating it cannot change the modeled medium. */
  readonly data: Uint8Array;
  readonly timing: IoTiming;
}

export interface HddState {
  readonly tick: number;
  readonly headTrack: number;
  /** Deep-owned numeric arrays; unlike typed arrays, these are immutable. */
  readonly sectors: readonly (readonly number[])[];
}

const Uint8ArrayConstructor = Uint8Array;
const typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype) as object;
const typedArrayLength = Object.getOwnPropertyDescriptor(
  typedArrayPrototype,
  "length",
)?.get;
const typedArrayBuffer = Object.getOwnPropertyDescriptor(
  typedArrayPrototype,
  "buffer",
)?.get;
const typedArrayTag = Object.getOwnPropertyDescriptor(
  typedArrayPrototype,
  Symbol.toStringTag,
)?.get;
const arrayBufferByteLength = Object.getOwnPropertyDescriptor(
  ArrayBuffer.prototype,
  "byteLength",
)?.get;

/** One synchronous disk. Calls complete immediately and preserve call order. */
export class HddSimulator {
  #tick = 0;
  #headTrack = 0;
  #sectors: Uint8Array[] = Array.from(
    {length: HDD_GEOMETRY_V1.sectorCount},
    () => new Uint8Array(HDD_GEOMETRY_V1.bytesPerSector),
  );

  get geometry(): HddGeometryV1 {
    return HDD_GEOMETRY_V1;
  }

  /** Return a deep, caller-proof snapshot of the complete medium state. */
  getState(): HddState {
    const sectors = this.#sectors.map((sector) => Object.freeze(Array.from(sector)));
    return Object.freeze({
      tick: this.#tick,
      headTrack: this.#headTrack,
      sectors: Object.freeze(sectors),
    });
  }

  /** Read one sector and advance seek, rotation, and transfer time. */
  readSector(lba: number): SectorResult {
    const {track, sector} = this.#validateLba(lba);
    const timing = this.#timingFor(track, sector);
    const data = new Uint8Array(this.#sectors[lba]!);
    this.#complete(timing, track);
    return Object.freeze({data, timing});
  }

  /** Validate and copy one sector before atomically updating medium state. */
  writeSector(lba: number, data: Uint8Array): IoTiming {
    const {track, sector} = this.#validateLba(lba);
    const dataLength = this.#validateData(data);
    if (dataLength !== HDD_GEOMETRY_V1.bytesPerSector) {
      throw new InvalidSectorDataError(
        `sector data must contain exactly ${HDD_GEOMETRY_V1.bytesPerSector} bytes`,
      );
    }
    const timing = this.#timingFor(track, sector);
    const copiedData = new Uint8ArrayConstructor(data);
    this.#sectors[lba] = copiedData;
    this.#complete(timing, track);
    return timing;
  }

  #validateData(data: Uint8Array): number {
    // ArrayBuffer.isView rejects proxies without invoking their traps. The
    // prototype test admits only plain Uint8Array, not subclasses or DataView.
    if (!ArrayBuffer.isView(data) || Object.getPrototypeOf(data) !== Uint8Array.prototype) {
      throw new InvalidSectorDataError("sector data must be a plain Uint8Array");
    }
    if (typedArrayTag!.call(data) !== "Uint8Array") {
      throw new InvalidSectorDataError("sector data must be a Uint8Array");
    }
    const buffer = typedArrayBuffer!.call(data) as ArrayBufferLike;
    try {
      // This brand check rejects SharedArrayBuffer even if someone changes its
      // prototype to ArrayBuffer.prototype; instanceof would be fooled.
      arrayBufferByteLength!.call(buffer);
    } catch {
      throw new InvalidSectorDataError("sector data must use an ArrayBuffer");
    }
    // Intrinsic getter ignores user-defined own accessors and reports zero for
    // a detached typed array, which fails the exact eight-byte check below.
    return typedArrayLength!.call(data) as number;
  }

  #validateLba(lba: number): {track: number; sector: number} {
    if (!Number.isSafeInteger(lba)) {
      throw new InvalidLbaError("LBA must be a safe integer sector address");
    }
    if (lba < 0 || lba >= HDD_GEOMETRY_V1.sectorCount) {
      throw new InvalidLbaError("LBA is outside this medium");
    }
    return {
      track: Math.floor(lba / HDD_GEOMETRY_V1.sectorsPerCylinder),
      sector: lba % HDD_GEOMETRY_V1.sectorsPerTrack,
    };
  }

  #timingFor(targetTrack: number, targetSector: number): IoTiming {
    return calculateIoTiming(
      this.#tick,
      this.#headTrack,
      targetTrack,
      targetSector,
      HDD_GEOMETRY_V1.seekTicksPerTrack,
      HDD_GEOMETRY_V1.sectorsPerTrack,
      HDD_GEOMETRY_V1.transferTicksPerSector,
    );
  }

  #complete(timing: IoTiming, targetTrack: number): void {
    this.#tick = timing.endTick;
    this.#headTrack = targetTrack;
  }
}
