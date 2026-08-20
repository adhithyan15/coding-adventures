/**
 * image-codec-png.test.ts — IC18 PNG codec tests.
 *
 * Two kinds of test here, and the second kind is the one that matters.
 *
 * Round-trip tests (`encodePng` then `decodePng`) prove the encoder and decoder
 * agree with each other. That is necessary and nowhere near sufficient: two
 * halves of one misunderstanding round-trip perfectly. So the encoder is also
 * checked against Node's `zlib` for its compressed payload, and the decoder is
 * fed PNGs built byte-by-byte to a reading of RFC 2083 rather than to whatever
 * the encoder happens to emit.
 */

import { describe, it, expect } from "vitest";
import { deflateSync, deflateRawSync, inflateSync } from "node:zlib";
import {
  createPixelContainer,
  setPixel,
  pixelAt,
  fillPixels,
} from "@coding-adventures/pixel-container";
import { crc32 } from "@coding-adventures/zip";
import { PngCodec, encodePng, decodePng, adler32 } from "../src/index.js";

const SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/** A small deterministic image with every channel varying. */
function sampleImage(width: number, height: number) {
  const c = createPixelContainer(width, height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      setPixel(c, x, y, (x * 7) & 0xff, (y * 11) & 0xff, (x * y) & 0xff, 255 - ((x + y) & 0x3f));
    }
  }
  return c;
}

function u32be(v: number): number[] {
  return [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
}

/** Build one chunk: length, type, data, CRC over type+data. */
function chunk(type: string, data: number[]): number[] {
  const typed = [...type.split("").map(ch => ch.charCodeAt(0)), ...data];
  return [...u32be(data.length), ...typed, ...u32be(crc32(new Uint8Array(typed)))];
}

/**
 * Assemble a PNG by hand from filtered scanlines, compressing with Node's zlib.
 * This is the foreign encoder the decoder is tested against.
 */
function foreignPng(
  width: number,
  height: number,
  colourType: number,
  channels: number,
  rows: number[][],
  filterTypes: number[],
): Uint8Array {
  const filtered: number[] = [];
  rows.forEach((row, y) => {
    filtered.push(filterTypes[y] ?? 0, ...row);
  });
  const idat = Array.from(deflateSync(Buffer.from(filtered)));
  return new Uint8Array([
    ...SIGNATURE,
    ...chunk("IHDR", [...u32be(width), ...u32be(height), 8, colourType, 0, 0, 0]),
    ...chunk("IDAT", idat),
    ...chunk("IEND", []),
  ]);
}

// --- Adler-32 ----------------------------------------------------------------

describe("adler32", () => {
  it("empty input is 1", () => {
    // a starts at 1 and b at 0, so an empty stream checksums to 0x00000001.
    expect(adler32(new Uint8Array(0))).toBe(1);
  });

  it("matches the RFC 1950 worked example", () => {
    expect(adler32(new TextEncoder().encode("Wikipedia"))).toBe(0x11e60398);
  });

  it("is order-sensitive, unlike a plain sum", () => {
    const a = adler32(new Uint8Array([1, 2, 3]));
    const b = adler32(new Uint8Array([3, 2, 1]));
    expect(a).not.toBe(b);
  });

  it("stays correct across the 5552-byte chunking boundary", () => {
    // The modulo is deferred in blocks; a wrong block size would show up as a
    // mismatch either side of the boundary.
    const data = new Uint8Array(20000);
    for (let i = 0; i < data.length; i++) data[i] = (i * 31) & 0xff;
    // Cross-check against the Adler-32 zlib itself puts in a stream trailer.
    const z = deflateSync(Buffer.from(data));
    const trailer = z.subarray(z.length - 4);
    const expected = ((trailer[0]! << 24) | (trailer[1]! << 16) | (trailer[2]! << 8) | trailer[3]!) >>> 0;
    expect(adler32(data)).toBe(expected);
  });
});

// --- Encoding: structure -----------------------------------------------------

describe("encodePng structure", () => {
  it("starts with the PNG signature", () => {
    const png = encodePng(sampleImage(4, 3));
    expect(Array.from(png.subarray(0, 8))).toEqual(SIGNATURE);
  });

  it("writes IHDR, IDAT and IEND in that order", () => {
    const png = encodePng(sampleImage(4, 3));
    const text = new TextDecoder("latin1").decode(png);
    const ihdr = text.indexOf("IHDR");
    const idat = text.indexOf("IDAT");
    const iend = text.indexOf("IEND");
    expect(ihdr).toBeGreaterThan(0);
    expect(idat).toBeGreaterThan(ihdr);
    expect(iend).toBeGreaterThan(idat);
  });

  it("declares 8-bit colour type 6, no interlace", () => {
    const png = encodePng(sampleImage(5, 2));
    // IHDR data begins 8 (signature) + 4 (length) + 4 (type) = 16 bytes in.
    const d = png.subarray(16, 16 + 13);
    expect(((d[0]! << 24) | (d[1]! << 16) | (d[2]! << 8) | d[3]!) >>> 0).toBe(5);
    expect(((d[4]! << 24) | (d[5]! << 16) | (d[6]! << 8) | d[7]!) >>> 0).toBe(2);
    expect(d[8]).toBe(8);   // bit depth
    expect(d[9]).toBe(6);   // truecolour with alpha
    expect(d[10]).toBe(0);  // deflate
    expect(d[11]).toBe(0);  // adaptive filtering
    expect(d[12]).toBe(0);  // no interlace
  });

  it("every chunk CRC verifies", () => {
    const png = encodePng(sampleImage(9, 7));
    let pos = 8;
    let chunks = 0;
    while (pos < png.length) {
      const len = ((png[pos]! << 24) | (png[pos + 1]! << 16) | (png[pos + 2]! << 8) | png[pos + 3]!) >>> 0;
      const typed = png.subarray(pos + 4, pos + 8 + len);
      const stored =
        ((png[pos + 8 + len]! << 24) | (png[pos + 9 + len]! << 16) |
         (png[pos + 10 + len]! << 8) | png[pos + 11 + len]!) >>> 0;
      expect(crc32(typed)).toBe(stored);
      chunks++;
      pos += 12 + len;
    }
    expect(chunks).toBe(3);
  });

  it("its IDAT payload is a zlib stream Node can inflate", () => {
    // The strongest single check on the encoder: a foreign implementation
    // agrees that what we wrote is a well-formed zlib stream.
    const image = sampleImage(16, 16);
    const png = encodePng(image);
    const text = new TextDecoder("latin1").decode(png);
    const at = text.indexOf("IDAT");
    const len =
      ((png[at - 4]! << 24) | (png[at - 3]! << 16) | (png[at - 2]! << 8) | png[at - 1]!) >>> 0;
    const idat = png.subarray(at + 4, at + 4 + len);

    const filtered = new Uint8Array(inflateSync(Buffer.from(idat)));
    expect(filtered.length).toBe(16 * (16 * 4 + 1));
    // Every row must name a filter in 0..4.
    for (let y = 0; y < 16; y++) {
      expect(filtered[y * (16 * 4 + 1)]!).toBeLessThanOrEqual(4);
    }
  });
});

// --- Round trips -------------------------------------------------------------

describe("round trip", () => {
  it("preserves every pixel of a varied image", () => {
    const image = sampleImage(23, 17);
    const back = decodePng(encodePng(image));
    expect(back.width).toBe(23);
    expect(back.height).toBe(17);
    expect(back.data).toEqual(image.data);
  });

  it("preserves a single pixel", () => {
    const c = createPixelContainer(1, 1);
    setPixel(c, 0, 0, 1, 2, 3, 4);
    expect(pixelAt(decodePng(encodePng(c)), 0, 0)).toEqual([1, 2, 3, 4]);
  });

  it("preserves a one-pixel-wide column and a one-pixel-tall row", () => {
    for (const [w, h] of [[1, 40], [40, 1]] as const) {
      const image = sampleImage(w, h);
      expect(decodePng(encodePng(image)).data).toEqual(image.data);
    }
  });

  it("preserves full transparency and full opacity side by side", () => {
    const c = createPixelContainer(2, 1);
    setPixel(c, 0, 0, 255, 255, 255, 0);
    setPixel(c, 1, 0, 255, 255, 255, 255);
    const back = decodePng(encodePng(c));
    expect(pixelAt(back, 0, 0)).toEqual([255, 255, 255, 0]);
    expect(pixelAt(back, 1, 0)).toEqual([255, 255, 255, 255]);
  });

  it("compresses a flat fill far below its raw size", { timeout: 120_000 }, () => {
    // The whole point of filtering: a uniform image becomes a run of zeroes.
    const c = createPixelContainer(120, 120);
    fillPixels(c, 12, 34, 56, 255);
    const png = encodePng(c);
    expect(png.length).toBeLessThan(c.data.length / 50);
    expect(decodePng(png).data).toEqual(c.data);
  });

  it("round-trips through the PngCodec class", () => {
    const codec = new PngCodec();
    expect(codec.mimeType).toBe("image/png");
    const image = sampleImage(8, 8);
    expect(codec.decode(codec.encode(image)).data).toEqual(image.data);
  });

  it("handles a large image without corrupting the last row", () => {
    // Off-by-one errors in the stride arithmetic show up at the end, not the
    // start, so the final row is checked explicitly.
    const image = sampleImage(150, 100);
    const back = decodePng(encodePng(image));
    for (let x = 0; x < 150; x++) {
      expect(pixelAt(back, x, 99)).toEqual(pixelAt(image, x, 99));
    }
  }, 120_000);
});

// --- Decoding foreign PNGs ---------------------------------------------------
//
// Built by hand from RFC 2083 and compressed with Node's zlib, so these test
// what the format says rather than what our encoder happens to do.

describe("decoding foreign PNGs", () => {
  it("reads truecolour+alpha with every filter type in turn", () => {
    // One row per filter, so each unfiltering branch runs against a row whose
    // predecessor was itself unfiltered by a different branch.
    const width = 4;
    const raw: number[][] = [];
    for (let y = 0; y < 5; y++) {
      const row: number[] = [];
      for (let x = 0; x < width; x++) row.push(x * 10 + y, 100 + y, 200 - x, 255);
      raw.push(row);
    }

    // Filter each row with type y, mirroring the encoder's definition.
    const bpp = 4;
    const filteredRows: number[][] = [];
    let prior = new Array(width * bpp).fill(0);
    for (let y = 0; y < 5; y++) {
      const cur = raw[y]!;
      const out: number[] = [];
      for (let i = 0; i < cur.length; i++) {
        const a = i >= bpp ? cur[i - bpp]! : 0;
        const b = prior[i]!;
        const c = i >= bpp ? prior[i - bpp]! : 0;
        const x = cur[i]!;
        let v: number;
        if (y === 1) v = x - a;
        else if (y === 2) v = x - b;
        else if (y === 3) v = x - ((a + b) >> 1);
        else if (y === 4) {
          const p = a + b - c;
          const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
          v = x - (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
        } else v = x;
        out.push(v & 0xff);
      }
      filteredRows.push(out);
      prior = cur;
    }

    const png = foreignPng(width, 5, 6, 4, filteredRows, [0, 1, 2, 3, 4]);
    const decoded = decodePng(png);
    for (let y = 0; y < 5; y++) {
      for (let x = 0; x < width; x++) {
        expect(pixelAt(decoded, x, y), `pixel ${x},${y}`).toEqual([
          raw[y]![x * 4]!, raw[y]![x * 4 + 1]!, raw[y]![x * 4 + 2]!, raw[y]![x * 4 + 3]!,
        ]);
      }
    }
  });

  it("reads 8-bit greyscale, widening one channel into RGB and opaque alpha", () => {
    const png = foreignPng(3, 2, 0, 1, [[10, 20, 30], [40, 50, 60]], [0, 0]);
    const d = decodePng(png);
    expect(pixelAt(d, 0, 0)).toEqual([10, 10, 10, 255]);
    expect(pixelAt(d, 2, 1)).toEqual([60, 60, 60, 255]);
  });

  it("reads 8-bit greyscale with alpha", () => {
    const png = foreignPng(2, 1, 4, 2, [[10, 128, 20, 0]], [0]);
    const d = decodePng(png);
    expect(pixelAt(d, 0, 0)).toEqual([10, 10, 10, 128]);
    expect(pixelAt(d, 1, 0)).toEqual([20, 20, 20, 0]);
  });

  it("reads 8-bit truecolour without alpha, filling alpha opaque", () => {
    const png = foreignPng(2, 1, 2, 3, [[1, 2, 3, 4, 5, 6]], [0]);
    const d = decodePng(png);
    expect(pixelAt(d, 0, 0)).toEqual([1, 2, 3, 255]);
    expect(pixelAt(d, 1, 0)).toEqual([4, 5, 6, 255]);
  });

  it("skips unknown ANCILLARY chunks", () => {
    const base = Array.from(foreignPng(2, 1, 6, 4, [[1, 2, 3, 4, 5, 6, 7, 8]], [0]));
    // Splice a lowercase chunk in after IHDR (8 signature + 25 IHDR bytes).
    const withText = [...base.slice(0, 33), ...chunk("tEXt", [65, 66]), ...base.slice(33)];
    const d = decodePng(new Uint8Array(withText));
    expect(pixelAt(d, 0, 0)).toEqual([1, 2, 3, 4]);
  });

  it("validates an APNG chunk CRC before refusing the feature", () => {
    const base = Array.from(foreignPng(1, 1, 6, 4, [[1, 2, 3, 4]], [0]));
    const animationControl = chunk("acTL", [...u32be(1), ...u32be(0)]);
    const last = animationControl.length - 1;
    animationControl[last] = animationControl[last]! ^ 0xff;
    const withCorruptApng = [
      ...base.slice(0, 33),
      ...animationControl,
      ...base.slice(33),
    ];
    expect(() => decodePng(new Uint8Array(withCorruptApng))).toThrow(/CRC-32 mismatch/);
  });

  it("refuses an unknown CRITICAL chunk instead of guessing", () => {
    const base = Array.from(foreignPng(2, 1, 6, 4, [[1, 2, 3, 4, 5, 6, 7, 8]], [0]));
    const withCritical = [...base.slice(0, 33), ...chunk("zZZZ".toUpperCase(), [1]), ...base.slice(33)];
    expect(() => decodePng(new Uint8Array(withCritical))).toThrow(/unsupported critical chunk/);
  });

  it("reads an image whose IDAT is split across several chunks", () => {
    // A split may land anywhere, including mid-symbol, so the parts must be
    // joined before anything is parsed.
    const filtered = [0, 9, 8, 7, 6, 5, 4, 3, 2];
    const z = Array.from(deflateSync(Buffer.from(filtered)));
    const a = z.slice(0, 3);
    const b = z.slice(3, 6);
    const c = z.slice(6);
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(2), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("IDAT", a), ...chunk("IDAT", b), ...chunk("IDAT", c),
      ...chunk("IEND", []),
    ]);
    const d = decodePng(png);
    expect(pixelAt(d, 0, 0)).toEqual([9, 8, 7, 6]);
    expect(pixelAt(d, 1, 0)).toEqual([5, 4, 3, 2]);
  });
});

// --- Rejections --------------------------------------------------------------

describe("decodePng rejections", () => {
  const okRow = [[1, 2, 3, 4]];

  it("rejects a file that is too short", () => {
    expect(() => decodePng(new Uint8Array(3))).toThrow(/too short/);
  });

  it("rejects a bad signature", () => {
    const bytes = new Uint8Array(20);
    bytes.set([0x89, 0x50, 0x4e, 0x00], 0);
    expect(() => decodePng(bytes)).toThrow(/invalid signature/);
  });

  it("rejects a corrupted chunk via its CRC", () => {
    const png = encodePng(sampleImage(4, 4));
    const corrupt = new Uint8Array(png);
    corrupt[20] ^= 0xff; // inside IHDR's data
    expect(() => decodePng(corrupt)).toThrow(/CRC-32 mismatch/);
  });

  it("rejects palette images by name", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 3, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/palette images/);
  });

  it("rejects 16-bit depths by name", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 16, 6, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/bit depth 16/);
  });

  it("rejects Adam7 interlacing by name", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 1]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/interlacing/);
  });

  it("rejects an unknown colour type", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 7, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/unknown colour type/);
  });

  it("rejects dimensions above the maximum, before allocating for them", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(100000), ...u32be(100000), 8, 6, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/exceed maximum/);
  });

  it("rejects a zero dimension", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(0), ...u32be(4), 8, 6, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/zero width or height/);
  });

  it("rejects a chunk length larger than the file", () => {
    const png = new Uint8Array([...SIGNATURE, ...u32be(0x7fffffff), 73, 72, 68, 82, 0, 0, 0, 0]);
    expect(() => decodePng(png)).toThrow(/chunk length exceeds file size/);
  });

  it("rejects a missing IEND", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("IDAT", Array.from(deflateSync(Buffer.from([0, 1, 2, 3, 4])))),
    ]);
    expect(() => decodePng(png)).toThrow(/no IEND/);
  });

  it("rejects a file with no IDAT", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/no IDAT/);
  });

  it("rejects any chunk before IHDR, which the spec requires to be first", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IDAT", [1, 2, 3]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/chunk before IHDR/);
  });

  it("rejects two IHDR chunks", () => {
    const hdr = chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]);
    const png = new Uint8Array([...SIGNATURE, ...hdr, ...hdr, ...chunk("IEND", [])]);
    expect(() => decodePng(png)).toThrow(/more than one IHDR/);
  });

  it("rejects a corrupt zlib header", () => {
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("IDAT", [0x78, 0x00, 1, 2, 3, 4]), // 0x7800 is not a multiple of 31
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/corrupt zlib header/);
  });

  it("rejects a preset dictionary", () => {
    // FDICT is bit 5 of FLG. 0x78 0xBB has it set and still checksums to 0 mod 31.
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("IDAT", [0x78, 0xbb, 1, 2, 3, 4]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/preset dictionary/);
  });

  it("rejects a decompressed size that disagrees with the header", () => {
    // Header says 4x4 RGBA, IDAT holds one short row.
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(4), ...u32be(4), 8, 6, 0, 0, 0]),
      ...chunk("IDAT", Array.from(deflateSync(Buffer.from([0, 1, 2, 3, 4])))),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/decompressed|expected/);
  });

  it("rejects a bad Adler-32", () => {
    const png = encodePng(sampleImage(4, 4));
    const bytes = Array.from(png);
    const text = new TextDecoder("latin1").decode(png);
    const at = text.indexOf("IDAT");
    const len = ((png[at - 4]! << 24) | (png[at - 3]! << 16) | (png[at - 2]! << 8) | png[at - 1]!) >>> 0;
    // Corrupt the last byte of the zlib trailer, then repair the chunk CRC so
    // the Adler check is what fails rather than the CRC.
    const trailerAt = at + 4 + len - 1;
    bytes[trailerAt] = bytes[trailerAt]! ^ 0xff;
    const typed = new Uint8Array(bytes.slice(at, at + 4 + len));
    const fixed = u32be(crc32(typed));
    for (let i = 0; i < 4; i++) bytes[at + 4 + len + i] = fixed[i]!;
    expect(() => decodePng(new Uint8Array(bytes))).toThrow(/Adler-32/);
  });

  it("rejects an unknown filter type", () => {
    const png = foreignPng(1, 1, 6, 4, [[1, 2, 3, 4]], [9]);
    expect(() => decodePng(png)).toThrow(/unknown filter type 9/);
  });
});

describe("encodePng rejections", () => {
  it("rejects a zero-pixel image", () => {
    expect(() => encodePng(createPixelContainer(0, 5))).toThrow(/at least one pixel/);
    expect(() => encodePng(createPixelContainer(5, 0))).toThrow(/at least one pixel/);
  });

  it("rejects non-integer or negative dimensions", () => {
    expect(() => encodePng({ width: 1.5, height: 2, data: new Uint8Array(12) }))
      .toThrow(/non-negative integers/);
    expect(() => encodePng({ width: -1, height: 2, data: new Uint8Array(0) }))
      .toThrow(/non-negative integers/);
  });

  it("rejects a data array that disagrees with the dimensions", () => {
    expect(() => encodePng({ width: 4, height: 4, data: new Uint8Array(10) }))
      .toThrow(/pixel data is 10 bytes/);
  });
});

// --- The total-pixel ceiling -------------------------------------------------
//
// A per-EDGE cap is not enough for a compressed format, and PNG is where that
// stops being a theoretical distinction. 16384x16384 passes the edge cap and is
// 268 million pixels -- roughly 3 GiB of peak allocation -- and DEFLATE's 1032:1
// ratio means about one megabyte of input buys it. BMP survives on an edge cap
// alone only because its pixels have to BE in the file.

describe("total-pixel ceiling", () => {
  /** An IHDR-only PNG claiming a size, with no IDAT: the header is the payload. */
  function headerClaiming(w: number, h: number): Uint8Array {
    return new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(w), ...u32be(h), 8, 6, 0, 0, 0]),
      ...chunk("IEND", []),
    ]);
  }

  it("rejects a 16384x16384 header, which is inside the per-edge cap", () => {
    // Both edges are exactly at MAX_DIMENSION, so only the pixel-count check
    // can stop this one.
    expect(() => decodePng(headerClaiming(16384, 16384)))
      .toThrow(/pixels, above the limit/);
  });

  it("rejects before allocating anything derived from the dimensions", () => {
    // No IDAT at all: if the size were used before being checked, the failure
    // would be an allocation rather than this message.
    expect(() => decodePng(headerClaiming(16384, 16384)))
      .toThrow(/268435456 pixels/);
  });

  it("honours a caller-supplied ceiling", () => {
    const png = encodePng(sampleImage(10, 10));
    expect(() => decodePng(png, { maxPixels: 99 })).toThrow(/above the limit of 99/);
    expect(decodePng(png, { maxPixels: 100 }).width).toBe(10);
  });

  it("rejects a nonsensical ceiling rather than ignoring it", () => {
    const png = encodePng(sampleImage(2, 2));
    expect(() => decodePng(png, { maxPixels: 0 })).toThrow(/positive safe integer/);
    expect(() => decodePng(png, { maxPixels: -1 })).toThrow(/positive safe integer/);
    expect(() => decodePng(png, { maxPixels: Infinity })).toThrow(/positive safe integer/);
    expect(() => decodePng(png, { maxPixels: Number.NaN })).toThrow(/positive safe integer/);
  });

  it("carries the ceiling through PngCodec, the shared interface", () => {
    // ImageCodec.decode takes only bytes, so the option has to live on the
    // codec instance or an embedder cannot express its budget at all.
    const png = encodePng(sampleImage(10, 10));
    expect(() => new PngCodec({ maxPixels: 50 }).decode(png)).toThrow(/above the limit/);
    expect(new PngCodec().decode(png).width).toBe(10);
  });
});

// --- Framing rules that keep a valid-looking PNG from carrying passengers ----
//
// Each of these describes a file that decodes to exactly the right image while
// containing bytes the image does not need. That combination is the point: the
// picture is identical either way, so nothing downstream notices, which is
// precisely why the decoder has to refuse rather than tolerate.

describe("framing rules", () => {
  const ihdr = chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]);
  const okIdat = Array.from(deflateSync(Buffer.from([0, 1, 2, 3, 4])));

  it("rejects bytes after IEND", () => {
    const png = new Uint8Array([
      ...SIGNATURE, ...ihdr, ...chunk("IDAT", okIdat), ...chunk("IEND", []),
      0xde, 0xad, 0xbe, 0xef,
    ]);
    expect(() => decodePng(png)).toThrow(/4 bytes follow IEND/);
  });

  it("rejects a non-empty IEND", () => {
    const png = new Uint8Array([
      ...SIGNATURE, ...ihdr, ...chunk("IDAT", okIdat), ...chunk("IEND", [1, 2, 3, 4]),
    ]);
    expect(() => decodePng(png)).toThrow(/IEND must be empty/);
  });

  it("rejects IDAT chunks separated by another chunk", () => {
    // RFC 2083 requires the IDATs to be consecutive. They are one stream cut
    // into pieces; a chunk wedged between them is corruption or a passenger.
    const png = new Uint8Array([
      ...SIGNATURE, ...ihdr,
      ...chunk("IDAT", okIdat.slice(0, 4)),
      ...chunk("tEXt", [65, 66]),
      ...chunk("IDAT", okIdat.slice(4)),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/not consecutive/);
  });

  it("rejects unused bytes between the compressed data and its Adler-32", () => {
    // The IDAT cavity. DEFLATE announces its own end with BFINAL, so a stream
    // can stop early and everything up to the checksum is dead space that a
    // decoder asking only for pixels never looks at.
    const filtered = new Uint8Array([0, 9, 8, 7, 6]);
    const raw = Array.from(deflateRawSync(filtered, { level: 9 }));
    let a = 1, b = 0;
    for (const byte of filtered) { a = (a + byte) % 65521; b = (b + a) % 65521; }
    const adler = ((b << 16) | a) >>> 0;
    const cavity = [0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41]; // "AAAAAAAA"

    const withCavity = new Uint8Array([
      ...SIGNATURE, ...ihdr,
      ...chunk("IDAT", [0x78, 0x9c, ...raw, ...cavity, ...u32be(adler)]),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(withCavity)).toThrow(/unused bytes/);

    // The identical file without the cavity decodes, so the rejection is about
    // the passengers and not about anything else in the construction.
    const clean = new Uint8Array([
      ...SIGNATURE, ...ihdr,
      ...chunk("IDAT", [0x78, 0x9c, ...raw, ...u32be(adler)]),
      ...chunk("IEND", []),
    ]);
    expect(pixelAt(decodePng(clean), 0, 0)).toEqual([9, 8, 7, 6]);
  });

  it("still accepts many zero-length ancillary chunks without stalling", () => {
    // The walk must always advance: a zero-length chunk is legal and common.
    const many: number[] = [];
    for (let i = 0; i < 2000; i++) many.push(...chunk("tEXt", []));
    const png = new Uint8Array([
      ...SIGNATURE, ...ihdr, ...many, ...chunk("IDAT", okIdat), ...chunk("IEND", []),
    ]);
    expect(decodePng(png).width).toBe(1);
  });
});

describe("IHDR must be the first chunk", () => {
  it("rejects an ancillary chunk ahead of IHDR", () => {
    // The last member of the carriage family. A tEXt before the header is a
    // chunk out of place; libpng refuses it, and accepting what the reference
    // implementation rejects is the differential this decoder exists not to
    // have. Nothing is corrupted by it -- that is exactly why it needs saying.
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("tEXt", [65, 66]),
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("IDAT", Array.from(deflateSync(Buffer.from([0, 1, 2, 3, 4])))),
      ...chunk("IEND", []),
    ]);
    expect(() => decodePng(png)).toThrow(/'tEXt' chunk before IHDR/);
  });

  it("still accepts the same file with the chunk moved after IHDR", () => {
    // Same bytes, legal order: proves the rejection is about position.
    const png = new Uint8Array([
      ...SIGNATURE,
      ...chunk("IHDR", [...u32be(1), ...u32be(1), 8, 6, 0, 0, 0]),
      ...chunk("tEXt", [65, 66]),
      ...chunk("IDAT", Array.from(deflateSync(Buffer.from([0, 1, 2, 3, 4])))),
      ...chunk("IEND", []),
    ]);
    expect(pixelAt(decodePng(png), 0, 0)).toEqual([1, 2, 3, 4]);
  });
});

describe("PngCodec validates its ceiling when it is supplied", () => {
  it("rejects a bad maxPixels at construction, not at first decode", () => {
    expect(() => new PngCodec({ maxPixels: Infinity })).toThrow(/positive safe integer/);
    expect(() => new PngCodec({ maxPixels: 0 })).toThrow(/positive safe integer/);
    expect(() => new PngCodec({ maxPixels: Number.NaN })).toThrow(/positive safe integer/);
  });
});
