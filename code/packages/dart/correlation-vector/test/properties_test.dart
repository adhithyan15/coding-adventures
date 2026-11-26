/// Property tests for `coding_adventures_correlation_vector`
/// slice 1, using the in-lane `pbt` package: ID shapes, uniqueness,
/// counter monotonicity, disabled-log laws, history fidelity,
/// deletion blocking, traversal consistency over random DAGs,
/// checked budgets, merge fidelity, and determinism — all with fixed
/// seeds.
library;

import 'package:coding_adventures_correlation_vector/correlation_vector.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

Gen<String> hostileText(int minLen, int maxLen) {
  const pieces = [
    'a',
    'app.ts',
    '5:12',
    'row:42',
    'stage',
    'tag',
    'x',
    '0',
    '9',
    '.',
    ':',
    '-',
    '_',
    '/',
    ' ',
    '\n',
    'é',
    '',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

Origin? maybeOrigin(int pick, String source, String location) =>
    pick == 0 ? null : Origin(source: source, location: location);

/// Runs a deterministic op script: [ops] create/derive/merge into
/// fresh ids, parents chosen by index modulo the live set. Returns
/// the log plus every minted id.
(CVLog, List<String>) runScript(List<List<int>> ops) {
  final log = CVLog();
  final ids = <String>[];
  for (final op in ops) {
    final kind = op[0] % 3;
    if (kind == 0 || ids.isEmpty) {
      ids.add(log
          .create(Origin(source: 's${op[1] % 4}', location: 'l${op[2] % 5}')));
    } else if (kind == 1) {
      ids.add(log.derive(ids[op[1] % ids.length], null));
    } else {
      final a = ids[op[1] % ids.length];
      final b = ids[op[2] % ids.length];
      ids.add(log.merge([a, b], null));
    }
  }
  return (log, ids);
}

bool topoValid(CVLog log, List<CVEntry> lineage) {
  final positions = <String, int>{};
  for (var i = 0; i < lineage.length; i++) {
    positions[lineage[i].id] = i;
  }
  for (final entry in lineage) {
    for (final parent in entry.parentIds) {
      final parentPos = positions[parent];
      // Dangling parents are skipped by lineage; present ones must
      // precede their child.
      if (parentPos != null && parentPos >= positions[entry.id]!) {
        return false;
      }
    }
  }
  return true;
}

void main() {
  group('properties', () {
    test('root id shapes hold', () {
      check(
        'id-shape',
        checkProperty<String>(
          BigInt.from(6401),
          PbtOptions(numTests: 200, seed: BigInt.from(6401)),
          (rng) => hostileText(0, 20).sample(rng),
          (source) {
            final log = CVLog();
            final id = log.create(Origin(source: source, location: 'l'));
            if (!RegExp(r'^[0-9a-f]{8}\.[1-9][0-9]*$').hasMatch(id)) {
              return false;
            }
            final compact = CVLog.compact();
            final cid = compact.create(Origin(source: source, location: 'l'));
            return RegExp(r'^cv1\.[0-9a-f]{16}$').hasMatch(cid);
          },
        ),
      );
    });

    test('minted ids are unique', () {
      check(
        'uniqueness',
        checkProperty<int>(
          BigInt.from(6402),
          PbtOptions(numTests: 50, seed: BigInt.from(6402)),
          (rng) => smallInt(1, 200).sample(rng),
          (count) {
            final log = CVLog();
            final ids = <String>{};
            for (var i = 0; i < count; i++) {
              final id = log
                  .create(Origin(source: 's${i % 7}', location: 'l${i % 11}'));
              if (!ids.add(id)) return false;
            }
            return ids.length == count;
          },
        ),
      );
    });

    test('same-base sequences increment by one', () {
      check(
        'monotonicity',
        checkProperty<int>(
          BigInt.from(6403),
          PbtOptions(numTests: 50, seed: BigInt.from(6403)),
          (rng) => smallInt(1, 50).sample(rng),
          (count) {
            final log = CVLog();
            final origin = Origin(source: 's', location: 'l');
            var last = 0;
            for (var i = 0; i < count; i++) {
              final id = log.create(origin);
              final seq = int.parse(id.split('.')[1]);
              if (seq != last + 1) return false;
              last = seq;
            }
            return true;
          },
        ),
      );
    });

    test('disabled logs mint without storing', () {
      check(
        'disabled-laws',
        checkProperty<int>(
          BigInt.from(6404),
          PbtOptions(numTests: 50, seed: BigInt.from(6404)),
          (rng) => smallInt(1, 50).sample(rng),
          (count) {
            final log = CVLog(enabled: false);
            for (var i = 0; i < count; i++) {
              final id = log.create(Origin(source: 's', location: 'l$i'));
              if (id.isEmpty) return false;
              if (log.get(id) != null) return false;
              if (log.history(id).isNotEmpty) return false;
            }
            return log.entries.isEmpty && log.passOrder.isEmpty;
          },
        ),
      );
    });

    test('history preserves contribution order', () {
      check(
        'history-fidelity',
        checkProperty<int>(
          BigInt.from(6405),
          PbtOptions(numTests: 50, seed: BigInt.from(6405)),
          (rng) => smallInt(1, 30).sample(rng),
          (count) {
            final log = CVLog();
            final id = log.create(null);
            for (var i = 0; i < count; i++) {
              log.contribute(id, 'stage$i', 'tag$i', const {});
            }
            final history = log.history(id);
            if (history.length != count) return false;
            for (var i = 0; i < count; i++) {
              if (history[i].source != 'stage$i' || history[i].tag != 'tag$i') {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('deletion blocks contributions', () {
      check(
        'deletion-blocks',
        checkProperty<String>(
          BigInt.from(6406),
          PbtOptions(numTests: 200, seed: BigInt.from(6406)),
          (rng) => hostileText(0, 20).sample(rng),
          (reason) {
            final log = CVLog();
            final id = log.create(null);
            log.delete(id, 'dce', reason, const {});
            try {
              log.contribute(id, 's', 't', const {});
            } on CvError {
              return log.get(id)!.deleted!.reason == reason;
            }
            return false;
          },
        ),
      );
    });

    test('traversals stay consistent over random dags', () {
      check(
        'traversal-consistency',
        checkProperty<List<List<int>>>(
          BigInt.from(6407),
          PbtOptions(numTests: 100, seed: BigInt.from(6407)),
          (rng) =>
              Gen.list(1, 15, Gen.list(3, 3, smallInt(0, 1000000))).sample(rng),
          (ops) {
            final (log, ids) = runScript(ops);
            for (final id in ids) {
              // Ancestors are all stored (scripts only use live
              // parents) and never contain the id itself.
              final ancestors = log.ancestors(id);
              if (ancestors.contains(id)) return false;
              for (final a in ancestors) {
                if (log.get(a) == null) return false;
              }
              // Descendants/ancestors agree in both directions.
              for (final d in log.descendants(id)) {
                if (!log.ancestors(d).contains(id)) return false;
              }
              // Lineage is exactly {id} + ancestors, topo-ordered,
              // self last.
              final lineage = log.lineage(id);
              final lineageIds = [for (final e in lineage) e.id];
              if (lineageIds.isEmpty || lineageIds.last != id) {
                return false;
              }
              final expected = {...ancestors, id};
              if (lineageIds.length != expected.length ||
                  !expected.containsAll(lineageIds)) {
                return false;
              }
              if (!topoValid(log, lineage)) return false;
            }
            return true;
          },
        ),
      );
    });

    test('checked node budget admits exactly n mints', () {
      check(
        'checked-budget',
        checkProperty<int>(
          BigInt.from(6408),
          PbtOptions(numTests: 30, seed: BigInt.from(6408)),
          (rng) => smallInt(0, 20).sample(rng),
          (budget) {
            final log = CVLog.newCheckedCompact(GraphLimits(maxNodes: budget));
            for (var i = 0; i < budget; i++) {
              log.tryCreate(null);
            }
            if (log.entries.length != budget) return false;
            try {
              log.tryCreate(null);
              return false;
            } on CvError catch (e) {
              return e.message.contains('nodes limit') &&
                  log.entries.length == budget;
            }
          },
        ),
      );
    });

    test('merge preserves parent order and duplicates', () {
      check(
        'merge-fidelity',
        checkProperty<List<int>>(
          BigInt.from(6409),
          PbtOptions(numTests: 100, seed: BigInt.from(6409)),
          (rng) => Gen.list(1, 5, smallInt(0, 2)).sample(rng),
          (picks) {
            final log = CVLog();
            final pool = [log.create(null), log.create(null), log.create(null)];
            final parents = [for (final p in picks) pool[p]];
            final merged = log.merge(parents, null);
            final stored = log.get(merged)!.parentIds;
            if (stored.length != parents.length) return false;
            for (var i = 0; i < parents.length; i++) {
              if (stored[i] != parents[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('same script mints identical ids', () {
      check(
        'determinism',
        checkProperty<List<List<int>>>(
          BigInt.from(6410),
          PbtOptions(numTests: 100, seed: BigInt.from(6410)),
          (rng) =>
              Gen.list(1, 15, Gen.list(3, 3, smallInt(0, 1000000))).sample(rng),
          (ops) {
            final (_, idsA) = runScript(ops);
            final (_, idsB) = runScript(ops);
            if (idsA.length != idsB.length) return false;
            for (var i = 0; i < idsA.length; i++) {
              if (idsA[i] != idsB[i]) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
