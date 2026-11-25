import 'errors.dart';

/// Diagnostic and error messages for Code 128 encoding in Dart, strictly
/// payload-blind: callers branch on the stable identifier, never the text.
const String code128ErrorInputTooLong = 'input-too-long';
const String code128ErrorInvalidCharacter = 'invalid-character';

const String code128MessageInputTooLong =
    'Code 128 input must contain at most 4096 characters';
const String code128MessageInvalidCharacter =
    'Code 128 Code Set B supports printable ASCII characters only';
const String code128MessageInvalidScalars =
    'Code 128 input must contain valid Unicode scalar values';
const String code128MessageNullInput = 'Code 128 input must not be null';

String code128MessageFor(Code128ErrorCode? code) {
  switch (code) {
    case Code128ErrorCode.inputTooLong:
      return code128MessageInputTooLong;
    case Code128ErrorCode.invalidCharacter:
    case null:
      return code128MessageInvalidCharacter;
  }
}
