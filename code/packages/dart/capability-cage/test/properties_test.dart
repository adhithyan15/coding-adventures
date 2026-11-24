/// Property tests for `coding_adventures_capability_cage`, using the
/// in-lane `pbt` package: glob laws, manifest/model agreement, summary
/// consistency, JSON round-trips, and check/throw duality — fixed seeds.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart'
    as rws;
import 'package:test/test.dart';

/// Glob fragments: literals, separators, and specials.
Gen<String> _globPiece() => Gen.oneOf([
      Gen.constant('a'),
      Gen.constant('foo'),
      Gen.constant('*'),
      Gen.constant('**'),
      Gen.constant('.tokens'),
      Gen.constant('/'),
      Gen.constant(':'),
      Gen.constant('443'),
      Gen.constant('['),
      Gen.constant(']'),
      Gen.bytes(0, 4).map(String.fromCharCodes),
    ]);

Gen<String> _globString() =>
    Gen.list(0, 6, _globPiece()).map((pieces) => pieces.join());

/// Plain segments: digits only (no specials, separators, or colons).
Gen<String> _plainString() => Gen.list(0, 6, Gen.u8()).map(
    (bytes) => bytes.map((b) => String.fromCharCode(0x30 + (b % 10))).join());

/// Trusted-read capabilities (RWS-clean by default, so arbitrary
/// multisets always load): env/time/stdin/rand/ui reads plus
/// package-scoped fs reads.
Capability _trustedRead(int tag, int variant) {
  const choices = [
    (Category.env, Action.read),
    (Category.time, Action.read),
    (Category.stdin, Action.read),
    (Category.rand, Action.read),
    (Category.ui, Action.read),
    (Category.fs, Action.read),
  ];
  final choice = choices[tag % choices.length];
  final target = choice.$1 == Category.fs
      ? 'package:/target-$tag-$variant'
      : 'target-$tag-$variant';
  return Capability(
    category: choice.$1,
    action: choice.$2,
    target: target,
    justification: 't',
  );
}

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('glob self-match holds universally', () {
      final verdict = checkProperty<String>(
        BigInt.from(181),
        PbtOptions(numTests: 500, seed: BigInt.from(181)),
        (rng) => _globString().sample(rng),
        (pattern) => matchTarget(pattern, pattern),
      );
      _check('glob-self-match', verdict);
    });

    test('star matches every target', () {
      final verdict = checkProperty<String>(
        BigInt.from(182),
        PbtOptions(numTests: 200, seed: BigInt.from(182)),
        (rng) => _globString().sample(rng),
        (candidate) => matchTarget('*', candidate),
      );
      _check('glob-star', verdict);
    });

    test('special-free patterns match by equality', () {
      final gen = Gen.tuple2(_plainString(), _plainString());
      final verdict = checkProperty<(String, String)>(
        BigInt.from(183),
        PbtOptions(numTests: 300, seed: BigInt.from(183)),
        (rng) => gen.sample(rng),
        (input) => matchTarget(input.$1, input.$2) == (input.$1 == input.$2),
      );
      _check('glob-literal', verdict);
    });

    test('has() agrees with the direct model', () {
      final gen = Gen.tuple2(
        Gen.list(0, 5, Gen.u8()),
        Gen.tuple3(Gen.u8(), Gen.u8(), _globString()),
      );
      final verdict = checkProperty<(List<int>, (int, int, String))>(
        BigInt.from(184),
        PbtOptions(numTests: 200, seed: BigInt.from(184)),
        (rng) => gen.sample(rng),
        (input) {
          final caps = [
            for (var i = 0; i < input.$1.length; i++)
              _trustedRead(input.$1[i], i)
          ];
          final manifest = Manifest(caps);
          final category =
              Category.values[input.$2.$1 % Category.values.length];
          final action = Action.values[input.$2.$2 % Action.values.length];
          final target = input.$2.$3;
          final want = caps.any((cap) =>
              cap.category == category &&
              cap.action == action &&
              matchTarget(cap.target, target));
          return manifest.has(category, action, target) == want;
        },
      );
      _check('has-model', verdict);
    });

    test('summary counters are consistent', () {
      final gen = Gen.list(0, 8, Gen.u8());
      final verdict = checkProperty<List<int>>(
        BigInt.from(185),
        PbtOptions(numTests: 200, seed: BigInt.from(185)),
        (rng) => gen.sample(rng),
        (tags) {
          final caps = [
            for (var i = 0; i < tags.length; i++) _trustedRead(tags[i], i)
          ];
          final s = Manifest(caps).capabilitySurfaceSummary();
          return s.totalCapabilities == caps.length &&
              s.uniqueCategories ==
                  caps.map((c) => c.category).toSet().length &&
              s.uniqueActions == caps.map((c) => c.action).toSet().length &&
              s.uniqueTargets == caps.map((c) => c.target).toSet().length &&
              s.filesystemCapabilities +
                      s.networkCapabilities +
                      s.processCapabilities +
                      s.environmentCapabilities +
                      s.ffiCapabilities +
                      s.timeCapabilities +
                      s.stdioCapabilities +
                      s.randCapabilities +
                      s.threadCapabilities +
                      s.uiCapabilities ==
                  caps.length &&
              s.isEmpty == caps.isEmpty &&
              s.fullyAnnotated ==
                  (caps.isEmpty || s.unannotatedCapabilities == 0) &&
              s.hasNetworkAccess == (s.networkCapabilities > 0) &&
              s.hasProcessAccess == (s.processCapabilities > 0) &&
              s.crossesUntrustedBoundary == (s.untrustedCapabilities > 0);
        },
      );
      _check('summary-consistent', verdict);
    });

    test('json round-trips trusted reads', () {
      final gen = Gen.list(0, 5, Gen.u8());
      final verdict = checkProperty<List<int>>(
        BigInt.from(186),
        PbtOptions(numTests: 200, seed: BigInt.from(186)),
        (rng) => gen.sample(rng),
        (tags) {
          final caps = [
            for (var i = 0; i < tags.length; i++) _trustedRead(tags[i], i)
          ];
          final document = jsonEncode({
            'version': 1,
            'capabilities': [
              for (final cap in caps)
                {
                  'category': cap.category.wire,
                  'action': cap.action.wire,
                  'target': cap.target,
                  'justification': cap.justification,
                }
            ],
          });
          final loaded = Manifest.loadFromString(document);
          if (loaded.capabilities.length != caps.length) return false;
          for (var i = 0; i < caps.length; i++) {
            if (loaded.capabilities[i] != caps[i]) return false;
          }
          return true;
        },
      );
      _check('json-round-trip', verdict);
    });

    test('check throws exactly when has denies', () {
      final gen = Gen.tuple2(
        Gen.list(0, 5, Gen.u8()),
        Gen.tuple3(Gen.u8(), Gen.u8(), _globString()),
      );
      final verdict = checkProperty<(List<int>, (int, int, String))>(
        BigInt.from(187),
        PbtOptions(numTests: 200, seed: BigInt.from(187)),
        (rng) => gen.sample(rng),
        (input) {
          final caps = [
            for (var i = 0; i < input.$1.length; i++)
              _trustedRead(input.$1[i], i)
          ];
          final manifest = Manifest(caps);
          final category =
              Category.values[input.$2.$1 % Category.values.length];
          final action = Action.values[input.$2.$2 % Action.values.length];
          final target = input.$2.$3;
          final covered = manifest.has(category, action, target);
          try {
            manifest.check(category, action, target);
            return covered;
          } on CapabilityViolationError catch (e) {
            return !covered &&
                e.category == category &&
                e.action == action &&
                e.target == target;
          }
        },
      );
      _check('check-duality', verdict);
    });

    test('rws conversion preserves wire forms', () {
      final gen = Gen.tuple3(Gen.u8(), Gen.u8(), _plainString());
      final verdict = checkProperty<(int, int, String)>(
        BigInt.from(188),
        PbtOptions(numTests: 200, seed: BigInt.from(188)),
        (rng) => gen.sample(rng),
        (input) {
          // Any valid pair converts with wire strings intact.
          final pairs = [
            for (final c in Category.values)
              for (final a in Action.values)
                if (isValidCombination(c, a)) (c, a)
          ];
          final pair = pairs[input.$1 % pairs.length];
          final target = input.$3.isEmpty ? 'x-$input' : input.$3;
          final converted = Capability(
            category: pair.$1,
            action: pair.$2,
            target: target,
            justification: 't',
            flavor: input.$2.isEven ? rws.CapabilityFlavor.ingestion : null,
            trust: input.$2.isOdd ? rws.CapabilityTrust.untrusted : null,
          ).toRwsCapability();
          return converted.category == pair.$1.wire &&
              converted.action == pair.$2.wire &&
              converted.target == target &&
              converted.justification == 't' &&
              converted.flavor ==
                  (input.$2.isEven ? rws.CapabilityFlavor.ingestion : null) &&
              converted.trust ==
                  (input.$2.isOdd ? rws.CapabilityTrust.untrusted : null);
        },
      );
      _check('rws-conversion', verdict);
    });
  });

  group('slice2 properties', () {
    test('envelope map preserves the record', () {
      final gen = Gen.tuple2(
        Gen.tuple3(_category(), _action(), _target()),
        Gen.list(0, 8, Gen.u8()),
      );
      final verdict = checkProperty<((Category, Action, String), List<int>)>(
        BigInt.from(281),
        PbtOptions(numTests: 300, seed: BigInt.from(281)),
        (rng) => gen.sample(rng),
        (input) {
          final cap = input.$1;
          final payload = input.$2;
          final mapped =
              Operation<List<int>>.allowed(cap.$1, cap.$2, cap.$3, payload)
                  .map((items) => items.length);
          return mapped.output == payload.length &&
              mapped.isAllowed &&
              mapped.auditRecord().category == cap.$1 &&
              mapped.auditRecord().action == cap.$2 &&
              mapped.auditRecord().target == cap.$3 &&
              mapped.auditRecord().message == null;
        },
      );
      _check('envelope-map', verdict);
    });

    test('denied envelopes carry the violation without output', () {
      final gen = Gen.tuple2(
        Gen.tuple3(_category(), _action(), _target()),
        _plainString(),
      );
      final verdict = checkProperty<((Category, Action, String), String)>(
        BigInt.from(282),
        PbtOptions(numTests: 300, seed: BigInt.from(282)),
        (rng) => gen.sample(rng),
        (input) {
          final cap = input.$1;
          final message = input.$2.isEmpty ? 'm' : input.$2;
          final operation = Operation.denied(CapabilityViolationError(
            category: cap.$1,
            action: cap.$2,
            target: cap.$3,
            message: message,
          ));
          return !operation.isAllowed &&
              operation.output == null &&
              operation.auditRecord().decision == OperationDecision.denied &&
              operation.auditRecord().message == message &&
              operation.auditRecord().capabilityKey() ==
                  '${cap.$1.wire}:${cap.$2.wire}:${cap.$3}';
        },
      );
      _check('denied-shape', verdict);
    });

    test('sinks keep arrival order and drain fully', () {
      final gen = Gen.list(
        1,
        12,
        Gen.tuple2(
          Gen.tuple3(_category(), _action(), _target()),
          Gen.boolean(),
        ),
      );
      final verdict = checkProperty<List<((Category, Action, String), bool)>>(
        BigInt.from(283),
        PbtOptions(numTests: 300, seed: BigInt.from(283)),
        (rng) => gen.sample(rng),
        (input) {
          final sink = VecAuditSink();
          final keys = <String>[];
          for (final item in input) {
            final cap = item.$1;
            final key = '${cap.$1.wire}:${cap.$2.wire}:${cap.$3}';
            keys.add(key);
            if (item.$2) {
              Operation<int>.allowed(cap.$1, cap.$2, cap.$3, 1).recordTo(sink);
            } else {
              Operation.denied(CapabilityViolationError(
                category: cap.$1,
                action: cap.$2,
                target: cap.$3,
                message: 'no',
              )).recordTo(sink);
            }
          }
          final recorded =
              sink.records().map((r) => r.capabilityKey()).toList();
          if (recorded.length != keys.length) return false;
          for (var i = 0; i < keys.length; i++) {
            if (recorded[i] != keys[i]) return false;
          }
          final drained = sink.takeRecords();
          if (drained.length != keys.length) return false;
          return sink.records().isEmpty;
        },
      );
      _check('sink-order-drain', verdict);
    });

    test('zone overrides nest and restore under throws', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      final verdict = checkProperty<(int, int)>(
        BigInt.from(284),
        PbtOptions(numTests: 200, seed: BigInt.from(284)),
        (rng) => gen.sample(rng),
        (input) {
          final depth = (input.$1 % 5) + 1;
          final throwAt = input.$2 % (depth + 1);
          final backends =
              List<TestBackend>.generate(depth, (_) => TestBackend());
          var deepest = -1;
          try {
            void nest(int level) {
              if (level == depth) {
                deepest = level;
                return;
              }
              runWithBackend(backends[level], () {
                if (!identical(currentBackend(), backends[level])) {
                  throw StateError('wrong ambient backend');
                }
                if (level == throwAt) throw StateError('injected');
                nest(level + 1);
                if (!identical(currentBackend(), backends[level])) {
                  throw StateError('restoration failed');
                }
              });
            }

            nest(0);
          } on StateError catch (e) {
            if (e.message != 'injected') return false;
          }
          if (throwAt >= depth && deepest != depth) return false;
          return currentBackend() is OpenBackend;
        },
      );
      _check('zone-nesting', verdict);
    });

    test('call records obey equality laws', () {
      final gen = Gen.tuple3(Gen.u8(), _target(), Gen.list(0, 6, Gen.u8()));
      final verdict = checkProperty<(int, String, List<int>)>(
        BigInt.from(285),
        PbtOptions(numTests: 300, seed: BigInt.from(285)),
        (rng) => gen.sample(rng),
        (input) {
          final call = _callFor(input.$1, input.$2, input.$3);
          final twin = _callFor(input.$1, input.$2, input.$3);
          // Reflexive, symmetric, hash-consistent.
          if (call != call || call != twin || twin != call) return false;
          if (call.hashCode != twin.hashCode) return false;
          // A different variant tag is never equal (same payload).
          final other = _callFor((input.$1 + 1) % 7, input.$2, input.$3);
          return call != other;
        },
      );
      _check('call-equality', verdict);
    });

    test('withMessage copies without mutating', () {
      final gen = Gen.tuple2(
        Gen.tuple3(_category(), _action(), _target()),
        _plainString(),
      );
      final verdict = checkProperty<((Category, Action, String), String)>(
        BigInt.from(286),
        PbtOptions(numTests: 300, seed: BigInt.from(286)),
        (rng) => gen.sample(rng),
        (input) {
          final cap = input.$1;
          final original = OperationRecord.allowed(cap.$1, cap.$2, cap.$3);
          final copy = original.withMessage(input.$2);
          return original.message == null &&
              copy.message == input.$2 &&
              copy.capabilityKey() == original.capabilityKey() &&
              copy.decision == original.decision;
        },
      );
      _check('with-message-copy', verdict);
    });
  });
}

/// Uniform random category.
Gen<Category> _category() =>
    Gen.integer(BigInt.zero, BigInt.from(Category.values.length - 1))
        .map((i) => Category.values[i.toInt()]);

/// Uniform random action.
Gen<Action> _action() =>
    Gen.integer(BigInt.zero, BigInt.from(Action.values.length - 1))
        .map((i) => Action.values[i.toInt()]);

/// Hostile targets: empty, NUL, separators, and raw byte strings.
Gen<String> _target() => Gen.oneOf([
      Gen.constant(''),
      Gen.constant('a'),
      Gen.constant(String.fromCharCode(0)),
      Gen.constant('./input.txt'),
      Gen.constant('api.example.test:443'),
      Gen.constant('C:\\temp\\x'),
      Gen.bytes(0, 8).map(String.fromCharCodes),
    ]);

/// Builds one of the seven call variants from a tag.
TestBackendCall _callFor(int tag, String target, List<int> bytes) {
  final data = Uint8List.fromList(bytes);
  switch (tag % 7) {
    case 0:
      return ReadFileCall(target);
    case 1:
      return WriteFileCall(target, data);
    case 2:
      return CreateFileCall(target);
    case 3:
      return DeleteFileCall(target);
    case 4:
      return ListDirCall(target);
    case 5:
      return ReadEnvCall(target);
    default:
      return WriteEnvCall(target, target);
  }
}
