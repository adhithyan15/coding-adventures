/// Unit tests for `coding_adventures_read_write_separation`.
///
/// Mirrors the Rust reference's inline suite case-for-case, then covers the
/// default tables, trust helpers, glob edges, message precedence, list
/// dedup, and value semantics the mirror leaves implicit.
library;

import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart';
import 'package:test/test.dart';

Capability cap(String category, String action, String target) =>
    Capability(category: category, action: action, target: target);

/// Runs [validateManifest], expecting an [RwsViolation], and returns it.
RwsViolation catchViolation(List<Capability> capabilities) {
  try {
    validateManifest(capabilities);
  } on RwsViolation catch (e) {
    return e;
  }
  fail('expected RwsViolation, but the manifest was accepted');
}

void main() {
  group('mirror: validate_manifest', () {
    test('pure ingestion manifest is accepted', () {
      final capabilities = [
        cap('net', 'connect', 'api.weather.gov:443')
            .withFlavor(CapabilityFlavor.ingestion),
        cap('channel', 'write', 'weather-snapshots'),
      ];
      expect(() => validateManifest(capabilities), returnsNormally);
    });

    test('pure actuation manifest is accepted', () {
      final capabilities = [
        cap('channel', 'read', 'email-drafts')
            .withTrust(CapabilityTrust.trusted),
        cap('net', 'connect', 'smtp.gmail.com:465'),
      ];
      expect(() => validateManifest(capabilities), returnsNormally);
    });

    test('mixed manifest is rejected with both lists', () {
      final capabilities = [
        cap('net', 'connect', 'imap.gmail.com:993')
            .withFlavor(CapabilityFlavor.ingestion),
        cap('fs', 'write', '/tmp/outbox/message.txt'),
      ];
      final violation = catchViolation(capabilities);
      expect(violation.untrustedInputs, hasLength(1));
      expect(violation.actuations, hasLength(1));
      expect(violation.untrustedInputs[0].identifier(),
          'net:connect:imap.gmail.com:993');
      expect(violation.actuations[0].identifier(),
          'fs:write:/tmp/outbox/message.txt');
      expect(violation.message,
          'read/write separation violation: manifest contains untrusted inputs and external actuations; split the agent or insert a trusted channel boundary');
    });

    test('fs read/write overlap on same path is rejected', () {
      final capabilities = [
        cap('fs', 'read', 'package:/state/cache.json'),
        cap('fs', 'write', 'package:/state/cache.json'),
      ];
      final violation = catchViolation(capabilities);
      expect(violation.untrustedInputs[0].identifier(),
          'fs:read:package:/state/cache.json');
      expect(violation.actuations[0].identifier(),
          'fs:write:package:/state/cache.json');
      expect(violation.message,
          'read/write separation violation: manifest contains overlapping read/write capabilities');
    });

    test('fs read/write overlap on glob is rejected', () {
      final capabilities = [
        cap('fs', 'read', 'package:/state/*'),
        cap('fs', 'write', 'package:/state/cache.json'),
      ];
      expect(
          () => validateManifest(capabilities), throwsA(isA<RwsViolation>()));
    });

    test('fs read/write on disjoint paths is accepted', () {
      final capabilities = [
        cap('fs', 'read', 'package:/templates/weather.txt'),
        cap('fs', 'write', '/tmp/weather-email.txt'),
      ];
      expect(() => validateManifest(capabilities), returnsNormally);
    });

    test('vault read/write same secret is rejected', () {
      final capabilities = [
        cap('vault', 'read', 'gmail-app-password'),
        cap('vault', 'write', 'gmail-app-password'),
      ];
      final violation = catchViolation(capabilities);
      expect(violation.untrustedInputs[0].identifier(),
          'vault:read:gmail-app-password');
      expect(violation.actuations[0].identifier(),
          'vault:write:gmail-app-password');
    });

    test('vault read and write disjoint secrets are accepted', () {
      final capabilities = [
        cap('vault', 'read', 'imap-credentials'),
        cap('vault', 'write', 'smtp-credentials'),
      ];
      expect(() => validateManifest(capabilities), returnsNormally);
    });

    test('channel read/write same channel is rejected', () {
      final capabilities = [
        cap('channel', 'read', 'weather-snapshots'),
        cap('channel', 'write', 'weather-snapshots'),
      ];
      final violation = catchViolation(capabilities);
      expect(violation.untrustedInputs[0].identifier(),
          'channel:read:weather-snapshots');
      expect(violation.actuations[0].identifier(),
          'channel:write:weather-snapshots');
    });

    test('explicit flavor overrides allow ingestion-only networks', () {
      final capabilities = [
        cap('net', 'connect', 'api.weather.gov:443')
            .withFlavor(CapabilityFlavor.ingestion),
        cap('net', 'connect', 'forecast.weather.gov:443')
            .withFlavor(CapabilityFlavor.ingestion),
      ];
      expect(() => validateManifest(capabilities), returnsNormally);
    });

    test('default net connect is actuation and conflicts with input', () {
      final def = cap('net', 'connect', 'smtp.gmail.com:465');
      final classification = classifyCapability(def);
      expect(classification.flavor, CapabilityFlavor.actuation);
      expect(classification.trust, CapabilityTrust.untrusted);
      expect(classification.isUntrustedInput, isFalse);
      expect(classification.isExternalActuation, isTrue);

      final capabilities = [def, cap('net', 'listen', '0.0.0.0:8080')];
      final violation = catchViolation(capabilities);
      expect(
          violation.untrustedInputs[0].identifier(), 'net:listen:0.0.0.0:8080');
      expect(violation.actuations[0].identifier(),
          'net:connect:smtp.gmail.com:465');
    });
  });

  group('mirror: summarize_manifest', () {
    test('summary counts capability shape and risk', () {
      final capabilities = [
        cap('net', 'connect', 'api.weather.gov:443')
            .withFlavor(CapabilityFlavor.ingestion)
            .withJustification('fetch weather alerts'),
        cap('channel', 'write', 'weather-snapshots'),
        cap('fs', 'read', 'package:/state/*'),
        cap('fs', 'write', 'package:/state/cache.json'),
      ];
      final summary = summarizeManifest(capabilities);
      expect(summary.totalCapabilities, 4);
      expect(summary.ingestionCapabilities, 1);
      expect(summary.actuationCapabilities, 1);
      expect(summary.internalCapabilities, 2);
      expect(summary.trustedCapabilities, 3);
      expect(summary.untrustedCapabilities, 1);
      expect(summary.inputCapabilities, 2);
      expect(summary.untrustedInputs, 1);
      expect(summary.externalActuations, 1);
      expect(summary.readSideCapabilities, 1);
      expect(summary.writeSideCapabilities, 2);
      expect(summary.overlappingReadWritePairs, 1);
      expect(summary.justifiedCapabilities, 1);
      expect(summary.hasRwsRisk, isTrue);
      expect(summary.hasSameResourceOverlap, isTrue);
      expect(summary.isEmpty, isFalse);
    });

    test('empty manifest summary is empty', () {
      final summary = summarizeManifest([]);
      expect(summary.isEmpty, isTrue);
      expect(summary.hasRwsRisk, isFalse);
      expect(summary.hasSameResourceOverlap, isFalse);
    });
  });

  group('EXTRA: default flavor table', () {
    test('actuation defaults', () {
      for (final c in [
        cap('net', 'connect', 'h'),
        cap('fs', 'write', 'p'),
        cap('fs', 'create', 'p'),
        cap('fs', 'delete', 'p'),
        cap('vault', 'write', 's'),
        cap('vault', 'request_lease', 's'),
        cap('proc', 'exec', 'x'),
        cap('proc', 'fork', 'x'),
        cap('proc', 'signal', 'x'),
        cap('proc', 'anything', 'x'),
      ]) {
        expect(classifyCapability(c).flavor, CapabilityFlavor.actuation,
            reason: c.identifier());
      }
    });

    test('internal defaults', () {
      for (final c in [
        cap('net', 'listen', 'h'),
        cap('fs', 'read', 'p'),
        cap('channel', 'read', 'c'),
        cap('channel', 'write', 'c'),
        cap('vault', 'read', 's'),
        cap('junk', 'junk', 'j'),
      ]) {
        expect(classifyCapability(c).flavor, CapabilityFlavor.internal,
            reason: c.identifier());
      }
    });

    test('explicit flavor always wins', () {
      final c = cap('proc', 'exec', 'x').withFlavor(CapabilityFlavor.ingestion);
      expect(classifyCapability(c).flavor, CapabilityFlavor.ingestion);
    });
  });

  group('EXTRA: default trust table', () {
    test('loopback listeners are trusted', () {
      for (final target in [
        'localhost',
        'localhost:8080',
        '127.0.0.1',
        '127.0.0.1:80',
        '::1',
        '[::1]:80',
      ]) {
        final c = cap('net', 'listen', target);
        expect(classifyCapability(c).trust, CapabilityTrust.trusted,
            reason: target);
      }
    });

    test('non-loopback listeners are untrusted', () {
      for (final target in [
        '0.0.0.0:8080',
        'example.com:443',
        'localhostx',
        '127.0.0.10',
        '::2',
        '[::1]',
      ]) {
        final c = cap('net', 'listen', target);
        expect(classifyCapability(c).trust, CapabilityTrust.untrusted,
            reason: target);
      }
    });

    test('package-internal reads are trusted', () {
      for (final target in [
        'package:/state/x.json',
        'pkg:/state/x.json',
        './package/state/x.json',
        'package/state/x.json',
      ]) {
        final c = cap('fs', 'read', target);
        expect(classifyCapability(c).trust, CapabilityTrust.trusted,
            reason: target);
      }
    });

    test('external reads and connects are untrusted', () {
      for (final c in [
        cap('fs', 'read', '/etc/passwd'),
        cap('fs', 'read', 'packages/x.json'),
        cap('net', 'connect', 'h:1'),
      ]) {
        expect(classifyCapability(c).trust, CapabilityTrust.untrusted,
            reason: c.identifier());
      }
    });

    test('everything else defaults to trusted', () {
      for (final c in [
        cap('channel', 'read', 'c'),
        cap('vault', 'read', 's'),
        cap('proc', 'exec', 'x'),
        cap('junk', 'junk', 'j'),
      ]) {
        expect(classifyCapability(c).trust, CapabilityTrust.trusted,
            reason: c.identifier());
      }
    });

    test('explicit trust always wins', () {
      expect(
          classifyCapability(cap('fs', 'read', '/etc/passwd')
                  .withTrust(CapabilityTrust.trusted))
              .trust,
          CapabilityTrust.trusted);
      expect(
          classifyCapability(cap('net', 'listen', 'localhost')
                  .withTrust(CapabilityTrust.untrusted))
              .trust,
          CapabilityTrust.untrusted);
    });
  });

  group('EXTRA: input classification', () {
    test('listen, fs-read, and channel-read are always inputs', () {
      for (final c in [
        cap('net', 'listen', 'h'),
        cap('fs', 'read', 'p'),
        cap('channel', 'read', 'c'),
      ]) {
        expect(classifyCapability(c).isInput, isTrue, reason: c.identifier());
      }
    });

    test('connect and the default arm follow the ingestion flavor', () {
      expect(classifyCapability(cap('net', 'connect', 'h')).isInput, isFalse);
      expect(
          classifyCapability(cap('net', 'connect', 'h')
                  .withFlavor(CapabilityFlavor.ingestion))
              .isInput,
          isTrue);
      expect(classifyCapability(cap('junk', 'junk', 'j')).isInput, isFalse);
      expect(
          classifyCapability(cap('junk', 'junk', 'j')
                  .withFlavor(CapabilityFlavor.ingestion))
              .isInput,
          isTrue);
    });
  });

  group('EXTRA: overlap edges', () {
    test('write-side globs match reads (reversed direction)', () {
      final capabilities = [
        cap('fs', 'read', 'package:/state/cache.json'),
        cap('fs', 'write', 'package:/state/*'),
      ];
      expect(
          () => validateManifest(capabilities), throwsA(isA<RwsViolation>()));
    });

    test('bare star glob matches everything', () {
      final capabilities = [
        cap('fs', 'read', '*'),
        cap('fs', 'write', 'anything/at/all'),
      ];
      expect(
          () => validateManifest(capabilities), throwsA(isA<RwsViolation>()));
    });

    test('same target in different categories does not overlap', () {
      final capabilities = [
        cap('fs', 'read', 'shared'),
        cap('channel', 'write', 'shared'),
      ];
      expect(() => validateManifest(capabilities), returnsNormally);
    });

    test('overlap message wins when both triggers fire', () {
      final capabilities = [
        cap('fs', 'read', 'package:/same.json'),
        cap('fs', 'write', 'package:/same.json'),
        cap('net', 'connect', 'evil.example:1')
            .withFlavor(CapabilityFlavor.ingestion),
      ];
      final violation = catchViolation(capabilities);
      expect(violation.message, contains('overlapping read/write'));
      expect(violation.message, isNot(contains('split the agent')));
      // The classified untrusted input plus the overlapping read.
      expect(violation.untrustedInputs, hasLength(2));
      expect(violation.actuations, hasLength(1));
    });

    test('duplicate capabilities collapse in violation lists', () {
      final dup = cap('net', 'connect', 'evil.example:1')
          .withFlavor(CapabilityFlavor.ingestion);
      final capabilities = [dup, dup, cap('fs', 'write', '/tmp/x')];
      final violation = catchViolation(capabilities);
      expect(violation.untrustedInputs, hasLength(1));
      expect(violation.actuations, hasLength(1));
    });
  });

  group('EXTRA: risk-flag combinations', () {
    test('one-sided manifests report no risk', () {
      final onlyInput = summarizeManifest([
        cap('net', 'connect', 'h:1').withFlavor(CapabilityFlavor.ingestion),
      ]);
      expect(onlyInput.untrustedInputs, 1);
      expect(onlyInput.externalActuations, 0);
      expect(onlyInput.hasRwsRisk, isFalse);

      final onlyActuation = summarizeManifest([cap('fs', 'write', 'p')]);
      expect(onlyActuation.untrustedInputs, 0);
      expect(onlyActuation.externalActuations, 1);
      expect(onlyActuation.hasRwsRisk, isFalse);
    });
  });

  group('EXTRA: value semantics', () {
    test('enums expose wire forms', () {
      expect(CapabilityFlavor.ingestion.wire, 'ingestion');
      expect(CapabilityFlavor.actuation.wire, 'actuation');
      expect(CapabilityFlavor.internal.wire, 'internal');
      expect(CapabilityTrust.trusted.wire, 'trusted');
      expect(CapabilityTrust.untrusted.wire, 'untrusted');
      expect(CapabilityFlavor.actuation.toString(), 'actuation');
      expect(CapabilityTrust.untrusted.toString(), 'untrusted');
    });

    test('Capability builders, identifier, and equality', () {
      final base = cap('fs', 'read', 'p');
      expect(base.flavor, isNull);
      expect(base.trust, isNull);
      expect(base.justification, isNull);
      final full = base
          .withFlavor(CapabilityFlavor.ingestion)
          .withTrust(CapabilityTrust.trusted)
          .withJustification('why');
      expect(full.flavor, CapabilityFlavor.ingestion);
      expect(full.trust, CapabilityTrust.trusted);
      expect(full.justification, 'why');
      // Builders copy: the original is unchanged.
      expect(base.flavor, isNull);
      expect(full.identifier(), 'fs:read:p');

      expect(cap('fs', 'read', 'p'), equals(cap('fs', 'read', 'p')));
      expect(cap('fs', 'read', 'p').hashCode, cap('fs', 'read', 'p').hashCode);
      expect(cap('fs', 'read', 'p') == cap('fs', 'write', 'p'), isFalse);
      expect(cap('fs', 'read', 'p') == cap('fs', 'read', 'q'), isFalse);
      expect(
          cap('fs', 'read', 'p') ==
              cap('fs', 'read', 'p').withJustification('j'),
          isFalse);
      expect(cap('fs', 'read', 'p') == 'fs:read:p', isFalse);
      expect(cap('fs', 'read', 'p').toString(), contains('fs:read:p'));
    });

    test('Classification equality', () {
      const a = CapabilityClassification(
        flavor: CapabilityFlavor.ingestion,
        trust: CapabilityTrust.untrusted,
        isInput: true,
        isUntrustedInput: true,
        isExternalActuation: false,
      );
      const b = CapabilityClassification(
        flavor: CapabilityFlavor.ingestion,
        trust: CapabilityTrust.untrusted,
        isInput: true,
        isUntrustedInput: true,
        isExternalActuation: false,
      );
      const c = CapabilityClassification(
        flavor: CapabilityFlavor.actuation,
        trust: CapabilityTrust.untrusted,
        isInput: false,
        isUntrustedInput: false,
        isExternalActuation: true,
      );
      expect(a, equals(b));
      expect(a.hashCode, b.hashCode);
      expect(a == c, isFalse);
      expect(a == 'x', isFalse);
      expect(a.toString(), contains('ingestion'));
    });

    test('Summary equality', () {
      final caps = [cap('fs', 'read', 'package:/x')];
      expect(summarizeManifest(caps), equals(summarizeManifest(caps)));
      expect(
          summarizeManifest(caps).hashCode, summarizeManifest(caps).hashCode);
      expect(summarizeManifest(caps) == summarizeManifest([]), isFalse);
      expect(summarizeManifest(caps) == 'x', isFalse);
      expect(summarizeManifest(caps).toString(), contains('total: 1'));
      expect(const CapabilityManifestSummary().isEmpty, isTrue);
    });

    test('Violation equality and display', () {
      final v1 =
          catchViolation([cap('fs', 'read', 's'), cap('fs', 'write', 's')]);
      final v2 =
          catchViolation([cap('fs', 'read', 's'), cap('fs', 'write', 's')]);
      expect(v1, equals(v2));
      expect(v1.hashCode, v2.hashCode);
      expect(v1.toString(), v1.message);
      expect(v1 == 'x', isFalse);
      // Length mismatch.
      const empty =
          RwsViolation(untrustedInputs: [], actuations: [], message: 'm');
      expect(
          v1 ==
              const RwsViolation(
                  untrustedInputs: [], actuations: [], message: 'm'),
          isFalse);
      expect(empty == empty, isTrue);
      // Element mismatch.
      final other = RwsViolation(
        untrustedInputs: [cap('fs', 'read', 'other')],
        actuations: v1.actuations,
        message: v1.message,
      );
      expect(v1 == other, isFalse);
      // Message mismatch.
      final otherMsg = RwsViolation(
        untrustedInputs: v1.untrustedInputs,
        actuations: v1.actuations,
        message: 'different',
      );
      expect(v1 == otherMsg, isFalse);
    });
  });
}
