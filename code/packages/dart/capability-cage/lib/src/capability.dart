/// The atomic capability declaration: one OS-level permission.
library;

import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart'
    as rws;

import 'category.dart';
import 'errors.dart';

/// A single OS-level permission declaration.
///
/// Immutable. Construct via [Capability.new], which validates the
/// (category, action) pair against the taxonomy and rejects empty targets.
class Capability {
  /// Capability category.
  final Category category;

  /// Capability action.
  final Action action;

  /// What the capability touches (path, host:port, channel, ...).
  final String target;

  /// Why the capability is needed.
  final String justification;

  /// Read/write-separation flavor annotation.
  final rws.CapabilityFlavor? flavor;

  /// Read/write-separation trust annotation.
  final rws.CapabilityTrust? trust;

  /// Creates a capability after validating the pair and target.
  ///
  /// Throws [UnsupportedPair] for pairs outside the taxonomy and
  /// [EmptyTarget] for empty targets. The [justification] is required,
  /// as upstream: every declaration documents why it exists.
  factory Capability({
    required Category category,
    required Action action,
    required String target,
    required String justification,
    rws.CapabilityFlavor? flavor,
    rws.CapabilityTrust? trust,
  }) {
    if (!isValidCombination(category, action)) {
      throw UnsupportedPair(category, action);
    }
    if (target.isEmpty) {
      throw const EmptyTarget();
    }
    return Capability._(
      category: category,
      action: action,
      target: target,
      justification: justification,
      flavor: flavor,
      trust: trust,
    );
  }

  const Capability._({
    required this.category,
    required this.action,
    required this.target,
    required this.justification,
    required this.flavor,
    required this.trust,
  });

  /// Copy with [flavor] set (for read/write separation checks).
  Capability withFlavor(rws.CapabilityFlavor flavor) => Capability._(
        category: category,
        action: action,
        target: target,
        justification: justification,
        flavor: flavor,
        trust: trust,
      );

  /// Copy with [trust] set (for read/write separation checks).
  Capability withTrust(rws.CapabilityTrust trust) => Capability._(
        category: category,
        action: action,
        target: target,
        justification: justification,
        flavor: flavor,
        trust: trust,
      );

  /// Converts to the read/write-separation lane's capability, carrying
  /// the wire-form category/action plus annotations.
  rws.Capability toRwsCapability() {
    var capability = rws.Capability(
      category: category.wire,
      action: action.wire,
      target: target,
    ).withJustification(justification);
    if (flavor != null) {
      capability = capability.withFlavor(flavor!);
    }
    if (trust != null) {
      capability = capability.withTrust(trust!);
    }
    return capability;
  }

  @override
  bool operator ==(Object other) =>
      other is Capability &&
      category == other.category &&
      action == other.action &&
      target == other.target &&
      justification == other.justification &&
      flavor == other.flavor &&
      trust == other.trust;

  @override
  int get hashCode =>
      Object.hash(category, action, target, justification, flavor, trust);

  @override
  String toString() => 'Capability(${category.wire}:${action.wire}:$target)';
}
