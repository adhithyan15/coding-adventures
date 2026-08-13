/// ZIP archive format (PKZIP, 1989) — CMP09.
///
/// ZIP bundles one or more files into a single `.zip` archive, compressing
/// each entry independently with **DEFLATE** (method 8) or storing it
/// verbatim (method 0). The same dual-header design underlies Java JARs,
/// Office Open XML (`.docx`/`.xlsx`/`.pptx`), Android APKs, Python wheels,
/// and many more container formats.
///
/// # Architecture
///
/// ```
/// ┌─────────────────────────────────────────────────────┐
/// │  [Local File Header + File Data]  ← entry 1          │
/// │  [Local File Header + File Data]  ← entry 2          │
/// │  ...                                                 │
/// │  ══════════ Central Directory ══════════             │
/// │  [Central Dir Header]  ← entry 1 (has local offset)  │
/// │  [Central Dir Header]  ← entry 2                     │
/// │  [End of Central Directory Record]                   │
/// └─────────────────────────────────────────────────────┘
/// ```
///
/// The dual-header design enables two workflows:
/// - **Sequential write**: append Local Headers one-by-one, write the
///   Central Directory (CD) at the end once every entry's final size and
///   CRC are known.
/// - **Random-access read**: seek straight to the End Of Central Directory
///   (EOCD) at the end of the file, read the CD, then jump directly to any
///   entry's data without scanning the whole archive.
///
/// # Wire Format (all integers little-endian)
///
/// Local File Header (30 + n + e bytes):
/// ```
/// [0x04034B50]           signature
/// [version_needed u16]   20=DEFLATE, 10=Stored
/// [flags u16]            bit 11 = UTF-8 filename
/// [method u16]           0=Stored, 8=DEFLATE
/// [mod_time u16]         MS-DOS packed time
/// [mod_date u16]         MS-DOS packed date
/// [crc32 u32]
/// [compressed_size u32]
/// [uncompressed_size u32]
/// [name_len u16]
/// [extra_len u16]
/// [name bytes...]
/// [extra bytes...]
/// [file data...]
/// ```
///
/// Central Directory Header (46 + n + e + c bytes) — one per entry, written
/// after every Local Header + data block:
/// ```
/// [0x02014B50]  signature
/// [version_made_by u16]
/// [version_needed u16]
/// [flags u16]
/// [method u16]
/// [mod_time u16]
/// [mod_date u16]
/// [crc32 u32]
/// [compressed_size u32]
/// [uncompressed_size u32]
/// [name_len u16]
/// [extra_len u16]
/// [comment_len u16]
/// [disk_start u16]
/// [int_attrs u16]
/// [ext_attrs u32]   Unix: (mode << 16)
/// [local_offset u32]
/// [name bytes...]
/// [extra bytes...]
/// [comment bytes...]
/// ```
///
/// End of Central Directory Record (22 bytes) — the anchor a reader finds
/// first, by scanning backward from EOF:
/// ```
/// [0x06054B50]  signature
/// [disk_num u16]
/// [cd_disk u16]
/// [entries_this_disk u16]
/// [entries_total u16]
/// [cd_size u32]
/// [cd_offset u32]
/// [comment_len u16]
/// ```
///
/// # DEFLATE Inside ZIP
///
/// ZIP method 8 stores **raw RFC 1951 DEFLATE** — no zlib wrapper (no
/// `CMF`/`FLG` header, no Adler-32 checksum). The wire format begins
/// directly with the 3-bit block header.
///
/// This package implements RFC 1951 itself rather than depending on the
/// `coding_adventures_deflate` package: that package's `compress`/
/// `decompress` pair uses a private, self-designed wire format for internal
/// round-tripping (an explicit header carrying its own LL/distance
/// code-length tables) rather than the standard RFC 1951 bit-stream a real
/// ZIP entry must carry — see `lessons.md` for the full story. Every other
/// language's `zip` package in this repository (Python, Go, Rust, Ruby,
/// TypeScript, Elixir, Lua, Swift, Perl, ...) follows the same shape: depend
/// on the sibling `lzss` package for LZ77 match-finding only, and implement
/// RFC 1951 framing directly in the `zip` package itself.
///
/// The **writer** emits RFC 1951 **fixed-Huffman** compressed blocks
/// (BTYPE=01) — the pre-agreed §3.2.6 tables mean no code table is ever
/// transmitted, keeping the encoder simple while still getting real
/// LZ77 compression via the `lzss` package's match-finding.
///
/// The **reader** (`inflate`) decodes **all three** RFC 1951 block types —
/// stored (BTYPE=00), fixed Huffman (BTYPE=01), and dynamic Huffman
/// (BTYPE=10). This matters because real-world producers (the `zip`(1)
/// command, Python's `zipfile`, Java's `jar`, Microsoft Office writing
/// `.docx`/`.xlsx`/`.pptx`) overwhelmingly emit dynamic-Huffman blocks — a
/// reader that only understood fixed Huffman could not open files people
/// actually have.
///
/// # Series
///
/// ```
/// CMP00 (LZ77,     1977) — Sliding-window backreferences.
/// CMP01 (LZ78,     1978) — Explicit dictionary (trie).
/// CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
/// CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
/// CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
/// CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
/// CMP09 (ZIP,      1989) — DEFLATE container; universal archive.  ← this package
/// ```
library coding_adventures_zip;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lzss/lzss.dart' as lzss;

// =============================================================================
// CRC-32
// =============================================================================
//
// ZIP uses CRC-32 (polynomial 0xEDB88320, the reflected form of 0x04C11DB7)
// to detect accidental corruption of an entry's decompressed content. It is
// NOT a cryptographic hash — for tamper detection use AES-GCM or a signed
// manifest, never CRC-32.
//
//   crc32(data, initial=0xFFFFFFFF):
//     crc = initial
//     for byte in data:
//       crc = crc_table[(crc ^ byte) & 0xFF] ^ (crc >> 8)
//     return crc ^ 0xFFFFFFFF

const int _mask32 = 0xFFFFFFFF;

List<int> _buildCrcTable() {
  final table = List<int>.filled(256, 0);
  for (var n = 0; n < 256; n++) {
    var c = n;
    for (var k = 0; k < 8; k++) {
      c = (c & 1) != 0 ? (0xEDB88320 ^ (c >> 1)) : (c >> 1);
    }
    table[n] = c;
  }
  return table;
}

final List<int> _crcTable = _buildCrcTable();

/// Compute CRC-32 over [data], starting from [initial] (0 for a fresh hash,
/// or a previous result to extend an incremental computation).
///
/// ```dart
/// crc32(utf8.encode('hello world')) == 0x0D4A1185
/// ```
int crc32(List<int> data, [int initial = 0]) {
  var crc = initial ^ _mask32;
  for (final byte in data) {
    crc = _crcTable[(crc ^ byte) & 0xFF] ^ (crc >> 8);
  }
  return (crc ^ _mask32) & _mask32;
}

/// Stable, payload-blind raw-inflate failures required by CMP09.
class RawInflateError extends FormatException {
  RawInflateError(this.code)
      : super(_rawInflateErrorMessages[code] ?? 'raw inflate failed');

  /// The language-neutral error identifier.
  final String code;
}

const Map<String, String> _rawInflateErrorMessages = <String, String>{
  'invalid-output-limit':
      'raw inflate output limit must be within the hard ceiling',
  'unexpected-eof': 'raw inflate input ended before the stream was complete',
  'reserved-block-type': 'raw inflate encountered a reserved block type',
  'stored-length-mismatch': 'raw inflate stored block length check failed',
  'huffman-oversubscribed': 'raw inflate Huffman tree is over-subscribed',
  'incomplete-code-length-tree': 'raw inflate code-length tree is incomplete',
  'incomplete-literal-length-tree':
      'raw inflate literal-length tree is incomplete',
  'incomplete-distance-tree': 'raw inflate distance tree is incomplete',
  'repeat-without-previous': 'raw inflate repeat has no previous code length',
  'repeat-overrun':
      'raw inflate code-length repeat overruns the declared alphabets',
  'invalid-literal-length-symbol':
      'raw inflate literal-length symbol is invalid',
  'reserved-distance-symbol': 'raw inflate distance symbol is reserved',
  'invalid-back-reference': 'raw inflate back-reference is invalid',
  'output-limit-exceeded': 'raw inflate output size limit exceeded',
};

Never _inflateFail(String code) => throw RawInflateError(code);

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O
// =============================================================================
//
// RFC 1951 packs bits into bytes LSB-first: the first bit produced fills the
// least-significant unused bit of the current byte. Huffman codes are the
// one exception — they are conceptually MSB-first, so the *encoder* bit-
// reverses a code's fixed-width value before feeding it through the same
// LSB-first writer, and the *decoder* rebuilds a code by shifting each new
// bit into the low end of a growing accumulator (`code = (code << 1) | bit`)
// — which reconstructs the code's original MSB-first value one bit at a
// time. Extra bits (length/distance extras, stored-block headers) are
// written and read directly, with no reversal.

class _BitWriter {
  final List<int> _out = <int>[];
  int _buf = 0;
  int _bits = 0;

  /// Write the low [n] bits of [value], LSB-first.
  void writeLsb(int value, int n) {
    if (n == 0) return;
    final mask = n >= 32 ? _mask32 : ((1 << n) - 1);
    _buf |= (value & mask) << _bits;
    _bits += n;
    while (_bits >= 8) {
      _out.add(_buf & 0xFF);
      _buf >>= 8;
      _bits -= 8;
    }
  }

  /// Write a Huffman code of [nbits] bits: bit-reverse [code], then write it
  /// LSB-first (see the module-level note above).
  void writeHuffman(int code, int nbits) {
    var reversed = 0;
    var c = code;
    for (var i = 0; i < nbits; i++) {
      reversed = (reversed << 1) | (c & 1);
      c >>= 1;
    }
    writeLsb(reversed, nbits);
  }

  /// Flush any partial byte (padding with zero bits), used before stored
  /// blocks and at the very end of the stream.
  void align() {
    if (_bits > 0) {
      _out.add(_buf & 0xFF);
      _buf = 0;
      _bits = 0;
    }
  }

  Uint8List finish() {
    align();
    return Uint8List.fromList(_out);
  }
}

/// Reads RFC 1951 bits from a byte buffer, LSB-first within each byte.
class _BitReader {
  final Uint8List _data;
  int _pos = 0; // next byte index
  int _bit = 0; // next bit index within _data[_pos], 0 = LSB

  _BitReader(this._data);

  bool get isByteAligned => _bit == 0;

  int readBit() {
    if (_pos >= _data.length) {
      _inflateFail('unexpected-eof');
    }
    final bit = (_data[_pos] >> _bit) & 1;
    _bit += 1;
    if (_bit == 8) {
      _bit = 0;
      _pos += 1;
    }
    return bit;
  }

  /// Read [n] raw bits, LSB-first (the first bit read becomes the least
  /// significant bit of the returned value). Used for extra bits and the
  /// fixed-width header fields — never for Huffman codes.
  int readBits(int n) {
    var value = 0;
    for (var i = 0; i < n; i++) {
      value |= readBit() << i;
    }
    return value;
  }

  /// Discard any partial byte so the next read starts on a byte boundary
  /// (required before a stored block's LEN/NLEN fields).
  void alignToByte() {
    if (_bit != 0) {
      _bit = 0;
      _pos += 1;
    }
  }

  /// Read one raw byte. Only valid when [isByteAligned].
  int readByte() {
    if (_pos >= _data.length) {
      _inflateFail('unexpected-eof');
    }
    final b = _data[_pos];
    _pos += 1;
    return b;
  }

  /// Bytes reached through the last bit read, excluding whole trailing bytes.
  int get bytesConsumed => _pos + (_bit == 0 ? 0 : 1);
}

// =============================================================================
// RFC 1951 DEFLATE — Canonical Huffman decoding
// =============================================================================
//
// A canonical Huffman code is built purely from an array of code lengths
// (one entry per symbol; 0 = symbol unused) via RFC 1951 §3.2.2:
//
//   1. bl_count[len] = how many symbols use each code length.
//   2. next_code[len] = the first code value assigned at that length —
//      computed by walking lengths 1..max in order, each time shifting the
//      running `code` left by one and adding the previous length's count.
//   3. Walk symbols in ASCENDING symbol-number order; whenever a symbol has
///     a nonzero length, give it `next_code[length]` and increment that
//      counter.
//
// This single algorithm builds both the *fixed* tables (RFC 1951 §3.2.6,
// lengths hard-coded) and every *dynamic* table (lengths transmitted
// per-block) — real-world encoders (zlib, Info-ZIP, Java, 7-Zip, Microsoft
// Office) all use it, so decoding a dynamic block from any of them means
// re-deriving the exact same table from the lengths they sent.
class _CanonicalDecoder {
  final Map<int, Map<int, int>> _codesByLength;
  final int _maxBits;
  final bool isComplete;
  final int symbolCount;
  final int oneBitSymbolCount;

  const _CanonicalDecoder._(
    this._codesByLength,
    this._maxBits,
    this.isComplete,
    this.symbolCount,
    this.oneBitSymbolCount,
  );

  /// Build a decoder from per-symbol code lengths (index = symbol, 0 = unused).
  ///
  /// A code-length set that oversubscribes the prefix-code space (more
  /// symbols at a given depth than available slots) is rejected — that can
  /// only happen with malformed or malicious input, since every real
  /// encoder's lengths satisfy the Kraft inequality by construction.
  ///
  /// The one deliberate exception: a table with **exactly one** active
  /// symbol is accepted even though it leaves half the code space unused.
  /// RFC 1951 explicitly allows this for the "no distance codes used"
  /// case (a single dummy code of length 1), and real encoders (zlib
  /// included) rely on it; our own writer never emits it since it only
  /// produces fixed-Huffman blocks, but real-world dynamic blocks do.
  static _CanonicalDecoder fromLengths(List<int> lengths) {
    var maxBits = 0;
    var symbolCount = 0;
    for (final len in lengths) {
      if (len < 0 || len > 15) {
        _inflateFail('invalid-literal-length-symbol');
      }
      if (len > maxBits) maxBits = len;
      if (len > 0) symbolCount += 1;
    }
    if (maxBits == 0) {
      return const _CanonicalDecoder._(
        <int, Map<int, int>>{},
        0,
        false,
        0,
        0,
      );
    }

    final blCount = List<int>.filled(maxBits + 1, 0);
    for (final len in lengths) {
      if (len > 0) blCount[len] += 1;
    }

    var available = 1;
    for (var bits = 1; bits <= maxBits; bits++) {
      available = (available << 1) - blCount[bits];
      if (available < 0) {
        _inflateFail('huffman-oversubscribed');
      }
    }

    final nextCode = List<int>.filled(maxBits + 1, 0);
    var code = 0;
    for (var bits = 1; bits <= maxBits; bits++) {
      code = (code + blCount[bits - 1]) << 1;
      nextCode[bits] = code;
    }

    final codesByLength = <int, Map<int, int>>{};
    for (var sym = 0; sym < lengths.length; sym++) {
      final len = lengths[sym];
      if (len == 0) continue;
      final assigned = nextCode[len];
      nextCode[len] = assigned + 1;
      codesByLength.putIfAbsent(len, () => <int, int>{})[assigned] = sym;
    }

    return _CanonicalDecoder._(
      codesByLength,
      maxBits,
      available == 0,
      symbolCount,
      blCount.length > 1 ? blCount[1] : 0,
    );
  }

  /// Decode one symbol: read bits one at a time, growing an MSB-first code
  /// accumulator, until it matches an assigned code of that length.
  int readSymbol(_BitReader br, String invalidCode) {
    var code = 0;
    for (var len = 1; len <= _maxBits; len++) {
      code = (code << 1) | br.readBit();
      final sym = _codesByLength[len]?[code];
      if (sym != null) return sym;
    }
    _inflateFail(invalidCode);
  }
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables (§3.2.6)
// =============================================================================
//
// Literal/Length code lengths:
//   Symbols   0–143: 8-bit codes, starting at 0b0011_0000 (=48)
//   Symbols 144–255: 9-bit codes, starting at 0b1_1001_0000 (=400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (=0)
//   Symbols 280–287: 8-bit codes, starting at 0b1100_0000 (=192)
//
// Distance codes:
//   Symbols 0–29: 5-bit codes equal to the symbol number.
//
// Both fixed tables carry no per-block overhead — encoder and decoder agree
// on them in advance — which is the whole appeal of BTYPE=01.

(int code, int nbits) _fixedLlEncode(int sym) {
  if (sym <= 143) return (0x30 + sym, 8);
  if (sym <= 255) return (0x190 + (sym - 144), 9);
  if (sym <= 279) return (sym - 256, 7);
  if (sym <= 287) return (0xC0 + (sym - 280), 8);
  throw ArgumentError.value(sym, 'sym', 'fixed_ll_encode: invalid LL symbol');
}

List<int> _fixedLlLengths() {
  final lengths = List<int>.filled(288, 0);
  for (var s = 0; s <= 143; s++) {
    lengths[s] = 8;
  }
  for (var s = 144; s <= 255; s++) {
    lengths[s] = 9;
  }
  for (var s = 256; s <= 279; s++) {
    lengths[s] = 7;
  }
  for (var s = 280; s <= 287; s++) {
    lengths[s] = 8;
  }
  return lengths;
}

List<int> _fixedDistLengths() => List<int>.filled(30, 5);

_CanonicalDecoder? _fixedLlDecoderCache;
_CanonicalDecoder get _fixedLlDecoder =>
    _fixedLlDecoderCache ??= _CanonicalDecoder.fromLengths(_fixedLlLengths());

_CanonicalDecoder? _fixedDistDecoderCache;
_CanonicalDecoder get _fixedDistDecoder => _fixedDistDecoderCache ??=
    _CanonicalDecoder.fromLengths(_fixedDistLengths());

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables (§3.2.5)
// =============================================================================
//
// Match lengths (3–255, the maximum our LZSS tokenizer produces) map to LL
// symbols 257–284 plus extra bits. Match distances (1–32768, the full
// window our tokenizer searches) map to distance codes 0–29 plus extra
// bits. These tables are identical across every language port in this
// repository (see `code/packages/rust/zip/src/lib.rs`), since they come
// directly from the RFC, not from any implementation choice.

/// `(base_length, extra_bits)` for LL symbols 257..=285.
///
/// Symbol 285 is a special fixed value (length 258, 0 extra bits) — RFC
/// 1951's maximum match length. Our own [deflateCompress] never emits it
/// (the `lzss` tokenizer this package uses caps matches at 255 bytes), but
/// a full 29-entry table is required to *read* arbitrary real-world DEFLATE
/// streams: unlike our writer, `zlib`/`Info-ZIP`/Java's `Deflater` are not
/// bound by that 255-byte cap and readily produce length-258 matches, which
/// a 28-entry table (covering only symbols 257–284) cannot decode — this
/// was caught by TC-10 CLI interop against the real `zip` command, which
/// promptly emitted symbol 285 in its very first dynamic-Huffman block.
const List<(int, int)> _lengthTable = <(int, int)>[
  (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), // 257-264
  (11, 1), (13, 1), (15, 1), (17, 1), // 265-268
  (19, 2), (23, 2), (27, 2), (31, 2), // 269-272
  (35, 3), (43, 3), (51, 3), (59, 3), // 273-276
  (67, 4), (83, 4), (99, 4), (115, 4), // 277-280
  (131, 5), (163, 5), (195, 5), (227, 5), // 281-284
  (258, 0), // 285
];

/// `(base_offset, extra_bits)` for distance codes 0..=29.
const List<(int, int)> _distTable = <(int, int)>[
  (1, 0),
  (2, 0),
  (3, 0),
  (4, 0),
  (5, 1),
  (7, 1),
  (9, 2),
  (13, 2),
  (17, 3),
  (25, 3),
  (33, 4),
  (49, 4),
  (65, 5),
  (97, 5),
  (129, 6),
  (193, 6),
  (257, 7),
  (385, 7),
  (513, 8),
  (769, 8),
  (1025, 9),
  (1537, 9),
  (2049, 10),
  (3073, 10),
  (4097, 11),
  (6145, 11),
  (8193, 12),
  (12289, 12),
  (16385, 13),
  (24577, 13),
];

/// Map a match length (3–255) to its RFC 1951 LL symbol, base, and extra bits.
(int sym, int base, int extraBits) _encodeLength(int length) {
  for (var i = _lengthTable.length - 1; i >= 0; i--) {
    final (base, extra) = _lengthTable[i];
    if (length >= base) return (257 + i, base, extra);
  }
  throw ArgumentError.value(length, 'length', 'encode_length: out of range');
}

/// Map a match offset (1–32768) to its distance code, base, and extra bits.
(int code, int base, int extraBits) _encodeDist(int offset) {
  for (var i = _distTable.length - 1; i >= 0; i--) {
    final (base, extra) = _distTable[i];
    if (offset >= base) return (i, base, extra);
  }
  throw ArgumentError.value(offset, 'offset', 'encode_dist: out of range');
}

// =============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// =============================================================================
//
// Strategy:
//   1. Run LZSS match-finding (window=32768, max_match=255, min_match=3) —
//      window and max length both fit inside the RFC 1951 length/distance
//      tables above, so every token the tokenizer produces is representable.
//   2. Emit a single BTYPE=01 (fixed Huffman) block containing the token
//      stream. RFC 1951 does not cap Huffman block sizes (only stored
//      blocks are limited to 65535 bytes), so one block per input suffices.
//   3. Literal bytes → fixed LL Huffman code.
//   4. Match (offset, length) → length LL code + extra bits + distance code
//      + extra bits.
//   5. End-of-block symbol (256) → fixed LL Huffman code.

/// Compress [data] to a raw RFC 1951 DEFLATE bit-stream (fixed Huffman,
/// single block). The output starts directly with the 3-bit block header —
/// no zlib wrapper, no length prefix.
Uint8List deflateCompress(Uint8List data) {
  final bw = _BitWriter();

  if (data.isEmpty) {
    // Smallest legal DEFLATE stream: one empty stored block.
    bw.writeLsb(1, 1); // BFINAL=1
    bw.writeLsb(0, 2); // BTYPE=00 (stored)
    bw.align();
    bw.writeLsb(0x0000, 16); // LEN=0
    bw.writeLsb(0xFFFF, 16); // NLEN=~LEN
    return bw.finish();
  }

  final tokens = lzss.encode(data, 32768, 255, 3);

  bw.writeLsb(1, 1); // BFINAL=1 (this is the only, final block)
  bw.writeLsb(1, 1); // BTYPE bit 0
  bw.writeLsb(0, 1); // BTYPE bit 1  →  BTYPE = 01 (fixed Huffman)

  for (final tok in tokens) {
    if (tok is lzss.Literal) {
      final (code, nbits) = _fixedLlEncode(tok.byte);
      bw.writeHuffman(code, nbits);
    } else {
      final match = tok as lzss.Match;

      final (sym, base, extraLenBits) = _encodeLength(match.length);
      final (code, nbits) = _fixedLlEncode(sym);
      bw.writeHuffman(code, nbits);
      if (extraLenBits > 0) {
        bw.writeLsb(match.length - base, extraLenBits);
      }

      final (distCode, distBase, extraDistBits) = _encodeDist(match.offset);
      bw.writeHuffman(distCode, 5); // fixed distance codes are always 5 bits
      if (extraDistBits > 0) {
        bw.writeLsb(match.offset - distBase, extraDistBits);
      }
    }
  }

  final (eobCode, eobBits) = _fixedLlEncode(256); // end-of-block
  bw.writeHuffman(eobCode, eobBits);

  return bw.finish();
}

// =============================================================================
// Byte-native growable buffer
// =============================================================================
//
// `_decodeHuffmanBlock` accumulates decoded output one byte (or one
// back-reference copy) at a time, so it needs an append-friendly growable
// buffer. A plain `List<int>` looks like the obvious choice, but on the
// Dart VM each `List<int>` slot is a full boxed word (8 bytes on 64-bit),
// not 1 byte — so bounding a `List<int>` accumulator at `maxOutput`
// *elements* actually allows roughly `8 * maxOutput` bytes of real memory,
// silently defeating a documented "256 MB decompression-bomb guard" by
// close to an order of magnitude. This buffer wraps a `Uint8List` with
// amortized-O(1) doubling growth instead, so the byte cap enforced
// elsewhere (`out.length > maxOutput`) tracks actual memory 1:1.
class _ByteBuffer {
  final int _limit;
  Uint8List _buf;
  int _length = 0;

  _ByteBuffer(this._limit) : _buf = Uint8List(_limit < 64 ? _limit : 64);

  int get length => _length;

  int operator [](int index) => _buf[index];

  void add(int byte) {
    _reserve(1);
    _buf[_length] = byte;
    _length += 1;
  }

  void copyBack(int distance, int count) {
    _reserve(count);
    for (var i = 0; i < count; i++) {
      _buf[_length] = _buf[_length - distance];
      _length += 1;
    }
  }

  void _reserve(int extra) {
    if (_length + extra > _limit) {
      _inflateFail('output-limit-exceeded');
    }
    if (_length + extra <= _buf.length) return;
    if (_length == _buf.length) _grow(_length + 1);
    if (_length + extra > _buf.length) _grow(_length + extra);
  }

  void _grow(int minCapacity) {
    var newCapacity = _buf.isEmpty ? 64 : _buf.length * 2;
    if (newCapacity < minCapacity) newCapacity = minCapacity;
    if (newCapacity > _limit) newCapacity = _limit;
    final newBuf = Uint8List(newCapacity);
    newBuf.setRange(0, _length, _buf);
    _buf = newBuf;
  }

  Uint8List toBytes() => Uint8List.sublistView(_buf, 0, _length);
}

// =============================================================================
// RFC 1951 DEFLATE — Decompress (inflate): stored, fixed, and dynamic blocks
// =============================================================================

/// Default upper bound on decompressed output — a guard against
/// decompression bombs (see the CMP09 spec's Security Considerations).
const int rawInflateMaxOutput = 256 * 1024 * 1024;

/// Backwards-compatible name for the raw-inflate hard and default ceiling.
const int defaultMaxOutputBytes = rawInflateMaxOutput;

void _validateOutputLimit(int maxOutput) {
  if (maxOutput < 0 || maxOutput > rawInflateMaxOutput) {
    _inflateFail('invalid-output-limit');
  }
}

/// RFC 1951 code-length alphabet transmission order (§3.2.7) — the order in
/// which the 3-bit code lengths for the *code-length alphabet itself*
/// appear on the wire, chosen so that common archives (which rarely use
/// the high-numbered CL symbols) can truncate the list early via `HCLEN`.
const List<int> _clOrder = <int>[
  16,
  17,
  18,
  0,
  8,
  7,
  9,
  6,
  10,
  5,
  11,
  4,
  12,
  3,
  13,
  2,
  14,
  1,
  15,
];

/// Read a dynamic block's transmitted Huffman tables (§3.2.7): the
/// code-length alphabet first, then the LL and distance tables — the
/// latter two are themselves RLE'd through the code-length alphabet, since
/// naively transmitting up to 288 + 30 raw code lengths would waste more
/// space than the tables save.
({_CanonicalDecoder ll, _CanonicalDecoder? dist}) _readDynamicTables(
  _BitReader br,
) {
  final hlit = br.readBits(5) + 257; // number of LL codes (257..286)
  final hdist = br.readBits(5) + 1; // number of distance codes (1..32)
  final hclen = br.readBits(4) + 4; // number of CL codes transmitted (4..19)

  if (hlit > 286) _inflateFail('invalid-literal-length-symbol');

  final clLengths = List<int>.filled(19, 0);
  for (var i = 0; i < hclen; i++) {
    clLengths[_clOrder[i]] = br.readBits(3);
  }
  final clDecoder = _CanonicalDecoder.fromLengths(clLengths);
  if (!clDecoder.isComplete) _inflateFail('incomplete-code-length-tree');

  // Decode HLIT + HDIST code lengths via the CL alphabet's RLE scheme:
  //   0-15: literal code length.
  //   16:   repeat the PREVIOUS length 3-6 times (2 extra bits, +3).
  //   17:   repeat length 0 for 3-10 times (3 extra bits, +3).
  //   18:   repeat length 0 for 11-138 times (7 extra bits, +11).
  final allLengths = <int>[];
  final total = hlit + hdist;
  while (allLengths.length < total) {
    final sym = clDecoder.readSymbol(br, 'invalid-literal-length-symbol');
    if (sym <= 15) {
      allLengths.add(sym);
    } else if (sym == 16) {
      if (allLengths.isEmpty) {
        _inflateFail('repeat-without-previous');
      }
      final repeat = br.readBits(2) + 3;
      final prev = allLengths.last;
      if (allLengths.length + repeat > total) _inflateFail('repeat-overrun');
      for (var i = 0; i < repeat; i++) allLengths.add(prev);
    } else if (sym == 17) {
      final repeat = br.readBits(3) + 3;
      if (allLengths.length + repeat > total) _inflateFail('repeat-overrun');
      for (var i = 0; i < repeat; i++) allLengths.add(0);
    } else if (sym == 18) {
      final repeat = br.readBits(7) + 11;
      if (allLengths.length + repeat > total) _inflateFail('repeat-overrun');
      for (var i = 0; i < repeat; i++) allLengths.add(0);
    } else {
      _inflateFail('invalid-literal-length-symbol');
    }
  }

  final llLengths = allLengths.sublist(0, hlit);
  final distLengths = allLengths.sublist(hlit, total);

  final llDecoder = _CanonicalDecoder.fromLengths(llLengths);
  if (!llDecoder.isComplete) {
    _inflateFail('incomplete-literal-length-tree');
  }
  final distDecoder = distLengths.any((len) => len > 0)
      ? _CanonicalDecoder.fromLengths(distLengths)
      : null;
  if (distDecoder != null &&
      !distDecoder.isComplete &&
      !(distDecoder.symbolCount == 1 && distDecoder.oneBitSymbolCount == 1)) {
    _inflateFail('incomplete-distance-tree');
  }

  return (ll: llDecoder, dist: distDecoder);
}

/// Decode one Huffman-coded block (fixed or dynamic — the only difference
/// is which tables were built) into [out], stopping at the end-of-block
/// symbol (256).
void _decodeHuffmanBlock(
  _BitReader br,
  _ByteBuffer out,
  _CanonicalDecoder llDecoder,
  _CanonicalDecoder? distDecoder,
) {
  while (true) {
    final sym = llDecoder.readSymbol(br, 'invalid-literal-length-symbol');
    if (sym == 256) return; // end-of-block
    if (sym < 256) {
      out.add(sym);
      continue;
    }
    if (sym > 285) {
      _inflateFail('invalid-literal-length-symbol');
    }

    final (base, extraBits) = _lengthTable[sym - 257];
    final length = base + (extraBits > 0 ? br.readBits(extraBits) : 0);

    if (distDecoder == null) {
      _inflateFail('reserved-distance-symbol');
    }
    final distSym = distDecoder.readSymbol(br, 'reserved-distance-symbol');
    if (distSym >= _distTable.length) {
      _inflateFail('reserved-distance-symbol');
    }
    final (distBase, extraDistBits) = _distTable[distSym];
    final distance =
        distBase + (extraDistBits > 0 ? br.readBits(extraDistBits) : 0);

    if (distance <= 0 || distance > out.length) {
      _inflateFail('invalid-back-reference');
    }

    // Copy byte-by-byte: overlapping back-references (distance < length)
    // are common and must read bytes this same loop just wrote.
    out.copyBack(distance, length);
  }
}

/// Decode a raw RFC 1951 DEFLATE bit-stream — the standard decoder that
/// reads `zlib`/`gzip`/Office/`zip`(1) streams as well as [deflateCompress]'s
/// own fixed-Huffman-only output. Handles multiple blocks (loops until
/// `BFINAL=1`), which real encoders sometimes emit even though our own
/// writer never does.
({Uint8List output, int bytesConsumed}) _deflateDecompress(
  Uint8List data, {
  int maxOutput = defaultMaxOutputBytes,
}) {
  _validateOutputLimit(maxOutput);
  final br = _BitReader(data);
  final out = _ByteBuffer(maxOutput);

  while (true) {
    final bfinal = br.readBit();
    final btypeBit0 = br.readBit();
    final btypeBit1 = br.readBit();
    final btype = btypeBit0 | (btypeBit1 << 1);

    switch (btype) {
      case 0: // Stored — verbatim bytes, byte-aligned.
        br.alignToByte();
        final len = br.readBits(16);
        final nlen = br.readBits(16);
        if ((len ^ 0xFFFF) & 0xFFFF != nlen) {
          _inflateFail('stored-length-mismatch');
        }
        for (var i = 0; i < len; i++) {
          out.add(br.readByte());
        }
        break;

      case 1: // Fixed Huffman.
        _decodeHuffmanBlock(
          br,
          out,
          _fixedLlDecoder,
          _fixedDistDecoder,
        );
        break;

      case 2: // Dynamic Huffman.
        final tables = _readDynamicTables(br);
        _decodeHuffmanBlock(br, out, tables.ll, tables.dist);
        break;

      default:
        _inflateFail('reserved-block-type');
    }

    if (bfinal == 1) break;
  }

  return (output: out.toBytes(), bytesConsumed: br.bytesConsumed);
}

/// Compress [data] to a raw RFC 1951 stream with no ZIP, zlib, or gzip frame.
Uint8List rawDeflate(Uint8List data) => deflateCompress(data);

/// Decode a raw RFC 1951 stream and report the exact bytes consumed.
({Uint8List output, int bytesConsumed}) rawInflateCounted(
  Uint8List data, {
  int maxOutput = defaultMaxOutputBytes,
}) =>
    _deflateDecompress(data, maxOutput: maxOutput);

/// Decode a raw RFC 1951 stream with a caller-lowerable output ceiling.
Uint8List rawInflate(
  Uint8List data, {
  int maxOutput = defaultMaxOutputBytes,
}) =>
    rawInflateCounted(data, maxOutput: maxOutput).output;

/// Backwards-compatible alias for [rawInflate].
Uint8List inflate(
  Uint8List data, {
  int maxOutput = defaultMaxOutputBytes,
}) =>
    rawInflate(data, maxOutput: maxOutput);

// =============================================================================
// MS-DOS Date / Time Encoding
// =============================================================================
//
// ZIP stores timestamps in the 16-bit MS-DOS packed format inherited from
// FAT:
//   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
//   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
// The combined 32-bit value is `(date << 16) | time`.

/// Encode a `(year, month, day, hour, minute, second)` tuple into the
/// 32-bit MS-DOS datetime used by ZIP Local and Central Directory headers.
int dosDatetime(
    int year, int month, int day, int hour, int minute, int second) {
  final t = (hour << 11) | (minute << 5) | (second ~/ 2);
  final d = ((year - 1980) << 9) | (month << 5) | day;
  return (d << 16) | t;
}

/// Fixed timestamp (1980-01-01 00:00:00) used when no real mtime is
/// available — the DOS epoch itself, so every field is zero except the
/// mandatory day-of-month.
const int dosEpoch = 0x00210000;

// =============================================================================
// Byte-buffer helpers
// =============================================================================

void _writeU16LE(List<int> buf, int value) {
  buf.add(value & 0xFF);
  buf.add((value >> 8) & 0xFF);
}

void _writeU32LE(List<int> buf, int value) {
  buf.add(value & 0xFF);
  buf.add((value >> 8) & 0xFF);
  buf.add((value >> 16) & 0xFF);
  buf.add((value >> 24) & 0xFF);
}

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================
//
// ZipWriter accumulates entries in memory: for each file it writes a Local
// File Header immediately, then the (possibly compressed) data, records the
// metadata needed for the Central Directory, and assembles the full archive
// on `finish()`.
//
// Auto-compression policy: try DEFLATE; use it (method=8) only if the
// compressed output is strictly smaller than the original. Otherwise fall
// back to Stored (method=0) — the common case for already-compressed
// formats (JPEG, PNG, an inner ZIP) and for inputs too small for DEFLATE's
// block overhead to pay for itself.

class _CdRecord {
  final List<int> name;
  final int method;
  final int dosDatetime;
  final int crc;
  final int compressedSize;
  final int uncompressedSize;
  final int localOffset;
  final int externalAttrs;

  _CdRecord({
    required this.name,
    required this.method,
    required this.dosDatetime,
    required this.crc,
    required this.compressedSize,
    required this.uncompressedSize,
    required this.localOffset,
    required this.externalAttrs,
  });
}

/// Maximum number of entries a ZIP archive may hold — the 16-bit
/// `Num_Entries_Total` field's natural ceiling (this implementation does
/// not support the ZIP64 extension), and a guard against pathological
/// inputs regardless.
const int maxZipEntries = 65535;

/// Builds a ZIP archive incrementally in memory.
///
/// ```dart
/// final w = ZipWriter();
/// w.addFile('hello.txt', utf8.encode('hello, world!') as Uint8List);
/// w.addDirectory('mydir/');
/// final bytes = w.finish();
/// // bytes is a valid .zip file
/// ```
class ZipWriter {
  final List<int> _buf = <int>[];
  final List<_CdRecord> _entries = <_CdRecord>[];

  /// Unix mode for a regular file: `0o100644` (rw-r--r--).
  static const int _regularFileMode = 0x81A4;

  /// Unix mode for a directory: `0o040755` (rwxr-xr-x).
  static const int _directoryMode = 0x41ED;

  /// Add a file entry.
  ///
  /// If [compress] is true (the default), DEFLATE is attempted; the
  /// compressed form is used only if it is strictly smaller than the
  /// uncompressed original — otherwise the entry falls back to Stored.
  void addFile(String name, Uint8List data, {bool compress = true}) {
    _addEntry(name, data, compress, _regularFileMode);
  }

  /// Add a directory entry. [name] should end with `/` (per §File Naming,
  /// a trailing slash is exactly what marks an entry as a directory).
  void addDirectory(String name) {
    _addEntry(name, Uint8List(0), false, _directoryMode);
  }

  void _addEntry(String name, Uint8List data, bool compress, int unixMode) {
    if (_entries.length >= maxZipEntries) {
      throw StateError(
        'zip: archive cannot contain more than $maxZipEntries entries',
      );
    }

    final nameBytes = utf8.encode(name);
    final crc = crc32(data);
    final uncompressedSize = data.length;

    int method;
    List<int> fileData;
    if (compress && data.isNotEmpty) {
      final compressed = deflateCompress(data);
      if (compressed.length < data.length) {
        method = 8;
        fileData = compressed;
      } else {
        method = 0;
        fileData = data;
      }
    } else {
      method = 0;
      fileData = data;
    }

    final compressedSize = fileData.length;
    final localOffset = _buf.length;
    final versionNeeded = method == 8 ? 20 : 10;
    const flags = 0x0800; // bit 11 = UTF-8 filename

    _writeU32LE(_buf, 0x04034B50);
    _writeU16LE(_buf, versionNeeded);
    _writeU16LE(_buf, flags);
    _writeU16LE(_buf, method);
    _writeU16LE(_buf, dosEpoch & 0xFFFF); // mod_time
    _writeU16LE(_buf, (dosEpoch >> 16) & 0xFFFF); // mod_date
    _writeU32LE(_buf, crc);
    _writeU32LE(_buf, compressedSize);
    _writeU32LE(_buf, uncompressedSize);
    _writeU16LE(_buf, nameBytes.length);
    _writeU16LE(_buf, 0); // extra_field_length
    _buf.addAll(nameBytes);
    _buf.addAll(fileData);

    _entries.add(
      _CdRecord(
        name: nameBytes,
        method: method,
        dosDatetime: dosEpoch,
        crc: crc,
        compressedSize: compressedSize,
        uncompressedSize: uncompressedSize,
        localOffset: localOffset,
        externalAttrs: unixMode << 16,
      ),
    );
  }

  /// Finish writing: append the Central Directory and EOCD, and return the
  /// complete archive bytes. The writer must not be reused afterward.
  Uint8List finish() {
    final cdOffset = _buf.length;
    final numEntries = _entries.length;

    for (final e in _entries) {
      final versionNeeded = e.method == 8 ? 20 : 10;
      _writeU32LE(_buf, 0x02014B50);
      _writeU16LE(_buf, 0x031E); // version_made_by: Unix, spec v3.0 (0x1E=30)
      _writeU16LE(_buf, versionNeeded);
      _writeU16LE(_buf, 0x0800); // flags (UTF-8)
      _writeU16LE(_buf, e.method);
      _writeU16LE(_buf, e.dosDatetime & 0xFFFF);
      _writeU16LE(_buf, (e.dosDatetime >> 16) & 0xFFFF);
      _writeU32LE(_buf, e.crc);
      _writeU32LE(_buf, e.compressedSize);
      _writeU32LE(_buf, e.uncompressedSize);
      _writeU16LE(_buf, e.name.length);
      _writeU16LE(_buf, 0); // extra_len
      _writeU16LE(_buf, 0); // comment_len
      _writeU16LE(_buf, 0); // disk_start
      _writeU16LE(_buf, 0); // internal_attrs
      _writeU32LE(_buf, e.externalAttrs);
      _writeU32LE(_buf, e.localOffset);
      _buf.addAll(e.name);
    }
    final cdSize = _buf.length - cdOffset;

    _writeU32LE(_buf, 0x06054B50);
    _writeU16LE(_buf, 0); // disk_number
    _writeU16LE(_buf, 0); // disk_with_cd_start
    _writeU16LE(_buf, numEntries); // entries on this disk
    _writeU16LE(_buf, numEntries); // entries total
    _writeU32LE(_buf, cdSize);
    _writeU32LE(_buf, cdOffset);
    _writeU16LE(_buf, 0); // comment_len

    return Uint8List.fromList(_buf);
  }
}

// =============================================================================
// ZIP Read — ZipEntry and ZipReader
// =============================================================================
//
// ZipReader uses the "EOCD-first" strategy for reliable random access:
//
//   1. Scan backward for the EOCD signature (PK\x05\x06), bounded to the
//      last 22 + 65535 bytes (the EOCD's comment field can be up to 65535
//      bytes) — an unbounded scan over an attacker-controlled file would be
//      a denial-of-service vector.
//   2. Read the CD offset and size from the EOCD.
//   3. Parse every Central Directory header into a [ZipEntry].
//   4. On [read], seek to the entry's Local Header via its recorded
//      offset, skip the (possibly different-length) name + extra fields,
//      read exactly `compressed_size` bytes, decompress, and verify CRC-32.
//
// The Central Directory is the *authoritative* source for size and method
// (per the CMP09 spec's Security Considerations); the Local Header is only
// consulted for its variable-length name/extra fields so the reader knows
// where the entry's data actually starts.

/// Metadata for a single entry inside a ZIP archive.
class ZipEntry {
  /// File name (UTF-8).
  final String name;

  /// Uncompressed size in bytes.
  final int size;

  /// Compressed size in bytes.
  final int compressedSize;

  /// Compression method: 0 = Stored, 8 = DEFLATE.
  final int method;

  /// CRC-32 of the uncompressed content.
  final int crc32;

  /// True if this entry is a directory (name ends with `/`).
  final bool isDirectory;

  /// Byte offset of this entry's Local File Header within the archive.
  final int localOffset;

  /// True if the Central Directory's General_Purpose_Bit_Flag marks this
  /// entry as encrypted (bit 0). Read from the Central Directory — the
  /// authoritative header — rather than the Local Header, so a crafted
  /// archive cannot hide encryption by disagreeing between the two copies
  /// of the flags field.
  final bool isEncrypted;

  const ZipEntry._({
    required this.name,
    required this.size,
    required this.compressedSize,
    required this.method,
    required this.crc32,
    required this.isDirectory,
    required this.localOffset,
    required this.isEncrypted,
  });

  @override
  String toString() =>
      'ZipEntry(name: $name, size: $size, compressedSize: $compressedSize, '
      'method: $method, isDirectory: $isDirectory)';
}

/// Reads entries from an in-memory ZIP archive.
///
/// ```dart
/// final reader = ZipReader(archiveBytes);
/// for (final entry in reader.entries()) {
///   print('${entry.name}: ${entry.size} bytes');
/// }
/// ```
class ZipReader {
  final Uint8List _data;
  final List<ZipEntry> _entries;

  ZipReader._(this._data, this._entries);

  /// Parse an in-memory ZIP archive.
  ///
  /// Throws [FormatException] if no valid EOCD record is found or the
  /// archive is structurally malformed.
  factory ZipReader(Uint8List data) {
    final eocdOffset = _findEocd(data);
    if (eocdOffset == null) {
      throw const FormatException(
        'zip: no End of Central Directory record found',
      );
    }

    final view = ByteData.sublistView(data);
    final cdOffset = view.getUint32(eocdOffset + 16, Endian.little);
    final cdSize = view.getUint32(eocdOffset + 12, Endian.little);

    if (cdOffset > data.length || cdOffset + cdSize > data.length) {
      throw FormatException(
        'zip: Central Directory [$cdOffset, ${cdOffset + cdSize}) out of '
        'bounds (file size ${data.length})',
      );
    }

    final entries = <ZipEntry>[];
    var pos = cdOffset;
    final cdEnd = cdOffset + cdSize;

    while (pos + 4 <= cdEnd) {
      final sig = view.getUint32(pos, Endian.little);
      if (sig != 0x02014B50) break; // end of CD (or trailing padding)

      if (pos + 46 > data.length) {
        throw const FormatException('zip: Central Directory entry truncated');
      }

      final cdFlags = view.getUint16(pos + 8, Endian.little);
      final method = view.getUint16(pos + 10, Endian.little);
      final crc = view.getUint32(pos + 16, Endian.little);
      final compressedSize = view.getUint32(pos + 20, Endian.little);
      final size = view.getUint32(pos + 24, Endian.little);
      final nameLen = view.getUint16(pos + 28, Endian.little);
      final extraLen = view.getUint16(pos + 30, Endian.little);
      final commentLen = view.getUint16(pos + 32, Endian.little);
      final localOffset = view.getUint32(pos + 42, Endian.little);

      final nameStart = pos + 46;
      final nameEnd = nameStart + nameLen;
      if (nameEnd > data.length) {
        throw const FormatException(
          'zip: Central Directory entry name out of bounds',
        );
      }
      final name = utf8.decode(
        data.sublist(nameStart, nameEnd),
        allowMalformed: true,
      );
      final isDirectory = name.endsWith('/');

      if (localOffset > data.length) {
        throw const FormatException(
          'zip: Central Directory entry local offset out of bounds',
        );
      }

      entries.add(
        ZipEntry._(
          name: name,
          size: size,
          compressedSize: compressedSize,
          method: method,
          crc32: crc,
          isDirectory: isDirectory,
          localOffset: localOffset,
          isEncrypted: cdFlags & 1 != 0,
        ),
      );
      if (entries.length > maxZipEntries) {
        throw const FormatException(
          'zip: more than 65535 Central Directory entries (possible zip bomb)',
        );
      }

      pos = nameEnd + extraLen + commentLen;
    }

    // Each entry's advance to the next header trusts that entry's own
    // (attacker-controlled) name/extra/comment lengths. A crafted archive
    // that inflates one of those fields desyncs `pos` from the real next
    // header — the next signature check then fails and the loop above
    // exits via `break`, silently returning a truncated entry list that
    // still looks like a well-formed (if smaller) archive. Cross-check
    // against the EOCD's own declared entry count so a desync is reported
    // as corruption instead of silently dropping entries.
    final declaredEntries = view.getUint16(eocdOffset + 10, Endian.little);
    if (entries.length != declaredEntries) {
      throw FormatException(
        'zip: Central Directory declares $declaredEntries entries but only '
        '${entries.length} were parsed (archive is truncated or corrupt)',
      );
    }

    return ZipReader._(data, entries);
  }

  /// Return all entries in the archive (files and directories), in
  /// Central Directory order.
  List<ZipEntry> entries() => List.unmodifiable(_entries);

  /// Decompress and return the data for [entry]. Verifies CRC-32.
  ///
  /// Throws [FormatException] on CRC mismatch, an unsupported compression
  /// method, an encrypted entry, or a structurally corrupt entry. Throws
  /// [ArgumentError] if the declared or actual decompressed size would
  /// exceed [maxUncompressedBytes] — the decompression-bomb guard from the
  /// CMP09 spec's Security Considerations.
  Uint8List read(
    ZipEntry entry, {
    int maxUncompressedBytes = defaultMaxOutputBytes,
  }) {
    if (entry.isDirectory) return Uint8List(0);

    // Check the Central Directory's copy of the encrypted flag (the
    // authoritative header) up front; the Local Header's copy is checked
    // again below once it's been read, in case the two disagree.
    if (entry.isEncrypted) {
      throw FormatException(
        "zip: entry '${entry.name}' is encrypted; not supported",
      );
    }

    if (entry.size > maxUncompressedBytes) {
      throw ArgumentError(
        "zip: entry '${entry.name}' declared size ${entry.size} exceeds "
        'the configured limit of $maxUncompressedBytes bytes',
      );
    }

    final view = ByteData.sublistView(_data);
    final lhOff = entry.localOffset;
    if (lhOff + 30 > _data.length) {
      throw const FormatException('zip: Local File Header out of bounds');
    }
    final sig = view.getUint32(lhOff, Endian.little);
    if (sig != 0x04034B50) {
      throw const FormatException(
        'zip: bad Local File Header signature',
      );
    }

    final localFlags = view.getUint16(lhOff + 6, Endian.little);
    if (localFlags & 1 != 0) {
      throw FormatException(
        "zip: entry '${entry.name}' is encrypted; not supported",
      );
    }

    // The Local Header's name/extra fields can differ in length from the
    // Central Directory's (rare, but legal) — re-read them here rather than
    // trusting the CD's lengths for this skip calculation.
    final lhNameLen = view.getUint16(lhOff + 26, Endian.little);
    final lhExtraLen = view.getUint16(lhOff + 28, Endian.little);
    final dataStart = lhOff + 30 + lhNameLen + lhExtraLen;
    final dataEnd = dataStart + entry.compressedSize;

    if (dataEnd > _data.length || dataStart > dataEnd) {
      throw FormatException(
        "zip: entry '${entry.name}' data [$dataStart, $dataEnd) out of bounds",
      );
    }
    final compressed = _data.sublist(dataStart, dataEnd);

    Uint8List decompressed;
    switch (entry.method) {
      case 0:
        decompressed = compressed;
        break;
      case 8:
        final result = rawInflateCounted(
          compressed,
          maxOutput: entry.size < maxUncompressedBytes
              ? entry.size
              : maxUncompressedBytes,
        );
        if (result.bytesConsumed != compressed.length) {
          throw const FormatException(
            'zip: DEFLATE stream does not consume its declared payload',
          );
        }
        decompressed = result.output;
        break;
      default:
        throw FormatException(
          "zip: unsupported compression method ${entry.method} for "
          "'${entry.name}' (only Stored=0 and DEFLATE=8 are supported)",
        );
    }

    // Trim to the declared uncompressed size (guards against a
    // decompressor that over-reads relative to what the header promised).
    if (decompressed.length > entry.size) {
      decompressed = decompressed.sublist(0, entry.size);
    }

    final actualCrc = crc32(decompressed);
    if (actualCrc != entry.crc32) {
      throw FormatException(
        "zip: CRC-32 mismatch for '${entry.name}': expected "
        '${entry.crc32.toRadixString(16).padLeft(8, '0')}, got '
        '${actualCrc.toRadixString(16).padLeft(8, '0')}',
      );
    }

    return decompressed;
  }

  /// Find an entry by name and return its decompressed data.
  Uint8List readByName(String name) {
    final entry = _entries.firstWhere(
      (e) => e.name == name,
      orElse: () => throw FormatException("zip: entry '$name' not found"),
    );
    return read(entry);
  }

  /// Scan backward from the end of [data] for the EOCD signature
  /// `0x06054B50`.
  ///
  /// The EOCD record is at most `22 + 65535` bytes from the end of the file
  /// (the trailing comment field can be 0–65535 bytes). The scan is bounded
  /// to that range so a crafted file cannot force an unbounded backward
  /// search.
  static int? _findEocd(Uint8List data) {
    const sig = 0x06054B50;
    const maxComment = 65535;
    const eocdMinSize = 22;

    if (data.length < eocdMinSize) return null;

    final view = ByteData.sublistView(data);
    final scanStart = data.length > eocdMinSize + maxComment
        ? data.length - eocdMinSize - maxComment
        : 0;

    for (var i = data.length - eocdMinSize; i >= scanStart; i--) {
      if (view.getUint32(i, Endian.little) == sig) {
        final commentLen = view.getUint16(i + 20, Endian.little);
        if (i + eocdMinSize + commentLen == data.length) {
          return i;
        }
      }
    }
    return null;
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Compress a list of `(name, data)` pairs into a ZIP archive.
///
/// Each file is compressed with DEFLATE if it reduces size; otherwise
/// stored verbatim.
///
/// ```dart
/// final archive = zipBytes([('hello.txt', utf8.encode('hello, world!'))]);
/// // archive is a valid .zip file
/// ```
Uint8List zipBytes(List<(String name, List<int> data)> entries) {
  final w = ZipWriter();
  for (final (name, data) in entries) {
    w.addFile(name, Uint8List.fromList(data));
  }
  return w.finish();
}

/// Decompress all file entries from a ZIP archive into a `{name: data}` map.
///
/// Directories (names ending with `/`) are skipped — call [ZipReader]
/// directly if you need to observe directory entries.
///
/// ```dart
/// final files = unzip(archive);
/// files['hello.txt']; // the decompressed bytes
/// ```
Map<String, Uint8List> unzip(Uint8List data) {
  final reader = ZipReader(data);
  final out = <String, Uint8List>{};
  for (final entry in reader.entries()) {
    if (!entry.isDirectory) {
      out[entry.name] = reader.read(entry);
    }
  }
  return out;
}
