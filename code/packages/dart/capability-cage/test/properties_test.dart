/// Property tests for `coding_adventures_capability_cage`, using the
/// in-lane `pbt` package: glob laws, manifest/model agreement, summary
/// consistency, JSON round-trips, and check/throw duality — fixed seeds.
library;

import 'dart:convert';

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
}
