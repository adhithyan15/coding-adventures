import 'messages.dart';

/// Error codes identifying BloomFilter validation and operation failures.
enum BloomFilterErrorCode {
  invalidExpectedItems,
  invalidFalsePositiveRate,
  invalidBitCount,
  invalidHashCount,
  invalidMemoryBytes,
  allocationLimitExceeded,
  capacityOverflow,
  invalidInput,
}

/// Closed sealed exception hierarchy for BloomFilter adhering to Standing Rule #20.
sealed class BloomFilterException implements Exception {
  final BloomFilterErrorCode code;
  final String message;

  const BloomFilterException._({
    required this.code,
    required this.message,
  });

  /// Factory for invalid expected items count.
  factory BloomFilterException.invalidExpectedItems([int? items]) =>
      _BloomFilterException(
        code: BloomFilterErrorCode.invalidExpectedItems,
        message: messageInvalidExpectedItems,
      );

  /// Factory for invalid false positive rate.
  factory BloomFilterException.invalidFalsePositiveRate([double? rate]) =>
      _BloomFilterException(
        code: BloomFilterErrorCode.invalidFalsePositiveRate,
        message: messageInvalidFalsePositiveRate,
      );

  /// Factory for invalid bit count.
  factory BloomFilterException.invalidBitCount([int? count]) =>
      _BloomFilterException(
        code: BloomFilterErrorCode.invalidBitCount,
        message: messageInvalidBitCount,
      );

  /// Factory for invalid hash count.
  factory BloomFilterException.invalidHashCount([int? count]) =>
      _BloomFilterException(
        code: BloomFilterErrorCode.invalidHashCount,
        message: messageInvalidHashCount,
      );

  /// Factory for invalid memory bytes.
  factory BloomFilterException.invalidMemoryBytes([int? bytes]) =>
      _BloomFilterException(
        code: BloomFilterErrorCode.invalidMemoryBytes,
        message: messageInvalidMemoryBytes,
      );

  /// Factory for exceeding maximum allowed allocation limits.
  factory BloomFilterException.allocationLimitExceeded([
    int? requested,
    int? maxBits,
  ]) =>
      _BloomFilterException(
        code: BloomFilterErrorCode.allocationLimitExceeded,
        message: messageAllocationLimitExceeded,
      );

  /// Factory for capacity calculation overflow.
  factory BloomFilterException.capacityOverflow() => _BloomFilterException(
        code: BloomFilterErrorCode.capacityOverflow,
        message: messageCapacityOverflow,
      );

  /// Factory for invalid or null input.
  factory BloomFilterException.invalidInput() => _BloomFilterException(
        code: BloomFilterErrorCode.invalidInput,
        message: messageInvalidInput,
      );

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is BloomFilterException &&
          runtimeType == other.runtimeType &&
          code == other.code &&
          message == other.message;

  @override
  int get hashCode => Object.hash(code, message);

  @override
  String toString() => 'BloomFilterException($code): $message';
}

final class _BloomFilterException extends BloomFilterException {
  const _BloomFilterException({
    required super.code,
    required super.message,
  }) : super._();
}
