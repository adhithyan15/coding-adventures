import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_sha512/coding_adventures_sha512.dart';
import 'package:coding_adventures_sha512/src/constants.dart';
import 'package:coding_adventures_sha512/src/messages.dart';
import 'package:coding_adventures_sha512/src/sha512.dart'
    show setHasherByteCountersForTesting;
import 'package:test/test.dart';

final RegExp _idPattern = RegExp(r'^[a-z0-9]+(?:-[a-z0-9]+)*$');
final RegExp _hexLowerPattern = RegExp(r'^[0-9a-f]*$');
final RegExp _digestHexPattern = RegExp(r'^[0-9a-f]{128}$');
final RegExp _word64HexPattern = RegExp(r'^[0-9a-f]{16}$');
final RegExp _chunksCsvPattern = RegExp(r'^[1-9][0-9]*(?:,[1-9][0-9]*)*$');

File _findFixturesFile() {
  Directory current = Directory.current;
  for (var i = 0; i < 10; i++) {
    final candidate = File(
      '${current.path}/code/specs/fixtures/sha512-v1/cases.txt',
    );
    if (candidate.existsSync()) {
      return candidate;
    }
    final candidate2 = File('${current.path}/fixtures/sha512-v1/cases.txt');
    if (candidate2.existsSync()) {
      return candidate2;
    }
    current = current.parent;
  }
  throw StateError('Could not find sha512-v1/cases.txt');
}

String _readBoundedCorpus(File file, int maxBytes) {
  final raf = file.openSync(mode: FileMode.read);
  try {
    final buf = Uint8List(maxBytes + 1);
    final readTotal = raf.readIntoSync(buf);
    if (readTotal < 1 || readTotal > maxBytes) {
      throw ArgumentError(Sha512Messages.errCorpusOversized);
    }
    return utf8.decode(buf.sublist(0, readTotal), allowMalformed: false);
  } finally {
    raf.closeSync();
  }
}

Uint8List _parseHex(String hex) {
  if (hex.isEmpty) return Uint8List(0);
  final result = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < result.length; i++) {
    result[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return result;
}

String _formatHex(List<int> bytes) {
  final sb = StringBuffer();
  for (final b in bytes) {
    sb.write(b.toRadixString(16).padLeft(2, '0'));
  }
  return sb.toString();
}

void _parseCorpus(String content) {
  final lines = content.split(RegExp(r'\r?\n'));
  var rowCount = 0;
  var vCount = 0;
  var lCount = 0;

  for (final line in lines) {
    final trimmed = line.trim();
    if (trimmed.isEmpty || trimmed.startsWith('#')) {
      continue;
    }
    rowCount++;
    final parts = trimmed.split('|');
    if (parts.length != 5) {
      throw ArgumentError(Sha512Messages.errCorpusMalformed);
    }
    final tag = parts[0];
    final id = parts[1];
    if (!_idPattern.hasMatch(id)) {
      throw ArgumentError(Sha512Messages.errCorpusMalformed);
    }

    if (tag == 'V') {
      vCount++;
      final inputHex = parts[2];
      final chunkCsv = parts[3];
      final expectedDigestHex = parts[4];

      if (!_hexLowerPattern.hasMatch(inputHex) ||
          (inputHex.length % 2 != 0) ||
          inputHex.length > 8192) {
        throw ArgumentError(Sha512Messages.errCorpusMalformed);
      }
      if (!_digestHexPattern.hasMatch(expectedDigestHex)) {
        throw ArgumentError(Sha512Messages.errCorpusMalformed);
      }
      final input = _parseHex(inputHex);
      if (input.isEmpty) {
        if (chunkCsv.isNotEmpty) {
          throw ArgumentError(Sha512Messages.errCorpusMalformed);
        }
      } else {
        if (chunkCsv.isEmpty || !_chunksCsvPattern.hasMatch(chunkCsv)) {
          throw ArgumentError(Sha512Messages.errCorpusMalformed);
        }
        final chunks = chunkCsv.split(',');
        if (chunks.length > 64) {
          throw ArgumentError(Sha512Messages.errCorpusMalformed);
        }
        var sum = 0;
        for (final c in chunks) {
          sum += int.parse(c);
        }
        if (sum != input.length) {
          throw ArgumentError(Sha512Messages.errCorpusMalformed);
        }
      }

      // 1. One-shot byte digest
      final digest = sha512(input);
      expect(digest.length, equals(64));
      expect(_formatHex(digest), equals(expectedDigestHex));

      // 2. One-shot hex helper
      final hex = sha512Hex(input);
      expect(hex, equals(expectedDigestHex));

      // 3. Streaming Hasher using chunk_sizes_csv
      final hasher = Sha512Hasher();
      if (chunkCsv.isNotEmpty) {
        final chunkStrs = chunkCsv.split(',');
        var offset = 0;
        for (final chunkStr in chunkStrs) {
          final chunkSize = int.parse(chunkStr.trim());
          final chunk = input.sublist(offset, offset + chunkSize);
          hasher.update(chunk);
          offset += chunkSize;
        }
      } else {
        hasher.update(input);
      }
      expect(hasher.hexDigest(), equals(expectedDigestHex));
    } else if (tag == 'L') {
      lCount++;
      final highStr = parts[2];
      final lowStr = parts[3];
      final expected = parts[4];

      if (!_word64HexPattern.hasMatch(highStr) ||
          !_word64HexPattern.hasMatch(lowStr)) {
        throw ArgumentError(Sha512Messages.errCorpusMalformed);
      }
      final high = BigInt.parse(highStr, radix: 16);
      final low = BigInt.parse(lowStr, radix: 16);

      if (expected == 'ERROR') {
        expect(
          () => encodeBitLength(high, low),
          throwsA(isA<Sha512Exception>()),
        );
      } else {
        if (!_hexLowerPattern.hasMatch(expected) || expected.length != 32) {
          throw ArgumentError(Sha512Messages.errCorpusMalformed);
        }
        final encoded = encodeBitLength(high, low);
        expect(encoded.length, equals(16));
        expect(_formatHex(encoded), equals(expected));
      }
    } else {
      throw ArgumentError('${Sha512Messages.errCorpusMalformed}: $line');
    }
  }

  if (rowCount < 1 || rowCount > 64) {
    throw ArgumentError(Sha512Messages.errCorpusMalformed);
  }
  expect(vCount, equals(9), reason: 'Corpus must have exactly 9 V rows');
  expect(lCount, equals(6), reason: 'Corpus must have exactly 6 L rows');
}

void main() {
  group('FIPS 180-4 Neutral Conformance Corpus (cases.txt)', () {
    test('exact 9 V rows and 6 L rows with bounded reader <= 4096 bytes', () {
      final fixtureFile = _findFixturesFile();
      final content = _readBoundedCorpus(fixtureFile, 4096);
      _parseCorpus(content);
    });

    test('corpus reader fails closed on oversized and empty files', () {
      final tempDir = Directory.systemTemp.createTempSync('sha512_test_');
      final tempFile = File('${tempDir.path}/oversized.txt');
      tempFile.writeAsBytesSync(Uint8List(4097));
      try {
        expect(
          () => _readBoundedCorpus(tempFile, 4096),
          throwsA(isA<ArgumentError>()),
        );
      } finally {
        tempDir.deleteSync(recursive: true);
      }

      final emptyDir = Directory.systemTemp.createTempSync('sha512_empty_');
      final emptyFile = File('${emptyDir.path}/empty.txt');
      emptyFile.writeAsBytesSync(Uint8List(0));
      try {
        expect(
          () => _readBoundedCorpus(emptyFile, 4096),
          throwsA(isA<ArgumentError>()),
        );
      } finally {
        emptyDir.deleteSync(recursive: true);
      }
    });

    test('corpus reader fails closed on malformed rows', () {
      // Unknown tag
      expect(
        () => _parseCorpus('X|unknown|1234|1|digest\n'),
        throwsA(isA<ArgumentError>()),
      );
      // Wrong column count
      expect(() => _parseCorpus('V|id|1234\n'), throwsA(isA<ArgumentError>()));
      // Uppercase hex in V input
      expect(
        () => _parseCorpus('V|id|AB|1|${'0' * 128}\n'),
        throwsA(isA<ArgumentError>()),
      );
      // Uppercase hex in V digest
      expect(
        () => _parseCorpus('V|id|aa|1|${'A' * 128}\n'),
        throwsA(isA<ArgumentError>()),
      );
      // Chunk sizes not summing to input size
      expect(
        () => _parseCorpus('V|id|aabb|1|${'0' * 128}\n'),
        throwsA(isA<ArgumentError>()),
      );
      // Invalid ID characters
      expect(
        () => _parseCorpus('V|ID_UPPER|aa|1|${'0' * 128}\n'),
        throwsA(isA<ArgumentError>()),
      );
      // Uppercase in L row word
      expect(
        () => _parseCorpus(
          'L|id|000000000000000A|0000000000000000|00000000000000000000000000000000\n',
        ),
        throwsA(isA<ArgumentError>()),
      );
      // Short L row word
      expect(
        () => _parseCorpus('L|id|0|0|ERROR\n'),
        throwsA(isA<ArgumentError>()),
      );
    });
  });

  // ─── 6-Class SHA-512 Resilience Showcase ────────────────────────────────────

  group('6-Class SHA-512 Resilience Showcase', () {
    test('Class 1: Counter wraparound past 61 bits (> 2^61 bytes)', () {
      final lowOverflow = BigInt.parse('2000000000000001', radix: 16);
      final encoded = encodeBitLength(BigInt.zero, lowOverflow);
      expect(_formatHex(encoded), equals('00000000000000010000000000000008'));

      final encodedHigh = encodeBitLength(BigInt.one, BigInt.zero);
      expect(
        _formatHex(encodedHigh),
        equals('00000000000000080000000000000000'),
      );
    });

    test(
      'Class 2: FIPS overflow failure atomicity in encodeBitLength and Hasher',
      () {
        final hasher = Sha512Hasher();
        hasher.update([1, 2, 3]);
        final hexBefore = hasher.hexDigest();

        expect(
          () => encodeBitLength(
            BigInt.parse('2000000000000000', radix: 16),
            BigInt.zero,
          ),
          throwsA(isA<Sha512Exception>()),
        );

        // Hasher state must remain intact
        expect(hasher.hexDigest(), equals(hexBefore));
        hasher.update([4, 5, 6]);
        expect(hasher.hexDigest(), isNot(equals(hexBefore)));

        // Exercise Hasher.update counter carry
        final carryHasher = Sha512Hasher();
        setHasherByteCountersForTesting(
          carryHasher,
          BigInt.zero,
          BigInt.parse('FFFFFFFFFFFFFFFE', radix: 16),
        );
        carryHasher.update([1, 2, 3]);
        expect(carryHasher.hexDigest(), isNotEmpty);

        // Exercise Hasher.update 128-bit domain overflow failure atomicity
        final overflowHasher = Sha512Hasher();
        overflowHasher.update([9, 8, 7]);
        setHasherByteCountersForTesting(
          overflowHasher,
          BigInt.parse('1FFFFFFFFFFFFFFF', radix: 16),
          BigInt.parse('FFFFFFFFFFFFFFFF', radix: 16),
        );
        final snap = overflowHasher.hexDigest();

        expect(
          () => overflowHasher.update([1]),
          throwsA(isA<Sha512Exception>()),
        );
        expect(overflowHasher.hexDigest(), equals(snap));
      },
    );

    test('Class 3: Padding boundary splits (single and double block)', () {
      const boundaries = [111, 112, 127, 128, 129, 239, 240, 255, 256, 257];
      for (final len in boundaries) {
        final data = Uint8List(len);
        for (var i = 0; i < len; i++) {
          data[i] = (i * 31 + 7) & 0xFF;
        }
        final oneShot = sha512Hex(data);

        // Incremental streaming in varying chunks must produce identical digest
        final hasher = Sha512Hasher();
        var offset = 0;
        final step = max(1, len ~/ 3);
        while (offset < len) {
          final end = min(len, offset + step);
          hasher.update(data.sublist(offset, end));
          offset = end;
        }
        expect(
          hasher.hexDigest(),
          equals(oneShot),
          reason: 'Mismatch at padding boundary $len',
        );
      }
    });

    test(
      'Class 4: Buffer aliasing, post-snapshot continuation, and cloneHasher',
      () {
        final input = Uint8List.fromList([10, 20, 30, 40]);
        final hasher = Sha512Hasher();
        hasher.update(input);

        // Caller mutates original buffer: hasher must not be affected
        input[0] = 99;
        final snapshot1 = hasher.hexDigest();

        hasher.update([50, 60]);
        final snapshot2 = hasher.hexDigest();
        expect(snapshot1, isNot(equals(snapshot2)));

        final copy = hasher.copy();
        expect(copy.hexDigest(), equals(snapshot2));

        final clone = hasher.cloneHasher();
        expect(clone.hexDigest(), equals(snapshot2));

        copy.update([70]);
        expect(hasher.hexDigest(), isNot(equals(copy.hexDigest())));
        expect(hasher.hexDigest(), equals(clone.hexDigest()));
      },
    );

    test(
        'Class 5: Fail-closed on invalid byte integers, negative indices, and wrapping attacks',
        () {
      final hasher = Sha512Hasher();
      hasher.update([1, 2, 3]);
      final snapBefore = hasher.hexDigest();

      // Byte > 255 throws without corrupting state
      expect(() => hasher.update([4, 5, 256]), throwsA(isA<Sha512Exception>()));
      expect(hasher.hexDigest(), equals(snapBefore));

      // Negative byte
      expect(() => hasher.update([1, 2, -1]), throwsA(isA<Sha512Exception>()));
      expect(hasher.hexDigest(), equals(snapBefore));

      // Negative offset
      expect(() => hasher.update([1, 2], -1, 1), throwsA(isA<RangeError>()));

      // Offset beyond length
      expect(() => hasher.update([1, 2], 5, 1), throwsA(isA<RangeError>()));

      // Length exceeding data length
      expect(() => hasher.update([1, 2], 0, 10), throwsA(isA<RangeError>()));

      // Integer wrapping attack on 64-bit int VM: offset = 1, length = MAX_INT
      expect(
        () => hasher.update([1, 2, 3, 4], 1, 0x7FFFFFFFFFFFFFFF),
        throwsA(isA<RangeError>()),
      );
      expect(hasher.hexDigest(), equals(snapBefore));
    });

    test('Class 6: Payload-blind error diagnostics (Standing Rule #20)', () {
      try {
        encodeBitLength(
          BigInt.parse('E000000000000000', radix: 16),
          BigInt.zero,
        );
        fail('Expected Sha512Exception');
      } on Sha512Exception catch (e) {
        expect(e.code, equals(Sha512ErrorCode.messageTooLong));
        expect(e.message, equals(Sha512Messages.errMessageTooLong));
        expect(e.message.contains('E000'), isFalse);
        expect(e.toString(), equals(Sha512Messages.errMessageTooLong));
      }

      for (final code in Sha512ErrorCode.values) {
        expect(Sha512Messages.messageFor(code), isNotEmpty);
      }
      expect(
        Sha512Messages.messageFor(null),
        equals(Sha512Messages.errUnknown),
      );
    });
  });

  // ─── Deterministic PRNG Property Tests and Fuzzing ──────

  group('Deterministic PRNG Property Tests and Fuzzing', () {
    test(
      'Multi-chunk streaming equivalence vs one-shot across diverse lengths',
      () {
        final rng = Random(0x5EED512);
        const lengths = [0, 1, 16, 64, 127, 128, 129, 256, 1000, 4096];

        for (final len in lengths) {
          final data = Uint8List(len);
          for (var i = 0; i < len; i++) {
            data[i] = rng.nextInt(256);
          }
          final expected = sha512Hex(data);

          final hasher = Sha512Hasher();
          var offset = 0;
          while (offset < len) {
            final chunkSize = min(len - offset, rng.nextInt(64) + 1);
            hasher.update(data, offset, chunkSize);
            offset += chunkSize;
          }
          expect(
            hasher.hexDigest(),
            equals(expected),
            reason: 'Mismatch on length $len',
          );
        }
      },
    );

    test('10,000-iteration deterministic PRNG chunk-plan fuzz loop', () {
      final rng = Random(0xDEADBEEF512);

      for (var iter = 0; iter < 10000; iter++) {
        final len = rng.nextInt(300);
        final data = Uint8List(len);
        for (var i = 0; i < len; i++) {
          data[i] = rng.nextInt(256);
        }
        final expected = sha512Hex(data);

        final hasher = Sha512Hasher();
        var offset = 0;
        while (offset < len) {
          final chunkSize = min(len - offset, rng.nextInt(50) + 1);
          hasher.update(data, offset, chunkSize);
          offset += chunkSize;
        }
        expect(
          hasher.hexDigest(),
          equals(expected),
          reason: 'Fuzz iteration failed at $iter',
        );
      }
    });

    test('Hasher reset and empty update branches', () {
      final hasher = Sha512Hasher();
      hasher.update([1, 2, 3]);
      hasher.update([4, 5, 6], 0, 0); // zero-length update
      final hexWithData = hasher.hexDigest();
      expect(hexWithData, equals(sha512Hex([1, 2, 3])));

      hasher.reset();
      expect(hasher.hexDigest(), equals(sha512Hex([])));
    });

    test('Word64 operators, equality, and hash code coverage', () {
      const w1 = Word64(0x12345678, 0x9ABCDEF0);
      const w2 = Word64(0x12345678, 0x9ABCDEF0);
      const w3 = Word64(0, 1);

      expect(w1 == w2, isTrue);
      expect(w1 == w3, isFalse);
      expect(w1.hashCode, equals(w2.hashCode));

      // Test shr at boundary conditions
      expect(w1.shr(0), equals(w1));
      expect(w1.shr(32), equals(const Word64(0, 0x12345678)));
      expect(w1.shr(40), equals(const Word64(0, 0x00123456)));

      // Test rotr at boundary conditions
      expect(w1.rotr(0), equals(w1));
      expect(w1.rotr(32), equals(const Word64(0x9ABCDEF0, 0x12345678)));
      expect(w1.rotr(40).hi != 0, isTrue);

      // Test operators
      final sum = w1 + w3;
      expect(sum.lo, equals(0x9ABCDEF1));
      final xorVal = w1 ^ w1;
      expect(xorVal, equals(Word64.zero));
      final andVal = w1 & Word64.zero;
      expect(andVal, equals(Word64.zero));
      final orVal = w1 | Word64.zero;
      expect(orVal, equals(w1));
      final notVal = ~Word64.zero;
      expect(notVal, equals(const Word64(0xFFFFFFFF, 0xFFFFFFFF)));
    });
  });
}
