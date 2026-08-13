import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_zip/coding_adventures_zip.dart';
import 'package:test/test.dart';

// ─── Helpers ─────────────────────────────────────────────────────────────────

Uint8List bytes(List<int> values) => Uint8List.fromList(values);

Uint8List utf8Bytes(String s) => Uint8List.fromList(utf8.encode(s));

/// Checks whether the `zip` and `unzip` CLI binaries are both reachable on
/// PATH. Used to skip the CLI-interop test gracefully in environments where
/// Info-ZIP isn't installed (CI/dev environments vary).
bool _isZipCliAvailable() {
  try {
    final z = Process.runSync('zip', ['-v']);
    final u = Process.runSync('unzip', ['-v']);
    return z.exitCode == 0 && u.exitCode == 0;
  } catch (_) {
    return false;
  }
}

void main() {
  // ── CRC-32 ──────────────────────────────────────────────────────────────

  group('CRC-32', () {
    test('known values', () {
      expect(crc32(utf8Bytes('hello world')), equals(0x0D4A1185));
      expect(crc32(utf8Bytes('123456789')), equals(0xCBF43926));
    });

    test('empty input is zero', () {
      expect(crc32(const []), equals(0));
    });

    test('incremental matches one-shot', () {
      final full = crc32(utf8Bytes('hello world'));
      final part1 = crc32(utf8Bytes('hello '));
      final part2 = crc32(utf8Bytes('world'), part1);
      expect(part2, equals(full));
    });
  });

  // ── RFC 1951 DEFLATE round-trips (own encoder vs own decoder) ────────────

  group('DEFLATE round-trip', () {
    void rt(Uint8List data) {
      final compressed = deflateCompress(data);
      final decompressed = inflate(compressed);
      expect(decompressed, equals(data));
    }

    test('empty', () => rt(bytes(const [])));
    test('single byte', () => rt(utf8Bytes('A')));
    test('all 256 byte values', () {
      rt(bytes(List<int>.generate(256, (i) => i)));
    });
    test('repetitive text compresses', () {
      final data = utf8Bytes('ABCABCABC' * 100);
      final compressed = deflateCompress(data);
      expect(inflate(compressed), equals(data));
      expect(compressed.length, lessThan(data.length));
    });
    test('long prose string', () {
      rt(utf8Bytes('the quick brown fox jumps over the lazy dog ' * 20));
    });
  });

  // ── TC-1: Round-trip single file (Stored) ─────────────────────────────────

  test('TC-1: round-trip single file (Stored)', () {
    final data = utf8Bytes('hello, world');
    final w = ZipWriter()..addFile('hello.txt', data, compress: false);
    final archive = w.finish();
    final result = unzip(archive);
    expect(result['hello.txt'], equals(data));

    final entry = ZipReader(archive).entries().single;
    expect(entry.method, equals(0));
  });

  // ── TC-2: Round-trip single file (DEFLATE) ────────────────────────────────

  test('TC-2: round-trip single file (DEFLATE)', () {
    final text = utf8Bytes('the quick brown fox jumps over the lazy dog ' * 10);
    final archive = zipBytes([('text.txt', text)]);
    final result = unzip(archive);
    expect(result['text.txt'], equals(text));

    final entry = ZipReader(archive).entries().single;
    expect(entry.method, equals(8));
  });

  // ── TC-3: Multiple files in one archive ───────────────────────────────────

  test('TC-3: multiple files in one archive', () {
    final files = <(String, List<int>)>[
      ('a.txt', utf8Bytes('file A content')),
      ('b.txt', utf8Bytes('file B content')),
      ('c.bin', List<int>.generate(256, (i) => i)),
    ];
    final archive = zipBytes(files);
    final result = unzip(archive);
    for (final (name, data) in files) {
      expect(result[name], equals(data), reason: 'mismatch for $name');
    }
  });

  // ── TC-4: Directory entry ─────────────────────────────────────────────────

  test('TC-4: directory entry', () {
    final w = ZipWriter()
      ..addDirectory('mydir/')
      ..addFile('mydir/file.txt', utf8Bytes('contents'));
    final archive = w.finish();
    final entries = ZipReader(archive).entries();
    final names = entries.map((e) => e.name).toSet();
    expect(names, contains('mydir/'));
    expect(names, contains('mydir/file.txt'));

    final dirEntry = entries.firstWhere((e) => e.name == 'mydir/');
    expect(dirEntry.isDirectory, isTrue);
  });

  // ── TC-5: CRC-32 verification ─────────────────────────────────────────────

  test('TC-5: CRC-32 corruption is detected', () {
    final archive = zipBytes([('f.txt', utf8Bytes('test'))]);
    final corrupted = Uint8List.fromList(archive);
    // Corrupt a data byte directly. Corrupting only the Local Header's CRC
    // field has no effect: the reader validates against the Central
    // Directory's CRC, the authoritative source per the CMP09 spec.
    corrupted[35] ^= 0xFF;

    expect(() => unzip(corrupted), throwsA(isA<FormatException>()));
  });

  // ── TC-6: EOCD detection and multi-file random access ─────────────────────

  test('TC-6: EOCD detection and random access', () {
    final files = List<(String, List<int>)>.generate(
      10,
      (i) => ('f$i.txt', utf8Bytes('content $i')),
    );
    final archive = zipBytes(files);
    final reader = ZipReader(archive);
    final entry5 = reader.entries().firstWhere((e) => e.name == 'f5.txt');
    expect(reader.read(entry5), equals(utf8Bytes('content 5')));
  });

  // ── TC-7: Incompressible data stored without compression ─────────────────

  test('TC-7: incompressible data falls back to Stored', () {
    // Pseudo-random bytes via LCG (seed=42): DEFLATE cannot shrink this.
    var seed = 42;
    final data = Uint8List(1024);
    for (var i = 0; i < data.length; i++) {
      seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF;
      data[i] = (seed >> 24) & 0xFF;
    }

    final archive = zipBytes([('random.bin', data)]);
    final result = unzip(archive);
    expect(result['random.bin'], equals(data));

    final reader = ZipReader(archive);
    final entry = reader.entries().single;
    expect(entry.method, equals(0));
  });

  // ── TC-8: Empty file ───────────────────────────────────────────────────────

  test('TC-8: empty file', () {
    final archive = zipBytes([('empty.txt', const <int>[])]);
    final result = unzip(archive);
    expect(result['empty.txt'], equals(const <int>[]));
  });

  // ── TC-9: Large file (multi-block-worthy DEFLATE, single block here) ─────

  test('TC-9: 100 KB repetitive file compresses', () {
    final data = utf8Bytes('abcdefghij' * 10000); // 100 KB
    final archive = zipBytes([('big.bin', data)]);
    final result = unzip(archive);
    expect(result['big.bin'], equals(data));
    expect(
      archive.length,
      lessThan(data.length),
      reason: 'repetitive 100 KB must compress',
    );
  });

  // ── TC-10: Cross-compatibility with system ZIP tools ──────────────────────
  //
  // Manual/subprocess-based, matching every other language port's TC-10 in
  // this repository. Skips gracefully when Info-ZIP (`zip`/`unzip`) is not
  // on PATH.

  group('TC-10: CLI interoperability', () {
    test('our writer → system unzip', () async {
      if (!_isZipCliAvailable()) {
        markTestSkipped('zip/unzip CLI not available on PATH');
        return;
      }
      final tmp = Directory.systemTemp.createTempSync('dart_zip_write_');
      addTearDown(() => tmp.deleteSync(recursive: true));

      final payload = 'the quick brown fox jumps over the lazy dog ' * 30;
      final archive = zipBytes([
        ('hello.txt', utf8Bytes(payload)),
        ('nested/dir/deep.txt', utf8Bytes('deep content')),
      ]);
      final archivePath = '${tmp.path}/ours.zip';
      File(archivePath).writeAsBytesSync(archive);

      final result = Process.runSync('unzip', ['-o', archivePath, '-d', tmp.path]);
      expect(result.exitCode, equals(0), reason: result.stderr.toString());
      expect(
        File('${tmp.path}/hello.txt').readAsStringSync(),
        equals(payload),
      );
      expect(
        File('${tmp.path}/nested/dir/deep.txt').readAsStringSync(),
        equals('deep content'),
      );
    });

    test('system zip → our reader', () {
      if (!_isZipCliAvailable()) {
        markTestSkipped('zip/unzip CLI not available on PATH');
        return;
      }
      final tmp = Directory.systemTemp.createTempSync('dart_zip_read_');
      addTearDown(() => tmp.deleteSync(recursive: true));

      final payload = 'SpreadsheetML cell A1: revenue=1000; ' * 20;
      File('${tmp.path}/sheet.xml').writeAsStringSync(payload);
      final archivePath = '${tmp.path}/theirs.zip';

      final result = Process.runSync(
        'zip',
        ['-9', archivePath, 'sheet.xml'],
        workingDirectory: tmp.path,
      );
      expect(result.exitCode, equals(0), reason: result.stderr.toString());

      final archiveBytes = File(archivePath).readAsBytesSync();
      final files = unzip(archiveBytes);
      expect(files['sheet.xml'], equals(utf8Bytes(payload)));
    });
  });

  // ── TC-11: Unicode filename ────────────────────────────────────────────────

  test('TC-11: unicode filename', () {
    const name = '日本語/résumé.txt';
    final archive = zipBytes([(name, utf8Bytes('content'))]);
    final result = unzip(archive);
    expect(result.containsKey(name), isTrue);
    expect(result[name], equals(utf8Bytes('content')));
  });

  // ── TC-12: Nested paths ─────────────────────────────────────────────────────

  test('TC-12: nested paths', () {
    final files = <(String, List<int>)>[
      ('root.txt', utf8Bytes('root')),
      ('dir/file.txt', utf8Bytes('nested')),
      ('dir/sub/deep.txt', utf8Bytes('deep')),
    ];
    final archive = zipBytes(files);
    final result = unzip(archive);
    for (final (name, data) in files) {
      expect(result[name], equals(data), reason: 'mismatch for $name');
    }
  });

  // ── Additional coverage: dynamic-Huffman real-world entry ─────────────────
  //
  // A hand-crafted ZIP whose single entry uses a DYNAMIC Huffman block
  // (BTYPE=10) — the block type our own writer never emits, but which
  // virtually every real-world producer (zlib, Python zipfile, Java jar,
  // Microsoft Office) uses. Proves `inflate` decodes dynamic blocks, not
  // just the fixed-Huffman blocks `deflateCompress` produces. Generated by
  // `python3 -c "import zipfile; ..."` (zlib level 9) with the payload
  // below, then captured as a byte literal so the test has no external
  // dependency.
  test('reads a real-world dynamic-Huffman ZIP entry', () {
    final expected = utf8Bytes(dynamicHuffmanPayload * 12);
    final files = unzip(dynamicHuffmanZipFixture);
    expect(files.length, equals(1));
    expect(files['sheet1.xml'], equals(expected));

    final reader = ZipReader(dynamicHuffmanZipFixture);
    expect(reader.readByName('sheet1.xml'), equals(expected));
  });

  test('rejects a suffix inside a declared DEFLATE payload', () {
    final archive = zipBytes([
      ('payload.txt', utf8Bytes('hidden cavity regression ' * 40)),
    ]);
    final reader = ZipReader(archive);
    final entry = reader.entries().single;
    expect(entry.method, 8);

    final originalView = ByteData.sublistView(archive);
    final originalEocd = archive.length - 22;
    final originalCd = originalView.getUint32(
      originalEocd + 16,
      Endian.little,
    );
    final tampered = Uint8List(archive.length + 1)
      ..setRange(0, originalCd, archive)
      ..[originalCd] = 0xDE
      ..setRange(originalCd + 1, archive.length + 1, archive, originalCd);
    final view = ByteData.sublistView(tampered);
    final newCd = originalCd + 1;
    final newEocd = originalEocd + 1;
    view.setUint32(18, entry.compressedSize + 1, Endian.little);
    view.setUint32(newCd + 20, entry.compressedSize + 1, Endian.little);
    view.setUint32(newEocd + 16, newCd, Endian.little);

    final tamperedReader = ZipReader(tampered);
    expect(
      () => tamperedReader.read(tamperedReader.entries().single),
      throwsA(isA<FormatException>()),
    );
  });
  // ── Additional coverage: read_by_name ──────────────────────────────────────

  test('read_by_name finds an entry and throws for missing names', () {
    final archive = zipBytes([
      ('alpha.txt', utf8Bytes('AAA')),
      ('beta.txt', utf8Bytes('BBB')),
    ]);
    final reader = ZipReader(archive);
    expect(reader.readByName('beta.txt'), equals(utf8Bytes('BBB')));
    expect(
      () => reader.readByName('nope.txt'),
      throwsA(isA<FormatException>()),
    );
  });

  // ── Additional coverage: empty archive ─────────────────────────────────────

  test('empty archive round-trips to no entries', () {
    final archive = zipBytes(const []);
    final result = unzip(archive);
    expect(result, isEmpty);
    expect(ZipReader(archive).entries(), isEmpty);
  });

  // ── Additional coverage: unsupported method rejected ───────────────────────

  test('unsupported compression method is rejected', () {
    final archive = zipBytes([('f.txt', utf8Bytes('hi'))]);
    final entry = ZipReader(archive).entries().single;
    // Patch the Central Directory's method field (offset 10 of the CD
    // header) to an unsupported value (12 = Bzip2) without touching the
    // Local Header, so the reader still finds real data to (fail to) decode.
    final cdMethodOffset = archive.length -
        22 - // EOCD
        (46 + entry.name.length) + // this entry's CD header
        10;
    final tampered = Uint8List.fromList(archive);
    tampered[cdMethodOffset] = 12;
    tampered[cdMethodOffset + 1] = 0;

    final reader = ZipReader(tampered);
    expect(
      () => reader.read(reader.entries().single),
      throwsA(isA<FormatException>()),
    );
  });

  // ── Additional coverage: encrypted entry rejected ──────────────────────────

  test('encrypted entry (GP flag bit 0) is rejected', () {
    final archive = zipBytes([('f.txt', utf8Bytes('hi'))]);
    final tampered = Uint8List.fromList(archive);
    // Local Header flags field is at offset 6; set bit 0 (encrypted).
    tampered[6] |= 0x01;

    final reader = ZipReader(tampered);
    expect(
      () => reader.read(reader.entries().single),
      throwsA(isA<FormatException>()),
    );
  });

  // ── Additional coverage: encrypted entry per Central Directory flags ──────
  //
  // The Central Directory is the authoritative header (per the CMP09 spec
  // and this package's own read() logic), so the encrypted check must
  // catch a crafted archive whose CD flags mark encryption even when the
  // Local Header's copy of the flags field disagrees.

  test('encrypted entry per Central Directory (not Local Header) is rejected', () {
    final archive = zipBytes([('f.txt', utf8Bytes('hi'))]);
    final entry = ZipReader(archive).entries().single;
    // Central Directory flags field is at offset 8 of the 46-byte CD
    // header; Local Header flags (offset 6) is deliberately left alone.
    final cdFlagsOffset =
        archive.length - 22 - (46 + entry.name.length) + 8;
    final tampered = Uint8List.fromList(archive);
    tampered[cdFlagsOffset] |= 0x01;

    final reader = ZipReader(tampered);
    final tamperedEntry = reader.entries().single;
    expect(tamperedEntry.isEncrypted, isTrue);
    expect(
      () => reader.read(tamperedEntry),
      throwsA(isA<FormatException>()),
    );
  });

  // ── Additional coverage: Central Directory entry-count mismatch ───────────
  //
  // A crafted archive can inflate one entry's name/extra/comment length so
  // the parser's `pos` desyncs from the real next Central Directory header,
  // which would otherwise make the parse loop's signature check fail and
  // silently `break` out early — returning a truncated-but-plausible entry
  // list with no error. Cross-checking the parsed count against the EOCD's
  // own declared total catches this instead of losing entries silently.

  test('Central Directory entry count mismatch is rejected', () {
    final archive = zipBytes([
      ('a.txt', utf8Bytes('AAA')),
      ('b.txt', utf8Bytes('BBB')),
    ]);
    final tampered = Uint8List.fromList(archive);
    // EOCD's entries_this_disk (offset 8) and entries_total (offset 10)
    // fields, each a little-endian u16; claim one more entry than exists.
    final eocdStart = archive.length - 22;
    tampered[eocdStart + 8] += 1;
    tampered[eocdStart + 10] += 1;

    expect(() => ZipReader(tampered), throwsA(isA<FormatException>()));
  });

  // ── Additional coverage: malformed archive has no EOCD ─────────────────────

  test('archive with no EOCD record throws', () {
    expect(
      () => ZipReader(utf8Bytes('not a zip file at all')),
      throwsA(isA<FormatException>()),
    );
  });

  // ── Additional coverage: MS-DOS datetime ───────────────────────────────────

  test('dosDatetime epoch matches the fixed constant', () {
    final dt = dosDatetime(1980, 1, 1, 0, 0, 0);
    expect(dt, equals(dosEpoch));
    expect(dt >> 16, equals(33)); // date field: (0<<9)|(1<<5)|1
    expect(dt & 0xFFFF, equals(0)); // time field
  });
}

// ─── Real-world dynamic-Huffman fixture ────────────────────────────────────

/// One repeat unit of the fixture's payload; the fixture's `sheet1.xml`
/// entry is this string repeated 12 times (1500 bytes uncompressed, per
/// its Local Header).
const String dynamicHuffmanPayload =
    'SpreadsheetML cell A1: revenue=1000; A2: revenue=2000; total=SUM(A1:A2). '
    'Office Open XML parts are raw DEFLATE inside a ZIP. ';

/// A ZIP produced by Python's `zipfile` module (zlib level 9), whose single
/// entry `sheet1.xml` is compressed with method 8 (DEFLATE) using a dynamic
/// Huffman block. Byte-identical to the fixture `rust/zip` carries for the
/// same purpose (`code/packages/rust/zip/src/lib.rs`).
final Uint8List dynamicHuffmanZipFixture = Uint8List.fromList(const <int>[
  0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x90, 0x88, 0xe2, 0x5c, 0x50, 0x87,
  0x66, 0x1d, 0x7f, 0x00, 0x00, 0x00, 0xdc, 0x05, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x73, 0x68,
  0x65, 0x65, 0x74, 0x31, 0x2e, 0x78, 0x6d, 0x6c, 0xed, 0xcd, 0xb1, 0x0a, 0xc2, 0x30, 0x14, 0x85,
  0xe1, 0x57, 0x39, 0xa3, 0x2e, 0x25, 0xcd, 0xa8, 0x74, 0x08, 0x58, 0x41, 0x68, 0xa9, 0x10, 0x05,
  0x71, 0xbb, 0xb4, 0xb7, 0x18, 0x08, 0x69, 0xb8, 0x89, 0xfa, 0xfa, 0x16, 0x17, 0x9f, 0xc0, 0x2d,
  0xeb, 0xcf, 0xe1, 0x7c, 0x36, 0x0a, 0xd3, 0x94, 0x1e, 0xcc, 0xb9, 0xef, 0x30, 0xb2, 0xf7, 0x30,
  0xf5, 0x0e, 0xc2, 0x2f, 0x0e, 0x4f, 0x6e, 0x6a, 0xa5, 0xd4, 0x1e, 0x46, 0xff, 0x8a, 0xfe, 0x96,
  0xbc, 0x64, 0xf2, 0x8d, 0xbd, 0xf6, 0x9b, 0x75, 0x6d, 0xf4, 0xb6, 0xc2, 0x30, 0xcf, 0x6e, 0x64,
  0x0c, 0x91, 0x03, 0x6e, 0xeb, 0x55, 0x24, 0xc9, 0x09, 0x24, 0x0c, 0xa1, 0x37, 0x0e, 0xed, 0xb1,
  0x33, 0x97, 0x16, 0x2e, 0x24, 0x37, 0x31, 0x08, 0xf7, 0xd3, 0xb9, 0x82, 0x2d, 0x78, 0xc1, 0x0b,
  0x5e, 0xf0, 0x82, 0xff, 0x03, 0xff, 0x00, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x00,
  0x00, 0x08, 0x00, 0x90, 0x88, 0xe2, 0x5c, 0x50, 0x87, 0x66, 0x1d, 0x7f, 0x00, 0x00, 0x00, 0xdc,
  0x05, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
  0x01, 0x00, 0x00, 0x00, 0x00, 0x73, 0x68, 0x65, 0x65, 0x74, 0x31, 0x2e, 0x78, 0x6d, 0x6c, 0x50,
  0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x38, 0x00, 0x00, 0x00, 0xa7,
  0x00, 0x00, 0x00, 0x00, 0x00,
]);
