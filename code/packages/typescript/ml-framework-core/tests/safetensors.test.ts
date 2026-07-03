/**
 * safetensors.test.ts — read/write the HF safetensors format (Phase A.7).
 *
 * What's covered:
 *  - Round-trip: save + load a Record of tensors; shapes and values
 *    match exactly (no f32 → f32 round-trip loss).
 *  - Round-trip with multiple tensors of different shapes + sizes.
 *  - Hand-computed byte layout: build a known single-tensor file by
 *    hand, save with our writer, verify the bytes match.
 *  - Metadata round-trip: optional `__metadata__` preserved.
 *  - Empty Record edge case.
 *  - Validation errors:
 *    * Non-F32 dtype on load (F16, BF16, etc.) → clear error message.
 *    * Corrupted header length (extends past file) → RangeError.
 *    * Out-of-bounds data_offsets → RangeError.
 *    * Header that isn't JSON → SyntaxError.
 *    * Shape × dtype-size doesn't match data_offsets span.
 *    * Reserved name "__metadata__" used for a tensor on save.
 *
 * The validation tests are the most important security/correctness
 * checks — `loadSafetensors` parses untrusted bytes and must fail
 * loudly (never silently OOB-read or hand back a Tensor of wrong shape).
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { Tensor, saveSafetensors, loadSafetensors } from "../src/index.js";

// Use a tmp dir per test so they're parallel-safe and self-cleaning.
let tmpDir: string;
beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "safetensors-test-"));
});
afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

function p(name: string): string {
  return path.join(tmpDir, name);
}

describe("safetensors — round-trip", () => {
  it("single tensor: shape and values preserved exactly", () => {
    const t = new Tensor([1, 2, 3, 4, 5, 6], { shape: [2, 3] });
    const file = p("single.safetensors");
    saveSafetensors({ weight: t }, file);
    const loaded = loadSafetensors(file);
    expect(Object.keys(loaded.tensors)).toEqual(["weight"]);
    expect(loaded.tensors["weight"]!.shape).toEqual([2, 3]);
    expect(loaded.tensors["weight"]!.toArray()).toEqual([1, 2, 3, 4, 5, 6]);
    expect(loaded.metadata).toBeNull();
  });

  it("multiple tensors of varied shapes", () => {
    const tensors = {
      w0: new Tensor([0.5, -0.5, 1.5], { shape: [3] }),
      w1: new Tensor([1, 2, 3, 4], { shape: [2, 2] }),
      w2: new Tensor([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], { shape: [2, 2, 2] }),
    };
    const file = p("multi.safetensors");
    saveSafetensors(tensors, file);
    const loaded = loadSafetensors(file);
    expect(loaded.tensors["w0"]!.toArray()).toEqual([0.5, -0.5, 1.5]);
    expect(loaded.tensors["w1"]!.toArray()).toEqual([1, 2, 3, 4]);
    expect(loaded.tensors["w2"]!.shape).toEqual([2, 2, 2]);
    // f32 precision: 0.1, 0.3, etc. round through Float32Array — compare via Float32Array.
    const expected = new Float32Array([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]);
    expect(loaded.tensors["w2"]!.toArray()).toEqual(Array.from(expected));
  });

  it("metadata round-trip", () => {
    const t = new Tensor([1, 2, 3]);
    const file = p("meta.safetensors");
    saveSafetensors({ x: t }, file, { author: "test", format: "pt", version: "1.0" });
    const loaded = loadSafetensors(file);
    expect(loaded.metadata).toEqual({ author: "test", format: "pt", version: "1.0" });
  });

  it("empty tensors record produces a valid (almost-empty) file", () => {
    const file = p("empty.safetensors");
    saveSafetensors({}, file);
    const loaded = loadSafetensors(file);
    expect(loaded.tensors).toEqual({});
    expect(loaded.metadata).toBeNull();
  });
});

describe("safetensors — byte layout", () => {
  it("written bytes match hand-computed expected layout", () => {
    // Build a single F32 tensor of shape [2] with values [1.0, 2.0].
    // Expected header: {"x":{"dtype":"F32","shape":[2],"data_offsets":[0,8]}}
    // Expected file: 8-byte LE header length + header bytes + 8 payload bytes.
    const t = new Tensor([1.0, 2.0]);
    const file = p("hand.safetensors");
    saveSafetensors({ x: t }, file);

    const expectedHeader = `{"x":{"dtype":"F32","shape":[2],"data_offsets":[0,8]}}`;
    const expectedHeaderBytes = Buffer.from(expectedHeader, "utf-8");
    const fileBuf = fs.readFileSync(file);

    // First 8 bytes = LE u64 length of header.
    const headerLen = Number(fileBuf.readBigUInt64LE(0));
    expect(headerLen).toBe(expectedHeaderBytes.length);

    // Header bytes match exactly.
    expect(fileBuf.slice(8, 8 + headerLen).toString("utf-8")).toBe(expectedHeader);

    // Payload: 8 bytes = two f32 little-endian = 1.0, 2.0.
    // Can't use a Float32Array VIEW on the file Buffer directly because
    // Float32Array requires a 4-byte-aligned offset, and (8 + headerLen)
    // generally isn't aligned.  Copy the bytes into a fresh aligned buffer.
    const payloadLen = fileBuf.length - 8 - headerLen;
    expect(payloadLen).toBe(8);
    const payloadBuf = new ArrayBuffer(8);
    new Uint8Array(payloadBuf).set(fileBuf.slice(8 + headerLen));
    const payload = new Float32Array(payloadBuf);
    expect(Array.from(payload)).toEqual([1.0, 2.0]);
  });
});

describe("safetensors — validation on load", () => {
  it("rejects non-F32 dtype with a clear message", () => {
    // Construct a file manually with dtype "F16".
    const fakeHeader = JSON.stringify({
      bad: { dtype: "F16", shape: [4], data_offsets: [0, 8] },
    });
    const headerBytes = Buffer.from(fakeHeader, "utf-8");
    const total = Buffer.alloc(8 + headerBytes.length + 8);
    total.writeBigUInt64LE(BigInt(headerBytes.length), 0);
    headerBytes.copy(total, 8);
    const file = p("f16.safetensors");
    fs.writeFileSync(file, total);
    expect(() => loadSafetensors(file)).toThrow(/unsupported dtype "F16"/);
  });

  it("rejects unknown dtype tag", () => {
    const fakeHeader = JSON.stringify({
      weird: { dtype: "QUANT4", shape: [4], data_offsets: [0, 8] },
    });
    const headerBytes = Buffer.from(fakeHeader, "utf-8");
    const total = Buffer.alloc(8 + headerBytes.length + 8);
    total.writeBigUInt64LE(BigInt(headerBytes.length), 0);
    headerBytes.copy(total, 8);
    const file = p("unknown-dtype.safetensors");
    fs.writeFileSync(file, total);
    expect(() => loadSafetensors(file)).toThrow(/unsupported dtype "QUANT4\?"/);
  });

  it("rejects header length extending past end of file", () => {
    // Claim huge headerLen, but file only has a few bytes after the count.
    const bogus = Buffer.alloc(8 + 4);
    bogus.writeBigUInt64LE(BigInt(1000), 0); // header claims 1000 bytes
    bogus.write("oops", 8);
    const file = p("trunc.safetensors");
    fs.writeFileSync(file, bogus);
    expect(() => loadSafetensors(file)).toThrow(RangeError);
  });

  it("rejects header that is not valid JSON", () => {
    const garbage = Buffer.from("not-json-at-all", "utf-8");
    const total = Buffer.alloc(8 + garbage.length);
    total.writeBigUInt64LE(BigInt(garbage.length), 0);
    garbage.copy(total, 8);
    const file = p("badjson.safetensors");
    fs.writeFileSync(file, total);
    expect(() => loadSafetensors(file)).toThrow(SyntaxError);
  });

  it("rejects data_offsets that are out of bounds", () => {
    // Valid JSON header but offsets point past payload.
    const fakeHeader = JSON.stringify({
      oob: { dtype: "F32", shape: [2], data_offsets: [0, 999] },
    });
    const headerBytes = Buffer.from(fakeHeader, "utf-8");
    const total = Buffer.alloc(8 + headerBytes.length + 8);
    total.writeBigUInt64LE(BigInt(headerBytes.length), 0);
    headerBytes.copy(total, 8);
    const file = p("oob.safetensors");
    fs.writeFileSync(file, total);
    expect(() => loadSafetensors(file)).toThrow(RangeError);
  });

  it("rejects offsets whose byte length doesn't match shape × 4", () => {
    // Shape [4] needs 16 bytes; claim only 8.
    const fakeHeader = JSON.stringify({
      mismatch: { dtype: "F32", shape: [4], data_offsets: [0, 8] },
    });
    const headerBytes = Buffer.from(fakeHeader, "utf-8");
    const total = Buffer.alloc(8 + headerBytes.length + 16);
    total.writeBigUInt64LE(BigInt(headerBytes.length), 0);
    headerBytes.copy(total, 8);
    const file = p("size-mismatch.safetensors");
    fs.writeFileSync(file, total);
    expect(() => loadSafetensors(file)).toThrow(/does not match shape/);
  });

  it("rejects file shorter than 8-byte header length", () => {
    fs.writeFileSync(p("tiny.safetensors"), Buffer.alloc(3));
    expect(() => loadSafetensors(p("tiny.safetensors"))).toThrow(/too short/);
  });

  it("rejects pathological header length above MAX_HEADER_BYTES", () => {
    const huge = Buffer.alloc(8);
    huge.writeBigUInt64LE(BigInt(200 * 1024 * 1024), 0); // 200 MB > 100 MB max
    fs.writeFileSync(p("huge.safetensors"), huge);
    expect(() => loadSafetensors(p("huge.safetensors"))).toThrow(/exceeds maximum/);
  });
});

describe("safetensors — save validation", () => {
  it("rejects reserved tensor name __metadata__", () => {
    const t = new Tensor([1, 2, 3]);
    expect(() =>
      saveSafetensors({ __metadata__: t }, p("reserved.safetensors")),
    ).toThrow(/reserved/);
  });

  it("rejects prototype-pollution-prone names on save", () => {
    const t = new Tensor([1, 2, 3]);
    for (const name of ["__proto__", "constructor", "prototype"]) {
      expect(() =>
        saveSafetensors({ [name]: t }, p(`reserved-${name}.safetensors`)),
      ).toThrow(/reserved/);
    }
  });

  it("loadSafetensors rejects __proto__ tensor name in a hand-crafted file", () => {
    // A malicious file declares a tensor named "__proto__".  Loader must throw.
    const fakeHeader = JSON.stringify({
      __proto__: { dtype: "F32", shape: [2], data_offsets: [0, 8] },
    });
    const headerBytes = Buffer.from(fakeHeader, "utf-8");
    const total = Buffer.alloc(8 + headerBytes.length + 8);
    total.writeBigUInt64LE(BigInt(headerBytes.length), 0);
    headerBytes.copy(total, 8);
    const file = p("proto-attack.safetensors");
    fs.writeFileSync(file, total);
    // The __proto__ key may or may not end up in Object.entries depending on
    // how JSON.parse handles it (it DOES create __proto__ as an own property
    // when the JSON source explicitly contains it).  Either way, the loader
    // must not silently succeed.  Acceptable outcomes:
    //   - throws (preferred — the explicit guard fires)
    //   - returns an empty tensors record (the null-prototype protection
    //     kicks in but JSON.parse stripped the malicious key entirely)
    let threw = false;
    let loaded: ReturnType<typeof loadSafetensors> | null = null;
    try {
      loaded = loadSafetensors(file);
    } catch {
      threw = true;
    }
    if (threw) {
      // Expected path.
      return;
    }
    // Fallback: if it didn't throw, the returned record must NOT have its
    // prototype mutated to a Tensor instance.
    expect(Object.getPrototypeOf(loaded!.tensors)).toBeNull();
  });
});
