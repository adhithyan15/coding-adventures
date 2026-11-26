/// Port of the reference `rust/correlation-vector` lib suite (41
/// tests, of which 37 mirror directly — the 4 serialization tests
/// land with serialization in slice 2), plus slice-1 extras for
/// compact IDs, checked limits, defensive copies, and exact
/// traversal orders.
library;

import 'package:coding_adventures_correlation_vector/correlation_vector.dart';
import 'package:test/test.dart';

Origin makeOrigin(String source, String location) =>
    Origin(source: source, location: location);

Map<String, Object?> meta(String key, String val) => {key: val};

void main() {
  group('correlation-vector', () {
    test('root create with origin id format', () {
      final log = CVLog();
      final id = log.create(makeOrigin('app.ts', '5:12'));
      expect('.'.allMatches(id).length, equals(1));
      final parts = id.split('.');
      expect(parts[0].length, equals(8));
      expect(RegExp(r'^[0-9a-f]{8}$').hasMatch(parts[0]), isTrue);
      expect(parts[1], equals('1'));
      // Pinned against hashlib: sha256("app.ts:5:12")[:8].
      expect(parts[0], equals('9fcf3313'));
    });

    test('root create without origin uses zero base', () {
      final log = CVLog();
      expect(log.create(null), startsWith('00000000.'));
    });

    test('root contribute appears in history', () {
      final log = CVLog();
      final id = log.create(null);
      log.contribute(id, 'parser', 'created', const {});
      log.contribute(id, 'scope', 'resolved', meta('binding', 'local:x'));
      final history = log.history(id);
      expect(history.length, equals(2));
      expect(history[0].source, equals('parser'));
      expect(history[0].tag, equals('created'));
      expect(history[1].source, equals('scope'));
      expect(history[1].tag, equals('resolved'));
      expect(history[1].meta['binding'], equals('local:x'));
    });

    test('root passthrough recorded', () {
      final log = CVLog();
      final id = log.create(null);
      log.passthrough(id, 'type_checker');
      final history = log.history(id);
      expect(history.length, equals(1));
      expect(history[0].source, equals('type_checker'));
      expect(history[0].tag, equals('passthrough'));
    });

    test('root delete sets record and blocks contributions', () {
      final log = CVLog();
      final id = log.create(null);
      log.contribute(id, 'stage_a', 'processed', const {});
      log.delete(id, 'dce', 'unreachable', meta('reason', 'no callers'));
      final entry = log.get(id)!;
      expect(entry.deleted, isNotNull);
      expect(entry.deleted!.source, equals('dce'));
      expect(entry.deleted!.reason, equals('unreachable'));
      expect(entry.deleted!.meta['reason'], equals('no callers'));
      try {
        log.contribute(id, 'optimizer', 'tried', const {});
        fail('expected CvError');
      } on CvError catch (e) {
        expect(e.message, contains('deleted'));
      }
    });

    test('root origin stored in entry', () {
      final log = CVLog();
      final id = log.create(const Origin(
          source: 'data.csv',
          location: 'row:42',
          timestamp: '2024-01-15T09:30:00Z',
          meta: {'schema': 'v2'}));
      final stored = log.get(id)!.origin!;
      expect(stored.source, equals('data.csv'));
      expect(stored.location, equals('row:42'));
      expect(stored.timestamp, equals('2024-01-15T09:30:00Z'));
    });

    test('derive two children have parent prefix', () {
      final log = CVLog();
      final parent = log.create(makeOrigin('app.ts', '10:5'));
      final child1 = log.derive(parent, null);
      final child2 = log.derive(parent, null);
      expect(child1, startsWith(parent));
      expect(child2, startsWith(parent));
      expect(child1, isNot(equals(child2)));
    });

    test('derive child sequence numbers', () {
      final log = CVLog();
      final parent = log.create(null);
      expect(log.derive(parent, null), endsWith('.1'));
      expect(log.derive(parent, null), endsWith('.2'));
    });

    test('ancestors of child returns parent', () {
      final log = CVLog();
      final parent = log.create(null);
      final child = log.derive(parent, null);
      expect(log.ancestors(child), equals([parent]));
    });

    test('descendants of parent returns children', () {
      final log = CVLog();
      final parent = log.create(null);
      final child1 = log.derive(parent, null);
      final child2 = log.derive(parent, null);
      expect(log.descendants(parent)..sort(), equals([child1, child2]..sort()));
    });

    test('derived cv has parent id in entry', () {
      final log = CVLog();
      final parent = log.create(null);
      final child = log.derive(parent, null);
      expect(log.get(child)!.parentIds, equals([parent]));
    });

    test('merge three cvs parent ids', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.create(null);
      final c = log.create(null);
      final merged = log.merge([a, b, c], null);
      expect(
          [...log.get(merged)!.parentIds]..sort(), equals([a, b, c]..sort()));
    });

    test('ancestors of merged returns all parents', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.create(null);
      final c = log.create(null);
      final merged = log.merge([a, b, c], null);
      expect(log.ancestors(merged)..sort(), equals([a, b, c]..sort()));
    });

    test('merge without origin uses zero base', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.create(null);
      expect(log.merge([a, b], null), startsWith('00000000.'));
    });

    test('merge with origin uses origin hash', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.create(null);
      final merged = log.merge([a, b], makeOrigin('join_stage', 'col:0-end'));
      expect(merged, isNot(startsWith('00000000.')));
      // Pinned: sha256("join_stage:col:0-end")[:8].
      expect(merged.split('.')[0], equals('cf7ce668'));
    });

    test('deep ancestry chain ancestors nearest first', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.derive(a, null);
      final c = log.derive(b, null);
      final d = log.derive(c, null);
      expect(log.ancestors(d), equals([c, b, a]));
    });

    test('deep ancestry chain lineage oldest first', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.derive(a, null);
      final c = log.derive(b, null);
      final d = log.derive(c, null);
      final lineage = log.lineage(d);
      expect(lineage.length, equals(4));
      expect(lineage[0].id, equals(a));
      expect(lineage[1].id, equals(b));
      expect(lineage[2].id, equals(c));
      expect(lineage[3].id, equals(d));
    });

    test('ancestors of root is empty', () {
      final log = CVLog();
      expect(log.ancestors(log.create(null)), isEmpty);
    });

    test('descendants of leaf is empty', () {
      final log = CVLog();
      final root = log.create(null);
      expect(log.descendants(log.derive(root, null)), isEmpty);
    });

    test('disabled log all ops succeed', () {
      final log = CVLog(enabled: false);
      final root = log.create(null);
      final child = log.derive(root, null);
      final merged = log.merge([root, child], null);
      log.contribute(root, 'stage', 'tag', const {});
      log.passthrough(child, 'inspector');
      log.delete(merged, 'dce', 'no refs', const {});
    });

    test('disabled log ids still generated', () {
      final log = CVLog(enabled: false);
      final id1 = log.create(null);
      final id2 = log.create(null);
      expect(id1, isNotEmpty);
      expect(id2, isNotEmpty);
      expect(id1, isNot(equals(id2)));
    });

    test('disabled log get returns none', () {
      final log = CVLog(enabled: false);
      expect(log.get(log.create(null)), isNull);
    });

    test('disabled log history is empty', () {
      final log = CVLog(enabled: false);
      final id = log.create(null);
      log.contribute(id, 'stage', 'tag', const {});
      expect(log.history(id), isEmpty);
    });

    test('disabled log counters increment', () {
      final log = CVLog(enabled: false);
      log.create(null);
      log.create(null);
      log.setEnabled(true);
      expect(log.create(null), endsWith('.3'));
    });

    test('id uniqueness same origin 10000', () {
      final log = CVLog();
      final ids = <String>{};
      for (var i = 0; i < 10000; i++) {
        final id = log.create(makeOrigin('same_source', '1:1'));
        expect(ids.add(id), isTrue, reason: 'duplicate ID: $id');
      }
      expect(ids.length, equals(10000));
    });

    test('id uniqueness mixed origins no collisions', () {
      final log = CVLog();
      final ids = <String>{};
      for (final source in ['file_a.ts', 'file_b.ts', 'file_c.ts']) {
        for (var line = 1; line <= 100; line++) {
          final id = log.create(makeOrigin(source, '$line:0'));
          expect(ids.add(id), isTrue, reason: 'duplicate ID: $id');
        }
      }
      expect(ids.length, equals(300));
    });

    test('pass order tracks contribution order', () {
      final log = CVLog();
      final id = log.create(null);
      log.contribute(id, 'parser', 'created', const {});
      log.contribute(id, 'scope', 'resolved', const {});
      log.contribute(id, 'parser', 'revisited', const {});
      log.contribute(id, 'dce', 'eliminated', const {});
      expect(log.passOrder, equals(['parser', 'scope', 'dce']));
    });

    test('get unknown id returns none', () {
      expect(CVLog().get('not_a_real_id'), isNull);
    });

    test('history unknown id returns empty', () {
      expect(CVLog().history('ghost_id'), isEmpty);
    });

    test('lineage unknown id returns empty', () {
      expect(CVLog().lineage('ghost_id'), isEmpty);
    });

    test('ancestors unknown id returns empty', () {
      expect(CVLog().ancestors('ghost_id'), isEmpty);
    });

    test('descendants unknown id returns empty', () {
      expect(CVLog().descendants('ghost_id'), isEmpty);
    });

    test('derive from deleted cv is allowed', () {
      final log = CVLog();
      final parent = log.create(null);
      log.delete(parent, 'dce', 'pruned', const {});
      expect(log.derive(parent, null), isNotEmpty);
    });

    test('merge with deleted cv as parent is allowed', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.create(null);
      log.delete(a, 'pruner', 'gone', const {});
      final merged = log.merge([a, b], null);
      expect(log.get(merged)!.parentIds, contains(a));
    });

    test('different origins get different bases', () {
      final log = CVLog();
      final base1 = log.create(makeOrigin('file_a.ts', '1:1')).split('.')[0];
      final base2 = log.create(makeOrigin('file_b.ts', '99:42')).split('.')[0];
      expect(base1, isNot(equals(base2)));
    });

    test('same origin always same base', () {
      final log1 = CVLog();
      final log2 = CVLog();
      final base1 = log1.create(makeOrigin('app.ts', '10:5')).split('.')[0];
      final base2 = log2.create(makeOrigin('app.ts', '10:5')).split('.')[0];
      expect(base1, equals(base2));
      expect(base1, equals('1c2137e9'));
    });

    test('deep derivation id structure', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.derive(a, null);
      final c = log.derive(b, null);
      final d = log.derive(c, null);
      expect('.'.allMatches(a).length, equals(1));
      expect('.'.allMatches(b).length, equals(2));
      expect('.'.allMatches(c).length, equals(3));
      expect('.'.allMatches(d).length, equals(4));
    });

    test('compact allocations share one sequence', () {
      final log = CVLog.compact();
      final a = log.tryCreate(Origin(
          source: 'example.js',
          location: '3:4',
          timestamp: '2026-10-07',
          meta: const {
            'span': {'start': 4, 'end': 8}
          }));
      final b = log.tryDerive(a, null);
      final c = log.tryCreate(null);
      final m = log.tryMerge([b, c, b], null);
      expect(a, equals('cv1.0000000000000001'));
      expect(b, equals('cv1.0000000000000002'));
      expect(c, equals('cv1.0000000000000003'));
      expect(m, equals('cv1.0000000000000004'));
      expect(log.get(m)!.parentIds, equals([b, c, b]));
    });

    test('checked nodes limit rejects the first mint', () {
      final log = CVLog.newCheckedCompact(const GraphLimits(maxNodes: 0));
      try {
        log.tryCreate(null);
        fail('expected CvError');
      } on CvError catch (e) {
        expect(e.message, contains('CV'));
        expect(e.message, contains('nodes limit'));
      }
      expect(log.entries, isEmpty);
    });

    test('checked mint stores entries and counts usage', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      final id = log.tryCreate(makeOrigin('app.ts', '5:12'));
      expect(id, equals('cv1.0000000000000001'));
      expect(log.get(id)!.origin!.source, equals('app.ts'));
      // Rejected ops leave state unchanged.
      final tight = CVLog.newCheckedCompact(const GraphLimits(maxNodes: 1));
      tight.tryCreate(null);
      expect(() => tight.tryCreate(null), throwsA(isA<CvError>()));
      expect(tight.entries.length, equals(1));
    });

    test('checked deletion is permanent', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      final id = log.tryCreate(null);
      log.tryDelete(id, 'dce', 'original reason', const {});
      expect(log.passOrder, equals(['dce']));
      expect(() => log.tryDelete(id, 'replacement', 'wrong reason', const {}),
          throwsA(isA<CvError>()));
      expect(() => log.contribute(id, 'later', 'changed', const {}),
          throwsA(isA<CvError>()));
      expect(() => log.setEnabled(false), throwsA(isA<CvError>()));
      expect(log.get(id)!.deleted!.reason, equals('original reason'));
    });

    test('checked derive requires a stored parent', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      expect(
          () => log.tryDerive('missing', null),
          throwsA(isA<CvError>().having((e) => e.message, 'message',
              contains('unknown checked CV parent'))));
      final parent = log.tryCreate(null);
      expect(log.tryDerive(parent, null), equals('cv1.0000000000000002'));
    });

    test('checked contribute requires a stored live entry', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      expect(
          () => log.contribute('missing', 's', 't', const {}),
          throwsA(isA<CvError>().having((e) => e.message, 'message',
              contains('unknown checked CV identity'))));
      final id = log.tryCreate(null);
      log.tryDelete(id, 'dce', 'gone', const {});
      expect(
          () => log.contribute(id, 's', 't', const {}),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('already deleted'))));
    });

    test('limits validate rejects deep metadata ceiling', () {
      expect(() => const GraphLimits(maxMetadataDepth: 65).validate(),
          throwsA(isA<CvError>()));
      const GraphLimits(maxMetadataDepth: 64).validate();
    });

    test('metadata byte budget pins exact encoding', () {
      Origin origin() =>
          Origin(source: '', location: '', meta: const {'k': 'λ\n\x01'});
      // Two quoted empty strings plus {"k":"λ\n\u0001"} encode to 22
      // bytes (mirrors the reference's exact-count vector).
      final exact = CVLog.newCheckedCompact(const GraphLimits(
          maxMetadataBytes: 22, maxMetadataValues: 2, maxMetadataDepth: 2));
      exact.tryCreate(origin());
      final tight = CVLog.newCheckedCompact(const GraphLimits(
          maxMetadataBytes: 21, maxMetadataValues: 2, maxMetadataDepth: 2));
      expect(() => tight.tryCreate(origin()), throwsA(isA<CvError>()));
      expect(tight.entries, isEmpty);
    });

    test('infallible twins wrap allocation failures', () {
      final log = CVLog.newCheckedCompact(const GraphLimits(maxNodes: 0));
      expect(() => log.create(null), throwsA(isA<StateError>()));
      expect(() => log.derive('p', null), throwsA(isA<StateError>()));
      expect(() => log.merge([], null), throwsA(isA<StateError>()));
      final log2 = CVLog.newCheckedCompact(const GraphLimits());
      final id = log2.tryCreate(null);
      log2.tryDelete(id, 'dce', 'gone', const {});
      expect(() => log2.delete(id, 'dce', 'again', const {}),
          throwsA(isA<StateError>()));
      expect(
          () => log2.passthrough('missing', 's'), throwsA(isA<StateError>()));
    });

    test('returned entries are immutable views', () {
      final log = CVLog();
      final parent = log.create(null);
      final id = log.derive(
          parent,
          Origin(source: 's', location: 'l', meta: {
            'nested': {
              'list': [1]
            }
          }));
      log.contribute(id, 's', 't', const {});
      final entry = log.get(id)!;
      expect(() => entry.parentIds.add('forged'), throwsUnsupportedError);
      expect(
          () => entry.contributions
              .add(const Contribution(source: 'x', tag: 'y')),
          throwsUnsupportedError);
      expect(() => entry.origin!.meta['k'] = 'forged', throwsUnsupportedError);
      expect(() => (entry.origin!.meta['nested'] as Map)['list'] = [],
          throwsUnsupportedError);
      expect(
          () => ((entry.origin!.meta['nested'] as Map)['list'] as List).add(2),
          throwsUnsupportedError);
      final viaLineage = log.lineage(id).firstWhere((e) => e.id == id);
      expect(() => viaLineage.parentIds.add('forged'), throwsUnsupportedError);
      expect(() => log.entries[id]!.parentIds.add('forged'),
          throwsUnsupportedError);
    });

    test('freshly minted entries are immutable too', () {
      final log = CVLog();
      final root = log.create(null);
      final child = log.derive(root, null);
      final merged = log.merge([root, child], null);
      for (final id in [root, child, merged]) {
        expect(
            () => log
                .get(id)!
                .contributions
                .add(const Contribution(source: 'x', tag: 'y')),
            throwsUnsupportedError);
      }
    });

    test('nested caller maps cannot mutate stored provenance', () {
      final log = CVLog();
      final nested = <String, Object?>{
        'list': [1]
      };
      final id = log
          .create(Origin(source: 's', location: 'l', meta: {'nested': nested}));
      (nested['list'] as List).add(2);
      nested['injected'] = true;
      expect(
          log.get(id)!.origin!.meta,
          equals({
            'nested': {
              'list': [1]
            }
          }));
    });

    test('cyclic meta fails closed', () {
      final log = CVLog();
      final selfMap = <String, Object?>{};
      selfMap['self'] = selfMap;
      expect(
          () =>
              log.tryCreate(Origin(source: 's', location: 'l', meta: selfMap)),
          throwsA(isA<CvError>()));
      final selfList = <Object?>[];
      selfList.add(selfList);
      expect(() => log.contribute(log.create(null), 's', 't', {'k': selfList}),
          throwsA(isA<CvError>()));
      final checked = CVLog.newCheckedCompact(const GraphLimits());
      expect(
          () => checked
              .tryCreate(Origin(source: 's', location: 'l', meta: selfMap)),
          throwsA(isA<CvError>()));
    });

    test('shared substructure freezes once and stays linear', () {
      final shared = {
        'list': [1, 2, 3]
      };
      final log = CVLog();
      final id = log.create(
          Origin(source: 's', location: 'l', meta: {'a': shared, 'b': shared}));
      expect(log.get(id)!.origin!.meta['a'],
          equals(log.get(id)!.origin!.meta['b']));
    });

    test('deeply nested meta does not overflow the stack', () {
      Object? nested = 'leaf';
      for (var i = 0; i < 20000; i++) {
        nested = [nested];
      }
      final log = CVLog();
      final id = log
          .tryCreate(Origin(source: 's', location: 'l', meta: {'k': nested}));
      expect(log.get(id), isNotNull);
    });

    test('stored meta is isolated from caller maps', () {
      final log = CVLog();
      final meta = <String, Object?>{'k': 'v'};
      final id = log.create(Origin(source: 's', location: 'l', meta: meta));
      meta['k'] = 'MUTATED';
      meta['injected'] = true;
      expect(log.get(id)!.origin!.meta, equals({'k': 'v'}));
      final cmeta = <String, Object?>{'a': 'b'};
      log.contribute(id, 's', 't', cmeta);
      cmeta['a'] = 'MUTATED';
      expect(log.history(id)[0].meta, equals({'a': 'b'}));
    });

    test('unsupported meta values fail closed', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      expect(
          () => log.tryCreate(Origin(
              source: 's', location: 'l', meta: {'when': DateTime(2024)})),
          throwsA(isA<CvError>()));
      expect(
          () => log.tryCreate(Origin(source: 's', location: 'l', meta: {
                'outer': {1: 'x'}
              })),
          throwsA(isA<CvError>()));
    });

    test('diamond traversals are exact', () {
      final log = CVLog();
      final a = log.create(null);
      final b = log.derive(a, null);
      final c = log.derive(a, null);
      final d = log.merge([b, c], null);
      expect(log.ancestors(d), equals([b, c, a]));
      expect(log.descendants(a), equals([b, c, d]));
      expect([for (final e in log.lineage(d)) e.id], equals([a, b, c, d]));
    });

    test('views are unmodifiable', () {
      final log = CVLog();
      final id = log.create(null);
      expect(() => log.entries['x'] = log.get(id)!, throwsUnsupportedError);
      expect(() => log.passOrder.add('x'), throwsUnsupportedError);
    });

    test('generic ops on missing ids succeed silently', () {
      final log = CVLog();
      log.contribute('ghost', 's', 't', const {});
      log.tryDelete('ghost', 'dce', 'gone', const {});
      log.delete('ghost', 'dce', 'gone', const {});
      log.passthrough('ghost', 's');
      log.tryPassthrough('ghost', 's');
      expect(log.entries, isEmpty);
    });

    test('generic passthrough swallows deleted entries', () {
      final log = CVLog();
      final id = log.create(null);
      log.delete(id, 'dce', 'gone', const {});
      log.passthrough(id, 's');
      expect(log.history(id), isEmpty);
    });

    test('dangling parents degrade gracefully', () {
      final log = CVLog();
      final child = log.derive('ghost-parent', null);
      expect(log.ancestors(child), equals(['ghost-parent']));
      expect([for (final e in log.lineage(child)) e.id], equals([child]));
    });

    test('edges budget rejects wide merges', () {
      final log2 = CVLog.newCheckedCompact(const GraphLimits(maxEdges: 1));
      final p1 = log2.tryCreate(null);
      final p2 = log2.tryCreate(null);
      expect(
          () => log2.tryMerge([p1, p2], null),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('parent edges limit'))));
    });

    test('events budget rejects contributions', () {
      final log = CVLog.newCheckedCompact(const GraphLimits(maxEvents: 0));
      final id = log.tryCreate(null);
      expect(
          () => log.contribute(id, 's', 't', const {}),
          throwsA(isA<CvError>()
              .having((e) => e.message, 'message', contains('events limit'))));
      expect(log.history(id), isEmpty);
    });

    test('work budget rejects on exhaustion', () {
      final log = CVLog.newCheckedCompact(const GraphLimits(maxWork: 0));
      expect(
          () => log.tryCreate(null),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('operation work limit'))));
      expect(log.entries, isEmpty);
    });

    test('values budget rejects rich meta', () {
      final log = CVLog.newCheckedCompact(
          const GraphLimits(maxMetadataValues: 1, maxMetadataDepth: 8));
      expect(
          () => log.tryCreate(
              Origin(source: '', location: '', meta: const {'k': 'v'})),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('metadata values limit'))));
      expect(log.entries, isEmpty);
    });

    test('depth zero rejects any meta', () {
      final log =
          CVLog.newCheckedCompact(const GraphLimits(maxMetadataDepth: 0));
      expect(
          () => log.tryCreate(
              Origin(source: '', location: '', meta: const {'k': 'v'})),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('metadata depth limit'))));
      // But origin-less mints still work.
      expect(log.tryCreate(null), startsWith('cv1.'));
    });

    test('deep meta rejects past the ceiling', () {
      Object? nested = 'leaf';
      for (var i = 0; i < 5; i++) {
        nested = [nested];
      }
      final log =
          CVLog.newCheckedCompact(const GraphLimits(maxMetadataDepth: 4));
      expect(
          () => log
              .tryCreate(Origin(source: '', location: '', meta: {'k': nested})),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('metadata depth limit'))));
    });

    test('meta of every json shape is accounted', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      final id = log.tryCreate(Origin(
          source: 's',
          location: 'l',
          timestamp: '2024-01-15T09:30:00Z',
          meta: {
            'null': null,
            'bool': true,
            'int': 42,
            'list': [1, 'two', false, null],
            'map': {
              'nested': [{}]
            },
            'emptyList': [],
            'emptyMap': {},
          }));
      expect(log.get(id)!.origin!.meta['int'], equals(42));
      log.contribute(id, 's', 't', {
        'strings': ['a', 'b'],
      });
      expect(log.history(id).length, equals(1));
    });

    test('oversized text fails before scanning', () {
      final log =
          CVLog.newCheckedCompact(const GraphLimits(maxMetadataBytes: 4));
      expect(
          () => log.tryCreate(Origin(source: '1234567890', location: '')),
          throwsA(isA<CvError>().having(
              (e) => e.message, 'message', contains('metadata bytes limit'))));
    });

    test('addBounded detects overflow and over-cap', () {
      const maxInt = 9223372036854775807;
      const count = maxInt;
      try {
        addBounded(count, 1, maxInt, 'test');
        fail('expected CvError');
      } on CvError catch (e) {
        expect(e.message, contains('accounting overflow'));
      }
      const three = 3;
      try {
        addBounded(three, 2, 4, 'test');
        fail('expected CvError');
      } on CvError catch (e) {
        expect(e.message, contains('limit exceeded'));
      }
      expect(addBounded(3, 1, 4, 'test'), equals(4));
    });

    test('stringBytes matches serde escaping widths', () {
      expect(stringBytes(''), equals(2));
      expect(stringBytes('k'), equals(3));
      expect(stringBytes('λ\n\x01'), equals(12));
      expect(stringBytes('"\\\n\r\t\b\f'), equals(2 + 7 * 2));
      expect(stringBytes('\x00\x1f'), equals(2 + 6 + 6));
      // Astral pair counts 4 bytes; lone surrogates count as U+FFFD
      // (3 bytes).
      expect(stringBytes('🎉'), equals(2 + 4));
      expect(stringBytes(String.fromCharCode(0xD800)), equals(2 + 3));
      expect(stringBytes(String.fromCharCode(0xDC00)), equals(2 + 3));
      expect(stringBytes('a${String.fromCharCode(0xD800)}b'), equals(2 + 5));
    });

    test('work reserveConversion doubles the spend', () {
      final work = Work(10);
      work.take(3);
      work.reserveConversion();
      expect(work.used, equals(6));
      expect(() => Work(5).take(6), throwsA(isA<CvError>()));
    });

    test('checked passthrough succeeds on live entries', () {
      final log = CVLog.newCheckedCompact(const GraphLimits());
      final id = log.tryCreate(null);
      log.passthrough(id, 'inspector');
      log.tryPassthrough(id, 'inspector2');
      expect(log.history(id).length, equals(2));
      expect(log.passOrder, equals(['inspector', 'inspector2']));
    });

    test('setEnabled toggles generic logs only', () {
      final generic = CVLog();
      generic.setEnabled(false);
      expect(generic.isEnabled, isFalse);
      generic.setEnabled(true);
      expect(generic.isEnabled, isTrue);
      final checked = CVLog.newCheckedCompact(const GraphLimits());
      checked.setEnabled(true);
      expect(checked.isEnabled, isTrue);
    });

    test('same-length maps with different keys differ', () {
      const a = Contribution(source: 's', tag: 't', meta: {'a': 1});
      const b = Contribution(source: 's', tag: 't', meta: {'b': 1});
      expect(a, isNot(equals(b)));
    });

    test('trailing lone surrogate counts as replacement', () {
      expect(stringBytes('a${String.fromCharCode(0xD800)}'), equals(2 + 4));
    });

    test('default limits pin documented values', () {
      const limits = GraphLimits();
      expect(limits.maxNodes, equals(1000000));
      expect(limits.maxEdges, equals(4000000));
      expect(limits.maxEvents, equals(4000000));
      expect(limits.maxMetadataValues, equals(1000000));
      expect(limits.maxMetadataDepth, equals(64));
      expect(limits.maxMetadataBytes, equals(128 * 1024 * 1024));
      expect(limits.maxInputBytes, equals(128 * 1024 * 1024));
      expect(limits.maxWork, equals(64000000));
      expect(limits.maxOutputBytes, equals(512 * 1024 * 1024));
    });

    test('limits and usage compare by value', () {
      expect(const GraphLimits(), equals(const GraphLimits()));
      expect(
          const GraphLimits(), isNot(equals(const GraphLimits(maxNodes: 1))));
      expect(
          const GraphLimits().hashCode, equals(const GraphLimits().hashCode));
      expect(const Usage(), equals(const Usage()));
      expect(const Usage().copyWith(nodes: 1), equals(const Usage(nodes: 1)));
      expect(const Usage(), isNot(equals(const Usage(nodes: 1))));
      expect(const Usage().hashCode, equals(const Usage().hashCode));
    });
  });

  group('types', () {
    test('origin compares and renders by value', () {
      const a = Origin(source: 's', location: 'l');
      const b = Origin(source: 's', location: 'l');
      const c = Origin(source: 's', location: 'other');
      expect(a, equals(b));
      expect(a, isNot(equals(c)));
      expect(a, isNot(equals('not an origin')));
      expect(a.hashCode, equals(b.hashCode));
      expect(a.toString(), equals('Origin(s, l)'));
    });

    test('contribution compares and renders by value', () {
      const a = Contribution(source: 's', tag: 't');
      const b = Contribution(source: 's', tag: 't');
      const c = Contribution(source: 's', tag: 'other');
      expect(a, equals(b));
      expect(a, isNot(equals(c)));
      expect(a.hashCode, equals(b.hashCode));
      expect(a.toString(), equals('Contribution(s, t)'));
    });

    test('deletion record compares and renders by value', () {
      const a = DeletionRecord(source: 's', reason: 'r');
      const b = DeletionRecord(source: 's', reason: 'r');
      const c = DeletionRecord(source: 's', reason: 'other');
      expect(a, equals(b));
      expect(a, isNot(equals(c)));
      expect(a.hashCode, equals(b.hashCode));
      expect(a.toString(), equals('DeletionRecord(s, r)'));
    });

    test('entry compares and renders by value', () {
      const a = CVEntry(
          id: 'x.1',
          parentIds: ['x'],
          origin: Origin(source: 's', location: 'l'),
          contributions: [Contribution(source: 's', tag: 't')],
          deleted: null);
      const b = CVEntry(
          id: 'x.1',
          parentIds: ['x'],
          origin: Origin(source: 's', location: 'l'),
          contributions: [Contribution(source: 's', tag: 't')],
          deleted: null);
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a.toString(), equals('CVEntry(x.1)'));
      expect(
          a,
          isNot(equals(const CVEntry(
              id: 'x.2',
              parentIds: ['x'],
              origin: Origin(source: 's', location: 'l'),
              contributions: [Contribution(source: 's', tag: 't')],
              deleted: null))));
      expect(
          a,
          isNot(equals(const CVEntry(
              id: 'x.1',
              parentIds: [],
              origin: Origin(source: 's', location: 'l'),
              contributions: [Contribution(source: 's', tag: 't')],
              deleted: null))));
      expect(
          a,
          isNot(equals(const CVEntry(
              id: 'x.1',
              parentIds: ['x'],
              origin: null,
              contributions: [Contribution(source: 's', tag: 't')],
              deleted: null))));
      expect(
          a,
          isNot(equals(const CVEntry(
              id: 'x.1',
              parentIds: ['x'],
              origin: Origin(source: 's', location: 'l'),
              contributions: [],
              deleted: null))));
      expect(
          a,
          isNot(equals(const CVEntry(
              id: 'x.1',
              parentIds: ['x'],
              origin: Origin(source: 's', location: 'l'),
              contributions: [Contribution(source: 's', tag: 't')],
              deleted: DeletionRecord(source: 's', reason: 'r')))));
    });

    test('nested meta compares deeply', () {
      const a = Contribution(source: 's', tag: 't', meta: {
        'list': [
          1,
          'two',
          {
            'three': [true, null]
          }
        ],
        'map': {'x': 'y'},
      });
      const b = Contribution(source: 's', tag: 't', meta: {
        'list': [
          1,
          'two',
          {
            'three': [true, null]
          }
        ],
        'map': {'x': 'y'},
      });
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      const shortList = Contribution(source: 's', tag: 't', meta: {
        'list': [1, 'two'],
        'map': {'x': 'y'},
      });
      expect(a, isNot(equals(shortList)));
      const deepDiff = Contribution(source: 's', tag: 't', meta: {
        'list': [
          1,
          'two',
          {
            'three': [false, null]
          }
        ],
        'map': {'x': 'y'},
      });
      expect(a, isNot(equals(deepDiff)));
      const missingKey = Contribution(source: 's', tag: 't', meta: {
        'list': [
          1,
          'two',
          {
            'three': [true, null]
          }
        ],
      });
      expect(a, isNot(equals(missingKey)));
      const extraKey = Contribution(source: 's', tag: 't', meta: {
        'list': [
          1,
          'two',
          {
            'three': [true, null]
          }
        ],
        'map': {'x': 'y'},
        'z': 1,
      });
      expect(a, isNot(equals(extraKey)));
    });

    test('cv error renders its message', () {
      const e = CvError('boom');
      expect(e.message, equals('boom'));
      expect(e.toString(), equals('boom'));
    });
  });
}
