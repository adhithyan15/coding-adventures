import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  dct,
  dct2d,
  dctI,
  dctIiViaFft,
  dctIiiViaFft,
  dctIv,
  DctNorm,
  DctType,
  dst,
  dstI,
  dstIi,
  dstIii,
  dstIv,
  DstType,
  EmptyInputError,
  idct,
  idct2d,
  idst,
  InvalidInputError,
  MAX_DCT_LEN,
  MAX_SAMPLES,
  parseDstType,
  parseNorm,
  parseType,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening", () => {
  it("rejects empty signals and non-finite floats fail-closed (CWE-20)", () => {
    assert.throws(() => dct([]), EmptyInputError);
    assert.throws(() => idct([]), EmptyInputError);
    assert.throws(() => dst([]), EmptyInputError);
    assert.throws(() => idst([]), EmptyInputError);
    assert.throws(() => dct2d([], 0, 0), EmptyInputError);
    assert.throws(() => idct2d([], 0, 0), EmptyInputError);

    assert.throws(() => dct([NaN, 1, 2]), InvalidInputError);
    assert.throws(() => dct([1, Infinity]), InvalidInputError);
    assert.throws(() => dst([1, -Infinity]), InvalidInputError);
  });

  it("enforces pre-flight bounds checks without allocating (CWE-400)", () => {
    const fakeHugeSignal = {
      length: MAX_DCT_LEN + 1,
      [0]: 1.0,
    };
    assert.throws(() => dct(fakeHugeSignal), InvalidInputError);
    assert.throws(() => idct(fakeHugeSignal), InvalidInputError);

    const fakeHugeSamples = {
      length: MAX_SAMPLES + 1,
      [0]: 1.0,
    };
    assert.throws(() => dct(fakeHugeSamples), InvalidInputError);

    // 2D bounds
    assert.throws(() => dct2d([1], MAX_DCT_LEN + 1, 1), InvalidInputError);
    assert.throws(() => dct2d([1], 1, MAX_DCT_LEN + 1), InvalidInputError);
    assert.throws(() => dct2d([1, 2], 1, 1), InvalidInputError); // length mismatch
    assert.throws(() => dct2d([1], -1, 1), InvalidInputError);
    assert.throws(() => dct2d([1], 1.5, 1), InvalidInputError);
  });

  it("rejects unknown types and norms fail-closed", () => {
    assert.throws(() => parseType("invalid"), InvalidInputError);
    assert.throws(() => parseNorm("invalid"), InvalidInputError);
    assert.throws(() => parseDstType("invalid"), InvalidInputError);
    assert.throws(() => dct([1, 2], 99), InvalidInputError);
    assert.throws(() => idct([1, 2], 99), InvalidInputError);
    assert.throws(() => dst([1, 2], 99), InvalidInputError);
    assert.throws(() => idst([1, 2], 99), InvalidInputError);
    assert.throws(() => dct([1], DctType.I), InvalidInputError); // DCT-I requires N >= 2
  });

  it("verifies required_capabilities.json is pure empty []", () => {
    const capPath = path.resolve(__dirname, "../../required_capabilities.json");
    const content = JSON.parse(fs.readFileSync(capPath, "utf-8"));
    assert.deepEqual(content.capabilities, []);
  });

  it("verifies src has zero forbidden imports or calls (VER08a)", () => {
    const srcDir = path.resolve(__dirname, "../../src");
    const files = fs.readdirSync(srcDir).filter((f) => f.endsWith(".ts"));
    for (const f of files) {
      const code = fs.readFileSync(path.join(srcDir, f), "utf-8");
      // zero node:fs, node:net, node:http, node:child_process, process, eval
      assert.ok(!code.includes('from "node:fs"'));
      assert.ok(!code.includes('from "node:net"'));
      assert.ok(!code.includes('from "node:child_process"'));
      assert.ok(!code.includes("eval("));
      assert.ok(!code.includes("Function("));
      // zero regular expressions (CWE-1333)
      assert.ok(!code.includes("RegExp"));
    }
  });

  it("exercises N = 1 special cases for DCT-III", () => {
    const out = dct([5.0], DctType.III);
    assert.equal(out.length, 1);
    assert.ok(Math.abs(out[0] - 2.5) < 1e-12);
  });

  it("enforces fail-closed validation on low-level transforms, types, and 2D arrays (CWE-20/CWE-400)", () => {
    // Non-integer numeric types
    assert.throws(() => parseType(1.5), InvalidInputError);
    assert.throws(() => parseDstType(2.5), InvalidInputError);

    // Direct low-level DCT validation
    assert.throws(() => dctIiViaFft([NaN], 1, DctNorm.NONE), InvalidInputError);
    assert.throws(() => dctIiViaFft([1], -1, DctNorm.NONE), InvalidInputError);
    assert.throws(() => dctIiViaFft([1], 1.5, DctNorm.NONE), InvalidInputError);
    assert.throws(() => dctIiViaFft([1], MAX_DCT_LEN + 1, DctNorm.NONE), InvalidInputError);
    assert.throws(() => dctIiiViaFft([NaN], 1), InvalidInputError);
    assert.throws(() => dctI([1], 1, DctNorm.NONE), InvalidInputError); // DCT-I requires N >= 2
    assert.throws(() => dctIv([Infinity], 1, DctNorm.NONE), InvalidInputError);

    // Direct low-level DST validation
    assert.throws(() => dstI([1], 1, DctNorm.NONE), InvalidInputError); // DST-I requires N >= 2
    assert.throws(() => dst([1], DstType.I), InvalidInputError); // DST-I requires N >= 2
    assert.throws(() => idst([1], DstType.I), InvalidInputError); // DST-I requires N >= 2
    assert.throws(() => dstIi([NaN], 1, DctNorm.NONE), InvalidInputError);
    assert.throws(() => dstIii([NaN], 1, DctNorm.NONE), InvalidInputError);
    assert.throws(() => dstIv([NaN], 1, DctNorm.NONE), InvalidInputError);

    // 2D image validation with non-finite float
    assert.throws(() => dct2d([1, 2, NaN, 4], 2, 2), InvalidInputError);
    assert.throws(() => idct2d([1, 2, 3, Infinity], 2, 2), InvalidInputError);

    // 2D image bounds: ensure total can exceed MAX_DCT_LEN up to MAX_SAMPLES without throwing
    const fake2dLarge = {
      length: 2048 * 1024, // 2_097_152 > MAX_DCT_LEN (1_048_576), <= MAX_SAMPLES (16_777_216)
      [0]: 1.0,
    };
    // image.length matches height * width but contains undefined elements -> should throw non-finite, NOT length > MAX_DCT_LEN
    assert.throws(() => dct2d(fake2dLarge, 2048, 1024), (err: unknown) => {
      return err instanceof InvalidInputError && err.message.includes("non-finite float");
    });
  });

  it("VER15: rejects > MAX_SAFE_INTEGER (1e100), Symbol, Object.create(null), and DataView cleanly", () => {
    // Number.isSafeInteger checks
    assert.throws(() => dctIiViaFft([1, 2], 1e100, DctNorm.ORTHO), InvalidInputError);
    assert.throws(() => dct2d([1], 1e100, 1), InvalidInputError);
    assert.throws(() => dct2d([1], 1, 1e100), InvalidInputError);
    assert.throws(() => dct({ length: -1 }), InvalidInputError);
    assert.throws(() => dct({ length: 1.5 }), InvalidInputError);
    assert.throws(() => dct2d({ length: -1 }, 1, 1), InvalidInputError);

    // Symbol, Object.create(null), throwing toString must not crash String() or Number()
    const sym = Symbol("hostile");
    const nullProto = Object.create(null);
    const throwingObj = {
      toString() {
        throw new Error("boom");
      },
    };
    for (const hostile of [sym, nullProto, throwingObj]) {
      assert.throws(() => parseType(hostile as DctType), InvalidInputError);
      assert.throws(() => parseNorm(hostile as DctNorm), InvalidInputError);
      assert.throws(() => parseDstType(hostile as DstType), InvalidInputError);
      assert.throws(() => dctIiViaFft([1, 2], hostile as number, DctNorm.ORTHO), InvalidInputError);
      assert.throws(() => dct([hostile as number, 1]), InvalidInputError);
      assert.throws(() => idct([hostile as number, 1]), InvalidInputError);
      assert.throws(() => dst([hostile as number, 1]), InvalidInputError);
      assert.throws(() => idst([hostile as number, 1]), InvalidInputError);
      assert.throws(() => dct2d([hostile as number], 1, 1), InvalidInputError);
      assert.throws(() => dct({ length: hostile as number }), InvalidInputError);
    }

    // ArrayBuffer.isView: DataView, BigInt64Array, oversized typed array pre-flight check
    const dv = new DataView(new ArrayBuffer(16));
    assert.throws(() => dct(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => idct(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => dst(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => dct2d(dv as unknown as Float64Array, 1, 1), InvalidInputError);

    const bi = new BigInt64Array([1n, 2n]);
    assert.throws(() => dct(bi as unknown as Float64Array), InvalidInputError);
    const oversized = new Uint8Array(MAX_DCT_LEN + 1);
    assert.throws(() => dct(oversized as unknown as Float64Array), InvalidInputError);

    // CWE-1321: prototype pollution resistance
    JSON.parse('{"__proto__":{"pollutedDctHostile":true}}');
    assert.equal(({} as { pollutedDctHostile?: boolean }).pollutedDctHostile, undefined);

    // CWE-400: oversized enum strings must reject in O(1) before .trim().toLowerCase()
    const hugeEnum = " ".repeat(50_000) + "ortho" + " ".repeat(50_000);
    assert.throws(() => parseNorm(hugeEnum), InvalidInputError);
    assert.throws(() => parseType(hugeEnum), InvalidInputError);
    assert.throws(() => parseDstType(hugeEnum), InvalidInputError);
  });
});
