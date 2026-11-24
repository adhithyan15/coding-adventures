/// Manifest-level read/write separation classifier.
///
/// The Dart lane of the Rust `read-write-separation` crate: each
/// [Capability] is classified by flavor ([CapabilityFlavor]) and trust
/// ([CapabilityTrust]), manifests are summarized ([summarizeManifest]) and
/// validated ([validateManifest]). A manifest is rejected when it mixes
/// untrusted inputs with external actuations, or when a read/write pair
/// touches the same (or glob-overlapping) resource.
///
/// One mapping note: the reference returns `Result<(), RwsViolation>`;
/// [validateManifest] throws the [RwsViolation] instead, which is the
/// Dart-idiomatic shape for a validator. Everything else — default tables,
/// overlap rule, glob rule, message texts — mirrors the reference exactly.
library;

/// What a capability is for: data flowing in, effects flowing out, or
/// neither (plumbing inside one trust boundary).
enum CapabilityFlavor {
  /// Data flowing into the agent.
  ingestion('ingestion'),

  /// Effects flowing out of the agent.
  actuation('actuation'),

  /// Plumbing inside one trust boundary.
  internal('internal');

  /// The stable wire form.
  final String wire;
  const CapabilityFlavor(this.wire);

  @override
  String toString() => wire;
}

/// Whether a capability's data source is trusted.
enum CapabilityTrust {
  /// Trusted source.
  trusted('trusted'),

  /// Untrusted source.
  untrusted('untrusted');

  /// The stable wire form.
  final String wire;
  const CapabilityTrust(this.wire);

  @override
  String toString() => wire;
}

/// One declared permission: a category/action/target triple with optional
/// flavor, trust, and justification annotations.
///
/// Immutable: [withFlavor], [withTrust], and [withJustification] each return
/// a copy. Equality compares every field.
class Capability {
  /// Capability category (free-form here; the cage validates taxonomy).
  final String category;

  /// Capability action (free-form here).
  final String action;

  /// What the capability touches (path, host:port, channel name, ...).
  final String target;

  /// Explicit flavor override (defaults apply when null).
  final CapabilityFlavor? flavor;

  /// Explicit trust override (defaults apply when null).
  final CapabilityTrust? trust;

  /// Why the capability is needed.
  final String? justification;

  /// Creates a capability with no annotations.
  const Capability({
    required this.category,
    required this.action,
    required this.target,
    this.flavor,
    this.trust,
    this.justification,
  });

  /// Copy with [flavor] set.
  Capability withFlavor(CapabilityFlavor flavor) => Capability(
        category: category,
        action: action,
        target: target,
        flavor: flavor,
        trust: trust,
        justification: justification,
      );

  /// Copy with [trust] set.
  Capability withTrust(CapabilityTrust trust) => Capability(
        category: category,
        action: action,
        target: target,
        flavor: flavor,
        trust: trust,
        justification: justification,
      );

  /// Copy with [justification] set.
  Capability withJustification(String justification) => Capability(
        category: category,
        action: action,
        target: target,
        flavor: flavor,
        trust: trust,
        justification: justification,
      );

  /// The `category:action:target` identifier.
  String identifier() => '$category:$action:$target';

  @override
  bool operator ==(Object other) =>
      other is Capability &&
      category == other.category &&
      action == other.action &&
      target == other.target &&
      flavor == other.flavor &&
      trust == other.trust &&
      justification == other.justification;

  @override
  int get hashCode =>
      Object.hash(category, action, target, flavor, trust, justification);

  @override
  String toString() => 'Capability(${identifier()})';
}

/// A capability with its effective flavor/trust resolved and its risk roles
/// computed.
class CapabilityClassification {
  /// Explicit flavor, or the category/action default.
  final CapabilityFlavor flavor;

  /// Explicit trust, or the category/action/target default.
  final CapabilityTrust trust;

  /// Whether the capability feeds data into the agent.
  final bool isInput;

  /// [isInput] from an untrusted source.
  final bool isUntrustedInput;

  /// Whether the capability actuates externally (flavor is actuation).
  final bool isExternalActuation;

  /// Creates a classification.
  const CapabilityClassification({
    required this.flavor,
    required this.trust,
    required this.isInput,
    required this.isUntrustedInput,
    required this.isExternalActuation,
  });

  @override
  bool operator ==(Object other) =>
      other is CapabilityClassification &&
      flavor == other.flavor &&
      trust == other.trust &&
      isInput == other.isInput &&
      isUntrustedInput == other.isUntrustedInput &&
      isExternalActuation == other.isExternalActuation;

  @override
  int get hashCode => Object.hash(
      flavor, trust, isInput, isUntrustedInput, isExternalActuation);

  @override
  String toString() =>
      'CapabilityClassification(flavor: $flavor, trust: $trust, '
      'isInput: $isInput, isUntrustedInput: $isUntrustedInput, '
      'isExternalActuation: $isExternalActuation)';
}

/// Counters describing one manifest's capability shape and risk.
class CapabilityManifestSummary {
  /// Total capabilities.
  final int totalCapabilities;

  /// Capabilities by effective flavor.
  final int ingestionCapabilities;

  /// Capabilities by effective flavor.
  final int actuationCapabilities;

  /// Capabilities by effective flavor.
  final int internalCapabilities;

  /// Capabilities by effective trust.
  final int trustedCapabilities;

  /// Capabilities by effective trust.
  final int untrustedCapabilities;

  /// Capabilities with [CapabilityClassification.isInput].
  final int inputCapabilities;

  /// Capabilities with [CapabilityClassification.isUntrustedInput].
  final int untrustedInputs;

  /// Capabilities with [CapabilityClassification.isExternalActuation].
  final int externalActuations;

  /// Capabilities on the read side (fs/vault/channel reads).
  final int readSideCapabilities;

  /// Capabilities on the write side (fs/vault/channel writes).
  final int writeSideCapabilities;

  /// Read/write pairs over the same (or glob-overlapping) resource.
  final int overlappingReadWritePairs;

  /// Capabilities carrying a justification.
  final int justifiedCapabilities;

  /// All-zero summary.
  const CapabilityManifestSummary({
    this.totalCapabilities = 0,
    this.ingestionCapabilities = 0,
    this.actuationCapabilities = 0,
    this.internalCapabilities = 0,
    this.trustedCapabilities = 0,
    this.untrustedCapabilities = 0,
    this.inputCapabilities = 0,
    this.untrustedInputs = 0,
    this.externalActuations = 0,
    this.readSideCapabilities = 0,
    this.writeSideCapabilities = 0,
    this.overlappingReadWritePairs = 0,
    this.justifiedCapabilities = 0,
  });

  /// Summarizes [capabilities], classifying each one.
  factory CapabilityManifestSummary.fromCapabilities(
      List<Capability> capabilities) {
    var ingestion = 0;
    var actuation = 0;
    var internal = 0;
    var trusted = 0;
    var untrusted = 0;
    var inputs = 0;
    var untrustedIns = 0;
    var actuations = 0;
    var reads = 0;
    var writes = 0;
    var justified = 0;
    for (final capability in capabilities) {
      final classification = classifyCapability(capability);
      switch (classification.flavor) {
        case CapabilityFlavor.ingestion:
          ingestion++;
        case CapabilityFlavor.actuation:
          actuation++;
        case CapabilityFlavor.internal:
          internal++;
      }
      switch (classification.trust) {
        case CapabilityTrust.trusted:
          trusted++;
        case CapabilityTrust.untrusted:
          untrusted++;
      }
      if (classification.isInput) inputs++;
      if (classification.isUntrustedInput) untrustedIns++;
      if (classification.isExternalActuation) actuations++;
      if (_isReadSide(capability)) reads++;
      if (_isWriteSide(capability)) writes++;
      if (capability.justification != null) justified++;
    }
    return CapabilityManifestSummary(
      totalCapabilities: capabilities.length,
      ingestionCapabilities: ingestion,
      actuationCapabilities: actuation,
      internalCapabilities: internal,
      trustedCapabilities: trusted,
      untrustedCapabilities: untrusted,
      inputCapabilities: inputs,
      untrustedInputs: untrustedIns,
      externalActuations: actuations,
      readSideCapabilities: reads,
      writeSideCapabilities: writes,
      overlappingReadWritePairs: _countOverlapPairs(capabilities),
      justifiedCapabilities: justified,
    );
  }

  /// True iff the manifest holds no capabilities.
  bool get isEmpty => totalCapabilities == 0;

  /// True iff untrusted inputs meet external actuations.
  bool get hasRwsRisk => untrustedInputs > 0 && externalActuations > 0;

  /// True iff some read/write pair shares a resource.
  bool get hasSameResourceOverlap => overlappingReadWritePairs > 0;

  @override
  bool operator ==(Object other) =>
      other is CapabilityManifestSummary &&
      totalCapabilities == other.totalCapabilities &&
      ingestionCapabilities == other.ingestionCapabilities &&
      actuationCapabilities == other.actuationCapabilities &&
      internalCapabilities == other.internalCapabilities &&
      trustedCapabilities == other.trustedCapabilities &&
      untrustedCapabilities == other.untrustedCapabilities &&
      inputCapabilities == other.inputCapabilities &&
      untrustedInputs == other.untrustedInputs &&
      externalActuations == other.externalActuations &&
      readSideCapabilities == other.readSideCapabilities &&
      writeSideCapabilities == other.writeSideCapabilities &&
      overlappingReadWritePairs == other.overlappingReadWritePairs &&
      justifiedCapabilities == other.justifiedCapabilities;

  @override
  int get hashCode => Object.hash(
        totalCapabilities,
        ingestionCapabilities,
        actuationCapabilities,
        internalCapabilities,
        trustedCapabilities,
        untrustedCapabilities,
        inputCapabilities,
        untrustedInputs,
        externalActuations,
        readSideCapabilities,
        writeSideCapabilities,
        overlappingReadWritePairs,
        justifiedCapabilities,
      );

  @override
  String toString() => 'CapabilityManifestSummary(total: $totalCapabilities, '
      'untrustedInputs: $untrustedInputs, '
      'externalActuations: $externalActuations, '
      'overlappingReadWritePairs: $overlappingReadWritePairs)';
}

/// A manifest that fails [validateManifest]: the offending capabilities plus
/// the human-readable reason.
class RwsViolation implements Exception {
  /// The untrusted-input capabilities (plus, on overlap violations, the
  /// overlapping reads — mirroring the reference, which funnels both
  /// through the same lists).
  final List<Capability> untrustedInputs;

  /// The external-actuation capabilities (plus overlapping writes, ditto).
  final List<Capability> actuations;

  /// Human-readable reason.
  final String message;

  /// Creates a violation.
  const RwsViolation({
    required this.untrustedInputs,
    required this.actuations,
    required this.message,
  });

  @override
  bool operator ==(Object other) =>
      other is RwsViolation &&
      _listsEqual(untrustedInputs, other.untrustedInputs) &&
      _listsEqual(actuations, other.actuations) &&
      message == other.message;

  @override
  int get hashCode => Object.hash(
      Object.hashAll(untrustedInputs), Object.hashAll(actuations), message);

  @override
  String toString() => message;
}

bool _listsEqual(List<Capability> a, List<Capability> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Classifies one [capability]: explicit flavor/trust win, otherwise the
/// category/action(/target) default tables apply.
CapabilityClassification classifyCapability(Capability capability) {
  final flavor = capability.flavor ?? _defaultFlavor(capability);
  final trust = capability.trust ?? _defaultTrust(capability);
  final isInput = _isInputCapability(capability, flavor);
  final isUntrustedInput = isInput && trust == CapabilityTrust.untrusted;
  final isExternalActuation = flavor == CapabilityFlavor.actuation;
  return CapabilityClassification(
    flavor: flavor,
    trust: trust,
    isInput: isInput,
    isUntrustedInput: isUntrustedInput,
    isExternalActuation: isExternalActuation,
  );
}

/// Summarizes [capabilities].
CapabilityManifestSummary summarizeManifest(List<Capability> capabilities) =>
    CapabilityManifestSummary.fromCapabilities(capabilities);

/// Validates [capabilities], throwing [RwsViolation] when the manifest mixes
/// untrusted inputs with external actuations or holds an overlapping
/// read/write pair.
///
/// The overlap message takes precedence when both triggers fire, exactly as
/// in the reference (which checks the overlap flag first).
void validateManifest(List<Capability> capabilities) {
  final untrustedInputs = <Capability>[];
  final actuations = <Capability>[];

  for (final capability in capabilities) {
    final classification = classifyCapability(capability);
    if (classification.isUntrustedInput) {
      _pushUnique(untrustedInputs, capability);
    }
    if (classification.isExternalActuation) {
      _pushUnique(actuations, capability);
    }
  }

  // Computed BEFORE the overlap collector appends to the same lists —
  // order matters and mirrors the reference.
  final hasUntrustedAndActuation =
      untrustedInputs.isNotEmpty && actuations.isNotEmpty;
  final hasOverlap =
      _collectOverlapViolations(capabilities, untrustedInputs, actuations);

  if (hasUntrustedAndActuation || hasOverlap) {
    final message = hasOverlap
        ? 'read/write separation violation: manifest contains overlapping read/write capabilities'
        : 'read/write separation violation: manifest contains untrusted inputs and external actuations; split the agent or insert a trusted channel boundary';
    throw RwsViolation(
      untrustedInputs: untrustedInputs,
      actuations: actuations,
      message: message,
    );
  }
}

CapabilityFlavor _defaultFlavor(Capability capability) {
  final category = capability.category;
  final action = capability.action;
  if (category == 'net' && action == 'connect') {
    return CapabilityFlavor.actuation;
  }
  if (category == 'fs' &&
      (action == 'write' || action == 'create' || action == 'delete')) {
    return CapabilityFlavor.actuation;
  }
  if (category == 'vault' && (action == 'write' || action == 'request_lease')) {
    return CapabilityFlavor.actuation;
  }
  if (category == 'proc') {
    return CapabilityFlavor.actuation;
  }
  return CapabilityFlavor.internal;
}

CapabilityTrust _defaultTrust(Capability capability) {
  final category = capability.category;
  final action = capability.action;
  if (category == 'net' && action == 'connect') {
    return CapabilityTrust.untrusted;
  }
  if (category == 'net' && action == 'listen') {
    return _isLoopbackTarget(capability.target)
        ? CapabilityTrust.trusted
        : CapabilityTrust.untrusted;
  }
  if (category == 'fs' && action == 'read') {
    return _isPackageInternalTarget(capability.target)
        ? CapabilityTrust.trusted
        : CapabilityTrust.untrusted;
  }
  return CapabilityTrust.trusted;
}

bool _isInputCapability(Capability capability, CapabilityFlavor flavor) {
  final category = capability.category;
  final action = capability.action;
  if (category == 'net' && action == 'connect') {
    return flavor == CapabilityFlavor.ingestion;
  }
  if ((category == 'net' && action == 'listen') ||
      (category == 'fs' && action == 'read') ||
      (category == 'channel' && action == 'read')) {
    return true;
  }
  return flavor == CapabilityFlavor.ingestion;
}

bool _isLoopbackTarget(String target) {
  return target == 'localhost' ||
      target.startsWith('localhost:') ||
      target == '127.0.0.1' ||
      target.startsWith('127.0.0.1:') ||
      target == '::1' ||
      target.startsWith('[::1]:');
}

bool _isPackageInternalTarget(String target) {
  return target.startsWith('package:') ||
      target.startsWith('pkg:') ||
      target.startsWith('./package/') ||
      target.startsWith('package/');
}

bool _collectOverlapViolations(List<Capability> capabilities,
    List<Capability> reads, List<Capability> writes) {
  var found = false;
  for (final read in capabilities) {
    if (!_isReadSide(read)) continue;
    for (final write in capabilities) {
      if (!_isWriteSide(write) || read.category != write.category) {
        continue;
      }
      if (_resourcesOverlap(read.target, write.target)) {
        _pushUnique(reads, read);
        _pushUnique(writes, write);
        found = true;
      }
    }
  }
  return found;
}

int _countOverlapPairs(List<Capability> capabilities) {
  var count = 0;
  for (final read in capabilities) {
    if (!_isReadSide(read)) continue;
    for (final write in capabilities) {
      if (!_isWriteSide(write) || read.category != write.category) {
        continue;
      }
      if (_resourcesOverlap(read.target, write.target)) {
        count++;
      }
    }
  }
  return count;
}

bool _isReadSide(Capability capability) {
  final category = capability.category;
  final action = capability.action;
  return (category == 'fs' && action == 'read') ||
      (category == 'vault' && action == 'read') ||
      (category == 'channel' && action == 'read');
}

bool _isWriteSide(Capability capability) {
  final category = capability.category;
  final action = capability.action;
  return (category == 'fs' &&
          (action == 'write' || action == 'create' || action == 'delete')) ||
      (category == 'vault' &&
          (action == 'write' || action == 'request_lease')) ||
      (category == 'channel' && action == 'write');
}

bool _resourcesOverlap(String left, String right) {
  return left == right ||
      _globPrefixMatches(left, right) ||
      _globPrefixMatches(right, left);
}

bool _globPrefixMatches(String pattern, String value) {
  if (!pattern.endsWith('*')) return false;
  final prefix = pattern.substring(0, pattern.length - 1);
  return value.startsWith(prefix);
}

void _pushUnique(List<Capability> collection, Capability capability) {
  if (!collection.contains(capability)) {
    collection.add(capability);
  }
}
