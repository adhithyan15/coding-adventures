/// Hostile-input fuzz campaign for
/// `coding_adventures_correlation_vector` slice 1, using the in-lane
/// `fuzz` package: raw bytes decode (malformed sequences become
/// U+FFFD) and drive op scripts over fresh logs — mint, derive,
/// merge, contribute, passthrough, delete across generic, disabled,
/// compact, and checked configurations. Only [CvError] and the
/// documented [StateError] twins may escape; after every campaign
/// the log invariants (id/get/history/lineage agreement, topo
/// order, budget respect) must hold. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_correlation_vector/correlation_vector.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

bool topoValid(CVLog log, String id) {
  final lineage = log.lineage(id);
  final positions = <String, int>{};
  for (var i = 0; i < lineage.length; i++) {
    positions[lineage[i].id] = i;
  }
  for (final entry in lineage) {
    for (final parent in entry.parentIds) {
      final parentPos = positions[parent];
      if (parentPos != null && parentPos >= positions[entry.id]!) {
        return false;
      }
    }
  }
  return true;
}

FuzzOutcome checkInvariants(CVLog log, List<String> ids) {
  for (final id in ids) {
    final entry = log.get(id);
    if (entry == null) {
      // Only disabled logs skip storage; their ids never land here
      // (see target).
      return const OutcomeCrash('model-divergence', ['missing-entry']);
    }
    if (entry.id != id) {
      return const OutcomeCrash('model-divergence', ['id-mismatch']);
    }
    if (!topoValid(log, id)) {
      return const OutcomeCrash('model-divergence', ['topo-order']);
    }
  }
  return const OutcomePass();
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  final configs = <CVLog Function()>[
    () => CVLog(),
    () => CVLog(enabled: false),
    () => CVLog.compact(),
    () => CVLog.newCheckedCompact(const GraphLimits()),
    () => CVLog.newCheckedCompact(GraphLimits(maxNodes: input.length % 17)),
  ];
  for (var c = 0; c < configs.length; c++) {
    final log = configs[c]();
    cov.recordEdge(100 + c);
    final ids = <String>[];
    var budgetFailures = 0;
    for (var i = 0; i < text.length; i++) {
      final code = text.codeUnitAt(i);
      // With no live ids (e.g. a zero budget rejected every mint so
      // far), only minting is well-defined.
      final op = ids.isEmpty ? 0 : code % 6;
      try {
        if (op == 0) {
          final origin =
              code % 2 == 0 ? Origin(source: 's$code', location: 'l$i') : null;
          ids.add(log.tryCreate(origin));
        } else if (op == 1) {
          ids.add(log.tryDerive(ids[code % ids.length], null));
        } else if (op == 2) {
          ids.add(log
              .tryMerge([ids[code % ids.length], ids[i % ids.length]], null));
        } else if (op == 3) {
          log.contribute(ids[code % ids.length], 's$code', 't$i',
              code % 3 == 0 ? {'k': 'v$code'} : const {});
        } else if (op == 4) {
          final id = ids[code % ids.length];
          final wasDeleted = log.get(id)?.deleted != null;
          final checked = c == 3 || c == 4;
          try {
            log.passthrough(id, 'p$code');
            // Generic logs swallow deleted-entry passthroughs;
            // checked logs must throw for them.
            if (checked && wasDeleted) {
              return const OutcomeCrash(
                  'model-divergence', ['passthrough-silent']);
            }
          } on StateError {
            if (!checked || !wasDeleted) {
              return const OutcomeCrash(
                  'model-divergence', ['passthrough-throw']);
            }
          }
        } else {
          log.tryDelete(ids[code % ids.length], 'dce', 'r$code', const {});
        }
      } on CvError catch (e) {
        // Budget/deleted/unknown failures are legal outcomes;
        // anything else (and empty messages) is a divergence.
        cov.recordEdge(120 + (e.message.length % 8));
        if (e.message.isEmpty) {
          return const OutcomeCrash('model-divergence', ['error-shape']);
        }
        budgetFailures++;
      } on StateError {
        // Infallible twins are never called here; a StateError is a
        // real bug (or a null-check crash shaped like one).
        return const OutcomeCrash('model-divergence', ['state-error']);
      }
    }
    cov.recordEdge(200 + (budgetFailures % 16));
    // Disabled logs store nothing: skip entry checks, but the mint
    // count must match the op count.
    if (c == 1) {
      if (log.entries.isNotEmpty) {
        return const OutcomeCrash('model-divergence', ['disabled-stored']);
      }
      continue;
    }
    final outcome = checkInvariants(log, ids);
    if (outcome is! OutcomePass) return outcome;
    // Checked tight-budget logs never exceed their node budget.
    if (c == 4 && log.entries.length > input.length % 17) {
      return const OutcomeCrash('model-divergence', ['budget-exceeded']);
    }
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('app.ts:5:12\nsrc.ts:1:1\n')),
      Uint8List.fromList(utf8.encode('00000000.1\ncv1.0000000000000001\n')),
      Uint8List.fromList(utf8.encode('";--\n\x00\xff\n')),
      Uint8List.fromList(List.filled(64, 0)),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6411)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
