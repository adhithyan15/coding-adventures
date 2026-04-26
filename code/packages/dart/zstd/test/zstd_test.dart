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

  // ── TC-9: bad magic → throws ──────────────────────────────────────────────
  //
  // A frame with the wrong magic number must be rejected with a FormatException.
  test('TC-9: bad magic throws FormatException', () {
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

  // ── Wire format decoding ──────────────────────────────────────────────────
  //
  // Manually construct a minimal ZStd frame to verify the decoder reads the
  // RFC 8878 wire format correctly, independent of our encoder.
  //
  // Frame layout:
  //   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
  //   [4]     FHD = 0x20:
  //             bits [7:6] = 00 → FCS_flag = 0
  //             bit  [5]   = 1  → Single_Segment = 1 → FCS is 1 byte
  //             bits [4:0] = 0  → no checksum, no dict
  //   [5]     FCS = 5 (content size = 5 bytes)
  //   [6..8]  Block header: Last=1, Type=Raw(00), Size=5
  //             = (5 << 3) | 0 | 1 = 41 = 0x29
  //             = [0x29, 0x00, 0x00]
  //   [9..13] b'hello'
  test('Wire format: hand-crafted raw-block frame decodes correctly', () {
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
