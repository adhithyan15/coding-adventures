import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";
import { PNG } from "pngjs";
import { describe, expect, it } from "vitest";

import {
  PNG_ERROR_CODES,
  PNG_MAX_DIMENSION,
  PNG_MAX_PIXELS,
  PngError,
  adler32,
  decodePng,
  encodePng,
  type PixelContainer,
} from "../src/index.js";

interface Pixels {
  width: number;
  height: number;
  rgba_hex: string;
}

interface FixtureOptions {
  max_pixels: number;
}

interface FixtureCase {
  id: string;
  operation: "decode" | "decode-error" | "encode" | "encode-error" | "adler32";
  png_hex?: string;
  input_hex?: string;
  input?: Pixels;
  options?: FixtureOptions;
  expected:
    | Pixels
    | { error_id: string }
    | {
        chunk_types: string[];
        filter_types: number[];
        bit_depth: number;
        colour_type: number;
        interlace: number;
      }
    | { adler32_hex: string };
}

interface FixtureDocument {
  limits: { max_dimension: number; default_max_pixels: number };
  error_ids: string[];
  cases: FixtureCase[];
}

const FIXTURE_PATH = fileURLToPath(
  new URL("../../../../specs/fixtures/image-codec-png-v1/cases.json", import.meta.url),
);
const FIXTURES = JSON.parse(readFileSync(FIXTURE_PATH, "utf8")) as FixtureDocument;

function fromHex(value: string): Uint8Array {
  return Uint8Array.from(Buffer.from(value, "hex"));
}

interface PngChunk {
  type: string;
  data: Uint8Array;
}

function chunks(png: Uint8Array): PngChunk[] {
  const result: PngChunk[] = [];
  let offset = 8;
  while (offset < png.length) {
    const length =
      ((png[offset]! << 24) |
        (png[offset + 1]! << 16) |
        (png[offset + 2]! << 8) |
        png[offset + 3]!) >>>
      0;
    result.push({
      type: String.fromCharCode(...png.subarray(offset + 4, offset + 8)),
      data: png.slice(offset + 8, offset + 8 + length),
    });
    offset += 12 + length;
  }
  return result;
}

function makePixels(input: Pixels): PixelContainer {
  return {
    width: input.width,
    height: input.height,
    data: fromHex(input.rgba_hex),
  };
}

function expectPngError(action: () => unknown, errorId: string): void {
  try {
    action();
    throw new Error("fixture unexpectedly succeeded");
  } catch (error: unknown) {
    expect(error).toBeInstanceOf(PngError);
    expect((error as PngError).code).toBe(errorId);
  }
}

describe("IC18 image-codec-png v1 language-neutral fixtures", () => {
  it("pins the public limits and closed error taxonomy", () => {
    expect(PNG_MAX_DIMENSION).toBe(FIXTURES.limits.max_dimension);
    expect(PNG_MAX_PIXELS).toBe(FIXTURES.limits.default_max_pixels);
    expect([...PNG_ERROR_CODES]).toEqual(FIXTURES.error_ids);
  });

  for (const fixture of FIXTURES.cases) {
    if (fixture.operation === "decode") {
      it(`decodes ${fixture.id}`, () => {
        const expected = fixture.expected as Pixels;
        const options = fixture.options
          ? { maxPixels: fixture.options.max_pixels }
          : undefined;
        const actual = decodePng(fromHex(fixture.png_hex!), options);
        expect(actual.width).toBe(expected.width);
        expect(actual.height).toBe(expected.height);
        expect(actual.data).toEqual(fromHex(expected.rgba_hex));
      });
    } else if (fixture.operation === "decode-error") {
      it(`fails closed for ${fixture.id}`, () => {
        const expected = fixture.expected as { error_id: string };
        const options = fixture.options
          ? { maxPixels: fixture.options.max_pixels }
          : undefined;
        expectPngError(
          () => decodePng(fromHex(fixture.png_hex!), options),
          expected.error_id,
        );
      });
    } else if (fixture.operation === "encode") {
      it(`foreign-decodes ${fixture.id}`, () => {
        const input = fixture.input!;
        const expected = fixture.expected as {
          chunk_types: string[];
          filter_types: number[];
          bit_depth: number;
          colour_type: number;
          interlace: number;
        };
        const encoded = encodePng(makePixels(input));
        const encodedChunks = chunks(encoded);
        expect(encodedChunks.map((chunk) => chunk.type)).toEqual(expected.chunk_types);
        expect(encoded[24]).toBe(expected.bit_depth);
        expect(encoded[25]).toBe(expected.colour_type);
        expect(encoded[28]).toBe(expected.interlace);
        const idat = Buffer.concat(
          encodedChunks
            .filter((chunk) => chunk.type === "IDAT")
            .map((chunk) => Buffer.from(chunk.data)),
        );
        const filtered = inflateSync(idat);
        const stride = input.width * 4;
        const filterTypes = Array.from(
          { length: input.height },
          (_, row) => filtered[row * (stride + 1)]!,
        );
        expect(filterTypes).toEqual(expected.filter_types);
        const foreign = PNG.sync.read(Buffer.from(encoded));
        expect(foreign.width).toBe(input.width);
        expect(foreign.height).toBe(input.height);
        expect(Uint8Array.from(foreign.data)).toEqual(fromHex(input.rgba_hex));
      });
    } else if (fixture.operation === "encode-error") {
      it(`rejects encoder input ${fixture.id}`, () => {
        const expected = fixture.expected as { error_id: string };
        expectPngError(() => encodePng(makePixels(fixture.input!)), expected.error_id);
      });
    } else {
      it(`checks Adler-32 for ${fixture.id}`, () => {
        const expected = fixture.expected as { adler32_hex: string };
        expect(adler32(fromHex(fixture.input_hex!)).toString(16).padStart(8, "0"))
          .toBe(expected.adler32_hex);
      });
    }
  }
});
