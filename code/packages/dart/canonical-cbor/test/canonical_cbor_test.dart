// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

import 'dart:typed_data';

import 'package:coding_adventures_canonical_cbor/canonical_cbor.dart';
import 'package:test/test.dart';

Uint8List hexToBytes(String hex) {
  if (hex.isEmpty) return Uint8List(0);
  final clean = hex.replaceAll(' ', '');
  final len = clean.length ~/ 2;
  final result = Uint8List(len);
  for (var i = 0; i < len; i++) {
    result[i] = int.parse(clean.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return result;
}

String bytesToHex(Uint8List bytes) {
  final buffer = StringBuffer();
  for (final b in bytes) {
    buffer.write(b.toRadixString(16).padLeft(2, '0'));
  }
  return buffer.toString();
}

CborValue buildNestedArray(int depth) {
  CborValue cur = const CborNull();
  for (var i = 0; i < depth; i++) {
    cur = CborArray([cur]);
  }
  return cur;
}

Uint8List buildNestedArrayWire(int depth) {
  final result = Uint8List(depth + 1);
  for (var i = 0; i < depth; i++) {
    result[i] = 0x81;
  }
  result[depth] = 0xF6;
  return result;
}

/// Deterministic Linear Congruential Generator (LCG) for zero-dependency property testing.
class DeterministicPrng {
  int _state;

  DeterministicPrng(int seed) : _state = seed & 0xFFFFFFFF;

  int nextUint32() {
    _state = (1664525 * _state + 1013904223) & 0xFFFFFFFF;
    return _state;
  }

  int nextInt(int max) {
    if (max <= 0) return 0;
    return nextUint32() % max;
  }

  BigInt nextBigInt64() {
    final high = BigInt.from(nextUint32());
    final low = BigInt.from(nextUint32());
    return (high << 32) | low;
  }

  Uint8List nextBytes(int length) {
    final bytes = Uint8List(length);
    for (var i = 0; i < length; i++) {
      bytes[i] = nextUint32() & 0xFF;
    }
    return bytes;
  }
}

void main() {
  group('Value Model, Constructors, and Defensive Slicing', () {
    test('Unsigned constructor and accessor', () {
      final u = CborUnsigned(BigInt.from(42));
      expect(u.kind, equals(CborKind.unsigned));
      expect(u.value, equals(BigInt.from(42)));
      expect(u.toString(), equals('CborUnsigned(42)'));

      final u2 = CborUnsigned.fromInt(100);
      expect(u2.value, equals(BigInt.from(100)));

      expect(() => CborUnsigned.fromInt(-1), throwsA(isA<CborError>()));
      expect(() => CborUnsigned(BigInt.from(-5)), throwsA(isA<CborError>()));
      expect(() => CborUnsigned(maxUint64 + BigInt.one),
          throwsA(isA<CborError>()));
    });

    test('Negative constructor and accessor', () {
      final n = CborNegative(BigInt.from(41)); // -42
      expect(n.kind, equals(CborKind.negative));
      expect(n.value, equals(BigInt.from(41)));
      expect(n.toString(), equals('CborNegative(41)'));

      final n2 = CborNegative.fromInt(0); // -1
      expect(n2.value, equals(BigInt.zero));

      expect(() => CborNegative.fromInt(-1), throwsA(isA<CborError>()));
      expect(() => CborNegative(BigInt.from(-10)), throwsA(isA<CborError>()));
      expect(() => CborNegative(maxUint64 + BigInt.one),
          throwsA(isA<CborError>()));
    });

    test('Bytes defensive copying', () {
      final source = Uint8List.fromList([1, 2, 3, 4]);
      final b = CborBytes(source);
      expect(b.kind, equals(CborKind.bytes));
      expect(b.length, equals(4));

      // Mutate source - internal bytes must remain untouched:
      source[0] = 99;
      expect(b.bytes[0], equals(1));

      // Mutate returned bytes - internal bytes must remain untouched:
      final retrieved = b.bytes;
      retrieved[0] = 88;
      expect(b.bytes[0], equals(1));
    });

    test('Text constructor and scalar validation', () {
      final t = CborText('hello world');
      expect(t.kind, equals(CborKind.text));
      expect(t.text, equals('hello world'));
      expect(t.toString(), equals('CborText("hello world")'));

      // Valid paired surrogate (emoji 😀):
      final emoji = CborText('😀');
      expect(emoji.text, equals('😀'));

      // Lone high surrogate must throw invalidUtf8:
      expect(() => CborText(String.fromCharCode(0xD800)),
          throwsA(isA<CborError>()));
      // Lone low surrogate must throw invalidUtf8:
      expect(() => CborText(String.fromCharCode(0xDC00)),
          throwsA(isA<CborError>()));
      // High surrogate followed by non-surrogate:
      expect(() => CborText('\uD800 '), throwsA(isA<CborError>()));
    });

    test('Array constructor and unmodifiable list', () {
      final items = [CborUnsigned.fromInt(1), const CborBool(true)];
      final arr = CborArray(items);
      expect(arr.kind, equals(CborKind.array));
      expect(arr.length, equals(2));
      expect(arr.items.length, equals(2));

      // External mutation of original list has no effect:
      items.add(const CborNull());
      expect(arr.length, equals(2));

      // Attempting to modify returned items throws:
      expect(() => arr.items.add(const CborNull()),
          throwsA(isA<UnsupportedError>()));
    });

    test('Map and MapEntry constructors', () {
      final entry1 = CborMapEntry(CborText('a'), CborUnsigned.fromInt(1));
      final entry2 = CborMapEntry(CborText('b'), CborUnsigned.fromInt(2));
      final mapVal = CborMap([entry1, entry2]);
      expect(mapVal.kind, equals(CborKind.map));
      expect(mapVal.length, equals(2));

      expect(
          entry1, equals(CborMapEntry(CborText('a'), CborUnsigned.fromInt(1))));
      expect(
          entry1.hashCode,
          equals(
              CborMapEntry(CborText('a'), CborUnsigned.fromInt(1)).hashCode));
      expect(entry1.toString(), contains('CborMapEntry'));
    });

    test('Tag constructor', () {
      final tagVal = CborTag(BigInt.zero, const CborNull());
      expect(tagVal.kind, equals(CborKind.tag));
      expect(tagVal.tag, equals(BigInt.zero));
      expect(tagVal.value, equals(const CborNull()));

      final tag2 = CborTag.fromInt(24, CborUnsigned.fromInt(10));
      expect(tag2.tag, equals(BigInt.from(24)));

      expect(() => CborTag.fromInt(-1, const CborNull()),
          throwsA(isA<CborError>()));
      expect(() => CborTag(maxUint64 + BigInt.one, const CborNull()),
          throwsA(isA<CborError>()));
    });

    test('Bool and Null constructors', () {
      const bTrue = CborBool(true);
      const bFalse = CborBool(false);
      const nullVal = CborNull();

      expect(bTrue.kind, equals(CborKind.boolean));
      expect(bTrue.value, isTrue);
      expect(bFalse.value, isFalse);
      expect(nullVal.kind, equals(CborKind.nullValue));

      expect(bTrue, equals(const CborBool(true)));
      expect(bTrue, isNot(equals(bFalse)));
      expect(bTrue.hashCode, equals(const CborBool(true).hashCode));
      expect(bTrue.toString(), equals('CborBool(true)'));
      expect(bFalse.toString(), equals('CborBool(false)'));
      expect(nullVal, equals(const CborNull()));
      expect(nullVal.hashCode, equals(0));
      expect(nullVal.toString(), equals('CborNull()'));
    });

    test('All toString() and hashCode branches on CborValue implementations',
        () {
      final neg = CborNegative.fromInt(10);
      expect(neg.toString(), equals('CborNegative(10)'));
      expect(neg.hashCode, isNotNull);

      final bytes = CborBytes([1, 2, 3]);
      expect(bytes.toString(), contains('CborBytes'));

      final arr = CborArray([neg]);
      expect(arr.toString(), contains('CborArray'));

      final map = CborMap([CborMapEntry(CborText('k'), neg)]);
      expect(map.toString(), contains('CborMap'));

      final tag = CborTag.fromInt(42, neg);
      expect(tag.toString(), contains('CborTag'));
      expect(tag.hashCode, isNotNull);

      const span1 = Span(0, 10);
      const span2 = Span(1, 10);
      const span3 = Span(0, 9);
      expect(span1 == span2, isFalse);
      expect(span1 == span3, isFalse);
      expect(span1 == 'not a span', isFalse);

      const resNonMap = DecodeMapSpansResult.nonMap();
      expect(resNonMap.hashCode, isNotNull);
      final resMap = DecodeMapSpansResult.map([
        SpannedMapEntry(
          key: CborText('a'),
          value: neg,
          valueSpan: span1,
        )
      ]);
      expect(resMap.hashCode, isNotNull);
      expect(resMap == resNonMap, isFalse);
      expect(resMap == 'string', isFalse);

      final resMapDiff = DecodeMapSpansResult.map([
        SpannedMapEntry(
          key: CborText('b'),
          value: neg,
          valueSpan: span1,
        )
      ]);
      expect(resMap == resMapDiff, isFalse);
    });

    test('Structural equality and hash codes across types', () {
      final u1 = CborUnsigned.fromInt(42);
      final u2 = CborUnsigned.fromInt(42);
      final u3 = CborUnsigned.fromInt(43);
      expect(u1, equals(u2));
      expect(u1.hashCode, equals(u2.hashCode));
      expect(u1, isNot(equals(u3)));
      expect(u1, isNot(equals(CborNegative.fromInt(42))));

      final bytes1 = CborBytes([1, 2, 3]);
      final bytes2 = CborBytes([1, 2, 3]);
      final bytes3 = CborBytes([1, 2, 4]);
      expect(bytes1, equals(bytes2));
      expect(bytes1.hashCode, equals(bytes2.hashCode));
      expect(bytes1, isNot(equals(bytes3)));

      final arr1 = CborArray([u1, bytes1]);
      final arr2 = CborArray([u2, bytes2]);
      expect(arr1, equals(arr2));
      expect(arr1.hashCode, equals(arr2.hashCode));

      final map1 = CborMap([CborMapEntry(u1, bytes1)]);
      final map2 = CborMap([CborMapEntry(u2, bytes2)]);
      expect(map1, equals(map2));
      expect(map1.hashCode, equals(map2.hashCode));
    });
  });

  group('Error Diagnostics & Clean Code', () {
    test('All 14 error IDs produce stable payload-blind diagnostics', () {
      for (final id in CborErrorId.values) {
        final err = CborError(id);
        expect(err.id, equals(id));
        expect(err.toString(), startsWith('canonical-cbor: '));
        expect(err, equals(CborError(id)));
        expect(err.hashCode, equals(id.hashCode));
      }
    });

    test('Span and SpannedMapEntry methods and redaction', () {
      const span = Span(5, 15);
      expect(span.start, equals(5));
      expect(span.end, equals(15));
      expect(span.length, equals(10));
      expect(span.toString(), equals('[5, 15)'));
      expect(span, equals(const Span(5, 15)));
      expect(span.hashCode, equals(const Span(5, 15).hashCode));

      final entry = SpannedMapEntry(
        key: CborText('SECRET_KEY'),
        value: CborText('SECRET_PASSWORD'),
        valueSpan: span,
      );
      final formatted = entry.toString();
      expect(formatted, contains('<redacted>'));
      expect(formatted, isNot(contains('SECRET_KEY')));
      expect(formatted, isNot(contains('SECRET_PASSWORD')));
      expect(
          entry,
          equals(SpannedMapEntry(
            key: CborText('SECRET_KEY'),
            value: CborText('SECRET_PASSWORD'),
            valueSpan: span,
          )));
      expect(entry.hashCode, isNotNull);
    });

    test('DecodeMapSpansResult equality and formatting', () {
      const resNonMap1 = DecodeMapSpansResult.nonMap();
      const resNonMap2 = DecodeMapSpansResult.nonMap();
      expect(resNonMap1, equals(resNonMap2));
      expect(resNonMap1.isMap, isFalse);
      expect(resNonMap1.entries, isNull);

      final resMap1 = DecodeMapSpansResult.map([
        SpannedMapEntry(
          key: CborUnsigned.fromInt(1),
          value: const CborBool(true),
          valueSpan: const Span(1, 2),
        ),
      ]);
      final resMap2 = DecodeMapSpansResult.map([
        SpannedMapEntry(
          key: CborUnsigned.fromInt(1),
          value: const CborBool(true),
          valueSpan: const Span(1, 2),
        ),
      ]);
      expect(resMap1, equals(resMap2));
      expect(resMap1.isMap, isTrue);
      expect(resMap1.entries!.length, equals(1));
    });
  });

  group('Convenience Wrappers & Atomic Rollback', () {
    test('encode and encodeInto convenience wrappers', () {
      final val = CborUnsigned.fromInt(42);
      final bytes = encode(val);
      expect(bytes, equals(Uint8List.fromList([0x18, 0x2A])));

      final dst = <int>[0xAA, 0xBB];
      encodeInto(val, dst);
      expect(dst, equals([0xAA, 0xBB, 0x18, 0x2A]));
    });

    test('encodeIntoChecked atomic rollback on failure', () {
      final dst = <int>[0x01, 0x02, 0x03];
      final badVal = buildNestedArray(129); // Exceeds maxNestingDepth

      expect(() => encodeIntoChecked(badVal, dst), throwsA(isA<CborError>()));
      // Destination buffer must remain 100% untouched:
      expect(dst, equals([0x01, 0x02, 0x03]));
    });
  });

  group('Integer and Argument Boundary Transitions', () {
    test(
        'Unsigned transitions: 23/24, 255/256, 65535/65536, 2^32-1/2^32, 2^64-1',
        () {
      final transitions = [
        (BigInt.zero, '00'),
        (BigInt.from(23), '17'),
        (BigInt.from(24), '1818'),
        (BigInt.from(255), '18ff'),
        (BigInt.from(256), '190100'),
        (BigInt.from(65535), '19ffff'),
        (BigInt.from(65536), '1a00010000'),
        (BigInt.from(0xFFFFFFFF), '1affffffff'),
        (BigInt.from(0x100000000), '1b0000000100000000'),
        (maxUint64, '1bffffffffffffffff'),
      ];

      for (final (num, hex) in transitions) {
        final val = CborUnsigned(num);
        final enc = encodeChecked(val);
        expect(bytesToHex(enc), equals(hex));
        final dec = decode(enc) as CborUnsigned;
        expect(dec.value, equals(num));
      }
    });

    test('Negative transitions: -1/-24, -25/-256, -257/-65536', () {
      final transitions = [
        (BigInt.zero, '20'), // -1
        (BigInt.from(23), '37'), // -24
        (BigInt.from(24), '3818'), // -25
        (BigInt.from(255), '38ff'), // -256
        (BigInt.from(256), '390100'), // -257
        (BigInt.from(65535), '39ffff'),
        (BigInt.from(65536), '3a00010000'),
      ];

      for (final (num, hex) in transitions) {
        final val = CborNegative(num);
        final enc = encodeChecked(val);
        expect(bytesToHex(enc), equals(hex));
        final dec = decode(enc) as CborNegative;
        expect(dec.value, equals(num));
      }
    });
  });

  group('Nesting Depth and Size Transitions', () {
    test('Depth: 127 and 128 accepted, 129 rejected', () {
      // Depth 127:
      final d127 = buildNestedArray(127);
      expect(encodeChecked(d127).length, equals(128));
      expect(decode(encodeChecked(d127)), equals(d127));

      // Depth 128 (maximum allowed):
      final d128 = buildNestedArray(128);
      expect(encodeChecked(d128).length, equals(129));
      expect(decode(encodeChecked(d128)), equals(d128));

      // Depth 129 (rejected):
      final d129 = buildNestedArray(129);
      expect(
          () => encodeChecked(d129),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.encodeTooDeep)));
      expect(
          () => decode(buildNestedArrayWire(129)),
          throwsA(
              predicate((e) => e is CborError && e.id == CborErrorId.tooDeep)));
    });

    test('Size limit transitions: 1048575, 1048576, 1048577 bytes', () {
      // 1,048,575 bytes total (5-byte header + 1,048,570 payload):
      final b1048570 = CborBytes(Uint8List(1048570));
      final enc1 = encodeChecked(b1048570);
      expect(enc1.length, equals(1048575));

      // Exactly 1,048,576 bytes total (5-byte header + 1,048,571 payload):
      final b1048571 = CborBytes(Uint8List(1048571));
      final enc2 = encodeChecked(b1048571);
      expect(enc2.length, equals(1048576));

      // 1,048,577 bytes total (5-byte header + 1,048,572 payload) -> encodeTooLarge:
      final b1048572 = CborBytes(Uint8List(1048572));
      expect(
          () => encodeChecked(b1048572),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.encodeTooLarge)));
    });
  });

  group('DecodeMapSpans & Pass-Through Invariant', () {
    test('Valid map with diverse entries reports accurate byte spans', () {
      final mapVal = CborMap([
        CborMapEntry(CborText('alpha'), CborUnsigned.fromInt(100)),
        CborMapEntry(CborText('beta'), CborBytes([10, 20, 30])),
        CborMapEntry(CborText('gamma'),
            CborArray([const CborBool(true), const CborNull()])),
      ]);

      final wire = encodeChecked(mapVal);
      final res = decodeMapSpans(wire);
      expect(res.isMap, isTrue);
      final entries = res.entries!;
      expect(entries.length, equals(3));

      // Check invariant: wire.sublist(start, end) == encodeChecked(entry.value)
      for (final entry in entries) {
        final spanBytes =
            wire.sublist(entry.valueSpan.start, entry.valueSpan.end);
        final expected = encodeChecked(entry.value);
        expect(spanBytes, equals(expected));
      }
    });

    test('Valid non-map document returns isMap == false without error', () {
      final nonMapValues = [
        CborUnsigned.fromInt(1),
        CborText('not a map'),
        CborArray([CborUnsigned.fromInt(1), CborUnsigned.fromInt(2)]),
        const CborBool(false),
      ];

      for (final val in nonMapValues) {
        final wire = encodeChecked(val);
        final res = decodeMapSpans(wire);
        expect(res.isMap, isFalse);
        expect(res.entries, isNull);
      }
    });

    test('Errors in decodeMapSpans throw corresponding CborError', () {
      // Empty input:
      expect(
          () => decodeMapSpans(Uint8List(0)),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.unexpectedEof)));

      // Trailing bytes after map:
      final mapWire = encodeChecked(CborMap([]));
      final trailing = Uint8List.fromList([...mapWire, 0x00]);
      expect(
          () => decodeMapSpans(trailing),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.trailingBytes)));

      // Out of order map keys: {"b": 1, "a": 2}
      final outOfOrder = hexToBytes('a2616201616102');
      expect(
          () => decodeMapSpans(outOfOrder),
          throwsA(predicate((e) =>
              e is CborError && e.id == CborErrorId.nonCanonicalMapOrder)));
    });

    test('CBR01 pass-through rationale: map > 1 MiB decodes spans', () {
      // Child value: 1,048,570 payload bytes (5-byte header -> 1,048,575 bytes <= maxEncodedBytes)
      final child = CborBytes(Uint8List(1048570));
      final childEncoded = encodeChecked(child);
      expect(childEncoded.length, equals(1048575));

      // Construct outer map wire:
      // 0xA1 (map of 1) + 0x00 (key 0) + childEncoded (1,048,575 bytes) = 1,048,577 bytes!
      final outerWire = Uint8List(1048577);
      outerWire[0] = 0xA1;
      outerWire[1] = 0x00;
      outerWire.setRange(2, 1048577, childEncoded);

      // 1. decodeMapSpans must succeed on the >1 MiB wire:
      final res = decodeMapSpans(outerWire);
      expect(res.isMap, isTrue);
      expect(res.entries!.length, equals(1));

      final entry = res.entries![0];
      expect(entry.valueSpan.start, equals(2));
      expect(entry.valueSpan.end, equals(1048577));

      // 2. EncodeChecked on outer map must fail with encodeTooLarge:
      final outerMap = CborMap([CborMapEntry(entry.key, entry.value)]);
      expect(
          () => encodeChecked(outerMap),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.encodeTooLarge)));

      // 3. But EncodeChecked on the child value succeeds, and b[s] == encodeChecked(v):
      final childReEncoded = encodeChecked(entry.value);
      final spanBytes =
          outerWire.sublist(entry.valueSpan.start, entry.valueSpan.end);
      expect(spanBytes, equals(childReEncoded));
    });
  });

  group('Non-Minimal Header Widening Rejection Across Major Types', () {
    test('Non-minimal headers across major types 0..6 are strictly rejected',
        () {
      final widenedHeaders = [
        // Major 0 (unsigned 0 widened to 1/2/4/8 bytes):
        hexToBytes('1800'),
        hexToBytes('190000'),
        hexToBytes('1a00000000'),
        hexToBytes('1b0000000000000000'),
        // Major 1 (negative -1 widened to 1/2/4/8 bytes):
        hexToBytes('3800'),
        hexToBytes('390000'),
        hexToBytes('3a00000000'),
        hexToBytes('3b0000000000000000'),
        // Major 2 (bytes length 0 widened):
        hexToBytes('5800'),
        hexToBytes('590000'),
        hexToBytes('5a00000000'),
        hexToBytes('5b0000000000000000'),
        // Major 3 (text length 0 widened):
        hexToBytes('7800'),
        hexToBytes('790000'),
        hexToBytes('7a00000000'),
        hexToBytes('7b0000000000000000'),
        // Major 4 (array count 0 widened):
        hexToBytes('9800'),
        hexToBytes('990000'),
        hexToBytes('9a00000000'),
        hexToBytes('9b0000000000000000'),
        // Major 5 (map count 0 widened):
        hexToBytes('b800'),
        hexToBytes('b90000'),
        hexToBytes('ba00000000'),
        hexToBytes('bb0000000000000000'),
        // Major 6 (tag 0 widened):
        hexToBytes('d800f6'),
        hexToBytes('d90000f6'),
        hexToBytes('da00000000f6'),
        hexToBytes('db0000000000000000f6'),
      ];

      for (final wire in widenedHeaders) {
        expect(
            () => decode(wire),
            throwsA(predicate((e) =>
                e is CborError && e.id == CborErrorId.nonMinimalInteger)));
      }
    });
  });

  group('CVE Resilience Showcase', () {
    test('CVE-1: Integer overflow and 64-bit wraparound in length math', () {
      final overflowVectors = [
        hexToBytes('5bffffffffffffffff'), // Major 2 length 2^64-1
        hexToBytes('7bffffffffffffffff'), // Major 3 length 2^64-1
        hexToBytes('9bffffffffffffffff'), // Major 4 count 2^64-1
        hexToBytes('bbffffffffffffffff'), // Major 5 count 2^64-1
        hexToBytes('ba40000001'), // Major 5 count > maxInt / 2
      ];

      for (final vec in overflowVectors) {
        expect(
            () => decode(vec),
            throwsA(predicate(
                (e) => e is CborError && e.id == CborErrorId.lengthTooLarge)));
      }
    });

    test('CVE-2: Out-Of-Memory (OOM) declared length bounds checking', () {
      final oomVectors = [
        hexToBytes('9a3b9aca00'), // 1 billion element array in 5 bytes
        hexToBytes('ba3b9aca00'), // 1 billion entry map in 5 bytes
        hexToBytes('5a05f5e100'), // 100 million byte string in 5 bytes
        hexToBytes('7a05f5e100'), // 100 million char text in 5 bytes
      ];

      for (final vec in oomVectors) {
        expect(
            () => decode(vec),
            throwsA(predicate(
                (e) => e is CborError && e.id == CborErrorId.lengthTooLarge)));
      }
    });

    test('CVE-3: Stack-overflow via deep nesting', () {
      final attackWire = buildNestedArrayWire(129);
      expect(
          () => decode(attackWire),
          throwsA(
              predicate((e) => e is CborError && e.id == CborErrorId.tooDeep)));

      final attackVal = buildNestedArray(129);
      expect(
          () => encodeChecked(attackVal),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.encodeTooDeep)));
    });

    test(
        'CVE-4: Signature bypass via non-minimal integers and duplicate/out-of-order keys',
        () {
      // Non-minimal integer:
      expect(
          () => decode(hexToBytes('1800')),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.nonMinimalInteger)));

      // Out of order map keys {"b": 1, "a": 2}:
      expect(
          () => decode(hexToBytes('a2616201616102')),
          throwsA(predicate((e) =>
              e is CborError && e.id == CborErrorId.nonCanonicalMapOrder)));

      // Duplicate map keys {"a": 1, "a": 2}:
      expect(
          () => decode(hexToBytes('a2616101616102')),
          throwsA(predicate((e) =>
              e is CborError && e.id == CborErrorId.nonCanonicalMapOrder)));

      // Duplicate map keys rejected on encode:
      final dupMap = CborMap([
        CborMapEntry(CborText('role'), CborText('guest')),
        CborMapEntry(CborText('role'), CborText('admin')),
      ]);
      expect(
          () => encodeChecked(dupMap),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.duplicateMapKey)));
    });

    test(
        'CVE-5: UTF-8 smuggling and Dart lone-surrogate U+FFFD collision defense',
        () {
      // Decode rejections:
      final smugglingVectors = [
        hexToBytes('62c080'), // Overlong 2-byte NUL
        hexToBytes('63e080af'), // Overlong 3-byte slash
        hexToBytes('64f08080af'), // Overlong 4-byte slash
        hexToBytes('63eda080'), // Surrogate U+D800
        hexToBytes('63edbfbf'), // Surrogate U+DFFF
        hexToBytes('61ff'), // Invalid 0xFF byte
      ];

      for (final wire in smugglingVectors) {
        expect(
            () => decode(wire),
            throwsA(predicate(
                (e) => e is CborError && e.id == CborErrorId.invalidUtf8)));
      }

      // Encode rejections: ensure lone surrogates throw invalidUtf8 with atomic rollback
      final invalidStrings = [
        String.fromCharCode(0xD800), // Lone high surrogate
        String.fromCharCode(0xDFFF), // Lone low surrogate
        '\uD800abc', // High surrogate followed by ASCII
        'abc\uDC00', // ASCII followed by low surrogate
      ];

      for (final str in invalidStrings) {
        expect(
            () => CborText(str),
            throwsA(predicate(
                (e) => e is CborError && e.id == CborErrorId.invalidUtf8)));

        final dst = <int>[0xDE, 0xAD];
        // Test that encodeIntoChecked on any invalid text rolls back:
        expect(() {
          try {
            final t = CborText(str);
            encodeIntoChecked(t, dst);
          } catch (_) {
            rethrow;
          }
        },
            throwsA(predicate(
                (e) => e is CborError && e.id == CborErrorId.invalidUtf8)));
        expect(dst, equals([0xDE, 0xAD]));
      }
    });

    test('CVE-6: Trailing-byte smuggling and stream confusion', () {
      final validItem = hexToBytes('01'); // unsigned 1
      final smuggled =
          Uint8List.fromList([...validItem, 0xDE, 0xAD, 0xBE, 0xEF]);
      expect(
          () => decode(smuggled),
          throwsA(predicate(
              (e) => e is CborError && e.id == CborErrorId.trailingBytes)));
    });

    test(
        'CVE-7: Secret and payload leakage in error diagnostics and toString()',
        () {
      const confidentialToken = 'SUPER_SECRET_AUTHENTICATION_KEY_12345';
      final badMap = CborMap([
        CborMapEntry(CborText(confidentialToken), CborUnsigned.fromInt(1)),
        CborMapEntry(CborText(confidentialToken), CborUnsigned.fromInt(2)),
      ]);

      try {
        encodeChecked(badMap);
        fail('expected encodeChecked to throw');
      } on CborError catch (e) {
        final msg = e.toString();
        expect(msg, isNot(contains(confidentialToken)));
        expect(msg, startsWith('canonical-cbor: '));
      }

      // SpannedMapEntry toString() redaction:
      final entry = SpannedMapEntry(
        key: CborText(confidentialToken),
        value: CborText('CONFIDENTIAL_PASSWORD_67890'),
        valueSpan: const Span(2, 10),
      );
      final str = entry.toString();
      expect(str, contains('<redacted>'));
      expect(str, isNot(contains(confidentialToken)));
      expect(str, isNot(contains('CONFIDENTIAL_PASSWORD_67890')));
    });
  });

  group('Property & Model Equivalence Tests', () {
    test(
        'Map permutation invariance: key order does not affect canonical wire bytes',
        () {
      final prng = DeterministicPrng(12345);
      final entries = <CborMapEntry>[
        CborMapEntry(CborText('delta'), CborUnsigned.fromInt(4)),
        CborMapEntry(CborText('alpha'), CborUnsigned.fromInt(1)),
        CborMapEntry(CborText('charlie'), CborUnsigned.fromInt(3)),
        CborMapEntry(CborText('bravo'), CborUnsigned.fromInt(2)),
        CborMapEntry(CborUnsigned.fromInt(10), CborText('num10')),
        CborMapEntry(CborUnsigned.fromInt(1), CborText('num1')),
      ];

      final canonicalEncoding = encodeChecked(CborMap(entries));

      // Test 100 random shuffles of entries:
      for (var iter = 0; iter < 100; iter++) {
        final shuffled = List<CborMapEntry>.from(entries);
        for (var i = shuffled.length - 1; i > 0; i--) {
          final j = prng.nextInt(i + 1);
          final temp = shuffled[i];
          shuffled[i] = shuffled[j];
          shuffled[j] = temp;
        }

        final shuffledEncoding = encodeChecked(CborMap(shuffled));
        expect(shuffledEncoding, equals(canonicalEncoding));
      }
    });

    test('Round-trip bijection across random structured values', () {
      final prng = DeterministicPrng(54321);

      for (var i = 0; i < 500; i++) {
        final randType = prng.nextInt(7);
        CborValue val;
        switch (randType) {
          case 0:
            val = CborUnsigned(prng.nextBigInt64());
            break;
          case 1:
            val = CborNegative(prng.nextBigInt64());
            break;
          case 2:
            val = CborBytes(prng.nextBytes(prng.nextInt(32)));
            break;
          case 3:
            val = CborText('text_${prng.nextUint32()}');
            break;
          case 4:
            val = CborArray([
              CborUnsigned.fromInt(prng.nextInt(100)),
              CborBool(prng.nextInt(2) == 1),
              const CborNull(),
            ]);
            break;
          case 5:
            val = CborTag(BigInt.from(prng.nextInt(100)),
                CborUnsigned.fromInt(prng.nextInt(1000)));
            break;
          default:
            val = CborBool(prng.nextInt(2) == 0);
            break;
        }

        final encoded = encodeChecked(val);
        expect(encoded.length, lessThanOrEqualTo(maxEncodedBytes));
        final decoded = decode(encoded);
        expect(decoded, equals(val));
        final reEncoded = encodeChecked(decoded);
        expect(reEncoded, equals(encoded));
      }
    });

    test('Adversarial byte mutation fuzz loop (10,000 iterations)', () {
      final prng = DeterministicPrng(99999);

      for (var iter = 0; iter < 10000; iter++) {
        final len = prng.nextInt(64);
        final bytes = prng.nextBytes(len);

        try {
          final val = decode(bytes);
          // If decode succeeded:
          if (bytes.length > maxEncodedBytes) {
            expect(
                () => encodeChecked(val),
                throwsA(predicate((e) =>
                    e is CborError && e.id == CborErrorId.encodeTooLarge)));
          } else {
            final reEncoded = encodeChecked(val);
            expect(reEncoded, equals(bytes));
            final val2 = decode(reEncoded);
            expect(val2, equals(val));
          }
        } on CborError catch (e) {
          // Expected rejection: verify stable prefix and closed error id
          expect(e.toString(), startsWith('canonical-cbor: '));
          expect(CborErrorId.values, contains(e.id));
        }
      }
    });
  });
}
