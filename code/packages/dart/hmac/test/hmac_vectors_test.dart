// Differential tests against CPython's hmac module: 200 seeded
// (variant, key, message) vectors over boundary-heavy key and message
// lengths. Regenerate with `python3 test/generate_hmac_vectors.py`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_hmac/hmac.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _tag(String variant, Uint8List key, Uint8List msg) => switch (variant) {
      'md5' => hmacMd5Hex(key, msg),
      'sha1' => hmacSha1Hex(key, msg),
      'sha256' => hmacSha256Hex(key, msg),
      'sha384' => hmacSha384Hex(key, msg),
      'sha512' => hmacSha512Hex(key, msg),
      _ => throw StateError('unknown variant $variant'),
    };

void main() {
  final fixture = jsonDecode(
    File('test/hmac_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('cpython hmac differential', () {
    test('fixture carries 200 seeded vectors', () {
      expect(fixture['seed'], equals(20261127));
      expect(vectors, hasLength(200));
    });

    test('all vectors match cpython', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final got = _tag(
          v['variant'] as String,
          _unhex(v['key'] as String),
          _unhex(v['msg'] as String),
        );
        expect(got, equals(v['tag']), reason: 'vector $i');
      }
    });

    test('segmented sha256/sha384 vectors match one-shot', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final variant = v['variant'] as String;
        if (variant != 'sha256' && variant != 'sha384') continue;
        final key = _unhex(v['key'] as String);
        final msg = _unhex(v['msg'] as String);
        final mid = msg.length ~/ 2;
        final segments = [
          msg.sublist(0, mid),
          <int>[],
          msg.sublist(mid),
        ];
        final Uint8List got;
        final Uint8List want;
        if (variant == 'sha256') {
          got = hmacSha256Segments(key, segments);
          want = hmacSha256(key, msg);
        } else {
          got = hmacSha384Segments(key, segments);
          want = hmacSha384(key, msg);
        }
        expect(got, equals(want), reason: 'vector $i');
      }
    });
  });
}
