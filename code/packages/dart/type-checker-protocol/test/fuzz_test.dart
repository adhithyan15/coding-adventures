/// Differential fuzz campaign for `coding_adventures_type_checker_protocol`,
/// using the in-lane `fuzz` package: hostile byte strings decode into hook
/// sets and nodes, and the library's dispatch outcome (return value plus
/// diagnostics) must equal an independent model of the dispatch rule on
/// every input. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_type_checker_protocol/coding_adventures_type_checker_protocol.dart';
import 'package:test/test.dart';

class _FNode {
  final String kind;
  _FNode(this.kind);
}

class _FHook {
  final String kind;
  final int behavior;
  final int marker;
  const _FHook(this.kind, this.behavior, this.marker);
}

/// Independent kind normalization: code units (not runes) with regex trim
/// (not index loops). Agrees with the library on every input the decoder
/// can produce (bytes 0-255, hence no surrogates).
String _modelNormalize(String kind) {
  final out = StringBuffer();
  var under = false;
  for (final unit in kind.codeUnits) {
    final ok = (unit >= 0x61 && unit <= 0x7a) ||
        (unit >= 0x41 && unit <= 0x5a) ||
        (unit >= 0x30 && unit <= 0x39);
    if (ok) {
      out.writeCharCode(unit);
      under = false;
    } else if (!under) {
      out.write('_');
      under = true;
    }
  }
  return out.toString().replaceAll(RegExp('^_+|_+\$'), '');
}

/// Independent dispatch model: returns (result, error messages).
(Object?, List<String>) _model(
    String phase, String nodeKind, List<_FHook> hooks) {
  final want = _modelNormalize(nodeKind);
  final errors = <String>[];
  for (final tier in ['$phase:$want', '$phase:*']) {
    for (final hook in hooks) {
      final key = hook.kind == '*'
          ? '$phase:*'
          : '$phase:${_modelNormalize(hook.kind)}';
      if (key != tier) continue;
      if (hook.behavior == 2 || hook.behavior == 3) {
        errors.add('e${hook.marker}');
      }
      if (hook.behavior == 1 || hook.behavior == 3) {
        return (hook.marker, errors);
      }
    }
  }
  return (null, errors);
}

class _FuzzChecker extends GenericTypeChecker<_FNode> {
  final String phase;
  Object? lastResult;

  _FuzzChecker(this.phase, List<_FHook> specs) {
    for (final spec in specs) {
      registerHook(phase, spec.kind, (node, args) {
        if (spec.behavior == 2 || spec.behavior == 3) {
          error('e${spec.marker}', node);
        }
        if (spec.behavior == 1 || spec.behavior == 3) {
          return spec.marker;
        }
        return notHandled();
      });
    }
  }

  @override
  void run(_FNode ast) {
    lastResult = dispatch(phase, ast);
  }

  @override
  String? nodeKind(_FNode node) => node.kind;
}

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield 0), so the decoder itself can never throw.
class _Decoder {
  final Uint8List input;
  final CoverageBitmap cov;
  int cursor = 0;

  _Decoder(this.input, this.cov);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }

  String takeString(int maxLen) {
    final len = takeByte() % (maxLen + 1);
    return String.fromCharCodes([for (var i = 0; i < len; i++) takeByte()]);
  }
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input, cov);
    final phase = d.takeString(5);
    final nodeKind = d.takeString(7);
    final hookCount = d.takeByte() % 5;
    final hooks = <_FHook>[];
    for (var i = 0; i < hookCount; i++) {
      hooks.add(_FHook(
        d.takeString(7),
        d.takeByte() % 4,
        d.takeByte(),
      ));
    }
    cov.recordEdge(hookCount);
    cov.recordEdge(10 + (phase.length % 6));

    final checker = _FuzzChecker(phase, hooks);
    final result = checker.check(_FNode(nodeKind));
    final (wantValue, wantErrors) = _model(phase, nodeKind, hooks);

    final gotErrors =
        result.errors.map((e) => '${e.message}@${e.line},${e.column}').toList();
    final wantRendered = wantErrors.map((m) => '$m@1,1').toList();
    cov.recordEdge(20 + (gotErrors.length % 8));
    if (checker.lastResult != wantValue) {
      return const OutcomeCrash('dispatch-divergence', ['result-mismatch']);
    }
    if (gotErrors.length != wantRendered.length) {
      return const OutcomeCrash(
          'dispatch-divergence', ['error-count-mismatch']);
    }
    for (var i = 0; i < gotErrors.length; i++) {
      if (gotErrors[i] != wantRendered[i]) {
        return const OutcomeCrash(
            'dispatch-divergence', ['error-content-mismatch']);
      }
    }
    if (result.ok != result.errors.isEmpty) {
      return const OutcomeCrash('result-shape', ['ok-flag-mismatch']);
    }
    if (!identical(result.typedAst.kind, nodeKind)) {
      return const OutcomeCrash('result-shape', ['typedast-identity']);
    }
    cov.recordEdge(30);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      // phase 'node', kind 'literal', one answering hook on 'literal'.
      Uint8List.fromList([
        4, 110, 111, 100, 101, // 'node'
        7, 108, 105, 116, 101, 114, 97, 108, // 'literal'
        1, // one hook
        7, 108, 105, 116, 101, 114, 97, 108, 1, 9, // kind, answer 9
      ]),
      // Wildcard-only hook set.
      Uint8List.fromList(
          [4, 110, 111, 100, 101, 3, 120, 121, 122, 1, 1, 42, 1, 5]),
      // Decline chain with errors.
      Uint8List.fromList(
          [1, 112, 2, 107, 49, 3, 2, 107, 49, 2, 1, 2, 107, 49, 3, 2]),
      // Empty-kind and star-kind corners.
      Uint8List.fromList([1, 112, 0, 2, 0, 1, 3, 1, 42, 0, 2]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 31); longer
/// ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(31)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: library agrees with the model, no crashes', () {
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
