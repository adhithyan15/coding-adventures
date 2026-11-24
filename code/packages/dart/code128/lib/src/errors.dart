import 'messages.dart';

/// Closed enumeration of valid Code 128 validation error codes in Dart.
enum Code128ErrorCode {
  inputTooLong(code128ErrorInputTooLong),
  invalidCharacter(code128ErrorInvalidCharacter);

  final String errorId;
  const Code128ErrorCode(this.errorId);
}

/// Sealed base error type for Code 128 operations, preventing unauthorized
/// subclassing.
sealed class Code128Error extends ArgumentError {
  Code128Error(super.message);
}

/// Thrown when Code 128 input fails validation against the symbology
/// specification.
final class InvalidCode128InputError extends Code128Error {
  final Code128ErrorCode errorCode;

  InvalidCode128InputError(this.errorCode)
      : super(code128MessageFor(errorCode));

  String get errorId => errorCode.errorId;
}
