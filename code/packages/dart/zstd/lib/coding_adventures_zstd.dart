/// Zstandard (ZStd) lossless compression — CMP07.
///
/// Zstandard (RFC 8878) is a high-ratio, fast compression format created by
/// Yann Collet at Facebook (2015). It combines:
///
/// - **LZ77 back-references** (via the `coding_adventures_lzss` package) to
///   exploit repetition in the data — the same "copy from earlier in the
///   output" trick as DEFLATE, but with a 32 KB window.
/// - **FSE (Finite State Entropy)** coding instead of Huffman for the
///   sequence descriptor symbols. FSE is an asymmetric numeral system that
///   approaches the Shannon entropy limit in a single pass.
/// - **Predefined decode tables** (RFC 8878 Appendix B) so short frames
///   need no table-description overhead.
///
/// # Frame layout (RFC 8878 §3)
///
/// ```
/// ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
/// │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
/// │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
/// └────────┴─────┴──────────────────────┴────────┴──────────────────┘
/// ```
///
/// Each **block** has a 3-byte header:
/// ```
/// bit  0      = Last_Block flag
/// bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
/// bits [23:3] = Block_Size
/// ```
///
/// # Compression strategy
///
/// 1. Split data into 128 KB blocks ([_maxBlockSize]).
/// 2. For each block, try in order:
///    a. **RLE** — all bytes identical → 4 bytes total (3-byte header + 1 payload).
///    b. **Compressed** (LZ77 + FSE) — if output < input length.
///    c. **Raw** — verbatim copy as fallback.
library coding_adventures_zstd;

import 'dart:typed_data';
import 'package:coding_adventures_lzss/lzss.dart' as lzss;

// ─── Constants ────────────────────────────────────────────────────────────────

/// ZStd magic number: `0xFD2FB528` (little-endian on the wire: `28 B5 2F FD`).
///
/// Every valid ZStd frame must start with these 4 bytes. The value was chosen
/// to be unlikely to appear at the start of arbitrary plaintext files.
const int _magic = 0xFD2FB528;

/// Maximum block size: 128 KB.
///
/// ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
/// blocks. The spec maximum is `min(WindowSize, 128 KB)`.
const int _maxBlockSize = 128 * 1024;

/// Maximum allowed decompressed output size (256 MB).
///
/// This is a safety guard against decompression bombs: inputs that claim to
/// expand to many gigabytes. Any frame whose uncompressed content would exceed
/// this limit is rejected.
const int _maxOutput = 256 * 1024 * 1024;

// ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────────
//
// Each sequence stores three numbers: literal_length (LL), match_length (ML),
// and match_offset (OF). Rather than encoding these raw values, ZStd maps them
// to a (code, extra_bits) pair. The code number selects a row in the table
// below; the extra bits refine the value within the range that the code covers.
//
// Example: LL code 16 covers literal lengths 16–17 (baseline=16, extra=1 bit).
//   If the extra bit is 0 → length 16; if 1 → length 17.

/// Literal Length code table: (baseline, extraBits) for codes 0..=35.
///
/// Codes 0..15 are one-to-one with literal lengths 0..15 (0 extra bits).
/// Codes 16..35 cover increasing ranges with more extra bits.
const List<(int, int)> _llCodes = [
  (0, 0),   (1, 0),   (2, 0),   (3, 0),   (4, 0),   (5, 0),
  (6, 0),   (7, 0),   (8, 0),   (9, 0),   (10, 0),  (11, 0),
  (12, 0),  (13, 0),  (14, 0),  (15, 0),
  // Grouped ranges start at code 16.
  (16, 1),  (18, 1),  (20, 1),  (22, 1),
  (24, 2),  (28, 2),
  (32, 3),  (40, 3),
  (48, 4),  (64, 6),
  (128, 7), (256, 8), (512, 9), (1024, 10), (2048, 11), (4096, 12),
  (8192, 13), (16384, 14), (32768, 15), (65536, 16),
];

/// Match Length code table: (baseline, extraBits) for codes 0..=52.
///
/// ZStd's minimum match length is 3 bytes, so code 0 = match length 3.
/// Codes 0..31 are individual values 3..34; codes 32+ cover ranges.
const List<(int, int)> _mlCodes = [
  // codes 0..31: individual values 3..34
  (3, 0),  (4, 0),  (5, 0),  (6, 0),  (7, 0),  (8, 0),
  (9, 0),  (10, 0), (11, 0), (12, 0), (13, 0), (14, 0),
  (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0),
  (21, 0), (22, 0), (23, 0), (24, 0), (25, 0), (26, 0),
  (27, 0), (28, 0), (29, 0), (30, 0), (31, 0), (32, 0),
  (33, 0), (34, 0),
  // codes 32+: grouped ranges
  (35, 1),  (37, 1),  (39, 1),   (41, 1),
  (43, 2),  (47, 2),
  (51, 3),  (59, 3),
  (67, 4),  (83, 4),
  (99, 5),  (131, 7),
  (259, 8), (515, 9), (1027, 10), (2051, 11),
  (4099, 12), (8195, 13), (16387, 14), (32771, 15), (65539, 16),
];

// ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted.
// The decoder builds the same FSE decode table from these fixed distributions.
//
// Entries of -1 mean probability 1/table_size — these symbols each get exactly
// one slot in the decode table and their encoder state never needs extra bits.

/// Predefined normalised distribution for Literal Length FSE.
/// Table accuracy log = 6 → table_size = 64 slots.
const List<int> _llNorm = [
   4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
   2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
  -1, -1, -1, -1,
];
const int _llAccLog = 6; // table_size = 64

/// Predefined normalised distribution for Match Length FSE.
/// Table accuracy log = 6 → table_size = 64 slots.
const List<int> _mlNorm = [
   1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
   1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
   1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
  -1, -1, -1, -1, -1,
];
const int _mlAccLog = 6;

/// Predefined normalised distribution for Offset FSE.
/// Table accuracy log = 5 → table_size = 32 slots.
const List<int> _ofNorm = [
   1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
   1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
];
const int _ofAccLog = 5; // table_size = 32

// ─── FSE decode table entry ───────────────────────────────────────────────────

/// One cell in the FSE decode table.
///
/// To decode a symbol from state [s]:
///   1. [sym] is the output symbol.
///   2. Read [nb] bits from the bitstream → [bits].
///   3. New state = [base] + [bits].
///
/// The table has `1 << accLog` entries. State values at decode time are in the
/// range `[0, table_size)`. State values used by the encoder are in
/// `[table_size, 2*table_size)`.
class _FseDe {
  final int sym;  // decoded symbol (index into the norm distribution)
  final int nb;   // number of extra bits to read for the next state
  final int base; // base value added to those bits to form the next state

  const _FseDe(this.sym, this.nb, this.base);
}

/// Build an FSE decode table from a normalised probability distribution.
///
/// The algorithm has three phases:
///
/// **Phase 1 — place probability -1 symbols at the top (high indices).**
/// These very-rare symbols each occupy exactly one slot. Because they are at
/// the high end, the step function in phase 2 naturally avoids them.
///
/// **Phase 2 — spread the remaining symbols.**
/// The step function `step = (sz >> 1) + (sz >> 3) + 3` is chosen so that
/// it is co-prime to `sz` (which is always a power of two in ZStd). Walking
/// with this step visits every slot in `[0, idx_limit]` exactly once, so each
/// symbol fills exactly as many slots as its normalised probability.
///
/// **Phase 3 — assign `nb` (number of next-state bits) and `base`.**
/// For a symbol with `count` occurrences, the j-th slot (in ascending index
/// order, starting at j=count) sets:
/// ```
///   nb   = accLog - floor(log2(j))
///   base = j * (1 << nb) - sz
/// ```
/// This ensures that `base + read(nb bits)` always lands back in `[0, sz)`.
List<_FseDe> _buildDecodeTable(List<int> norm, int accLog) {
  final sz = 1 << accLog;
  // Step function: co-prime to sz (a power of two) so it visits all slots.
  final step = (sz >> 1) + (sz >> 3) + 3;

  final tbl = List<_FseDe>.filled(sz, const _FseDe(0, 0, 0));
  // symNext[s] will track the "next counter" for symbol s during phase 3.
  // During phases 1 and 2, we use it to store the probability count.
  final symNext = List<int>.filled(norm.length, 0);

  // Phase 1: -1 probability symbols occupy the high indices.
  var high = sz - 1;
  for (var s = 0; s < norm.length; s++) {
    if (norm[s] == -1) {
      tbl[high] = _FseDe(s, 0, 0); // nb/base filled in phase 3
      if (high > 0) high--;
      symNext[s] = 1; // count = 1 for these symbols
    }
  }

  // Phase 2: spread the remaining symbols into the lower portion.
  // Two-pass approach (matching the Rust reference):
  //   Pass 0: symbols with count > 1 (spread first to avoid clustering)
  //   Pass 1: symbols with count == 1
  var pos = 0;
  for (var pass = 0; pass < 2; pass++) {
    for (var s = 0; s < norm.length; s++) {
      final c = norm[s];
      if (c <= 0) continue;
      final cnt = c;
      // Process in the correct pass:
      //   pass 0 handles symbols with count > 1
      //   pass 1 handles symbols with count == 1
      if ((pass == 0) != (cnt > 1)) continue;
      symNext[s] = cnt;
      for (var k = 0; k < cnt; k++) {
        tbl[pos] = _FseDe(s, 0, 0);
        pos = (pos + step) & (sz - 1);
        // Skip slots reserved for -1 probability symbols.
        while (pos > high) {
          pos = (pos + step) & (sz - 1);
        }
      }
    }
  }

  // Phase 3: assign nb and base for each slot.
  //
  // We iterate in ascending slot order. For each slot we track which
  // occurrence (j) of symbol s this is, using sn[s] (starts at count[s],
  // increments each time we see that symbol).
  //
  // floor(log2(ns)):
  //   Compute by right-shifting ns until it becomes 1. Each shift is one
  //   power of two. This is equivalent to (bit_length(ns) - 1).
  final sn = List<int>.from(symNext);
  for (var i = 0; i < sz; i++) {
    final s = tbl[i].sym;
    final ns = sn[s];
    sn[s]++;
    // Compute floor(log2(ns)) by counting how many times we can halve ns.
    var fl2 = 0;
    var tmp = ns;
    while (tmp > 1) {
      tmp >>= 1;
      fl2++;
    }
    final nb = accLog - fl2;
    // base = ns * (1 << nb) - sz
    // This is always non-negative because ns >= 1 and nb = accLog - floor(log2(ns)),
    // so ns * (1 << nb) >= sz.
    final base = (ns << nb) - sz;
    tbl[i] = _FseDe(s, nb, base);
  }

  return tbl;
}

// ─── FSE encode table entry ───────────────────────────────────────────────────

/// Encode transform for one symbol.
///
/// Given encoder state S (in range `[sz, 2*sz)`):
/// 1. `nb_out = (S + deltaNb) >> 16`   — number of bits to emit
/// 2. Emit the low `nb_out` bits of S to the backward bitstream.
/// 3. New state = `st[(S >> nb_out) + deltaFs]`
///
/// The [deltaNb] and [deltaFs] values are precomputed from the distribution
/// so the hot-path encode loop only needs arithmetic and one table lookup.
class _FseEe {
  final int deltaNb; // (maxBitsOut << 16) - (count << maxBitsOut)
  final int deltaFs; // cumul[sym] - count (may be negative)

  const _FseEe(this.deltaNb, this.deltaFs);
}

/// Build FSE encode tables from a normalised distribution.
///
/// Returns a record with:
/// - `ee`: one [_FseEe] entry per symbol (indexed by symbol number).
/// - `st`: the encoder state table (slot → output state in `[sz, 2*sz)`).
///
/// ### Encode/decode symmetry
///
/// The decoder assigns `(sym, nb, base)` to each cell in **index order** as
/// described in [_buildDecodeTable]. The encoder must use the same ordering
/// so that after encoding symbol s from encode-slot `cumul[s]+j`, the decoder
/// at the corresponding decode-table index reads back the same bits and
/// reconstructs the encoder's pre-encoding state.
({List<_FseEe> ee, List<int> st}) _buildEncodeSym(List<int> norm, int accLog) {
  final sz = 1 << accLog;

  // Step 1: compute cumulative sums of the probabilities.
  // cumul[s] = sum of all counts for symbols 0..(s-1).
  // These cumulative values are the starting encode-slot indices for each symbol.
  final cumul = List<int>.filled(norm.length, 0);
  var total = 0;
  for (var s = 0; s < norm.length; s++) {
    cumul[s] = total;
    final cnt = norm[s] == -1 ? 1 : (norm[s] > 0 ? norm[s] : 0);
    total += cnt;
  }

  // Step 2: build the spread table using the same spreading algorithm as
  // _buildDecodeTable. spread[i] = which symbol was placed at decode-table slot i.
  // We need this to match the decoder's slot ordering exactly.
  final step = (sz >> 1) + (sz >> 3) + 3;
  final spread = List<int>.filled(sz, 0);
  var idxHigh = sz - 1;

  // Phase 1: -1 probability symbols at the high end.
  for (var s = 0; s < norm.length; s++) {
    if (norm[s] == -1) {
      spread[idxHigh] = s;
      if (idxHigh > 0) idxHigh--;
    }
  }
  final idxLimit = idxHigh; // highest free slot for phase 2

  // Phase 2: spread remaining symbols.
  var pos = 0;
  for (var pass = 0; pass < 2; pass++) {
    for (var s = 0; s < norm.length; s++) {
      if (norm[s] <= 0) continue;
      final cnt = norm[s];
      if ((pass == 0) != (cnt > 1)) continue;
      for (var k = 0; k < cnt; k++) {
        spread[pos] = s;
        pos = (pos + step) & (sz - 1);
        while (pos > idxLimit) {
          pos = (pos + step) & (sz - 1);
        }
      }
    }
  }

  // Step 3: build the state table.
  //
  // Iterate the spread table in ascending index order. For each index i,
  // determine which occurrence (j) of symbol s = spread[i] this is. The
  // encoder stores output state `i + sz` in encode-slot `cumul[s] + j`.
  //
  // When the encoder is in state `i + sz` for symbol s, the decoder at
  // decode-table index i will decode s and reconstruct the encoder's state.
  final symOcc = List<int>.filled(norm.length, 0);
  final st = List<int>.filled(sz, 0);

  for (var i = 0; i < sz; i++) {
    final s = spread[i];
    final j = symOcc[s];
    symOcc[s]++;
    final slot = cumul[s] + j;
    st[slot] = i + sz; // output state = decode-table index + sz
  }

  // Step 4: build FseEe entries.
  //
  // For symbol s with count c:
  //   maxBitsOut = accLog - floor(log2(c))   [0 extra bits if c == sz, etc.]
  //   deltaNb    = (maxBitsOut << 16) - (c << maxBitsOut)
  //   deltaFs    = cumul[s] - c
  //
  // Encoding step: given current encoder state E ∈ [sz, 2*sz):
  //   nb      = (E + deltaNb) >> 16    — state bits to emit
  //   emit low nb bits of E
  //   new E   = st[(E >> nb) + deltaFs]
  final ee = List<_FseEe>.filled(norm.length, const _FseEe(0, 0));
  for (var s = 0; s < norm.length; s++) {
    final cnt = norm[s] == -1 ? 1 : (norm[s] > 0 ? norm[s] : 0);
    if (cnt == 0) continue;
    // maxBitsOut = accLog - floor(log2(cnt))
    final int mbo;
    if (cnt == 1) {
      mbo = accLog; // log2(1) = 0, so mbo = accLog
    } else {
      var fl2 = 0;
      var tmp = cnt;
      while (tmp > 1) {
        tmp >>= 1;
        fl2++;
      }
      mbo = accLog - fl2;
    }
    // deltaNb: wrapping subtraction handled naturally in Dart 64-bit int.
    final deltaNb = (mbo << 16) - (cnt << mbo);
    final deltaFs = cumul[s] - cnt;
    ee[s] = _FseEe(deltaNb, deltaFs);
  }

  return (ee: ee, st: st);
}

// ─── Reverse bit-writer ───────────────────────────────────────────────────────
//
// ZStd's sequence bitstream is written *backwards* relative to the data flow:
// the encoder writes bits that the decoder will read last, first. This allows
// the decoder to read a forward-only stream while decoding sequences in order.
//
// Byte layout: [byte0, byte1, ..., byteN] where byteN is the last byte
// written and contains a **sentinel bit** (the highest set bit) that marks
// where the meaningful data ends. The decoder initialises by finding this
// sentinel in the last byte.
//
// Bit layout within each byte: LSB = first bit written.
//
// Example: write bits 1, 0, 1, 1 (4 bits total) then flush:
//   reg = 0b1011, bits = 4
//   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
//   Output: [0x1B]
//
// The decoder reads 0x1B, finds the MSB (sentinel) at position 4, then reads
// the 4 data bits below it as 0b1011 = the original bits in LSB-first order.

class _RevBitWriter {
  final List<int> _buf = [];
  int _reg = 0;  // accumulation register; bits fill from the LSB upward
  int _bits = 0; // number of valid bits currently in _reg

  /// Add the low [nb] bits of [val] to the backward bitstream.
  ///
  /// Bits accumulate in [_reg] from LSB to MSB. When there are at least 8
  /// valid bits, the low byte is flushed to [_buf].
  void addBits(int val, int nb) {
    if (nb == 0) return;
    // Mask off any bits above position nb-1.
    final mask = nb == 64 ? -1 : (1 << nb) - 1;
    _reg |= (val & mask) << _bits;
    _bits += nb;
    // Flush complete bytes (8 bits each) from the low end of _reg.
    while (_bits >= 8) {
      _buf.add(_reg & 0xFF);
      _reg >>>= 8; // logical right shift: no sign extension
      _bits -= 8;
    }
  }

  /// Flush remaining partial-byte bits with a sentinel and seal the stream.
  ///
  /// The sentinel is a `1` bit placed at bit position [_bits] in the last
  /// byte. The decoder locates this bit by finding the highest set bit in
  /// the last byte, which tells it how many data bits are valid below it.
  ///
  /// Example: _reg = 0b101 (_bits = 3)
  ///   sentinel = 1 << 3 = 0b1000
  ///   last byte = 0b101 | 0b1000 = 0b1101 = 0x0D
  void flush() {
    final sentinel = 1 << _bits; // bit above all data bits
    _buf.add((_reg & 0xFF) | sentinel);
    _reg = 0;
    _bits = 0;
  }

  /// Return the completed byte buffer as a [Uint8List].
  Uint8List finish() => Uint8List.fromList(_buf);
}

// ─── Reverse bit-reader ───────────────────────────────────────────────────────
//
// Mirrors _RevBitWriter: reads bits from the END of the buffer going backwards.
// The stream is laid out so the LAST bits written by the encoder are at the
// END of the byte buffer (in the sentinel-containing last byte). The reader
// initialises at the last byte and reads backward toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// read_bits(n) extracts the top n bits and shifts the register left by n.
//
// Why left-aligned? The writer fills LSB-first. Within each flushed byte,
// bit 0 = earliest written, bit 7 = latest. To read the LATEST bits first
// (the highest byte/bit positions), we need a left-aligned register so that
// extracting from the top gives the highest-position bits first.

class _RevBitReader {
  final Uint8List _data;
  int _reg = 0;   // shift register; valid bits are packed at the TOP (MSB)
  int _bits = 0;  // how many valid bits are loaded (count from MSB downward)
  int _pos = 0;   // next byte index to load (decrements toward 0)

  _RevBitReader(Uint8List data) : _data = data {
    if (data.isEmpty) throw ArgumentError('empty bitstream');

    // Find the sentinel bit in the last byte.
    // The sentinel is the highest set bit; valid data bits are below it.
    final last = data.last;
    if (last == 0) throw ArgumentError('bitstream last byte is zero (no sentinel)');

    // sentinel_pos = bit position (0=LSB) of the highest set bit.
    // valid_bits = number of data bits strictly below the sentinel.
    int sp = 0;
    while ((1 << (sp + 1)) <= last) sp++;
    final validBits = sp; // data bits = bits 0..(sp-1)

    // Place the valid data bits of the sentinel byte at the TOP of the register.
    // Example: last = 0b0001_1110, sentinel at bit 4, validBits = 4.
    //   data bits = last & ((1<<4)-1) = 0b1110.
    //   After shifting to top of 64-bit reg: bits 63..60 = 1110.
    final mask = validBits > 0 ? (1 << validBits) - 1 : 0;
    _reg = validBits > 0 ? (last & mask) << (64 - validBits) : 0;
    _bits = validBits;
    _pos = data.length - 1; // sentinel byte consumed; load from pos-1 onward

    _reload();
  }

  /// Load more bytes into the register from the stream (going backwards).
  ///
  /// Each new byte is placed just below the currently loaded bits. In the
  /// left-aligned register that means at bit position `(64 - _bits - 8)`.
  ///
  /// We reload whenever _bits drops below 24 to keep the register full.
  /// The threshold 56 here means: keep loading as long as there's room for
  /// another full byte and there are bytes remaining.
  void _reload() {
    while (_bits <= 56 && _pos > 0) {
      _pos--;
      // Place this byte just below the currently valid bits.
      final shift = 64 - _bits - 8;
      // Dart int is 64-bit signed. We must be careful: if shift >= 63 the
      // left shift could produce negative numbers via sign overflow. However,
      // since _bits >= 0 and we only enter this loop when _bits <= 56, the
      // shift is at least 0 and at most 64-0-8 = 56. Safe.
      _reg |= (_data[_pos] & 0xFF) << shift;
      _bits += 8;
    }
  }

  /// Read [nb] bits from the top of the register.
  ///
  /// This returns the most recently written bits first (highest stream
  /// positions first), mirroring the encoder's backward-write order.
  int readBits(int nb) {
    if (nb == 0) return 0;
    // Extract the top nb bits using a logical (unsigned) right shift.
    // >>> is the Dart 2.14+ logical right shift operator.
    final val = _reg >>> (64 - nb);
    // Shift the register left to consume those nb bits.
    _reg = nb == 64 ? 0 : (_reg << nb);
    _bits -= nb;
    if (_bits < 0) _bits = 0;
    if (_bits < 24) _reload();
    return val;
  }
}

// ─── FSE encode/decode helpers ────────────────────────────────────────────────

/// Encode one symbol into the backward bitstream, updating the FSE state.
///
/// The encoder maintains state in `[sz, 2*sz)`. To emit symbol [sym]:
/// 1. nb = `(state + deltaNb) >> 16` — how many state bits to flush.
/// 2. Write the low [nb] bits of [state] to [bw].
/// 3. New state = `st[(state >> nb) + deltaFs]`.
///
/// After all symbols, the final state minus sz is written as [accLog] bits
/// so the decoder can re-initialise.
void _fseEncodeSym(
  _RevBitWriter bw,
  List<int> state, // passed as a 1-element list so we can mutate in place
  int sym,
  List<_FseEe> ee,
  List<int> st,
) {
  final e = ee[sym];
  final nb = (state[0] + e.deltaNb) >> 16;
  bw.addBits(state[0], nb);
  final slotI = (state[0] >> nb) + e.deltaFs;
  state[0] = st[slotI];
}

/// Decode one symbol from the backward bitstream, updating the FSE state.
///
/// 1. Look up `de[state]` → `(sym, nb, base)`.
/// 2. New state = `base + read(nb bits)`.
int _fseDecodeSym(
  List<int> state, // 1-element mutable wrapper
  List<_FseDe> de,
  _RevBitReader br,
) {
  final e = de[state[0]];
  final sym = e.sym;
  state[0] = e.base + br.readBits(e.nb);
  return sym;
}

// ─── LL / ML code number helpers ──────────────────────────────────────────────

/// Map a literal length value to its LL code number (0..35).
///
/// Codes are in increasing baseline order. The correct code is the last entry
/// whose baseline is ≤ [ll].
int _llToCode(int ll) {
  var code = 0;
  for (var i = 0; i < _llCodes.length; i++) {
    if (_llCodes[i].$1 <= ll) {
      code = i;
    } else {
      break;
    }
  }
  return code;
}

/// Map a match length value to its ML code number (0..52).
int _mlToCode(int ml) {
  var code = 0;
  for (var i = 0; i < _mlCodes.length; i++) {
    if (_mlCodes[i].$1 <= ml) {
      code = i;
    } else {
      break;
    }
  }
  return code;
}

// ─── Sequence struct ──────────────────────────────────────────────────────────

/// One ZStd sequence: (literal_length, match_length, match_offset).
///
/// A sequence means: emit [ll] literal bytes from the literals section,
/// then copy [ml] bytes starting [off] positions back in the output buffer.
/// After all sequences, any remaining literals are appended.
class _Seq {
  final int ll;  // literal length (bytes to emit from the literals section)
  final int ml;  // match length (bytes to copy from output history)
  final int off; // match offset (1-indexed: 1 = most recently written byte)

  const _Seq(this.ll, this.ml, this.off);
}

/// Convert LZSS tokens into a ZStd sequences list and a flat literals buffer.
///
/// LZSS produces a stream of [lzss.Literal] and [lzss.Match] tokens. ZStd
/// groups the consecutive literals preceding each match into a single sequence
/// entry. Trailing literals (after the last match) go into the literals buffer
/// without a corresponding sequence.
({List<int> lits, List<_Seq> seqs}) _tokensToSeqs(List<lzss.Token> tokens) {
  final lits = <int>[];
  final seqs = <_Seq>[];
  var litRun = 0;

  for (final tok in tokens) {
    if (tok is lzss.Literal) {
      lits.add(tok.byte);
      litRun++;
    } else if (tok is lzss.Match) {
      seqs.add(_Seq(litRun, tok.length, tok.offset));
      litRun = 0;
    }
  }
  // Trailing literals remain in `lits` without a sequence entry.
  return (lits: lits, seqs: seqs);
}

// ─── Literals section encoding ────────────────────────────────────────────────
//
// ZStd literals can be Huffman-coded or raw. We always use Raw_Literals (type=0),
// which stores bytes verbatim with no Huffman table overhead. This is simpler
// and always correct; Huffman coding would reduce size further for typical data.
//
// Raw_Literals header format (RFC 8878 §3.1.1.2.1):
//   bits [1:0] = Literals_Block_Type = 00 (Raw)
//   bits [3:2] = Size_Format
//
//   Size_Format 00 or 10 → 1-byte header: size in bits [7:3]  (5 bits, 0..31)
//   Size_Format 01       → 2-byte header: size in bits [11:4] (12 bits, 0..4095)
//   Size_Format 11       → 3-byte header: size in bits [19:4] (20 bits, 0..~1MB)

List<int> _encodeLiteralsSection(List<int> lits) {
  final n = lits.length;
  final out = <int>[];

  if (n <= 31) {
    // 1-byte header: size_format=00, type=00 → header = (n << 3) | 0b000
    out.add((n << 3) & 0xFF);
  } else if (n <= 4095) {
    // 2-byte header: size_format=01, type=00 → low nibble = 0b0100
    final hdr = (n << 4) | 0x04;
    out.add(hdr & 0xFF);
    out.add((hdr >> 8) & 0xFF);
  } else {
    // 3-byte header: size_format=11, type=00 → low nibble = 0b1100
    final hdr = (n << 4) | 0x0C;
    out.add(hdr & 0xFF);
    out.add((hdr >> 8) & 0xFF);
    out.add((hdr >> 16) & 0xFF);
  }

  out.addAll(lits);
  return out;
}

/// Decode the literals section, returning `(literals, bytesConsumed)`.
///
/// [bytesConsumed] includes the header bytes plus the raw literal data.
({List<int> lits, int consumed}) _decodeLiteralsSection(Uint8List data, int offset) {
  if (offset >= data.length) {
    throw FormatException('empty literals section at offset $offset');
  }

  final b0 = data[offset];
  final ltype = b0 & 0x03; // bottom 2 bits = Literals_Block_Type

  if (ltype != 0) {
    // Only Raw_Literals (type=0) is supported. Our encoder always produces
    // type=0, so type != 0 means input from an incompatible encoder.
    throw FormatException(
      'unsupported literals type $ltype (only Raw=0 is supported)',
    );
  }

  final sizeFormat = (b0 >> 2) & 0x03; // bits [3:2] = Size_Format

  final int n;       // number of literal bytes
  final int headerBytes; // number of header bytes consumed

  switch (sizeFormat) {
    case 0:
    case 2:
      // 1-byte header: size in bits [7:3] (5 bits)
      n = b0 >> 3;
      headerBytes = 1;
    case 1:
      // 2-byte LE header: 12-bit size in bits [11:4]
      if (offset + 2 > data.length) {
        throw const FormatException('truncated literals header (2-byte)');
      }
      n = (b0 >> 4) | (data[offset + 1] << 4);
      headerBytes = 2;
    case 3:
      // 3-byte LE header: 20-bit size in bits [19:4]
      if (offset + 3 > data.length) {
        throw const FormatException('truncated literals header (3-byte)');
      }
      n = (b0 >> 4) | (data[offset + 1] << 4) | (data[offset + 2] << 12);
      headerBytes = 3;
    default:
      throw StateError('unreachable: sizeFormat is 2 bits');
  }

  final start = offset + headerBytes;
  final end = start + n;
  if (end > data.length) {
    throw FormatException(
      'literals data truncated: need $end bytes, have ${data.length}',
    );
  }

  return (lits: data.sublist(start, end), consumed: headerBytes + n);
}

// ─── Sequences section encoding ───────────────────────────────────────────────
//
// Layout of the sequences section:
//   [sequence_count: 1–3 bytes]
//   [symbol_compression_modes: 1 byte]   (0x00 = all Predefined)
//   [FSE bitstream: variable length]
//
// Symbol compression modes byte (RFC 8878 §3.1.1.3.2):
//   bits [7:6] = LL mode
//   bits [5:4] = OF mode
//   bits [3:2] = ML mode
//   bits [1:0] = reserved (0)
//   Mode 0 = Predefined, 1 = RLE, 2 = FSE_Compressed, 3 = Repeat.
//   We always write 0x00 (all Predefined).
//
// The FSE bitstream is a reversed bitstream (built with _RevBitWriter):
//   Sequences are encoded in REVERSE ORDER (last sequence first).
//   For each sequence (in reverse):
//     1. OF extra bits
//     2. ML extra bits
//     3. LL extra bits
//     4. FSE encode ML symbol  (symbol bits, written to backward stream)
//     5. FSE encode OF symbol
//     6. FSE encode LL symbol  (LL written last = at the top = decoded first)
//   After all sequences:
//     7. Flush final LL state (LL_ACC_LOG bits)
//     8. Flush final ML state (ML_ACC_LOG bits)
//     9. Flush final OF state (OF_ACC_LOG bits)
//     10. Call flush() to add sentinel and seal the byte stream.
//
// The decoder mirrors this exactly in reverse:
//   1. Read LL_ACC_LOG bits → initial state_ll
//   2. Read ML_ACC_LOG bits → initial state_ml
//   3. Read OF_ACC_LOG bits → initial state_of
//   4. For each sequence:
//        decode LL symbol (state transition)
//        decode OF symbol
//        decode ML symbol
//        read LL extra bits
//        read ML extra bits
//        read OF extra bits
//   5. Apply sequence to output buffer.

/// Encode the sequence count using the RFC 8878 variable-length format.
///
/// - count < 128: 1 byte (value as-is).
/// - 128 ≤ count < 32768: 2 bytes LE where byte0 has bit7 set.
///   We use a correct encoding: byte0 = (count >> 8) | 0x80, byte1 = count & 0xFF.
/// - count ≥ 32768: 3 bytes: [0xFF, low, high] where low:high = count - 0x7F00.
List<int> _encodeSeqCount(int count) {
  if (count == 0) {
    return [0];
  } else if (count < 128) {
    return [count];
  } else if (count < 32768) {
    // 2-byte encoding: set the high bit of byte0 to signal 2-byte form.
    // byte0 carries bits [14:8] of count, byte1 carries bits [7:0].
    // This avoids the "or 0x8000 as LE16" form which can produce byte0 < 128
    // for small-ish counts, making it ambiguous with the 1-byte form.
    final byte0 = (count >> 8) | 0x80;
    final byte1 = count & 0xFF;
    return [byte0, byte1];
  } else {
    // 3-byte encoding: first byte = 0xFF, then (count - 0x7F00) as LE u16.
    final r = count - 0x7F00;
    return [0xFF, r & 0xFF, (r >> 8) & 0xFF];
  }
}

/// Decode a sequence count, returning `(count, bytesConsumed)`.
({int count, int consumed}) _decodeSeqCount(Uint8List data, int offset) {
  if (offset >= data.length) {
    throw const FormatException('empty sequence count');
  }
  final b0 = data[offset];
  if (b0 < 128) {
    // 1-byte: value directly.
    return (count: b0, consumed: 1);
  } else if (b0 < 0xFF) {
    // 2-byte: bits [14:8] in b0 (minus high bit), bits [7:0] in b1.
    if (offset + 2 > data.length) {
      throw const FormatException('truncated sequence count (2-byte)');
    }
    final count = ((b0 & 0x7F) << 8) | data[offset + 1];
    return (count: count, consumed: 2);
  } else {
    // 3-byte: byte0=0xFF, then (count - 0x7F00) as LE u16.
    if (offset + 3 > data.length) {
      throw const FormatException('truncated sequence count (3-byte)');
    }
    final count = 0x7F00 + data[offset + 1] + (data[offset + 2] << 8);
    return (count: count, consumed: 3);
  }
}

/// Encode a list of sequences using predefined FSE tables.
///
/// Returns the raw bytes of the FSE bitstream (not including the sequence
/// count or modes byte; the caller appends those separately).
Uint8List _encodeSequencesSection(List<_Seq> seqs) {
  // Build encode tables from the predefined distributions.
  final (:ee, :st) = _buildEncodeSym(_llNorm, _llAccLog);
  final llEe = ee; final llSt = st;
  final (ee: mlEe, st: mlSt) = _buildEncodeSym(_mlNorm, _mlAccLog);
  final (ee: ofEe, st: ofSt) = _buildEncodeSym(_ofNorm, _ofAccLog);

  final szLl = 1 << _llAccLog;
  final szMl = 1 << _mlAccLog;
  final szOf = 1 << _ofAccLog;

  // FSE encoder states start at table_size (= sz). The valid encoder state
  // range is [sz, 2*sz). State - sz gives the decode-table index.
  final stateLl = [szLl];
  final stateMl = [szMl];
  final stateOf = [szOf];

  final bw = _RevBitWriter();

  // Encode sequences in reverse order.
  // The backward bitstream reverses the order so the decoder sees them forward.
  for (var i = seqs.length - 1; i >= 0; i--) {
    final seq = seqs[i];
    final llCode = _llToCode(seq.ll);
    final mlCode = _mlToCode(seq.ml);

    // Offset encoding (RFC 8878 §3.1.1.3.2.1):
    //   raw_offset = actual_offset + 3
    //   of_code    = floor(log2(raw_offset))
    //   of_extra   = raw_offset - (1 << of_code)
    //
    // The +3 bias keeps of_code > 0 for small offsets, avoiding degenerate
    // encoding (offset 0 and 1 are special "repeat offsets" in full ZStd,
    // but we don't use repeat-offset mode here, so the +3 keeps us clear).
    final rawOff = seq.off + 3;
    final ofCode = rawOff <= 1
        ? 0
        : (rawOff.bitLength - 1); // floor(log2(rawOff)) = bitLength - 1
    final ofExtra = rawOff - (1 << ofCode);

    // Write extra bits in OF, ML, LL order (they will be read back in LL, ML, OF order
    // by the decoder after the backward stream reversal).
    bw.addBits(ofExtra, ofCode);
    final mlExtra = seq.ml - _mlCodes[mlCode].$1;
    bw.addBits(mlExtra, _mlCodes[mlCode].$2);
    final llExtra = seq.ll - _llCodes[llCode].$1;
    bw.addBits(llExtra, _llCodes[llCode].$2);

    // FSE encode symbols in ML → OF → LL order.
    // Since the backward stream reverses write order, the decoder will read
    // them as LL → OF → ML — the specified decode order.
    _fseEncodeSym(bw, stateMl, mlCode, mlEe, mlSt);
    _fseEncodeSym(bw, stateOf, ofCode, ofEe, ofSt);
    _fseEncodeSym(bw, stateLl, llCode, llEe, llSt);
  }

  // Flush final states (written in OF, ML, LL order so decoder reads LL, ML, OF).
  bw.addBits(stateOf[0] - szOf, _ofAccLog);
  bw.addBits(stateMl[0] - szMl, _mlAccLog);
  bw.addBits(stateLl[0] - szLl, _llAccLog);
  bw.flush();

  return bw.finish();
}

// ─── Block-level compress ─────────────────────────────────────────────────────

/// Compress one block of data into ZStd compressed-block format.
///
/// Returns [null] if the compressed form is not smaller than the input.
/// In that case the caller should use a Raw block as a fallback.
Uint8List? _compressBlock(Uint8List block) {
  // Use LZSS to find LZ77 back-references.
  // Window = 32 KB (gives better ratio than the 4 KB LZSS default).
  // maxMatch = 255 (LZSS limitation).
  // minMatch = 3 (ZStd minimum match length).
  final tokens = lzss.encode(block, 32768, 255, 3);

  // Convert LZSS tokens to ZStd sequences + flat literals buffer.
  final (:lits, :seqs) = _tokensToSeqs(tokens);

  // If LZSS found no matches, a compressed block with 0 sequences still has
  // header overhead. Fall back to Raw.
  if (seqs.isEmpty) return null;

  final out = <int>[];

  // Encode the literals section (Raw_Literals format).
  out.addAll(_encodeLiteralsSection(lits));

  // Encode the sequence count, modes byte, and FSE bitstream.
  out.addAll(_encodeSeqCount(seqs.length));
  out.add(0x00); // Symbol_Compression_Modes = 0x00 = all Predefined

  final bitstream = _encodeSequencesSection(seqs);
  out.addAll(bitstream);

  // Only use the compressed form if it is actually smaller.
  if (out.length >= block.length) return null;
  return Uint8List.fromList(out);
}

/// Decompress one ZStd Compressed block.
///
/// Reads the literals section, sequences section, and applies the sequences
/// to [out] to reconstruct the original data.
void _decompressBlock(Uint8List data, List<int> out) {
  // ── Literals section ───────────────────────────────────────────────────────
  final (:lits, :consumed) = _decodeLiteralsSection(data, 0);
  var pos = consumed;

  // If there is no sequence data after the literals, the block is literals-only.
  if (pos >= data.length) {
    out.addAll(lits);
    return;
  }

  // ── Sequence count ─────────────────────────────────────────────────────────
  final (count: nSeqs, consumed: scBytes) = _decodeSeqCount(data, pos);
  pos += scBytes;

  if (nSeqs == 0) {
    out.addAll(lits);
    return;
  }

  // ── Symbol compression modes ───────────────────────────────────────────────
  if (pos >= data.length) {
    throw const FormatException('missing symbol compression modes byte');
  }
  final modesByte = data[pos];
  pos++;

  // Verify all modes are Predefined (0). Our encoder always writes 0x00.
  final llMode = (modesByte >> 6) & 3;
  final ofMode = (modesByte >> 4) & 3;
  final mlMode = (modesByte >> 2) & 3;
  if (llMode != 0 || ofMode != 0 || mlMode != 0) {
    throw FormatException(
      'unsupported FSE modes: LL=$llMode OF=$ofMode ML=$mlMode '
      '(only Predefined=0 supported)',
    );
  }

  // ── FSE bitstream ──────────────────────────────────────────────────────────
  final bitstreamData = data.sublist(pos);
  final br = _RevBitReader(bitstreamData);

  // Build predefined decode tables.
  final dtLl = _buildDecodeTable(_llNorm, _llAccLog);
  final dtMl = _buildDecodeTable(_mlNorm, _mlAccLog);
  final dtOf = _buildDecodeTable(_ofNorm, _ofAccLog);

  // Read initial FSE states.
  // The encoder wrote them in LL, ML, OF order (last written = at top of
  // the backward stream = read first by the decoder).
  final stateLl = [br.readBits(_llAccLog)];
  final stateMl = [br.readBits(_mlAccLog)];
  final stateOf = [br.readBits(_ofAccLog)];

  // Track our position in the literals buffer.
  var litPos = 0;

  // Decode and apply each sequence.
  for (var i = 0; i < nSeqs; i++) {
    // Decode symbols (FSE state transitions) in LL → OF → ML order.
    final llCode = _fseDecodeSym(stateLl, dtLl, br);
    final ofCode = _fseDecodeSym(stateOf, dtOf, br);
    final mlCode = _fseDecodeSym(stateMl, dtMl, br);

    // Validate codes are in range.
    if (llCode >= _llCodes.length) {
      throw FormatException('invalid LL code $llCode');
    }
    if (mlCode >= _mlCodes.length) {
      throw FormatException('invalid ML code $mlCode');
    }

    // Read extra bits to resolve the exact LL and ML values.
    final llInfo = _llCodes[llCode];
    final mlInfo = _mlCodes[mlCode];
    final ll = llInfo.$1 + br.readBits(llInfo.$2);
    final ml = mlInfo.$1 + br.readBits(mlInfo.$2);

    // Decode offset:
    //   of_raw = (1 << of_code) | extra_bits
    //   actual_offset = of_raw - 3  (reverses the +3 encoder bias)
    final ofRaw = (1 << ofCode) | br.readBits(ofCode);
    if (ofRaw < 3) {
      throw FormatException(
        'decoded offset underflow: of_raw=$ofRaw (expected >= 3)',
      );
    }
    final offset = ofRaw - 3;

    // Emit [ll] literal bytes from the literals buffer.
    final litEnd = litPos + ll;
    if (litEnd > lits.length) {
      throw FormatException(
        'literal run $ll overflows literals buffer '
        '(pos=$litPos, len=${lits.length})',
      );
    }
    if (out.length + ll > _maxOutput) {
      throw ArgumentError('decompressed size exceeds limit of $_maxOutput bytes');
    }
    out.addAll(lits.sublist(litPos, litEnd));
    litPos = litEnd;

    // Copy [ml] bytes from [offset] positions back in the output buffer.
    // offset=0 would reference past the end; minimum valid is 1.
    if (offset == 0 || offset > out.length) {
      throw FormatException(
        'bad match offset $offset (output length ${out.length})',
      );
    }
    if (out.length + ml > _maxOutput) {
      throw ArgumentError('decompressed size exceeds limit of $_maxOutput bytes');
    }
    final copyStart = out.length - offset;
    // Copy byte-by-byte to handle overlapping back-references correctly.
    // (The source may overlap the destination if ml > offset.)
    for (var j = 0; j < ml; j++) {
      out.add(out[copyStart + j]);
    }
  }

  // Append any trailing literals after the last sequence.
  if (out.length + lits.length - litPos > _maxOutput) {
    throw ArgumentError('decompressed size exceeds limit of $_maxOutput bytes');
  }
  out.addAll(lits.sublist(litPos));
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Compress [data] to ZStd format (RFC 8878).
///
/// The output is a valid ZStd frame that can be decompressed by the `zstd`
/// command-line tool or any conforming ZStd implementation.
///
/// The frame uses:
/// - 8-byte Frame_Content_Size (for easy pre-allocation by decoders).
/// - Single_Segment mode (no window descriptor).
/// - Per-block fallback: RLE if all bytes match, Compressed if beneficial,
///   Raw otherwise.
///
/// Example:
/// ```dart
/// import 'dart:typed_data';
/// import 'package:coding_adventures_zstd/coding_adventures_zstd.dart';
///
/// final compressed = compress(Uint8List.fromList([65, 65, 65])); // b'AAA'
/// ```
Uint8List compress(Uint8List data) {
  final out = <int>[];

  // ── ZStd frame header ───────────────────────────────────────────────────────
  // Magic number (4 bytes, little-endian).
  final bd = ByteData(4);
  bd.setUint32(0, _magic, Endian.little);
  out.addAll(bd.buffer.asUint8List());

  // Frame Header Descriptor (FHD = 1 byte):
  //   bits [7:6] = FCS_Field_Size = 11 → 8-byte Frame_Content_Size
  //   bit  [5]   = Single_Segment_Flag = 1 → no Window_Descriptor follows
  //   bit  [4]   = Content_Checksum_Flag = 0
  //   bits [3:2] = reserved = 0
  //   bits [1:0] = Dict_ID_Flag = 0
  //   = 0b1110_0000 = 0xE0
  out.add(0xE0);

  // Frame_Content_Size (8 bytes, little-endian) — the uncompressed byte count.
  final fcs = ByteData(8);
  fcs.setUint64(0, data.length, Endian.little);
  out.addAll(fcs.buffer.asUint8List());

  // ── Blocks ──────────────────────────────────────────────────────────────────
  // Special case: empty input → emit one empty Raw block.
  // A Raw block with size=0 is the canonical empty frame body.
  if (data.isEmpty) {
    // 3-byte block header: Last=1, Type=Raw(00), Size=0
    //   = 0b0000_0001 = 0x01, 0x00, 0x00
    out.addAll([0x01, 0x00, 0x00]);
    return Uint8List.fromList(out);
  }

  // Split data into blocks of at most _maxBlockSize bytes.
  var offset = 0;
  while (offset < data.length) {
    final end = (offset + _maxBlockSize).clamp(0, data.length);
    final block = data.sublist(offset, end);
    final isLast = end == data.length;
    final lastBit = isLast ? 1 : 0;

    // ── Try RLE block ─────────────────────────────────────────────────────────
    // If every byte in the block is the same value, an RLE block encodes the
    // entire block as (3-byte header) + (1 payload byte) = 4 bytes total.
    if (block.isNotEmpty && _allSameByte(block)) {
      // Block header: Last=[lastBit], Type=RLE(01), Size=block.length
      //   = (block.length << 3) | (0b01 << 1) | lastBit
      final hdr = (block.length << 3) | (0x01 << 1) | lastBit;
      out.add(hdr & 0xFF);
      out.add((hdr >> 8) & 0xFF);
      out.add((hdr >> 16) & 0xFF);
      out.add(block[0]); // single payload byte
    } else {
      // ── Try Compressed block ─────────────────────────────────────────────────
      final compressed = _compressBlock(block);
      if (compressed != null) {
        final hdr = (compressed.length << 3) | (0x02 << 1) | lastBit;
        out.add(hdr & 0xFF);
        out.add((hdr >> 8) & 0xFF);
        out.add((hdr >> 16) & 0xFF);
        out.addAll(compressed);
      } else {
        // ── Raw block (fallback) ──────────────────────────────────────────────
        final hdr = (block.length << 3) | (0x00 << 1) | lastBit;
        out.add(hdr & 0xFF);
        out.add((hdr >> 8) & 0xFF);
        out.add((hdr >> 16) & 0xFF);
        out.addAll(block);
      }
    }

    offset = end;
  }

  return Uint8List.fromList(out);
}

/// Decompress a ZStd frame, returning the original data.
///
/// Accepts any valid ZStd frame containing:
/// - Raw, RLE, or Compressed blocks.
/// - Predefined FSE modes only (no per-frame FSE table descriptions).
/// - Any combination of Single_Segment and multi-segment layouts.
///
/// Throws [FormatException] if the input is malformed, truncated, or uses
/// unsupported features (non-predefined FSE tables, Huffman literals, or
/// reserved block types).
///
/// Example:
/// ```dart
/// import 'dart:typed_data';
/// import 'package:coding_adventures_zstd/coding_adventures_zstd.dart';
///
/// final original = Uint8List.fromList([104, 101, 108, 108, 111]); // b'hello'
/// final roundTripped = decompress(compress(original));
/// ```
Uint8List decompress(Uint8List data) {
  if (data.length < 5) {
    throw const FormatException('ZStd frame too short');
  }

  final view = ByteData.view(data.buffer, data.offsetInBytes, data.lengthInBytes);

  // ── Validate magic ──────────────────────────────────────────────────────────
  final magic = view.getUint32(0, Endian.little);
  if (magic != _magic) {
    throw FormatException(
      'bad ZStd magic: 0x${magic.toRadixString(16).padLeft(8, '0')} '
      '(expected 0x${_magic.toRadixString(16).padLeft(8, '0')})',
    );
  }

  var pos = 4;

  // ── Frame Header Descriptor ─────────────────────────────────────────────────
  final fhd = data[pos];
  pos++;

  // FCS_Field_Size: bits [7:6] of FHD controls how many bytes follow for FCS.
  //   00 → 0 bytes (unless Single_Segment=1, then 1 byte)
  //   01 → 2 bytes (value + 256)
  //   10 → 4 bytes
  //   11 → 8 bytes
  final fcsFlag = (fhd >> 6) & 3;

  // Single_Segment_Flag: bit 5. When set, no Window_Descriptor follows.
  final singleSeg = (fhd >> 5) & 1;

  // Dict_ID_Flag: bits [1:0]. Controls how many dict ID bytes follow.
  final dictFlag = fhd & 3;

  // ── Window Descriptor ───────────────────────────────────────────────────────
  // Present only when Single_Segment_Flag = 0. We skip it; we don't enforce
  // window size limits in this educational implementation.
  if (singleSeg == 0) {
    pos++; // skip Window_Descriptor byte
  }

  // ── Dict ID ─────────────────────────────────────────────────────────────────
  // Skip the dict ID bytes. We do not support custom dictionaries.
  const dictIdBytes = [0, 1, 2, 4];
  pos += dictIdBytes[dictFlag];
  if (pos > data.length) {
    throw ArgumentError('zstd: frame header truncated (dict ID field)');
  }

  // ── Frame Content Size ───────────────────────────────────────────────────────
  // Read but do not validate; we trust the block data to be consistent.
  final int fcsBytes;
  switch (fcsFlag) {
    case 0:
      fcsBytes = singleSeg == 1 ? 1 : 0;
    case 1:
      fcsBytes = 2;
    case 2:
      fcsBytes = 4;
    case 3:
      fcsBytes = 8;
    default:
      fcsBytes = 0;
  }
  pos += fcsBytes; // skip FCS
  if (pos > data.length) {
    throw ArgumentError('zstd: frame header truncated (FCS field)');
  }

  // ── Blocks ──────────────────────────────────────────────────────────────────
  final out = <int>[];

  for (;;) {
    // Each block begins with a 3-byte header (24 bits, little-endian).
    if (pos + 3 > data.length) {
      throw const FormatException('truncated block header');
    }

    // Reconstruct the 24-bit block header from 3 bytes (LE).
    final hdr = data[pos] | (data[pos + 1] << 8) | (data[pos + 2] << 16);
    pos += 3;

    final isLast = (hdr & 1) != 0;         // bit 0 = Last_Block flag
    final btype = (hdr >> 1) & 3;           // bits [2:1] = Block_Type
    final bsize = hdr >> 3;                  // bits [23:3] = Block_Size

    switch (btype) {
      case 0:
        // Raw block: [bsize] bytes of verbatim content follow.
        if (pos + bsize > data.length) {
          throw FormatException(
            'raw block truncated: need $bsize bytes at pos $pos',
          );
        }
        if (out.length + bsize > _maxOutput) {
          throw FormatException(
            'decompressed size exceeds limit of $_maxOutput bytes',
          );
        }
        out.addAll(data.sublist(pos, pos + bsize));
        pos += bsize;

      case 1:
        // RLE block: one byte follows, repeated [bsize] times in the output.
        if (pos >= data.length) {
          throw const FormatException('RLE block missing payload byte');
        }
        if (out.length + bsize > _maxOutput) {
          throw FormatException(
            'decompressed size exceeds limit of $_maxOutput bytes',
          );
        }
        final byte = data[pos];
        pos++;
        for (var i = 0; i < bsize; i++) {
          out.add(byte);
        }

      case 2:
        // Compressed block: [bsize] bytes of ZStd compressed content.
        if (pos + bsize > data.length) {
          throw FormatException(
            'compressed block truncated: need $bsize bytes at pos $pos',
          );
        }
        final blockData = data.sublist(pos, pos + bsize);
        pos += bsize;
        _decompressBlock(blockData, out);

      case 3:
        throw const FormatException('reserved block type 3');

      default:
        throw StateError('unreachable: block type is 2 bits');
    }

    if (isLast) break;
  }

  return Uint8List.fromList(out);
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Returns true if every byte in [block] is the same value.
///
/// Used to detect RLE opportunities: a block that is all 0x41 compresses
/// to a 4-byte RLE block instead of a (much larger) raw or compressed block.
bool _allSameByte(Uint8List block) {
  if (block.isEmpty) return true;
  final first = block[0];
  for (var i = 1; i < block.length; i++) {
    if (block[i] != first) return false;
  }
  return true;
}
