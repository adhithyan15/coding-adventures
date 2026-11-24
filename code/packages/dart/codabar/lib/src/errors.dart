import 'messages.dart';

/// Closed enumeration of valid Codabar validation error codes in Dart.
enum CodabarErrorCode {
  inputTooLong(codabarErrorInputTooLong),
  invalidCharacter(codabarErrorInvalidCharacter),
  invalidGuard(codabarErrorInvalidGuard),
  guardConflict(codabarErrorGuardConflict);

  final String errorId;
  const CodabarErrorCode(this.errorId);
}

/// Sealed base error type for Codabar operations, preventing unauthorized
/// subclassing.
sealed class CodabarError extends ArgumentError {
  CodabarError(super.message);
}

/// Thrown when Codabar input fails validation against the symbology
/// specification.
final class InvalidCodabarInputError extends CodabarError {
  final CodabarErrorCode errorCode;

  InvalidCodabarInputError(this.errorCode)
      : super(codabarMessageFor(errorCode));

  String get errorId => errorCode.errorId;
}
