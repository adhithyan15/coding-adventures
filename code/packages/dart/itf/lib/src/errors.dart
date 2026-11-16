import 'messages.dart';

/// Closed enumeration of valid ITF validation error codes in Dart.
enum ItfErrorCode {
  inputTooLong(itfErrorInputTooLong),
  invalidLength(itfErrorInvalidLength),
  invalidCharacter(itfErrorInvalidCharacter);

  final String errorId;
  const ItfErrorCode(this.errorId);
}

/// Sealed base error type for ITF operations, preventing unauthorized subclassing.
sealed class ItfError extends ArgumentError {
  ItfError(super.message);
}

/// Thrown when ITF input fails validation against the ITF specification.
final class InvalidItfInputError extends ItfError {
  final ItfErrorCode errorCode;

  InvalidItfInputError(this.errorCode) : super(itfMessageFor(errorCode));

  String get errorId => errorCode.errorId;
}
