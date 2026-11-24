/// Bounded fuzz campaign for `coding_adventures_read_write_separation`,
/// using the in-lane `fuzz` package: hostile byte strings decode into
/// capability manifests, and every manifest must satisfy the summary
/// invariants, verdict determinism (repeat, reverse, duplicate), and
/// totality (only [RwsViolation] may escape). Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart';
import 'package:test/test.dart';

const _categories = [
  'fs',
  'net',
  'proc',
  'vault',
  'channel',
  'env',
  'ffi',
  'junk',
];
const _actions = [
  'read',
  'write',
  'create',
  'delete',
  'connect',
  'listen',
  'exec',
  'request_lease',
  'junk',
];
const _targets = [
  'localhost',
  'localhost:8080',
  '127.0.0.1:80',
  '0.0.0.0:8080',
  'example.com:443',
  'package:/state/x.json',
  'pkg:/x',
  '/etc/passwd',
  'package:/state/*',
  '*',
  'weather-snapshots',
  'smtp-credentials',
];

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

  String takeChoice(List<String> options) =>
      options[takeByte() % options.length];

  String takeString(int maxLen) {
    final len = takeByte() % (maxLen + 1);
    return String.fromCharCodes([for (var i = 0; i < len; i++) takeByte()]);
  }

  Capability takeCapability() {
    // Vocabulary words dominate, but raw hostile strings stay reachable.
    final category =
        takeByte().isEven ? takeChoice(_categories) : takeString(6);
    final action = takeByte().isEven ? takeChoice(_actions) : takeString(6);
    final target = takeByte() % 3 == 0 ? takeString(10) : takeChoice(_targets);
    CapabilityFlavor? flavor;
    switch (takeByte() % 4) {
      case 1:
        flavor = CapabilityFlavor.ingestion;
      case 2:
        flavor = CapabilityFlavor.actuation;
      case 3:
        flavor = CapabilityFlavor.internal;
    }
    CapabilityTrust? trust;
    switch (takeByte() % 3) {
      case 1:
        trust = CapabilityTrust.trusted;
      case 2:
        trust = CapabilityTrust.untrusted;
    }
    return Capability(
      category: category,
      action: action,
      target: target,
      flavor: flavor,
      trust: trust,
      justification: takeByte().isOdd ? 'why' : null,
    );
  }
}

bool _violates(List<Capability> capabilities) {
  try {
    validateManifest(capabilities);
    return false;
  } on RwsViolation {
    return true;
  }
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final count = d.takeByte() % 7;
    final caps = [for (var i = 0; i < count; i++) d.takeCapability()];
    cov.recordEdge(count);

    final summary = summarizeManifest(caps);
    if (summary.totalCapabilities != caps.length ||
        summary.ingestionCapabilities +
                summary.actuationCapabilities +
                summary.internalCapabilities !=
            caps.length ||
        summary.trustedCapabilities + summary.untrustedCapabilities !=
            caps.length ||
        summary.hasRwsRisk !=
            (summary.untrustedInputs > 0 && summary.externalActuations > 0) ||
        summary.isEmpty != caps.isEmpty) {
      return const OutcomeCrash('summary-invariant', ['counter-mismatch']);
    }
    cov.recordEdge(10 + (summary.untrustedInputs % 4) * 4);

    final verdict = _violates(caps);
    cov.recordEdge(30 + (verdict ? 1 : 0));
    // Determinism: same input, same verdict, every time.
    if (_violates(List.of(caps)) != verdict) {
      return const OutcomeCrash(
          'verdict-nondeterministic', ['repeat-mismatch']);
    }
    // Reversal and duplication preserve the verdict.
    if (_violates(caps.reversed.toList()) != verdict) {
      return const OutcomeCrash('verdict-unstable', ['reversal-mismatch']);
    }
    if (caps.isNotEmpty && _violates([...caps, caps.first]) != verdict) {
      return const OutcomeCrash('verdict-unstable', ['duplication-mismatch']);
    }
    cov.recordEdge(40);
    return const OutcomePass();
  } on RwsViolation {
    // classify/summarize paths never throw; reaching here means an
    // invariant helper itself violated (impossible by construction).
    return const OutcomeCrash('unexpected-throw', ['rws-leak']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([1, 0, 0, 0, 1, 1, 1]),
      Uint8List.fromList([2, 0, 0, 7, 1, 1, 1, 0, 1, 7, 0, 0, 1]),
      Uint8List.fromList([3, 1, 5, 4, 0, 0, 1, 1, 0, 10, 2, 2, 0]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 47); longer
/// ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(47)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: invariants hold, no crashes', () {
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
