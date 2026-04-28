/**
 * zip.ts — CMP09: ZIP archive format (PKZIP, 1989).
 *
 * ZIP bundles one or more files into a single `.zip` archive, compressing each
 * entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
 * The same format underlies Java JARs, Office Open XML (.docx/.xlsx), Android
 * APKs (.apk), Python wheels (.whl), and many more.
 *
 * Architecture:
 * ```
 * ┌─────────────────────────────────────────────────────┐
 * │  [Local File Header + File Data]  ← entry 1         │
 * │  [Local File Header + File Data]  ← entry 2         │
 * │  ...                                                │
 * │  ══════════ Central Directory ══════════            │
 * │  [Central Dir Header]  ← entry 1 (has local offset)│
 * │  [Central Dir Header]  ← entry 2                   │
 * │  [End of Central Directory Record]                  │
 * └─────────────────────────────────────────────────────┘
 * ```
 *
 * DEFLATE Inside ZIP:
 * ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper. This
 * implementation uses fixed Huffman blocks (BTYPE=01) and the `lzss` package
 * for LZ77 match-finding.
 *
 * Series:
 * ```
 * CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← dependency
 * CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
 * CMP09 (ZIP,     1989) — DEFLATE container; universal archive. ← this file
 * ```
 */

import { encode as lzssEncode, type Token as LzssToken } from "@coding-adventures/lzss";

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).

const CRC_TABLE: Uint32Array = (() => {
  const t = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    }
    t[i] = c >>> 0;
  }
  return t;
})();

/**
 * Compute CRC-32 over `data`, starting from `initial` (0 for a fresh hash).
 * For incremental updates, pass the previous result as `initial`.
 *
 * @example
 * crc32(new TextEncoder().encode("hello world"), 0) === 0x0D4A1185
 */
export function crc32(data: Uint8Array, initial = 0): number {
  let crc = (initial ^ 0xffffffff) >>> 0;
  for (const byte of data) {
    crc = ((CRC_TABLE[(crc ^ byte) & 0xff] ?? 0) ^ (crc >>> 8)) >>> 0;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O
// =============================================================================
//
// RFC 1951 packs bits LSB-first. Huffman codes are written MSB-first logically,
// so we bit-reverse them before writing LSB-first. We use BigInt for the
// accumulator so we can safely buffer up to 64 bits without overflow.

function reverseBits(value: number, nbits: number): number {
  let result = 0;
  for (let i = 0; i < nbits; i++) {
    result = ((result << 1) | (value & 1)) >>> 0;
    value >>>= 1;
  }
  return result;
}

class BitWriter {
  private buf: bigint = 0n;
  private bits = 0;
  private out: number[] = [];

  writeLSB(value: number, nbits: number): void {
    this.buf |= BigInt(value >>> 0) << BigInt(this.bits);
    this.bits += nbits;
    while (this.bits >= 8) {
      this.out.push(Number(this.buf & 0xffn));
      this.buf >>= 8n;
      this.bits -= 8;
    }
  }

  writeHuffman(code: number, nbits: number): void {
    this.writeLSB(reverseBits(code, nbits), nbits);
  }

  align(): void {
    if (this.bits > 0) {
      this.out.push(Number(this.buf & 0xffn));
      this.buf = 0n;
      this.bits = 0;
    }
  }

  finish(): Uint8Array {
    this.align();
    return new Uint8Array(this.out);
  }
}

class BitReader {
  private pos = 0;
  private buf = 0n;
  private bits = 0;

  constructor(private readonly data: Uint8Array) {}

  private fill(need: number): boolean {
    while (this.bits < need) {
      if (this.pos >= this.data.length) return false;
      this.buf |= BigInt(this.data[this.pos]!) << BigInt(this.bits);
      this.pos++;
      this.bits += 8;
    }
    return true;
  }

  readLSB(nbits: number): number | null {
    if (nbits === 0) return 0;
    if (!this.fill(nbits)) return null;
    const mask = (1n << BigInt(nbits)) - 1n;
    const val = Number(this.buf & mask);
    this.buf >>= BigInt(nbits);
    this.bits -= nbits;
    return val;
  }

  readMSB(nbits: number): number | null {
    const v = this.readLSB(nbits);
    return v === null ? null : reverseBits(v, nbits);
  }

  align(): void {
    const discard = this.bits % 8;
    if (discard > 0) {
      this.buf >>= BigInt(discard);
      this.bits -= discard;
    }
  }
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// RFC 1951 §3.2.6 fixed code lengths:
//   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
//   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
//   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
// Distance codes 0–29: 5-bit codes equal to the code number.

function fixedLLEncode(sym: number): [number, number] {
  if (sym <= 143) return [0b00110000 + sym, 8];
  if (sym <= 255) return [0b110010000 + (sym - 144), 9];
  if (sym <= 279) return [sym - 256, 7];
  if (sym <= 287) return [0b11000000 + (sym - 280), 8];
  throw new Error(`fixedLLEncode: invalid symbol ${sym}`);
}

function fixedLLDecode(br: BitReader): number | null {
  const v7 = br.readMSB(7);
  if (v7 === null) return null;
  if (v7 <= 23) return v7 + 256; // 7-bit: 256-279
  const extra = br.readLSB(1);
  if (extra === null) return null;
  const v8 = (v7 << 1) | extra;
  if (v8 >= 48 && v8 <= 191) return v8 - 48;    // literals 0-143
  if (v8 >= 192 && v8 <= 199) return v8 + 88;    // symbols 280-287
  const extra2 = br.readLSB(1);
  if (extra2 === null) return null;
  const v9 = (v8 << 1) | extra2;
  if (v9 >= 400 && v9 <= 511) return v9 - 256;   // literals 144-255
  return null;
}

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================

type TableEntry = readonly [number, number]; // [base, extraBits]

const LENGTH_TABLE: ReadonlyArray<TableEntry> = [
  [3, 0], [4, 0], [5, 0], [6, 0], [7, 0], [8, 0], [9, 0], [10, 0], // 257-264
  [11, 1], [13, 1], [15, 1], [17, 1],                                 // 265-268
  [19, 2], [23, 2], [27, 2], [31, 2],                                 // 269-272
  [35, 3], [43, 3], [51, 3], [59, 3],                                 // 273-276
  [67, 4], [83, 4], [99, 4], [115, 4],                                // 277-280
  [131, 5], [163, 5], [195, 5], [227, 5],                             // 281-284
];

const DIST_TABLE: ReadonlyArray<TableEntry> = [
  [1, 0], [2, 0], [3, 0], [4, 0],
  [5, 1], [7, 1], [9, 2], [13, 2],
  [17, 3], [25, 3], [33, 4], [49, 4],
  [65, 5], [97, 5], [129, 6], [193, 6],
  [257, 7], [385, 7], [513, 8], [769, 8],
  [1025, 9], [1537, 9], [2049, 10], [3073, 10],
  [4097, 11], [6145, 11], [8193, 12], [12289, 12],
  [16385, 13], [24577, 13],
];

function encodeLength(length: number): [number, number, number] {
  for (let i = LENGTH_TABLE.length - 1; i >= 0; i--) {
    const [base, extra] = LENGTH_TABLE[i]!;
    if (length >= base) return [257 + i, base, extra];
  }
  throw new Error(`encodeLength: unreachable for length=${length}`);
}

function encodeDist(offset: number): [number, number, number] {
  for (let i = DIST_TABLE.length - 1; i >= 0; i--) {
    const [base, extra] = DIST_TABLE[i]!;
    if (offset >= base) return [i, base, extra];
  }
  throw new Error(`encodeDist: unreachable for offset=${offset}`);
}

// =============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// =============================================================================

function deflateCompress(data: Uint8Array): Uint8Array {
  const bw = new BitWriter();

  if (data.length === 0) {
    bw.writeLSB(1, 1);       // BFINAL=1
    bw.writeLSB(0, 2);       // BTYPE=00 (stored)
    bw.align();
    bw.writeLSB(0x0000, 16); // LEN=0
    bw.writeLSB(0xffff, 16); // NLEN=~0
    return bw.finish();
  }

  const tokens: LzssToken[] = lzssEncode(data, 32768, 255, 3);

  bw.writeLSB(1, 1); // BFINAL
  bw.writeLSB(1, 1); // BTYPE bit 0 = 1
  bw.writeLSB(0, 1); // BTYPE bit 1 = 0  → BTYPE = 01

  for (const tok of tokens) {
    if (tok.kind === "literal") {
      const [code, nbits] = fixedLLEncode(tok.byte);
      bw.writeHuffman(code, nbits);
    } else {
      const [sym, baseLen, extraLenBits] = encodeLength(tok.length);
      const [code, nbits] = fixedLLEncode(sym);
      bw.writeHuffman(code, nbits);
      if (extraLenBits > 0) bw.writeLSB(tok.length - baseLen, extraLenBits);

      const [distCode, baseDist, extraDistBits] = encodeDist(tok.offset);
      bw.writeHuffman(distCode, 5);
      if (extraDistBits > 0) bw.writeLSB(tok.offset - baseDist, extraDistBits);
    }
  }

  const [eobCode, eobBits] = fixedLLEncode(256);
  bw.writeHuffman(eobCode, eobBits);
  return bw.finish();
}

// =============================================================================
// RFC 1951 DEFLATE — Decompress
// =============================================================================

const MAX_OUTPUT = 256 * 1024 * 1024;

function deflateDecompress(data: Uint8Array): Uint8Array {
  const br = new BitReader(data);
  const out: number[] = [];

  for (;;) {
    const bfinal = br.readLSB(1);
    if (bfinal === null) throw new Error("deflate: unexpected EOF reading BFINAL");
    const btype = br.readLSB(2);
    if (btype === null) throw new Error("deflate: unexpected EOF reading BTYPE");

    if (btype === 0) {
      // Stored block
      br.align();
      const lenVal = br.readLSB(16);
      if (lenVal === null) throw new Error("deflate: EOF reading stored LEN");
      const nlen = br.readLSB(16);
      if (nlen === null) throw new Error("deflate: EOF reading stored NLEN");
      if ((nlen ^ 0xffff) !== lenVal) throw new Error(`deflate: LEN/NLEN mismatch: ${lenVal} vs ${nlen}`);
      if (out.length + lenVal > MAX_OUTPUT) throw new Error("deflate: output size limit exceeded");
      for (let i = 0; i < lenVal; i++) {
        const b = br.readLSB(8);
        if (b === null) throw new Error("deflate: EOF inside stored block data");
        out.push(b);
      }
    } else if (btype === 1) {
      // Fixed Huffman block
      for (;;) {
        const sym = fixedLLDecode(br);
        if (sym === null) throw new Error("deflate: EOF decoding fixed Huffman symbol");
        if (sym < 256) {
          if (out.length >= MAX_OUTPUT) throw new Error("deflate: output size limit exceeded");
          out.push(sym);
        } else if (sym === 256) {
          break;
        } else if (sym >= 257 && sym <= 285) {
          const idx = sym - 257;
          const entry = LENGTH_TABLE[idx];
          if (!entry) throw new Error(`deflate: invalid length sym ${sym}`);
          const [baseLen, extraLenBits] = entry;
          const extraLen = br.readLSB(extraLenBits);
          if (extraLen === null) throw new Error("deflate: EOF reading length extra bits");
          const length = baseLen + extraLen;

          const distCode = br.readMSB(5);
          if (distCode === null) throw new Error("deflate: EOF reading distance code");
          const distEntry = DIST_TABLE[distCode];
          if (!distEntry) throw new Error(`deflate: invalid dist code ${distCode}`);
          const [baseDist, extraDistBits] = distEntry;
          const extraDist = br.readLSB(extraDistBits);
          if (extraDist === null) throw new Error("deflate: EOF reading distance extra bits");
          const offset = baseDist + extraDist;

          if (offset > out.length) throw new Error(`deflate: back-reference offset ${offset} > output len ${out.length}`);
          if (out.length + length > MAX_OUTPUT) throw new Error("deflate: output size limit exceeded");
          for (let i = 0; i < length; i++) out.push(out[out.length - offset]!);
        } else {
          throw new Error(`deflate: invalid LL symbol ${sym}`);
        }
      }
    } else if (btype === 2) {
      throw new Error("deflate: dynamic Huffman blocks (BTYPE=10) not supported");
    } else {
      throw new Error("deflate: reserved BTYPE=11");
    }

    if (bfinal === 1) break;
  }
  return new Uint8Array(out);
}

// =============================================================================
// MS-DOS Date / Time Encoding
// =============================================================================

/**
 * Encode a timestamp into the 32-bit MS-DOS datetime used by ZIP headers.
 *
 * @example
 * dosDatetime(1980, 1, 1) >>> 16 === 33  // date field
 * dosDatetime(1980, 1, 1) & 0xFFFF === 0  // time field
 */
export function dosDatetime(
  year: number, month: number, day: number,
  hour = 0, minute = 0, second = 0
): number {
  const t = (hour << 11) | (minute << 5) | (second >>> 1);
  const d = (Math.max(0, year - 1980) << 9) | (month << 5) | day;
  return (((d & 0xffff) << 16) | (t & 0xffff)) >>> 0;
}

/** Fixed timestamp for 1980-01-01 00:00:00. */
export const DOS_EPOCH: number = dosDatetime(1980, 1, 1);

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================

interface CdRecord {
  name: Uint8Array;
  method: number;
  crc: number;
  compressedSize: number;
  uncompressedSize: number;
  localOffset: number;
  externalAttrs: number;
}

/** Builds a ZIP archive incrementally in memory. */
export class ZipWriter {
  private buf: number[] = [];
  private entries: CdRecord[] = [];

  /** Add a file entry. Compress with DEFLATE if it reduces size. */
  addFile(name: string, data: Uint8Array, compress = true): void {
    this.addEntry(name, data, compress, 0o100644);
  }

  /** Add a directory entry (name should end with '/'). */
  addDirectory(name: string): void {
    this.addEntry(name, new Uint8Array(0), false, 0o040755);
  }

  private addEntry(name: string, data: Uint8Array, compress: boolean, unixMode: number): void {
    const nameBytes = new TextEncoder().encode(name);
    const checksum = crc32(data);
    const uncompressedSize = data.length;

    let method: number;
    let fileData: Uint8Array;
    if (compress && data.length > 0) {
      const compressed = deflateCompress(data);
      if (compressed.length < data.length) {
        method = 8; fileData = compressed;
      } else {
        method = 0; fileData = data;
      }
    } else {
      method = 0; fileData = data;
    }

    const compressedSize = fileData.length;
    const localOffset = this.buf.length;
    const versionNeeded = method === 8 ? 20 : 10;
    const flags = 0x0800;

    // Local File Header
    this.pushLE32(0x04034b50);
    this.pushLE16(versionNeeded);
    this.pushLE16(flags);
    this.pushLE16(method);
    this.pushLE16(DOS_EPOCH & 0xffff);         // mod_time
    this.pushLE16((DOS_EPOCH >>> 16) & 0xffff); // mod_date
    this.pushLE32(checksum);
    this.pushLE32(compressedSize);
    this.pushLE32(uncompressedSize);
    this.pushLE16(nameBytes.length);
    this.pushLE16(0); // extra_field_length = 0
    for (const b of nameBytes) this.buf.push(b);
    for (const b of fileData) this.buf.push(b);

    this.entries.push({ name: nameBytes, method, crc: checksum, compressedSize, uncompressedSize, localOffset, externalAttrs: (unixMode << 16) >>> 0 });
  }

  /** Append Central Directory and EOCD; return the archive as Uint8Array. */
  finish(): Uint8Array {
    const cdOffset = this.buf.length;
    const cdStart = this.buf.length;
    for (const e of this.entries) {
      const versionNeeded = e.method === 8 ? 20 : 10;
      this.pushLE32(0x02014b50);
      this.pushLE16(0x031e);                            // version_made_by
      this.pushLE16(versionNeeded);
      this.pushLE16(0x0800);                            // flags (UTF-8)
      this.pushLE16(e.method);
      this.pushLE16(DOS_EPOCH & 0xffff);                // mod_time
      this.pushLE16((DOS_EPOCH >>> 16) & 0xffff);       // mod_date
      this.pushLE32(e.crc);
      this.pushLE32(e.compressedSize);
      this.pushLE32(e.uncompressedSize);
      this.pushLE16(e.name.length);
      this.pushLE16(0); // extra_len
      this.pushLE16(0); // comment_len
      this.pushLE16(0); // disk_start
      this.pushLE16(0); // internal_attrs
      this.pushLE32(e.externalAttrs);
      this.pushLE32(e.localOffset);
      for (const b of e.name) this.buf.push(b);
    }
    const cdSize = this.buf.length - cdStart;

    this.pushLE32(0x06054b50); // EOCD signature
    this.pushLE16(0);
    this.pushLE16(0);
    this.pushLE16(this.entries.length);
    this.pushLE16(this.entries.length);
    this.pushLE32(cdSize);
    this.pushLE32(cdOffset);
    this.pushLE16(0);

    return new Uint8Array(this.buf);
  }

  private pushLE16(v: number): void {
    this.buf.push(v & 0xff, (v >>> 8) & 0xff);
  }

  private pushLE32(v: number): void {
    const u = v >>> 0;
    this.buf.push(u & 0xff, (u >>> 8) & 0xff, (u >>> 16) & 0xff, (u >>> 24) & 0xff);
  }
}

// =============================================================================
// ZIP Read — ZipEntry and ZipReader
// =============================================================================

/** Metadata for a single entry inside a ZIP archive. */
export interface ZipEntry {
  readonly name: string;
  readonly size: number;
  readonly compressedSize: number;
  readonly method: number;
  readonly crc32: number;
  readonly isDirectory: boolean;
  readonly localOffset: number;
}

/** Reads entries from an in-memory ZIP archive. */
export class ZipReader {
  private readonly entries_: ZipEntry[] = [];

  constructor(private readonly data: Uint8Array) {
    const eocdOffset = this.findEOCD();
    if (eocdOffset === null) throw new Error("zip: no End of Central Directory record found");

    const cdOffset = readLE32(data, eocdOffset + 16);
    const cdSize = readLE32(data, eocdOffset + 12);
    if (cdOffset === null || cdSize === null) throw new Error("zip: EOCD too short");
    if (cdOffset + cdSize > data.length) throw new Error(`zip: Central Directory out of bounds`);

    let pos = cdOffset;
    while (pos + 4 <= cdOffset + cdSize) {
      const sig = readLE32(data, pos);
      if (sig !== 0x02014b50) break;

      const method = readLE16(data, pos + 10)!;
      const crc32v = readLE32(data, pos + 16)!;
      const compressedSize = readLE32(data, pos + 20)!;
      const size = readLE32(data, pos + 24)!;
      const nameLen = readLE16(data, pos + 28)!;
      const extraLen = readLE16(data, pos + 30)!;
      const commentLen = readLE16(data, pos + 32)!;
      const localOffset = readLE32(data, pos + 42)!;

      const nameStart = pos + 46;
      const nameEnd = nameStart + nameLen;
      if (nameEnd > data.length) throw new Error("zip: CD entry name out of bounds");
      const name = new TextDecoder().decode(data.slice(nameStart, nameEnd));

      this.entries_.push({ name, size, compressedSize, method, crc32: crc32v, isDirectory: name.endsWith("/"), localOffset });
      pos = nameEnd + extraLen + commentLen;
    }
  }

  entries(): ZipEntry[] { return [...this.entries_]; }

  read(entry: ZipEntry): Uint8Array {
    if (entry.isDirectory) return new Uint8Array(0);

    const localFlags = readLE16(this.data, entry.localOffset + 6);
    if (localFlags === null) throw new Error("zip: local header out of bounds");
    if (localFlags & 1) throw new Error(`zip: entry '${entry.name}' is encrypted`);

    const lhNameLen = readLE16(this.data, entry.localOffset + 26)!;
    const lhExtraLen = readLE16(this.data, entry.localOffset + 28)!;
    const dataStart = entry.localOffset + 30 + lhNameLen + lhExtraLen;
    const dataEnd = dataStart + entry.compressedSize;
    if (dataEnd > this.data.length) throw new Error(`zip: entry '${entry.name}' data out of bounds`);

    const compressed = this.data.slice(dataStart, dataEnd);

    let decompressed: Uint8Array;
    if (entry.method === 0) {
      decompressed = compressed;
    } else if (entry.method === 8) {
      decompressed = deflateDecompress(compressed);
    } else {
      throw new Error(`zip: unsupported compression method ${entry.method} for '${entry.name}'`);
    }

    if (decompressed.length > entry.size) {
      decompressed = decompressed.slice(0, entry.size);
    }

    const actualCRC = crc32(decompressed);
    if (actualCRC !== entry.crc32) {
      throw new Error(`zip: CRC-32 mismatch for '${entry.name}': expected ${entry.crc32.toString(16)}, got ${actualCRC.toString(16)}`);
    }

    return decompressed;
  }

  readByName(name: string): Uint8Array {
    const entry = this.entries_.find(e => e.name === name);
    if (!entry) throw new Error(`zip: entry '${name}' not found`);
    return this.read(entry);
  }

  private findEOCD(): number | null {
    const eocdSig = 0x06054b50;
    const maxComment = 65535;
    const eocdMinSize = 22;
    const data = this.data;
    if (data.length < eocdMinSize) return null;
    const scanStart = Math.max(0, data.length - eocdMinSize - maxComment);
    for (let i = data.length - eocdMinSize; i >= scanStart; i--) {
      if (readLE32(data, i) === eocdSig) {
        const commentLen = readLE16(data, i + 20);
        if (commentLen !== null && i + eocdMinSize + commentLen === data.length) return i;
      }
    }
    return null;
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Compress a list of `(name, data)` pairs into a ZIP archive.
 *
 * @example
 * const archive = zipBytes([["hello.txt", new TextEncoder().encode("Hello!")]]);
 */
export function zipBytes(entries: Array<[string, Uint8Array]>, compress = true): Uint8Array {
  const w = new ZipWriter();
  for (const [name, data] of entries) w.addFile(name, data, compress);
  return w.finish();
}

/**
 * Decompress all file entries from a ZIP archive.
 *
 * @example
 * const files = unzip(archive);
 * files.get("hello.txt")  // Uint8Array
 */
export function unzip(data: Uint8Array): Map<string, Uint8Array> {
  const reader = new ZipReader(data);
  const out = new Map<string, Uint8Array>();
  for (const entry of reader.entries()) {
    if (!entry.isDirectory) out.set(entry.name, reader.read(entry));
  }
  return out;
}

// =============================================================================
// Little-endian helpers
// =============================================================================

function readLE16(data: Uint8Array, offset: number): number | null {
  if (offset + 2 > data.length) return null;
  return (data[offset]! | (data[offset + 1]! << 8)) & 0xffff;
}

function readLE32(data: Uint8Array, offset: number): number | null {
  if (offset + 4 > data.length) return null;
  return ((data[offset]! | (data[offset + 1]! << 8) | (data[offset + 2]! << 16) | (data[offset + 3]! << 24)) >>> 0);
}
