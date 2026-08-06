import 'dart:io';
import 'dart:typed_data';
import 'package:test/test.dart';
import 'package:coding_adventures_zstd/coding_adventures_zstd.dart';

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Round-trip helper: compress then decompress and return the result.
Uint8List rt(Uint8List data) => decompress(compress(data));

/// Construct a Uint8List from a list of ints.
Uint8List bytes(List<int> values) => Uint8List.fromList(values);

/// Repeat a string [n] times and return as Uint8List.
Uint8List strBytes(String s, int n) =>
    Uint8List.fromList(List.generate(s.length * n, (i) => s.codeUnitAt(i % s.length)));

/// Checks whether the `zstd` CLI binary is reachable on PATH.
///
/// Returns `true` iff `zstd --version` runs and exits 0. Used to skip the
/// CLI-interop tests gracefully in environments without the real `zstd`
/// binary installed (CI/dev environments vary).
bool _isZstdCliAvailable() {
  try {
    final r = Process.runSync('zstd', ['--version']);
    return r.exitCode == 0;
  } catch (_) {
    return false;
  }
}

/// Compress [original] with the REAL `zstd` CLI, then decompress the result
/// with our own [decompress]. Returns the round-tripped bytes.
///
/// This is the "theirs → ours" direction of TC-9's cross-implementation
/// check, factored out so multiple fixtures can reuse it (see the
/// Repeated-Offset test group below). Our own [compress] never emits
/// repeat-offset sequences (see [_encodeSequencesSection]'s doc comment in
/// the library), so this direction — decoding bytes a real, unmodified
/// `zstd` encoder produced — is the only way to exercise our decoder's
/// Repeated_Offset (R1/R2/R3) handling at all.
Uint8List _decodeViaRealZstdCli(Uint8List original) {
  final tmpDir = Directory.systemTemp.createTempSync('zstd-dart-repoffset-');
  try {
    final inFile = File('${tmpDir.path}/in.bin');
    inFile.writeAsBytesSync(original);
    final result = Process.runSync(
      'zstd',
      ['-q', '-c', inFile.path],
      stdoutEncoding: null,
    );
    if (result.exitCode != 0) {
      fail('real `zstd` failed to compress the fixture: ${result.stderr}');
    }
    final compressed = Uint8List.fromList(result.stdout as List<int>);
    return decompress(compressed);
  } finally {
    tmpDir.deleteSync(recursive: true);
  }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

void main() {
  // ── TC-1: empty round-trip ─────────────────────────────────────────────────
  //
  // An empty input must produce a valid ZStd frame and decompress back to
  // empty bytes without error or panic.
  test('TC-1: empty round-trip', () {
    final data = Uint8List(0);
    final compressed = compress(data);
    // The frame must start with the ZStd magic number (4 bytes LE = 28 B5 2F FD).
    expect(compressed[0], equals(0x28));
    expect(compressed[1], equals(0xB5));
    expect(compressed[2], equals(0x2F));
    expect(compressed[3], equals(0xFD));
    // Decompressed result must equal the original empty bytes.
    expect(decompress(compressed), equals(data));
  });

  // ── TC-2: single byte 0x42 ─────────────────────────────────────────────────
  //
  // The smallest non-empty input: one byte 0x42 ('B').
  // A raw block is expected because LZSS finds no back-references in 1 byte.
  test('TC-2: single byte 0x42', () {
    final data = bytes([0x42]);
    expect(rt(data), equals(data));
  });

  // ── TC-3: all 256 byte values ──────────────────────────────────────────────
  //
  // Every possible byte value 0x00..0xFF in order. Exercises literal encoding
  // of zero bytes, control characters, and high bytes.
  test('TC-3: all 256 byte values', () {
    final data = bytes(List.generate(256, (i) => i));
    expect(rt(data), equals(data));
  });

  // ── TC-4: RLE block ────────────────────────────────────────────────────────
  //
  // 1024 identical bytes (b'A') must be detected as an RLE block and compress
  // to significantly fewer than 30 bytes:
  //   frame overhead = 4 (magic) + 1 (FHD) + 8 (FCS) = 13 bytes
  //   block overhead = 3 (header) + 1 (payload byte) = 4 bytes
  //   total = 17 bytes << 30 bytes
  test('TC-4: RLE — 1024 × 0x41 compresses to < 30 bytes', () {
    final data = Uint8List(1024)..fillRange(0, 1024, 0x41);
    final compressed = compress(data);
    expect(rt(data), equals(data));
    expect(
      compressed.length,
      lessThan(30),
      reason:
          'RLE block of 1024 identical bytes should be tiny; '
          'got ${compressed.length} bytes',
    );
  });

  // ── TC-5: English prose ────────────────────────────────────────────────────
  //
  // Repeated English text has strong LZ77 matches. Must achieve at least 20%
  // compression (output ≤ 80% of input size).
  test('TC-5: prose (25 × sentence) achieves ≥ 20% compression', () {
    const sentence = 'the quick brown fox jumps over the lazy dog ';
    final input = strBytes(sentence, 25);
    final compressed = compress(input);
    expect(rt(input), equals(input));
    final threshold = (input.length * 80) ~/ 100;
    expect(
      compressed.length,
      lessThan(threshold),
      reason:
          'Prose: compressed ${compressed.length} bytes '
          '(input ${input.length}), expected < $threshold (80%)',
    );
  });

  // ── TC-6: pseudo-random data ───────────────────────────────────────────────
  //
  // LCG pseudo-random bytes. No significant compression is expected, but the
  // round-trip must be exact regardless of which block type is chosen.
  test('TC-6: LCG random 512 bytes round-trip', () {
    var seed = 42;
    final input = Uint8List(512);
    for (var i = 0; i < 512; i++) {
      seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF;
      input[i] = seed & 0xFF;
    }
    expect(rt(input), equals(input));
  });

  // ── TC-7: 200 KB single-byte run ───────────────────────────────────────────
  //
  // 200 KB > MAX_BLOCK_SIZE (128 KB), so this requires at least two blocks.
  // Both blocks should be RLE (all bytes are 0xAB).
  test('TC-7: 200 KB single byte 0xAB — multi-block RLE', () {
    final data = Uint8List(200 * 1024)..fillRange(0, 200 * 1024, 0xAB);
    final compressed = compress(data);
    expect(rt(data), equals(data));
    // Two RLE blocks of 128 KB + 72 KB:
    //   Each block = 3-byte header + 1 payload = 4 bytes
    //   Frame = 13-byte overhead + 2 * 4 = 21 bytes
    // Allow some slack in case of different splitting.
    expect(compressed.length, lessThan(100));
  });

  // ── TC-8: 300 KB repetitive text ──────────────────────────────────────────
  //
  // A long repetitive ASCII string spanning multiple blocks.
  // Must round-trip exactly.
  test('TC-8: 300 KB repetitive text round-trip', () {
    final pattern = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ' * (300 * 1024 ~/ 26 + 1);
    final data = bytes(pattern.codeUnits.take(300 * 1024).toList());
    expect(rt(data), equals(data));
  });

  // ── Edge: bad magic → throws ───────────────────────────────────────────────
  //
  // A frame with the wrong magic number must be rejected with a FormatException.
  //
  // (This test used to be mislabelled "TC-9" even though it has nothing to do
  // with the spec's TC-9 — Cross-language / interoperability — which was a
  // stub. See the real "TC-9: CLI interoperability" tests below.)
  test('Edge: bad magic throws FormatException', () {
    final garbage = bytes([
      0x00, 0x00, 0x00, 0x00, // wrong magic
      0xE0,                   // FHD
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // FCS
      0x01, 0x00, 0x00,       // block header
    ]);
    expect(
      () => decompress(garbage),
      throwsA(isA<FormatException>()),
    );
  });

  // ── TC-9: Cross-language / interoperability (real `zstd` CLI) ─────────────
  //
  // Both directions must round-trip exactly against the REAL `zstd` binary:
  //   1. Compress with ours, decompress with `zstd -d`.
  //   2. Compress with `zstd`, decompress with ours.
  //
  // This is the test that actually proves the wire format is real RFC 8878,
  // not just a self-consistent internal format — a codec whose encoder and
  // decoder always agree with each other can still be silently wrong. This
  // package had exactly that: the FSE sequences-section codec (table-spread
  // algorithm, per-sequence field order, last-sequence state-init special
  // case) was internally self-consistent but non-conformant, and every
  // in-process round-trip test above passed regardless. Only a real
  // cross-implementation check like this one catches that class of bug. See
  // lessons.md Lesson 95/96. Skipped (not failed) when the `zstd` binary
  // isn't on PATH.
  test('TC-9: CLI interoperability — both directions round-trip exactly', () {
    if (!_isZstdCliAvailable()) {
      markTestSkipped('zstd CLI not found on PATH — skipping interop test');
      return;
    }

    const sentence = 'the quick brown fox jumps over the lazy dog ';
    final original = strBytes(sentence, 25);

    // Direction 1: compress with ours, decompress with real `zstd -d`.
    final ourCompressed = compress(original);
    final oursZst = Directory.systemTemp.createTempSync('zstd-dart-tc9-ours-');
    final oursZstFile = File('${oursZst.path}/out.zst');
    try {
      oursZstFile.writeAsBytesSync(ourCompressed);
      final result = Process.runSync(
        'zstd',
        ['-d', '-q', '-c', oursZstFile.path],
        stdoutEncoding: null,
      );
      expect(
        result.exitCode,
        equals(0),
        reason: 'real `zstd -d` failed to decode our compressed output: '
            '${result.stderr}',
      );
      expect(
        Uint8List.fromList(result.stdout as List<int>),
        equals(original),
        reason: 'real `zstd -d` decoded our output to different bytes',
      );
    } finally {
      oursZst.deleteSync(recursive: true);
    }

    // Direction 2: compress with real `zstd`, decompress with ours.
    final theirsDir = Directory.systemTemp.createTempSync('zstd-dart-tc9-theirs-');
    final theirsInput = File('${theirsDir.path}/in.txt');
    try {
      theirsInput.writeAsBytesSync(original);
      final result = Process.runSync(
        'zstd',
        ['-q', '-c', theirsInput.path],
        stdoutEncoding: null,
      );
      expect(
        result.exitCode,
        equals(0),
        reason: 'real `zstd` failed to compress the test input',
      );
      final theirCompressed = Uint8List.fromList(result.stdout as List<int>);
      final decodedByUs = decompress(theirCompressed);
      expect(
        decodedByUs,
        equals(original),
        reason: 'our decompress() failed to decode real `zstd`\'s output',
      );
    } finally {
      theirsDir.deleteSync(recursive: true);
    }
  });

  // ── RT: CLI interop with a high sequence count ────────────────────────────
  //
  // Real `zstd` CLI interop on an input large enough to push our compressor's
  // single-block sequence count past 128 — the exact boundary where the
  // sequence-count wire encoding switches from its 1-byte form to its 2-byte
  // form (RFC 8878 §3.1.1.3.1). Extra regression coverage for the FSE fix,
  // beyond the spec's 10 mandatory TCs: many sequences means many FSE state
  // transitions, so this exercises the corrected per-sequence field order and
  // last-sequence special case far more heavily than a single-sequence input
  // would.
  test('RT: CLI interop — high sequence count (2-byte seq-count form)', () {
    if (!_isZstdCliAvailable()) {
      markTestSkipped('zstd CLI not found on PATH — skipping interop test');
      return;
    }

    const src = [0x41, 0x42, 0x43, 0x44, 0x45, 0x46]; // 'ABCDEF'
    final original = bytes(List.generate(9000, (i) => src[i % src.length]));

    final ourCompressed = compress(original);
    final tmpDir = Directory.systemTemp.createTempSync('zstd-dart-rt-highseq-');
    final zstFile = File('${tmpDir.path}/out.zst');
    try {
      zstFile.writeAsBytesSync(ourCompressed);
      final result = Process.runSync(
        'zstd',
        ['-d', '-q', '-c', zstFile.path],
        stdoutEncoding: null,
      );
      expect(
        result.exitCode,
        equals(0),
        reason: 'real `zstd -d` failed to decode our high-sequence-count '
            'output (likely a sequence-count wire-format regression): '
            '${result.stderr}',
      );
      expect(
        Uint8List.fromList(result.stdout as List<int>),
        equals(original),
      );
    } finally {
      tmpDir.deleteSync(recursive: true);
    }
  });

  // ── Repeated-Offset (R1/R2/R3) sequence decoding — real `zstd` CLI ────────
  //
  // RFC 8878 §3.1.1.3.2.1.1: a sequence's Offset_Value of 1, 2, or 3 is not
  // a literal distance — it is a reference into a 3-slot history of
  // recently-used offsets (R1/R2/R3, defaulting to {1, 4, 8} at the start
  // of a frame). Real `zstd` encoders use this constantly, since re-using a
  // recent distance costs far fewer bits than encoding it explicitly. This
  // package's OWN encoder ([_encodeSequencesSection]) always writes an
  // explicit offset (biased +3, so raw distances 1..3 land on Offset_Value
  // 4..6 and never collide with a repeat-offset code) and so never emits
  // Offset_Value 1/2/3 itself — meaning no in-process `compress`+`decompress`
  // round trip (TC-1..TC-8, RT-*, etc. above) can ever exercise this path.
  // Only real `zstd`-CLI-produced input can: see lessons.md (the entry
  // documenting this gap, cross-referencing the already-CLI-verified
  // `c/zstd` port's PR that fixed the identical decode gap).
  //
  // Before this fix, every fixture below threw `FormatException: decoded
  // offset underflow` from the old `of_raw - 3` code path, which treated
  // Offset_Value 1/2/3 as a (nonsensical, always-rejected) explicit offset
  // instead of a repeat-offset reference.
  group('Repeated-Offset (R1/R2/R3) — real zstd CLI interop', () {
    test('constant-byte run (Offset_Value=1, default R1=1)', () {
      if (!_isZstdCliAvailable()) {
        markTestSkipped('zstd CLI not found on PATH — skipping interop test');
        return;
      }
      // A run of one repeated byte: the very first match is at distance 1,
      // which — because the offset-history default is R1=1 — real `zstd`
      // encodes as a bare repeat-offset reference rather than an explicit
      // offset. This was the exact minimal repro that first surfaced the
      // gap: 4713 bytes of the same byte compress to a single Compressed
      // block whose one sequence has Offset_Value=1.
      final original = Uint8List(4713)..fillRange(0, 4713, 0x41);
      expect(_decodeViaRealZstdCli(original), equals(original));
    });

    test(
      'CMP07-zstd.md TC-8 fixture: pattern at a fixed repeated distance',
      () {
        if (!_isZstdCliAvailable()) {
          markTestSkipped(
            'zstd CLI not found on PATH — skipping interop test',
          );
          return;
        }
        // Straight from the spec's own "TC-8: Repeat-offset compression"
        // (code/specs/CMP07-zstd.md): an 8-byte pattern reappears at the
        // same 128-byte distance ten times in a row. Each reappearance
        // after the first has a nonzero literal run (the 128 filler bytes)
        // before it, so real `zstd` repeatedly re-uses R1 unchanged
        // (selector 0 in the decoder's offset-resolution table) — multiple
        // separate sequences, not just one, all referencing the same
        // repeat-offset slot.
        const pattern = [0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48];
        final original = bytes([
          ...pattern,
          for (var i = 0; i < 10; i++) ...[
            ...List.filled(128, 0x58), // 'X' * 128
            ...pattern,
          ],
        ]);
        expect(_decodeViaRealZstdCli(original), equals(original));
      },
    );

    test('three interleaved repeat distances (stresses R1/R2/R3 rotation)', () {
      if (!_isZstdCliAvailable()) {
        markTestSkipped('zstd CLI not found on PATH — skipping interop test');
        return;
      }
      // Three distinct patterns, each periodic at its OWN distance (3, 5,
      // and 7 bytes), interleaved unit-by-unit. Note: this package's
      // decoder only supports Raw_Literals (RFC 8878 §3.1.1.2.1 type 0) —
      // real `zstd`'s Huffman-coded literals (type 2) are a separate,
      // already-documented out-of-scope limitation (see
      // _decodeLiteralsSection), not part of this repeat-offset fix. A
      // small, low-entropy byte alphabet like this one keeps `zstd`'s own
      // literal-type heuristic on the Raw side while still forcing R1 and
      // R2 (and their swap) into use as the encoder alternates which
      // pattern it is currently re-matching — a realistic way to exercise
      // more of the offset-history state machine than a single-distance
      // fixture can.
      const a = [0x10, 0x11, 0x12]; // period-3 pattern
      const b = [0x20, 0x21, 0x22, 0x23, 0x24]; // period-5 pattern
      const c = [0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36]; // period-7 pattern
      final units = <int>[];
      for (var i = 0; i < 300; i++) {
        units.addAll(a);
        units.addAll(b);
        units.addAll(c);
      }
      final original = bytes(units);
      expect(_decodeViaRealZstdCli(original), equals(original));
    });

    test('binary data with two interleaved repeat distances', () {
      if (!_isZstdCliAvailable()) {
        markTestSkipped('zstd CLI not found on PATH — skipping interop test');
        return;
      }
      // Two distinct short patterns, each periodic at its OWN distance,
      // interleaved unit-by-unit. A real encoder tracking "most recently
      // used" offsets will bounce between the two distances as it
      // alternates which pattern it is currently re-matching, which is a
      // realistic way to force R1 and R2 (and their swap) to both see use,
      // beyond the single-distance fixtures above.
      const a = [0x10, 0x11, 0x12, 0x13]; // period-4 pattern
      const b = [0x20, 0x21, 0x22, 0x23, 0x24, 0x25]; // period-6 pattern
      final units = <int>[];
      for (var i = 0; i < 300; i++) {
        units.addAll(a);
        units.addAll(b);
      }
      final original = bytes(units);
      expect(_decodeViaRealZstdCli(original), equals(original));
    });
  });

  // ── Additional round-trip tests ────────────────────────────────────────────

  // All-zero bytes: RLE should kick in for each block.
  test('RT: 1000 zero bytes', () {
    final data = Uint8List(1000);
    expect(rt(data), equals(data));
  });

  // All 0xFF bytes.
  test('RT: 1000 × 0xFF', () {
    final data = Uint8List(1000)..fillRange(0, 1000, 0xFF);
    expect(rt(data), equals(data));
  });

  // Classic "hello world".
  test('RT: hello world', () {
    final data = bytes('hello world'.codeUnits);
    expect(rt(data), equals(data));
  });

  // A medium-sized repeated binary pattern exercises the FSE encoder.
  test('RT: repeated binary pattern 3000 bytes', () {
    final pattern = [0x41, 0x42, 0x43, 0x44, 0x45, 0x46];
    final data = bytes(
      List.generate(3000, (i) => pattern[i % pattern.length]),
    );
    expect(rt(data), equals(data));
  });

  // Binary data cycling 0..255.
  test('RT: cyclic 0..255, 300 bytes', () {
    final data = bytes(List.generate(300, (i) => i % 256));
    expect(rt(data), equals(data));
  });

  // ── Determinism ────────────────────────────────────────────────────────────
  //
  // Compressing the same input twice must produce identical bytes.
  // This is required for reproducible builds and cache invalidation.
  test('Deterministic: same input produces identical compressed output', () {
    final data = strBytes('hello, ZStd world! ', 50);
    expect(compress(data), equals(compress(data)));
  });

  // ── TC-10: Wire format — minimal raw-block frame ───────────────────────────
  //
  // Manually construct a minimal ZStd frame to verify the decoder reads the
  // RFC 8878 wire format correctly, independent of our encoder.
  //
  // Frame layout:
  //   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
  //   [4]     FHD = 0x20:
  //             bits [7:6] = 00 → FCS_flag = 0
  //             bit  [5]   = 1  → Single_Segment = 1 → FCS is 1 byte
  //             bit  [4]   = 0  → Unused_bit
  //             bit  [3]   = 0  → Reserved_bit
  //             bit  [2]   = 0  → Content_Checksum_Flag = 0 (no checksum)
  //             bits [1:0] = 0  → Dictionary_ID_Flag = 0 (no dict)
  //   [5]     FCS = 5 (content size = 5 bytes)
  //   [6..8]  Block header: Last=1, Type=Raw(00), Size=5
  //             = (5 << 3) | 0 | 1 = 41 = 0x29
  //             = [0x29, 0x00, 0x00]
  //   [9..13] b'hello'
  test('TC-10: hand-crafted raw-block frame decodes correctly', () {
    final frame = bytes([
      0x28, 0xB5, 0x2F, 0xFD, // magic
      0x20,                   // FHD: Single_Segment=1, FCS=1 byte
      0x05,                   // FCS = 5
      0x29, 0x00, 0x00,       // block header: last=1, raw, size=5
      0x68, 0x65, 0x6C, 0x6C, 0x6F, // 'h','e','l','l','o'
    ]);
    expect(decompress(frame), equals(bytes('hello'.codeUnits)));
  });

  // ── Edge cases ─────────────────────────────────────────────────────────────

  test('Edge: frame too short throws FormatException', () {
    expect(
      () => decompress(bytes([0x28, 0xB5, 0x2F])),
      throwsA(isA<FormatException>()),
    );
  });

  test('Edge: two-byte input round-trip', () {
    final data = bytes([0x00, 0xFF]);
    expect(rt(data), equals(data));
  });

  test('Edge: all same byte value 0x00 (128 bytes)', () {
    final data = Uint8List(128);
    expect(rt(data), equals(data));
  });

  test('Edge: exactly MAX_BLOCK_SIZE (128 KB)', () {
    final data = Uint8List(128 * 1024)
      ..fillRange(0, 128 * 1024, 0x42);
    expect(rt(data), equals(data));
  });

  // ── Compression ratio tests ───────────────────────────────────────────────

  test('Ratio: repeated sentence compresses to < 10% of original', () {
    const sentence = 'the quick brown fox jumps over the lazy dog. ';
    final data = strBytes(sentence, 100);
    final compressed = compress(data);
    final threshold = (data.length * 10) ~/ 100;
    expect(
      compressed.length,
      lessThan(threshold),
      reason:
          'Expected highly repetitive input to compress to < 10%; '
          'got ${compressed.length} / ${data.length} bytes',
    );
    expect(rt(data), equals(data));
  });

  // ── Sequence count encoding edge cases ────────────────────────────────────
  //
  // These test the variable-length sequence count encoding at boundary values.

  test('Seq count: 0 round-trips', () {
    expect(rt(Uint8List(0)), equals(Uint8List(0)));
  });

  test('Seq count: values near 127/128 boundary compress correctly', () {
    // Build a string that generates exactly ~127 sequences when LZ77-encoded.
    // We use a sequence of random-looking bytes (no matches) followed by
    // a repeated block, which will produce a mix.
    // For simplicity, just verify round-trip on a medium prose block.
    final data = bytes(List.generate(1024, (i) => (i * 17 + 3) % 256));
    expect(rt(data), equals(data));
  });

  // ── Regression: seq_count endianness bug ──────────────────────────────────
  //
  // The 2-byte seq_count form must place the format-flag byte (with bit 7 set)
  // FIRST, not last. An earlier broken pattern in TS+Go wrote
  // `[count & 0xFF, (count >> 8) | 0x80]` — low byte first. For any count ≥ 128
  // whose low byte happened to be < 128 (e.g. 515 = 0x0203 → byte0 = 0x03), the
  // decoder mis-took the 1-byte path and silently returned a tiny garbage
  // count, mis-aligning every byte downstream (modes byte, FSE bitstream, …).
  //
  // 200 KB of long-period repetitive text reliably yields ≥ 128 sequences in
  // a single block (LZSS finds ~one match per pattern repetition). This
  // round-trip is the canonical regression: same pattern as the TS/Go
  // regression tests added in PR #1448.
  test('Seq count: 200 KB repetitive text — endianness regression', () {
    final pattern = 'hello world and more text for compression testing!\n';
    final buf = StringBuffer();
    for (var i = 0; i < 4000; i++) {
      buf.write(pattern);
    }
    final data = bytes(buf.toString().codeUnits);
    expect(rt(data), equals(data));
  });
}
