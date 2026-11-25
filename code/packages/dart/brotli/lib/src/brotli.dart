/// Brotli lossless compression (CMP06): context-dependent literal
/// trees, insert-and-copy commands, and a 65535-byte sliding window.
///
/// Encoding order per command: `[ICC code][insert_extra
/// bits][copy_extra bits][literals...][dist code][dist_extra bits]`,
/// then the ICC-63 sentinel. See the reference crate docs for the
/// full wire format.
///
/// ## Lane hardenings
///
/// Two corrupt-output cases the reference mishandles fail closed
/// here instead: a literal bucket with all 256 symbols (the 1-byte
/// table count would truncate to 0 — proven on random 2KB+ inputs
/// against the reference) throws [BrotliTooManyLiteralSymbols],
/// and inputs past the 4 GiB header field throw
/// [BrotliInputTooLarge] (the reference truncates silently).
library;

import 'dart:typed_data';

import 'package:coding_adventures_huffman_tree/huffman_tree.dart';

import 'errors.dart';

/// One Insert-Copy Code table row: insert/copy bases and extra-bit
/// counts. Actual lengths add LSB-first extra bits to the bases;
/// code 63 is the end-of-data sentinel.
class _IccEntry {
  final int insertBase;
  final int insertExtra;
  final int copyBase;
  final int copyExtra;

  const _IccEntry(
      this.insertBase, this.insertExtra, this.copyBase, this.copyExtra);
}

const List<_IccEntry> _iccTable = [
  // Codes 0–15: insert=0, copy varies.
  _IccEntry(0, 0, 4, 0), // 0
  _IccEntry(0, 0, 5, 0), // 1
  _IccEntry(0, 0, 6, 0), // 2
  _IccEntry(0, 0, 8, 1), // 3
  _IccEntry(0, 0, 10, 1), // 4
  _IccEntry(0, 0, 14, 2), // 5
  _IccEntry(0, 0, 18, 2), // 6
  _IccEntry(0, 0, 26, 3), // 7
  _IccEntry(0, 0, 34, 3), // 8
  _IccEntry(0, 0, 50, 4), // 9
  _IccEntry(0, 0, 66, 4), // 10
  _IccEntry(0, 0, 98, 5), // 11
  _IccEntry(0, 0, 130, 5), // 12
  _IccEntry(0, 0, 194, 6), // 13
  _IccEntry(0, 0, 258, 7), // 14
  _IccEntry(0, 0, 514, 8), // 15
  // Codes 16–23: insert=1.
  _IccEntry(1, 0, 4, 0), // 16
  _IccEntry(1, 0, 5, 0), // 17
  _IccEntry(1, 0, 6, 0), // 18
  _IccEntry(1, 0, 8, 1), // 19
  _IccEntry(1, 0, 10, 1), // 20
  _IccEntry(1, 0, 14, 2), // 21
  _IccEntry(1, 0, 18, 2), // 22
  _IccEntry(1, 0, 26, 3), // 23
  // Codes 24–31: insert=2.
  _IccEntry(2, 0, 4, 0), // 24
  _IccEntry(2, 0, 5, 0), // 25
  _IccEntry(2, 0, 6, 0), // 26
  _IccEntry(2, 0, 8, 1), // 27
  _IccEntry(2, 0, 10, 1), // 28
  _IccEntry(2, 0, 14, 2), // 29
  _IccEntry(2, 0, 18, 2), // 30
  _IccEntry(2, 0, 26, 3), // 31
  // Codes 32–39: insert=3+1bit.
  _IccEntry(3, 1, 4, 0), // 32
  _IccEntry(3, 1, 5, 0), // 33
  _IccEntry(3, 1, 6, 0), // 34
  _IccEntry(3, 1, 8, 1), // 35
  _IccEntry(3, 1, 10, 1), // 36
  _IccEntry(3, 1, 14, 2), // 37
  _IccEntry(3, 1, 18, 2), // 38
  _IccEntry(3, 1, 26, 3), // 39
  // Codes 40–47: insert=5+2bits.
  _IccEntry(5, 2, 4, 0), // 40
  _IccEntry(5, 2, 5, 0), // 41
  _IccEntry(5, 2, 6, 0), // 42
  _IccEntry(5, 2, 8, 1), // 43
  _IccEntry(5, 2, 10, 1), // 44
  _IccEntry(5, 2, 14, 2), // 45
  _IccEntry(5, 2, 18, 2), // 46
  _IccEntry(5, 2, 26, 3), // 47
  // Codes 48–55: insert=9+3bits.
  _IccEntry(9, 3, 4, 0), // 48
  _IccEntry(9, 3, 5, 0), // 49
  _IccEntry(9, 3, 6, 0), // 50
  _IccEntry(9, 3, 8, 1), // 51
  _IccEntry(9, 3, 10, 1), // 52
  _IccEntry(9, 3, 14, 2), // 53
  _IccEntry(9, 3, 18, 2), // 54
  _IccEntry(9, 3, 26, 3), // 55
  // Codes 56–62: insert=17+4bits.
  _IccEntry(17, 4, 4, 0), // 56
  _IccEntry(17, 4, 5, 0), // 57
  _IccEntry(17, 4, 6, 0), // 58
  _IccEntry(17, 4, 8, 1), // 59
  _IccEntry(17, 4, 10, 1), // 60
  _IccEntry(17, 4, 14, 2), // 61
  _IccEntry(17, 4, 18, 2), // 62
  // Code 63: end-of-data sentinel.
  _IccEntry(0, 0, 0, 0), // 63
];

class _DistEntry {
  final int base;
  final int extraBits;

  const _DistEntry(this.base, this.extraBits);
}

const List<_DistEntry> _distTable = [
  _DistEntry(1, 0), // 0
  _DistEntry(2, 0), // 1
  _DistEntry(3, 0), // 2
  _DistEntry(4, 0), // 3
  _DistEntry(5, 1), // 4
  _DistEntry(7, 1), // 5
  _DistEntry(9, 2), // 6
  _DistEntry(13, 2), // 7
  _DistEntry(17, 3), // 8
  _DistEntry(25, 3), // 9
  _DistEntry(33, 4), // 10
  _DistEntry(49, 4), // 11
  _DistEntry(65, 5), // 12
  _DistEntry(97, 5), // 13
  _DistEntry(129, 6), // 14
  _DistEntry(193, 6), // 15
  _DistEntry(257, 7), // 16
  _DistEntry(385, 7), // 17
  _DistEntry(513, 8), // 18
  _DistEntry(769, 8), // 19
  _DistEntry(1025, 9), // 20
  _DistEntry(1537, 9), // 21
  _DistEntry(2049, 10), // 22
  _DistEntry(3073, 10), // 23
  _DistEntry(4097, 11), // 24
  _DistEntry(6145, 11), // 25
  _DistEntry(8193, 12), // 26
  _DistEntry(12289, 12), // 27
  _DistEntry(16385, 13), // 28
  _DistEntry(24577, 13), // 29
  _DistEntry(32769, 14), // 30
  _DistEntry(49153, 14), // 31
];

/// The distance code (0–31) for a back-reference offset (1–65535).
int _distCodeFor(int offset) {
  for (var i = 0; i < _distTable.length; i++) {
    final e = _distTable[i];
    if (offset <= e.base + (1 << e.extraBits) - 1) {
      return i;
    }
  }
  return 31;
}

/// The ICC code with the fewest total extra bits covering both
/// lengths, or null when no single code covers them. Ties keep the
/// lowest code, mirroring the reference's strict comparison.
int? _findIcc(int insertLen, int copyLen) {
  int? best;
  var bestExtra = 1 << 30;
  for (var icc = 0; icc < 63; icc++) {
    final e = _iccTable[icc];
    final maxInsert = e.insertBase + (1 << e.insertExtra) - 1;
    final maxCopy = e.copyBase + (1 << e.copyExtra) - 1;
    if (insertLen >= e.insertBase &&
        insertLen <= maxInsert &&
        copyLen >= e.copyBase &&
        copyLen <= maxCopy) {
      final totalExtra = e.insertExtra + e.copyExtra;
      if (totalExtra < bestExtra) {
        best = icc;
        bestExtra = totalExtra;
      }
    }
  }
  return best;
}

/// The maximum insert length representable alongside [copyLen].
int _maxInsertForCopy(int copyLen) {
  var best = 0;
  for (var icc = 0; icc < 63; icc++) {
    final e = _iccTable[icc];
    final maxCopy = e.copyBase + (1 << e.copyExtra) - 1;
    if (copyLen >= e.copyBase && copyLen <= maxCopy) {
      final maxIns = e.insertBase + (1 << e.insertExtra) - 1;
      if (maxIns > best) {
        best = maxIns;
      }
    }
  }
  return best;
}

/// Clamps a copy length to the largest ICC-representable value.
/// The table has gaps (7, 12–13, …); reducing wastes bytes but
/// keeps encoding correct. Falls back to 4 (always valid).
int _clampCopyLen(int len) {
  var best = 0;
  for (var icc = 0; icc < 63; icc++) {
    final e = _iccTable[icc];
    if (e.copyBase > len) {
      continue;
    }
    final maxCopy = e.copyBase + (1 << e.copyExtra) - 1;
    final candidate = len < maxCopy ? len : maxCopy;
    if (candidate > best) {
      best = candidate;
    }
  }
  return best == 0 ? 4 : best;
}

/// The literal context bucket (0–3) for the last output byte:
/// lowercase→3, uppercase→2, digit→1, everything else (or
/// stream start)→0.
int literalContext(int? lastByte) {
  if (lastByte == null) {
    return 0;
  }
  if (lastByte >= 0x61 && lastByte <= 0x7a) {
    return 3;
  }
  if (lastByte >= 0x41 && lastByte <= 0x5a) {
    return 2;
  }
  if (lastByte >= 0x30 && lastByte <= 0x39) {
    return 1;
  }
  return 0;
}

const int _maxMatch = 769;
const int _minMatch = 4;
const int _window = 65535;

/// The longest match at [pos] within the sliding window as
/// (distance, length), or (0, 0). Scans backward so nearer matches
/// (cheaper distance codes) win ties; allows overlap.
(int, int) _findLongestMatch(List<int> data, int pos) {
  final windowStart = pos - _window < 0 ? 0 : pos - _window;
  var maxLen = data.length - pos;
  if (maxLen > _maxMatch) {
    maxLen = _maxMatch;
  }
  if (maxLen < _minMatch || pos == 0) {
    return (0, 0);
  }

  var bestLen = 0;
  var bestDist = 0;
  var search = pos - 1;
  while (true) {
    var len = 0;
    while (len < maxLen &&
        pos + len < data.length &&
        data[search + len] == data[pos + len]) {
      len += 1;
    }
    if (len >= _minMatch && len > bestLen) {
      bestLen = len;
      bestDist = pos - search;
      if (bestLen == _maxMatch) {
        break;
      }
    }
    if (search == windowStart) {
      break;
    }
    search -= 1;
  }

  return bestLen >= _minMatch ? (bestDist, bestLen) : (0, 0);
}

/// A command from the LZ pass. `copyDistance == 0` means "no
/// actual copy" (distance symbol 32 tells the decoder to skip).
class _Command {
  final List<int> literals;
  final int copyLength;
  final int copyDistance;

  _Command(this.literals, this.copyLength, this.copyDistance);
}

/// Accumulates bits into bytes, LSB-first.
class _BitBuilder {
  int _buf = 0;
  int _bitPos = 0;
  final List<int> out = [];

  /// Writes a Huffman code string bit by bit, LSB-first.
  void writeBitString(String s) {
    for (var i = 0; i < s.length; i++) {
      if (s.codeUnitAt(i) == 0x31) {
        _buf |= 1 << _bitPos;
      }
      _bitPos += 1;
      _drainFullBytes();
    }
  }

  /// Writes [n] raw bits of [val], bit 0 first.
  void writeRawBitsLsb(int val, int n) {
    for (var i = 0; i < n; i++) {
      if ((val >> i) & 1 == 1) {
        _buf |= 1 << _bitPos;
      }
      _bitPos += 1;
      _drainFullBytes();
    }
  }

  void _drainFullBytes() {
    while (_bitPos >= 8) {
      out.add(_buf & 0xff);
      _buf >>= 8;
      _bitPos -= 8;
    }
  }

  Uint8List finish() {
    if (_bitPos > 0) {
      out.add(_buf & 0xff);
    }
    return Uint8List.fromList(out);
  }
}

/// Expands bytes to 0/1 values, LSB-first.
List<int> _unpackBits(List<int> data) {
  final bits = <int>[];
  for (final byte in data) {
    for (var i = 0; i < 8; i++) {
      bits.add((byte >> i) & 1);
    }
  }
  return bits;
}

/// Reconstructs canonical codes from (symbol, length) pairs:
/// `code[0] = 0`, `code[i] = (code[i-1] + 1) << (len[i] - len[i-1])`.
/// A single pair gets code "0".
Map<int, String> _buildCanonicalCodes(List<(int, int)> pairs) {
  final result = <int, String>{};
  if (pairs.isEmpty) {
    return result;
  }
  if (pairs.length == 1) {
    result[pairs[0].$1] = '0';
    return result;
  }
  var code = 0;
  var prevLen = pairs[0].$2;
  for (final (symbol, codeLen) in pairs) {
    if (codeLen > prevLen) {
      code <<= codeLen - prevLen;
    }
    result[symbol] = code.toRadixString(2).padLeft(codeLen, '0');
    code += 1;
    prevLen = codeLen;
  }
  return result;
}

/// Inverts a code map: code-string → symbol, for prefix decoding.
Map<String, int> _reverseCodeMap(Map<int, String> m) =>
    {for (final e in m.entries) e.value: e.key};

/// Flushes [pending] literals as "no-copy" commands in ≤32-byte
/// chunks (`max_insert_for_copy(4) = 32`).
void _flushLiteralsAsNoCopy(List<int> pending, List<_Command> out) {
  var remaining = List<int>.of(pending);
  while (remaining.isNotEmpty) {
    final chunkSize = remaining.length < 32 ? remaining.length : 32;
    out.add(_Command(remaining.sublist(0, chunkSize), 4, 0));
    remaining = remaining.sublist(chunkSize);
  }
}

/// Compresses [data] to CMP06 wire-format bytes.
///
/// Pass 1 matches LZ back-references (bundling pending literals
/// into copy commands, flushing overflow and trailing literals as
/// no-copy blocks); pass 2 tallies frequencies, builds canonical
/// Huffman trees, and encodes the stream.
///
/// Throws [BrotliTooManyLiteralSymbols] when a context bucket
/// holds all 256 symbols (unrepresentable — see the library docs),
/// and [BrotliInputTooLarge] past the 4 GiB header field.
Uint8List compress(List<int> data) {
  final originalLength = data.length;

  if (originalLength == 0) {
    return Uint8List.fromList([0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 63, 1, 0]);
  }
  if (originalLength > 0xffffffff) {
    throw BrotliInputTooLarge(originalLength);
  }

  // Pass 1: LZ matching → final_commands.
  final finalCommands = <_Command>[];
  var insertBuf = <int>[];
  var pos = 0;
  while (pos < data.length) {
    final (dist, len) = _findLongestMatch(data, pos);
    if (len >= _minMatch) {
      final clamped = _clampCopyLen(len);
      final pending = insertBuf;
      insertBuf = [];
      final maxIns = _maxInsertForCopy(clamped);
      if (pending.length <= maxIns) {
        finalCommands.add(_Command(pending, clamped, dist));
      } else {
        final splitPoint = pending.length - maxIns;
        _flushLiteralsAsNoCopy(pending.sublist(0, splitPoint), finalCommands);
        finalCommands.add(_Command(pending.sublist(splitPoint), clamped, dist));
      }
      pos += clamped;
    } else {
      insertBuf.add(data[pos]);
      pos += 1;
    }
  }
  if (insertBuf.isNotEmpty) {
    _flushLiteralsAsNoCopy(insertBuf, finalCommands);
  }

  // Debug-only structural check, mirroring the reference.
  assert(() {
    var histLen = 0;
    for (final cmd in finalCommands) {
      histLen += cmd.literals.length;
      assert(cmd.copyLength >= 4);
      if (cmd.copyDistance > 0) {
        assert(histLen >= cmd.copyDistance);
        histLen += cmd.copyLength;
      }
    }
    return true;
  }());

  // Pass 2a: tally frequencies, simulating output context.
  final litFreq = List.generate(4, (_) => <int, int>{});
  final iccFreq = <int, int>{};
  final distFreq = <int, int>{};
  final history = <int>[];
  for (final cmd in finalCommands) {
    for (final byte in cmd.literals) {
      final ctx = literalContext(history.isEmpty ? null : history.last);
      litFreq[ctx][byte] = (litFreq[ctx][byte] ?? 0) + 1;
      history.add(byte);
    }
    final found = _findIcc(cmd.literals.length, cmd.copyLength);
    // _findIcc provably never returns null here (covering tiers
    // nest, insert ranges chain contiguously from 0, pass 1 caps
    // pending literals at the maximum), so unwrap loudly rather
    // than silently degrading like the reference fallback would.
    final icc = found!;
    iccFreq[icc] = (iccFreq[icc] ?? 0) + 1;
    if (cmd.copyDistance == 0) {
      distFreq[32] = (distFreq[32] ?? 0) + 1;
    } else {
      final dc = _distCodeFor(cmd.copyDistance);
      distFreq[dc] = (distFreq[dc] ?? 0) + 1;
      final start = history.length - cmd.copyDistance;
      for (var i = 0; i < cmd.copyLength; i++) {
        history.add(history[start + i]);
      }
    }
  }
  iccFreq[63] = (iccFreq[63] ?? 0) + 1;

  // Pass 2b: build Huffman trees.
  Map<int, String> buildTable(Map<int, int> freq) {
    final weights = [for (final e in freq.entries) (e.key, e.value)];
    return HuffmanTree.build(weights).canonicalCodeTable();
  }

  final iccCodeTable = buildTable(iccFreq);
  final distCodeTable = buildTable(distFreq);
  final litCodeTables = <Map<int, String>>[
    for (var ctx = 0; ctx < 4; ctx++)
      litFreq[ctx].isEmpty ? <int, String>{} : buildTable(litFreq[ctx]),
  ];
  for (var ctx = 0; ctx < 4; ctx++) {
    if (litCodeTables[ctx].length > 255) {
      throw BrotliTooManyLiteralSymbols(ctx);
    }
  }

  // Pass 2c: encode the bit stream.
  final bb = _BitBuilder();
  final history2 = <int>[];
  for (final cmd in finalCommands) {
    final insLen = cmd.literals.length;
    final found = _findIcc(insLen, cmd.copyLength);
    // _findIcc provably never returns null here (covering tiers
    // nest, insert ranges chain contiguously from 0, pass 1 caps
    // pending literals at the maximum), so unwrap loudly rather
    // than silently degrading like the reference fallback would.
    final icc = found!;
    final e = _iccTable[icc];
    bb.writeBitString(iccCodeTable[icc]!);
    if (e.insertExtra > 0) {
      bb.writeRawBitsLsb(insLen - e.insertBase, e.insertExtra);
    }
    if (e.copyExtra > 0) {
      bb.writeRawBitsLsb(cmd.copyLength - e.copyBase, e.copyExtra);
    }
    for (final byte in cmd.literals) {
      final ctx = literalContext(history2.isEmpty ? null : history2.last);
      bb.writeBitString(litCodeTables[ctx][byte]!);
      history2.add(byte);
    }
    if (cmd.copyDistance == 0) {
      bb.writeBitString(distCodeTable[32]!);
    } else {
      final dc = _distCodeFor(cmd.copyDistance);
      final dEntry = _distTable[dc];
      bb.writeBitString(distCodeTable[dc]!);
      if (dEntry.extraBits > 0) {
        bb.writeRawBitsLsb(cmd.copyDistance - dEntry.base, dEntry.extraBits);
      }
      final start = history2.length - cmd.copyDistance;
      for (var i = 0; i < cmd.copyLength; i++) {
        history2.add(history2[start + i]);
      }
    }
  }
  bb.writeBitString(iccCodeTable[63]!);
  final packedBits = bb.finish();

  // Assemble the wire format.
  List<(int, int)> sortedPairs(Map<int, String> map) {
    final v = [for (final e in map.entries) (e.key, e.value.length)];
    v.sort(
        (a, b) => a.$2 != b.$2 ? a.$2.compareTo(b.$2) : a.$1.compareTo(b.$1));
    return v;
  }

  final iccPairs = sortedPairs(iccCodeTable);
  final distPairs = sortedPairs(distCodeTable);
  final litPairs = [
    for (var ctx = 0; ctx < 4; ctx++) sortedPairs(litCodeTables[ctx])
  ];

  final out = <int>[
    (originalLength >> 24) & 0xff,
    (originalLength >> 16) & 0xff,
    (originalLength >> 8) & 0xff,
    originalLength & 0xff,
    iccPairs.length,
    distPairs.length,
    litPairs[0].length,
    litPairs[1].length,
    litPairs[2].length,
    litPairs[3].length,
  ];
  for (final (sym, len) in [...iccPairs, ...distPairs]) {
    out.add(sym);
    out.add(len);
  }
  for (final pairs in litPairs) {
    for (final (sym, len) in pairs) {
      out.add((sym >> 8) & 0xff);
      out.add(sym & 0xff);
      out.add(len);
    }
  }
  out.addAll(packedBits);
  return Uint8List.fromList(out);
}

/// Bit-stream reader state shared by the decompress closures.
class _BitReader {
  final List<int> bits;
  var pos = 0;

  _BitReader(this.bits);

  /// Reads [n] bits LSB-first.
  int readBits(int n) {
    var val = 0;
    for (var i = 0; i < n; i++) {
      if (pos >= bits.length) {
        throw const BrotliBitStreamExhausted();
      }
      val |= bits[pos] << i;
      pos += 1;
    }
    return val;
  }

  /// Decodes one Huffman symbol by accumulating prefix bits.
  int nextHuffman(Map<String, int> rev) {
    final acc = StringBuffer();
    while (true) {
      if (pos >= bits.length) {
        throw const BrotliHuffmanExhausted();
      }
      acc.write(bits[pos] == 1 ? '1' : '0');
      pos += 1;
      final sym = rev[acc.toString()];
      if (sym != null) {
        return sym;
      }
      if (acc.length > 32) {
        throw const BrotliHuffmanOverrun();
      }
    }
  }
}

/// Decompresses CMP06 wire-format [data] to the original bytes.
///
/// Stops at `original_length` bytes or the ICC-63 sentinel,
/// whichever comes first (early stop drops the encoder's synthetic
/// trailing copies). Never sizes allocations from the untrusted
/// header length.
///
/// Throws [BrotliError] on malformed input.
Uint8List decompress(List<int> data) {
  if (data.length < 10) {
    throw BrotliDataTooShort(data.length);
  }
  final originalLength =
      (data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3];
  if (originalLength == 0) {
    return Uint8List(0);
  }

  final iccEntryCount = data[4];
  final distEntryCount = data[5];
  final ctxEntryCounts = [data[6], data[7], data[8], data[9]];
  var off = 10;

  final iccLengths = <(int, int)>[];
  for (var i = 0; i < iccEntryCount; i++) {
    if (off + 2 > data.length) {
      throw const BrotliIccTableTruncated();
    }
    iccLengths.add((data[off], data[off + 1]));
    off += 2;
  }
  final distLengths = <(int, int)>[];
  for (var i = 0; i < distEntryCount; i++) {
    if (off + 2 > data.length) {
      throw const BrotliDistTableTruncated();
    }
    distLengths.add((data[off], data[off + 1]));
    off += 2;
  }
  final litLengths = <List<(int, int)>>[[], [], [], []];
  for (var ctx = 0; ctx < 4; ctx++) {
    for (var i = 0; i < ctxEntryCounts[ctx]; i++) {
      if (off + 3 > data.length) {
        throw BrotliLitTableTruncated(ctx);
      }
      litLengths[ctx].add(((data[off] << 8) | data[off + 1], data[off + 2]));
      off += 3;
    }
  }

  for (final (sym, _) in iccLengths) {
    if (sym >= 64) {
      throw BrotliInvalidTableSymbol('ICC', sym);
    }
  }
  for (final (sym, _) in distLengths) {
    if (sym > 32) {
      throw BrotliInvalidTableSymbol('dist', sym);
    }
  }
  for (var ctx = 0; ctx < 4; ctx++) {
    for (final (sym, _) in litLengths[ctx]) {
      if (sym >= 256) {
        throw BrotliInvalidTableSymbol('lit$ctx', sym);
      }
    }
  }

  final iccRev = _reverseCodeMap(_buildCanonicalCodes(iccLengths));
  final distRev = _reverseCodeMap(_buildCanonicalCodes(distLengths));
  final litRevs = [
    for (var ctx = 0; ctx < 4; ctx++)
      _reverseCodeMap(_buildCanonicalCodes(litLengths[ctx]))
  ];

  final reader = _BitReader(_unpackBits(data.sublist(off)));
  final output = <int>[];

  while (true) {
    if (output.length >= originalLength) {
      break;
    }
    final icc = reader.nextHuffman(iccRev);
    if (icc == 63) {
      break;
    }
    final e = _iccTable[icc];
    final insertLen = e.insertBase + reader.readBits(e.insertExtra);
    final copyLen = e.copyBase + reader.readBits(e.copyExtra);

    for (var i = 0; i < insertLen; i++) {
      if (output.length >= originalLength) {
        break;
      }
      final ctx = literalContext(output.isEmpty ? null : output.last);
      final byteSym = reader.nextHuffman(litRevs[ctx]);
      output.add(byteSym & 0xff);
    }

    final dc = reader.nextHuffman(distRev);
    if (dc == 32) {
      continue;
    }
    if (copyLen > 0) {
      final dEntry = _distTable[dc];
      final copyDist = dEntry.base + reader.readBits(dEntry.extraBits);
      final start = output.length - copyDist;
      if (start < 0) {
        throw BrotliInvalidCopyDistance(copyDist, output.length);
      }
      for (var i = 0; i < copyLen; i++) {
        if (output.length >= originalLength) {
          break;
        }
        output.add(output[start + i]);
      }
    }
  }

  if (output.length != originalLength) {
    throw BrotliLengthMismatch(originalLength, output.length);
  }
  return Uint8List.fromList(output);
}
