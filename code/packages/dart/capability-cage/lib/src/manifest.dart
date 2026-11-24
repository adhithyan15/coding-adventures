/// Immutable capability manifest.
///
library;

import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart'
    as rws;

import 'capability.dart';
import 'category.dart';
import 'errors.dart';
import 'glob.dart';

/// An immutable capability manifest: the answer to *does this manifest
/// cover this (category, action, target) triple?*
///
/// The default — an empty manifest — grants no OS access, which is the
/// expected state for pure-computation packages.
class Manifest {
  final List<Capability> _capabilities;

  const Manifest._(this._capabilities);

  /// Builds a manifest from already-validated [capabilities].
  ///
  /// Each capability must have been constructed via [Capability.new];
  /// this constructor does not re-check pairs or read/write separation.
  /// Prefer [Manifest.checked] for manifests assembled from metadata.
  factory Manifest(List<Capability> capabilities) =>
      Manifest._(List.of(capabilities));

  /// Builds a manifest and validates read/write separation.
  ///
  /// Throws [ManifestRwsViolation] when the set mixes untrusted inputs
  /// with external actuations or holds an overlapping read/write pair.
  factory Manifest.checked(List<Capability> capabilities) {
    final manifest = Manifest(capabilities);
    manifest.validateReadWriteSeparation();
    return manifest;
  }

  /// The zero-capability manifest.
  factory Manifest.empty() => Manifest(const []);

  /// Loads a manifest from a `required_capabilities.json` document.
  ///
  /// Requires integer `version: 1` and a `capabilities` array of
  /// `{category, action, target}` objects with optional `justification`,
  /// `flavor`, and `trust` fields. Extra fields are ignored. Throws
  /// [ManifestParseError], [ManifestSchemaError],
  /// [ManifestInvalidCombination], or [ManifestRwsViolation].
  factory Manifest.loadFromString(String document) {
    final dynamic root;
    try {
      root = jsonDecode(document);
    } on FormatException catch (e) {
      throw ManifestParseError(e.toString());
    }
    if (root is! Map<String, dynamic>) {
      throw const ManifestSchemaError('top-level value must be a JSON object');
    }
    final version = root['version'];
    if (version is! int) {
      throw const ManifestSchemaError('missing or non-integer field: version');
    }
    if (version != 1) {
      throw ManifestSchemaError(
          'unsupported manifest version: $version (only 1 is supported in v1)');
    }
    if (!root.containsKey('capabilities')) {
      throw const ManifestSchemaError('missing field: capabilities');
    }
    final capsValue = root['capabilities'];
    if (capsValue is! List) {
      throw const ManifestSchemaError('capabilities must be an array');
    }
    final caps = <Capability>[];
    for (var idx = 0; idx < capsValue.length; idx++) {
      caps.add(_parseEntry(capsValue[idx], idx));
    }
    return Manifest.checked(caps);
  }

  /// Loads a manifest from the file at [path].
  ///
  /// Throws [ManifestIoError] when the file cannot be read and
  /// [ManifestParseError] when its bytes are not UTF-8.
  factory Manifest.loadFromFile(String path) {
    final List<int> bytes;
    try {
      bytes = File(path).readAsBytesSync();
    } catch (e) {
      throw ManifestIoError(e.toString());
    }
    final String document;
    try {
      document = utf8.decode(bytes);
    } on FormatException catch (e) {
      throw ManifestParseError('manifest is not UTF-8: $e');
    }
    return Manifest.loadFromString(document);
  }

  /// True iff a declared capability covers ([category], [action], [target]).
  ///
  /// Targets in the manifest are glob patterns matched against the
  /// literal [target].
  bool has(Category category, Action action, String target) =>
      _capabilities.any((cap) =>
          cap.category == category &&
          cap.action == action &&
          matchTarget(cap.target, target));

  /// Throws [CapabilityViolationError] unless [has] covers the triple.
  void check(Category category, Action action, String target) {
    if (!has(category, action, target)) {
      throw CapabilityViolationError(
        category: category,
        action: action,
        target: target,
        message: 'capability ${category.wire}:${action.wire}:$target '
            'not declared; add it to required_capabilities.json',
      );
    }
  }

  /// The declared capabilities (unmodifiable view).
  List<Capability> get capabilities => List.unmodifiable(_capabilities);

  /// Payload-free summary of the manifest's capability surface.
  CapabilitySurfaceSummary capabilitySurfaceSummary() =>
      CapabilitySurfaceSummary.fromCapabilities(_capabilities);

  /// Re-runs read/write separation validation for this manifest.
  void validateReadWriteSeparation() {
    final rwsCaps = _capabilities.map((cap) => cap.toRwsCapability()).toList();
    try {
      rws.validateManifest(rwsCaps);
    } on rws.RwsViolation catch (e) {
      throw ManifestError.fromRwsViolation(e);
    }
  }

  @override
  bool operator ==(Object other) =>
      other is Manifest &&
      _capabilitiesEqual(_capabilities, other._capabilities);

  @override
  int get hashCode => Object.hashAll(_capabilities);

  @override
  String toString() => 'Manifest(${_capabilities.length} capabilities)';
}

bool _capabilitiesEqual(List<Capability> a, List<Capability> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

Capability _parseEntry(dynamic entry, int idx) {
  if (entry is! Map<String, dynamic>) {
    throw ManifestSchemaError('capabilities[$idx] must be a JSON object');
  }
  final categoryStr = entry['category'];
  if (categoryStr is! String) {
    throw ManifestSchemaError('capabilities[$idx] missing string \'category\'');
  }
  final actionStr = entry['action'];
  if (actionStr is! String) {
    throw ManifestSchemaError('capabilities[$idx] missing string \'action\'');
  }
  final target = entry['target'];
  if (target is! String) {
    throw ManifestSchemaError('capabilities[$idx] missing string \'target\'');
  }
  final justification = entry['justification'];
  final flavor = _parseOptionalEnum(
      entry, 'flavor', idx, rws.CapabilityFlavor.values, (v) => v.wire);
  final trust = _parseOptionalEnum(
      entry, 'trust', idx, rws.CapabilityTrust.values, (v) => v.wire);
  final category = Category.parse(categoryStr);
  if (category == null) {
    throw ManifestSchemaError(
        'capabilities[$idx].category \'$categoryStr\' is not a known category');
  }
  final action = Action.parse(actionStr);
  if (action == null) {
    throw ManifestSchemaError(
        'capabilities[$idx].action \'$actionStr\' is not a known action');
  }
  try {
    return Capability(
      category: category,
      action: action,
      target: target,
      justification: justification is String ? justification : '',
      flavor: flavor,
      trust: trust,
    );
  } on InvalidCombination catch (e) {
    throw ManifestInvalidCombination(e);
  }
}

T? _parseOptionalEnum<T>(
  Map<String, dynamic> entry,
  String field,
  int idx,
  List<T> values,
  String Function(T value) wire,
) {
  if (!entry.containsKey(field)) return null;
  final raw = entry[field];
  if (raw is! String) {
    throw ManifestSchemaError('capabilities[$idx].$field must be a string');
  }
  for (final value in values) {
    if (wire(value) == raw) return value;
  }
  throw ManifestSchemaError(
      'capabilities[$idx].$field \'$raw\' is not supported');
}

/// Payload-free read-side view of one manifest: counts by category,
/// read/write-likeness, effects, and read/write-separation annotations.
class CapabilitySurfaceSummary {
  /// Total capabilities.
  final int totalCapabilities;

  /// Distinct categories, actions, and targets.
  final int uniqueCategories;

  /// Distinct categories, actions, and targets.
  final int uniqueActions;

  /// Distinct categories, actions, and targets.
  final int uniqueTargets;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int filesystemCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int networkCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int processCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int environmentCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int ffiCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int timeCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int stdioCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int randCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int threadCapabilities;

  /// Capabilities per category (`stdin` and `stdout` share one).
  final int uiCapabilities;

  /// Capabilities with read-like actions.
  final int readLikeCapabilities;

  /// Capabilities with write-like actions.
  final int writeLikeCapabilities;

  /// Capabilities with external effects.
  final int externalEffectCapabilities;

  /// Capabilities by flavor annotation.
  final int ingestionCapabilities;

  /// Capabilities by flavor annotation.
  final int actuationCapabilities;

  /// Capabilities by flavor annotation.
  final int internalCapabilities;

  /// Capabilities by trust annotation.
  final int trustedCapabilities;

  /// Capabilities by trust annotation.
  final int untrustedCapabilities;

  /// Capabilities with neither annotation.
  final int unannotatedCapabilities;

  /// All-zero summary.
  const CapabilitySurfaceSummary({
    this.totalCapabilities = 0,
    this.uniqueCategories = 0,
    this.uniqueActions = 0,
    this.uniqueTargets = 0,
    this.filesystemCapabilities = 0,
    this.networkCapabilities = 0,
    this.processCapabilities = 0,
    this.environmentCapabilities = 0,
    this.ffiCapabilities = 0,
    this.timeCapabilities = 0,
    this.stdioCapabilities = 0,
    this.randCapabilities = 0,
    this.threadCapabilities = 0,
    this.uiCapabilities = 0,
    this.readLikeCapabilities = 0,
    this.writeLikeCapabilities = 0,
    this.externalEffectCapabilities = 0,
    this.ingestionCapabilities = 0,
    this.actuationCapabilities = 0,
    this.internalCapabilities = 0,
    this.trustedCapabilities = 0,
    this.untrustedCapabilities = 0,
    this.unannotatedCapabilities = 0,
  });

  /// The all-zero summary.
  factory CapabilitySurfaceSummary.empty() => const CapabilitySurfaceSummary();

  /// Counts [capabilities].
  factory CapabilitySurfaceSummary.fromCapabilities(
      List<Capability> capabilities) {
    final categories = <Category>{};
    final actions = <Action>{};
    final targets = <String>{};
    var filesystem = 0;
    var network = 0;
    var process = 0;
    var environment = 0;
    var ffi = 0;
    var time = 0;
    var stdio = 0;
    var rand = 0;
    var thread = 0;
    var ui = 0;
    var readLike = 0;
    var writeLike = 0;
    var externalEffect = 0;
    var ingestion = 0;
    var actuation = 0;
    var internal = 0;
    var trusted = 0;
    var untrusted = 0;
    var unannotated = 0;
    for (final capability in capabilities) {
      categories.add(capability.category);
      actions.add(capability.action);
      targets.add(capability.target);
      switch (capability.category) {
        case Category.fs:
          filesystem++;
        case Category.net:
          network++;
        case Category.proc:
          process++;
        case Category.env:
          environment++;
        case Category.ffi:
          ffi++;
        case Category.time:
          time++;
        case Category.stdin:
        case Category.stdout:
          stdio++;
        case Category.rand:
          rand++;
        case Category.thread:
          thread++;
        case Category.ui:
          ui++;
      }
      if (_isReadLikeAction(capability.action)) readLike++;
      if (_isWriteLikeAction(capability.action)) writeLike++;
      if (_isExternalEffectCapability(capability)) externalEffect++;
      switch (capability.flavor) {
        case rws.CapabilityFlavor.ingestion:
          ingestion++;
        case rws.CapabilityFlavor.actuation:
          actuation++;
        case rws.CapabilityFlavor.internal:
          internal++;
        case null:
          break;
      }
      switch (capability.trust) {
        case rws.CapabilityTrust.trusted:
          trusted++;
        case rws.CapabilityTrust.untrusted:
          untrusted++;
        case null:
          break;
      }
      if (capability.flavor == null && capability.trust == null) {
        unannotated++;
      }
    }
    return CapabilitySurfaceSummary(
      totalCapabilities: capabilities.length,
      uniqueCategories: categories.length,
      uniqueActions: actions.length,
      uniqueTargets: targets.length,
      filesystemCapabilities: filesystem,
      networkCapabilities: network,
      processCapabilities: process,
      environmentCapabilities: environment,
      ffiCapabilities: ffi,
      timeCapabilities: time,
      stdioCapabilities: stdio,
      randCapabilities: rand,
      threadCapabilities: thread,
      uiCapabilities: ui,
      readLikeCapabilities: readLike,
      writeLikeCapabilities: writeLike,
      externalEffectCapabilities: externalEffect,
      ingestionCapabilities: ingestion,
      actuationCapabilities: actuation,
      internalCapabilities: internal,
      trustedCapabilities: trusted,
      untrustedCapabilities: untrusted,
      unannotatedCapabilities: unannotated,
    );
  }

  /// True iff no capabilities were counted.
  bool get isEmpty => totalCapabilities == 0;

  /// True iff any network capability was counted.
  bool get hasNetworkAccess => networkCapabilities > 0;

  /// True iff any process capability was counted.
  bool get hasProcessAccess => processCapabilities > 0;

  /// True iff write-like or actuating capabilities exist.
  bool get hasWriteOrActuation =>
      writeLikeCapabilities > 0 || actuationCapabilities > 0;

  /// True iff any untrusted capability was counted.
  bool get crossesUntrustedBoundary => untrustedCapabilities > 0;

  /// True iff empty or every capability carries an annotation.
  bool get fullyAnnotated => isEmpty || unannotatedCapabilities == 0;

  @override
  bool operator ==(Object other) =>
      other is CapabilitySurfaceSummary &&
      totalCapabilities == other.totalCapabilities &&
      uniqueCategories == other.uniqueCategories &&
      uniqueActions == other.uniqueActions &&
      uniqueTargets == other.uniqueTargets &&
      filesystemCapabilities == other.filesystemCapabilities &&
      networkCapabilities == other.networkCapabilities &&
      processCapabilities == other.processCapabilities &&
      environmentCapabilities == other.environmentCapabilities &&
      ffiCapabilities == other.ffiCapabilities &&
      timeCapabilities == other.timeCapabilities &&
      stdioCapabilities == other.stdioCapabilities &&
      randCapabilities == other.randCapabilities &&
      threadCapabilities == other.threadCapabilities &&
      uiCapabilities == other.uiCapabilities &&
      readLikeCapabilities == other.readLikeCapabilities &&
      writeLikeCapabilities == other.writeLikeCapabilities &&
      externalEffectCapabilities == other.externalEffectCapabilities &&
      ingestionCapabilities == other.ingestionCapabilities &&
      actuationCapabilities == other.actuationCapabilities &&
      internalCapabilities == other.internalCapabilities &&
      trustedCapabilities == other.trustedCapabilities &&
      untrustedCapabilities == other.untrustedCapabilities &&
      unannotatedCapabilities == other.unannotatedCapabilities;

  @override
  int get hashCode => Object.hashAll([
        totalCapabilities,
        uniqueCategories,
        uniqueActions,
        uniqueTargets,
        filesystemCapabilities,
        networkCapabilities,
        processCapabilities,
        environmentCapabilities,
        ffiCapabilities,
        timeCapabilities,
        stdioCapabilities,
        randCapabilities,
        threadCapabilities,
        uiCapabilities,
        readLikeCapabilities,
        writeLikeCapabilities,
        externalEffectCapabilities,
        ingestionCapabilities,
        actuationCapabilities,
        internalCapabilities,
        trustedCapabilities,
        untrustedCapabilities,
        unannotatedCapabilities,
      ]);

  @override
  String toString() => 'CapabilitySurfaceSummary(total: $totalCapabilities, '
      'categories: $uniqueCategories, actions: $uniqueActions)';
}

/// Receiving a UI event is a read: input flows into the package.
bool _isReadLikeAction(Action action) =>
    action == Action.read ||
    action == Action.list ||
    action == Action.dns ||
    action == Action.event;

bool _isWriteLikeAction(Action action) =>
    action == Action.write ||
    action == Action.create ||
    action == Action.delete ||
    action == Action.connect ||
    action == Action.listen ||
    action == Action.exec ||
    action == Action.fork ||
    action == Action.signal ||
    action == Action.call ||
    action == Action.load ||
    action == Action.sleep ||
    action == Action.spawn ||
    action == Action.message;

bool _isExternalEffectCapability(Capability capability) =>
    capability.category == Category.net ||
    capability.category == Category.proc ||
    capability.category == Category.ffi ||
    capability.category == Category.stdout ||
    _isWriteLikeAction(capability.action);
