// Differential tests against CPython's hashlib.pbkdf2_hmac: 90
// seeded vectors (30 per PRF) over passwords, salts, iteration
// counts, and output lengths. Regenerate with
// `python3 test/generate_pbkdf2_vectors.py`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_pbkdf2/pbkdf2.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

Uint8List _derive(
  String prf,
  Uint8List password,
  Uint8List salt,
  int iterations,
  int keyLength,
) {
  // hashlib accepts empty passwords; the lane needs the flag.
  final allowEmpty = password.isEmpty;
  return switch (prf) {
    'sha1' => pbkdf2HmacSha1(password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmpty),
    'sha256' => pbkdf2HmacSha256(password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmpty),
    'sha512' => pbkdf2HmacSha512(password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmpty),
    _ => throw StateError('unknown prf $prf'),
  };
}

void main() {
  final fixture = jsonDecode(
    File('test/pbkdf2_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('hashlib differential', () {
    test('fixture carries 90 diverse seeded vectors', () {
      expect(fixture['seed'], equals(20261130));
      expect(vectors, hasLength(90));
      final dks = <String>{};
      for (final v in vectors) {
        dks.add(v['dk'] as String);
      }
      expect(dks.length, equals(90));
    });

    test('all vectors match hashlib', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final got = _derive(
          v['prf'] as String,
          _unhex(v['password'] as String),
          _unhex(v['salt'] as String),
          v['iterations'] as int,
          v['key_length'] as int,
        );
        expect(_hex(got), equals(v['dk']), reason: 'vector $i');
      }
    });
  });
}
