/**
 * zip.test.ts — CMP09 ZIP package tests (TC-1 through TC-12).
 *
 * Each test case exercises a distinct capability of the ZIP writer/reader.
 */

import { describe, it, expect } from "vitest";
import { deflateRawSync } from "node:zlib";
import {
  crc32,
  rawDeflate,
  rawInflate,
  dosDatetime,
  DOS_EPOCH,
  ZipWriter,
  ZipReader,
  zipBytes,
  unzip,
} from "../src/index.js";

const enc = new TextEncoder();
const dec = new TextDecoder();

// ─── CRC-32 known vectors ─────────────────────────────────────────────────────

describe("crc32", () => {
  it("empty input → 0x00000000", () => {
    expect(crc32(new Uint8Array(0))).toBe(0x00000000);
  });

  it("'hello world' → 0x0D4A1185", () => {
    expect(crc32(enc.encode("hello world"))).toBe(0x0d4a1185);
  });

  it("incremental matches single-call", () => {
    const data = enc.encode("hello world");
    const half = data.length >> 1;
    const a = crc32(data.slice(0, half));
    const b = crc32(data.slice(half), a);
    expect(b).toBe(crc32(data));
  });
});

// ─── DOS datetime ─────────────────────────────────────────────────────────────

describe("dosDatetime", () => {
  it("DOS_EPOCH encodes 1980-01-01 00:00:00", () => {
    expect(DOS_EPOCH).toBe(0x00210000);
  });

  it("time field is zero for midnight", () => {
    expect(dosDatetime(1980, 1, 1) & 0xffff).toBe(0);
  });

  it("date field for 1980-01-01 is 33", () => {
    expect((dosDatetime(1980, 1, 1) >>> 16) & 0xffff).toBe(33);
  });
});

// ─── TC-1: Single file, Stored (no compression) ───────────────────────────────

describe("TC-1 — single file stored", () => {
  it("round-trips a file without compression", () => {
    const data = enc.encode("Hello, ZIP!");
    const archive = zipBytes([["hello.txt", data]], false);
    const files = unzip(archive);
    expect(files.has("hello.txt")).toBe(true);
    expect(dec.decode(files.get("hello.txt")!)).toBe("Hello, ZIP!");
  });

  it("stored entry has method 0", () => {
    const archive = zipBytes([["a.txt", enc.encode("abc")]], false);
    const entries = new ZipReader(archive).entries();
    expect(entries[0]!.method).toBe(0);
  });
});

// ─── TC-2: Single file, DEFLATE ───────────────────────────────────────────────

describe("TC-2 — single file DEFLATE", () => {
  it("round-trips repetitive text via DEFLATE", () => {
    const data = enc.encode("ABCABCABCABCABC".repeat(100));
    const archive = zipBytes([["rep.txt", data]], true);
    const files = unzip(archive);
    expect(dec.decode(files.get("rep.txt")!)).toBe(dec.decode(data));
  });

  it("DEFLATE shrinks repetitive data", () => {
    const data = enc.encode("x".repeat(1000));
    const archive = zipBytes([["x.txt", data]], true);
    const entries = new ZipReader(archive).entries();
    expect(entries[0]!.compressedSize).toBeLessThan(entries[0]!.size);
    expect(entries[0]!.method).toBe(8);
  });
});

// ─── TC-3: Multiple files ─────────────────────────────────────────────────────

describe("TC-3 — multiple files", () => {
  it("packs and unpacks three files", () => {
    const files: Array<[string, Uint8Array]> = [
      ["a.txt", enc.encode("alpha")],
      ["b.txt", enc.encode("beta")],
      ["c.txt", enc.encode("gamma")],
    ];
    const archive = zipBytes(files);
    const out = unzip(archive);
    expect(dec.decode(out.get("a.txt")!)).toBe("alpha");
    expect(dec.decode(out.get("b.txt")!)).toBe("beta");
    expect(dec.decode(out.get("c.txt")!)).toBe("gamma");
  });

  it("entry list has correct count", () => {
    const archive = zipBytes([
      ["one.txt", enc.encode("1")],
      ["two.txt", enc.encode("2")],
    ]);
    expect(new ZipReader(archive).entries().length).toBe(2);
  });
});

// ─── TC-4: Directory entry ────────────────────────────────────────────────────

describe("TC-4 — directory entry", () => {
  it("directory entry has isDirectory=true", () => {
    const w = new ZipWriter();
    w.addDirectory("mydir/");
    const archive = w.finish();
    const entries = new ZipReader(archive).entries();
    expect(entries.some(e => e.name === "mydir/" && e.isDirectory)).toBe(true);
  });

  it("reading a directory entry returns empty bytes", () => {
    const w = new ZipWriter();
    w.addDirectory("dir/");
    const archive = w.finish();
    const reader = new ZipReader(archive);
    const dir = reader.entries().find(e => e.name === "dir/")!;
    expect(reader.read(dir)).toEqual(new Uint8Array(0));
  });
});

// ─── TC-5: CRC-32 mismatch ────────────────────────────────────────────────────

describe("TC-5 — CRC-32 mismatch", () => {
  it("throws on corrupted data", () => {
    const data = enc.encode("important data");
    const archive = zipBytes([["file.txt", data]], false);

    // Corrupt a byte in the file data section (after the 30+8 = 38-byte local header)
    const corrupt = new Uint8Array(archive);
    const lhNameLen = corrupt[26]! | (corrupt[27]! << 8);
    const dataStart = 30 + lhNameLen;
    corrupt[dataStart] ^= 0xff;

    const reader = new ZipReader(corrupt);
    const entry = reader.entries()[0]!;
    expect(() => reader.read(entry)).toThrow(/CRC-32 mismatch/);
  });
});

// ─── TC-6: Random-access read ─────────────────────────────────────────────────

describe("TC-6 — random-access read", () => {
  it("reads a specific file from a 10-file archive", () => {
    const entries: Array<[string, Uint8Array]> = Array.from({ length: 10 }, (_, i) =>
      [`f${i}.txt` as string, enc.encode(`content of f${i}`)] as [string, Uint8Array]
    );
    const archive = zipBytes(entries);
    const content = dec.decode(new ZipReader(archive).readByName("f5.txt"));
    expect(content).toBe("content of f5");
  });
});

// ─── TC-7: Incompressible data → Stored ──────────────────────────────────────

describe("TC-7 — incompressible data", () => {
  it("incompressible data is stored as method 0", () => {
    // 256 distinct bytes — DEFLATE will expand, so zip falls back to Stored
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    const archive = zipBytes([["rand.bin", data]], true);
    const reader = new ZipReader(archive);
    const entry = reader.entries()[0]!;
    // Stored because compressed >= original
    expect(entry.method).toBe(0);
    expect(reader.read(entry)).toEqual(data);
  });
});

// ─── TC-8: Empty file ─────────────────────────────────────────────────────────

describe("TC-8 — empty file", () => {
  it("empty file round-trips correctly", () => {
    const archive = zipBytes([["empty.txt", new Uint8Array(0)]]);
    const out = unzip(archive);
    expect(out.get("empty.txt")).toEqual(new Uint8Array(0));
  });

  it("empty file has size 0 in entries", () => {
    const archive = zipBytes([["e.txt", new Uint8Array(0)]]);
    const entry = new ZipReader(archive).entries()[0]!;
    expect(entry.size).toBe(0);
    expect(entry.compressedSize).toBe(0);
  });
});

// ─── TC-9: Large file ─────────────────────────────────────────────────────────

describe("TC-9 — large file", () => {
  it("compresses and decompresses 100 KB of repetitive data", { timeout: 30_000 }, () => {
    const data = new Uint8Array(100_000);
    for (let i = 0; i < data.length; i++) data[i] = i % 26 + 65; // A-Z repeating
    const archive = zipBytes([["big.bin", data]], true);
    const out = unzip(archive);
    expect(out.get("big.bin")).toEqual(data);
  });

  it("10 KB all-same-byte data compresses significantly", () => {
    const data = new Uint8Array(10_000).fill(65);
    const archive = zipBytes([["aaaa.bin", data]], true);
    const entry = new ZipReader(archive).entries()[0]!;
    expect(entry.compressedSize).toBeLessThan(200);
  });
});

// ─── TC-10: Unicode filename ──────────────────────────────────────────────────

describe("TC-10 — unicode filename", () => {
  it("preserves unicode filenames", () => {
    const name = "日本語/résumé.txt";
    const archive = zipBytes([[name, enc.encode("hello")]]);
    const out = unzip(archive);
    expect(out.has(name)).toBe(true);
    expect(dec.decode(out.get(name)!)).toBe("hello");
  });
});

// ─── TC-11: Nested paths ─────────────────────────────────────────────────────

describe("TC-11 — nested paths", () => {
  it("preserves deep nested filenames", () => {
    const name = "a/b/c/deep.txt";
    const archive = zipBytes([[name, enc.encode("deep")]]);
    const out = unzip(archive);
    expect(dec.decode(out.get(name)!)).toBe("deep");
  });

  it("mixed nested and flat files", () => {
    const archive = zipBytes([
      ["root.txt", enc.encode("root")],
      ["sub/file.txt", enc.encode("sub")],
      ["sub/deep/file.txt", enc.encode("deep")],
    ]);
    const out = unzip(archive);
    expect(dec.decode(out.get("root.txt")!)).toBe("root");
    expect(dec.decode(out.get("sub/file.txt")!)).toBe("sub");
    expect(dec.decode(out.get("sub/deep/file.txt")!)).toBe("deep");
  });
});

// ─── TC-12: Empty archive ────────────────────────────────────────────────────

describe("TC-12 — empty archive", () => {
  it("empty ZipWriter produces a valid archive", () => {
    const archive = new ZipWriter().finish();
    const reader = new ZipReader(archive);
    expect(reader.entries()).toHaveLength(0);
  });

  it("unzip of empty archive returns empty map", () => {
    const archive = new ZipWriter().finish();
    expect(unzip(archive).size).toBe(0);
  });
});

// ─── ZipReader error paths ────────────────────────────────────────────────────

describe("ZipReader error paths", () => {
  it("throws on invalid bytes (no EOCD)", () => {
    expect(() => new ZipReader(enc.encode("not a zip"))).toThrow(/no End of Central Directory/);
  });

  it("throws on unsupported compression method", () => {
    // Build an archive then patch the method field in the Central Directory
    const archive = zipBytes([["f.txt", enc.encode("x")]], false);
    const patched = new Uint8Array(archive);

    // Find CD header (sig 0x02014B50) and patch method field at offset +10
    for (let i = 0; i < patched.length - 4; i++) {
      if (
        patched[i] === 0x50 && patched[i + 1] === 0x4b &&
        patched[i + 2] === 0x01 && patched[i + 3] === 0x02
      ) {
        patched[i + 10] = 99; // unsupported method
        patched[i + 11] = 0;
        break;
      }
    }

    const reader = new ZipReader(patched);
    expect(() => reader.read(reader.entries()[0]!)).toThrow(/unsupported compression method/);
  });

  it("readByName throws for missing entry", () => {
    const archive = zipBytes([["f.txt", enc.encode("x")]]);
    expect(() => new ZipReader(archive).readByName("missing.txt")).toThrow(/not found/);
  });
});

// ─── Coverage helpers — crafted raw ZIP bytes ─────────────────────────────────
//
// Some error paths in deflateDecompress require crafted DEFLATE streams that
// a normal write/read cycle never produces. Build them from raw bytes.

function makeDeflateZip(deflateBytes: Uint8Array): Uint8Array {
  const name = enc.encode("f.bin");
  const cmpLen = deflateBytes.length;
  const w: number[] = [];
  const le16 = (v: number) => [v & 0xff, (v >>> 8) & 0xff];
  const le32 = (v: number) => [v & 0xff, (v >>> 8) & 0xff, (v >>> 16) & 0xff, (v >>> 24) & 0xff];

  // Local file header
  w.push(0x50, 0x4b, 0x03, 0x04, ...le16(20), ...le16(0x0800), ...le16(8),
         ...le16(0), ...le16(0x21), ...le32(0), ...le32(cmpLen), ...le32(0),
         ...le16(name.length), ...le16(0), ...Array.from(name), ...Array.from(deflateBytes));

  const cdOffset = w.length;

  // Central directory entry
  w.push(0x50, 0x4b, 0x01, 0x02, ...le16(0x031e), ...le16(20), ...le16(0x0800),
         ...le16(8), ...le16(0), ...le16(0x21), ...le32(0), ...le32(cmpLen), ...le32(0),
         ...le16(name.length), ...le16(0), ...le16(0), ...le16(0), ...le16(0),
         ...le32(0), ...le32(0), ...Array.from(name));

  const cdSize = w.length - cdOffset;

  // EOCD
  w.push(0x50, 0x4b, 0x05, 0x06, ...le16(0), ...le16(0), ...le16(1), ...le16(1),
         ...le32(cdSize), ...le32(cdOffset), ...le16(0));

  return new Uint8Array(w);
}

// ─── DEFLATE error paths ──────────────────────────────────────────────────────

describe("deflate error paths via crafted ZIP", () => {
  it("BTYPE=10 (dynamic Huffman) with a truncated header throws, not silently succeeds", () => {
    // byte 0x05 = bits: bfinal=1 (bit0), btype=10 (bits1-2 = 1,0) = BTYPE=2.
    // The block type is now supported, so the failure must come from running
    // out of bits inside the dynamic header rather than from refusing the type.
    const zip = makeDeflateZip(new Uint8Array([0x05]));
    const reader = new ZipReader(zip);
    expect(() => reader.read(reader.entries()[0]!)).toThrow(/EOF|Huffman/);
  });

  it("BTYPE=11 (reserved) throws reserved error", () => {
    // byte 0x07 = bits: bfinal=1 (bit0), btype=11 (bits1-2 = 1,1) = BTYPE=3
    const zip = makeDeflateZip(new Uint8Array([0x07]));
    const reader = new ZipReader(zip);
    expect(() => reader.read(reader.entries()[0]!)).toThrow(/reserved BTYPE/);
  });
});

// ─── ZipReader edge cases ─────────────────────────────────────────────────────

describe("ZipReader edge cases", () => {
  it("throws on data ≥ 22 bytes but with no valid EOCD signature", () => {
    // Data longer than min EOCD size (22) but no 0x06054b50 signature present
    const data = new Uint8Array(30).fill(0x41);
    expect(() => new ZipReader(data)).toThrow(/no End of Central Directory/);
  });

  it("truncates decompressed data to stated uncompressed size", () => {
    // Build a raw ZIP where the stored data (5 bytes "hello") is larger than the
    // stated uncompressed size (3). ZipReader should slice to 3 bytes and CRC-check.
    const stored = enc.encode("hello"); // 5 bytes
    const stated = 3;
    const truncatedCRC = crc32(stored.slice(0, stated));
    const name = enc.encode("t.txt");

    const le16 = (v: number) => [v & 0xff, (v >>> 8) & 0xff];
    const le32 = (v: number) => [v & 0xff, (v >>> 8) & 0xff, (v >>> 16) & 0xff, (v >>> 24) & 0xff];
    const w: number[] = [];

    // Local header (method=0, stored: compressedSize=5, uncompressedSize=5)
    w.push(0x50, 0x4b, 0x03, 0x04, ...le16(10), ...le16(0x0800), ...le16(0),
           ...le16(0), ...le16(0x21), ...le32(truncatedCRC), ...le32(5), ...le32(5),
           ...le16(name.length), ...le16(0), ...Array.from(name), ...Array.from(stored));

    const cdOffset = w.length;

    // CD entry: uncompressedSize=3 (stated), compressedSize=5, crc=truncatedCRC
    w.push(0x50, 0x4b, 0x01, 0x02, ...le16(0x031e), ...le16(10), ...le16(0x0800),
           ...le16(0), ...le16(0), ...le16(0x21), ...le32(truncatedCRC),
           ...le32(5), ...le32(stated),
           ...le16(name.length), ...le16(0), ...le16(0), ...le16(0), ...le16(0),
           ...le32(0), ...le32(0), ...Array.from(name));

    const cdSize = w.length - cdOffset;
    w.push(0x50, 0x4b, 0x05, 0x06, ...le16(0), ...le16(0), ...le16(1), ...le16(1),
           ...le32(cdSize), ...le32(cdOffset), ...le16(0));

    const reader = new ZipReader(new Uint8Array(w));
    const result = reader.readByName("t.txt");
    expect(result).toEqual(stored.slice(0, stated));
  });
});

// ─── ZipWriter + ZipReader combinatorial ─────────────────────────────────────

describe("ZipWriter direct API", () => {
  it("addFile + addDirectory combined", () => {
    const w = new ZipWriter();
    w.addDirectory("docs/");
    w.addFile("docs/readme.txt", enc.encode("Read me"), false);
    const archive = w.finish();
    const reader = new ZipReader(archive);
    const entries = reader.entries();
    expect(entries.length).toBe(2);
    expect(entries[0]!.isDirectory).toBe(true);
    expect(dec.decode(reader.readByName("docs/readme.txt"))).toBe("Read me");
  });
});

// --- Raw DEFLATE: the compressor half, exported for zlib/gzip/PNG ------------
//
// These use Node's own zlib as an ORACLE. Round-tripping our encoder through
// our decoder only proves the two agree with each other; the question that
// matters for PNG is whether we can read what the rest of the world writes,
// and only a foreign encoder can answer it.

describe("rawDeflate / rawInflate", () => {
  it("round-trips text through our own encoder", () => {
    const data = enc.encode("hello hello hello hello world world world");
    expect(rawInflate(rawDeflate(data))).toEqual(data);
  });

  it("round-trips empty input", () => {
    expect(rawInflate(rawDeflate(new Uint8Array(0)))).toEqual(new Uint8Array(0));
  });

  it("round-trips every byte value", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    expect(rawInflate(rawDeflate(data))).toEqual(data);
  });

  it("reads a DYNAMIC Huffman stream written by zlib", () => {
    // Structured, repetitive, and long enough that zlib picks BTYPE=10.
    const parts: string[] = [];
    for (let i = 0; i < 400; i++) parts.push(`line ${i}: the quick brown fox jumps over the lazy dog\n`);
    const data = enc.encode(parts.join(""));

    const foreign = new Uint8Array(deflateRawSync(data));
    // Confirm the oracle really did emit a dynamic block: BFINAL is bit 0 and
    // BTYPE is bits 1-2 of the first byte, LSB-first, so BTYPE = (b >> 1) & 3.
    expect((foreign[0]! >> 1) & 3).toBe(2);

    expect(rawInflate(foreign)).toEqual(data);
  });

  it("reads a zlib stream containing length symbol 285 (a 258-byte match)", () => {
    // A long run of one byte is exactly what produces maximum-length matches,
    // and 258 is the longest DEFLATE can express. zlib encodes those as symbol
    // 285, which this decoder used to reject outright.
    const data = new Uint8Array(4096).fill(0x41);
    const foreign = new Uint8Array(deflateRawSync(data));
    expect(rawInflate(foreign)).toEqual(data);
  });

  it("reads a zlib stream of incompressible random-ish bytes", () => {
    // Deterministic pseudo-random: no Math.random, so a failure reproduces.
    const data = new Uint8Array(3000);
    let x = 123456789;
    for (let i = 0; i < data.length; i++) {
      x = (x * 1103515245 + 12345) & 0x7fffffff;
      data[i] = (x >>> 16) & 0xff;
    }
    const foreign = new Uint8Array(deflateRawSync(data));
    expect(rawInflate(foreign)).toEqual(data);
  });

  it("reads a zlib stream at every compression level", () => {
    const data = enc.encode("abcabcabcabc".repeat(200));
    for (let level = 0; level <= 9; level++) {
      const foreign = new Uint8Array(deflateRawSync(data, { level }));
      expect(rawInflate(foreign), `level ${level}`).toEqual(data);
    }
  });

  it("rejects a code-length repeat that overruns the alphabet", () => {
    // Dynamic header claiming the minimum alphabets, then a repeat long enough
    // to run past their combined length. Built by hand because no encoder emits
    // it: the point is that a malformed stream fails loudly rather than
    // reading off the end of the buffer.
    const bits: number[] = [];
    const push = (value: number, n: number) => {
      for (let i = 0; i < n; i++) bits.push((value >> i) & 1);
    };
    push(1, 1); // BFINAL
    push(2, 2); // BTYPE = 10
    push(0, 5); // HLIT  -> 257 literal/length codes
    push(0, 5); // HDIST -> 1 distance code
    push(0, 4); // HCLEN -> 4 code-length codes
    // Code-length code lengths, in permuted order 16, 17, 18, 0:
    // give symbol 18 (long zero-run) a 1-bit code and symbol 0 a 1-bit code.
    push(0, 3); // 16 -> unused
    push(0, 3); // 17 -> unused
    push(1, 3); // 18 -> 1 bit
    push(1, 3); //  0 -> 1 bit
    // Canonical: symbol 0 gets code 0, symbol 18 gets code 1.
    bits.push(1);   // symbol 18
    push(127, 7);   // repeat 11 + 127 = 138 zeros, twice over the 258 slots
    bits.push(1);
    push(127, 7);
    bits.push(1);
    push(127, 7);

    const bytes = new Uint8Array(Math.ceil(bits.length / 8));
    bits.forEach((b, i) => { if (b) bytes[i >> 3]! |= 1 << (i & 7); });

    expect(() => rawInflate(bytes)).toThrow(/overruns/);
  });
});

// --- Malformed-stream guards -------------------------------------------------
//
// A decompressor that accepts MORE than the reference implementation is not
// being generous; it is a place where two programs read the same bytes
// differently, which is the shape of a content-inspection bypass. These check
// that the tables a stream describes are actually valid Huffman codes.

/** Assemble a bit list (LSB-first, as RFC 1951 packs) into bytes. */
function bitsToBytes(bits: number[]): Uint8Array {
  const bytes = new Uint8Array(Math.ceil(bits.length / 8));
  bits.forEach((b, i) => { if (b) bytes[i >> 3]! |= 1 << (i & 7); });
  return bytes;
}

/**
 * Build a dynamic-block header by hand. `clLengths` gives the code length of
 * each of the 19 code-length symbols, indexed by symbol number.
 */
function dynamicHeader(clLengths: number[], numCodeLen = 19): number[] {
  const order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
  const bits: number[] = [];
  const push = (value: number, n: number) => {
    for (let i = 0; i < n; i++) bits.push((value >> i) & 1);
  };
  push(1, 1);                 // BFINAL
  push(2, 2);                 // BTYPE = 10 (dynamic)
  push(0, 5);                 // HLIT  -> 257 literal/length codes
  push(0, 5);                 // HDIST -> 1 distance code
  push(numCodeLen - 4, 4);    // HCLEN
  for (let i = 0; i < numCodeLen; i++) push(clLengths[order[i]!] ?? 0, 3);
  return bits;
}

describe("rawInflate malformed-stream guards", () => {
  it("rejects an INCOMPLETE code-length table", () => {
    // Only symbol 0 has a code, and it is 2 bits long -- so three of the four
    // two-bit patterns decode to nothing. zlib refuses this outright and so
    // must we, rather than succeeding on whichever streams happen to dodge the
    // holes.
    const cl = new Array(19).fill(0);
    cl[0] = 2;
    const bits = dynamicHeader(cl);
    for (let i = 0; i < 64; i++) bits.push(1);
    expect(() => rawInflate(bitsToBytes(bits))).toThrow(/incomplete code-length/);
  });

  it("rejects an OVER-SUBSCRIBED code-length table", () => {
    // Five symbols all claiming a 2-bit code, when only four 2-bit codes exist.
    // The surplus symbols would simply be unreachable, so decoding would appear
    // to work while disagreeing with every other inflater about the meaning.
    const cl = new Array(19).fill(0);
    for (const sym of [0, 1, 2, 3, 4]) cl[sym] = 2;
    const bits = dynamicHeader(cl);
    for (let i = 0; i < 64; i++) bits.push(1);
    expect(() => rawInflate(bitsToBytes(bits))).toThrow(/over-subscribed/);
  });

  it("rejects an incomplete literal/length table", () => {
    // A valid code-length alphabet (symbols 0 and 18, one bit each), used to
    // declare every literal/length code absent. An empty LL alphabet cannot
    // even encode end-of-block, so the stream is unreadable by construction.
    const cl = new Array(19).fill(0);
    cl[0] = 1;
    cl[18] = 1;
    const bits = dynamicHeader(cl);
    // Canonical assignment over {0, 18} at length 1: symbol 0 -> "0", 18 -> "1".
    // Symbol 18 repeats zero 11 + extra times, so 138 then 120 zeroes exactly
    // fills the 257 + 1 declared lengths without overrunning them.
    for (const extra of [127, 109]) {
      bits.push(1);
      for (let i = 0; i < 7; i++) bits.push((extra >> i) & 1);
    }
    expect(() => rawInflate(bitsToBytes(bits))).toThrow(/incomplete literal\/length/);
  });

  it("rejects HDIST claiming more distance codes than RFC 1951 defines", () => {
    // HDIST is five bits, so it can say 32; the spec defines 30.
    const bits: number[] = [];
    const push = (value: number, n: number) => {
      for (let i = 0; i < n; i++) bits.push((value >> i) & 1);
    };
    push(1, 1);  // BFINAL
    push(2, 2);  // BTYPE = 10
    push(0, 5);  // HLIT  -> 257
    push(31, 5); // HDIST -> 32, two more than exist
    push(0, 4);
    expect(() => rawInflate(bitsToBytes(bits))).toThrow(/distance codes exceeds/);
  });

  it("rejects HLIT claiming more literal/length codes than RFC 1951 defines", () => {
    const bits: number[] = [];
    const push = (value: number, n: number) => {
      for (let i = 0; i < n; i++) bits.push((value >> i) & 1);
    };
    push(1, 1);  // BFINAL
    push(2, 2);  // BTYPE = 10
    push(31, 5); // HLIT -> 288, two more than exist
    push(0, 5);
    push(0, 4);
    expect(() => rawInflate(bitsToBytes(bits))).toThrow(/literal\/length codes exceeds/);
  });
});

// --- The output cap ----------------------------------------------------------

describe("rawInflate output cap", () => {
  it("honours a caller-supplied byte ceiling", { timeout: 30_000 }, () => {
    const data = new Uint8Array(5_000).fill(0x5a);
    const compressed = rawDeflate(data);
    // Well under the default cap, so only the explicit one can stop it.
    expect(() => rawInflate(compressed, 1024)).toThrow(/output size limit exceeded/);
    expect(rawInflate(compressed, 5_000)).toEqual(data);
  });

  it("caps a stored block too, not just compressed ones", () => {
    // Stored blocks grow the output without going through a Huffman decoder,
    // so they need the same check.
    const data = new Uint8Array(300);
    for (let i = 0; i < data.length; i++) data[i] = (i * 7) & 0xff;
    const compressed = new Uint8Array(deflateRawSync(data, { level: 0 }));
    expect(() => rawInflate(compressed, 100)).toThrow(/output size limit exceeded/);
  });

  it("rejects a nonsensical ceiling rather than silently ignoring it", () => {
    const compressed = rawDeflate(enc.encode("x"));
    expect(() => rawInflate(compressed, -1)).toThrow(/non-negative/);
    expect(() => rawInflate(compressed, Number.NaN)).toThrow(/non-negative/);
  });

  it("counts the cap in BYTES, so a real bomb is stopped at the stated size", () => {
    // Built with zlib rather than our own encoder, because zlib gets far closer
    // to DEFLATE's 1032:1 ceiling and this test is about what a hostile input
    // can actually do. The cap used to count entries of a number[], where each
    // byte cost four to eight, so "256 MB" meant one to two gigabytes of
    // backing store and the process died before the limit was ever reached.
    const run = new Uint8Array(4_000_000).fill(0x00);
    const bomb = new Uint8Array(deflateRawSync(run, { level: 9 }));
    expect(bomb.length).toBeLessThan(run.length / 500); // genuinely a bomb

    expect(() => rawInflate(bomb, 1_000_000)).toThrow(/output size limit exceeded/);
    expect(rawInflate(bomb, run.length).length).toBe(run.length);
  }, 60_000);
});

// --- Regressions from security review ----------------------------------------

describe("ZipReader does not trust the declared uncompressed size as a limit", () => {
  it("clamps a lying central-directory size to the reader's own ceiling", () => {
    // `entry.size` is four bytes the ARCHIVE chose. Passing it to the inflater
    // as the memory ceiling -- which an earlier revision of this file did --
    // replaces a fixed limit with an attacker-chosen one, and the CRC that
    // would catch the lie only runs after the memory is already committed.
    //
    // This archive declares 4 GiB uncompressed while carrying 5,000 bytes. With
    // the reader capped at 1 KB, the declared size must lose: the read has to
    // fail on the LIMIT, not sail past it and fail later on the CRC.
    const payload = new Uint8Array(deflateRawSync(new Uint8Array(5000).fill(0x41)));
    const name = enc.encode("bomb");
    const le16 = (v: number) => [v & 0xff, (v >>> 8) & 0xff];
    const le32 = (v: number) => [
      v & 0xff, (v >>> 8) & 0xff, (v >>> 16) & 0xff, (v >>> 24) & 0xff,
    ];
    const LIE = 0xffffffff;
    const w: number[] = [];

    w.push(0x50, 0x4b, 0x03, 0x04, ...le16(20), ...le16(0x0800), ...le16(8),
           ...le16(0), ...le16(0x21), ...le32(0), ...le32(payload.length), ...le32(LIE),
           ...le16(name.length), ...le16(0), ...Array.from(name), ...Array.from(payload));

    const cdOffset = w.length;
    w.push(0x50, 0x4b, 0x01, 0x02, ...le16(0x031e), ...le16(20), ...le16(0x0800),
           ...le16(8), ...le16(0), ...le16(0x21), ...le32(0),
           ...le32(payload.length), ...le32(LIE),
           ...le16(name.length), ...le16(0), ...le16(0), ...le16(0), ...le16(0),
           ...le32(0), ...le32(0), ...Array.from(name));

    const cdSize = w.length - cdOffset;
    w.push(0x50, 0x4b, 0x05, 0x06, ...le16(0), ...le16(0), ...le16(1), ...le16(1),
           ...le32(cdSize), ...le32(cdOffset), ...le16(0));

    const archive = new Uint8Array(w);
    const capped = new ZipReader(archive, { maxOutput: 1024 });
    const entry = capped.entries()[0]!;
    expect(entry.size).toBe(LIE);
    expect(() => capped.read(entry)).toThrow(/output size limit exceeded/);

    // And with a ceiling above the real size it gets far enough to catch the
    // lie the honest way, on the checksum -- proving the cap, not some other
    // guard, is what stopped it above.
    const generous = new ZipReader(archive, { maxOutput: 1 << 20 });
    expect(() => generous.read(generous.entries()[0]!)).toThrow(/CRC-32 mismatch/);
  });

  it("still reads an honest entry, including a zero-length one", () => {
    const archive = zipBytes([
      ["empty.bin", new Uint8Array(0)],
      ["real.txt", enc.encode("x".repeat(3000))],
    ], true);
    const out = unzip(archive);
    expect(out.get("empty.bin")).toEqual(new Uint8Array(0));
    expect(out.get("real.txt")!.length).toBe(3000);
  });
});

describe("the distance-table exception is keyed on code LENGTH, not symbol count", () => {
  it("rejects a lone distance code longer than one bit, as zlib does", () => {
    // RFC 1951 s3.2.7 permits ONE incomplete case: if only one distance code is
    // used "it is encoded using one bit, not zero bits; in this case there is a
    // single code length of one." A lone TWO-bit code is therefore not the
    // exception -- it leaves a hole, and zlib rejects it ("invalid distances
    // set"). Reading a stream a scanner refused is the differential to avoid.
    //
    // Built by hand, because no encoder emits this. The literal/length alphabet
    // is deliberately COMPLETE so the distance check is unambiguously what
    // fires: symbol 0 and symbol 256 at one bit each, everything between at zero.
    const order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
    const bits: number[] = [];
    const push = (value: number, n: number) => {
      for (let i = 0; i < n; i++) bits.push((value >> i) & 1);
    };

    // Code-length alphabet over {18, 1, 2}: 18 at one bit, 1 and 2 at two bits.
    // Kraft: 1/2 + 1/4 + 1/4 = 1, so it is complete.
    const cl = new Array(19).fill(0);
    cl[18] = 1;
    cl[1] = 2;
    cl[2] = 2;

    push(1, 1);   // BFINAL
    push(2, 2);   // BTYPE = 10 (dynamic)
    push(0, 5);   // HLIT  -> 257 literal/length codes
    push(0, 5);   // HDIST -> 1 distance code
    push(15, 4);  // HCLEN -> all 19, since symbol 1 sits last in the permutation
    for (let i = 0; i < 19; i++) push(cl[order[i]!] ?? 0, 3);

    // Canonical assignment: length 1 -> {18} = "0"; length 2 -> {1, 2} = "10", "11".
    // Huffman codes are written most-significant bit first.
    const CODES: Record<number, number[]> = { 18: [0], 1: [1, 0], 2: [1, 1] };
    const emit = (sym: number) => bits.push(...CODES[sym]!);

    emit(1);                 // LL symbol 0   -> code length 1
    emit(18); push(127, 7);  // 11 + 127 = 138 zeros
    emit(18); push(106, 7);  // 11 + 106 = 117 zeros   (138 + 117 = 255)
    emit(1);                 // LL symbol 256 -> code length 1  (257 LL lengths)
    emit(2);                 // the single distance code -> length 2, the bug

    const bytes = new Uint8Array(Math.ceil(bits.length / 8));
    bits.forEach((b, i) => { if (b) bytes[i >> 3]! |= 1 << (i & 7); });

    expect(() => rawInflate(bytes)).toThrow(/incomplete distance Huffman table/);
  });
});
