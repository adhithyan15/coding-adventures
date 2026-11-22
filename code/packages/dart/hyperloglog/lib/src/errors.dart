import 'messages.dart';

/// Error codes identifying HyperLogLog validation and operation failures.
enum HyperLogLogErrorCode {
  invalidPrecision,
  precisionMismatch,
  invalidDesiredError,
  invalidInput,
}

/// Closed sealed exception hierarchy for HyperLogLog adhering to Standing Rule #20.
sealed class HyperLogLogException implements Exception {
  final HyperLogLogErrorCode code;
  final String message;

  const HyperLogLogException._({
    required this.code,
    required this.message,
  });

  /// Factory for precision out of bounds.
  factory HyperLogLogException.invalidPrecision([int? precision]) =>
      _HyperLogLogException(
        code: HyperLogLogErrorCode.invalidPrecision,
        message: messageInvalidPrecision,
      );

  /// Factory for merging sketches with mismatched precision.
  factory HyperLogLogException.precisionMismatch([int? left, int? right]) =>
      _HyperLogLogException(
        code: HyperLogLogErrorCode.precisionMismatch,
        message: messagePrecisionMismatch,
      );

  /// Factory for invalid desired error specification.
  factory HyperLogLogException.invalidDesiredError([double? error]) =>
      _HyperLogLogException(
        code: HyperLogLogErrorCode.invalidDesiredError,
        message: messageInvalidDesiredError,
      );

  /// Factory for invalid or null input.
  factory HyperLogLogException.invalidInput() => _HyperLogLogException(
        code: HyperLogLogErrorCode.invalidInput,
        message: messageInvalidInput,
      );

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is HyperLogLogException &&
          runtimeType == other.runtimeType &&
          code == other.code &&
          message == other.message;

  @override
  int get hashCode => Object.hash(code, message);

  @override
  String toString() => 'HyperLogLogException($code): $message';
}

final class _HyperLogLogException extends HyperLogLogException {
  const _HyperLogLogException({
    required super.code,
    required super.message,
  }) : super._();
}
