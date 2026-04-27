/**
 * zip.test.ts — CMP09 ZIP package tests (TC-1 through TC-12).
 *
 * Each test case exercises a distinct capability of the ZIP writer/reader.
 */

import { describe, it, expect } from "vitest";
import {
  crc32,
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
  it("BTYPE=10 (dynamic Huffman) throws not-supported error", () => {
    // byte 0x05 = bits: bfinal=1 (bit0), btype=10 (bits1-2 = 1,0) = BTYPE=2
    const zip = makeDeflateZip(new Uint8Array([0x05]));
    const reader = new ZipReader(zip);
    expect(() => reader.read(reader.entries()[0]!)).toThrow(/dynamic Huffman/);
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
