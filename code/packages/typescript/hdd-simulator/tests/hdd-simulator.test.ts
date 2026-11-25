import {readFileSync} from "node:fs";
import {resolve} from "node:path";
import {describe, expect, it} from "vitest";
import {
  ClockOverflowError,
  HddSimulator,
  InvalidLbaError,
  InvalidSectorDataError,
} from "../src/index.js";
import {calculateIoTiming, MAX_SAFE_TICK} from "../src/timing.js";

interface VectorTiming {
  start_tick: number; seek_ticks: number; rotation_ticks: number;
  transfer_ticks: number; total_ticks: number; end_tick: number;
}
interface VectorOperation {
  op: "read" | "write"; lba: number; data_hex?: string;
  timing?: VectorTiming; state?: {tick: number; head_track: number};
  error?: "InvalidLbaError" | "InvalidSectorDataError"; state_unchanged?: boolean;
}
interface VectorFile {
  profile: Record<string, number>;
  cases: {name: string; operations: VectorOperation[]}[];
  timing_probes: {
    name: string; start_tick: number; head_track: number; target_track: number;
    target_sector: number; timing?: VectorTiming; error?: "ClockOverflowError";
  }[];
}
const fixturePath = resolve(process.cwd(), "../../../specs/fixtures/database-media-v1/hdd-simulator.json");
const vectors = JSON.parse(readFileSync(fixturePath, "utf8")) as VectorFile;
const hex = (bytes: Uint8Array): string => Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
const timingRecord = (timing: {startTick: number; seekTicks: number; rotationTicks: number; transferTicks: number; totalTicks: number; endTick: number}): VectorTiming => ({
  start_tick: timing.startTick, seek_ticks: timing.seekTicks, rotation_ticks: timing.rotationTicks,
  transfer_ticks: timing.transferTicks, total_ticks: timing.totalTicks, end_tick: timing.endTick,
});
const snapshot = (disk: HddSimulator): string => JSON.stringify(disk.getState());

describe("shared HDD v1 vectors", () => {
  it("matches shared portable clock boundary probes", () => {
    for (const probe of vectors.timing_probes) {
      if (probe.error) {
        expect(() => calculateIoTiming(probe.start_tick, probe.head_track,
          probe.target_track, probe.target_sector, 3, 4, 1)).toThrow(ClockOverflowError);
      } else {
        const actual = calculateIoTiming(probe.start_tick, probe.head_track,
          probe.target_track, probe.target_sector, 3, 4, 1);
        expect(timingRecord(actual)).toEqual(probe.timing);
      }
    }
  });

  it("matches portable geometry and every operation vector", () => {
    expect(vectors.profile).toEqual({
      surfaces: 2, tracks_per_surface: 4, sectors_per_track: 4,
      bytes_per_sector: 8, seek_ticks_per_track: 3, transfer_ticks_per_sector: 1,
    });
    for (const testCase of vectors.cases) {
      const disk = new HddSimulator();
      for (const operation of testCase.operations) {
        const before = snapshot(disk);
        if (operation.error) {
          const ErrorType = operation.error === "InvalidLbaError" ? InvalidLbaError : InvalidSectorDataError;
          expect(() => {
            if (operation.op === "read") disk.readSector(operation.lba);
            else disk.writeSector(operation.lba, Uint8Array.from(Buffer.from(operation.data_hex ?? "", "hex")));
          }).toThrow(ErrorType);
          expect(snapshot(disk)).toBe(before);
          continue;
        }
        let timing;
        if (operation.op === "read") {
          const result = disk.readSector(operation.lba);
          expect(hex(result.data)).toBe(operation.data_hex);
          timing = result.timing;
        } else {
          timing = disk.writeSector(operation.lba, Uint8Array.from(Buffer.from(operation.data_hex ?? "", "hex")));
        }
        expect(timingRecord(timing)).toEqual(operation.timing);
        expect({tick: disk.getState().tick, head_track: disk.getState().headTrack}).toEqual(operation.state);
      }
    }
  });
});

describe("state, boundaries, and copied byte storage", () => {
  it("starts with zero sectors and returns a deep immutable snapshot", () => {
    const disk = new HddSimulator();
    const state = disk.getState();
    expect(disk.geometry).toEqual({
      surfaces: 2, tracksPerSurface: 4, sectorsPerTrack: 4,
      bytesPerSector: 8, seekTicksPerTrack: 3, transferTicksPerSector: 1,
      sectorCount: 32, sectorsPerCylinder: 8,
    });
    expect(state.tick).toBe(0);
    expect(state.headTrack).toBe(0);
    expect(state.sectors).toHaveLength(32);
    expect(state.sectors.every((sector) => sector.length === 8 && sector.every((b) => b === 0))).toBe(true);
    expect(Object.isFrozen(state)).toBe(true);
    expect(Object.isFrozen(state.sectors)).toBe(true);
    expect(Object.isFrozen(state.sectors[0])).toBe(true);
    disk.writeSector(0, new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]));
    expect(state.sectors[0]).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });

  it.each([-1, 32, 2 ** 80, 0.5, Number.NaN, Number.POSITIVE_INFINITY, "0", true, null])(
    "rejects malformed/out of range LBA %s atomically", (lba) => {
      const disk = new HddSimulator(); const before = snapshot(disk);
      expect(() => disk.readSector(lba as number)).toThrow(InvalidLbaError);
      expect(snapshot(disk)).toBe(before);
    },
  );

  it.each([new Uint8Array(0), new Uint8Array(7), new Uint8Array(9), "abcdefgh", null, 5])(
    "rejects invalid write payload atomically", (data) => {
      const disk = new HddSimulator(); const before = snapshot(disk);
      expect(() => disk.writeSector(0, data as Uint8Array)).toThrow(InvalidSectorDataError);
      expect(snapshot(disk)).toBe(before);
    },
  );

  it("fuzzes malformed requests with a fixed seed and preserves full state", () => {
    let state = 0xBAD5EC7;
    const next = (limit: number): number => {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
      return state % limit;
    };
    const disk = new HddSimulator();
    const badLbas: unknown[] = [true, false, -1, 32, 0.25, Number.NaN, "0", null, {}];
    const badData: unknown[] = [new Uint8Array(0), new Uint8Array(7), new Uint8Array(9), "abcdefgh", null, 7, {}];
    for (let i = 0; i < 64; i += 1) {
      const before = snapshot(disk);
      expect(() => disk.readSector(badLbas[next(badLbas.length)] as number)).toThrow(InvalidLbaError);
      expect(snapshot(disk)).toBe(before);
      expect(() => disk.writeSector(0, badData[next(badData.length)] as Uint8Array)).toThrow(InvalidSectorDataError);
      expect(snapshot(disk)).toBe(before);
    }
  });

  it("rejects proxies, detached/shared buffers, and spoofed typed arrays", () => {
    const disk = new HddSimulator();
    let trapCalls = 0;
    const proxy = new Proxy(new Uint8Array(8), {
      get() { trapCalls += 1; throw new Error("must not invoke proxy trap"); },
      getPrototypeOf() { trapCalls += 1; throw new Error("must not invoke proxy trap"); },
    });
    expect(() => disk.writeSector(0, proxy)).toThrow(InvalidSectorDataError);
    expect(trapCalls).toBe(0);

    const detached = new Uint8Array(8);
    structuredClone(detached.buffer, {transfer: [detached.buffer]});
    expect(() => disk.writeSector(0, detached)).toThrow(InvalidSectorDataError);

    if (typeof SharedArrayBuffer !== "undefined") {
      const sharedBuffer = new SharedArrayBuffer(8);
      const shared = new Uint8Array(sharedBuffer);
      expect(() => disk.writeSector(0, shared)).toThrow(InvalidSectorDataError);
      Object.setPrototypeOf(sharedBuffer, ArrayBuffer.prototype);
      const spoofedBufferView = new Uint8Array(sharedBuffer);
      expect(() => disk.writeSector(0, spoofedBufferView)).toThrow(InvalidSectorDataError);
    }

    const spoofed = new Uint16Array(8);
    Object.setPrototypeOf(spoofed, Uint8Array.prototype);
    expect(() => disk.writeSector(0, spoofed as unknown as Uint8Array)).toThrow(InvalidSectorDataError);
  });

  it("reads typed-array metadata through intrinsics, not own accessors", () => {
    const disk = new HddSimulator();
    const data = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
    Object.defineProperty(data, "length", {get() { throw new Error("accessor called"); }});
    disk.writeSector(0, data);
    expect(disk.readSector(0).data).toEqual(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]));
  });

  it("copies write and read buffers at the API boundary", () => {
    const disk = new HddSimulator();
    const source = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
    disk.writeSector(5, source);
    source[0] = 99;
    const read = disk.readSector(5).data;
    expect(read[0]).toBe(1);
    read[0] = 88;
    expect(disk.readSector(5).data[0]).toBe(1);
  });
});

describe("bounded exhaustive and generative oracles", () => {
  it("enumerates every legal LBA mapping and track/phase placement", () => {
    for (let lba = 0; lba < 32; lba += 1) {
      expect(Math.floor(lba / 8)).toBe(Math.floor(lba / 8));
      expect(lba % 4).toBe(lba % 4);
    }
    for (let phase = 0; phase < 4; phase += 1) {
      for (let head = 0; head < 4; head += 1) {
        for (let target = 0; target < 4; target += 1) {
          for (let sector = 0; sector < 4; sector += 1) {
            const disk = new HddSimulator();
            const setupSector = (phase + 3) % 4;
            const setup = disk.readSector(head * 8 + setupSector);
            const start = setup.timing.endTick;
            expect(start % 4).toBe(phase);
            let tick = start;
            let seek = Math.abs(target - head) * 3;
            const expectedSeek = seek;
            while (seek > 0) { tick += 1; seek -= 1; }
            let rotation = 0;
            while (tick % 4 !== sector) { tick += 1; rotation += 1; }
            tick += 1;
            const timing = disk.readSector(target * 8 + sector).timing;
            expect(timing).toEqual({startTick: start, seekTicks: expectedSeek,
              rotationTicks: rotation, transferTicks: 1,
              totalTicks: tick - start, endTick: tick});
          }
        }
      }
    }
  });

  it("rejects timing beyond the portable exact clock ceiling", () => {
    expect(() => calculateIoTiming(MAX_SAFE_TICK, 0, 0, 0, 3, 4, 1)).toThrow(ClockOverflowError);
    expect(calculateIoTiming(MAX_SAFE_TICK - 1, 0, 0, 2, 3, 4, 1).endTick).toBe(MAX_SAFE_TICK);
  });

  it("matches an independent seeded tick model and deterministic replay", () => {
    let randomState = 0x484444;
    const random = (limit: number): number => {
      randomState = (Math.imul(randomState, 1664525) + 1013904223) >>> 0;
      return randomState % limit;
    };
    const first = new HddSimulator(); const replay = new HddSimulator();
    const media = Array.from({length: 32}, () => new Uint8Array(8));
    let modelTick = 0; let modelHead = 0;
    for (let i = 0; i < 250; i += 1) {
      const lba = random(32); const targetTrack = Math.floor(lba / 8); const targetSector = lba % 4;
      const payload = Uint8Array.from({length: 8}, () => random(256));
      const write = random(2) === 1; const start = modelTick;
      let seek = Math.abs(targetTrack - modelHead) * 3;
      const expectedSeek = seek;
      while (seek > 0) { modelTick += 1; seek -= 1; }
      let rotation = 0;
      while (modelTick % 4 !== targetSector) { modelTick += 1; rotation += 1; }
      modelTick += 1; modelHead = targetTrack;
      let a; let b;
      if (write) { a = first.writeSector(lba, payload); b = replay.writeSector(lba, payload); media[lba] = payload; }
      else { const ra = first.readSector(lba); const rb = replay.readSector(lba); expect(ra.data).toEqual(media[lba]); expect(rb.data).toEqual(ra.data); a = ra.timing; b = rb.timing; }
      expect(a).toEqual(b);
      expect(a).toEqual({startTick: start, seekTicks: expectedSeek, rotationTicks: rotation, transferTicks: 1, totalTicks: modelTick - start, endTick: modelTick});
      expect(first.getState()).toEqual(replay.getState());
      expect(first.getState().tick).toBe(modelTick);
      expect(first.getState().headTrack).toBe(modelHead);
    }
  });
});
