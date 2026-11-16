import 'messages.dart';

/// Closed set of ML02 validation failures, listed in precedence order.
enum GradientDescentErrorCode {
  /// Kept for cross-lane parity with Java/Kotlin. Dart's sound null safety
  /// makes it unreachable from typed callers.
  nullInput(errorIdNullInput, messageNullInput),
  lengthMismatch(errorIdLengthMismatch, messageLengthMismatch),
  emptyInput(errorIdEmptyInput, messageEmptyInput),
  inputTooLarge(errorIdInputTooLarge, messageInputTooLarge),
  nonFiniteLearningRate(
    errorIdNonFiniteLearningRate,
    messageNonFiniteLearningRate,
  ),
  nonFiniteInput(errorIdNonFiniteInput, messageNonFiniteInput),
  numericOverflow(errorIdNumericOverflow, messageNumericOverflow);

  /// Stable, language-neutral identifier (for example `length-mismatch`).
  final String errorId;

  /// Static human-readable message. Never contains caller data.
  final String message;

  const GradientDescentErrorCode(this.errorId, this.message);
}

/// Sealed base error so callers can match exhaustively and nobody outside
/// this library can add subtypes.
sealed class GradientDescentError extends ArgumentError {
  GradientDescentError(super.message);
}

/// Thrown when `sgd` rejects its inputs.
///
/// The constructor accepts only a closed [GradientDescentErrorCode], so the
/// message is always the static text for that code (payload-blind, CWE-209).
final class InvalidGradientDescentInputError extends GradientDescentError {
  /// The validation failure that was detected.
  final GradientDescentErrorCode errorCode;

  InvalidGradientDescentInputError(this.errorCode) : super(errorCode.message);

  /// Stable identifier of [errorCode].
  String get errorId => errorCode.errorId;
}
