/// Unit tests for `coding_adventures_capability_cage`.
///
/// Mirrors the Rust reference's inline suites case-for-case (taxonomy
/// incl. the exhaustive fixture check, capability, errors, glob,
/// manifest), then covers every schema-rejection branch, file loading,
/// value semantics, and summary helpers the mirror leaves implicit.
library;

import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';
import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart'
    as rws;
import 'package:test/test.dart';

/// The shared taxonomy fixture, read relative to the package directory
/// where `dart test` runs (same pattern as the uuid lane's corpus test).
Map<String, dynamic> loadTaxonomyFixture() => jsonDecode(
    File('../../../specs/fixtures/capability-security-v1/taxonomy.json')
        .readAsStringSync()) as Map<String, dynamic>;

void main() {
  group('mirror: taxonomy', () {
    test('category round trips', () {
      for (final s in [
        'fs',
        'net',
        'proc',
        'env',
        'ffi',
        'time',
        'stdin',
        'stdout',
        'rand',
        'thread',
        'ui',
      ]) {
        final cat = Category.parse(s)!;
        expect(cat.wire, s);
        expect('$cat', s);
      }
    });

    test('action round trips', () {
      for (final s in [
        'read',
        'write',
        'create',
        'delete',
        'list',
        'connect',
        'listen',
        'dns',
        'exec',
        'fork',
        'signal',
        'call',
        'load',
        'sleep',
        'spawn',
        'message',
        'event',
      ]) {
        final act = Action.parse(s)!;
        expect(act.wire, s);
      }
    });

    test('nondeterminism pairs are exactly the six', () {
      for (final pair in [
        (Category.rand, Action.read),
        (Category.thread, Action.spawn),
        (Category.thread, Action.message),
        (Category.ui, Action.read),
        (Category.ui, Action.write),
        (Category.ui, Action.event),
      ]) {
        expect(isValidCombination(pair.$1, pair.$2), isTrue,
            reason: '${pair.$1}:${pair.$2}');
      }
      // The new actions belong to one category each, and the new
      // categories take none of the old process or clock actions.
      for (final pair in [
        (Category.proc, Action.spawn),
        (Category.net, Action.message),
        (Category.stdout, Action.event),
        (Category.rand, Action.write),
        (Category.thread, Action.fork),
        (Category.thread, Action.exec),
        (Category.ui, Action.message),
        (Category.ui, Action.sleep),
      ]) {
        expect(isValidCombination(pair.$1, pair.$2), isFalse,
            reason: '${pair.$1}:${pair.$2}');
      }
      // No category or action means "all".
      expect(Category.parse('all'), isNull);
      expect(Action.parse('*'), isNull);
    });

    test('unknown category rejected', () {
      expect(Category.parse('network'), isNull);
      expect(Category.parse(''), isNull);
    });

    test('unknown action rejected', () {
      expect(Action.parse('destroy'), isNull);
      expect(Action.parse(''), isNull);
    });

    test('shared fixture exhaustively matches pair table', () {
      final fixture = loadTaxonomyFixture();
      final categories = fixture['categories'] as Map<String, dynamic>;
      final allActions = List<String>.from(fixture['all_actions'] as List);
      // Bijection, both directions: enum growth or renames outside the
      // fixture's vocabulary must fail here (round-1 review finding).
      expect(Category.values.length, categories.length);
      expect(Action.values.length, allActions.length);
      for (final category in Category.values) {
        expect(categories.keys, contains(category.wire));
      }
      for (final action in Action.values) {
        expect(allActions, contains(action.wire));
      }

      var validCount = 0;
      var invalidCount = 0;
      for (final entry in categories.entries) {
        final category = Category.parse(entry.key)!;
        final allowed = (entry.value as List).map((v) => v as String).toSet();
        for (final actionName in allActions) {
          final action = Action.parse(actionName)!;
          final want = allowed.contains(actionName);
          expect(isValidCombination(category, action), want,
              reason: 'pair mismatch for $category:$action');
          if (want) {
            validCount++;
          } else {
            invalidCount++;
          }
        }
      }
      expect(validCount, fixture['expected_valid_pair_count']);
      expect(invalidCount, fixture['expected_invalid_cross_pair_count']);
    });
  });

  group('mirror: capability', () {
    test('valid capability round trips fields', () {
      final cap = Capability(
        category: Category.fs,
        action: Action.read,
        target: './grammars/json.tokens',
        justification: 'load lexer DFA',
      );
      expect(cap.category, Category.fs);
      expect(cap.action, Action.read);
      expect(cap.target, './grammars/json.tokens');
      expect(cap.justification, 'load lexer DFA');
      expect(cap.flavor, isNull);
      expect(cap.trust, isNull);
    });

    test('read write annotations are preserved', () {
      final cap = Capability(
        category: Category.net,
        action: Action.connect,
        target: 'imap.example.test:993',
        justification: 'mail',
      )
          .withFlavor(rws.CapabilityFlavor.ingestion)
          .withTrust(rws.CapabilityTrust.untrusted);
      final converted = cap.toRwsCapability();
      expect(cap.flavor, rws.CapabilityFlavor.ingestion);
      expect(cap.trust, rws.CapabilityTrust.untrusted);
      expect(converted.flavor, rws.CapabilityFlavor.ingestion);
      expect(converted.trust, rws.CapabilityTrust.untrusted);
      expect(converted.justification, 'mail');
    });

    test('invalid pair rejected', () {
      expect(
          () => Capability(
                category: Category.fs,
                action: Action.connect,
                target: 'x',
                justification: 'y',
              ),
          throwsA(const UnsupportedPair(Category.fs, Action.connect)));
    });

    test('empty target rejected', () {
      expect(
          () => Capability(
                category: Category.fs,
                action: Action.read,
                target: '',
                justification: 'y',
              ),
          throwsA(const EmptyTarget()));
    });

    test('capabilities compare by value', () {
      final a = Capability(
          category: Category.fs,
          action: Action.read,
          target: 'x',
          justification: 'j');
      final b = Capability(
          category: Category.fs,
          action: Action.read,
          target: 'x',
          justification: 'j');
      final c = Capability(
          category: Category.fs,
          action: Action.read,
          target: 'y',
          justification: 'j');
      expect(a, equals(b));
      expect(a == c, isFalse);
    });
  });

  group('mirror: errors', () {
    test('capability violation error displays message', () {
      const err = CapabilityViolationError(
        category: Category.fs,
        action: Action.read,
        target: '/etc/passwd',
        message: 'fs:read:/etc/passwd not declared',
      );
      expect(err.toString(), 'fs:read:/etc/passwd not declared');
    });

    test('invalid combination messages', () {
      const pair = UnsupportedPair(Category.fs, Action.connect);
      expect(pair.toString(), contains('fs'));
      expect(pair.toString(), contains('connect'));
      expect(const EmptyTarget().toString(), contains('empty'));
      expect(const InvalidTargetFormat('expected host:port').toString(),
          contains('expected host:port'));
    });

    test('manifest error into chain', () {
      // InvalidCombination converts, preserving the inner error...
      const inner = EmptyTarget();
      const chained = ManifestInvalidCombination(inner);
      expect(chained.inner, inner);
      // ...and I/O failures convert with their detail.
      const io = ManifestIoError('not found');
      expect(io.toString(), contains('not found'));
    });

    test('rws violation displays message', () {
      const err = ManifestRwsViolation(
          untrustedInputs: [], actuations: [], message: 'split agent');
      expect(err.toString(), contains('read/write separation'));
      expect(err.toString(), contains('split agent'));
    });
  });

  group('mirror: glob', () {
    test('star matches everything', () {
      expect(matchTarget('*', 'anything'), isTrue);
      expect(matchTarget('*', '/etc/passwd'), isTrue);
      expect(matchTarget('*', 'api.weather.gov:443'), isTrue);
    });

    test('literal match', () {
      expect(matchTarget('foo', 'foo'), isTrue);
      expect(matchTarget('foo', 'bar'), isFalse);
      expect(matchTarget('foo', 'foobar'), isFalse);
    });

    test('single star one component', () {
      expect(matchTarget('./grammars/*', './grammars/json.tokens'), isTrue);
      expect(
          matchTarget('./grammars/*', './grammars/sub/json.tokens'), isFalse);
    });

    test('double star any depth', () {
      expect(matchTarget('./**/*.tokens', './grammars/json.tokens'), isTrue);
      expect(matchTarget('./**/*.tokens', './grammars/sub/deep/json.tokens'),
          isTrue);
      expect(matchTarget('./**/*.tokens', './grammars/json.json'), isFalse);
    });

    test('extension glob within segment', () {
      expect(matchTarget('*.tokens', 'json.tokens'), isTrue);
      expect(matchTarget('*.tokens', 'json.toml'), isFalse);
      expect(
          matchTarget('./grammars/*.tokens', './grammars/json.tokens'), isTrue);
    });

    test('middle wildcard', () {
      expect(matchTarget('foo*.bar', 'fooXY.bar'), isTrue);
      expect(matchTarget('foo*.bar', 'foo.bar'), isTrue);
      expect(matchTarget('foo*.bar', 'foo.baz'), isFalse);
    });

    test('net target match', () {
      expect(matchTarget('api.weather.gov:443', 'api.weather.gov:443'), isTrue);
      expect(matchTarget('api.weather.gov:443', 'api.weather.gov:80'), isFalse);
      expect(
          matchTarget('api.weather.gov:443', 'evil.example.com:443'), isFalse);
    });

    test('net wildcard port', () {
      expect(matchTarget('api.weather.gov:*', 'api.weather.gov:443'), isTrue);
      expect(matchTarget('api.weather.gov:*', 'api.weather.gov:80'), isTrue);
      expect(matchTarget('api.weather.gov:*', 'evil.example.com:443'), isFalse);
    });

    test('net wildcard host', () {
      expect(matchTarget('*:443', 'api.weather.gov:443'), isTrue);
      expect(matchTarget('*:443', 'evil.example.com:443'), isTrue);
      expect(matchTarget('*:443', 'api.weather.gov:80'), isFalse);
    });

    test('windows paths normalize', () {
      expect(matchTarget('./grammars/*.tokens', '.\\grammars\\json.tokens'),
          isTrue);
    });
  });

  group('mirror: manifest basics', () {
    Capability fsReadCap(String target) => Capability(
        category: Category.fs,
        action: Action.read,
        target: target,
        justification: 'test');

    test('empty manifest has nothing', () {
      final m = Manifest.empty();
      expect(m.has(Category.fs, Action.read, '/anything'), isFalse);
      try {
        m.check(Category.fs, Action.read, '/x');
        fail('check did not throw');
      } on CapabilityViolationError catch (e) {
        expect(e.category, Category.fs);
        expect(e.action, Action.read);
        expect(e.target, '/x');
        expect(e.message, contains('fs:read:/x'));
      }
    });

    test('exact match', () {
      final m = Manifest([fsReadCap('./grammars/json.tokens')]);
      expect(m.has(Category.fs, Action.read, './grammars/json.tokens'), isTrue);
      expect(
          m.has(Category.fs, Action.read, './grammars/other.tokens'), isFalse);
    });

    test('glob match', () {
      final m = Manifest([fsReadCap('./grammars/*.tokens')]);
      expect(m.has(Category.fs, Action.read, './grammars/json.tokens'), isTrue);
      expect(m.has(Category.fs, Action.read, './grammars/yaml.tokens'), isTrue);
      expect(m.has(Category.fs, Action.read, './grammars/sub/json.tokens'),
          isFalse);
    });

    test('category action must match', () {
      final m = Manifest([fsReadCap('./x')]);
      expect(m.has(Category.fs, Action.write, './x'), isFalse);
      expect(m.has(Category.net, Action.connect, './x'), isFalse);
    });

    test('check returns ok on match', () {
      final m = Manifest([fsReadCap('./x')]);
      m.check(Category.fs, Action.read, './x');
    });

    test('load minimal pure manifest', () {
      const json = '''
{
    "version": 1,
    "package": "rust/capability-cage-test",
    "capabilities": [],
    "justification": "pure computation"
}''';
      final m = Manifest.loadFromString(json);
      expect(m.capabilities.length, 0);
    });

    test('load manifest with one cap', () {
      const json = '''
{
    "version": 1,
    "package": "rust/json-lexer",
    "capabilities": [
        {
            "category": "fs",
            "action": "read",
            "target": "./grammars/*.tokens",
            "justification": "load lexer DFA"
        }
    ]
}''';
      final m = Manifest.loadFromString(json);
      expect(m.capabilities.length, 1);
      expect(m.has(Category.fs, Action.read, './grammars/json.tokens'), isTrue);
    });
  });

  group('mirror: manifest summaries', () {
    test('counts manifest shape without targets', () {
      final manifest = Manifest([
        Capability(
                category: Category.fs,
                action: Action.read,
                target: './input/*.json',
                justification: 'x')
            .withFlavor(rws.CapabilityFlavor.ingestion)
            .withTrust(rws.CapabilityTrust.untrusted),
        Capability(
                category: Category.fs,
                action: Action.write,
                target: './out/report.json',
                justification: 'x')
            .withFlavor(rws.CapabilityFlavor.actuation)
            .withTrust(rws.CapabilityTrust.trusted),
        Capability(
            category: Category.env,
            action: Action.read,
            target: 'APP_MODE',
            justification: 'x'),
        Capability(
            category: Category.stdout,
            action: Action.write,
            target: 'stdout',
            justification: 'x'),
      ]);
      expect(
          manifest.capabilitySurfaceSummary(),
          const CapabilitySurfaceSummary(
            totalCapabilities: 4,
            uniqueCategories: 3,
            uniqueActions: 2,
            uniqueTargets: 4,
            filesystemCapabilities: 2,
            environmentCapabilities: 1,
            stdioCapabilities: 1,
            readLikeCapabilities: 2,
            writeLikeCapabilities: 2,
            externalEffectCapabilities: 2,
            ingestionCapabilities: 1,
            actuationCapabilities: 1,
            trustedCapabilities: 1,
            untrustedCapabilities: 1,
            unannotatedCapabilities: 2,
          ));
      final summary = manifest.capabilitySurfaceSummary();
      expect(summary.hasWriteOrActuation, isTrue);
      expect(summary.crossesUntrustedBoundary, isTrue);
      expect(summary.hasNetworkAccess, isFalse);
      expect(summary.hasProcessAccess, isFalse);
      expect(summary.fullyAnnotated, isFalse);
    });

    test('counts nondeterminism categories', () {
      final manifest = Manifest([
        Capability(
            category: Category.rand,
            action: Action.read,
            target: '*',
            justification: 'session keys'),
        Capability(
            category: Category.thread,
            action: Action.spawn,
            target: 'decode-pool',
            justification: 'decode'),
        Capability(
            category: Category.thread,
            action: Action.message,
            target: 'decode-pool',
            justification: 'jobs'),
        Capability(
            category: Category.ui,
            action: Action.event,
            target: '#editor',
            justification: 'keystrokes'),
        Capability(
            category: Category.ui,
            action: Action.write,
            target: '#editor',
            justification: 'render'),
      ]);
      final summary = manifest.capabilitySurfaceSummary();
      expect(summary.totalCapabilities, 5);
      expect(summary.randCapabilities, 1);
      expect(summary.threadCapabilities, 2);
      expect(summary.uiCapabilities, 2);
      // rand:read and ui:event bring data in; spawn, message and
      // ui:write have effects.
      expect(summary.readLikeCapabilities, 2);
      expect(summary.writeLikeCapabilities, 3);
      expect(summary.hasNetworkAccess, isFalse);
      expect(summary.hasProcessAccess, isFalse);
    });

    test('empty summary is safe default', () {
      final summary = Manifest.empty().capabilitySurfaceSummary();
      expect(summary, CapabilitySurfaceSummary.empty());
      expect(summary.isEmpty, isTrue);
      expect(summary.fullyAnnotated, isTrue);
      expect(summary.hasWriteOrActuation, isFalse);
      expect(summary.crossesUntrustedBoundary, isFalse);
    });
  });

  group('mirror: manifest json', () {
    test('nondeterminism categories load', () {
      final manifest = Manifest.loadFromString('''
{
    "version": 1,
    "package": "rust/editor",
    "capabilities": [
        {"category": "rand", "action": "read", "target": "*",
         "justification": "Generates document identifiers."},
        {"category": "thread", "action": "spawn", "target": "spellcheck",
         "justification": "Spell checks off the input thread."},
        {"category": "ui", "action": "event", "target": "#editor",
         "justification": "Receives keystrokes from the editor view."}
    ],
    "justification": "An editor view with a background spell checker."
}''');
      expect(manifest.has(Category.rand, Action.read, '*'), isTrue);
      expect(manifest.has(Category.thread, Action.spawn, 'spellcheck'), isTrue);
      expect(manifest.has(Category.thread, Action.spawn, 'network-poller'),
          isFalse);
      expect(manifest.has(Category.ui, Action.event, '#editor'), isTrue);
      expect(manifest.has(Category.ui, Action.write, '#editor'), isFalse);
    });

    test('rejects nondeterminism cross pairs', () {
      for (final pair in [
        ('proc', 'spawn'),
        ('ui', 'message'),
        ('rand', 'write'),
      ]) {
        final document = '{"version": 1, "package": "rust/x", '
            '"capabilities": [{"category": "${pair.$1}", '
            '"action": "${pair.$2}", "target": "*", '
            '"justification": "cross-pair fixture"}], '
            '"justification": "cross-pair fixture"}';
        expect(() => Manifest.loadFromString(document),
            throwsA(isA<ManifestError>()),
            reason: '${pair.$1}:${pair.$2} must be rejected');
      }
    });

    test('preserves read write annotations', () {
      const json = '''
{
    "version": 1,
    "package": "rust/mail-ingest",
    "capabilities": [
        {
            "category": "net",
            "action": "connect",
            "target": "imap.example.test:993",
            "flavor": "ingestion",
            "trust": "untrusted",
            "justification": "read mail"
        }
    ]
}''';
      final m = Manifest.loadFromString(json);
      final cap = m.capabilities[0];
      expect(cap.flavor, rws.CapabilityFlavor.ingestion);
      expect(cap.trust, rws.CapabilityTrust.untrusted);
      m.validateReadWriteSeparation();
    });

    test('rejects unknown read write annotations', () {
      const json = '''
{
    "version": 1,
    "capabilities": [
        {
            "category": "net",
            "action": "connect",
            "target": "imap.example.test:993",
            "flavor": "read"
        }
    ]
}''';
      try {
        Manifest.loadFromString(json);
        fail('load did not throw');
      } on ManifestSchemaError catch (e) {
        expect(e.reason, contains('flavor'));
      }
    });

    test('rejects mixed untrusted input and actuation', () {
      const json = '''
{
    "version": 1,
    "package": "rust/mail-agent",
    "capabilities": [
        {
            "category": "net",
            "action": "connect",
            "target": "imap.example.test:993",
            "flavor": "ingestion",
            "trust": "untrusted",
            "justification": "read mail"
        },
        {
            "category": "fs",
            "action": "write",
            "target": "package:/state/outbox.json",
            "justification": "write outbox"
        }
    ]
}''';
      try {
        Manifest.loadFromString(json);
        fail('load did not throw');
      } on ManifestRwsViolation catch (e) {
        expect(e.untrustedInputs.length, 1);
        expect(e.actuations.length, 1);
        expect(e.message, contains('untrusted inputs'));
      }
    });

    test('rejects same path read write overlap', () {
      const json = '''
{
    "version": 1,
    "package": "rust/cache-agent",
    "capabilities": [
        {
            "category": "fs",
            "action": "read",
            "target": "package:/state/cache.json",
            "trust": "trusted"
        },
        {
            "category": "fs",
            "action": "write",
            "target": "package:/state/cache.json"
        }
    ]
}''';
      try {
        Manifest.loadFromString(json);
        fail('load did not throw');
      } on ManifestRwsViolation catch (e) {
        expect(e.message, contains('overlap'));
      }
    });

    test('accepts pure actuation default net connect', () {
      const json = '''
{
    "version": 1,
    "package": "rust/mail-send",
    "capabilities": [
        {
            "category": "net",
            "action": "connect",
            "target": "smtp.example.test:465"
        }
    ]
}''';
      final m = Manifest.loadFromString(json);
      expect(m.capabilities.length, 1);
    });

    test('rejects missing version', () {
      try {
        Manifest.loadFromString('{ "capabilities": [] }');
        fail('load did not throw');
      } on ManifestSchemaError catch (e) {
        expect(e.reason, contains('version'));
      }
    });

    test('rejects unknown version', () {
      try {
        Manifest.loadFromString('{ "version": 2, "capabilities": [] }');
        fail('load did not throw');
      } on ManifestSchemaError catch (e) {
        expect(e.reason, contains('version'));
      }
    });

    test('rejects missing capabilities', () {
      try {
        Manifest.loadFromString('{ "version": 1 }');
        fail('load did not throw');
      } on ManifestSchemaError catch (e) {
        expect(e.reason, contains('capabilities'));
      }
    });

    test('rejects capabilities not array', () {
      try {
        Manifest.loadFromString('{ "version": 1, "capabilities": {} }');
        fail('load did not throw');
      } on ManifestSchemaError catch (e) {
        expect(e.reason, contains('array'));
      }
    });

    test('rejects unknown category', () {
      const json = '''
{
    "version": 1,
    "capabilities": [
        { "category": "network", "action": "connect", "target": "x:80" }
    ]
}''';
      try {
        Manifest.loadFromString(json);
        fail('load did not throw');
      } on ManifestSchemaError catch (e) {
        expect(e.reason, contains('category'));
      }
    });

    test('rejects invalid pair', () {
      // fs + connect is not a valid combination.
      const json = '''
{
    "version": 1,
    "capabilities": [
        { "category": "fs", "action": "connect", "target": "x" }
    ]
}''';
      try {
        Manifest.loadFromString(json);
        fail('load did not throw');
      } on ManifestInvalidCombination catch (e) {
        expect(e.inner, const UnsupportedPair(Category.fs, Action.connect));
      }
    });

    test('rejects empty target', () {
      const json = '''
{
    "version": 1,
    "capabilities": [
        { "category": "fs", "action": "read", "target": "" }
    ]
}''';
      try {
        Manifest.loadFromString(json);
        fail('load did not throw');
      } on ManifestInvalidCombination catch (e) {
        expect(e.inner, const EmptyTarget());
      }
    });

    test('rejects unparseable json', () {
      expect(() => Manifest.loadFromString('not json'),
          throwsA(isA<ManifestParseError>()));
    });

    test('checked rejects programmatic rws violations', () {
      final input = Capability(
        category: Category.net,
        action: Action.connect,
        target: 'imap.example.test:993',
        justification: 'read mail',
      )
          .withFlavor(rws.CapabilityFlavor.ingestion)
          .withTrust(rws.CapabilityTrust.untrusted);
      final actuation = Capability(
        category: Category.fs,
        action: Action.write,
        target: 'package:/state/outbox.json',
        justification: 'write',
      );
      expect(() => Manifest.checked([input, actuation]),
          throwsA(isA<ManifestRwsViolation>()));
    });
  });

  group('EXTRA: schema branches', () {
    ManifestSchemaError loadError(String document) {
      try {
        Manifest.loadFromString(document);
      } on ManifestSchemaError catch (e) {
        return e;
      }
      fail('load did not throw for: $document');
    }

    test('top-level non-object rejected', () {
      for (final document in ['[]', '"x"', '1', 'true', 'null']) {
        expect(loadError(document).reason,
            'top-level value must be a JSON object');
      }
    });

    test('non-integer versions rejected', () {
      for (final document in [
        '{"version": "one", "capabilities": []}',
        '{"version": 1.0, "capabilities": []}',
        '{"version": null, "capabilities": []}',
        '{"version": true, "capabilities": []}',
      ]) {
        expect(loadError(document).reason,
            'missing or non-integer field: version');
      }
    });

    test('wrong integer version names itself', () {
      expect(loadError('{ "version": 2, "capabilities": [] }').reason,
          'unsupported manifest version: 2 (only 1 is supported in v1)');
    });

    test('non-object entries rejected', () {
      for (final entry in ['1', '"x"', '[]', 'null', 'true']) {
        final err = loadError('{"version": 1, "capabilities": [$entry]}');
        expect(err.reason, 'capabilities[0] must be a JSON object');
      }
    });

    test('missing or non-string fields rejected', () {
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"action": "read", "target": "x"}]}')
              .reason,
          'capabilities[0] missing string \'category\'');
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "target": "x"}]}')
              .reason,
          'capabilities[0] missing string \'action\'');
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "action": "read"}]}')
              .reason,
          'capabilities[0] missing string \'target\'');
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": 7}]}')
              .reason,
          'capabilities[0] missing string \'target\'');
    });

    test('unknown action rejected', () {
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "action": "destroy", "target": "x"}]}')
              .reason,
          'capabilities[0].action \'destroy\' is not a known action');
    });

    test('bad trust annotation rejected', () {
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": "x", "trust": "maybe"}]}')
              .reason,
          'capabilities[0].trust \'maybe\' is not supported');
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": "x", "trust": 7}]}')
              .reason,
          'capabilities[0].trust must be a string');
    });

    test('non-string flavor rejected', () {
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": "x", "flavor": 7}]}')
              .reason,
          'capabilities[0].flavor must be a string');
    });

    test('non-string justification ignored', () {
      final m = Manifest.loadFromString(
          '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": "x", "justification": 7}]}');
      expect(m.capabilities[0].justification, '');
    });

    test('flavor errors precede category errors', () {
      // Unknown flavor fires before the unknown category is even parsed.
      expect(
          loadError(
                  '{"version": 1, "capabilities": [{"category": "warp", "action": "read", "target": "x", "flavor": "read"}]}')
              .reason,
          contains('flavor'));
    });
  });

  group('EXTRA: file loading', () {
    test('round-trips a real manifest file', () {
      final dir = Directory.systemTemp.createTempSync('cage-test');
      try {
        final file = File('${dir.path}/required_capabilities.json');
        file.writeAsStringSync(
            '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": "./x"}]}');
        final m = Manifest.loadFromFile(file.path);
        expect(m.capabilities.length, 1);
        expect(m.has(Category.fs, Action.read, './x'), isTrue);
      } finally {
        dir.deleteSync(recursive: true);
      }
    });

    test('missing file raises io error', () {
      final dir = Directory.systemTemp.createTempSync('cage-test');
      try {
        final missing = '${dir.path}/does-not-exist.json';
        try {
          Manifest.loadFromFile(missing);
          fail('load did not throw');
        } on ManifestIoError catch (e) {
          expect(e.toString(), contains('manifest I/O error:'));
        }
      } finally {
        dir.deleteSync(recursive: true);
      }
    });

    test('non-utf8 bytes raise parse error', () {
      final dir = Directory.systemTemp.createTempSync('cage-test');
      try {
        final file = File('${dir.path}/bad.json');
        file.writeAsBytesSync([0xff, 0xfe, 0x00, 0x28]);
        try {
          Manifest.loadFromFile(file.path);
          fail('load did not throw');
        } on ManifestParseError catch (e) {
          expect(e.detail, contains('manifest is not UTF-8:'));
        }
      } finally {
        dir.deleteSync(recursive: true);
      }
    });

    test('loads this package’s own manifest', () {
      // Slice 2 contract change (deliberate, surfaced in the PR): the
      // manifest now declares the backend seam (fs x5 + env read) instead
      // of []. Under-declaring would be a false negative in a security
      // manifest. It is intentionally NOT RWS-clean — separation is
      // enforced per caller manifest at each wrapped call — so it loads
      // through pair validation only, never the checked constructor.
      final document = File('required_capabilities.json').readAsStringSync();
      final decoded = jsonDecode(document) as Map<String, dynamic>;
      final caps = (decoded['capabilities'] as List).map((entry) {
        final cap = entry as Map<String, dynamic>;
        return Capability(
          category:
              Category.values.firstWhere((v) => v.wire == cap['category']),
          action: Action.values.firstWhere((v) => v.wire == cap['action']),
          target: cap['target'] as String,
          justification: cap['justification'] as String,
        );
      }).toList();
      final m = Manifest(caps);
      expect(m.capabilities, hasLength(6));
      for (final action in Action.values) {
        final covered = m.has(Category.fs, action, '/any/path');
        expect(
          covered,
          equals(action == Action.read ||
              action == Action.write ||
              action == Action.create ||
              action == Action.delete ||
              action == Action.list),
          reason: 'fs:$action',
        );
      }
      expect(m.has(Category.env, Action.read, 'ANY_VAR'), isTrue);
      expect(m.has(Category.env, Action.write, 'ANY_VAR'), isFalse);
      expect(
        () => m.validateReadWriteSeparation(),
        throwsA(isA<ManifestRwsViolation>()),
      );
    });
  });

  group('EXTRA: values and helpers', () {
    test('constructors differ on rws checks', () {
      final caps = [
        Capability(
            category: Category.net,
            action: Action.listen,
            target: '0.0.0.0:1',
            justification: 't'),
        Capability(
            category: Category.proc,
            action: Action.exec,
            target: 'tool',
            justification: 't'),
      ];
      // Unchecked builds fine...
      expect(Manifest(caps).capabilities.length, 2);
      // ...checked rejects.
      expect(
          () => Manifest.checked(caps), throwsA(isA<ManifestRwsViolation>()));
      // And validate reruns on demand.
      expect(() => Manifest(caps).validateReadWriteSeparation(),
          throwsA(isA<ManifestRwsViolation>()));
    });

    test('capabilities getter is unmodifiable', () {
      final m = Manifest.empty();
      expect(
          () => m.capabilities.add(Capability(
              category: Category.fs,
              action: Action.read,
              target: 'x',
              justification: 't')),
          throwsUnsupportedError);
    });

    test('manifest equality and display', () {
      Capability cap() => Capability(
          category: Category.fs,
          action: Action.read,
          target: 'x',
          justification: 't');
      expect(Manifest([cap()]), equals(Manifest([cap()])));
      expect(Manifest([cap()]).hashCode, Manifest([cap()]).hashCode);
      expect(Manifest([cap()]) == Manifest.empty(), isFalse);
      expect(Manifest([cap()]) == 'x', isFalse);
      expect(Manifest([cap()]).toString(), 'Manifest(1 capabilities)');
      final other = Capability(
          category: Category.fs,
          action: Action.read,
          target: 'y',
          justification: 't');
      expect(Manifest([cap()]) == Manifest([other]), isFalse);
    });

    test('capability equality and display', () {
      Capability cap(String target) => Capability(
          category: Category.fs,
          action: Action.read,
          target: target,
          justification: 't');
      expect(cap('x'), equals(cap('x')));
      expect(cap('x').hashCode, cap('x').hashCode);
      expect(cap('x') == cap('y'), isFalse);
      expect(cap('x') == 'x', isFalse);
      expect(cap('x').toString(), 'Capability(fs:read:x)');
      expect(cap('x').withFlavor(rws.CapabilityFlavor.ingestion) == cap('x'),
          isFalse);
    });

    test('error equality', () {
      // Non-const instances throughout: identical consts would
      // short-circuit == without invoking the operators.
      CapabilityViolationError violation(String target) =>
          CapabilityViolationError(
              category: Category.fs,
              action: Action.read,
              target: target,
              message: 'm');
      final v1 = violation('x');
      final v2 = violation('x');
      final v3 = violation('y');
      expect(v1, equals(v2));
      expect(v1.hashCode, v2.hashCode);
      expect(v1 == v3, isFalse);
      expect(v1 == 'x', isFalse);
      expect(v1.toString(), 'm');
      UnsupportedPair pair(Action action) =>
          UnsupportedPair(Category.fs, action);
      expect(pair(Action.read), equals(pair(Action.read)));
      expect(pair(Action.read).hashCode, pair(Action.read).hashCode);
      expect(pair(Action.read) == pair(Action.write), isFalse);
      expect(pair(Action.read) == 'x', isFalse);
      expect(pair(Action.read).toString(), contains('fs'));
      expect(EmptyTarget(), equals(EmptyTarget()));
      expect(EmptyTarget().hashCode, 0);
      expect(EmptyTarget() == 'x', isFalse);
      expect(EmptyTarget().toString(), contains('empty'));
      InvalidTargetFormat fmt(String reason) => InvalidTargetFormat(reason);
      expect(fmt('a'), equals(fmt('a')));
      expect(fmt('a').hashCode, fmt('a').hashCode);
      expect(fmt('a') == fmt('b'), isFalse);
      expect(fmt('a') == 'x', isFalse);
      expect(fmt('a').toString(), contains('a'));
      ManifestParseError parseErr(String detail) => ManifestParseError(detail);
      expect(parseErr('a'), equals(parseErr('a')));
      expect(parseErr('a').hashCode, parseErr('a').hashCode);
      expect(parseErr('a') == parseErr('b'), isFalse);
      expect(parseErr('a') == 'x', isFalse);
      expect(parseErr('a').toString(), contains('a'));
      ManifestSchemaError schemaErr(String reason) =>
          ManifestSchemaError(reason);
      expect(schemaErr('a'), equals(schemaErr('a')));
      expect(schemaErr('a').hashCode, schemaErr('a').hashCode);
      expect(schemaErr('a') == schemaErr('b'), isFalse);
      expect(schemaErr('a') == parseErr('a'), isFalse);
      expect(schemaErr('a').toString(), contains('a'));
      ManifestInvalidCombination combErr(InvalidCombination inner) =>
          ManifestInvalidCombination(inner);
      expect(combErr(EmptyTarget()), equals(combErr(EmptyTarget())));
      expect(combErr(EmptyTarget()).hashCode, combErr(EmptyTarget()).hashCode);
      expect(combErr(EmptyTarget()) == combErr(fmt('a')), isFalse);
      expect(combErr(EmptyTarget()) == 'x', isFalse);
      expect(combErr(EmptyTarget()).toString(), contains('empty'));
      ManifestIoError ioErr(String detail) => ManifestIoError(detail);
      expect(ioErr('a'), equals(ioErr('a')));
      expect(ioErr('a').hashCode, ioErr('a').hashCode);
      expect(ioErr('a') == ioErr('b'), isFalse);
      expect(ioErr('a') == 'x', isFalse);
      expect(ioErr('a').toString(), contains('a'));
      ManifestRwsViolation rwsErr(String message, String target) =>
          ManifestRwsViolation(untrustedInputs: [
            rws.Capability(category: 'c', action: 'a', target: target)
          ], actuations: const [], message: message);
      expect(rwsErr('m', 'x'), equals(rwsErr('m', 'x')));
      expect(rwsErr('m', 'x').hashCode, rwsErr('m', 'x').hashCode);
      expect(rwsErr('m', 'x') == rwsErr('m', 'y'), isFalse);
      expect(rwsErr('m', 'x') == rwsErr('n', 'x'), isFalse);
      expect(rwsErr('m', 'x') == 'x', isFalse);
      expect(rwsErr('m', 'x').toString(), contains('m'));
      final twoItem = ManifestRwsViolation(untrustedInputs: [
        rws.Capability(category: 'c', action: 'a', target: 'x'),
        rws.Capability(category: 'c', action: 'a', target: 'y'),
      ], actuations: const [], message: 'm');
      expect(rwsErr('m', 'x') == twoItem, isFalse);
    });

    test('summary helpers on empty and pure manifests', () {
      final empty = Manifest.empty().capabilitySurfaceSummary();
      expect(empty.hasNetworkAccess, isFalse);
      expect(empty.hasProcessAccess, isFalse);
      expect(empty.hasWriteOrActuation, isFalse);
      expect(empty.crossesUntrustedBoundary, isFalse);
      final pure = Manifest([
        Capability(
            category: Category.fs,
            action: Action.read,
            target: 'x',
            justification: 't'),
      ]).capabilitySurfaceSummary();
      expect(pure.hasWriteOrActuation, isFalse);
      expect(pure.fullyAnnotated, isFalse);
    });

    test('summary equality and display', () {
      final a = Manifest([
        Capability(
            category: Category.fs,
            action: Action.read,
            target: 'x',
            justification: 't'),
      ]).capabilitySurfaceSummary();
      final b = Manifest([
        Capability(
            category: Category.fs,
            action: Action.read,
            target: 'x',
            justification: 't'),
      ]).capabilitySurfaceSummary();
      expect(a, equals(b));
      expect(a.hashCode, b.hashCode);
      expect(a == CapabilitySurfaceSummary.empty(), isFalse);
      expect(a == 'x', isFalse);
      expect(a.toString(), contains('total: 1'));
    });

    test('rws conversion carries wire forms', () {
      final converted = Capability(
        category: Category.thread,
        action: Action.spawn,
        target: 'pool',
        justification: 't',
      ).toRwsCapability();
      expect(converted.category, 'thread');
      expect(converted.action, 'spawn');
      expect(converted.target, 'pool');
      expect(converted.justification, 't');
      expect(converted.flavor, isNull);
      expect(converted.trust, isNull);
    });
  });

  group('EXTRA: glob edges', () {
    test('colons fall back to paths with slashes or spaces', () {
      // Pattern holds ':' and '/': path branch, exact segments.
      expect(matchTarget('a:b/c', 'a:b/c'), isTrue);
      expect(matchTarget('a:b/c', 'a:b/d'), isFalse);
      // Candidate holds ':' but pattern does not: path branch.
      expect(matchTarget('ab', 'a:b'), isFalse);
      // Space in the pattern port: path branch.
      expect(matchTarget('h:80 80', 'h:80 80'), isTrue);
    });

    test('partial host globs never match', () {
      // Net hosts match exactly or by full '*': anything else is
      // literal, even when it contains star characters.
      expect(matchTarget('api.*.gov:443', 'api.us.gov:443'), isFalse);
      expect(matchTarget('api.*.gov:443', 'api.*.gov:443'), isTrue);
      expect(matchTarget('*.gov:443', 'a.gov:443'), isFalse);
    });

    test('empty and root patterns', () {
      expect(matchTarget('', ''), isTrue);
      expect(matchTarget('', 'a'), isFalse);
      expect(matchTarget('/', '/'), isTrue);
    });

    test('double star matches zero segments', () {
      expect(matchTarget('a/**/c', 'a/c'), isTrue);
      expect(matchTarget('**/c', 'c'), isTrue);
      expect(matchTarget('a/**', 'a'), isTrue);
    });

    test('matching is case sensitive', () {
      expect(matchTarget('A', 'a'), isFalse);
      expect(matchTarget('AbC', 'abc'), isFalse);
    });

    test('middle piece misses fail the segment', () {
      expect(matchTarget('a*b*c', 'axc'), isFalse);
      expect(matchTarget('a*b*c', 'axbyc'), isTrue);
    });

    test('summary counts all categories', () {
      Capability cap(Category category, Action action) => Capability(
          category: category, action: action, target: 't', justification: 't');
      // One capability per category arm incl. net/proc/ffi/time/stdin.
      final summary = Manifest([
        cap(Category.fs, Action.list),
        cap(Category.net, Action.connect),
        cap(Category.proc, Action.exec),
        cap(Category.env, Action.write),
        cap(Category.ffi, Action.load),
        cap(Category.time, Action.sleep),
        cap(Category.stdin, Action.read),
        cap(Category.stdout, Action.write),
        cap(Category.rand, Action.read),
        cap(Category.thread, Action.spawn),
        cap(Category.ui, Action.write),
      ]).capabilitySurfaceSummary();
      expect(summary.totalCapabilities, 11);
      expect(summary.uniqueCategories, 11);
      expect(summary.networkCapabilities, 1);
      expect(summary.processCapabilities, 1);
      expect(summary.ffiCapabilities, 1);
      expect(summary.timeCapabilities, 1);
      expect(summary.stdioCapabilities, 2);
    });

    test('summary counts internal flavor', () {
      final summary = Manifest([
        Capability(
                category: Category.fs,
                action: Action.read,
                target: 'x',
                justification: 't')
            .withFlavor(rws.CapabilityFlavor.internal),
      ]).capabilitySurfaceSummary();
      expect(summary.internalCapabilities, 1);
      expect(summary.ingestionCapabilities, 0);
      expect(summary.actuationCapabilities, 0);
    });
  });
}
