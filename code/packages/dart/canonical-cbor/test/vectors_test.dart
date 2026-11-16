// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_canonical_cbor/canonical_cbor.dart';
import 'package:test/test.dart';

const String canonicalCborCasesJson = r'''{
  "schema_version": 1,
  "profile": "rfc8949-section-4.2.3-length-first",
  "limits": {
    "max_nesting_depth": 128,
    "max_encoded_bytes": 1048576
  },
  "error_ids": [
    "unexpected-eof",
    "trailing-bytes",
    "reserved",
    "indefinite",
    "non-minimal-integer",
    "invalid-utf8",
    "non-canonical-map-order",
    "unsupported-simple",
    "float-not-supported",
    "too-deep",
    "length-too-large",
    "duplicate-map-key",
    "encode-too-deep",
    "encode-too-large"
  ],
  "cases": [
    { "id": "cbor-v1-unsigned-zero", "operation": "round-trip", "input": "00", "expected": "00" },
    { "id": "cbor-v1-unsigned-23", "operation": "round-trip", "input": "17", "expected": "17" },
    { "id": "cbor-v1-unsigned-24", "operation": "round-trip", "input": "1818", "expected": "1818" },
    { "id": "cbor-v1-unsigned-255", "operation": "round-trip", "input": "18ff", "expected": "18ff" },
    { "id": "cbor-v1-unsigned-256", "operation": "round-trip", "input": "190100", "expected": "190100" },
    { "id": "cbor-v1-unsigned-65535", "operation": "round-trip", "input": "19ffff", "expected": "19ffff" },
    { "id": "cbor-v1-unsigned-65536", "operation": "round-trip", "input": "1a00010000", "expected": "1a00010000" },
    { "id": "cbor-v1-unsigned-u32max", "operation": "round-trip", "input": "1affffffff", "expected": "1affffffff" },
    { "id": "cbor-v1-unsigned-u32plus1", "operation": "round-trip", "input": "1b0000000100000000", "expected": "1b0000000100000000" },
    { "id": "cbor-v1-unsigned-u64max", "operation": "round-trip", "input": "1bffffffffffffffff", "expected": "1bffffffffffffffff" },
    { "id": "cbor-v1-negative-one", "operation": "round-trip", "input": "20", "expected": "20" },
    { "id": "cbor-v1-negative-24", "operation": "round-trip", "input": "37", "expected": "37" },
    { "id": "cbor-v1-negative-25", "operation": "round-trip", "input": "3818", "expected": "3818" },
    { "id": "cbor-v1-bytes-empty", "operation": "round-trip", "input": "40", "expected": "40" },
    { "id": "cbor-v1-bytes-three", "operation": "round-trip", "input": "430001ff", "expected": "430001ff" },
    { "id": "cbor-v1-text-empty", "operation": "round-trip", "input": "60", "expected": "60" },
    { "id": "cbor-v1-text-utf8", "operation": "round-trip", "input": "63e29883", "expected": "63e29883" },
    { "id": "cbor-v1-array", "operation": "round-trip", "input": "8300f5f6", "expected": "8300f5f6" },
    { "id": "cbor-v1-map-wire", "operation": "round-trip", "input": "a300f460f56161f6", "expected": "a300f460f56161f6" },
    { "id": "cbor-v1-tag-zero", "operation": "round-trip", "input": "c0f6", "expected": "c0f6" },
    { "id": "cbor-v1-tag-24", "operation": "round-trip", "input": "d81800", "expected": "d81800" },
    { "id": "cbor-v1-simple-values", "operation": "round-trip", "input": "83f4f5f6", "expected": "83f4f5f6" },
    { "id": "cbor-v1-map-input-order", "operation": "encode-map", "input": "6162=f6;1818=f5;60=f4;00=01;6161=02", "expected": "a5000160f41818f56161026162f6" },
    { "id": "cbor-v1-depth-128", "operation": "generated-round-trip", "input": "nested-array:128", "expected": "wire:nested-array:128" },
    { "id": "cbor-v1-size-limit", "operation": "generated-round-trip", "input": "bytes-repeat:1048571:00", "expected": "wire:bytes-repeat:1048571:00" },
    { "id": "cbor-v1-empty-input", "operation": "decode-error", "input": "", "expected": "unexpected-eof" },
    { "id": "cbor-v1-truncated-argument", "operation": "decode-error", "input": "1a0001", "expected": "unexpected-eof" },
    { "id": "cbor-v1-trailing", "operation": "decode-error", "input": "0000", "expected": "trailing-bytes" },
    { "id": "cbor-v1-reserved-28", "operation": "decode-error", "input": "1c", "expected": "reserved" },
    { "id": "cbor-v1-reserved-30", "operation": "decode-error", "input": "1e", "expected": "reserved" },
    { "id": "cbor-v1-indefinite-bytes", "operation": "decode-error", "input": "5f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-text", "operation": "decode-error", "input": "7f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-array", "operation": "decode-error", "input": "9f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-map", "operation": "decode-error", "input": "bf", "expected": "indefinite" },
    { "id": "cbor-v1-break", "operation": "decode-error", "input": "ff", "expected": "indefinite" },
    { "id": "cbor-v1-nonminimal-inline", "operation": "decode-error", "input": "1800", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u8", "operation": "decode-error", "input": "1900ff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u16", "operation": "decode-error", "input": "1a0000ffff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u32", "operation": "decode-error", "input": "1b00000000ffffffff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-length", "operation": "decode-error", "input": "5800", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-tag", "operation": "decode-error", "input": "d800f6", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-invalid-utf8", "operation": "decode-error", "input": "61ff", "expected": "invalid-utf8" },
    { "id": "cbor-v1-invalid-utf8-overlong", "operation": "decode-error", "input": "62c080", "expected": "invalid-utf8" },
    { "id": "cbor-v1-map-out-of-order", "operation": "decode-error", "input": "a2616200616101", "expected": "non-canonical-map-order" },
    { "id": "cbor-v1-map-duplicate-wire", "operation": "decode-error", "input": "a2616100616101", "expected": "non-canonical-map-order" },
    { "id": "cbor-v1-unsupported-simple", "operation": "decode-error", "input": "f0", "expected": "unsupported-simple" },
    { "id": "cbor-v1-undefined", "operation": "decode-error", "input": "f7", "expected": "unsupported-simple" },
    { "id": "cbor-v1-float16", "operation": "decode-error", "input": "f90000", "expected": "float-not-supported" },
    { "id": "cbor-v1-float32", "operation": "decode-error", "input": "fa00000000", "expected": "float-not-supported" },
    { "id": "cbor-v1-float64", "operation": "decode-error", "input": "fb0000000000000000", "expected": "float-not-supported" },
    { "id": "cbor-v1-hostile-bytes-length", "operation": "decode-error", "input": "5bffffffffffffffff", "expected": "length-too-large" },
    { "id": "cbor-v1-depth-129-decode", "operation": "decode-error", "input": "nested-array-wire:129", "expected": "too-deep" },
    { "id": "cbor-v1-duplicate-encode", "operation": "encode-error", "input": "duplicate-map-key", "expected": "duplicate-map-key" },
    { "id": "cbor-v1-depth-129-encode", "operation": "encode-error", "input": "nested-array:129", "expected": "encode-too-deep" },
    { "id": "cbor-v1-size-limit-plus-one", "operation": "encode-error", "input": "bytes-repeat:1048572:00", "expected": "encode-too-large" }
  ]
}''';

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
    result[i] = 0x81; // Array of length 1
  }
  result[depth] = 0xF6; // Null
  return result;
}

void main() {
  final fixture = json.decode(canonicalCborCasesJson) as Map<String, dynamic>;
  final cases = fixture['cases'] as List<dynamic>;

  group('CBR01 Conformance Vectors', () {
    test('covers all 55 canonical fixture cases', () {
      expect(cases.length, equals(55));
    });

    for (final rawCase in cases) {
      final caseMap = rawCase as Map<String, dynamic>;
      final id = caseMap['id'] as String;
      final op = caseMap['operation'] as String;
      final input = caseMap['input'] as String;
      final expected = caseMap['expected'] as String;

      test('$id ($op)', () {
        switch (op) {
          case 'round-trip':
            final inBytes = hexToBytes(input);
            final val = decode(inBytes);
            final outBytes = encodeChecked(val);
            expect(bytesToHex(outBytes), equals(expected));
            final val2 = decode(outBytes);
            expect(val2, equals(val));
            break;

          case 'encode-map':
            // input format: "6162=f6;1818=f5;60=f4;00=01;6161=02"
            final pairs = input.split(';');
            final entries = <CborMapEntry>[];
            for (final pair in pairs) {
              final parts = pair.split('=');
              final k = decode(hexToBytes(parts[0]));
              final v = decode(hexToBytes(parts[1]));
              entries.add(CborMapEntry(k, v));
            }
            final mapVal = CborMap(entries);
            final outBytes = encodeChecked(mapVal);
            expect(bytesToHex(outBytes), equals(expected));
            break;

          case 'generated-round-trip':
            if (input == 'nested-array:128') {
              final val = buildNestedArray(128);
              final wire = buildNestedArrayWire(128);
              final enc = encodeChecked(val);
              expect(enc, equals(wire));
              final dec = decode(wire);
              expect(dec, equals(val));
            } else if (input == 'bytes-repeat:1048571:00') {
              final payload = Uint8List(1048571);
              final val = CborBytes(payload);
              final enc = encodeChecked(val);
              expect(enc.length, equals(1048576));
              final dec = decode(enc);
              expect(dec, equals(val));
            } else {
              fail('unknown generated-round-trip input: $input');
            }
            break;

          case 'decode-error':
            Uint8List wire;
            if (input == 'nested-array-wire:129') {
              wire = buildNestedArrayWire(129);
            } else {
              wire = hexToBytes(input);
            }
            try {
              decode(wire);
              fail('expected decode error $expected, but decode succeeded');
            } on CborError catch (e) {
              expect(e.id.value, equals(expected));
              expect(e.toString(), startsWith('canonical-cbor: '));
            }
            break;

          case 'encode-error':
            CborValue val;
            if (input == 'duplicate-map-key') {
              val = CborMap([
                CborMapEntry(CborText('dup'), CborUnsigned.fromInt(1)),
                CborMapEntry(CborText('dup'), CborUnsigned.fromInt(2)),
              ]);
            } else if (input == 'nested-array:129') {
              val = buildNestedArray(129);
            } else if (input == 'bytes-repeat:1048572:00') {
              val = CborBytes(Uint8List(1048572));
            } else {
              fail('unknown encode-error input: $input');
            }
            try {
              encodeChecked(val);
              fail('expected encode error $expected, but encode succeeded');
            } on CborError catch (e) {
              expect(e.id.value, equals(expected));
              expect(e.toString(), startsWith('canonical-cbor: '));
            }
            break;

          default:
            fail('unknown operation: $op');
        }
      });
    }
  });
}
