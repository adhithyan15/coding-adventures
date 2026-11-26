import 'messages.dart';

/// Closed enumeration of valid Code 39 validation error codes in Dart.
enum Code39ErrorCode {
  inputTooLong(code39ErrorInputTooLong),
  invalidCharacter(code39ErrorInvalidCharacter),
  reservedCharacter(code39ErrorReservedCharacter);

  final String errorId;
  const Code39ErrorCode(this.errorId);
}

/// Sealed base error type for Code 39 operations, preventing unauthorized
/// subclassing.
sealed class Code39Error extends ArgumentError {
  Code39Error(super.message);
}

/// Thrown when Code 39 input fails validation against the symbology
/// specification.
final class InvalidCode39InputError extends Code39Error {
  final Code39ErrorCode errorCode;

  InvalidCode39InputError(this.errorCode) : super(code39MessageFor(errorCode));

  String get errorId => errorCode.errorId;
}
