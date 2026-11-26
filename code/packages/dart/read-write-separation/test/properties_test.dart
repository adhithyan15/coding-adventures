/// Property tests for `coding_adventures_read_write_separation`, using the
/// in-lane `pbt` package: verdict stability (duplication, reversal),
/// summary-counter consistency, classification laws, and overlap symmetry —
/// all with fixed seeds.
library;

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart';
import 'package:test/test.dart';

Gen<String> _category() => Gen.oneOf([
      for (final c in [
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
        'vault',
        'channel',
        'junk',
      ])
        Gen.constant(c),
      Gen.bytes(0, 5).map(String.fromCharCodes),
    ]);

Gen<String> _action() => Gen.oneOf([
      for (final a in [
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
        'request_lease',
        'junk',
      ])
        Gen.constant(a),
      Gen.bytes(0, 5).map(String.fromCharCodes),
    ]);

Gen<String> _target() => Gen.oneOf([
      Gen.constant('localhost'),
      Gen.constant('localhost:8080'),
      Gen.constant('127.0.0.1'),
      Gen.constant('0.0.0.0:8080'),
      Gen.constant('example.com:443'),
      Gen.constant('package:/state/x.json'),
      Gen.constant('pkg:/x'),
      Gen.constant('/etc/passwd'),
      Gen.constant('package:/state/*'),
      Gen.constant('*'),
      Gen.constant('weather-snapshots'),
      Gen.bytes(0, 8).map(String.fromCharCodes),
    ]);

Gen<CapabilityFlavor?> _flavor() => Gen.oneOf([
      Gen.constant(null),
      Gen.constant(CapabilityFlavor.ingestion),
      Gen.constant(CapabilityFlavor.actuation),
      Gen.constant(CapabilityFlavor.internal),
    ]);

Gen<CapabilityTrust?> _trust() => Gen.oneOf([
      Gen.constant(null),
      Gen.constant(CapabilityTrust.trusted),
      Gen.constant(CapabilityTrust.untrusted),
    ]);

Gen<Capability> _capability() => Gen.tuple3(_category(), _action(), _target())
    .flatMap((triple) => Gen.tuple3(_flavor(), _trust(), Gen.boolean())
        .map((annotations) => Capability(
              category: triple.$1,
              action: triple.$2,
              target: triple.$3,
              flavor: annotations.$1,
              trust: annotations.$2,
              justification: annotations.$3 ? 'why' : null,
            )));

/// True iff [validateManifest] rejects [capabilities]. Any other throw
/// propagates, failing the property as a panic (only [RwsViolation] is a
/// legitimate outcome).
bool _violates(List<Capability> capabilities) {
  try {
    validateManifest(capabilities);
    return false;
  } on RwsViolation {
    return true;
  }
}

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('verdict is stable under duplication', () {
      final gen = Gen.list(1, 6, _capability());
      final verdict = checkProperty<List<Capability>>(
        BigInt.from(41),
        PbtOptions(numTests: 200, seed: BigInt.from(41)),
        (rng) => gen.sample(rng),
        (caps) => _violates(caps) == _violates([...caps, caps.first]),
      );
      _check('duplication-stable', verdict);
    });

    test('verdict is stable under reversal', () {
      final gen = Gen.list(0, 6, _capability());
      final verdict = checkProperty<List<Capability>>(
        BigInt.from(42),
        PbtOptions(numTests: 200, seed: BigInt.from(42)),
        (rng) => gen.sample(rng),
        (caps) => _violates(caps) == _violates(caps.reversed.toList()),
      );
      _check('reversal-stable', verdict);
    });

    test('summary counters are consistent', () {
      final gen = Gen.list(0, 6, _capability());
      final verdict = checkProperty<List<Capability>>(
        BigInt.from(43),
        PbtOptions(numTests: 200, seed: BigInt.from(43)),
        (rng) => gen.sample(rng),
        (caps) {
          final s = summarizeManifest(caps);
          return s.totalCapabilities == caps.length &&
              s.ingestionCapabilities +
                      s.actuationCapabilities +
                      s.internalCapabilities ==
                  caps.length &&
              s.trustedCapabilities + s.untrustedCapabilities == caps.length &&
              s.justifiedCapabilities <= caps.length &&
              s.untrustedInputs <= caps.length &&
              s.externalActuations <= caps.length &&
              s.hasRwsRisk ==
                  (s.untrustedInputs > 0 && s.externalActuations > 0) &&
              s.hasSameResourceOverlap == (s.overlappingReadWritePairs > 0) &&
              s.isEmpty == caps.isEmpty;
        },
      );
      _check('summary-consistent', verdict);
    });

    test('classification laws hold', () {
      final gen = _capability();
      final verdict = checkProperty<Capability>(
        BigInt.from(44),
        PbtOptions(numTests: 300, seed: BigInt.from(44)),
        (rng) => gen.sample(rng),
        (cap) {
          final c = classifyCapability(cap);
          return (!c.isUntrustedInput || c.isInput) &&
              c.isExternalActuation ==
                  (c.flavor == CapabilityFlavor.actuation) &&
              (cap.flavor == null || c.flavor == cap.flavor) &&
              (cap.trust == null || c.trust == cap.trust);
        },
      );
      _check('classification-laws', verdict);
    });

    test('overlap count is symmetric under reversal', () {
      final gen = Gen.list(0, 6, _capability());
      final verdict = checkProperty<List<Capability>>(
        BigInt.from(45),
        PbtOptions(numTests: 200, seed: BigInt.from(45)),
        (rng) => gen.sample(rng),
        (caps) =>
            summarizeManifest(caps).overlappingReadWritePairs ==
            summarizeManifest(caps.reversed.toList()).overlappingReadWritePairs,
      );
      _check('overlap-symmetric', verdict);
    });

    test('empty manifest validates and summarizes empty', () {
      // Degenerate but load-bearing: no crash, no violation, zero summary.
      final verdict = checkProperty<int>(
        BigInt.from(46),
        PbtOptions(numTests: 10, seed: BigInt.from(46)),
        (rng) => Gen.u8().sample(rng),
        (_) {
          final s = summarizeManifest(const []);
          return !_violates(const []) && s.isEmpty;
        },
      );
      _check('empty-manifest', verdict);
    });
  });
}
