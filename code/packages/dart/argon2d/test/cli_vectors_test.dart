// Differential tests against the reference C `argon2` CLI (the RFC
// 9106 reference implementation) in Argon2d mode: 60 seeded vectors
// over passwords, salts, and cost-parameter sweeps. Regenerate with
// `python3 test/generate_argon2_vectors.py` (requires the CLI).
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_argon2d/argon2d.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

void main() {
  final fixture = jsonDecode(
    File('test/argon2_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('reference CLI differential', () {
    test('fixture carries 60 seeded vectors', () {
      expect(fixture['seed'], equals(20261128));
      expect(vectors, hasLength(60));
    });

    test('all vectors match the reference CLI', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final got = argon2dHex(
          _unhex(v['password'] as String),
          (v['salt'] as String).codeUnits,
          timeCost: v['time_cost'] as int,
          memoryCost: v['memory_cost'] as int,
          parallelism: v['parallelism'] as int,
          tagLength: v['tag_length'] as int,
        );
        expect(got, equals(v['digest']), reason: 'vector $i');
      }
    });
  });
}
