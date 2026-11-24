/// Bounded fuzz campaign for `coding_adventures_arithmetic`, using the
/// in-lane `fuzz` package: hostile operands (junk bits, empty and
/// mismatched shapes) go through every operation. Valid inputs must match
/// the [BigInt] oracle with exact-length outputs (no width-driven
/// amplification); invalid inputs must fail with [ArgumentError] and
/// nothing else. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_arithmetic/coding_adventures_arithmetic.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield 0), so the decoder itself can never throw.
class _Decoder {
  final Uint8List input;
  int cursor = 0;

  _Decoder(this.input);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }

  /// A bit list: mostly valid bits, sometimes hostile bytes.
  List<int> takeBits() {
    final len = takeByte() % 17;
    return [
      for (var i = 0; i < len; i++)
        takeByte() % 8 == 0 ? takeByte() : takeByte() % 2
    ];
  }
}

BigInt _fromBits(List<int> bits) {
  var acc = BigInt.zero;
  for (var i = 0; i < bits.length; i++) {
    if (bits[i] == 1) acc |= BigInt.one << i;
  }
  return acc;
}

bool _validBits(List<int> bits) => bits.every((b) => b == 0 || b == 1);

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final a = d.takeBits();
    final b = d.takeBits();
    final op = AluOp.values[d.takeByte() % AluOp.values.length];
    cov.recordEdge(a.length % 8);
    cov.recordEdge(10 + (op.index % 6));

    final shapesOk = a.isNotEmpty && (op == AluOp.not || a.length == b.length);
    final bitsOk = _validBits(a) && (op == AluOp.not || _validBits(b));
    final expectThrow = !shapesOk || !bitsOk;

    AluResult? result;
    try {
      result = alu(a, b, op);
      if (expectThrow) {
        return const OutcomeCrash(
            'missing-rejection', ['alu-accepted-hostile']);
      }
    } on ArgumentError {
      if (!expectThrow) {
        return const OutcomeCrash('spurious-rejection', ['alu-rejected-valid']);
      }
      cov.recordEdge(20);
      return const OutcomePass();
    }
    cov.recordEdge(30);

    // Valid inputs: differential check against BigInt + no amplification.
    final w = a.length;
    if (result.result.length != w) {
      return const OutcomeCrash('amplification', ['result-wrong-length']);
    }
    final mod = BigInt.one << w;
    final av = _fromBits(a);
    final bv = _fromBits(b);
    final rv = _fromBits(result.result);
    final BigInt want;
    switch (op) {
      case AluOp.add:
        want = (av + bv) % mod;
      case AluOp.sub:
        want = (av - bv) % mod;
      case AluOp.and:
        want = av & bv;
      case AluOp.or:
        want = av | bv;
      case AluOp.xor:
        want = av ^ bv;
      case AluOp.not:
        want = ((BigInt.one << w) - BigInt.one) ^ av;
    }
    if (rv != want) {
      return const OutcomeCrash('model-divergence', ['alu-value-mismatch']);
    }
    if (result.zero != (rv == BigInt.zero)) {
      return const OutcomeCrash('model-divergence', ['zero-flag-mismatch']);
    }
    cov.recordEdge(40);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([4, 1, 0, 1, 0, 1, 1, 0, 0, 0]),
      Uint8List.fromList([8, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 1]),
      Uint8List.fromList([3, 7, 7, 7, 5, 200, 100, 44, 4]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 131);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(131)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: oracle agrees, rejections exact, no crashes', () {
      final report = runCampaign();
      expect(report.crashes, isEmpty,
          reason: report.crashes
              .map((c) =>
                  '${c.signature} ${c.kind} ${c.frames} input=${c.minimalInput}')
              .join('; '));
      expect(report.iterations, greaterThan(100));
      expect(report.totalEdges, greaterThan(5));
    });
  });
}
