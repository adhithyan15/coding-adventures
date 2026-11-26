import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  AddressError,
  EraseLimitError,
  GeometryError,
  NANDFlash,
  NANDFlashError,
  PayloadError,
  ProgramError,
} from "../src/index.js";

interface FixtureOperation {
  op: "read" | "program" | "erase_block";
  block: number;
  page?: number;
  data?: string;
  result?: string | null;
  error?: { type: string; message: string };
}
interface Fixture {
  format: string;
  geometry: {
    block_count: number;
    pages_per_block: number;
    page_size_bytes: number;
    max_erase_cycles: number | null;
  };
  initial_state: unknown;
  operations: FixtureOperation[];
  states_after_operations: unknown[];
}

const fixture = JSON.parse(
  readFileSync("../../../specs/fixtures/database-media-v1/nand-flash.json", "utf8"),
) as Fixture;

function createFromFixture(): NANDFlash {
  return new NANDFlash({
    blockCount: fixture.geometry.block_count,
    pagesPerBlock: fixture.geometry.pages_per_block,
    pageSizeBytes: fixture.geometry.page_size_bytes,
    maxEraseCycles: fixture.geometry.max_erase_cycles,
  });
}

function runOperation(device: NANDFlash, operation: FixtureOperation): void | Uint8Array {
  if (operation.op === "read") return device.read(operation.block, operation.page!);
  if (operation.op === "program") {
    const raw = operation.data!;
    const bytes = new Uint8Array(raw.length / 2);
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Number.parseInt(raw.slice(index * 2, index * 2 + 2), 16);
    }
    device.program(operation.block, operation.page!, bytes);
    return;
  }
  device.eraseBlock(operation.block);
}

function mulberry32(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let value = Math.imul(state ^ (state >>> 15), 1 | state);
    value ^= value + Math.imul(value ^ (value >>> 7), 61 | value);
    return ((value ^ (value >>> 14)) >>> 0) / 4_294_967_296;
  };
}

function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

describe("NANDFlash", () => {
  it("matches the shared language-neutral transcript including failures and state", () => {
    expect(fixture.format).toBe("coding-adventures-nand-flash-v1");
    const device = createFromFixture();
    expect(device.snapshot()).toEqual(fixture.initial_state);
    fixture.operations.forEach((operation, index) => {
      if (operation.error) {
        try {
          runOperation(device, operation);
          throw new Error("expected vector command to fail");
        } catch (error) {
          expect(error).toBeInstanceOf(NANDFlashError);
          expect((error as Error).name).toBe(operation.error!.type);
          expect((error as Error).message).toBe(operation.error!.message);
        }
      } else {
        const result = runOperation(device, operation);
        if (operation.op === "read") expect(hex(result as Uint8Array)).toBe(operation.result);
        else expect(result).toBeUndefined();
      }
      expect(device.snapshot()).toEqual(fixture.states_after_operations[index]);
    });
  });

  it("copies reads and every level of snapshot state", () => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 2 });
    expect(device.blockCount).toBe(1);
    expect(device.pagesPerBlock).toBe(1);
    expect(device.pageSizeBytes).toBe(2);
    expect(device.maxEraseCycles).toBeNull();
    const read = device.read(0, 0);
    read.fill(0);
    const snapshot = device.snapshot();
    snapshot.pages[0][0] = "0000";
    snapshot.erase_counts[0] = 42;
    snapshot.programmed[0][0] = true;
    expect(device.snapshot()).toEqual({
      pages: [["ffff"]], erase_counts: [0], programmed: [[false]],
    });
  });

  it.each([
    [0, 1, 1, null], [1, 0, 1, null], [1, 1, 0, null], [true, 1, 1, null],
    [1, 1, 1.5, null], [1025, 1, 1, null], [1, 257, 1, null],
    [1, 1, 65537, null], [1024, 256, 65536, null], [1, 1, 1, -1],
    [1, 1, 1, true], [1, 1, 1, Number.MAX_SAFE_INTEGER + 1],
  ] as const)("rejects invalid geometry %#", (blocks, pages, size, maxCycles) => {
    expect(() => new NANDFlash({
      blockCount: blocks as number, pagesPerBlock: pages as number,
      pageSizeBytes: size as number, maxEraseCycles: maxCycles as number | null,
    })).toThrowError(new GeometryError("geometry values are outside supported bounds"));
  });

  it.each([-1, 1, true, "0", null, 0.5])("rejects invalid block %p without changes", (block) => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    const before = device.snapshot();
    expect(() => device.eraseBlock(block as number)).toThrowError(
      new AddressError("block index out of range"),
    );
    expect(device.snapshot()).toEqual(before);
  });

  it.each([-1, 1, true, "0", null, 0.5])("rejects invalid page %p without changes", (page) => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    const before = device.snapshot();
    expect(() => device.read(0, page as number)).toThrowError(
      new AddressError("page index out of range"),
    );
    expect(device.snapshot()).toEqual(before);
  });

  it("validates addresses before exact Uint8Array payload shape and marker", () => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    const before = device.snapshot();
    expect(() => device.program(2, 2, "x" as unknown as Uint8Array)).toThrowError(
      new AddressError("block index out of range"),
    );
    expect(() => device.program(0, 2, "x" as unknown as Uint8Array)).toThrowError(
      new AddressError("page index out of range"),
    );
    expect(() => device.program(0, 0, [1] as unknown as Uint8Array)).toThrowError(
      new PayloadError("page data must be bytes"),
    );
    expect(() => device.program(0, 0, new Uint8Array(0))).toThrowError(
      new PayloadError("page data must match page_size_bytes"),
    );
    expect(device.snapshot()).toEqual(before);
    device.program(0, 0, new Uint8Array([0xff]));
    const programmed = device.snapshot();
    expect(() => device.program(0, 0, new Uint8Array(0))).toThrowError(
      new PayloadError("page data must match page_size_bytes"),
    );
    expect(device.snapshot()).toEqual(programmed);
  });

  it("rejects Uint8Array subclasses before touching page state", () => {
    class SpecialBytes extends Uint8Array {}
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    const before = device.snapshot();
    expect(() => device.program(0, 0, new SpecialBytes([1]))).toThrowError(
      new PayloadError("page data must be bytes"),
    );
    expect(device.snapshot()).toEqual(before);
  });

  it("ignores an own length accessor and rejects proxy, shared, and detached views", () => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    const before = device.snapshot();
    const accessorBytes = new Uint8Array([0x7f]);
    Object.defineProperty(accessorBytes, "length", {
      get() { throw new Error("caller accessor must not run"); },
    });
    Object.defineProperty(accessorBytes, "constructor", {
      get() { throw new Error("caller constructor accessor must not run"); },
    });
    device.program(0, 0, accessorBytes);
    accessorBytes.fill(0);
    expect(device.read(0, 0)).toEqual(new Uint8Array([0x7f]));

    const other = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    expect(() => other.program(0, 0, new Proxy(new Uint8Array([1]), {}))).toThrowError(
      new PayloadError("page data must be bytes"),
    );
    const forgedKind = new Uint16Array([1]);
    Object.setPrototypeOf(forgedKind, Uint8Array.prototype);
    expect(() => other.program(0, 0, forgedKind as unknown as Uint8Array)).toThrowError(
      new PayloadError("page data must be bytes"),
    );
    const forgedDataView = new DataView(new ArrayBuffer(1));
    Object.setPrototypeOf(forgedDataView, Uint8Array.prototype);
    expect(() => other.program(0, 0, forgedDataView as unknown as Uint8Array)).toThrowError(
      new PayloadError("page data must be bytes"),
    );
    if (typeof SharedArrayBuffer !== "undefined") {
      const shared = new Uint8Array(new SharedArrayBuffer(1));
      expect(() => other.program(0, 0, shared)).toThrowError(
        new PayloadError("page data must be bytes"),
      );
      const spoofedSharedBuffer = new SharedArrayBuffer(1);
      Object.setPrototypeOf(spoofedSharedBuffer, ArrayBuffer.prototype);
      const spoofedShared = new Uint8Array(spoofedSharedBuffer);
      expect(spoofedShared.buffer instanceof ArrayBuffer).toBe(true);
      expect(() => other.program(0, 0, spoofedShared)).toThrowError(
        new PayloadError("page data must be bytes"),
      );
    }
    const detached = new Uint8Array([1]);
    structuredClone(detached.buffer, { transfer: [detached.buffer] });
    expect(() => other.program(0, 0, detached)).toThrowError(
      new PayloadError("page data must match page_size_bytes"),
    );
    expect(other.snapshot()).toEqual(before);
  });

  it("keeps geometry getter-only and state in unforgeable private fields", () => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1 });
    expect(Reflect.set(device, "blockCount", 100)).toBe(false);
    expect(Reflect.set(device, "pages", [])).toBe(true);
    expect(device.blockCount).toBe(1);
    expect(device.snapshot()).toEqual({ pages: [["ff"]], erase_counts: [0], programmed: [[false]] });
  });

  it("consumes a marker on an all-FF write and keeps rejected state atomic", () => {
    const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 2 });
    device.program(0, 0, new Uint8Array([255, 255]));
    const before = device.snapshot();
    expect(() => device.program(0, 0, new Uint8Array([0, 0]))).toThrowError(
      new ProgramError("page has already been programmed since erase"),
    );
    expect(device.snapshot()).toEqual(before);
  });

  it("limits erases independently, including zero and portable safe-counter ceiling", () => {
    const device = new NANDFlash({ blockCount: 2, pagesPerBlock: 1, pageSizeBytes: 1, maxEraseCycles: 1 });
    device.eraseBlock(0);
    device.eraseBlock(1);
    const before = device.snapshot();
    expect(() => device.eraseBlock(0)).toThrowError(new EraseLimitError("block erase limit reached"));
    expect(device.snapshot()).toEqual(before);
    const zero = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1, maxEraseCycles: 0 });
    expect(() => zero.eraseBlock(0)).toThrowError(new EraseLimitError("block erase limit reached"));
    expect(zero.snapshot().erase_counts).toEqual([0]);
    expect(() => new NANDFlash({
      blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1,
      maxEraseCycles: Number.MAX_SAFE_INTEGER + 1,
    })).toThrowError(new GeometryError("geometry values are outside supported bounds"));
  });

  it("checks all length-three traces against an independent integer oracle", () => {
    const actions = ["read", "write_ff", "write_00", "erase"] as const;
    for (const a of actions) for (const b of actions) for (const c of actions) {
      const device = new NANDFlash({ blockCount: 1, pagesPerBlock: 1, pageSizeBytes: 1, maxEraseCycles: 1 });
      let data = 255;
      let written = false;
      let erases = 0;
      for (const action of [a, b, c]) {
        const before = device.snapshot();
        if (action === "read") {
          expect(device.read(0, 0)[0]).toBe(data);
        } else if (action === "write_ff" || action === "write_00") {
          const value = action === "write_ff" ? 255 : 0;
          if (written) {
            expect(() => device.program(0, 0, new Uint8Array([value]))).toThrow(ProgramError);
            expect(device.snapshot()).toEqual(before);
          } else {
            device.program(0, 0, new Uint8Array([value]));
            data &= value;
            written = true;
          }
        } else if (erases === 1) {
          expect(() => device.eraseBlock(0)).toThrow(EraseLimitError);
          expect(device.snapshot()).toEqual(before);
        } else {
          device.eraseBlock(0);
          data = 255;
          written = false;
          erases += 1;
        }
        expect(device.read(0, 0)[0]).toBe(data);
        expect(device.snapshot().erase_counts).toEqual([erases]);
        expect(device.snapshot().programmed).toEqual([[written]]);
      }
    }
  });

  it("replays a seeded generated trace against an independent byte-array oracle", () => {
    const seed = 0xdb01;
    const random = mulberry32(seed);
    const device = new NANDFlash({ blockCount: 3, pagesPerBlock: 2, pageSizeBytes: 4, maxEraseCycles: 3 });
    const pages = Array.from({ length: 3 }, () => Array.from({ length: 2 }, () => [255, 255, 255, 255]));
    const programmed = Array.from({ length: 3 }, () => [false, false]);
    const eraseCounts = [0, 0, 0];
    const trace: string[] = [];
    for (let step = 0; step < 250; step += 1) {
      const block = Math.floor(random() * 3);
      const page = Math.floor(random() * 2);
      const action = ["read", "program", "erase"][Math.floor(random() * 3)];
      trace.push(`${action}:${block}:${page}`);
      if (action === "read") {
        expect(Array.from(device.read(block, page))).toEqual(pages[block][page]);
      } else if (action === "program") {
        const payload = Array.from({ length: 4 }, () => Math.floor(random() * 256));
        if (programmed[block][page]) {
          const before = device.snapshot();
          expect(() => device.program(block, page, new Uint8Array(payload))).toThrow(ProgramError);
          expect(device.snapshot()).toEqual(before);
        } else {
          device.program(block, page, new Uint8Array(payload));
          pages[block][page] = pages[block][page].map((old, index) => old & payload[index]);
          programmed[block][page] = true;
        }
      } else if (eraseCounts[block] === 3) {
        const before = device.snapshot();
        expect(() => device.eraseBlock(block)).toThrow(EraseLimitError);
        expect(device.snapshot()).toEqual(before);
      } else {
        device.eraseBlock(block);
        pages[block] = Array.from({ length: 2 }, () => [255, 255, 255, 255]);
        programmed[block] = [false, false];
        eraseCounts[block] += 1;
      }
      expect(device.snapshot()).toEqual({
        pages: pages.map((blockPages) => blockPages.map((bytes) => hex(new Uint8Array(bytes)))),
        erase_counts: eraseCounts,
        programmed,
      });
    }
    expect(trace).toHaveLength(250);
  });

  it("fuzzes malformed argument shapes deterministically without mutation", () => {
    const random = mulberry32(0xbadda7a);
    const device = new NANDFlash({ blockCount: 2, pagesPerBlock: 2, pageSizeBytes: 3, maxEraseCycles: 2 });
    const badAddresses: unknown[] = [-1, 2, true, false, null, "0", 0.5];
    const badPayloads: unknown[] = [null, "abc", [1, 2, 3], new Uint8Array(0), new Uint8Array(2)];
    for (let index = 0; index < 100; index += 1) {
      const before = device.snapshot();
      const value = badAddresses[Math.floor(random() * badAddresses.length)];
      if (index % 2 === 0) {
        expect(() => device.read(value as number, 0)).toThrow(AddressError);
      } else {
        expect(() => device.read(0, value as number)).toThrow(AddressError);
      }
      expect(device.snapshot()).toEqual(before);
      const block = Math.floor(random() * 2);
      const page = Math.floor(random() * 2);
      const payload = badPayloads[Math.floor(random() * badPayloads.length)];
      expect(() => device.program(block, page, payload as Uint8Array)).toThrow(PayloadError);
      expect(device.snapshot()).toEqual(before);
    }
  });
});
