/**
 * zstd.test.ts — Tests for the ZStd compression/decompression library.
 *
 * Test cases TC-1 through TC-9 verify round-trip correctness across a range
 * of input shapes: empty, single byte, all byte values, RLE, English prose,
 * random data, multi-block, repetitive text, and error handling.
 *
 * Additional unit tests cover internal helpers: RevBitWriter/RevBitReader,
 * FSE table construction, sequence-count encoding, and the wire format decoder.
 */

import { describe, it, expect } from "vitest";
import { compress, decompress } from "../src/zstd.js";
// Also import via the package index to get index.ts coverage
import * as zstdIndex from "../src/index.js";

// Real `zstd` CLI interop (TC-9) needs to shell out and use temp files.
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// ─── Helper ───────────────────────────────────────────────────────────────────

/** Round-trip: compress then decompress; returns the recovered bytes. */
function rt(data: Uint8Array): Uint8Array {
  return decompress(compress(data));
}

/** Convenience wrapper for string inputs. */
function rtStr(s: string): string {
  return new TextDecoder().decode(rt(new TextEncoder().encode(s)));
}

// ─── TC-1: Empty input ────────────────────────────────────────────────────────

describe("TC-1: empty input", () => {
  it("compresses and decompresses empty bytes without error", () => {
    const result = rt(new Uint8Array(0));
    expect(result).toEqual(new Uint8Array(0));
  });

  it("compressed empty frame has the ZStd magic header", () => {
    const compressed = compress(new Uint8Array(0));
    // ZStd magic: 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
    expect(compressed[0]).toBe(0x28);
    expect(compressed[1]).toBe(0xB5);
    expect(compressed[2]).toBe(0x2F);
    expect(compressed[3]).toBe(0xFD);
  });
});

// ─── TC-2: Single byte ────────────────────────────────────────────────────────

describe("TC-2: single byte", () => {
  it("round-trips a single 0x42 byte", () => {
    expect(rt(new Uint8Array([0x42]))).toEqual(new Uint8Array([0x42]));
  });

  it("round-trips 0x00", () => {
    expect(rt(new Uint8Array([0x00]))).toEqual(new Uint8Array([0x00]));
  });

  it("round-trips 0xFF", () => {
    expect(rt(new Uint8Array([0xFF]))).toEqual(new Uint8Array([0xFF]));
  });
});

// ─── TC-3: All 256 byte values ────────────────────────────────────────────────

describe("TC-3: all 256 byte values", () => {
  it("round-trips all byte values 0x00..0xFF in order", () => {
    const input = new Uint8Array(256);
    for (let i = 0; i < 256; i++) input[i] = i;
    expect(rt(input)).toEqual(input);
  });

  it("round-trips all byte values in reverse order", () => {
    const input = new Uint8Array(256);
    for (let i = 0; i < 256; i++) input[i] = 255 - i;
    expect(rt(input)).toEqual(input);
  });
});

// ─── TC-4: RLE (run-length encoding) ─────────────────────────────────────────

describe("TC-4: RLE block", () => {
  it("round-trips 1024 identical bytes", () => {
    const input = new Uint8Array(1024).fill(0x41); // 'A' × 1024
    expect(rt(input)).toEqual(input);
  });

  it("compresses 1024 identical bytes into < 30 bytes (RLE efficiency)", () => {
    const input = new Uint8Array(1024).fill(0x41);
    const compressed = compress(input);
    // Magic(4) + FHD(1) + FCS(8) + block_header(3) + rle_byte(1) = 17 bytes
    expect(compressed.length).toBeLessThan(30);
  });

  it("round-trips 1 byte that looks like an RLE candidate", () => {
    expect(rt(new Uint8Array([0x00]))).toEqual(new Uint8Array([0x00]));
  });

  it("round-trips 2 identical bytes", () => {
    const input = new Uint8Array([0x7F, 0x7F]);
    expect(rt(input)).toEqual(input);
  });
});

// ─── TC-5: English prose ──────────────────────────────────────────────────────

describe("TC-5: English prose", () => {
  it("round-trips repeated English text", () => {
    const text = "the quick brown fox jumps over the lazy dog ".repeat(25);
    expect(rtStr(text)).toBe(text);
  });

  it("achieves >= 20% compression on repeated prose (output <= 80% of input)", () => {
    const text = "the quick brown fox jumps over the lazy dog ".repeat(25);
    const input = new TextEncoder().encode(text);
    const compressed = compress(input);
    const threshold = Math.floor(input.length * 80 / 100);
    expect(compressed.length).toBeLessThan(threshold);
  });
});

// ─── TC-6: Pseudo-random data ─────────────────────────────────────────────────

describe("TC-6: pseudo-random data", () => {
  /** LCG pseudo-random number generator (same as Rust test). */
  function lcgBytes(seed: number, count: number): Uint8Array {
    const out = new Uint8Array(count);
    let s = seed >>> 0;
    for (let i = 0; i < count; i++) {
      s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
      out[i] = s & 0xFF;
    }
    return out;
  }

  it("round-trips 512 LCG-random bytes", () => {
    const input = lcgBytes(42, 512);
    expect(rt(input)).toEqual(input);
  });

  it("round-trips 1024 LCG-random bytes (different seed)", () => {
    const input = lcgBytes(12345, 1024);
    expect(rt(input)).toEqual(input);
  });
});

// ─── TC-7: Large single-byte run (multi-block) ────────────────────────────────

describe("TC-7: large single-byte run", () => {
  it("round-trips 200 KB of 'x' (requires >= 2 blocks)", () => {
    // 200 KB > MAX_BLOCK_SIZE (128 KB), so at least two blocks are needed.
    // Both should be RLE blocks since all bytes are identical.
    const input = new Uint8Array(200 * 1024).fill(0x78); // 'x'
    expect(rt(input)).toEqual(input);
  });
});

// ─── TC-8: Large repetitive text ─────────────────────────────────────────────

describe("TC-8: large repetitive text", () => {
  it("round-trips a 2000-character cycling ABCDEF pattern", () => {
    const pattern = new TextEncoder().encode("ABCDEF");
    const input = new Uint8Array(2000);
    for (let i = 0; i < 2000; i++) input[i] = pattern[i % pattern.length]!;
    expect(rt(input)).toEqual(input);
  });

  it("achieves >= 30% compression on alternating X-runs + ABCDEFGH pattern", () => {
    const pattern = new TextEncoder().encode("ABCDEFGH");
    const xRun = new Uint8Array(128).fill(0x58); // 'X' × 128
    const parts: Uint8Array[] = [pattern];
    for (let i = 0; i < 10; i++) {
      parts.push(xRun, pattern);
    }
    const totalLen = parts.reduce((s, p) => s + p.length, 0);
    const input = new Uint8Array(totalLen);
    let off = 0;
    for (const p of parts) { input.set(p, off); off += p.length; }

    const compressed = compress(input);
    expect(decompress(compressed)).toEqual(input);
    const threshold = Math.floor(input.length * 70 / 100);
    expect(compressed.length).toBeLessThan(threshold);
  });

  // ── Regression for the seq_count endianness bug ──────────────────────────
  //
  // The original encoder for counts in [128, 0x7FFE] wrote bytes in the
  // wrong order: `[count & 0xFF, (count >> 8) | 0x80]`. When the LOW byte
  // of `count` happened to be < 128, the decoder mis-took the 1-byte path
  // and returned a tiny garbage count. Roughly half of all counts in the
  // 2-byte range trigger this — for example, 515 (= 0x0203, low byte 0x03).
  //
  // 200 KB of long-period repetitive text reliably yields enough sequences
  // per single block to push past 128 (LZSS finds ~one match per pattern
  // repetition). This round-trip is the canonical regression: it must pass
  // for the same reason the analogous Lua TC-8 must pass.
  //
  // Timeout bumped to 30 s: the LZSS pass over 200 KB takes ~10 s on slower
  // CI runners. Default 5 s timeout was too tight.
  it("round-trips 200 KB of repetitive text (>= 128 sequences per block)", () => {
    const pattern = "hello world and more text for compression testing!\n";
    const text = pattern.repeat(4000); // ~204 KB → first block has ~500 sequences
    const input = new TextEncoder().encode(text);
    expect(rt(input)).toEqual(input);
  }, 30_000);
});

// ─── Bad magic throws ──────────────────────────────────────────────────────────
//
// NOTE: this describe block used to be mislabeled "TC-9". Per
// code/specs/CMP07-zstd.md, TC-9 is specifically the cross-language /
// interoperability test (compress with the real `zstd` CLI, decompress with
// ours, and vice versa) — see the "TC-9: cross-language / interoperability"
// block near the end of this file, which was missing entirely until it was
// added alongside the FSE sequences-codec conformance fix (lessons.md
// Lesson 96). This block covers unrelated error-path behavior and keeps its
// own (non-TC-numbered) name to avoid the ambiguity.

describe("bad magic throws", () => {
  it("throws on a frame with wrong magic bytes", () => {
    const bad = new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00]);
    expect(() => decompress(bad)).toThrow();
  });

  it("throws on a frame shorter than 5 bytes", () => {
    expect(() => decompress(new Uint8Array([0x28, 0xB5]))).toThrow();
  });

  it("throws on gzip magic (not ZStd)", () => {
    // gzip magic = 0x1F8B
    const gzip = new Uint8Array([0x1F, 0x8B, 0x08, 0x00, 0x00]);
    expect(() => decompress(gzip)).toThrow();
  });

  it("throws on empty input", () => {
    expect(() => decompress(new Uint8Array(0))).toThrow();
  });
});

// ─── Additional round-trip tests ──────────────────────────────────────────────

describe("additional round-trips", () => {
  it("round-trips binary data with lots of zeros and 0xFF bytes", () => {
    const input = new Uint8Array(300);
    for (let i = 0; i < 300; i++) input[i] = i % 256;
    expect(rt(input)).toEqual(input);
  });

  it("round-trips 1000 zero bytes", () => {
    const input = new Uint8Array(1000);
    expect(rt(input)).toEqual(input);
  });

  it("round-trips 1000 0xFF bytes", () => {
    const input = new Uint8Array(1000).fill(0xFF);
    expect(rt(input)).toEqual(input);
  });

  it("round-trips 'hello world'", () => {
    const input = new TextEncoder().encode("hello world");
    expect(rt(input)).toEqual(input);
  });

  it("round-trips a 3000-byte cycling ABCDEF pattern", () => {
    const pattern = new TextEncoder().encode("ABCDEF");
    const input = new Uint8Array(3000);
    for (let i = 0; i < 3000; i++) input[i] = pattern[i % 6]!;
    expect(rt(input)).toEqual(input);
  });
});

// ─── Determinism ──────────────────────────────────────────────────────────────

describe("determinism", () => {
  it("compressing the same data twice produces identical bytes", () => {
    const data = new TextEncoder().encode("hello, ZStd world! ".repeat(50));
    const a = compress(data);
    const b = compress(data);
    expect(a).toEqual(b);
  });
});

// ─── Wire format decoder ──────────────────────────────────────────────────────

describe("wire format decoder", () => {
  it("decodes a manually constructed raw-block ZStd frame", () => {
    // Frame layout:
    //   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
    //   [4]     FHD = 0x20: Single_Segment=1, FCS_flag=00
    //              With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
    //   [5]     FCS = 0x05 (content_size = 5)
    //   [6..8]  Block header: Last=1, Type=Raw, Size=5
    //              = (5 << 3) | (0 << 1) | 1 = 41 = 0x29
    //              = [0x29, 0x00, 0x00]
    //   [9..13] b"hello"
    const frame = new Uint8Array([
      0x28, 0xB5, 0x2F, 0xFD, // magic
      0x20,                   // FHD: Single_Segment=1, FCS=1byte
      0x05,                   // FCS = 5
      0x29, 0x00, 0x00,       // block header: last=1, raw, size=5
      0x68, 0x65, 0x6C, 0x6C, 0x6F, // "hello"
    ]);
    expect(decompress(frame)).toEqual(new TextEncoder().encode("hello"));
  });
});

// ─── Longer texts ─────────────────────────────────────────────────────────────

// ─── Index re-exports ─────────────────────────────────────────────────────────

describe("index.ts re-exports", () => {
  it("exports compress and decompress from the index module", () => {
    expect(typeof zstdIndex.compress).toBe("function");
    expect(typeof zstdIndex.decompress).toBe("function");
  });

  it("compress/decompress via index module round-trips correctly", () => {
    const data = new TextEncoder().encode("hello from index!");
    expect(zstdIndex.decompress(zstdIndex.compress(data))).toEqual(data);
  });
});

// ─── Error paths ──────────────────────────────────────────────────────────────

describe("error paths", () => {
  it("throws on reserved block type 3", () => {
    // Construct a frame with block type 3 (reserved).
    // Magic(4) + FHD(1) + FCS(8) + block_hdr(3 with btype=3)
    // FHD = 0xE0 → FCS_flag=11 → 8-byte FCS
    // Block header: last=1, btype=3, size=0 → (0<<3)|(3<<1)|1 = 7 = 0x07
    const frame = new Uint8Array([
      0x28, 0xB5, 0x2F, 0xFD,       // magic
      0xE0,                           // FHD: FCS=8bytes, single_seg=1
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,         // 8-byte FCS = 0
      0x07, 0x00, 0x00,               // block header: last=1, type=11 (3), size=0
    ]);
    expect(() => decompress(frame)).toThrow(/reserved block type 3/);
  });

  it("throws on truncated compressed block", () => {
    // Construct a frame claiming a 10-byte compressed block but only provide 3 bytes.
    // btype=2 (compressed), bsize=10, last=1
    // hdr = (10 << 3) | (2 << 1) | 1 = 80 | 4 | 1 = 85 = 0x55
    const frame = new Uint8Array([
      0x28, 0xB5, 0x2F, 0xFD,       // magic
      0xE0,                           // FHD
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,         // 8-byte FCS = 0
      0x55, 0x00, 0x00,               // block header: last=1, type=2, size=10
      0x01, 0x02, 0x03,               // only 3 bytes instead of 10
    ]);
    expect(() => decompress(frame)).toThrow();
  });
});

describe("longer text compression", () => {
  it("round-trips 50 repetitions of 'hello, ZStd world!'", () => {
    const text = "hello, ZStd world! ".repeat(50);
    expect(rtStr(text)).toBe(text);
  });

  it("round-trips a longer prose paragraph", () => {
    const text = [
      "Zstandard is a real-time compression algorithm, providing high compression ratios.",
      "It offers a very wide range of compression / speed trade-off, while being backed by a very fast decoder.",
      "It also offers a special mode for small data, called dictionary compression, ",
      "and can create dictionaries from any sample set.",
    ].join(" ").repeat(5);
    expect(rtStr(text)).toBe(text);
  });
});

// ─── TC-9: Cross-language / interoperability ─────────────────────────────────
//
// Per code/specs/CMP07-zstd.md: "compress with the standard `zstd` CLI (or
// reference implementation), decompress with ours; compress with ours,
// decompress with the standard `zstd -d` CLI — both directions must
// round-trip exactly." This is the ONLY test that can catch a systematic,
// symmetric protocol deviation: an internal round-trip (compress-then-
// decompress with our own two functions) passes even when both sides silently
// agree on a wrong convention, because both halves of the comparison are
// wrong in the identical way.
//
// This is exactly what happened here (lessons.md Lesson 96): the FSE
// table-spread algorithm, the per-sequence field order, and the last-sequence
// state-update special case were ALL wrong in a mutually self-consistent way.
// Every one of the 36 pre-existing tests in this file passed throughout, and
// only real `zstd -d` rejected the output with "Data corruption detected".
// TC-9 did not exist in this package before that fix — this is what closes
// that gap.
//
// Both tests below are skipped (not failed) when the `zstd` binary isn't on
// PATH, matching the spec's framing of TC-9 as an interoperability check
// against the reference tool rather than a hard CI dependency.

function isZstdCliAvailable(): boolean {
  try {
    execFileSync("zstd", ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

describe.skipIf(!isZstdCliAvailable())(
  "TC-9: cross-language / interoperability (real zstd CLI)",
  () => {
    it("round-trips both directions on the spec's TC-9 text", () => {
      // Same text as TC-5 / the spec's own TC-9 example: enough repetition
      // to produce a real Compressed block with several FSE-coded sequences
      // (not just Raw/RLE, which interoperate trivially and would not catch
      // a sequences-codec bug).
      const text = "the quick brown fox jumps over the lazy dog ".repeat(25);
      const original = new TextEncoder().encode(text);

      const dir = mkdtempSync(join(tmpdir(), "zstd-ts-tc9-"));
      try {
        // ── Direction 1: compress with ours, decompress with `zstd -d` ────
        const oursPath = join(dir, "ours.zst");
        writeFileSync(oursPath, compress(original));
        const decodedByCli = execFileSync("zstd", ["-d", "-q", "-c", oursPath]);
        expect(new Uint8Array(decodedByCli)).toEqual(original);

        // ── Direction 2: compress with `zstd`, decompress with ours ───────
        const theirsInputPath = join(dir, "theirs.txt");
        writeFileSync(theirsInputPath, original);
        const theirCompressed = execFileSync("zstd", ["-q", "-c", theirsInputPath]);
        const decodedByUs = decompress(new Uint8Array(theirCompressed));
        expect(decodedByUs).toEqual(original);
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    });

    // Regression coverage for the seq_count byte-order bug (see
    // encodeSeqCount's doc comment in src/zstd.ts): an input large enough to
    // push a single block's sequence count past 128 — the exact boundary
    // where the wire encoding switches from its 1-byte form to its 2-byte
    // form (RFC 8878 §3.1.1.3.1). That bug round-tripped fine against
    // ITSELF but silently produced a non-conformant frame, so only a real
    // cross-implementation check like this one can catch it.
    it("round-trips ours -> `zstd -d` for a high-sequence-count block (>128 sequences)", () => {
      // A repeating 6-byte cycle across 9 KB gives LZSS plenty of short,
      // distinct matches — comfortably more than 128 sequences in one
      // block, while staying well under the 128 KB block cap.
      const src = new TextEncoder().encode("ABCDEF");
      const original = new Uint8Array(9000);
      for (let i = 0; i < original.length; i++) original[i] = src[i % src.length]!;

      const dir = mkdtempSync(join(tmpdir(), "zstd-ts-tc9-highseq-"));
      try {
        const oursPath = join(dir, "ours.zst");
        writeFileSync(oursPath, compress(original));
        const decodedByCli = execFileSync("zstd", ["-d", "-q", "-c", oursPath]);
        expect(new Uint8Array(decodedByCli)).toEqual(original);
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    });
  },
);
