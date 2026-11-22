import 'messages.dart';

/// Error codes identifying distinct failure modes in HashSet.
enum HashSetErrorCode {
  invalidCapacity,
  invalidBatchElements,
  disjointFailed,
  internalInconsistency,
}

/// Sealed exception hierarchy for HashSet complying with Standing Rule #20.
///
/// Constructors are private or internal factories with static messages, ensuring
/// payload-blind diagnostics without caller parameter reflection.
sealed class HashSetException implements Exception {
  const HashSetException._({
    required this.code,
    required this.message,
    required this.errorId,
  });

  /// The strongly-typed internal error code.
  final HashSetErrorCode code;

  /// The human-readable static localized diagnostic string.
  final String message;

  /// The canonical static identifier for this error category.
  final String errorId;

  /// Creates an exception when capacity is invalid or non-positive.
  factory HashSetException.invalidCapacity() => const _HashSetException(
        code: HashSetErrorCode.invalidCapacity,
        message: messageInvalidCapacity,
        errorId: 'invalid-capacity',
      );

  /// Creates an exception when batch elements exceed safety bound.
  factory HashSetException.invalidBatchElements() => const _HashSetException(
        code: HashSetErrorCode.invalidBatchElements,
        message: messageInvalidBatchElements,
        errorId: 'invalid-batch-elements',
      );

  /// Creates an exception when a disjoint relation cannot be verified.
  factory HashSetException.disjointFailed() => const _HashSetException(
        code: HashSetErrorCode.disjointFailed,
        message: messageDisjointFailed,
        errorId: 'disjoint-failed',
      );

  /// Creates an exception when an internal set state inconsistency occurs.
  factory HashSetException.internalInconsistency() => const _HashSetException(
        code: HashSetErrorCode.internalInconsistency,
        message: messageInternalInconsistency,
        errorId: 'internal-inconsistency',
      );

  @override
  String toString() => message;
}

final class _HashSetException extends HashSetException {
  const _HashSetException({
    required super.code,
    required super.message,
    required super.errorId,
  }) : super._();
}
