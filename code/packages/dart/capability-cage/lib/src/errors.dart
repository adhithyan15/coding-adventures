/// Errors produced by the capability cage.
library;

import 'package:coding_adventures_read_write_separation/coding_adventures_read_write_separation.dart'
    as rws;

import 'category.dart';

/// A capability check denied an OS operation.
///
/// Thrown by [Manifest.check] (see `manifest.dart`) when the manifest does
/// not cover the requested triple.
class CapabilityViolationError implements Exception {
  /// Requested category.
  final Category category;

  /// Requested action.
  final Action action;

  /// Requested target.
  final String target;

  /// Human-readable diagnostic.
  final String message;

  /// Creates a violation.
  const CapabilityViolationError({
    required this.category,
    required this.action,
    required this.target,
    required this.message,
  });

  @override
  bool operator ==(Object other) =>
      other is CapabilityViolationError &&
      category == other.category &&
      action == other.action &&
      target == other.target &&
      message == other.message;

  @override
  int get hashCode => Object.hash(category, action, target, message);

  @override
  String toString() => message;
}

/// A `(category, action)` pair that is not meaningful per the taxonomy, or
/// a target that does not parse.
sealed class InvalidCombination implements Exception {
  const InvalidCombination();
}

/// The pair is not in the cage taxonomy.
final class UnsupportedPair extends InvalidCombination {
  /// The offending category.
  final Category category;

  /// The offending action.
  final Action action;

  /// Creates the error.
  const UnsupportedPair(this.category, this.action);

  @override
  bool operator ==(Object other) =>
      other is UnsupportedPair &&
      category == other.category &&
      action == other.action;

  @override
  int get hashCode => Object.hash(category, action);

  @override
  String toString() =>
      'category $category does not support action $action per the cage taxonomy';
}

/// The capability target is empty.
final class EmptyTarget extends InvalidCombination {
  const EmptyTarget();

  @override
  bool operator ==(Object other) => other is EmptyTarget;

  @override
  int get hashCode => 0;

  @override
  String toString() => 'capability target is empty';
}

/// The capability target has a bad format.
final class InvalidTargetFormat extends InvalidCombination {
  /// Why the target is bad.
  final String reason;

  /// Creates the error.
  const InvalidTargetFormat(this.reason);

  @override
  bool operator ==(Object other) =>
      other is InvalidTargetFormat && reason == other.reason;

  @override
  int get hashCode => reason.hashCode;

  @override
  String toString() => 'invalid capability target: $reason';
}

/// Anything that can go wrong loading a `required_capabilities.json`.
sealed class ManifestError implements Exception {
  const ManifestError();

  /// Converts an [rws.RwsViolation] into the manifest-level variant,
  /// mirroring the reference's `From` impl.
  factory ManifestError.fromRwsViolation(rws.RwsViolation violation) =>
      ManifestRwsViolation(
        untrustedInputs: violation.untrustedInputs,
        actuations: violation.actuations,
        message: violation.message,
      );
}

/// The JSON did not parse (or is not UTF-8).
final class ManifestParseError extends ManifestError {
  /// The underlying parse failure.
  final String detail;

  /// Creates the error.
  const ManifestParseError(this.detail);

  @override
  bool operator ==(Object other) =>
      other is ManifestParseError && detail == other.detail;

  @override
  int get hashCode => detail.hashCode;

  @override
  String toString() => 'manifest parse error: $detail';
}

/// The JSON parsed but did not match the manifest schema.
final class ManifestSchemaError extends ManifestError {
  /// Why the document is not a manifest.
  final String reason;

  /// Creates the error.
  const ManifestSchemaError(this.reason);

  @override
  bool operator ==(Object other) =>
      other is ManifestSchemaError && reason == other.reason;

  @override
  int get hashCode => reason.hashCode;

  @override
  String toString() => 'manifest schema error: $reason';
}

/// A capability failed pair/target validation.
final class ManifestInvalidCombination extends ManifestError {
  /// The underlying combination error.
  final InvalidCombination inner;

  /// Creates the error.
  const ManifestInvalidCombination(this.inner);

  @override
  bool operator ==(Object other) =>
      other is ManifestInvalidCombination && inner == other.inner;

  @override
  int get hashCode => inner.hashCode;

  @override
  String toString() => 'manifest validation error: $inner';
}

/// An I/O error reading the manifest file.
final class ManifestIoError extends ManifestError {
  /// The underlying failure, rendered.
  final String detail;

  /// Creates the error.
  const ManifestIoError(this.detail);

  @override
  bool operator ==(Object other) =>
      other is ManifestIoError && detail == other.detail;

  @override
  int get hashCode => detail.hashCode;

  @override
  String toString() => 'manifest I/O error: $detail';
}

/// The manifest violates read/write separation.
final class ManifestRwsViolation extends ManifestError {
  /// The offending untrusted inputs (plus overlapping reads).
  final List<rws.Capability> untrustedInputs;

  /// The offending actuations (plus overlapping writes).
  final List<rws.Capability> actuations;

  /// Human-readable reason.
  final String message;

  /// Creates the error.
  const ManifestRwsViolation({
    required this.untrustedInputs,
    required this.actuations,
    required this.message,
  });

  @override
  bool operator ==(Object other) =>
      other is ManifestRwsViolation &&
      _capabilitiesEqual(untrustedInputs, other.untrustedInputs) &&
      _capabilitiesEqual(actuations, other.actuations) &&
      message == other.message;

  @override
  int get hashCode => Object.hash(
      Object.hashAll(untrustedInputs), Object.hashAll(actuations), message);

  @override
  String toString() => 'manifest read/write separation error: $message';
}

bool _capabilitiesEqual(List<rws.Capability> a, List<rws.Capability> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
